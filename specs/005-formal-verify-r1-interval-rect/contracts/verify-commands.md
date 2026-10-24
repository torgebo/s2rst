# Contract: Verification Commands (changes for `r1::Interval` and `s2::Rect`)

The 001–004 contracts still apply. This file lists only what changes.

## `make verify-values [P=cell|interval|real|rect]` (extended)

- `P=real` keeps `--harness ri_`, and `P=rect` keeps `--harness rl_`.
- Without `P`, the target runs all four parts: `--harness cl_ --harness iv_ --harness ri_
  --harness rl_`.
- The models' sanity tests (`cargo test` in `verify/`) cover `real_spec` and `rect_spec` too.

## `make verify-slow F=values [P=…]` (extended)

The weekly members of this feature join 004's: the rectangle's expansion containment, and any
member over the per-change budget (research R8).

## CI: `.github/workflows/verify.yml`

| Job | Change |
|-----|--------|
| `verify` | The matrix gains `values-real` and `values-rect` (`make verify-values P=real` and `P=rect`). The path filters gain `core/src/s2/latlng.rs` (the rectangle takes latitude–longitudes); `core/src/r1/interval.rs` and `core/src/s2/rect.rs` are already listed |
| `slow` | 004's pending `values` leg covers both features once it is added (004 T042) |

## `make test` (unchanged definition)

It runs the new ordinary tests:

- the FR-020 regression tests in `core/src/s2/rect_tests.rs`;
- the extended `core/tests/cpp_values_diff.rs`;
- the generic trait tests, unchanged.
