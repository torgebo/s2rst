# Implementation Plan: Formal Verification of `PointRegion`, and `CellUnion`'s Floating-Point Methods

**Branch**: `tb/011-frm/1` | **Date**: 2026-10-28 (R0 run the same day) |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/011-formal-verify-point-region-cell-union/spec.md`

## Summary

The feature proves `PointRegion` with Kani as the eleventh member of 004's value-type suite,
brings `CellUnion`'s nine floating-point inventory rows of 002 into scope, and adds the two
`Region` invariants that had no generic law:

- **`PointRegion`**: 10 inventory entries, all with Kani crash proofs (`rect_bound` under
  `atan2`'s contract, `cell_union_bound` under the new `CellId::from_point` contract, weekly), 15
  catalogue entries; five of the six `Region` laws proven (two of them weekly), and the vertex
  law split by face in the weekly tier, where R0's unsplit harness timed out (research R0, R6).
- **`CellUnion`**: crash proofs within 002's size bounds for six of the nine rows, three
  inherited tool limits (`exact_area`, the covering, `expand_by_radius`'s expansion); 17
  catalogue entries, of which the bounds' containment of each cell's bound, the point test, the
  areas and containment-implies-intersection are proven, and the other `Region` laws are
  recorded with reasons (they wait on `Cell`'s own bounding laws or `asin`'s value). The
  vertex law is recorded as not a law of `CellUnion`, with the probe's counterexample.
- **Two new generic laws** in `laws::region` (FR-010): the bounding rectangle and the covering
  contain every contained point.
- **New stubs** (`stubs.rs`): a pure `CellId::from_point` contract (any valid leaf, one per
  point, resting on a new weekly lemma), uninterpreted `Cell::cap_bound`, `CellId::to_point`,
  cell areas and `average_area_for_level` (research R5). R0 found the checker treats `powi` as
  it treats `sqrt` (KI-013).

Changes to `core/`, each its own commit with a test that fails without it:

- **FR-017** (K1): `Cell::rect_bound`'s polar latitude is upstream's `asin(sqrt(1/3))`.
- **FR-018** (K2): the empty union's areas are +0.
- **FR-019** (K3, K4): `PointRegion`'s precondition and the debug panics R0 found documented;
  its deserialiser rejects a point that is not unit length; `expand_by_radius`'s NaN panic
  documented.
- **FR-021**: a new C++ generator and vector file for both types.

## Technical Context

**Language/Version**: Rust, edition 2024; Kani's pinned nightly for the proofs; `core/` stays
on MSRV 1.88. The C++ generator builds against the existing s2geometry release build, not in
CI.

**Primary Dependencies**: Kani 0.68.0 (`-Z stubbing`), CBMC 6.11.0. **No dependency changes.**

**Storage**: N/A. Markdown, Rust and CSV rows.

**Testing**:
- `make verify-values P=regions` (new part: `pr_*` and `uf_*`); `make verify-slow F=values
  P=…` for the weekly members; `make verify-release` for the new `n_`/`o_` members;
- `cargo test` in `verify/`: `region_spec_tests` (the drift guards), `libm_contracts` (the new
  contracts on real points and cells), the enumeration of `average_area_for_level`,
  `property_kinds`;
- `make verify-mutants M='M-95* M-96* M-97*'`;
- `core/src/s2/cell.rs` tests (FR-017), `core/src/s2/cell_union_tests.rs` (FR-018, FR-019),
  `core/src/s2/point_region.rs` tests (FR-019), `core/tests/cpp_region_diff.rs` (new),
  `core/tests/trait_property_tests.rs` (FR-010, FR-013);
- the gate `make test`, unchanged.

**Target Platform**: Linux x86_64, the workstation and GitHub `ubuntu-latest` (4 vCPUs, 16 GB).

**Project Type**: An extension of the verification harness for a library.

**Performance Goals**: the `values-regions` CI leg under 10 minutes (SC-006). R0's per-change
members sum to about 1,000 s of harness time (4 jobs, so about 5 minutes with the build); the
members above 300 s go weekly: `PointRegion`'s covering (563.4 s, 452.8 s), the vertex law,
and `CellUnion`'s three-cell bounds and areas, its two-cell cap bound (research R0).

**Constraints**:
- no public API change and no `Cargo.toml` change;
- stubs only as spec FR-004 allows, each named in its harness, the status following them;
- stub tables in `static Mutex`, fixed capacity, element-wise keys;
- concrete union lengths; unwind 9 for a `Cell`, 11 with a stub table;
- booleans compared only with `==`; no comparison of two `sqrt` or `powi` results;
- every run through `run-verify.sh`, one at a time, memory-capped.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | `PointRegion` 12 (10 Kani, 1 tested, 1 excluded); `CellUnion` 9 rows (6 Kani, 3 inherited limits) |
| Catalogue entries | `PointRegion` 15 (14 counted); `CellUnion` 17 (11 counted) |
| Kani harnesses | about 40 per change, a few weekly |
| Native contract tests | `CellId::from_point` (valid leaf), `Cell::cap_bound` and `CellId::to_point` (in the box) |
| Mutants | about 22, `M-950` … |
| New C++ vector sections | 7 (research R9) |
| Upstream records | none (no upstream defect met) |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | The description comes from upstream (`s2point_region.{h,cc}`, `s2cell_union.{h,cc}`, `s2cell.cc`), and new C++ rows from a running build check it (R9). FR-017 and FR-018 restore parity (K1, K2). FR-019's deserialiser matches upstream's decoder and 008's `Cap` | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom over the documented domains; the debug panics R0 found (upstream's `DCHECK`s) are documented and proven exactly where they happen. No `unsafe` (stub tables use `Mutex`) | ✅ Pass |
| **III. Test-Backed Changes** | Each fix's test fails without it (research R3) | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change. Python and wasm inherit FR-017 to FR-019; their suites are run | ✅ Pass |
| **V. Measured Performance** | FR-017 changes a constant, FR-018 a fold's start value; no performance change, no benchmark claim | ✅ Pass |
| **Tech constraints** | No new dependency; SPDX headers on new files; MSRV unchanged | ✅ Pass |
| **Workflow: gate, deep verification** | `make test` unchanged and Kani-free; no new decoder or fuzz target | ✅ Pass |
| **Workflow: verification records (v1.2.0)** | Every harness change updates the inventories, catalogues, index (and HTML), `TRUSTED_BASE.md`, `MUTANTS.md`, `VERIFIED_TYPES.md`, `CORE_TRAITS.md`, `COVERAGE.md` and `KANI_ISSUES.md` in the same commit | ✅ Pass |
| **Workflow: focused PRs** | FR-017 to FR-021 ship with the feature, each its own commit, as in 001–010 | ⚠️ Justified, see Complexity Tracking |

**Gate result: PASS**, with the exception recorded below.

**Post-design re-check (after Phase 1)**: still PASS.

- `core/` is touched only for FR-017 to FR-019 and documentation, each with its test
  (data-model §6).
- The statements use `region_spec` and 002's `union_spec`; constructions are tied to upstream by
  drift guards and the C++ vectors, not by recomputing products.
- Every stub is a tested contract, an uninterpreted function, or rests on a proven lemma;
  harnesses name them, and the status follows them.

**Spec criteria at risk:**

- **SC-003** (90% of the counted entries): `PointRegion` 14 counted, `CellUnion` 11 counted
  (data-model §4); the bar is 23 of 25. At risk: P-PRREG-03, the vertex law (a 12-minute
  timeout unsplit; weekly by face). If it stays a tool limit it leaves the count (13 of 13 and
  11 of 11 then).
- **SC-004**: `CellUnion`'s bounding laws (P-UREG-02, -04, -05, -06) wait on `Cell`'s own laws
  and `asin`'s value: recorded with reasons, as the spec allows.
- **SC-006**: unmeasured on a hosted runner, as 004–010's legs (a push is needed).

## Project Structure

### Documentation (this feature)

```text
specs/011-formal-verify-point-region-cell-union/
├── plan.md              # This file
├── research.md          # Phase 0: R0–R11 (R0, the Kani probe, 2026-10-28)
├── data-model.md        # Phase 1: description, domains, stubs, catalogue, inventory, changes, mutants
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # P=regions; CI leg values-regions; mutants M-95N…
│   └── harness-conventions.md    # pr_* and uf_* modules, the new stubs and laws
├── probe/               # survey (cpp/, rust/), R0's harnesses and results, README
└── checklists/requirements.md
```

### Source Code (repository root)

```text
verify/src/{region_spec,region_spec_tests}.rs                       # NEW
verify/src/pr_{crash,con,bnd,cel,eq,reg}.rs                         # NEW (PointRegion)
verify/src/uf_{crash,pt,bnd,area,exp,reg}.rs                        # NEW (CellUnion's float half)
verify/src/{lib,stubs,laws}.rs; verify/src/cl_crash.rs (L-PR-01)
verify/tests/{libm_contracts,area_enumeration}.rs
verify/values/{INVENTORY,PROPERTIES,EVALUATION}.md; verify/cell_union/{INVENTORY,PROPERTIES}.md
verify/{PROPERTIES_BY_KIND.md,PROPERTIES_BY_KIND.html,TRUSTED_BASE,MUTANTS,COVERAGE}.md
verify/mutants/M-95N-<slug>.patch
core/src/s2/cell.rs                    # FR-017
core/src/s2/cell_union.rs, core/src/s2/cell_union_tests.rs   # FR-018, FR-019
core/src/s2/point_region.rs            # FR-019
core/tests/cpp_region_diff.rs, core/tests/data/cpp_region_vectors.csv   # NEW
core/tests/trait_property_tests.rs     # FR-010, FR-013
tools/refgen-cpp/generate_region.cc (NEW), CMakeLists.txt, README.md
VERIFIED_TYPES.md, CORE_TRAITS.md, KANI_ISSUES.md, Makefile (P=regions)
.github/workflows/verify.yml   # + values-regions (per change and weekly); paths
```

**Structure Decision**: the proofs join 004's `values` suite as one part, `P=regions`, with
`pr_` modules for `PointRegion` and `uf_` modules for `CellUnion`'s floating-point half (the
latter also gated on `cellunion`, for 002's domains). The new stubs go in `stubs.rs`, the laws
in `laws.rs`. Changes outside `verify/` are separate commits.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| FR-017 to FR-021 ship in this feature's PR | The proofs' statements rest on the fixed behaviour (the +0 sums, the documented panics) and the vectors on the fixed constant | Separate PRs first would hold the proofs back; each change stays its own commit, as in 001–010 |
| A contract on a library function (`CellId::from_point` returns a valid leaf, purely) | The covering law and the union's point test call it twice on one point; its square root is not one value to the checker (KI-007), and it costs 689 s alone | Leaving P-PRREG-06 and P-UPT-01 unproven would lose two laws that hold by construction once the leaf is one value; the contract rests on a weekly Kani lemma and a native test |
| `FR-017` changes `Cell`, spec 004's type, inside spec 011 | The probe found it through `CellUnion::rect_bound`, which this feature verifies against C++ | Deferring it would leave `cpp_region_diff`'s rows for the polar faces as deviations |
