---

description: "Task list for the formal verification of LatLng and of s2::Rect as a Region (spec 010)"
---

# Tasks: Formal Verification of `LatLng`, and `s2::Rect` as a `Region`

**Input**: Design documents from `specs/010-formal-verify-latlng/`:

- plan.md, spec.md (clarified 2026-10-27);
- research.md (R0–R12; R0, the cap-bound probe, ran 2026-10-28);
- data-model.md; contracts/ (verify-commands, harness-conventions); quickstart.md;
- the probes in `probe/`: the survey (`cpp/survey.cc`, `rust/src/main.rs`; K1–K5) and R0
  (`rust/src/bin/capbound.rs`, `cpp/capbound_cases.cc`, with their results).

The feature extends the `values` suite of 004–008 in `verify/`. 008's files are the style
reference: `verify/src/cap_spec.rs`, `cp_*.rs`, `stubs.rs`, `value_domain.rs`, `laws.rs`;
`verify/tests/values_spec_vs_cpp.rs`, `libm_contracts.rs`; `tools/refgen-cpp/generate_cap.cc`
and `core/tests/cpp_cap_diff.rs`.

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **For T007 only**: the C++ build installed at the scratchpad prefix (`probe/README.md`).
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run (28 GB on 2026-10-28;
  memory "disk space and the Kani cache").

**Tests**: The spec asks for these ordinary tests, so they appear as tasks: the regression
tests of FR-017 to FR-019 (each failing without its change), the differential tests against
the new C++ vectors (FR-021), the description's unit and sanity tests (FR-007), the native
tests of the new contracts (FR-003), and the quickcheck boundary points (FR-011). The Kani
harnesses are the verification deliverable, so they are implementation tasks.

**Organization**:

| Story | Priority | Phase |
|-------|----------|-------|
| US4: parity restored (the library changes) | P2 | 3 (first: the proofs rest on it) |
| US1: proven crash freedom | P1 | 4 |
| US2: proven meaning against `latlng_spec` | P1 | 5 |
| US3: the `Region` laws for `Rect` | P2 | 6 |
| US5: CI, records and evaluation | P3 | 7 |

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks). For Kani
  **runs**, "parallel" applies only to writing the code; the runs go one after another.
- **[Story]**: the user story the task belongs to (US1–US5).

## Memory safety (read first)

- **Every** Kani run goes through a Makefile target or `verify/run-verify.sh`:
  `make verify-values P=latlng`, `make verify-harness H=…`, `make verify-mutants M=…`.
- **One run at a time**; check `pgrep -fa '[c]bmc'` first, and never kill another run.
  `verify/` must compile at every save while a run is going.

## Upstream findings rule (FR-022, read first)

U-016 (research R0, R1) is the only one so far. Another one gets the next free **U-017**, a
task in Phase 8, an entry in `UPSTREAM.md` and the spec-010 evaluation, and a citation in the
affected harness. Never change upstream code, never file upstream.

## Known-defect rule (FR-014, read first)

- A proof that fails on a **real library defect** gets the next free **D-010**, recorded under
  "Defects found" in the spec-010 evaluation. K1–K5 are the survey's, handled by FR-017 to
  FR-020.
