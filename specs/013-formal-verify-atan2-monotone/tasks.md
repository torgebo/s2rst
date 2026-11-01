---

description: "Task list for a monotone atan2 and the bounding laws it opens (spec 013)"
---

# Tasks: A Monotone `atan2`, and the Bounding Laws It Opens

**Input**: `specs/013-formal-verify-atan2-monotone/` (plan, spec, research R0–R9, data-model,
contracts, quickstart, probe). Style reference: spec 012's `ad_*` modules and records.

**Tests**: the spec asks for the contracts' native tests (FR-002, FR-003) and the twin (FR-011);
the Kani harnesses are implementation tasks.

**Memory safety**: every Kani run through the Makefile or `run-verify.sh`, one at a time; check
`pgrep -fa '[c]bmc'` first. **Upstream rule**: a finding gets U-018. **Defect rule**: a library
defect gets D-010 and the maintainer's go-ahead before a fix.

| Story | Priority | Phase |
|-------|----------|-------|
| US1 the order contract | P1 | 3 |
| US2 `Cell`'s bounding-rectangle law | P1 | 4 |
| US3 `Rect`'s bounding-cap law and the adaptors' | P2 | 5 |
| US4 what else the contract opens | P3 | 5 |
| US5 records, CI, evaluation | P3 | 6 |

## Phase 1: Setup

- [X] T001 Remove the temporary R0 module from `verify/src/lib.rs`; add `mod cr_reg` (values
  feature) and a doc bullet; `P=cellrect` in the `Makefile` (`verify-values` help text too) and
  `verify/Cargo.toml`'s comment.

## Phase 2: Foundational

- [X] T002 The spec 013 section of `verify/src/stubs.rs` (data-model §1): `atan2_order`,
  `sqrt_order`, `sumsq`, `latitude_lemma`, `quadrant_uv_bound`, their logs cleared by `reset`.

## Phase 3: US1 — the order contract

- [X] T003 [P] [US1] `verify/tests/libm_contracts.rs`: `atan2_keeps_quadrant_order` (ten million
  pairs per quadrant rule: near ties, a shared coordinate, signed zeros, subnormals),
  `atan2_anchors_are_exact`, `sqrt_is_monotone`, `sum_of_squares_is_monotone`; heavy ones
  `#[ignore]`d and run in release.
- [X] T004 [P] [US1] `verify/tests/uv_projection.rs`: the quadrant premise's sign assertions
  (research R3).
- [X] T005 [P] [US1] The drift guard `latitude_is_the_librarys` in
  `verify/src/region_spec_tests.rs`.

## Phase 4: US2 — `Cell`'s bounding-rectangle law

- [X] T006 [US2] `verify/src/cr_reg.rs`: `p_creg_02__l0`, `p_creg_02__lat`,
  `p_creg_02__lng_f<0..5>` (weekly, `cfg(feature = "slow")`), with covers (research R4).
- [X] T007 [US2] The twin `region_cell_rect_contains_directions` in
  `core/tests/trait_property_tests.rs`; `make test`.
- [X] T008 [US2] Run `make verify-values P=cellrect` and the weekly members; split if over 10
  minutes.
- [X] T009 [US2] Mutants `M-988` … `M-992` (data-model §4), the generator in `probe/`;
  `make verify-mutants`; every patch applies.

## Phase 5: US3, US4 — the recorded limits

- [X] T010 [US3] [US4] Records of R5–R8: P-CREG-02's normalised clause, P-LREG-03, P-ADREG-04,
  P-PTDST-02 (sharpened reasons and measurements), P-UREG-05 by composition.

## Phase 6: US5 — records, CI, evaluation

- [X] T011 [US5] Records: `values/PROPERTIES.md` (§1 P-CREG-02, L-CR-01, -02; §7; §9; §13),
  `cell_union/PROPERTIES.md`, the index by kind and its HTML, `TRUSTED_BASE.md`, `MUTANTS.md`,
  `VERIFIED_TYPES.md` §2.14, `CORE_TRAITS.md`.
- [X] T012 [US5] CI: `values-cellrect` per change and weekly, the path filters.
- [X] T013 [US5] The spec-013 part of `verify/values/EVALUATION.md`.

## Phase 7: Polish

- [X] T014 Final run (quickstart), the spec's status, measured values; commit.
- [ ] T015 Live CI check (SC-006): needs a push; **never push**, so it stays open.

## Dependencies

Phase 1 → 2 → 3 → 4 (Kani runs one at a time) → 5 → 6 → 7. T009 after T006–T008. T003–T005 in
parallel.

## Implementation strategy

US1 and US2 are the MVP: the contract, tested, and the law it proves. US3–US5 are records.
