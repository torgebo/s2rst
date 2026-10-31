---

description: "Task list for the formal verification of PointRegion and CellUnion's floating-point methods (spec 011)"
---

# Tasks: Formal Verification of `PointRegion`, and `CellUnion`'s Floating-Point Methods

**Input**: Design documents from `specs/011-formal-verify-point-region-cell-union/`:

- plan.md, spec.md (scope chosen by the maintainer 2026-10-28);
- research.md (R0–R11; R0, the Kani probe, ran 2026-10-28);
- data-model.md; contracts/ (verify-commands, harness-conventions); quickstart.md;
- the probes in `probe/`: the survey (`cpp/survey.cc`, `rust/src/main.rs`; K1–K5), R0
  (`r0_harnesses.rs`, `r0_results.txt`) and the native leaf check (`rust/src/bin/leaf.rs`).

The feature extends the `values` suite of 004–010 in `verify/`. 010's files are the style
reference: `verify/src/latlng_spec.rs`, `ll_*.rs`, `stubs.rs`, `laws.rs`; 002's
`union_domain.rs` and `cu_mem.rs` for unions of concrete length; `tools/refgen-cpp/generate_latlng.cc`
and `core/tests/cpp_latlng_diff.rs`.

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **For T007 only**: the C++ build installed at the scratchpad prefix (`probe/README.md`).
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run (90 GB on 2026-10-28).

**Tests**: The spec asks for these ordinary tests, so they appear as tasks: the regression
tests of FR-017 to FR-019 (each failing without its change), the differential tests against
the new C++ vectors (FR-021), the description's unit tests and drift guards (FR-007), the
native tests of the new contracts (FR-004), the enumeration of `average_area_for_level`
(P-UAREA-04), and the quickcheck twins of the two new laws (FR-010, FR-013). The Kani
harnesses are the verification deliverable, so they are implementation tasks.

**Organization**:

| Story | Priority | Phase |
|-------|----------|-------|
| US4: parity restored (the library changes) | P2 | 3 (first: the proofs rest on it) |
| US1: proven crash freedom | P1 | 4 |
| US2: the `Region` laws for `PointRegion` (and its catalogue) | P1 | 5 |
| US3: `CellUnion`'s floating-point methods and laws | P2 | 6 |
| US5: CI, records and evaluation | P3 | 7 |

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks). For Kani
  **runs**, "parallel" applies only to writing the code; the runs go one after another.
- **[Story]**: the user story the task belongs to (US1–US5).

## Memory safety (read first)

- **Every** Kani run goes through a Makefile target or `verify/run-verify.sh`:
  `make verify-values P=regions`, `make verify-harness H=…`, `make verify-slow F=values P=…`,
  `make verify-mutants M=…`. Exploratory runs set `VERIFY_PLAYBACK=0` (research R0).
- **One run at a time**; check `pgrep -fa '[c]bmc'` first, and never kill another run.
  `verify/` must compile at every save while a run is going.

## Upstream findings rule (FR-022, read first)

None so far (research R1). One found later gets the next free **U-017**, a task in Phase 8, an
entry in `UPSTREAM.md` and the spec-011 evaluation, and a citation in the affected harness.
Never change upstream code, never file upstream.

## Known-defect rule (FR-015, read first)

- A proof that fails on a **real library defect** gets the next free **D-010**, recorded under
  "Defects found" in the spec-011 evaluation. K1–K5 are the survey's, handled by FR-017 to
  FR-020.
- **Ask the user before fixing** a defect not in the spec or this plan.
- **Replay a failure natively before believing it** (CBMC's `cos`, `log2`, `%`, `x / ∞`, and
  two `sqrt` or `powi` results, KI-007, KI-013); with stubs, look first for a fact the stub lacks.

## Facts these tasks rely on

1. **`PointRegion`** (`core/src/s2/point_region.rs`): struct with a private `point`, derived
   `Clone, Copy, Debug, PartialEq` and serde (lines 17–22); `new` 26, `point` 31; `Region` 36–64;
   inline tests from 66. Python: `python/src/regions_extra.rs:57–80`.
