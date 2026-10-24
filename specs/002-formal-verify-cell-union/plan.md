# Implementation Plan: Formal Verification of `CellUnion`

**Branch**: `002-next-verification/1` | **Date**: 2026-10-02 (revised after the R0 probe,
the user's decision and a consistency pass) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-formal-verify-cell-union/spec.md`

## Summary

The feature proves that `s2::CellUnion`'s integer operations never panic or overflow, and
that they agree with an independent model of what a union means, the set of leaf cells it
covers. The model covers normalisation, membership, set algebra, range construction,
denormalisation, the leaf count, and the binary encoding, including decoding untrusted
bytes. The tool and harness setup are 001's: Kani 0.68.0 (CBMC 6.11.0), the `verify/`
crate, the memory-guarded runner, mutants for non-vacuity, and a path-filtered CI
workflow. The new proofs sit behind a default Cargo feature, `cellunion`, so each suite can
run alone (R1).

A union has no fixed length, so every proof is bounded. The planning probe (R0) measured
how far Kani reaches within 001's budget of 4 GB and 12 minutes per harness:
- **Read-only operations** stay cheap at 3 cells or more: the queries, the validity and
  normalisation checks, the leaf count, the accessors and encoding.
- **Normalising** needs about 14 GB at 3 cells, so it is proven for inputs of up to 2
  cells. The 4-cell sibling merge is out of reach.
- **Union and intersection** fit at 1 × 1 cells.
- **`difference` and the three range constructors** do not fit even at one cell, so they
  become tool-limit exclusions. R17's new quickcheck properties and the C++ vectors cover
  them.

After the probe, the user decided to proceed on exactly these terms (clarification
2026-10-02). The spec's SC-001 and SC-002 were amended to the measured bounds. Every
proof uses concrete lengths (`__n<k>` and `__b<L>` families) and the smallest unwind bound
that passes. Both choices cost two to three orders of magnitude when made wrongly (R0, R4,
R5).

The feature also makes four separately committed changes to `core/`, `python/` and
`tools/refgen-cpp/`:

- **FR-015**: `denormalize` clamps the rounded level to 30, as C++ does, instead of
  panicking (R9).
- **FR-016**: documentation only. The verbatim constructor and the leaf count state
  upstream's valid-union precondition. The leaf count gets a `# Panics` section and notes
  its signed result type as a deviation (R10).
- **FR-017**: decoding keeps the identifiers as encoded and rejects input that is not a
  valid union. Its error messages keep their text but are formatted only when displayed,
  because `format!` over decoded values put `core::fmt` into every decoding proof (R0,
  R11).
- **FR-003 / SC-011**: `# Panics` rustdoc for every operation the proofs show can panic
  (R14).

FR-015 and FR-017 are checked against C++ by a new reference-vector generator,
`generate_cellunion.cc`, and its committed CSV (R13). The same vectors sanity-test the
model. One upstream finding is known in advance: U-003, C++ `Decode` accepting invalid
unions (R12).

## Technical Context

**Language/Version**: Rust, edition 2024. The proofs build with Kani's bundled nightly
toolchain. The model's sanity tests, the regression tests and the differential tests build
with the pinned stable toolchain. The C++ generator builds against the local s2geometry
install (`/tmp/s2-install`) and is not part of CI.

**Primary Dependencies**: Kani 0.68.0, CBMC 6.11.0, CaDiCaL, all unchanged from 001.
`s2rst` by path. **No new dependencies** in `core/`, `verify/`, `python/` or `wasm/`.

**Storage**: N/A. The artifacts are Markdown, Rust and one CSV.

**Testing**:
- **Proofs:** `make verify-union` (`run-verify.sh --harness cu_`). `make verify` runs both
  suites.
- **Property tests:** new quickcheck properties in `core/tests/property_tests.rs` for
  the sizes and operations the proofs cannot reach (R17).
- **Model:** `cargo test` in `verify/` runs `tests/union_spec_vs_cpp.rs` against
  `cpp_cellunion_vectors.csv`.
- **Non-vacuity:** `make verify-mutants`, now with the `M-1NN` patches.
- **Changes to `core/`:**
  - regression tests in the new `core/src/s2/cell_union_tests.rs`;
  - differential tests in `core/tests/cpp_cellunion_diff.rs`;
  - updated `cellunion` seeds in `core/tests/decoder_robustness.rs`.
- **Bindings:** two tests in `python/tests/test_encoding.py` (an unsorted encoding raises,
  and four sibling cells round-trip), and one in `wasm/tests/web.rs` (an unsorted encoding
  throws).
- **Gate:** `make test`, definition unchanged.

**Target Platform**: Linux x86_64, both on the workstation and on GitHub `ubuntu-latest`.
The SC-005 reference runner has 4 vCPUs and 16 GB.

**Project Type**: An extension of the verification harness crate for a library.

**Performance Goals** (R15):
- The `CellUnion` per-change suite runs in under 10 min wall-clock on the reference runner,
  within 35 CPU-minutes.
- The `CellId` suite keeps its 15 min and 45 CPU-minutes.
- Per harness: a 60 s target, a 5-minute soft limit, a 12-minute hard limit (8 minutes on
  the workstation) and 4 GB.

**Constraints**:
- **`core/`:** changes only for FR-015 (`cell_union.rs`), FR-017 (`encoding.rs`), the
  rustdoc for FR-016 and R14, the new `cell_union_tests.rs` and its wiring, and the new
  test and data files under `core/tests/`. No change to `core/Cargo.toml`.
- **`python/` and `wasm/`:** tests only. The binding code is unchanged. The Python stub is
  unchanged too, because no signature changes.
- **Proof rules:** 001's. No stubbing, unwinding assertions on, and partial domains only
  through a stated size, loop or recursion bound (FR-007). Every Kani run goes through
  `run-verify.sh` and its memory guard. Separate verification jobs run one after another,
  not side by side.
- **`verify/`:** `#![deny(unsafe_code)]` is unchanged. The new generators need no
  `unsafe`: bytes are already a byte domain, and no `&str` is involved.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 42: 29 inherent methods and 13 hand-written trait methods (checked against `cell_union.rs` and `encoding.rs`) |
| Excluded | 15: 9 floating-point; `expand_at_level`, which depends on the excluded `all_neighbors`; 5 tool limits: `Display` (token formatter) plus `difference`, `from_begin_end`, `from_min_max` and `from_range`, measured in R0 |
| In scope | 27 (64%; amended SC-002 needs 60%). `S2Decode` is conditional on R11 (SC-001) |
| Functional properties | 27 (data-model §4.1). 23 have all their operations in scope, and SC-003 needs 21 of them |
| Harnesses | about 140, mostly `__n<k>` and `__b<L>` family members (R4, R15) |
| Mutants | about 25 (`M-100` … `M-159`) |
| New quickcheck properties | about 8, for the sizes and operations the proofs cannot reach (R17) |
| New C++ vector sections | 7 in a new file (R13) |
| `# Panics` sections | about 12 candidates (R14); the proofs fix the final set |
| Upstream findings | 1 known (U-003); more recorded as found |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | Properties are stated against a model of the covered set, written from `s2cell_union.h`/`.cc` (FR-006), not from the Rust code. The four tool-limit operations are checked against the same C++ vectors by differential tests and against the covered-set meaning by quickcheck properties (R13, R17). The model is sanity-tested against new C++ vectors (R13). FR-015 removes a deviation (the `denormalize` panic). FR-017 replaces an undocumented deviation (normalising on decode) with a smaller, documented one: rejecting input that is not a valid union. The user chose it on 2026-10-02, and it is documented in the decoder's rustdoc as the principle requires. FR-016 documents upstream's own precondition, and the signed result type is documented as a deviation. Both behaviour fixes get **differential tests** against the C++ oracle. U-003 is recorded as a follow-up, not silently absorbed | ✅ Pass |
| **II. Robustness & Memory Safety** | The core purpose: crash-freedom proofs, including every byte string up to the bound given to the decoder. `core/` stays `unsafe_code = "forbid"`, and `verify/` adds no `unsafe`. The decoder already returns `Result` and already has robustness tests and a fuzz target. Both are kept and extended (R16). The `# Panics` gaps (none documented today) are closed in one documentation-only commit (R14) | ✅ Pass |
| **III. Test-Backed Changes** | FR-015 and FR-017 each get regression tests that fail without the fix, in the new sibling `cell_union_tests.rs` (the required `<module>_tests.rs` layout). Each also gets a differential test. The decoder change, visible in both bindings, gets a pytest and a wasm-bindgen test | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding code changes. Python's `decode_cell_union` and wasm's `CellUnion.decode` inherit FR-017, and Python's `denormalize` inherits FR-015. No signature changes, so the stub is unchanged. A pytest and a wasm-bindgen test cover the visible decoder change | ✅ Pass |
| **V. Measured Performance** | No performance change. FR-017 adds one comparison per decoded id on a path that already reads 8 bytes per id. The verification budget is operational, not a benchmark claim | ✅ N/A |
| **Tech: Toolchain/MSRV** | `core` MSRV 1.88 is untouched. The new code uses only stable features available in 1.88 | ✅ Pass |
| **Tech: Lints** | `core`'s config is untouched. `verify/` keeps its `[lints]`, and every new `#[allow]` carries a `reason` | ✅ Pass |
| **Tech: Dependencies** | None added. `cargo-deny` scope unchanged | ✅ Pass |
| **Tech: Licensing** | New files carry the two-line SPDX header. Original files carry the "written for this crate" note. `generate_cellunion.cc` follows `generate_cellid.cc`'s header | ✅ Pass |
| **Workflow: gate** | `make test` is unchanged and runs the new ordinary tests. CI calls the Makefile targets `verify-cellid`, `verify-union` and `verify-mutants` | ✅ Pass |
| **Workflow: deep verification** | No new fuzz target is needed. `decode_cell_union` exists and keeps its assertion (R16) | ✅ Pass |
| **Workflow: focused PRs** | PRs SHOULD stay focused. The FR-015 and FR-017 fixes and the `# Panics` and FR-016 docs ship with the feature, by the spec's design (FR-010 exception) and the user's decision, as in 001. Each is its own commit | ⚠️ Justified exception, see Complexity Tracking |

**Gate result: PASS**, with one exception recorded in Complexity Tracking.

**Post-design re-check (after Phase 1)**: Still PASS.
- **`core/` changes.** The design touches `core/` only for FR-015, FR-016, FR-017 and
  R14.
- **Verification boundaries.** Verification stays out of `make test`, CI uses Makefile
  targets, and no stubs or `unsafe` are added.
- **The FR-017 deviation.** It is documented in rustdoc (Principle I) and recorded in the
  evaluation. U-003 explains why upstream behaviour is not the target there.
- **Proof domains (FR-003).** Each domain is the operation's documented one, checked
  against upstream. `denormalize` uses `ValidUnion`, because upstream `Denormalize`
  states no normalisation precondition. Every generator has a concrete length (R4), and
  every property is stated against the model, never against another implementation
  method (FR-006).

**Spec criteria at risk** (none is a constitution issue):
- **SC-001, decoding.** R0 could not measure decoding before the R11 message change.
  If it still does not fit, decoding becomes a tool-limit exclusion, as the amended
  SC-001 allows. SC-002 then sits at 62%. SC-003 loses P-UENC-02/03/04 from its count
  and needs 18 of the remaining 20 (data-model §4).
- **SC-001, "to confirm" rows.** R4 lists four estimated bounds (for example
  `intersection_with_cell_id` at 2 cells). They are measured first, and lowered to what
  fits if necessary.
- **SC-005.** The projection is about 30 CPU-minutes without decoding, against a budget
  of 35 (R15). Decoding's longest members may need the weekly tier.

## Project Structure

### Documentation (this feature)

```text
specs/002-formal-verify-cell-union/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R0–R16 (R0 is the 2026-10-02 probe)
├── data-model.md        # Phase 1: union model, domains, artifact schemas, initial catalogue
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md       # make verify / verify-cellid / verify-union, CI legs
│   └── harness-conventions.md   # cu_* modules, __n/__b/__d/__g families, generators
├── checklists/
│   └── requirements.md  # from /speckit-specify
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
verify/
├── Cargo.toml                   # + [features] default = ["cellunion"], cellunion = []
├── run-verify.sh                # + CBMC's own out-of-memory exit reported as OOM (R18)
├── README.md                    # + CellUnion section, new targets
├── TRUSTED_BASE.md              # + CellUnion section (reliance on 001, bounds)
├── MUTANTS.md                   # + M-1NN rows (regenerated by run-mutants.sh)
├── cell_union/                  # NEW per-type records
│   ├── INVENTORY.md             # FR-002 (42 rows) + crash results
│   ├── PROPERTIES.md            # FR-005 catalogue (P-U*)
│   └── EVALUATION.md            # FR-014
├── mutants/M-1NN-<slug>.patch   # NEW, about 25
├── src/
│   ├── lib.rs                   # + pub mod union_spec; cfg(all(kani, feature = "cellunion")) mods
│   ├── union_spec.rs            # NEW model of a union (always compiled)
│   ├── union_spec_tests.rs      # NEW hand-made unit tests of the model (Constitution III)
│   ├── union_domain.rs          # NEW generators
│   ├── cu_crash.rs              # NEW crash harnesses
│   └── cu_{acc,inv,norm,mem,set,range,den,meas,enc}.rs   # NEW property harnesses
└── tests/
    └── union_spec_vs_cpp.rs     # NEW model vs cpp_cellunion_vectors.csv

core/src/s2/cell_union.rs        # FR-015 denormalize; FR-016 + R14 rustdoc; test-module wiring
core/src/s2/cell_union_tests.rs  # NEW FR-015 / FR-017 regression tests
core/src/s2/encoding.rs          # FR-017 decode; its rustdoc
core/tests/cpp_cellunion_diff.rs # NEW differential tests (R13)
core/tests/data/cpp_cellunion_vectors.csv   # NEW, generated locally
core/tests/decoder_robustness.rs # + cellunion seeds (four siblings, unsorted)
core/tests/property_tests.rs     # + CellUnion quickcheck properties for larger unions (R17)
tools/refgen-cpp/generate_cellunion.cc      # NEW generator
tools/refgen-cpp/CMakeLists.txt  # + add_executable(generate_cellunion)
tools/refgen-cpp/README.md       # + section table
python/tests/test_encoding.py    # + unsorted encoding raises; four siblings round-trip
wasm/tests/web.rs                # + unsorted encoding throws
Makefile                         # + verify-cellid, verify-union; verify runs both
.github/workflows/verify.yml     # + paths; suite matrix; mutants limit 180 min
```

**Structure Decision**:
- **One crate.** The work extends the existing `verify/` crate (R1, R2). The `CellUnion`
  harnesses sit in crate-root `cu_*` modules behind the default `cellunion` feature.
- **Per-type records.** They go under `verify/cell_union/`, and 001's records stay where
  they are.
- **Changes outside `verify/`.** Only these commits touch `core/`, `python/`, `Makefile`,
  `tools/refgen-cpp/` and `.github/workflows/verify.yml`:
  - the FR-015 and FR-017 fixes, each with its tests and vector sections;
  - the FR-016 and R14 documentation;
  - the CI and Makefile wiring.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| Two behaviour fixes (FR-015 `denormalize` clamp, FR-017 decode verbatim with validation) and the FR-016 / `# Panics` docs ship in this feature's PR, against "PRs SHOULD stay focused on one concern" | The spec makes them an explicit exception to FR-010, because the survey found them before the feature. The user chose the FR-017 behaviour on 2026-10-02. The proofs are stated against the fixed behaviour: P-UDEN-01 needs the clamp, and P-UENC-02/03 need the verbatim decode. Each fix is small and comes with C++ differential vectors | Separate PRs first would hold the proofs back, or force them to be stated against behaviour that is about to change. Each change stays reviewable as its own commit with its own tests, as in 001 |
