---

description: "Task list for the PointRegion and LatLng codecs (spec 014)"
---

# Tasks: The `PointRegion` and `LatLng` Codecs

**Input**: `specs/014-pointregion-latlng-codecs/` (plan, spec, research R0–R6, data-model,
contracts, quickstart). Style reference: spec 003's fixed-format codecs and their records.

**Tests**: the spec asks for unit tests, C++ diff tests and twins (FR-008, FR-009); the Kani
harnesses are implementation tasks.

**Memory safety**: every Kani run through the Makefile or `run-verify.sh`, one at a time.
**Upstream rule**: a finding gets U-018. **Defect rule**: D-010 and the maintainer's go-ahead.

| Story | Priority | Phase |
|-------|----------|-------|
| US1 read and write upstream's encodings | P1 | 3 |
| US2 proven like the other codecs | P1 | 4 |
| US3 records, CI, evaluation | P3 | 5 |

## Phase 1: Setup

- [X] T001 Confirm `core/src/s2/encoding.rs`'s helpers (`read_f64`, `write_f64`, error kinds) and
  the decoding suite's `codecs` part cover the new code; no new module.

## Phase 2: Foundational

- [X] T002 [P] `verify/src/wire_spec.rs`: `ref_point_region`, `enc_point_region`, `point_unit`,
  `ref_latlng`, `enc_latlng` (research R2).
- [X] T003 [P] `tools/refgen-cpp/generate_coding.cc`: the four row kinds appended (research R4);
  regenerate `core/tests/data/cpp_coding_vectors.csv`, checking every existing row is unchanged.

## Phase 3: US1 — the codecs

- [X] T004 [US1] `core/src/s2/encoding.rs`: the four implementations (data-model §1, §2), documented,
  with unit tests (short inputs, versions 0/1/2, non-unit, NaN, trailing bytes, round trips).
- [X] T005 [US1] `core/tests/cpp_coding_diff.rs` and `verify/tests/wire_spec_vs_cpp.rs`: the four
  kinds; the twins in `core/tests/property_tests.rs`; `make test`; commit.

## Phase 4: US2 — the proofs

- [X] T006 [US2] `verify/src/codec.rs`: `PointRegionC` (R18 fallback), `LatLngC`.
- [X] T007 [US2] `verify/src/cd_fx.rs`: P-FX-11 … P-FX-14 (research R3), with covers.
- [X] T008 [US2] Run `make verify-decoding P=codecs`; move any member over budget to weekly.
- [X] T009 [US2] Mutants `M-993` … `M-998` (data-model §5), the generator in the spec folder;
  `make verify-mutants`; every patch applies.

## Phase 5: US3 — records, CI, evaluation

- [X] T010 [US3] Records: `verify/decoding/INVENTORY.md`, `PROPERTIES.md` (P-FX-11 … 14), the
  index by kind and its HTML, `MUTANTS.md`, `VERIFIED_TYPES.md` (§2.15, the codec traits),
  `CORE_TRAITS.md`.
- [X] T011 [US3] CI: check the path filters cover the changed files; split the `codecs` leg (`P=fixed`, research R7).
- [X] T012 [US3] The spec-014 part of `verify/decoding/EVALUATION.md`.

## Phase 6: Polish

- [X] T013 Final run (quickstart), the spec's status; commit.
- [ ] T014 Live CI check (SC-004): needs a push; **never push**, so it stays open.

## Dependencies

Phase 1 → 2 → 3 (commit) → 4 (Kani runs one at a time) → 5 → 6. T002, T003 in parallel.

## Implementation strategy

US1 is the MVP: the codecs and their C++ diff tests. US2 brings them to the suite's standard.
