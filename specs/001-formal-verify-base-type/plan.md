# Implementation Plan: Formal Verification of a Core Base Type (`CellId`)

**Branch**: `tb/001-formal-verify-base-type` | **Date**: 2026-10-01 (revised after the
2026-10-01 clarifications, the text and neighbour probe, and the second clarification
round) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-formal-verify-base-type/spec.md`

## Summary

The pilot proves, for **all** inputs, that `s2::CellId`'s operations never panic or overflow.
It also proves that they agree with an independent reference model of the S2 cell hierarchy,
covering validity, hierarchy, ranges and containment, traversal, encodings, common ancestor,
and neighbours. The technique is exhaustive bounded model checking with **Kani 0.68.0**
(CBMC 6.11.0).

The proofs live in a new standalone workspace, `verify/`, modelled on the existing `fuzz/`
layout. It depends on `s2rst` by path and uses only its public API, so the proofs need no
changes to `core/`. Mutant patches show that every functional property can fail
(non-vacuity), and cover checks do the same for the crash-freedom proofs. A `make verify`
target and a path-filtered CI workflow keep the proofs current without touching `make test`.
Every Kani run is memory-capped, so a runaway proof fails one harness instead of exhausting
the machine (R15).

Beyond the proofs, the feature makes four small, separately committed changes to `core/`,
`fuzz/` and `tools/refgen-cpp/`:

- **FR-015**: `CellId::from_token` stops accepting a leading `+` (R11). Differential tests
  check it against new C++ reference vectors for tokens and debug strings (R16).
- **FR-016**: invalid ids print as `Invalid: ` plus 16 zero-padded hex digits, as in C++
  (R13). A C++ `ToString` vector section checks it (R16).
- **FR-017**: robustness tests and a fuzz target for the three text parsers (R14).
- **FR-003 / SC-011**: `# Panics` rustdoc for every operation the proofs show can panic,
  in a documentation-only commit (R17).

Any undefined or wrong behaviour found in the upstream reference becomes a follow-up task
before the affected proof is completed (FR-018, R18). Two are already known: C++ `level()`
(U-001) and C++ `GetCommonAncestorLevel` (U-002).

Feasibility was measured in two memory-capped spikes, R0 and the 2026-10-01 probe (R6, R12):

| Harness | Time | Memory |
|---------|------|--------|
| Hierarchy proofs over all 2^64 inputs | about 0.5 s each | small |
| The table-driven (face, i, j) round trip over every leaf cell | 28 s | — |
| Text parsers over every byte string of length L ≤ 17 / 33 | 0.2–14 s per length | ≤ 0.5 GB |
| `edge_neighbors`, all valid cells, through the float wrap path | 411 s | 2.5 GB |
| `vertex_neighbors`, all valid cells | 638 s | 1.9 GB |
| `to_token`, `to_debug_string`, `Display` (`core::fmt`, `String` growth) | > 5 min, or OOM above 6 GB | — |
| `all_neighbors`, gap 0 | OOM above 9.5 GB in 105 s | — |

Probe outcomes:

- **The parsers are proven within their byte-length bound.** They use a byte-level text
  domain, which needs one audited `unsafe` helper in `verify/`, approved by the user (R6).
- **Excluded as tool limits:** the three formatters, and `all_neighbors` (crash-freedom
  intractable, under the FR-002 neighbour clause). That leaves 48 of 64 inventory entries
  in scope, exactly the SC-002 floor.
- **`vertex_neighbors` is the main remaining risk.** If it can't be split under the R15
  budget, SC-002 is missed (R12).

## Technical Context

**Language/Version**: Rust. `verify/` uses edition 2024 to match `core/`. Proofs run on Kani's
bundled nightly toolchain, installed by `cargo kani setup`. The reference-model sanity tests,
the regression tests and the robustness tests build on the repository's pinned stable
toolchain. The fuzz target builds on nightly, like the existing ones.

**Primary Dependencies**: Kani 0.68.0 (`kani-verifier`), CBMC 6.11.0, and the default CaDiCaL
SAT solver. Kissat, MiniSat and the SMT backends were tried and did not do better (R6, R12).
`s2rst` is a path dependency. **No new dependencies are added to `core/`.** The fuzz target
uses the existing `libfuzzer-sys` dependency of `fuzz/`.

**Storage**: N/A. Artifacts are Markdown and Rust files in `verify/`.

**Testing**:
- **Proofs:** `cargo kani`, through `verify/run-verify.sh`.
  - Kani 0.68.0 exits 0 even when a `kani::cover!` is unsatisfiable, so the script fails
    the run itself in that case (R7).
  - The script also applies the memory guard and reports `OOM` (R15).
  - For each failed harness, it prints a concrete counterexample through concrete playback
    (R10; FR-007, US3).
