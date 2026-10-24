# Quickstart: Validating the Verification of `Cell` and `s1::Interval`

This is a run guide that shows the feature working end to end.

- The command interfaces are in [contracts/verify-commands.md](contracts/verify-commands.md).
- Harness naming and generators are in
  [contracts/harness-conventions.md](contracts/harness-conventions.md).
- The catalogue and the inventory are in [data-model.md](data-model.md) §4 and §5.

Prerequisites:

- **Kani 0.68.0** (`make verify-kani-check`).
- Always go through the Makefile targets, which apply the memory guard, and run one
  verification job at a time.
- Run `df -h /` first: the Kani build cache under `verify/target/kani` grows by several GB.

## 1. The proofs (US1, US2, SC-001, SC-003, SC-005)

```sh
make verify-values            # or one part: P=cell or P=interval
```

Expected:

- `cargo test` in `verify/` passes. That includes `values_spec_vs_cpp`: both reference
  descriptions against every row of `cpp_values_vectors.csv`.
- Every `cl_*` and `iv_*` harness line reads `SUCCESSFUL`, with all covers satisfied.
- The exit code is `0`. Each part takes under 10 minutes on a 4-vCPU, 16 GB runner, and CI
  runs the two parts as separate legs.
- The deeper levels of the bound families, and any split members, need
  `make verify-slow F=values` (weekly, one at a time, up to 60 minutes each).
- `uv_projection` (part of `cargo test`) reports all 2^30 + 1 boundary values checked, with
  0 differences.

Cross-checks:

- Every in-scope row of `verify/values/INVENTORY.md` names a harness or family that appears in
  the output.
- Every `proven` row of `verify/values/PROPERTIES.md` names a harness in the output.
- `cargo test --test property_kinds` in `verify/` passes, so every new entry is in
  `PROPERTIES_BY_KIND.md` under its kind (SC-012).

## 2. The seam correction is contained (FR-020, the containment proof)

```sh
make verify-harness H=iv_mea::p_imea_01     # off the corner: bit for bit upstream's length
make verify-harness H=iv_mea::p_imea_03     # the corner is the only negative case upstream
```

Expected: both `SUCCESSFUL`.

- P-IMEA-01 says that the fix changes `length` for exactly one valid interval.
- P-IMEA-03 says that this interval is the only one upstream's arithmetic got wrong.

Then check that the proof is what holds the fix in place:

```sh
make verify-mutants M=M-3NN     # seam-fix-widened: the stable form for every inverted interval
```

Expected: the mutant is reported `killed`, with `iv_mea::p_imea_01` FAILED and a concrete
inverted interval whose length changed in its last bit.

## 3. The regression tests (SC-009, SC-010)

```sh
cargo test -p s2rst --features serde s2::cell::cell_tests
cargo test -p s2rst s1::interval::interval_tests
make -C python test        # includes the corner's length through the bindings
```

Expected: all pass. With the FR-018, FR-020 or D-005 change reverted (`git stash` of that
one file), the matching tests fail:

- **FR-018:** the level-12 edit and the invalid identifier deserialise, and up to 23% of
  cells come back with a bound one ulp off.
- **FR-020:** the corner reports −1, `expanded(3.2)` is not full, and the rectangle's area
  is negative.
- **D-005:** `[1, 2].directed_hausdorff_distance([0, 1e−17])` panics in the debug test
  build.

## 3a. The generic trait tests (Constitution III, research R15)

```sh
cargo test -p s2rst --test trait_property_tests
cargo test -p s2rst --lib s2::rect::rect_tests     # D-006
```

Expected: 110 or more tests pass, including `region_union_cap_contains` (D-006) and the new
`Cell` instances of the integer `Region` laws. With D-006's one-line fix reverted,
`cap_bound_contains_wide_rect_across_antimeridian` fails.

## 4. The differential tests (SC-011)

```sh
cargo test -p s2rst --test cpp_values_diff
```

Expected: every row passes. The rows marked `deviation:seam-length` match the documented
deviation, not the C++ value.

To regenerate the vectors (needs the local s2geometry install, not part of CI):

```sh
cmake --build tools/refgen-cpp/build --target generate_values
tools/refgen-cpp/build/generate_values > core/tests/data/cpp_values_vectors.csv
```

## 5. Non-vacuity (FR-009, SC-004)

```sh
make verify-mutants           # every M-3NN patch, among the others
```

Expected: every `M-3NN` row is `killed`, and `verify/MUTANTS.md` has one row per patch.

## 6. CI (US3, SC-005)

Open a pull request that changes the `Right` arm of `Cell::uv_coord_of_edge` to return
`x.lo`. The `verify / values-cell` leg fails, naming `cl_bnd::p_cbnd_03` with a concrete
cell.
Revert it, and the leg passes. A pull request that touches none of the paths in
contracts/verify-commands.md does not start the leg.

## 7. The evaluation (US4, SC-007)

`verify/values/EVALUATION.md` answers these four questions on its own:

- Should the value types continue?
- With which approach?
- Do type-level invariants come first?
- What comes next?

`VERIFIED_TYPES.md` lists `Cell` and `s1::Interval` under "Types with proofs".
