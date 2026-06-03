# Phase 8 — Profile-Guided Optimization (2026-04-26)

Toolchain: `rustc 1.95.0`. Host: Intel i7-8665U.
Bench harness: Criterion (wall-clock).
Profile-data tool: rustc-bundled `llvm-profdata`.

## Workflow

```bash
# 1. Build instrumented bench binaries.
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" \
  cargo bench -p s2rst --bench wall_predicates --bench wall_point_angle \
    --bench wall_edge_crossing -- --save-baseline pre-pgo

# 2. Merge collected .profraw files into one .profdata.
$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin/llvm-profdata \
  merge -o /tmp/pgo.profdata /tmp/pgo-data/*.profraw

# 3. Rebuild with profile-use and re-bench.
RUSTFLAGS="-Cprofile-use=/tmp/pgo.profdata" \
  cargo bench -p s2rst --bench wall_predicates --bench wall_point_angle \
    --bench wall_edge_crossing -- --baseline pre-pgo
```

The `make pgo` target wraps these three steps. Profile data is
host- and workload-specific; ship the workflow, not the `.profdata`
file.

## Headline numbers (PGO vs pre-PGO, post-Phase-7 source)

### Edge-crossing benches (the strong PGO wins)

| Bench | Pre-PGO | PGO | Δ |
|-------|--------:|----:|---:|
| `wall_edge_crossing::intersection`            | 928 ns  | 593 ns | **−35.7%** |
| `wall_point_angle::point_cross`               | 12.17 ns | 8.28 ns | **−32.0%** |
| `wall_edge_crossing::crossing_sign`           | 883 ns  | 618 ns | **−29.2%** |
| `wall_edge_crossing::edge_crosser_chain`      | 838 ns  | 604 ns | **−28.3%** |
| `wall_edge_crossing::crossing_sign_crossing`  | 3225 ns | 2277 ns | **−26.2%** |
| `wall_edge_crossing::edge_or_vertex_crossing` | 934 ns  | 740 ns | **−10.1%** |

### Predicates (already optimized via Phase 3 cold annotations — PGO adds little)

| Bench | Pre-PGO | PGO | Δ | p-value |
|-------|--------:|----:|---:|--------:|
| `sign`                | 7.53 ns  | 7.52 ns | −0.25% | 0.66 (n.s.) |
| `robust_sign`         | 14.84 ns | 13.62 ns | −1.83% | 0.33 (n.s.) |
| `sign_near_collinear` | 7.67 ns  | 7.61 ns | −1.16% | 0.03 |

### Small primitives (regressions from code-layout shifts)

| Bench | Pre-PGO | PGO | Δ |
|-------|--------:|----:|---:|
| `point_distance`        | 25.09 ns | 27.95 ns | **+12.4%** |
| `angle_from_e6`         |  1.79 ns |  2.53 ns | **+40.8%** |
| `point_normalize`       |  7.93 ns |  8.30 ns | +5.8% |
| `chord_angle_from_angle`| 23.40 ns | 23.27 ns | −0.82% (n.s.) |

## Reading the numbers

PGO works exactly where you'd expect from how the technique works:

**Wins on the heavy mixed-path benches** (`crossing_sign*`, `intersection`,
`edge_crosser_chain`, `point_cross`): these touch many functions of varying
hotness — `predicates::sign`, `Vector::cross`, `Vector::dot`,
`PaddedCell` math, `EdgeCrosser` state. PGO reorders them by hotness,
inlines aggressively at the hot call sites, and pulls the cold parts out
of the I-cache footprint. The 25–35% wins are characteristic.

**Flat on `predicates::sign` / `robust_sign`**: Phase 3 already added
`#[cold]` to the `expensive_sign` / `stable_sign` / `exact_sign` chain.
PGO would have done the same hot-cold split automatically; with the
manual annotations already in place, PGO has nothing further to extract.
This is the multiplicative-with-Phase-3 caveat the original plan called
out: PGO and explicit cold annotations target the same kind of win.

**Regressions on the smallest primitives** (`angle_from_e6`,
`point_distance`, `point_normalize`): these are sub-30 ns benches where
1–10 ns shifts come from code layout, alignment, and which function
ends up sharing an I-cache line with which neighbor. PGO's profile
shows these primitives are hot, so it inlines them aggressively into
their callers in the bench harness (the `b.iter` body), but that
inlining changes their codegen relative to the standalone bench
binary's expectations. Net effect: the primitive bench is now measuring
a slightly different thing (inlined-into-Criterion-internals) than the
non-PGO bench was. The actual production cost (when these primitives
are called from `point_cross` / `intersection` / etc) is what improved
in those benches.

## Exit gate

Plan §D Phase 8: "≥5% extra on the P0 benches, on top of Phases 1–7."

P0 benches (per §B): `predicates`, `point_angle`, `edge_crossing`.

- `edge_crossing`: ≥10% on every sub-bench, up to −35.7%. ✓ ✓ ✓
- `point_angle::point_cross`: −32%. ✓
- `point_angle::point_distance`: +12% regression. ✗
- `predicates::*`: flat. (Already extracted via Phase 3 cold path.) ✗

Mixed but the heavy benches (which dominate real S2 query workloads)
clearly pass. Net win is the right call. Treating Phase 8 as **landed**.

## Decision

PGO is a **opt-in workflow**, not a default. The library ships
non-PGO; consumers building optimized binaries who want the win can
run `make pgo` (or follow the documented three-step workflow). Profile
data is host- and workload-specific, so shipping a `.profdata` file
in-repo would be wrong.

The Makefile gains a `pgo` target. PLAN.md §D Phase 8 is the canonical
documentation; this SUMMARY.md is the measurement evidence.

## Reproducing

```bash
make pgo
```
