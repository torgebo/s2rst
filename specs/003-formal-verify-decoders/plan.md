# Implementation Plan: Formal Verification of the Decoding Layer

**Branch**: `003-next-specs/1` | **Date**: 2026-10-05 (after the R0 probe and the R1
re-check of upstream) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-formal-verify-decoders/spec.md`

## Summary

The feature proves that the decoding layer never panics, overflows, aborts or loops on any
input within stated bounds, and that every codec meets one decoder contract. The layer is
the integer coding primitives, the integer, cell-id, string and point vectors, and the
binary codecs of `CellId`, `Point`, `Cap`, `Rect` and `Cell`. The contract is: round trip,
exact consumption, sound acceptance, rejection, and bounded reservation. Every property is
stated against an independent reference description of the wire formats, written from
upstream. Two tracks carry the proofs:

- **Bounded (Kani 0.68.0)**, the CI gate. It reuses 001/002's setup: the `verify/` crate,
  the memory-guarded runner, mutants and CI. A new default feature `decoding` gates the
  `cd_*` harnesses (R2). The contract is one generic statement, the `Codec` trait in
  `verify/src/codec.rs`, instantiated per codec (R4).
- **Deductive (Verus 0.2026.09.20)**, for the variable-length formats at every length. It
  runs on restated copies in a new unpublished crate, `verify/deductive/` (R11). Three links
  tie each copy to the library: Kani-checked bounded agreement, tested agreement, and a
  drift guard that fails when a mirrored library function changes (R12).

The planning probe (R0) measured both tracks:

- **Kani**: the fixed-size decoders are proven over every input of their encoded length in
  2–26 s, and `Rect` in 3 s once its error message is formatted lazily. Every vector decoder
  ran out of memory at 4 GiB, even on a 2-byte input. The cause is the same `Vec`
  reservation sized from the decoded count that makes K2 abort. Capping that reservation did
  not help. Growing on demand did: the integer vectors verify at 11 bytes in about 20 s, and
  the cell-id vector in 82 s. The string vector (OOM from 4 bytes) and the point vector (OOM
  at 25 bytes, and its compact branch at 11) remain out of reach at 4 GiB. Following the
  user's choice (spec clarification 2026-10-05), their bounds are set by measurement during
  implementation, with tool-limit exclusions where nothing useful fits. A harness running
  encoder and decoder together does not fit even for one element, so the round trip is proven
  by composition (R4).
- **Verus**: the varint and 64-bit vector round trips, with exact consumption, are proven for
  every length in 3.4 s at 0.33 GB (`probe/uint_vector.rs`, 51 obligations). The restated
  file also builds with stable `rustc` and Kani's nightly through the crates.io `vstd`
  crates. A Kani harness proved the restated decoder and the library's decoder agree on
  every 6-byte input in 27 s.

Re-reading the pinned upstream (R1) withdrew one survey finding. K3 is not a defect,
because upstream adopted the library's string-vector behaviour in December 2025. It also
added two items: the count limits become documented deviations, and the varint tenth byte
is a D-003 candidate. The spec is amended accordingly (R1).

The feature changes `core/` in four separately committed ways:

- **FR-019 (K2)**: the decoders grow their vectors on demand instead of reserving from a
  decoded count. The string vector reads its data through `Read::take`. Regression test:
  the K2 inputs run in a child process under a 256 MiB address-space limit (R7). The
  existing iai-callgrind bench records the instruction-count effect (Constitution V).
- **R6 (FR-013)**: the `Rect` and point-vector decoders format their error messages only
  when displayed, as 002 did for `CellUnion`. The text is unchanged.
- **FR-020 to FR-022 (K1, K3, K4, limits)**: documentation only (R8).
- **FR-023**: three new fuzz targets, `decode_cell_id`, `decode_point` and `decode_rect`
  (R9).

A new C++ generator, `generate_coding.cc`, produces `cpp_coding_vectors.csv`. It checks both
the library and the reference description (R10).

## Technical Context

**Language/Version**: Rust, edition 2024.
- The bounded proofs build with Kani's pinned nightly (`nightly-2026-08-21`).
- The deductive crate verifies with Verus's pinned Rust 1.98.1. It also builds with the
  project's stable toolchain for its tests, because `verus!` erases ghost code there.
- `core/` stays on its MSRV 1.88.
- The C++ generator builds against the local s2geometry install and is not part of CI.

**Primary Dependencies**:
- Kani 0.68.0 / CBMC 6.11.0, unchanged.
- Verus `0.2026.09.20.aef82ed`, with Z3 4.16.0 bundled.
- `verify/deductive/` depends on `vstd`, `verus_builtin` and `verus_builtin_macros`,
  pinned to the matching crates.io versions (all MIT). Its dev-dependencies are
  `quickcheck`, `syn` and `quote`.
- `verify/` gains a path dependency on `s2rst-deductive` (for the agreement harnesses) and a
  dev-dependency on `stats_alloc` 0.1.10 (MIT, for the reservation measurement).
- **No dependency changes** in `core/`, `python/` or `wasm/`.

**Storage**: N/A. The artifacts are Markdown, Rust, TOML and one CSV.

**Testing**:
- **Bounded proofs:** `make verify-decoding` (`run-verify.sh --harness cd_`).
- **Deductive proofs:** `make verify-deductive`.
- **Models and links:** `cargo test` in `verify/`. That covers `wire_spec_vs_cpp` and the
  reservation measurement. `cargo test` in `verify/deductive/` covers tested agreement and
  the drift guard.
- **Non-vacuity:** `make verify-mutants` (`M-2NN`) and `make verify-deductive MUTANTS=1`
  (`V-NNN`).
- **Changes to `core/`:**
  - the reservation regression test in `core/tests/decoder_robustness.rs`;
  - property tests and 1,000,000-element cases in four new sibling `*_tests.rs` files;
  - the differential tests `core/tests/cpp_coding_diff.rs`;
  - display-text tests for the R6 messages.
- **Bindings:** one pytest in `python/tests/test_encoding_rich.py`. It decodes the K2 inputs
  through `decode_s2cell_id_vector` and `decode_s2point_vector` in a subprocess with
  `resource.setrlimit(RLIMIT_AS, 512 MiB)`, and expects `ValueError`, not a crash. wasm
  exposes none of the affected decoders.
- **Measurement:** `make verify-mutation-measure` (R14) and the iai-callgrind
  `encoded_uint_vector` bench before and after FR-019.
- **Gate:** `make test`, definition unchanged.

**Target Platform**: Linux x86_64, both the workstation and GitHub `ubuntu-latest`. The
reference runner has 4 vCPUs and 16 GB.

**Project Type**: An extension of the verification harness for a library, plus a new
deductive-verification crate.

**Performance Goals** (R15):
- The decoding per-change leg runs in under 10 min wall-clock on the reference runner,
  with at most about 150 harnesses.
- The deductive job runs in under 10 min including setup from cache.
- The `CellId` and `CellUnion` legs are unchanged.

**Constraints**:
- **`core/` changes** are limited to:
  - FR-019: `encoded_uint_vector.rs`, `encoded_s2cell_id_vector.rs`,
    `encoded_string_vector.rs` and `encoded_s2point_vector.rs` (grow on demand; no shared
    constant is needed);
  - R6: `encoding.rs` (`Rect`) and `encoded_s2point_vector.rs`;
  - the rustdoc for FR-020 to FR-022;
  - the new sibling test files and their wiring;
  - new test and data files under `core/tests/`.

  No change to `core/Cargo.toml`.
- **Proof rules** (001/002): no stubbing, unwinding assertions on, partial domains only
  through a stated size or loop bound. Every Kani run goes through `run-verify.sh`. Every
  Verus run goes through `run-verus.sh`. Verification jobs run one after another.
- **Deductive rules**: restate statement by statement, with no escape hatches
  (contracts/deductive-conventions.md).
- **`verify/`**: `#![deny(unsafe_code)]` is unchanged, with no new `unsafe`. `stats_alloc`
  needs none in our code. `verify/deductive/` is `forbid(unsafe_code)`.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 24 (data-model §5): 3 helpers, 4 integer-vector operations, 2 cell-id vector, 2 string vector, 3 point vector, 10 fixed-size codec operations |