- **Ask the user before fixing** a defect not in the spec or this plan.
- **Replay a failure natively before believing it** (CBMC's `cos`, `log2`, `%` and `x / ∞`);
  with stubs, look first for a fact the stub lacks.

## Facts these tasks rely on

1. **`LatLng`** (`core/src/s2/latlng.rs`): struct with public `lat`, `lng` (`Angle`), derived
   `Clone, Copy, Debug, PartialEq` and serde (lines 50–58); constructors 64–135; `latitude` 142,
   `longitude` 150, `from_point` 155; `is_valid` 165; `normalized` 171–181 (K1); `to_point`
   187–196; `get_distance` 205–215 (K3); `approx_eq`, `approx_eq_with` 222–231;
   `to_string_in_degrees` 235; the operators 243–285; `Default` 287; `Display` 294; the `From`
   impls 300–311; inline `mod tests` from 312 and `mod quickcheck_tests` from 760. New tests go
   to a sibling `latlng_tests.rs`.
2. **`Angle::normalized`** (`core/src/s1/angle.rs:176–189`): `%` then one correction, −π to π.
   `angle_tests.rs` checks it against `crate::test_util::ieee_remainder_2pi`.
3. **`Rect`**: `cap_bound` `core/src/s2/rect.rs:641–676`; `contains_point` through
   `LatLng::from_point`; `Region for Rect` `core/src/s2/region.rs:104–129`. `Cell::rect_bound`
   `core/src/s2/cell.rs:540–600`.
4. **Upstream**: `s2latlng.cc` (`Normalized` 56–66, `ToPoint` 68–76, `GetDistance` 85–109,
   `ToStringInDegrees` 111–114), `s2latlng.h` (`Latitude` 235, `Longitude` 244, `is_valid` 252,
   `Invalid` 230, `ApproxEquals` 158); `s2latlng_rect.cc` (`GetCapBound` 298–332,
   `Contains(S2Cell)` 343).
5. **Records and numbering**: INVENTORY §1–11 (this feature adds §12), PROPERTIES §1–10 (adds
   §11; `Rect` is §5); mutants from `M-901` (series 9); U-016; D-010. CI's `verify` matrix is at
   `.github/workflows/verify.yml:139`, the mutants' series at `:363`; the path filters already
   name `latlng.rs`, `angle.rs`, `rect.rs`, `region.rs`.
6. **The quickcheck twins** (`core/tests/trait_property_tests.rs`): the `Region` property
   functions and the rectangle instances (`region_rect_*`).

---

## Phase 1: Setup (Shared Infrastructure)

- [X] T001 Add the `ll_*` modules, `ll_reg` and `latlng_spec` to `verify/src/lib.rs`, and the
  `ll_*` part to `verify/Cargo.toml`'s `values` comment (contracts/harness-conventions.md).
- [X] T002 [P] Extend `make verify-values` with `P=latlng` in `Makefile` (its comment, the
  `VALUES_HARNESSES` chain, the default list, the target's help text).
- [X] T003 [P] Add the record sections: `verify/values/INVENTORY.md` §12 and
  `verify/values/PROPERTIES.md` §11 headers, filled in by T020 and T031.

## Phase 2: Foundational (Blocking Prerequisites)

- [X] T004 Write `verify/src/latlng_spec.rs` and `verify/src/latlng_spec_tests.rs`: the
  description and restatements (data-model §1), unit tests at the special values, and the drift
  guard of `reduce_fmod_copy` (after T012 lands the function it copies).
- [X] T005 Extend `verify/src/stubs.rs`: the pure contracts, `sqrt_contract`, `pure_sqrt`,
  `pure_from_point`, `any_cell_bound`, `cell_rect_bound_contract` (data-model §3; research R5,
  R6).
- [X] T006 [P] Extend `verify/src/value_domain.rs` with `any_lat_lng_bits` and
  `any_finite_lat_lng` (data-model §2).
- [X] T007 [P] Add `tools/refgen-cpp/generate_latlng.cc` (a `CMakeLists.txt` target, a README
  step) and generate `core/tests/data/cpp_latlng_vectors.csv` (research R9).
- [X] T008 Extend `verify/tests/values_spec_vs_cpp.rs` with the `LATLNG_*` sections against
  `latlng_spec`'s restatements (depends on T004, T007).
- [X] T009 [P] Extend `verify/tests/libm_contracts.rs`: `sqrt_contract` (specials and 10⁶
  random inputs), and the `Cell::rect_bound` contract on every cell to level 5 and 10⁶ random
  cells (research R6).
- [X] T010 Write `verify/src/ll_lem.rs` (L-LL-01) and measure it first (`make verify-harness
  H=ll_lem::l_ll_01`).

## Phase 3: User Story 4 — Parity restored (Priority: P2)

**Goal**: K1, K3 and K4 fixed, each as its own commit with a test that fails without it.

**Independent test**: `make test`; each fix's test fails on the commit before it.

- [X] T011 [US4] Create `core/src/s2/latlng_tests.rs`, wired into `latlng.rs` with
  `#[cfg(test)] #[path = "latlng_tests.rs"] mod latlng_tests;`.
- [X] T012 [US4] FR-017 (K1): `remainder_2pi` and `reduce_fmod` in `core/src/s1/angle.rs`,
  `Angle::normalized` and `LatLng::normalized` through it (research R3). Tests first in
  `latlng_tests.rs` (R10's values, the exact-remainder property, validity for every finite
  input) and `angle_tests.rs` (`remainder_2pi` itself, ties ±π to ±7π); red, then green.
  Commit.
- [X] T013 [US4] FR-018 (K3): `get_distance` clamps before the root; the survey's pair returns
  NaN. Red, then green. Commit.
- [X] T014 [US4] FR-019 (K4, U-016): `Rect::cap_bound` widened after upstream's choice (research
  R0); tests in `core/src/s2/rect_tests.rs`: R0's worst cases, the survey's 114 points, a
  native sweep of 20,000 rectangles with R0's families and points, the centre upstream's. U-016
  in `UPSTREAM.md` (both caps, the inputs, the margins). Red, then green; record any test
  pinning a covering that changes. Commit.
- [X] T015 [P] [US4] Document the domains in `core/src/s2/latlng.rs` (`to_point`'s finite
  precondition, `get_distance`'s NaN outside its domain, `normalized`'s ties and zeros; spec
  SC-010).
- [X] T016 [US4] Add `core/tests/cpp_latlng_diff.rs` over the seven sections (research R9;
  depends on T007, T012–T014).
- [X] T017 [P] [US4] FR-011: rectangles near the poles and boundary points in the `Region`
  twins of `core/tests/trait_property_tests.rs`, and the probe's cases as fixed examples.
- [X] T018 [US4] Run `make test`, then the probe again (quickstart §2; spec SC-007, SC-009).
  Commit the vectors and tests.

## Phase 4: User Story 1 — Proven crash freedom (Priority: P1)

**Goal**: every in-scope entry crash-free over its domain.

**Independent test**: `make verify-values P=latlng` reports every `ll_crash` harness
`SUCCESSFUL` with its covers satisfied.

- [X] T019 [US1] Write `verify/src/ll_crash.rs` (data-model §5 rows 1–9; `o_get_distance` for
  the pairs outside the documented domain).
- [X] T020 [US1] Run the crash suite and fill `verify/values/INVENTORY.md` §12.
- [X] T021 [US1] Record any undocumented precondition a proof needed (spec FR-004; SC-010).

## Phase 5: User Story 2 — Proven meaning (Priority: P1)

**Goal**: the catalogue of data-model §4.1.

**Independent test**: every entry proven, by composition or under contract, or not proven with
a reason; each proven one killed by a mutant.

- [X] T022 [P] [US2] P-LLCON-01 in `verify/src/ll_con.rs`.
- [X] T023 [P] [US2] P-LLFP-01, -02 in `verify/src/ll_fp.rs`.
- [X] T024 [P] [US2] P-LLTP-01, -02 in `verify/src/ll_tp.rs`.
- [X] T025 [P] [US2] P-LLNRM-01, -03 in `verify/src/ll_nrm.rs`; the compositions P-LLNRM-02,
  -04, -05 written down in the module's documentation.
- [X] T026 [P] [US2] P-LLVAL-01 in `verify/src/ll_val.rs`.
- [X] T027 [P] [US2] P-LLDST-01 to -03 in `verify/src/ll_dst.rs`.
- [X] T028 [P] [US2] P-LLAPX-01, -02 in `verify/src/ll_apx.rs`.
- [X] T029 [P] [US2] P-LLARI-01 in `verify/src/ll_ari.rs`; P-LLEQ-01 in `verify/src/ll_eq.rs`.
- [X] T030 [US2] P-LLGEO-01 and P-LLDES-01: the composition argument and the tests they rest on
  (`latlng_tests::serde_round_trip`; the `geo_types_interop` tests).
- [X] T031 [US2] Run the part and fill `verify/values/PROPERTIES.md` §11 (spec SC-003).

## Phase 6: User Story 3 — The `Region` laws for `Rect` (Priority: P2)

**Goal**: P-LREG-01 to -04 and P-LMEM-03 (data-model §4.2).

**Independent test**: `make verify-values P=latlng` runs `ll_reg`'s harnesses; each fails on its
mutant.

- [X] T032 [US3] Write `verify/src/ll_reg.rs`: P-LREG-01, P-LREG-02, P-LMEM-03 (research R6).
- [X] T033 [US3] Run them and record P-LREG-01 to -04 and P-LMEM-03 in
  `verify/values/PROPERTIES.md` §5 (replacing its closing note on the `region` laws).
- [X] T034 [US3] Write the mutants `verify/mutants/M-901…` (data-model §7) and run them
  (`make verify-mutants M='M-9*'`); record them in `verify/MUTANTS.md`.

## Phase 7: User Story 5 — CI, records and evaluation (Priority: P3)

- [X] T035 [US5] Extend `.github/workflows/verify.yml`: `values-latlng` in the `verify`
  matrix, `cpp_latlng_vectors.csv` in the path filters, series `9` in `mutants`
  (contracts/verify-commands.md).
- [X] T036 [P] [US5] Extend `verify/TRUSTED_BASE.md`: the pure contracts, `sqrt_contract`, the
  `Cell::rect_bound` contract, IEEE `fmod`'s exactness.
- [X] T037 [P] [US5] `verify/PROPERTIES_BY_KIND.md` and its HTML render (spec SC-011).
- [X] T038 [P] [US5] `VERIFIED_TYPES.md`: §2.11 for `LatLng`, §3's `Region` row gains `Rect`,
  the missing upstream operations (FR-020); `CORE_TRAITS.md`'s `Region` invariants name
  `Rect`'s instances; `verify/COVERAGE.md`.
- [X] T039 [US5] Write the spec-010 part of `verify/values/EVALUATION.md` (spec FR-016,
  SC-013).

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T040 Final run, one at a time (quickstart §1–5).
- [X] T041 Check each SC against the evidence and update the spec's status line.
- [X] T042 [P] `quickstart.md`: replace the expected values with the measured ones.
- [ ] T043 Live CI check (spec SC-006): needs a push and a draft PR. **The user said never
  push**, so it stays open; SC-006 is measured on the workstation only.

---

## Dependencies & Execution Order

- **Phase 1 → Phase 2 → Phase 3** in order; T004's drift guard waits for T012.
- **Phase 3** before Phases 4–6: the statements rest on the fixed behaviour.
- **Phases 4, 5, 6** in any order after Phase 3; their Kani runs go one at a time.
- **T034** after T019–T033 (the mutants need their harnesses).
- **Phase 7** after the runs it records; **Phase 8** last.

### Parallel opportunities

T002, T003; T006, T007, T009; T015, T017; T022–T029 (writing); T036–T038.

## Implementation Strategy

- **MVP**: Phases 1–3 (the fixes, with their tests and vectors) and Phase 4 (crash freedom).
- **Then** Phase 5 (the catalogue) and Phase 6 (the laws), then the records.
- Each fix is its own commit; the harnesses and their records are committed together
  (constitution, "Verification records"). Never push.
