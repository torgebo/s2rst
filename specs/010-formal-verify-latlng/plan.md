# Implementation Plan: Formal Verification of `LatLng`, and `s2::Rect` as a `Region`

**Branch**: `tb/010-frm/1` | **Date**: 2026-10-28 (R0 run the same day) |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/010-formal-verify-latlng/spec.md`

## Summary

The feature proves `s2::LatLng` with Kani as the tenth member of 004's value-type suite, and
instantiates the generic `Region` laws for `s2::Rect`:

- **33 inventory entries**: 26 Kani crash proofs (8 under the function contracts of 008 and the
  coverage review), the 4 `geo-types` conversions by composition, `Deserialize` tested, and
  `Display` and `to_string_in_degrees`'s formatting excluded.
- **21 catalogue entries** for `LatLng` (19 counted for SC-003) against a new `latlng_spec` (a
  pair of angles; validity is membership in the principal ranges; normalisation is the clamp
  and the exact reduction by whole turns), **4 law instances** for `Rect` (P-LREG-01 to -04),
  **one composition** in `Rect`'s catalogue (its point test receives a valid `LatLng`, FR-012)
  and **one lemma** (`Cell::rect_bound` is valid and non-empty).
- **Two new stub kinds**, both in `stubs.rs`: *pure* function contracts (one contract value per
  input, so a restatement can call `sin`, `cos`, `sin_cos` and `asin` again and get the same
  value), and a function contract for a library function, `Cell::rect_bound`, which the
  `Rect` laws need (research R5, R6).

Changes to `core/`, each its own commit with a test that fails without it:

- **FR-017** (K1): `LatLng::normalized` and `Angle::normalized` share one exact IEEE remainder
  by 2π, ties to even, `s1::angle::remainder_2pi` (research R3).
- **FR-018** (K3): `get_distance` clamps before the square root.
- **FR-019** (K4, U-016): `Rect::cap_bound` keeps upstream's choice of cap and then widens it by
  `1e-15` rad and `8ε` of its length². R0 found that the cap around the centre misses points
  too (a point a few ulps inside a vertex), in C++ as in the library, so the widening applies
  to both caps; with it, 0 of 74.6 million adversarial contained points lie outside
  (research R0).
- **FR-021**: a new C++ generator and vector file for `LatLng` and `Rect` as a region.

## Technical Context

**Language/Version**: Rust, edition 2024; Kani's pinned nightly for the proofs; `core/` stays
on MSRV 1.88. The C++ generator builds against the existing s2geometry release build, not in
CI.

**Primary Dependencies**: Kani 0.68.0 (`-Z stubbing`), CBMC 6.11.0. **No dependency changes**:
the exact remainder reuses `core/src/test_util.rs`'s `num-bigint` helper; `geo-types` stays
out of the verification crate (research R8).

**Storage**: N/A. Markdown, Rust and CSV rows.

**Testing**:
- `make verify-values P=latlng` (new part `ll_*`, with `Rect`'s laws in `ll_reg`);
  `make verify-slow F=values P=…` for the weekly members, if any;
- `cargo test` in `verify/`: `latlng_spec_tests`, `values_spec_vs_cpp` (the `LATLNG_*`
  sections), `libm_contracts` (the `Cell::rect_bound` contract), `property_kinds`;
- `make verify-mutants M='M-9*'`;
- `core/src/s2/latlng_tests.rs` (new sibling), `core/src/s2/rect_tests.rs` (FR-019),
  `core/src/s1/angle_tests.rs`, `core/tests/cpp_latlng_diff.rs` (new),
  `core/tests/trait_property_tests.rs`;
- the gate `make test`, unchanged.

**Target Platform**: Linux x86_64, the workstation and GitHub `ubuntu-latest` (4 vCPUs, 16 GB).

**Project Type**: An extension of the verification harness for a library.

**Performance Goals**: the `values-latlng` CI leg under 10 minutes (SC-006). `LatLng`'s
operations are a few float operations each; the costliest expected harnesses are the
distance restatement (four pure contracts) and the `Rect` laws (`Rect::intersects` over two
symbolic rectangles, 24.4 s in 005).

**Constraints**:
- no public API change and no `Cargo.toml` change;
- stubs only as spec FR-003 allows, each named in its harness, the status following them
  (contracts/harness-conventions.md);
- stub tables in `static Mutex`, fixed capacity, element-wise keys;
- booleans compared only with `==`; no `f64 %` in any proof (the reduction's correction step is
  proven on a drift-guarded copy, research R3);
- every run through `run-verify.sh`, one at a time, memory-capped.

**Scale/Scope**:

| Item | Count |
|------|-------|
| Inventory entries | 33: 26 Kani crash proofs, 4 by composition, 1 tested, 2 excluded |
| Catalogue entries | 21 `LatLng` (19 counted), 4 `Rect` law instances, 1 `Rect` composition, 1 lemma |
| Kani harnesses | about 35 per change; no weekly member expected |
| Native contract tests | `Cell::rect_bound` against the vertices (research R6) |
| Mutants | about 34 `M-9NN` |
| New C++ vector sections | 7 (research R9) |
| Upstream records | U-016 (`GetCapBound`, both caps) |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle / rule | Assessment | Status |
|------------------|-----------|--------|
| **I. Upstream Parity** | The description and restatements come from upstream (`s2latlng.{h,cc}`, `s2latlng_rect.cc`), and new C++ rows from a running build check them (R9). FR-017 and FR-018 restore parity (probe K1, K3). FR-019 deviates where upstream breaks its own `Region` contract (U-016), documented in the rustdoc and `UPSTREAM.md`, at the maintainer's choice (spec clarification); the deviation rows are marked | ✅ Pass |
| **II. Robustness & Memory Safety** | Crash freedom over the documented domains; `normalized` returns a valid `LatLng` for every finite input after FR-017. No `unsafe` (stub tables use `Mutex`) | ✅ Pass |
| **III. Test-Backed Changes** | Each fix's test fails without it: `latlng_tests.rs` (new sibling), `rect_tests.rs`, `angle_tests.rs`; the reduction's exactness is a property test against the exact remainder | ✅ Pass |
| **IV. Thin, Synchronized Bindings** | No binding change. Python and wasm inherit FR-017 to FR-019; their tests are run (R10 lists what they pin) | ✅ Pass |
| **V. Measured Performance** | FR-017 adds one comparison, and a second `%` only at an exact tie; FR-019 adds a chord-angle sum per `cap_bound`. Neither is a performance change; the existing `core/benches` are not run (no performance claim) | ✅ Pass |
| **Tech constraints** | No new dependency; SPDX headers on new files; MSRV unchanged | ✅ Pass |
| **Workflow: gate, deep verification** | `make test` unchanged and Kani-free; no new decoder or fuzz target | ✅ Pass |
| **Workflow: verification records (v1.2.0)** | Every harness change updates the inventory, catalogue, index (and HTML), `TRUSTED_BASE.md`, `MUTANTS.md`, `VERIFIED_TYPES.md`, `CORE_TRAITS.md` and `COVERAGE.md` in the same commit | ✅ Pass |
| **Workflow: focused PRs** | FR-017 to FR-021 ship with the feature, each its own commit, as in 001–008 | ⚠️ Justified, see Complexity Tracking |

**Gate result: PASS**, with the exception recorded below.

**Post-design re-check (after Phase 1)**: still PASS.

- `core/` is touched only for FR-017 to FR-019 and documentation, each with its test
  (data-model §6).
- The statements use `latlng_spec`; restatements are kept apart and checked against C++.
- Every stub is a function contract tested against the real function, or a lemma stub with
  its lemmas proven; harnesses name them, and the status follows them.

**Spec criteria at risk:**

- **SC-003** (90% of the counted entries): 19 of 21 entries are counted (P-LLDST-04's symmetry
  within a tolerance needs the functions' values, P-LLDES-01 is outside the checker); the bar is
  18. At risk: the distance restatement (P-LLDST-01, five pure contracts in one harness), not
  measured; the three `normalized` compositions rest on IEEE `fmod`'s exactness, a trusted
  fact.
- **SC-004**: P-LREG-03 and P-LREG-04 need the values of `atan2`, `sin_cos` and `asin`, so they
  are recorded as not proven with that reason (spec FR-010 allows it); SC-004's measurable part
  is the vectors' and the probe's zero misses after FR-019.
- **SC-006**: unmeasured on a hosted runner, as 004–008's legs (a push is needed).

## Project Structure

### Documentation (this feature)

```text
specs/010-formal-verify-latlng/
├── plan.md              # This file
├── research.md          # Phase 0: R0–R12 (R0, the cap-bound probe, 2026-10-28)
├── data-model.md        # Phase 1: description, domains, stubs, catalogue, inventory, changes, mutants
├── quickstart.md        # Phase 1: end-to-end validation guide
├── contracts/
│   ├── verify-commands.md        # P=latlng; CI leg values-latlng; mutants series 9
│   └── harness-conventions.md    # ll_* modules, ll_reg, the new stubs, M-9NN
├── probe/               # survey (cpp/, rust/), R0's capbound probe and results, README
└── checklists/requirements.md
```

### Source Code (repository root)

```text
verify/src/{latlng_spec,latlng_spec_tests}.rs                         # NEW
verify/src/ll_{crash,con,fp,tp,nrm,val,dst,apx,ari,eq,lem}.rs          # NEW
verify/src/ll_reg.rs                                                   # NEW (Rect's Region laws)
verify/src/{lib,value_domain,stubs}.rs; verify/tests/{values_spec_vs_cpp,libm_contracts}.rs
verify/values/{INVENTORY,PROPERTIES,EVALUATION}.md
verify/{PROPERTIES_BY_KIND.md,PROPERTIES_BY_KIND.html,TRUSTED_BASE,MUTANTS,COVERAGE}.md
verify/mutants/M-9NN-<slug>.patch
core/src/s1/angle.rs, core/src/s1/angle_tests.rs          # FR-017 (the shared remainder)
core/src/s2/latlng.rs, core/src/s2/latlng_tests.rs (NEW)  # FR-017, FR-018
core/src/s2/rect.rs, core/src/s2/rect_tests.rs            # FR-019
core/tests/cpp_latlng_diff.rs, core/tests/data/cpp_latlng_vectors.csv   # NEW
core/tests/trait_property_tests.rs                        # FR-011
tools/refgen-cpp/generate_latlng.cc (NEW), CMakeLists.txt, README.md
VERIFIED_TYPES.md, CORE_TRAITS.md, UPSTREAM.md, Makefile (P=latlng)
.github/workflows/verify.yml   # + values-latlng; mutants series 9
```

**Structure Decision**: the proofs join 004's `values` suite with `ll_` modules (research R2);
`Rect`'s law instances are `ll_reg`, in the `LatLng` part, so they run in the `values-latlng` leg
with `Rect`'s other proofs. The new stubs go in `stubs.rs`, as 008's. Changes outside
`verify/` are separate commits.

## Complexity Tracking

| Exception | Why it is needed | Simpler alternative rejected because |
|-----------|------------------|--------------------------------------|
| FR-017 to FR-021 ship in this feature's PR | The proofs' statements rest on the fixed behaviour (the exact reduction, the clamp before the root, the widened bound) | Separate PRs first would hold the proofs back; each change stays its own commit, as in 001–008 |
| A function contract on a library function (`Cell::rect_bound` contains its vertices' `LatLng`s) | Rect's vertex law needs the relation between a cell's bound and its vertices, which goes through `atan2`'s value on both sides (R6) | Proving it on the real code needs `atan2`'s value; leaving P-LREG-02 unproven would lose the one law that holds by construction once the relation is granted, and the relation is tested on every cell to level 5 and 10⁶ random cells |
| Widening the cap around the centre as well as the pole cap (beyond the clarified "widen the pole cap") | R0 found the cap around the centre misses points next to a vertex, upstream too; FR-019's own clause makes that a finding to fix | Widening only the pole cap leaves 489,465 of R0's 74.6 million contained points outside the bound |