| Excluded | 1 by design: the compact point-vector encoder. Possible tool limits, measured during implementation (R5): the string-vector decoder and the point vector's compact branch |
| In scope | 23 if both measure in (96%), 22 if the string-vector decoder becomes a tool limit (92%); SC-002 needs 83% |
| Functional properties | 32 (data-model §4.1), 31 of them proofs. 21 carry a deductive track |
| Kani harnesses | about 120–150, mostly `__b<L>` family members (R5, R15) |
| Restated modules | 5 (varint, fixed width, integer vectors, cell-id vector, string vector) |
| Mutants | about 25 `M-2NN`, plus one `V-NNN` per theorem (about 10) |
| New C++ vector sections | 13 (R10) |
| New fuzz targets | 3 |
| Documentation additions | 3 `# Panics`, 2 deviation notes, the count limits, the string-vector rule, `# Errors` for 10 decoders |
| Candidate findings | D-003, U-004, U-005 (R1, R16) |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Properties are stated against a reference description written from upstream's format code (FR-007, R3), checked against new C++ vectors (R10). The library is checked against the same vectors by differential tests. Planning re-read the **pinned** oracle (R1). That withdrew K3, which would have been a new deviation, and found the undocumented count limits, now documented as deviations (FR-022). The `CellId` and `Cap` rejections become documented deviations. D-003 (varint tenth byte) is a parity defect candidate, fixed under FR-014 with its own differential rows | ✅ Pass |
| **II. Robustness & Memory Safety** | This is the core purpose. K2 is an abort on a 5-byte input, which Principle II forbids; FR-019 fixes it with a regression test. Crash-freedom proofs cover every decoder within bounds, plus every length on the restated code. The three missing fuzz targets are added (FR-023). `core/` stays `forbid(unsafe_code)` | ✅ Pass |
| **III. Test-Backed Changes** | The FR-019 regression test fails before the fix (seen first). New unit and property tests go in sibling `<module>_tests.rs` files. The vector round trips get quickcheck properties for sizes beyond the bounds (R13). R6 gets display-text tests. Python gets a test for the visible K2 behaviour | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding code changes. Python's `decode_s2cell_id_vector` and `decode_s2point_vector` inherit FR-019, and a pytest covers it. No signature changes, so the stub is unchanged | ✅ Pass |
| **V. Measured Performance** | FR-019 is a robustness fix, not a performance change. It replaces up-front reservation with growth on demand, a few extra reallocations per vector. The existing iai-callgrind bench (`core/benches/encoded_uint_vector.rs`, 1,024 and 65,536 elements) is recorded before and after, with `make baseline` rules, and the numbers go in the evaluation. Benchmarks stay out of `make test` | ✅ Pass (evidence recorded) |
| **Tech: Toolchain/MSRV** | `core` MSRV 1.88 untouched. Verus's Rust 1.98.1 is a separate tool toolchain, like the nightly used by `fuzz/` and Kani; it builds only `verify/deductive/` | ✅ Pass |
| **Tech: Lints** | `core`'s config is untouched. `verify/` keeps its lints. `verify/deductive/` forbids `unsafe`, and every `#[allow]` carries a `reason` | ✅ Pass |
| **Tech: Dependencies** | None in published crates. The new unpublished crate depends on MIT-licensed Verus crates (on the `deny.toml` allowlist). `verify/` adds MIT `stats_alloc` as a dev-dependency. Neither crate is in the root workspace that `cargo-deny` checks | ✅ Pass |
| **Tech: Licensing** | New files carry the two-line SPDX header and the "written for this crate" note. `generate_coding.cc` follows `generate_cellunion.cc` | ✅ Pass |
| **Workflow: gate** | `make test` unchanged and needs neither Kani nor Verus. CI calls `make verify-decoding` and `make verify-deductive` | ✅ Pass |
| **Workflow: deep verification** | Three new fuzz targets join the nightly matrix | ✅ Pass |
| **Workflow: focused PRs** | The FR-019 fix, R6 and the documentation ship with the feature, each as its own commit, as in 001 and 002 | ⚠️ Justified exception, see Complexity Tracking |