- **Reference model:** `cargo test` in `verify/` checks it against the committed C++ vectors.
- **Non-vacuity:** `make verify-mutants`.
- **The `core/` and `fuzz/` changes:**
  - regression tests in `core/src/s2/cell_id_tests.rs` (FR-015, FR-016);
  - differential tests in `core/tests/cpp_cellid_diff.rs` against three new sections of
    `cpp_cellid_vectors.csv`, regenerated locally with `tools/refgen-cpp`. CI needs no C++
    (R16);
  - bounded robustness tests `text_parsers_*` in `core/tests/decoder_robustness.rs`
    (FR-017);
  - the fuzz target `parse_cell_id_text` (FR-017).
- **Gate:** `make test` keeps its definition. It simply runs the new core tests.

**Target Platform**: Linux x86_64, both for development and on GitHub `ubuntu-latest`. The
SC-005 reference runner has 4 vCPUs and 16 GB (clarification 2026-10-01). On a smaller runner,
`VERIFY_JOBS` adapts to it, and the evaluation reports both times. Kani also supports macOS,
but the memory guard is Linux-only, and CI does not use macOS.

**Project Type**: A verification harness crate for a library, in a standalone workspace
alongside `core/`, `python/`, `wasm/`, and `fuzz/`.

**Performance Goals** (R15):
- `make verify` takes under 15 min wall-clock on the 4-vCPU / 16 GB reference runner with
  `-j 4` (SC-005). `VERIFY_JOBS` defaults to what the machine's cores and memory allow.
- The whole run fits in 45 CPU-minutes. The projection is about 34 CPU-minutes, plus
  P-NBR-02.
- Each harness targets 60 s. The soft limit is 5 min. The hard limit is 12 min on CI (8 min
  on the workstation until CI is measured) and 4 GB.
- Over a hard limit, a harness is split. If splitting doesn't help, it moves to the weekly
  slow tier (`make verify-slow`, 30 min per harness; user decision 2026-10-01). Over that
  limit too, it is a tool-limit result (FR-007).
- Implementation outcome: `vertex_neighbors`' crash proof and all neighbour properties are
  in the slow tier (research R12).

**Constraints**:

- **`core/`:** no changes to `core/Cargo.toml`. The only `core/` changes are:
  - the FR-015 fix (`CellId::from_token`);
  - the FR-016 fix (`CellId::to_debug_string`);
  - their regression tests in the new `core/src/s2/cell_id_tests.rs`;
  - their differential tests in `core/tests/cpp_cellid_diff.rs`, with appended sections in
    `core/tests/data/cpp_cellid_vectors.csv` (R16);
  - the FR-017 robustness tests in `core/tests/decoder_robustness.rs`;
  - the `# Panics` rustdoc sections (R17) (FR-009, SC-006).
- **`tools/refgen-cpp/`:** `generate_cellid.cc` and its README gain the three sections (R16).
  Existing sections must regenerate byte-identically.
- **Proof rules:**
  - No stubbing of the code under test.
  - Unwinding assertions stay on.
  - Partial-domain results are allowed only for input-dependent loop bounds or input-size
    bounds. The parsers' byte-length bound is an input-size bound. Any other domain
    restriction makes the operation a tool-limit exclusion (FR-007, SC-001).
- **Memory:** every Kani run goes through `run-verify.sh`, which applies the memory guard
  (R15):
  - `ulimit -v` per process, `VERIFY_MEM_PER_HARNESS` (default 4 GB);
  - plus a `systemd-run --user` cgroup backstop where available.

  Never run an uncapped `cargo kani` over the whole crate.
