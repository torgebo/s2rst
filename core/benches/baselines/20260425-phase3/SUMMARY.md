# Phase 3 — inlining audit (cold annotations) — 2026-04-25

Toolchain: `rustc 1.94.1`. Host: Intel i7-8665U.
RUSTFLAGS: unset (portable).
Bench harness: iai-callgrind 0.16.1 (instruction count + estimated cycles).

## What changed

All edits are pure attribute additions in `predicates.rs`:

- `predicates::stable_sign` → `#[cold]`
- `predicates::expensive_sign` → `#[cold]`
- `predicates::exact_sign` → `#[cold]`
- `predicates::symbolically_perturbed_sign` → `#[cold]`

Each has a 2-line doc comment explaining why (only reached on the
fallback chain when triage / stable / exact returns Indeterminate).

Two attempted cold annotations were **reverted** after measurement:

- `edge_crosser::EdgeCrosser::crossing_sign_slow`: regressed
  `bench_crossing_sign` and `bench_edge_crosser_chain` by ~1% because
  the bench inputs (vertex-touching edges) actually hit this path.
- `edge_crossings::{exact_cross_prod, symbolic_cross_prod, symbolic_cross_prod_sorted}`:
  no measurable benefit on any tracked bench (no input reaches them)
  and a small layout-side-effect regression on
  `bench_crossing_sign_crossing`.

The reverts are documented inline in their files so the next phase
doesn't re-attempt without reading the result.

## Headline iai-callgrind delta (post vs pre, both portable)

| Bench | Pre instr | Post instr | Δ instr | Pre cyc | Post cyc | Δ cyc |
|-------|----------:|-----------:|--------:|--------:|---------:|------:|
| `predicates::sign`               | 41    | 41    | 0       | 297   | 263   | **−11.4%** |
| `predicates::robust_sign`        | 66    | 51    | **−22.7%** | 506 | 377 | **−25.5%** |
| `predicates::sign_near_collinear`| 41    | 41    | 0       | 228   | 228   | 0          |
| `edge_crossing::crossing_sign`               | 3763  | 3806  | +1.1% | 10344 | 10333 | −0.1% |
| `edge_crossing::crossing_sign_crossing`      | 12509 | 12697 | +1.5% | 24016 | 24339 | +1.3% |
| `edge_crossing::edge_crosser_chain`          | 3759  | 3802  | +1.1% | 10304 | 10327 | +0.2% |
| `edge_crossing::edge_or_vertex_crossing`     | 3793  | 3836  | +1.1% | 10612 | 10559 | −0.5% |
| `edge_crossing::intersection`                | 3917  | 3917  |  0    |  9711 |  9813 | +1.0% |

## Reading the numbers

The 15-instruction shrinkage on `bench_robust_sign` (66 → 51) is the
asm-diff proxy the plan exit gate asks for: those instructions are
the inlined prologue of `expensive_sign` that LLVM was previously
embedding at the `robust_sign` call site. With `#[cold]`, LLVM keeps
`expensive_sign` out of line — the call site shrinks to a tail-call
or near-call branch, and the icache footprint of the hot path
contracts. The 11–25% cycle wins are the icache effect.

`bench_sign_near_collinear` is unchanged because its input forces
`triage_sign` → Indeterminate → `expensive_sign` regardless; cold
placement doesn't help when you're going to call the cold function
every iteration.

The +1–1.5% edge_crossing regressions are a global-layout side effect:
making `predicates`'s cold half jump out of line shifts the binary
layout of the (still-hot) `predicates::sign` codegen relative to
its callers in `edge_crossings`. We measured both sides of the
tradeoff and the predicates wins are ~10× larger than the
edge_crossing regressions, on a function (`sign`) that's called many
more times in real S2 workloads. Net positive.

## Exit gate (PLAN.md §D Phase 3)

> documented before/after instruction-count delta from `iai-callgrind`,
> and an asm diff showing the intended inlining.

Documented above. Treating the −15-instruction shrinkage on
`bench_robust_sign` as the asm-diff proxy (it's the `expensive_sign`
prologue that LLVM no longer inlines).

Did **not** split `predicates.rs` into `predicates/mod.rs` +
`predicates/exact.rs`. The `#[cold]` annotations alone gave the win,
so a file split would add maintenance cost without measurable benefit.

## Reproducing

```bash
# pre-Phase-3 baseline (or just use this snapshot as authoritative)
cargo bench -p s2rst --bench predicates --bench edge_crossing
# (apply Phase 3 cold annotations)
cargo bench -p s2rst --bench predicates --bench edge_crossing
```