**Gate result: PASS**, with the exceptions recorded in Complexity Tracking.

**Post-design re-check (after Phase 1)**: still PASS.
- **`core/` changes.** The design touches `core/` only for FR-019, R6, FR-020 to FR-022, and
  tests.
- **Verification boundaries.** Kani and Verus stay out of `make test`. CI uses Makefile
  targets. No stubs and no `unsafe` are added. No published manifest changes.
- **Restated code.** It is linked three ways (R12), and the trusted base names it (R17).
- **Proof domains (FR-005).** Every decoder's domain is every byte string. Every encoder's
  domain is every value or sequence. Fixed-size decoders get whole-domain proofs (R5).

**Spec criteria at risk** (none is a constitution issue):
- **SC-001, string and point vectors.** Their bounds are set by measurement (amended SC-001).
  If the string-vector decoder becomes a tool limit, its crash-freedom and contract rest on
  the deductive track, tested agreement and fuzzing. SC-002 stays at 92% or more, and
  SC-003 loses P-SV-02's bounded track but keeps its deductive one.
- **SC-001, `Cap` and `Rect` agreement.** Asserting on the validity predicates' float results
  may be expensive. R18 has the stated fallback.
- **SC-005.** Three modules (fixed width, cell-id vector, string vector) have no probe
  yet. The cell-id vector's base and shift arithmetic is the most effort. The string
  vector's empty-entry rule is a new spec decoder.
