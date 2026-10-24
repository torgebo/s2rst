# Contract: Verification Commands (changes for `r2::Rect` and `Point`)

The 001–005 contracts still apply. This file lists only what changes.

## `make verify-values [P=cell|interval|real|rect|uvrect|point]` (extended)

- `P=uvrect` selects `--harness ur_`, and `P=point` selects `--harness pt_`.
- Without `P`, the target runs all six parts: `--harness cl_ --harness iv_ --harness ri_
  --harness rl_ --harness ur_ --harness pt_`.
- The models' sanity tests (`cargo test` in `verify/`) cover `uv_spec` and `point_spec` too.

## `make verify-slow F=values [P=…]` (extended)

The weekly members of this feature join 004's and 005's: any member over the per-change budget
that R0 or the first run finds (research R12), expected to include the point's unit-length
properties.

## CI: `.github/workflows/verify.yml`

| Job | Change |
|-----|--------|
| `verify` | The matrix gains `values-uvrect` and `values-point`. The path filters gain `core/src/s2/edge_crossings.rs` (FR-021 moves the scaling's caller there); `core/src/r2/**`, `core/src/r3/**` and `core/src/s2/point.rs` are already listed |
| `slow` | 004's pending `values` leg covers this feature once it is added (004 T042) |

## `make test` (unchanged definition)

It runs the new ordinary tests:

- `core/src/r2/rect_tests.rs` (FR-017's debug check, FR-019);
- `core/src/s2/point_tests.rs` (FR-020);
- `core/src/s2/edge_crossings_tests.rs` (FR-021);
- the extended `core/tests/cpp_values_diff.rs`.