2. **`CellUnion`** (`core/src/s2/cell_union.rs`): `contains_point` 249; `expand_by_radius`
   454–472; `average_based_area` 481, `approx_area` 493, `exact_area` 508 (K2: `.sum()`);
   `Region for CellUnion` 722–770 (the bounds' `# Panics` above it). Tests:
   `cell_union_tests.rs` and the inline module from 775.
3. **`Cell`** (`core/src/s2/cell.rs`): `POLE_MIN_LAT` 33 (K1), used at 583 and 595 in
   `rect_bound` (540–600); `contains_point` 503 (K4); `average_area_for_level` 456 (`powi`).
4. **The debug checks of K4**: `core/src/s2/coords.rs:532` (`valid_face_xyz_to_uv`);
   `core/src/s1/interval.rs:99` (`from_point`, through `new_checked`'s `is_valid_pair`);
   `core/src/s2/metric.rs:94` (`max_level`'s NaN check); `CellId::from_point`'s `# Panics`
   (`core/src/s2/cell_id.rs:258–265`); `Rect::from_lat_lng` (`core/src/s2/rect.rs:231`).
5. **Upstream**: `s2point_region.{h,cc}`, `s2cell_union.cc` (`GetCapBound` 235–259,
   `GetRectBound` 261–267, `Expand` 471–484, areas 495–513, `Contains(p)` 564), `s2cell.cc`
   (`GetRectBound` 226–275, `Contains(p)` 291–320), `s2metrics.h:186–200`.
6. **Records and numbering**: values INVENTORY §1–12 (this feature adds §13), PROPERTIES §1–11
   (adds §12); `verify/cell_union/INVENTORY.md` rows 12, 25–28, 35–37, 40; mutants from `M-950`
   (series 9, research R2); KI-013; D-010; U-017. CI's `verify` matrix is at
   `.github/workflows/verify.yml:141`, the `slow` matrix at `:235`, the mutants' at `:368`.
7. **Stubs and domains to reuse**: `stubs::{asin_contract, atan2_contract, sqrt_contract,
   pure_from_point, any_cell_bound, any_uv_bound, pure_chord, pure_add}`;
   `value_domain::{any_triple, any_plain_triple, any_box_point, any_cell, any_cell_vertex,
   any_box_cap, any_angle}`; `union_domain::any_valid_union::<N>()`.

---

## Phase 1: Setup (Shared Infrastructure)

- [X] T001 Add the `pr_*` and `uf_*` modules and `region_spec` to `verify/src/lib.rs` (`uf_*`
  gated on `values` and `cellunion`), and the two prefixes to `verify/Cargo.toml`'s `values`
  comment (contracts/harness-conventions.md).
- [X] T002 [P] Extend `make verify-values` with `P=regions` in `Makefile` (its comment, the
  `VALUES_HARNESSES` chain calling `values_part` for `pr` and `uf`, the default list, the help
  text).
- [X] T003 [P] Add the record sections: `verify/values/INVENTORY.md` §13 and
  `verify/values/PROPERTIES.md` §12 headers, and a "Floating-point methods (spec 011)" heading
  in `verify/cell_union/PROPERTIES.md`, filled in by T021, T030 and T038.

## Phase 2: Foundational (Blocking Prerequisites)

- [X] T004 Write `verify/src/region_spec.rs` and `verify/src/region_spec_tests.rs`: the
  description (data-model §1), unit tests at the special values (±0, NaN), and the five drift
  guards on the library's source text (after T012–T014 land the text they read).
- [X] T005 Extend `verify/src/stubs.rs`: `pure_leaf`, `pure_cell_cap`, `any_center`,
  `pure_cell_area`, `pure_avg_area`, `sqrt_between` (data-model §3; research R5), with tables
  of ten slots and `reset()` clearing them.
- [X] T006 [P] Extend `verify/src/laws.rs` with `region::rect_bound_contains_points` and
  `region::covering_contains_points::<R, MAX>` (FR-010; research R6), each doc comment naming
  its instances and quickcheck twins; `no_intersect_no_vertex`'s comment names `CellUnion` as
  a region it does not hold for.
- [X] T007 [P] Add `tools/refgen-cpp/generate_region.cc` (a `CMakeLists.txt` target, a README
  step) and generate `core/tests/data/cpp_region_vectors.csv` (research R9; seven sections).
- [X] T008 [P] Extend `verify/tests/libm_contracts.rs`: `sqrt_between`'s clause (specials and
  10⁶ random inputs), `pure_leaf`'s basis (`CellId::from_point` a valid leaf for the 1,331
  special triples and 10⁶ random ones, in release builds; the debug panics are K4's), and
  `pure_cell_cap`'s and `any_center`'s (the centre in the unit box, the radius in [0, 4]) on
  every cell to level 5 and 10⁶ random cells.
- [X] T009 [P] Add `verify/tests/area_enumeration.rs`: `Cell::average_area_for_level(l)` equals
  `ldexp(4π/6, −2l)` bit for bit for each of the 31 levels (P-UAREA-04).
- [X] T010 Write the lemmas: L-PR-01 as an assertion in `cl_crash::c_fg_from_point` (weekly)
  that the result is a valid leaf; L-PR-02 in `verify/src/uf_bnd.rs` (`l_pr_02`, `BoxCap³`,
  `pure_chord`, `pure_add`); measure L-PR-02 first (`make verify-harness H=uf_bnd::l_pr_02`).

## Phase 3: User Story 4 — Parity restored (Priority: P2)

**Goal**: K1 and K2 fixed and K3, K4 documented, each as its own commit with a test that fails
without it.

**Independent test**: `make test`; each fix's test fails on the commit before it.

- [X] T011 [US4] Create `core/src/s2/point_region_tests.rs`, wired into `point_region.rs` with
  `#[cfg(test)] #[path = "point_region_tests.rs"] mod point_region_tests;`.
- [X] T012 [US4] FR-017 (K1): `POLE_MIN_LAT = 0.615_479_708_670_387_3` in `core/src/s2/cell.rs`
  with a comment (upstream computes `asin(sqrt(1./3))`; research R3). Tests first: the polar face
  cells' latitude bits are upstream's (`0x3fe3b2028082e8d0`), and the constant equals
  `(1.0f64 / 3.0).sqrt().asin()`; red, then green. Run the Python and wasm suites. Commit.
- [X] T013 [US4] FR-018 (K2): `approx_area` and `exact_area` fold from `0.0` in
  `core/src/s2/cell_union.rs`; test in `cell_union_tests.rs`: the empty union's areas are +0 bit
  for bit. Red, then green. Commit.
- [X] T014 [US4] FR-019 (K3): `PointRegion::new`'s precondition (a unit point, upstream's
  `DCHECK`) and each method's behaviour outside it documented; a hand-written `Deserialize`
  rejecting a point that is not unit length (`Point::is_unit`, as `Cap`'s in
  `core/src/s2/cap.rs:645`); tests in `point_region_tests.rs`: (3, 4, 0), the zero vector and
  NaN rejected, a unit point round-trips. Red, then green. Commit.
- [X] T015 [US4] FR-019 (K4): document each debug-build panic exactly (spec K4): in
  `CellUnion::expand_by_radius`, `Cell::contains_point`, `CellId::from_point` (two or more
  infinite coordinates), `s1::Interval::from_point`, `Rect::from_lat_lng`, and on
  `PointRegion`'s `rect_bound`, `intersects_cell`, `cell_union_bound` and `CellUnion`'s
  `contains_point`; a `#[should_panic]` debug test for each (`#[cfg(debug_assertions)]`).
  Commit.
- [X] T016 [US4] Add `core/tests/cpp_region_diff.rs` over the seven sections, and
  `cell_union_tests::vertex_law_does_not_hold` pinning one of the probe's counterexamples
  (research R9; depends on T007, T012–T015).
- [X] T017 [P] [US4] FR-010, FR-013: the twins of the two new laws in
  `core/tests/trait_property_tests.rs` for `Cap`, `Rect`, `Cell`, `CellUnion` and
  `PointRegion`; `PointRegion`s at cell vertices, face boundaries and the poles; unions with the
  polar faces, opposite faces and clustered cells.
- [X] T018 [US4] Run `make test`, then the probe again (quickstart §2; spec SC-007, SC-009).
  Commit the vectors and tests.

## Phase 4: User Story 1 — Proven crash freedom (Priority: P1)

**Goal**: every in-scope entry crash-free over its domain, or a measured (inherited) limit.

**Independent test**: `make verify-values P=regions` reports every `pr_crash` and `uf_crash`
harness `SUCCESSFUL` with its covers satisfied; `make verify-release` the `n_`/`o_` members.

- [X] T019 [US1] Write `verify/src/pr_crash.rs` (data-model §5.1 rows 1–4: `c_basic`,
  `c_rect_bound`, `c_intersects_cell`, `c_cell_union_bound` weekly; `n_rect_bound_nan`,
  `n_intersects_cell_nan`; `o_` members for the release pass where a debug check is skipped).
- [X] T020 [US1] Write `verify/src/uf_crash.rs` (data-model §5.2: `c_contains_point__n{0..3}`,
  `c_average_based_area__n{1..3}`, `c_approx_area__n{1..3}`, `c_cap_bound__n1` and `__n2`
  weekly, `c_rect_bound__n1` real and `__n{2,3}` stubbed, `n_expand_by_radius_nan`).
- [X] T021 [US1] Run the crash suites and fill `verify/values/INVENTORY.md` §13 and the nine
  rows of `verify/cell_union/INVENTORY.md` (the inherited limits written down: `exact_area`,
  `cell_union_bound`, `expand_by_radius`'s expansion).
- [X] T022 [US1] Record any further undocumented precondition a proof needed (spec FR-005;
  SC-010).

## Phase 5: User Story 2 — `PointRegion`'s catalogue and laws (Priority: P1)

**Goal**: data-model §4.1.

**Independent test**: every entry proven, by composition or under contract, or not proven with a
reason; each proven one killed by a mutant.

- [X] T023 [P] [US2] P-PRCON-01 in `verify/src/pr_con.rs`.
- [X] T024 [P] [US2] P-PRBND-01, -02, -03 (weekly) in `verify/src/pr_bnd.rs`.
- [X] T025 [P] [US2] P-PRCEL-01, -02, -03 in `verify/src/pr_cel.rs`.
- [X] T026 [P] [US2] P-PREQ-01 in `verify/src/pr_eq.rs` (`laws::eq::all` over `PlainTriple`).
- [X] T027 [US2] `verify/src/pr_reg.rs`: P-PRREG-01, -04, -05 per change; P-PRREG-06 weekly;
  P-PRREG-03 weekly, one harness per face (`__f0` … `__f5`, `sqrt_between`); P-PRREG-02's
  composition in the module's documentation.
- [X] T028 [US2] Run the weekly members (`make verify-slow F=values P='pr_bnd::p_prbnd_03
  pr_reg::p_prreg_03 pr_reg::p_prreg_06 pr_crash::c_cell_union_bound'`); a member that times out
  at 60 minutes goes to `verify/known-timeouts.txt` and is recorded as a tool limit.
- [X] T029 [US2] P-PRDES-01: the deserialiser tests of T014 cited.
- [X] T030 [US2] Fill `verify/values/PROPERTIES.md` §12 (spec SC-003, SC-004).

## Phase 6: User Story 3 — `CellUnion`'s floating-point methods and laws (Priority: P2)

**Goal**: data-model §4.2.

**Independent test**: `make verify-values P=regions` runs the `uf_*` harnesses; each fails on its
mutant.

- [X] T031 [P] [US3] P-UPT-01 in `verify/src/uf_pt.rs`.
- [X] T032 [P] [US3] P-UBND-01, -03 and the weekly `p_ubnd_02__n1` in `verify/src/uf_bnd.rs`
  (beside L-PR-02); P-UBND-02's composition and P-UBND-04's drift guards in its documentation.
- [X] T033 [P] [US3] P-UAREA-01 to -03 in `verify/src/uf_area.rs` (`__n0` is FR-018's: it fails
  before T013); P-UAREA-04 cites T009.
- [X] T034 [P] [US3] `verify/src/uf_exp.rs`: the documentation of P-UEXP-01's composition (008's
  P-CPBND-02 and the drift guard) and P-UEXP-02's limit; no harness.
- [X] T035 [US3] `verify/src/uf_reg.rs`: P-UREG-01 (`__n0` … `__n3`); the reasons for P-UREG-02,
  -04, -05, -06 and the vertex-law exception (P-UREG-03) in its documentation.
- [X] T036 [US3] Run the weekly members (`make verify-slow F=values P='uf_bnd::p_ubnd_01__n3
  uf_bnd::p_ubnd_02__n1 uf_area::p_uarea_02__n3 uf_area::p_uarea_03__n3 uf_crash::c_cap_bound__n2
  cl_crash::c_fg_from_point'`).
- [X] T037 [US3] Write the mutants `verify/mutants/M-950…` (data-model §7) and run them
  (`make verify-mutants M='M-95* M-96* M-97*'`); record them in `verify/MUTANTS.md`.
- [X] T038 [US3] Fill `verify/cell_union/PROPERTIES.md`'s new entries (spec SC-003, SC-004).

## Phase 7: User Story 5 — CI, records and evaluation (Priority: P3)

- [X] T039 [US5] Extend `.github/workflows/verify.yml`: `values-regions` in the `verify` matrix,
  a `values-regions` leg in `slow` with T028's and T036's filters, `point_region.rs` and
  `cpp_region_vectors.csv` in the path filters, the `mutants` comment (series 9 holds 010's and
  011's) (contracts/verify-commands.md).
- [X] T040 [P] [US5] Extend `verify/TRUSTED_BASE.md` (the new stubs and contracts, L-PR-01,
  `sqrt_between`) and `KANI_ISSUES.md` (KI-013, `powi`, with R0b's reproducer).
- [X] T041 [P] [US5] `verify/PROPERTIES_BY_KIND.md` and its HTML render (spec SC-011).
- [X] T042 [P] [US5] `VERIFIED_TYPES.md`: §2.12 for `PointRegion` and `CellUnion`'s float half,
  §3's `Region` row, the missing `S2PointRegion` codec (FR-020); `CORE_TRAITS.md`'s `Region`
  invariants name the new instances and `CellUnion`'s vertex-law exception; `verify/COVERAGE.md`.
- [X] T043 [US5] Write the spec-011 part of `verify/values/EVALUATION.md` (spec FR-023, SC-013).

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T044 Final run, one at a time (quickstart §1–5), including `make verify-values P=latlng`
  and `P=cell` (FR-017 touches `Cell`, and L-PR-01 `cl_crash`).
- [X] T045 Check each SC against the evidence and update the spec's status line.
- [X] T046 [P] `quickstart.md`: replace the expected values with the measured ones.
- [ ] T047 Live CI check (spec SC-006): needs a push and a draft PR. **The user said never
  push**, so it stays open; SC-006 is measured on the workstation only.

---

## Dependencies & Execution Order

- **Phase 1 → Phase 2 → Phase 3** in order; T004's drift guards wait for T012–T014.
- **Phase 3** before Phases 4–6: the statements rest on the fixed behaviour (+0 sums, the
  documented panics).
- **Phases 4, 5, 6** in any order after Phase 3; their Kani runs go one at a time.
- **T037** after T019–T036 (the mutants need their harnesses).
- **Phase 7** after the runs it records; **Phase 8** last.

### Parallel opportunities

T002, T003; T006–T009; T017; T023–T026 (writing); T031–T034 (writing); T040–T042.

## Implementation Strategy

- **MVP**: Phases 1–3 (the fixes, with their tests and vectors) and Phase 4 (crash freedom).
- **Then** Phase 5 (`PointRegion`'s laws) and Phase 6 (`CellUnion`'s), then the records.
- Each fix is its own commit; the harnesses and their records are committed together
  (constitution, "Verification records"). Never push.
