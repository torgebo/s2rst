# Quickstart: Validating the Verification of `r1::Interval` and `s2::Rect`

A run guide. The commands are in [contracts/verify-commands.md](contracts/verify-commands.md),
the conventions in [contracts/harness-conventions.md](contracts/harness-conventions.md), and the
catalogue and inventories in [data-model.md](data-model.md) §4 and §5.

Prerequisites:

- **Kani 0.68.0** (`make verify-kani-check`).
- Run verification jobs one at a time, through the Makefile targets, after `df -h /`.
- 004's open jobs (its weekly members and mutants) are not run at the same time.

## 1. The proofs (US1, US2, SC-001, SC-003, SC-005)

```sh
make verify-values P=real
make verify-values P=rect
```

Expected:

- `cargo test` in `verify/` passes, including `real_spec`, `rect_spec` and
  `values_spec_vs_cpp` on the new sections.
- Every `ri_*` and `rl_*` line reads `SUCCESSFUL` with all covers satisfied.
- Each part finishes in under 10 minutes on a 4-vCPU, 16 GB runner. Measured on the
  8-thread workstation with `VERIFY_JOBS=4` (2026-10-14): `P=real` 25 harnesses in 2 min 20 s,
  `P=rect` 36 in 5 min 37 s. The CI measurement needs a push (T039).
- The weekly members need `make verify-slow F=values P=real` (P-REXP-03's symmetry, 564.5 s)
  and `P=rect` (P-LBND-01 673.1 s, P-LEXP-01's two containment members 130 s and 156 s, and
  P-LEXP-02's symmetry, which times out at 60 minutes and is reported `KNOWN-TIMEOUT`).

Cross-checks:

- Sections 3–4 of `verify/values/INVENTORY.md` and 4–5 of `PROPERTIES.md` name every harness in
  the output.
- `cargo test --test property_kinds` passes.

## 2. Composition (SC-010)

`verify/values/EVALUATION.md`, spec-005 part, reports two things:

- the share of rectangle properties whose statements use 004's `arc_spec` (target: every
  property with a longitude);
- the rectangle's harness times against 004's circular-interval times, with R0's
  component-wise timeout as the counterpoint.

## 3. The regression tests (SC-009)

```sh
cargo test -p s2rst --features serde s2::rect::rect_tests
```

Expected: the invalid rectangles do not deserialise, and the valid ones, the boundary values
included, round-trip and stay valid. With FR-020's change reverted, the rejection tests fail.

## 4. The differential tests (SC-011)

```sh
cargo test -p s2rst --test cpp_values_diff
```

Expected: every row passes. The rows marked `deviation:infinite-corner` match the documented
behaviour (FR-017), and those marked `deviation:center-overflow` the true centre where
upstream overflows (U-010). Regenerate as 004's quickstart §4 describes.

## 5. Non-vacuity (SC-004)

```sh
VERIFY_SLOW_HARNESS_TIMEOUT=60m make verify-mutants M='M-4*'   # the M-4NN patches, one at a time
```

Expected: every `M-4NN` row in `verify/MUTANTS.md` reads `killed`. The five `Tier: slow`
mutants (`M-414`, `M-416`, `M-431`, `M-432`, `M-438`) need the weekly 60 minutes; `M-414` is
killed in 968 s. The campaign took 69 minutes, and 32 more for `M-414` (2026-10-13/14).

## 6. CI and the evaluation (US3, US4, SC-007)

- A pull request that makes `Rect::contains_lat_lng` skip its validity screen fails the
  `verify / values-rect` leg, naming `rl_mem::p_lmem_01`. Locally, `make verify-values
  P=rect` fails that way (T040); the CI half needs a push.
- `verify/values/EVALUATION.md` answers SC-007's four questions for the value types.