- **Workspace rules:**
  - Verification is not added to `make test` (FR-012).
  - `verify/` uses `#![deny(unsafe_code)]`. The one exception is `domain::with_text`,
    compiled under `cfg(kani)` only (R6, Complexity Tracking).
  - SPDX headers go on every new source file (`.rs`, `.sh`, `.toml`).

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 64: 54 methods, 3 free functions, 7 trait impls (checked against `cell_id.rs`) |
| Excluded | 16: 12 floating-point; 3 formatter tool limits (R6); `all_neighbors`, with crash-freedom intractable (R12) |
| In scope | 48, exactly the SC-002 floor. `vertex_neighbors` is at risk (R12) |
| Within bounds | 3 (`from_token`, `from_debug_string`, `FromStr`), 6% of in-scope (SC-001 allows 10%) |
| Crash-freedom harnesses | about 43 `crash::c_*`. The parsers and neighbour operations share their property harnesses (R6, R12) |
| Text harnesses | 86 (`__b<L>` families) |
| Functional properties | 49 (VAL 10, HIER 8, RANGE 6, TRAV 7, ENC 12, ANC 2, NBR 4). 43 count towards SC-003 (data-model §4.1) |
| Mutants | about 45: M-001 (the crash-proof overflow demo), plus at least one per proven property |
| New C++ vector sections | 3 (`CELLID_FROM_TOKEN`, `CELLID_FROM_DEBUG_STRING`, `CELLID_TO_STRING`; R16) |
| `# Panics` sections | at least 15: the 13 `impl Into<Level>` functions (`vertex_neighbors` also for level 30), plus `level` and `common_ancestor_level`. The proofs fix the final set (R17) |
| Upstream findings | 2 known (U-001, C++ `level()`; U-002, C++ `GetCommonAncestorLevel`); more recorded as found (R18) |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Properties are stated against a reference model derived from upstream `s2cell_id.h`/`s2coords_internal.h` (FR-006), not the Rust code. The model is checked against the committed C++ oracle vectors, including the new text sections, so the chain is C++ oracle ⇔ model (sampled) ⇔ implementation (proved). The neighbour wrap path is proven with exact IEEE-754 semantics, on the same algorithm as upstream `FromFaceIJWrap`. The pilot introduces no behavioural deviations, and it removes two: `from_token` accepting a leading `+` (FR-015, R11), and the unpadded `Invalid:` debug string (FR-016, R13). Both fixes get **differential tests** against new refgen-cpp vector sections, as the principle requires wherever an oracle exists (R16). That resolves analysis finding C1. Upstream undefined or wrong behaviour becomes a recorded follow-up (FR-018, R18), never a silent precondition. Other defects found go through the parity process (FR-010). CI needs no C++ or Go toolchain | ✅ Pass |
| **II. Robustness & Memory Safety** | The pilot proves panic-freedom and overflow-freedom, which is this principle made exhaustive. `core/` stays `unsafe_code = "forbid"`. The one `unsafe` call is in the unpublished `verify/` crate, under `cfg(kani)`, and builds proof inputs (R6, Complexity Tracking). The three text parsers take untrusted input: their proofs are bounded by length, and they gain robustness tests in `decoder_robustness.rs` and a fuzz target in the nightly matrix (FR-017, R14). `from_token` and `from_debug_string` keep their upstream return types instead of `Result`, which is a recorded exception (Complexity Tracking). `FromStr` already returns `Result`. Any public function the proofs show can panic without a `# Panics` section gets one, in this feature's documentation-only commit (FR-003, SC-011, R17). That closes the "MUST document when it panics" gap for `CellId`. If the panic also differs from upstream, it is recorded as a `D-NNN` for a separate fix (FR-010) | ⚠️ Pass with recorded exceptions |
| **III. Test-Backed Changes** | Proofs complement the tests and do not replace them. The FR-015 and FR-016 fixes each get a regression test in the new sibling `core/src/s2/cell_id_tests.rs`, plus a differential test, and each test fails without its fix (R16). For FR-016 those tests are the only evidence, because `to_debug_string` is excluded from the proofs (R6, R13). Defects found later get regression tests from `--concrete-playback` (FR-010). Harnesses are verification targets like fuzz targets, not unit tests, so the `<module>_tests.rs` rule does not apply to them. The reference model has its own `cargo test` sanity tests | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding or API changes. Python and wasm pick up both fixes through the core calls. No binding test depends on the old `Invalid:` format or on accepting `+` (checked) | ✅ N/A |
| **V. Measured Performance** | No performance changes to the library. Verification time and memory form an operational budget (SC-005, R15), not a benchmark claim | ✅ N/A |
| **Tech: Toolchain/MSRV** | `core` MSRV 1.88 is untouched. `verify/` is a standalone workspace built only by Kani's toolchain (and stable for the model tests), and is outside the MSRV job | ✅ Pass |
| **Tech: Lints** | `core`'s lint config is untouched. `verify/` declares its own `[lints]`: `unsafe_code = "deny"`, plus `unexpected_cfgs` with `check-cfg = ['cfg(kani)']`. The single `#[allow(unsafe_code)]` and any other `#[allow]` carry a `reason` | ✅ Pass |
| **Tech: Dependencies** | `core` gains no dependencies. `verify/` depends only on `s2rst`. The fuzz target reuses `fuzz/`'s dependencies. `cargo-deny` scope is unchanged | ✅ Pass |
| **Tech: Licensing** | New files carry the two-line SPDX header and an "original, not ported" note. No Google attribution is added | ✅ Pass |
| **Workflow: gate** | `make test` keeps its definition. The new CI workflow calls `make verify` and `make verify-mutants`, so CI and local runs use the same targets | ✅ Pass |
| **Workflow: deep verification** | The new fuzz target `parse_cell_id_text` is added to the `fuzz.yml` matrix, as the constitution requires. Scheduled weekly jobs (proofs + mutants) sit beside the nightly fuzzing | ✅ Pass |
| **Workflow: focused PRs** | PRs SHOULD stay focused on one concern. The FR-015 and FR-016 fixes, the FR-017 coverage and the `# Panics` docs ship with the pilot by user decision (clarifications 2026-10-01). Each is its own commit | ⚠️ Justified exception, see Complexity Tracking |

