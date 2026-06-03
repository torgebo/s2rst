# Phase 6 — cache blocking: deferred after measurement (2026-04-25)

Toolchain: `rustc 1.94.1`. Host: Intel i7-8665U (32 KB L1d per core).
Bench harness: iai-callgrind 0.16.1 (instructions + cache hit counters).

## What this is

iai cache-hit numbers on the two benches the Phase 6 exit gate names
(`boolean_operation`, `region_coverer`) plus a documented decision to
**defer Phase 6 without code changes**. iai-callgrind reports L1/LL/RAM
hits per bench, which serves as a built-in cachegrind for this purpose.

## Headline cache numbers (committed for the next attempt)

L1 hit ratio = `L1 / (L1 + LL + RAM)`. Cache blocking only helps when
that ratio is materially below 99% AND the loop reuses data across
outer iterations.

| Bench | Instr | L1 Hits | LL Hits | RAM Hits | **L1 hit %** | CPI |
|-------|------:|--------:|--------:|---------:|-------------:|----:|
| `boolean_operation::union_regular_polygon_256_vertices` | 16.08 M | 21.33 M | 80.8 K | 6.0 K | **99.59%** | 1.36 |
| `boolean_operation::three_overlapping_bars` | 1.84 M | 2.59 M | 2.7 K | 3.2 K | **99.77%** | 1.48 |
| `boolean_operation::union_regular_polygon_64_vertices` | 3.31 M | 4.65 M | 1.8 K | 3.6 K | **99.88%** | 1.44 |
| `region_coverer::bench_covering_loop` | 3.32 M | 4.70 M | 5.1 K | 1.0 K | **99.87%** | 1.44 |
| `region_coverer::bench_covering_loop_256_vertices` | 0.89 M | 1.21 M | 1.4 K | 2.3 K | **99.69%** | 1.47 |
| `region_coverer::bench_covering_cap` | 90 K | 116 K | 19 | 363 | **99.67%** | 1.42 |

All target benches sit at **≥99.5% L1 hit rate**. The CPI numbers
(1.36–1.48) are typical for compute-bound geometric code on Skylake-class
cores, not memory-bound code (which would sit at CPI > 3 with cache
misses dominating).

There is no cache pressure to relieve. Cache blocking would add loop
overhead with zero upside.

## Survey findings (per-target, why blocking doesn't fit)

A subagent survey confirmed the iai numbers — the targets are not
shape-blockable in addition to not being cache-bound:

### `BooleanOperation::process_edge` (`crossing_processor.rs:755`)

Innermost loop is a `while eid < chain_limit` driven by a sparse
crossing iterator (`next_crossing.a_id().edge_id`). Each edge is
processed once, in crossing-detection order, not in cartesian-product
nested order. There is no `for outer { for inner { ... } }` shape to
block.

### `RegionCoverer::covering_internal` (`region_coverer.rs:587`)

Outer loop is `while let Some(cand) = self.pq.pop()` — heap-driven, not
nested. The "inner" for-loop over children has ≤4 elements per pop. No
data is reused across outer iterations that blocking could amplify.

### `CellIteratorJoin::process_nearby` (`cell_iterator_join.rs:146`)

This *is* a classic `for cell_a { for cell_b { ... } }`, but
`cells_a.len() ≤ 12` and `cells_b.len() ≤ 12` (capped by `COVER_LIMIT`).
Working set: 12 × 12 × 64 B ≈ **9 KB** — already well below the 32 KB
L1d. The inner work is `distance_to_cell` (~30–50 cycles of geometric
math), so the loop is compute-bound at this size; cache pressure
would only emerge at >300×300 cells, which the API's `COVER_LIMIT`
prevents.

## Decision

Phase 6 is deferred. The cache-blocking technique does not apply to
this codebase's algorithms in their current form. The exit gate is
unreachable not because the implementation is bad but because the
loops are either (a) data-driven not nested, (b) heap-driven not
nested, or (c) bounded to a working set that already fits in L1d.

A blocking pass would only become relevant if the algorithm changed —
e.g., if `BooleanOperation` switched from sparse crossing iteration to
a sweepline/sort-then-pair shape, or if `CellIteratorJoin` removed its
12-cell cap and operated on full polylines (1000+ cells). Neither is
on the project roadmap.

## Net plan status

The active Phases that remain are:
- Phase 7 (rayon, behind a feature gate): independent of SoA/cache;
  unblocked.
- Phase 8 (PGO): independent; would compose multiplicatively with the
  Phase 1 + Phase 3 wins.

Phases 4 (SoA), 5 (SIMD), 6 (cache blocking) are all measured-and-
deferred — the underlying assumptions about hot-loop shape don't hold
for this codebase's algorithms.

## Reproducing

```bash
cargo bench -p s2rst --bench boolean_operation --bench region_coverer
# inspect L1/LL/RAM hit counts in the output;
# any L1 hit % < 95% on a hot bench is a real blocking opportunity.
```
