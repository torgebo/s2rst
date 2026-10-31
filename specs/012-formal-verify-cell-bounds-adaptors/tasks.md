---

description: "Task list for Cell's bounding laws and the region adaptors (spec 012)"
---

# Tasks: `Cell`'s Bounding Laws, and the Region Adaptors

**Input**: `specs/012-formal-verify-cell-bounds-adaptors/` (plan, spec clarified 2026-10-30,
research R0–R7, data-model, contracts, quickstart, probe). Style reference: spec 011's `pr_*`
and `uf_*` modules and records.

**Tests**: the spec asks for the documentation changes' tests (FR-016, FR-017), the differential
tests against new C++ vectors (FR-018) and the twins (FR-012); the Kani harnesses are
implementation tasks.

**Memory safety**: every Kani run through the Makefile or `run-verify.sh`, one at a time; check
`pgrep -fa '[c]bmc'` first. **Upstream rule**: U-017 is this feature's; another gets U-018.
**Defect rule**: a new library defect gets D-010 and the maintainer's go-ahead before a fix.

| Story | Priority | Phase |
|-------|----------|-------|
| US1 `Cell`'s bounding laws (records, U-017) | P1 | 3 |
| US2 the adaptors' laws | P1 | 5 |
| US3 the adaptors' crash freedom | P2 | 4 |
| US4 records, CI, evaluation | P3 | 6 |

## Phase 1: Setup

- [X] T001 Add `any_region` and the `ad_*` modules to `verify/src/lib.rs`, the part to
  `verify/Cargo.toml`'s comment, and `P=adaptors` to the `Makefile`.

## Phase 2: Foundational

- [X] T002 `verify/src/any_region.rs` (research R4) and `stubs.rs`'s crate-visible table.
- [X] T003 [P] `tools/refgen-cpp/generate_cell_bounds.cc` and `core/tests/data/cpp_cell_bounds_vectors.csv`
  (research R6).
- [X] T004 [P] The adaptors' drift guard in `verify/src/region_spec_tests.rs` (P-ADBND-02).

## Phase 3: US1 — `Cell`'s bounding laws and U-017

- [X] T005 [US1] FR-016: `Cell::contains_point`, `cap_bound`, `rect_bound` rustdoc; the fixed test
  `cell_tests::bounds_miss_the_margin` (one of the probe's points); `UPSTREAM.md` U-017; commit.
- [X] T006 [US1] FR-017: `RegionIntersection::region`'s `# Panics`, a `should_panic` test; commit.
- [X] T007 [US1] `core/tests/cpp_cell_bounds_diff.rs` (depends on T003); FR-012's twins in
  `core/tests/trait_property_tests.rs` (cells' own points, adaptors of mixed and nested members);
  `make test`; commit.

## Phase 4: US3 — the adaptors' crash freedom

- [X] T008 [US3] `verify/src/ad_crash.rs` (data-model §5): the API, the tests, the bounds (empty
  per change, 1 and 2 members weekly), `n_region_past_end`.

## Phase 5: US2 — the adaptors' meaning and laws

- [X] T009 [P] [US2] `verify/src/ad_mean.rs`: P-ADCON-01, P-ADBND-01, -03, P-ADTST-01.
- [X] T010 [P] [US2] `verify/src/ad_reg.rs`: P-ADREG-01 to -03; P-ADREG-04, -05 and the any-`N`
  argument in its documentation.
- [X] T011 [US2] Run `make verify-values P=adaptors` and the weekly members; split or move members
  if the part is over 10 minutes.
- [X] T012 [US2] Mutants `M-974` … (data-model §7), `make verify-mutants`.

## Phase 6: US4 — records, CI, evaluation

- [X] T013 [US4] Records: `values/INVENTORY.md` §14, `PROPERTIES.md` §13 and §2's `P-CREG` rows,
  `cell_union/PROPERTIES.md` (P-UREG-04, -05), the index by kind and its HTML, `TRUSTED_BASE.md`,
  `VERIFIED_TYPES.md` §2.13, `CORE_TRAITS.md`, `COVERAGE.md`.
- [X] T014 [US4] CI: `values-adaptors` per change and weekly, the path filters.
- [X] T015 [US4] The spec-012 part of `verify/values/EVALUATION.md`.

## Phase 7: Polish

- [X] T016 Final run (quickstart), the spec's status, the quickstart's measured values; commit.
- [ ] T017 Live CI check (SC-006): needs a push; **never push**, so it stays open.

## Dependencies

Phase 1 → 2 → 3 (commits) → 4, 5 (Kani runs one at a time) → 6 → 7. T012 after T008–T011.