- **SC-012.** cargo-mutants is not installed yet. Its run time is an estimate (R14).

## Project Structure

### Documentation (this feature)

```text
specs/003-formal-verify-decoders/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R0–R18 (R0 is the 2026-10-04/05 probe)
├── data-model.md        # Phase 1: reference description, domains, contract, catalogue, inventory
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # make verify-decoding / -deductive / -mutation-measure; CI
│   ├── harness-conventions.md    # cd_* modules, families, generators, the Codec bodies
│   └── deductive-conventions.md  # restated crate, rules, theorems, links, V-mutants
├── probe/
│   └── uint_vector.rs   # R0 Verus probe (varint + u64 vector, 51 obligations); draft for verify/deductive
├── checklists/
│   └── requirements.md  # from /speckit-specify, iteration 2 after R1
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
verify/
├── Cargo.toml                    # + feature `decoding` (default); + s2rst-deductive (path); + dev stats_alloc
├── README.md                     # + decoding section, new targets
├── TRUSTED_BASE.md               # + decoding section (R17)
├── MUTANTS.md                    # + M-2NN rows
├── known-defects.txt             # + D-003 while open
├── decoding/                     # NEW per-suite records
│   ├── INVENTORY.md              # FR-002 (24 rows)
│   ├── PROPERTIES.md             # FR-006 catalogue (P-FW, P-VAR, P-UV, P-CV, P-SV, P-PV, P-FX, P-PAR)
│   ├── MUTATION.md               # FR-018 measurement (before/after)
│   └── EVALUATION.md             # FR-017
├── mutants/M-2NN-<slug>.patch    # NEW, about 25
├── src/
│   ├── lib.rs                    # + pub mod wire_spec; cfg(all(kani, feature = "decoding")) mods
│   ├── wire_spec.rs              # NEW reference description (always compiled)
│   ├── wire_spec_tests.rs        # NEW hand-made unit tests
│   ├── codec.rs                  # NEW Codec trait + generic contract bodies
│   ├── wire_domain.rs            # NEW generators
│   ├── cd_crash.rs               # NEW crash harnesses not shared with properties
│   ├── cd_{fw,uv,cv,sv,pv,fx}.rs # NEW property harnesses
│   └── cd_agree.rs               # NEW bounded agreement with the restated copies
└── tests/
    ├── wire_spec_vs_cpp.rs       # NEW reference description vs cpp_coding_vectors.csv
    └── reservation.rs            # NEW SC-008 measurement (stats_alloc)

verify/deductive/                 # NEW crate s2rst-deductive (publish = false, own workspace)
├── Cargo.toml                    # vstd, verus_builtin, verus_builtin_macros pinned; dev quickcheck, syn, quote, s2rst
├── run-verus.sh                  # guarded Verus runner
├── mirrors.toml                  # drift-guard fingerprints
├── src/{lib,varint,fixed_width,uint_vector,cellid_vector,string_vector}.rs
├── tests/{agree,drift}.rs
└── mutants/V-NNN-<slug>.patch

core/src/s2/encoded_uint_vector.rs        # FR-019; FR-021 rustdoc; test wiring
core/src/s2/encoded_uint_vector_tests.rs  # NEW
core/src/s2/encoded_s2cell_id_vector.rs   # FR-019; FR-022 rustdoc; test wiring
core/src/s2/encoded_s2cell_id_vector_tests.rs  # NEW
core/src/s2/encoded_string_vector.rs      # FR-019; FR-020/FR-022 rustdoc; test wiring
core/src/s2/encoded_string_vector_tests.rs     # NEW
core/src/s2/encoded_s2point_vector.rs     # FR-019; R6; FR-022 rustdoc; test wiring
core/src/s2/encoded_s2point_vector_tests.rs    # NEW
core/src/s2/encoding.rs                   # R6 (Rect); FR-022 rustdoc (CellId, Cap, Point, Rect, Cell)
core/tests/cpp_coding_diff.rs             # NEW differential tests
core/tests/data/cpp_coding_vectors.csv    # NEW, generated locally
core/tests/decoder_robustness.rs          # + reservation regression test (R7)
fuzz/fuzz_targets/decode_{cell_id,point,rect}.rs   # NEW
fuzz/Cargo.toml                           # + 3 [[bin]]
tools/refgen-cpp/generate_coding.cc       # NEW generator
tools/refgen-cpp/CMakeLists.txt           # + add_executable(generate_coding)
tools/refgen-cpp/README.md                # + section table
python/tests/test_encoding_rich.py        # + K2 inputs raise under an address-space limit
Makefile                                  # + verify-decoding, verify-deductive, verify-verus-check,
                                          #   verify-deductive-bless, verify-mutation-measure; VERUS_VERSION
.github/workflows/verify.yml              # + paths; matrix leg `decoding`; job `deductive`
.github/workflows/fuzz.yml                # + 3 targets
```

