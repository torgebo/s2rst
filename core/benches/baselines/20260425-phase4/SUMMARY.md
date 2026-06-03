# Phase 4 — data layout / SoA: deferred after measurement (2026-04-25)

Toolchain: `rustc 1.94.1`. Host: Intel i7-8665U.
Bench harness: iai-callgrind 0.16.1.

## What this is

Baseline iai snapshot on the three benches the Phase 4 exit gate names
(`closest_edge_query`, `hausdorff_distance`, `shape_index`) plus a
documented decision to **defer Phase 4 without code changes**.

The baseline is captured here so the next attempt has a fixed reference
point to measure against — and so the deferral is itself reproducible.

## Why deferred

A code survey of the three target call sites + the exit-gate workloads
(see "Survey findings" below) concluded the exit gate
**≥15% on all three benches** is unreachable inside the current
Shape-trait design. The minimal change that could *plausibly* clear
the gate would require:

- a new bulk `Shape::edges_soa()` API that exposes a contiguous SoA
  view of edge vertices (rather than the existing one-at-a-time
  `Shape::edge(i)` virtual call), **and**
- a new bulk-target API like
  `target.update_distance_to_edges(&[v0], &[v1]) -> &[ChordAngle]` so
  closest-edge can drive the distance loop in batches, **and**
- restructuring `ClosestEdgeQuery::process_edges` to remove the
  per-edge `tested_edges.insert(...)` HashSet round-trip from the hot
  loop.

That's a public-API change spanning Shape, every Target impl,
ClosestEdgeQuery, and (transitively) Hausdorff. It's out of scope for
this performance plan, which explicitly says
"do not replace the public API — the `Point` type is load-bearing
everywhere" (PLAN.md §D Phase 4). The same constraint blocks the
narrower fix.

## Headline baseline numbers (committed for the next attempt)

| Bench | Instructions | Est. cycles |
|-------|-------------:|------------:|
| `closest_edge_query::find_closest_point_12288_edges`     | 24.87 M | 45.03 M |
| `closest_edge_query::find_closest_to_index_12288`        | 61.53 M | 93.08 M |
| `closest_edge_query::find_closest_scattered_10000_shapes`| 28.35 M | 42.83 M |
| `hausdorff_distance::hausdorff_polyline_1000`            | 223.80 M | 304.84 M |
| `hausdorff_distance::hausdorff_polyline_100`             | 2.97 M  | 4.08 M  |
| `hausdorff_distance::hausdorff_polygon_64`               | 3.62 M  | 5.11 M  |
| `shape_index::build_polyline_10000_edges`                | 19.58 M | 34.48 M |
| `shape_index::build_polygon_1024_edges`                  | 8.49 M  | 12.89 M |
| `shape_index::build_10000_polyline_shapes`               | 28.43 M | 42.73 M |

Full table in `iai_pre.txt`.

## Survey findings (per-target)

Subagent survey of the source confirmed the agent's analysis from
PLAN.md §B (which already flagged "shape_index" as Medium-priority):

### `ClosestEdgeQuery::process_edges` (`closest_edge_query.rs:1160`)

Inner loop body:
```rust
for &edge_id in &clipped.edges {
    if state.avoid_duplicates && !state.tested_edges.insert((shape_id, edge_id)) {
        continue;                        // HashSet lookup per edge
    }
    let edge = shape.edge(edge_id as usize);                     // virtual call
    let (dist, updated) = target.update_distance_to_edge(...);   // virtual call
    if updated { state.add_result(...); }                        // BTreeSet insert
}
```

`clipped.edges` is `Vec<i32>` (sparse edge ids), so iteration is
*indexed lookup*, not linear streaming over points. The body has
two virtual calls and a HashSet insert per edge. SoA does nothing
for any of these — the bottleneck is per-iteration side effects, not
point-coordinate cache pressure.

### `HausdorffDistanceQuery::get_directed_result` (`hausdorff_distance_query.rs:130`)

Inner loop calls `self.update_max_distance(vertex, ...)` which
internally invokes a full `ClosestEdgeQuery::find_closest_edge`. The
vertex iteration is O(V); each iteration is **O(query)** where query
is a hierarchical index search. SoA over the V vertices doesn't help
because the *vertex iteration* isn't where the cycles live — the
*per-vertex query* is. The only Phase 4 attack here would be a
`bulk_closest_distance(&[vertex]) -> &[ChordAngle]` API, which doesn't
exist and is out of scope.

### `ShapeIndex::build` / `add_face_edge` (`shape_index.rs:933`)

Per-edge fast path is `get_face` × 2 + `valid_face_xyz_to_uv` × 2 +
range check + push. The slow path clips to all 6 faces. Both have
data-dependent control flow (face-match check, padding bounds).

Buffering edges into a temporary SoA layout for autovectorization
*could* pay ~5–10% on the clipping arithmetic, but the convert-to-SoA
+ convert-back overhead would eat most of it; below the 15% gate.

## Decision

Phase 4 is deferred until either:

1. The Shape trait gains a bulk `edges_soa()` accessor (and Targets gain
   a matching bulk method), at which point Phase 4 becomes a small
   plumbing change with realistic ≥15% upside.
2. Or until Phase 1 + 3 wins are deemed insufficient and the project
   accepts the API redesign cost.

In particular, **Phase 5 (SIMD) is also deferred** as a consequence —
PLAN.md §D Phase 5 says SIMD work starts "with the SoA dot/norm2
added in Phase 4," which doesn't exist and won't until the API
redesign above.

The "ILP retry on Hausdorff after Phase 3 inlining" possibility from
the Phase 3 phase log is also closed: Hausdorff's per-vertex
ClosestEdgeQuery doesn't expose anything ILP can grip.

## Next move

Phase 6 (cache blocking) and Phase 7 (rayon, behind a feature gate)
remain unblocked — they don't depend on SoA. Phase 8 (PGO) is also
independent and would compose with the Phase 1 + 3 wins.

## Reproducing

```bash
cargo bench -p s2rst --bench shape_index --bench closest_edge_query --bench hausdorff_distance
```
