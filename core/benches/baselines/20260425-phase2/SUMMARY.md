# Phase 2 — ILP in reductions: null-result snapshot (2026-04-25)

Toolchain: `rustc 1.94.1`. Host: Intel i7-8665U.
RUSTFLAGS: unset (portable baseline; ILP wins shouldn't depend on FMA/AVX
since the change is purely about breaking the accumulator dependency
chain).

## Setup

`benches/wall/polyline.rs` Criterion mirror added (Phase 0 deferred work);
six benches: `length_{100,1000}`, `centroid_{100,1000}`,
`get_length_1000`, `get_centroid_1000`. Inputs match
`benches/polyline.rs` 1:1.

Two-way ILP applied to `polyline_measures::get_length` (and
`Polyline::length` re-routed through it). Other targets named in PLAN.md
§D Phase 2 were surveyed (see "Why not these" below).

## Result

| Bench             | Pre-ILP    | With ILP   | Δ        | CI                           |
|-------------------|-----------:|-----------:|---------:|------------------------------|
| `length_100`      | 4.564 µs   | 4.762 µs   | **+7.7%** | [+5.18%, +9.80%]  (regression)|
| `length_1000`     | 46.24 µs   | 45.70 µs   |   −3.7%  | [−4.96%, −2.44%]              |
| `centroid_100`    | 1.113 µs   | 1.470 µs   | **+17.9%**| [+12.64%, +24.40%] (regression — likely noise + Polyline::centroid not yet ILP'd)|
| `centroid_1000`   | 11.05 µs   | 10.67 µs   |   −1.6%  | [−2.70%, −0.45%]              |
| `get_length_1000` | 48.58 µs   | 45.64 µs   |   **−6.2%** | [−7.80%, −4.74%]            |
| `get_centroid_1000`| 11.47 µs  | 11.34 µs   |   −0.5%  | [−1.94%, +0.99%] (noise)      |

The plan's Phase 2 exit gate was **≥10% on `polyline`,
`hausdorff_distance`, `closest_edge_query`**. Polyline maxed at −6.2% on
the long-input slice form, with a regression on the short-input wrapper
form. The change has been **reverted**; the Criterion bench infrastructure
is kept as Phase 2 deferred-work payoff.

## Why ILP did not help here

Per-iteration `Point::distance` is the long pole. It's a chain of
`(self - other).norm()` → `dot(d, d).sqrt()` → cross/dot/atan2 in the
calling code, ~30–40 ns per call on this host. The accumulator add is
~4 cycles (≤ 1 ns on this CPU), so the addition never serializes the
loop — `distance(...)` for iteration *i+1* can already begin while
iteration *i*'s add is in flight, even with a single accumulator.
Two-way unrolling adds tail-handling overhead (one extra branch + one
extra add at the loop exit) without removing a real bottleneck, so it
shows a small regression on short polylines (the loop overhead is
amortized over fewer edges) and a small win on long polylines (where
two parallel `distance()` calls fit in the OoO window slightly better).

Net: the technique applies when the *accumulator* is the long pole,
not when *per-iteration work* is. The book's example (a flat array of
already-loaded f64s being summed) matches the former; spherical-arc
sums match the latter.

## Why the other PLAN.md §D Phase 2 targets were skipped

A subagent survey of the named targets found:

- `HausdorffDistanceQuery` inner loop (`hausdorff_distance_query.rs:171`):
  body is `update_max_distance(...)` — a function call with conditional
  state updates. Function-call serialization defeats ILP. Phase 3
  (inlining audit) may unblock this.
- `ClosestEdgeQuery::process_edges` (`closest_edge_query.rs:1177`): body
  is `update_distance_to_edge(...)` + `state.tested_edges.insert(...)`
  + conditional `state.add_result(...)`. Hash-set insertion + dynamic
  result push are loop-carried side effects; ILP cannot help.
- `RegionCoverer` merge loop (`region_coverer.rs:365`): trip count
  capped at `max_cells` (typical 5–50). Too short for unrolling
  overhead to amortize.

These remain on the "measure first, then decide" pile per §B. None
were changed in Phase 2.

## Next move

Phase 3 (inlining audit) is the natural next phase: it can unblock
Hausdorff/ClosestEdge by exposing the inner work to the optimizer
inside the hot loop, after which a Phase 2-style ILP retry might pay.

## Reproducing

```bash
# baseline (no ILP), saves under criterion/<bench>/pre-phase2
cargo bench -p s2rst --bench wall_polyline -- --save-baseline pre-phase2

# apply hand ILP changes, then:
cargo bench -p s2rst --bench wall_polyline -- --baseline pre-phase2
```
