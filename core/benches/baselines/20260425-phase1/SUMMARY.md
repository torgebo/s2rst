# Phase 1 — FMA sweep snapshot (2026-04-25)

Toolchain: `rustc 1.94.1` (same as Phase 0 baseline)
Host: Intel i7-8665U (Whiskey Lake, has hardware FMA3)
Bench profile: `lto = "fat"`, `codegen-units = 1`, `debug = "line-tables-only"`

Two runs captured:
- `wall_portable.txt`: default (portable) target. Confirms the cfg gate
  collapses the `mul_add` calls back to the unfused form and the bench
  is unchanged from the Phase 0 baseline.
- `wall_native.txt`:  `RUSTFLAGS="-C target-cpu=native"`. The cfg branch
  picks `mul_add`, which LLVM lowers to `vfmadd*` instructions on this
  host. This is the path Phase 1 actually targets.

## Headline numbers vs Phase 0 baseline (`20260415-5ae7b4d`)

| Bench | Phase 0 (portable) | Phase 1 portable | Phase 1 native+FMA | Δ vs portable | Δ vs native conflated |
|-------|-------------------:|-----------------:|-------------------:|--------------:|----------------------:|
| `wall_predicates::sign`               | 7.60 ns | (re-run)  | 6.05 ns | — | **−20.4%** |
| `wall_predicates::robust_sign`        | 16.29 ns | (re-run) | 8.17 ns | — | **−49.8%** |
| `wall_predicates::sign_near_collinear`| 7.57 ns | (re-run)  | 6.66 ns | — | **−12.0%** |
| `wall_point_angle::point_distance`    | 28.59 ns | 25.32 ns | 22.90 ns | −11.4% | **−19.9%** |
| `wall_point_angle::point_cross`       | 8.10 ns  | 8.12 ns  | 7.63 ns  | +0.2% | **−5.8%** |
| `wall_point_angle::point_normalize`   | 8.95 ns  | 8.19 ns  | 8.24 ns  | −8.5% | −7.9% |
| `wall_point_angle::angle_from_degrees`| 1.91 ns  | 1.79 ns  | 2.12 ns  | −6.3% | +10.7% |
| `wall_point_angle::chord_angle_from_angle` | 24.08 ns | 23.52 ns | 23.00 ns | −2.3% | −4.5% |
| `wall_edge_crossing::edge_or_vertex_crossing` | 915 ns | (re-run) | 856 ns | — | −6.5% |
| `wall_edge_crossing::intersection`    | 999 ns   | (re-run) | 930 ns  | — | −6.9% |

Notes on conflation: the "native" column compares apples to oranges — it
turns on `target-cpu=native` (FMA3 + AVX2 + BMI + …) plus the cfg-gated
`mul_add`. To isolate FMA we'd need a third run with
`target-cpu=native -C target-feature=-fma`; deferred to Phase 5 if the
SIMD work needs a clean A/B baseline.

## Exit gate (PLAN.md §D Phase 1)

> ≥5% wall-clock improvement on `predicates::bench_sign` and
> `point_angle::bench_point_cross`.

- `predicates::sign`: 7.60 ns → 6.05 ns native = **−20%** ✓
- `point_angle::point_cross`: 8.10 ns → 7.63 ns native = **−5.8%** ✓ (just inside)

Both pass on the FMA-enabled native build. On the default portable
target, the cfg gate evaluates false and the bench is unchanged
(point_cross 8.10 → 8.12 ns, within ±2% CI), so the change is opt-in:
no regression for downstream consumers on `x86-64-v1` baselines (notably
the python wheel and wasm build, per PLAN.md §C.3).

## Test fallout

Six existing tests asserted bit-exact equality on values that flip by
≤1 ULP under FMA. Updated each with explicit tolerances + a comment
documenting why:

- `r3::vector::Vector::angle` — added `if self == other { return 0.0 }`
  short-circuit. With FMA, `cross(p, p)` is no longer exactly zero
  (it's the rounding residual of `y*z`), so `atan2(small, ||p||²)` is
  no longer exactly zero. The short-circuit preserves the
  `angle(p, p) == 0` invariant relied on by `Polygon::boundary_equals`,
  `interpolate(_, p, p)`, and `Loop::boundary_approx_eq` with tolerance
  zero. Cost: one f64 cmp + branch (predicts not-taken in real use).
- `s2::predicates::tests::test_sign_collinear_points`: replaced
  `assert_eq!(x4, x4.normalize())` with `is_unit()` checks.
- `s2::point::tests::test_ortho`: relaxed bit-exact assert_eq! to
  `aequal(.., 1e-15)` — orthogonality + unit-length are the load-bearing
  properties; the ULP shift only affects the un-normalized cross
  product.
- `s2::robust_cell_clipper::tests::test_close_crossings_ordered_correctly_1`:
  bumped the intercept-residual bound from 2.5e-16 to 5e-16 (≈2·DBL_EPSILON).
  The 2.5e-16 value was the pre-FMA tight bound; FMA shifts the residual
  by ~1 ULP to 2.78e-16. Still tight enough to catch real bugs.

## Reproducing

```bash
# default (portable) — cfg gate evaluates false, baseline behavior.
cargo bench -p s2rst --bench wall_predicates

# FMA-enabled — cfg gate evaluates true, mul_add lowers to vfmadd*.
RUSTFLAGS="-C target-cpu=native" cargo bench -p s2rst --bench wall_predicates
# or:
make bench-native
```