**Gate result: PASS**, with the three exceptions recorded in Complexity Tracking. That
resolves the analysis findings C1 (the R4 wording) and C2 (untrusted-input parsers).

**Post-design re-check (after Phase 1)**: Still PASS.

- The design artifacts change `core/` only for FR-015, FR-016 and FR-017.
- They keep verification out of `make test`, route CI through Makefile targets, and add the
  new fuzz target to the nightly matrix.
- Crash proofs are recorded in the inventory and get cover-based non-vacuity, so the
  catalogue and SC-003/SC-004 count functional properties only (data-model §3–§5).
- The probe decisions (R6, R12, R15) add no `core/` change. They add one `unsafe` helper in
  `verify/` and four tool-limit exclusions, each with measurements for the evaluation.
- The second clarification round adds three things: the C++ differential tests (R16),
  closing analysis finding C1; the `# Panics` docs commit (R17); and the upstream-findings
  rule (R18). None of them changes behaviour beyond FR-015 and FR-016.

**Spec criteria at risk** (none is a constitution issue, so none blocks the gate):

- **SC-002** sits exactly at its floor, 48/64. It fails if `vertex_neighbors` can't meet
  the R15 budget (R12).
- **SC-003** has one entry of slack, 39 of 43 needed (data-model §4.1).
- **SC-005** depends on P-NBR-02 (R15).

Each fallback is fixed in research. A miss is recorded as a pilot finding, and the spec is
not changed without the user.

## Project Structure

### Documentation (this feature)

```text
specs/001-formal-verify-base-type/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R0–R18 (R6, R12, R15 from the 2026-10-01 probe;
│                        #   R16–R18 from the second clarification round)
├── data-model.md        # Phase 1: reference model, domains, artifact schemas, initial catalogue
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md       # make verify / verify-harness / verify-mutants, memory guard, CI jobs
│   └── harness-conventions.md   # naming, split families, harness shape, budget, mutant patch format
├── checklists/
│   └── requirements.md  # from /speckit-specify
└── tasks.md             # Phase 2 (/speckit-tasks), with a "Follow-ups" section for U-NNN (R18).
                         #   Stale: regenerate after this revision
```

### Source Code (repository root)

