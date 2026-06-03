# Phase 7 — rayon per-face parallelism: implemented, measured, reverted (2026-04-26)

Toolchain: `rustc 1.94.1`. Host: Intel i7-8665U (4 cores / 8 threads).
Bench harness: Criterion (wall-clock).
Builds: `--release` (default profile-bench inherited).

## What this is

Full implementation of per-face parallelism in `ShapeIndex::build` behind
a `parallel = ["dep:rayon"]` feature gate, plus the wall benches that
measure the speedup, plus the **revert** after the numbers came in.

The Cargo.toml changes (rayon dep + parallel feature) and the `build()`
parallel branch were reverted; the refactor that exposed
`update_face_edges_into` / `update_edges_into` as free-function forms
was **kept** because it makes the build-state plumbing explicit and is
useful for future per-cell parallelism attempts.

The wall bench `benches/wall/shape_index.rs` is also kept — it is
permanent Phase 0 deferred-work payoff for the `shape_index` area.

## Headline numbers

Sequential (`cargo bench -p s2rst --bench wall_shape_index`):

| Bench | Mean (95% CI) |
|-------|--------------:|
| `build_polyline_1000`         | 710 µs (704 – 716) |
| `build_polyline_10000`        | 4.75 ms (4.68 – 4.82) |
| `build_10000_polyline_shapes` | 7.99 ms (7.93 – 8.07) |

Parallel (`cargo bench -p s2rst --bench wall_shape_index --features parallel`):

| Bench | Mean (95% CI) | Δ vs sequential |
|-------|--------------:|----------------:|
| `build_polyline_1000`         | 1.23 ms (1.21 – 1.26) | **+70% regression** |
| `build_polyline_10000`        | 5.34 ms (5.31 – 5.38) | **+12% regression** |
| `build_10000_polyline_shapes` | 8.01 ms (7.94 – 8.09) | +0.2% (within CI) |

**Exit gate ≥4× speedup not met. Net regression on 2 of 3 benches.**
Reverted.

## Why per-face parallelism failed

The bench inputs that mirror the iai bench (`make_zigzag_points`) place
all polyline edges in a tiny region (lat 45–55, lng −120 to −119.5).
That region maps to **one or two cube faces**. Per-face parallelism
spawns 6 rayon workers; the 4–5 with empty `face_edges` finish
immediately. Net effect: rayon overhead (task spawn, work-stealing
synchronization, thread wakeup) is paid for zero parallel work. The
`build_polyline_1000` regression is dominated by this: the 1000-edge
work is ~700 µs sequentially, but rayon's per-task setup costs ~1
ms when threads are cold (first-build wakeup in the bench's hot loop).

`build_10000_polyline_shapes` does spread edges across all 6 faces but
shows zero net change — the per-face work (~1.3 ms each) is just
balanced enough to offset rayon's overhead, no more. To beat
sequential meaningfully, per-face work would need to be
≥10 ms each (≥60 ms total), which corresponds to ~50K-edge shape
indexes — not the typical workload.

A more realistic analysis: most S2 use cases are **regional**
(single-country polygons, city-scale polylines, sensor footprints).
These all concentrate on 1–2 faces. Per-face parallelism is the wrong
granularity. The right granularity for a future attempt would be
**per-cell within a face** (rayon-recursive subdivision in
`update_edges_into`), but that requires lock-free or per-thread
accumulators inside the recursive walk and is a much bigger change.

## What's kept from this Phase (deferred-work payoff)

- `core/src/s2/shape_index.rs::update_face_edges_into` and
  `update_edges_into` are kept as free-function forms (the original
  `&mut self` versions are removed). This makes the build-state
  plumbing explicit and unblocks future per-cell parallelism without
  re-doing the refactor.
- `core/benches/wall/shape_index.rs` Criterion mirror is kept; it's
  permanent measurement infrastructure for the `shape_index` area
  (Phase 0 deferred work).
- `wall_shape_index` `[[bench]]` entry in `core/Cargo.toml` is kept.

## What's reverted

- `rayon` optional dependency.
- `parallel = ["dep:rayon"]` feature.
- The `#[cfg(feature = "parallel")]` block in `ShapeIndex::build` that
  drove the per-face rayon path. Replaced with an inline comment that
  records this experiment so the next Phase 7 retry knows what was
  tried.

## Decision

Phase 7 deferred. The plan exit gate of "≥4× on bulk benches with
`--features parallel`, zero regression without" is unreachable with
per-face granularity for typical S2 workloads. A future Phase 7 retry
should target per-cell parallelism inside `update_edges_into` (deeper
in the recursion, where the work is more uniform and the granularity
can be tuned), but that's a much larger implementation than the plan's
~50–100 LOC estimate suggested.

## Reproducing

```bash
# revert is in the source tree; re-apply the parallel branch on a
# scratch branch to reproduce the regression measurement, then:
cargo bench -p s2rst --bench wall_shape_index -- --save-baseline phase7-seq
cargo bench -p s2rst --bench wall_shape_index --features parallel -- --baseline phase7-seq
```
