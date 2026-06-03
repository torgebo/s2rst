# Baseline — 2026-04-15 (commit 5ae7b4d)

Toolchain: `rustc 1.94.1 (e408947bf 2026-03-25)`
Host: see `host.txt`
Bench profile: `lto = "fat"`, `codegen-units = 1`, `debug = "line-tables-only"`
RUSTFLAGS: *(unset — portable baseline, no `-C target-cpu=native`)*

## iai-callgrind (instruction counts — deterministic)

| Benchmark | Instructions | Est. cycles |
|-----------|-------------:|------------:|
| `predicates::bench_sign` | 41 | 229 |
| `predicates::bench_robust_sign` | 66 | 438 |
| `predicates::bench_sign_near_collinear` | 41 | 228 |
| `point_angle::bench_point_distance` | see `iai_point_angle.txt` | |
| `point_angle::bench_point_cross` | see `iai_point_angle.txt` | |
| `point_angle::bench_point_normalize` | see `iai_point_angle.txt` | |
| `edge_crossing::bench_crossing_sign` | 3 935 | 10 734 |
| `edge_crossing::bench_crossing_sign_crossing` | 13 265 | 25 185 |
| `edge_crossing::bench_edge_crosser_chain` | 3 931 | 10 728 |
| `edge_crossing::bench_edge_or_vertex_crossing` | 3 965 | 10 968 |
| `edge_crossing::bench_intersection` | 4 403 | 10 513 |

## Criterion (wall-clock, lower/mean/upper 95% CI)

| Benchmark | Lower | Mean | Upper |
|-----------|------:|-----:|------:|
| `wall_predicates::sign` | 7.48 ns | 7.60 ns | 7.75 ns |
| `wall_predicates::robust_sign` | 16.18 ns | 16.29 ns | 16.40 ns |
| `wall_predicates::sign_near_collinear` | 7.50 ns | 7.57 ns | 7.68 ns |
| `wall_point_angle::point_distance` | 28.06 ns | 28.59 ns | 29.20 ns |
| `wall_point_angle::point_cross` | 7.95 ns | 8.10 ns | 8.34 ns |
| `wall_point_angle::point_normalize` | 8.73 ns | 8.95 ns | 9.24 ns |
| `wall_point_angle::angle_from_e6` | 1.92 ns | 1.94 ns | 1.96 ns |
| `wall_point_angle::angle_to_e6` | 11.56 ns | 11.75 ns | 11.95 ns |
| `wall_point_angle::angle_from_degrees` | 1.88 ns | 1.91 ns | 1.94 ns |
| `wall_point_angle::angle_from_radians` | 1.85 ns | 1.86 ns | 1.88 ns |
| `wall_point_angle::chord_angle_from_angle` | 23.83 ns | 24.08 ns | 24.40 ns |
| `wall_edge_crossing::crossing_sign` | 926 ns | 942 ns | 958 ns |
| `wall_edge_crossing::crossing_sign_crossing` | 3.51 µs | 3.58 µs | 3.68 µs |
| `wall_edge_crossing::edge_crosser_chain` | 871 ns | 887 ns | 909 ns |
| `wall_edge_crossing::edge_or_vertex_crossing` | 898 ns | 915 ns | 938 ns |
| `wall_edge_crossing::intersection` | 990 ns | 999 ns | 1010 ns |

## Observations seeded for later phases

- `predicates::sign` ≈ 7.6 ns and 229 estimated cycles: this is the baseline
  Phase 1 (FMA) will try to beat. 2×2 determinants internal to `triage_sign`
  are plain `a*b - c*d` today, ripe for `mul_add`.
- `point_distance` is 4× `point_cross` — not surprising given `distance` calls
  `norm` which calls `sqrt` — keep the split so Phase 1 can claim the cross
  win without `sqrt` noise.
- `edge_crosser_chain` is 6 ns faster than `crossing_sign` (887 vs 942 ns)
  confirming the chain-cached tangent optimization works; the `crossing_sign`
  variant redoes `restart_at` each call.
- `crossing_sign_crossing` is 3.8× `crossing_sign` because the "actually
  cross" case takes the `intersection`-computing branch — this is the right
  bench to watch for Phase 3 (inlining audit of `expensive_sign`).
- Criterion outlier counts on `edge_crossing` benches (up to 30%) suggest the
  host is not fully quiescent; consider rerunning on an isolated core before
  claiming phase-to-phase deltas below 5%.

## Reproducing

```bash
make baseline   # writes to a new dated dir
# or per-bench:
cargo bench -p s2rst --bench predicates
cargo bench -p s2rst --bench wall_predicates -- --save-baseline main
```