```text
verify/                          # NEW standalone workspace (like fuzz/), publish = false
├── Cargo.toml                   # [workspace]; s2rst = { path = "../core" }; [lints] unsafe_code = "deny"
├── README.md                    # what is proven, how to run, how to add a property, memory guard
├── INVENTORY.md                 # FR-002 inventory + crash-freedom results (C-<op>)
├── PROPERTIES.md                # FR-005 catalogue: functional properties only (P-*)
├── MUTANTS.md                   # FR-008 mutant table + recorded outcomes
├── TRUSTED_BASE.md              # FR-013
├── EVALUATION.md                # FR-014 pilot evaluation (measurements from US1 on)
├── known-defects.txt            # harnesses failing on a recorded, not-yet-fixed defect (R9)
├── run-verify.sh                # codegen, then Kani -j under the memory guard; per-harness summary,
│                                #   cover, OOM and known-defect checks (R7, R9, R10, R15)
├── run-mutants.sh               # worktree-based mutant runner (called by make verify-mutants)
├── mutants/
│   └── M-NNN-<slug>.patch       # one defect each, header + unified diff
├── src/                         # harness modules at the crate root, so --exact names match
│   ├── lib.rs                   # #![deny(unsafe_code)]; pub mod spec; #[cfg(kani)] mods
│   ├── spec.rs                  # reference model (plain Rust, always compiled)
│   ├── domain.rs                # #[cfg(kani)] generators: any_valid, any_level, with_text (the one unsafe)
│   ├── crash.rs                 # crash::c_<op> for the integer-only in-scope entries
│   ├── val.rs                   # val::p_val_NN
│   ├── hier.rs                  # hier::p_hier_NN
│   ├── range.rs                 # range::p_range_NN
│   ├── trav.rs                  # trav::p_trav_NN
│   ├── enc.rs                   # enc::p_enc_NN, incl. the parser families p_enc_{06,09,10}__b<L>
│   ├── anc.rs                   # anc::p_anc_NN
│   └── nbr.rs                   # nbr::p_nbr_NN (edge and vertex neighbours; R12 splits)
└── tests/
    └── spec_vs_cpp.rs           # reference model vs core/tests/data/cpp_cellid_vectors.csv

core/src/s2/cell_id.rs           # FR-015 from_token; FR-016 to_debug_string; test-module wiring;
                                 #   # Panics rustdoc (R17)
core/src/s2/cell_id_tests.rs     # NEW: FR-015 and FR-016 regression tests (later FR-010 tests)
core/tests/cpp_cellid_diff.rs    # + diff tests: FROM_TOKEN, FROM_DEBUG_STRING, TO_STRING (R16)
core/tests/data/cpp_cellid_vectors.csv  # + three appended sections (regenerated locally)
tools/refgen-cpp/generate_cellid.cc     # + the three sections; README table updated
core/tests/decoder_robustness.rs # + FR-017 text_parsers_* tests: token / debug-string / FromStr
fuzz/fuzz_targets/parse_cell_id_text.rs  # NEW FR-017 fuzz target
fuzz/Cargo.toml                  # + [[bin]] parse_cell_id_text
fuzz/seeds/parse_cell_id_text/   # NEW seed-NN inputs (tokens, debug strings, edge cases)
.github/workflows/fuzz.yml       # + matrix entry parse_cell_id_text
Makefile                         # + verify, verify-harness, verify-mutants targets
.github/workflows/verify.yml     # NEW: path-filtered PR/push + weekly; calls make targets
```

**Structure Decision**:
- **Standalone workspace.** `verify/` mirrors `fuzz/`. The harnesses use only the public
  `s2rst` API, so `core/` needs no `#[cfg(kani)]` code.
- **Crate-root harness modules.** Kani's `--exact` matches the full module path, so
  `crash::c_parent` resolves only when `crash` sits at the crate root. This was checked on
  Kani 0.68.0; a nested `harness::crash` path failed to match.
- **Stable reference model.** The model (`spec.rs`) is ordinary Rust, and is checked against
  the C++ oracle vectors on stable.
- **Changes outside `verify/`.** Only these commits touch `core/`, `fuzz/`,
  `tools/refgen-cpp/` and `.github/workflows/fuzz.yml`:
  - FR-015 and FR-016, each with its own vector sections;
  - FR-017;
  - the `# Panics` docs.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| Two behaviour fixes (FR-015 `from_token` `+`, FR-016 `Invalid:` padding), the FR-017 robustness coverage and the `# Panics` docs (R17) ship in the pilot's PR, against "PRs SHOULD stay focused on one concern" | The user decided on 2026-10-01 to handle them in this feature. Each fix is a few lines, restores C++ parity, and comes with C++ differential vectors. The docs close a Constitution II MUST. The pilot's text properties are stated against upstream behaviour, so they rely on the FR-015 fix | Separate PRs were the default under FR-010. The user set them aside. Each change stays reviewable as its own commit with its own tests |
| `from_token` (returns `CellId`) and `from_debug_string` (returns `Option`) keep their return types, against Constitution II "decoders and any other API that takes untrusted bytes MUST return a `Result`". `FromStr` already returns `Result<CellId, ()>` | The types mirror upstream C++ (`S2CellId::None()` on bad input), as Principle I requires. Changing them would break the public API (FR-009) and every binding | Returning `Result` was rejected for those reasons. The rest of II (no panic, bounded robustness tests, fuzz target) is met by FR-017 and the proofs |
| One `unsafe` call (`str::from_utf8_unchecked`) in `verify/src/domain.rs`, against this plan's earlier `#![forbid(unsafe_code)]` for `verify/`. Constitution II's forbid covers `core` only | It is the only tractable way to give the text parsers a complete input domain. Each parser reads only bytes, so a proof over every byte string of length L covers every `&str` of that length. Measured at 0.2–14 s per length (R6). The user approved it on 2026-10-01 | Every safe construction (symbolic `char`s, or symbolic bytes validated by `from_utf8`) ran out of memory or time (R6). Excluding the parsers would have dropped in-scope to 45/64, below SC-002. ASCII-only text is a restriction that FR-007 forbids |