**Structure Decision**:
- **Bounded proofs extend `verify/`**, behind the `decoding` feature, with crate-root `cd_*`
  modules and per-suite records in `verify/decoding/` (R2).
- **The deductive track is its own unpublished crate**, `verify/deductive/`. Its Verus
  dependencies stay out of every published crate (FR-012). `verify/` depends on it only so
  the agreement harnesses can call the restated code.
- **Changes outside `verify/`** come only in these separate commits:
  - the FR-019 fix with its tests and bench record;
  - R6 with its display tests;
  - the FR-020 to FR-022 documentation;
  - the fuzz targets;
  - the vector generator and CSV;
  - the CI and Makefile wiring;
  - D-003's fix, if the user approves it (R1).

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| The FR-019 fix, R6 and the FR-020 to FR-022 documentation ship in this feature's PR, against "PRs SHOULD stay focused on one concern" | The spec makes the survey findings an exception to FR-014. The proofs are stated against the fixed behaviour: R0 shows the vector decoders are out of the checker's reach until FR-019 removes the count-sized reservation, and `Rect` and the point vector until R6 | Separate PRs first would hold the proofs back, or force them against code that cannot be checked. Each change stays its own commit with its own tests, as in 001 and 002 |
| A second verification toolchain (Verus with Rust 1.98.1) and a new unpublished crate | The user chose a deductive track with its own CI job (spec Clarifications). Removing the size bound needs loop invariants, which Kani 0.68.0 cannot take without annotating `core/` | Verifying `core/` in place would add `vstd` to a published crate (FR-012). Kani loop contracts are unstable and need `core/` annotations. Creusot has no local install or pilot evidence (R11) |
| Restated copies of five codecs, which can drift from the library | Verus verifies only code it compiles inside `verus!`, and moving `core/` there is rejected above | Three links (R12) make drift a CI failure: Kani agreement within bounds, quickcheck agreement beyond them, and token fingerprints of every mirrored function |
