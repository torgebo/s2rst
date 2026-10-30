# Research: `Cell`'s Bounding Laws, and the Region Adaptors

Phase 0 of the plan. R0 is the Kani probe; the rest are decisions.

## R0: Planning probe (run 2026-10-30)

`probe/r0_harnesses.rs` (22 harnesses), run inside the verification crate through a temporary
`#[path]` module (not committed), 4 GiB, 12 minutes, 4 jobs, replays off. Every line is in
`probe/r0_results.txt`.

| Question | Harness | Result | Decision |
|----------|---------|--------|----------|
| `Cell`'s cap bound contains every point of its own u-v bound | `r0_cell_cap_own` (CBMC's `sqrt`), `r0_cell_cap_own_sqrtb` (`sqrt_between`) | **TIMEOUT** both | not proven (tool limit), weekly attempt by face as spec 011's vertex law did not pay; recorded with the probe's 240,000 own points and the C++ rows |
| …a vertex only | `r0_cell_cap_vertex` (`pure_sqrt`) | **TIMEOUT** | spec 008's P-CCAP-01 (the cap contains the vertices) stands on its abstract cell; no new harness |
| `Cell`'s rectangle bound contains its own points | `r0_cell_rect_own` (`atan2_contract`) | **FAILED**, 286 s: the range contract lets `from_point(p)`'s longitude fall outside the bound, which the real `atan2` never does | not proven: it needs `atan2`'s monotonicity, a value the contracts do not fix (as spec 010's P-LREG-03) |
| Containment implies intersection, preserved | `r0_ru_ci__n1`, `__n3`, `r0_ri_ci__n3` | 12.1 s, 23.2 s, 20.0 s | per change, `n ≤ 3` |
| The bounding rectangle, preserved | `r0_ru_rect__n1`, `__n2`, `__n3`; `r0_ri_rect__n2`, `__n3` | 21.5, 118.3, 441.6 s; 80.2, 236.1 s | per change `n = 1`; weekly `n = 2, 3` |
| The vertex law, preserved | `r0_ru_vertex__n3`, `r0_ri_vertex__n3`; `r0b_ru_vertex__n1`, `__n2`, `r0b_ri_vertex__n2` | **TIMEOUT** at 3 (twelve vertex keys over the tables' ten slots); 80.5, 235.2, 264.6 s | per change `n = 1`, weekly `n = 2` |
| Crash freedom, every method | `r0_ru_crash__n3`, `r0_ri_crash__n3`; `r0b_ru_crash__n1`, `r0b_ri_crash__n2` | 342.6, 579.5; 259.5, 308.2 s, all in the bounds (`Rect`'s union and intersection, `Rect::cap_bound`) | the tests per change for `n ≤ 3`; the bounds weekly for `n ≥ 1`, per change for the empty adaptors |
| `RegionIntersection::region(i)` panics past the end | `r0_ri_region_past_end` | 1.9 s, covers never | per change (`n_` harness) |

**Lesson:** the abstract region's tables hold ten keys; a statement whose members are queried at
more inputs than that gets unrecorded answers (more behaviours, never a false proof), and in R0
the twelve vertex keys of three members timed out. Member counts are chosen so the keys fit.

**Any number of members:** the adaptors' answers are `any`/`all` folds and their rectangle a
fold of `union`/`intersection`; each preservation statement is a conjunction over the members of
one member's law. The proofs at `n ≤ 3` (2 for the vertex law) and the fold's associativity give
every `n` by a written argument (data-model §4), as spec 002's leaf count did.

## R1: Upstream, read and run (v0.14.0-40-g37a4501)

Read in `s2cell.cc`, `s2cap.cc`, `s2region_union.{h,cc}` and `s2region_intersection.{h,cc}`; run
through `probe/cpp/survey.cc`.

- **`S2Cell::GetCapBound`**: the cap around the normalised u-v centre, grown by `AddPoint` of
  each of the four vertices. `AddPoint` takes `max(radius, S1ChordAngle(center, p))`, the same
  computation `Contains(p)` makes, so the cap contains its vertices exactly; it adds no margin.
- **`S2Cell::GetRectBound`** (level ≥ 1): the latitude from one diagonal pair of vertices and the
  longitude from the other, grown by `2·DBL_EPSILON` in each, then `PolarClosure`; level 0 has
  fixed bounds per face (spec 011 K1).
- **`S2Cell::Contains(const S2Point&)`**: the u-v bound expanded by `(5/3)·DBL_EPSILON` (issue
  #463), so that `S2Cell(S2CellId(p)).Contains(p)`. The bounds are not grown to match: U-017.
- **`S2RegionUnion`**, **`S2RegionIntersection`**: the rectangle bound folded with `Union` from
  `Empty()` (`Intersection` from `Full()`); `GetCapBound` is `GetRectBound().GetCapBound()` ("could
  be optimized"); `GetCellUnionBound` is the cap's; `Contains(cell)`, `MayIntersect` and
  `Contains(p)` are `any` (`all`) of the members'. `region(i)` indexes without a check.

## R2: Suite gating, names and records

| Item | Value |
|------|-------|
| Modules | `ad_crash`, `ad_mean`, `ad_reg` (the adaptors, a new part `P=adaptors`), `any_region` (the abstract region). `Cell`'s bounding laws get no harness: R0 found them out of reach (they are catalogue rows with reasons) |
| Catalogue ids | `P-CREG-NN` (`Cell`'s bounding laws, in `PROPERTIES.md` §2), `P-ADCON`, `P-ADBND`, `P-ADTST`, `P-ADREG` (the adaptors) |
| `make` | `make verify-values P=adaptors` |
| CI leg | `values-adaptors`, and a `values-adaptors` slow leg |
| Mutants | `M-974` onwards (series 9, as spec 011's) |
| Records | `values/INVENTORY.md` §14 and `PROPERTIES.md` §13 (the adaptors); `Cell`'s laws in §2 of `PROPERTIES.md` (its catalogue) |

No module in `verify/src/` starts with `ad_` or is named `cl_reg` (checked 2026-10-30).

## R3: U-017 and the documentation (FR-016, FR-017)

- `Cell::contains_point`'s rustdoc gains a paragraph: the margin, why (a point's leaf cell contains
  it), and that `cap_bound` and `rect_bound` cover the cell's own points but not every point of the
  margin, as upstream's. `cap_bound` and `rect_bound` say the same from their side.
- `CORE_TRAITS.md`'s two bounding invariants name `Cell`'s margin as an exception, with U-017.
- `UPSTREAM.md` U-017: the code, the probe's counts by offset, and a minimal input (a fixed test in
  `core/src/s2/cell_tests.rs` pins one point the cell contains and its cap does not).
- `RegionIntersection::region`'s rustdoc gains `# Panics` (an index past the end), with a
  `should_panic` test.

## R4: The abstract region (`verify/src/any_region.rs`)

`AnyRegion { id }` implements `Region` with answers from `static Mutex` tables keyed by the id
and the input's bits: one boolean per cell id for `contains_cell` and `intersects_cell`, one per
point for `contains_point`, one valid rectangle and one box cap per region, and an empty covering
(the adaptors never read their members'). A preservation statement reads each member's answers
through the same methods the adaptor calls, assumes the law of them, and asserts it of the
adaptor; the tables make the two reads one value. Since every implementor's answers are among the
abstract region's, the statement holds for every implementor that satisfies the law.

## R5: The adaptors' laws

| Law | Union | Intersection |
|-----|-------|--------------|
| Containment implies intersection | some member contains ⇒ it intersects ⇒ the union does | every member contains ⇒ every member intersects |
| The vertex law | the union misses ⇒ every member misses ⇒ no member contains a vertex | the intersection misses ⇒ some member misses ⇒ it contains no vertex, so the intersection does not |
| The bounding rectangle | a member contains `p` ⇒ its rectangle does ⇒ the union of rectangles does | every member contains `p` ⇒ every rectangle does ⇒ their intersection does |
| The bounding cap | the cap bound is the rectangle's cap bound: by composition, the rectangle law and `Rect`'s bounding-cap law (spec 010's P-LREG-03, not proven: tested, widened against U-016) | the same |
| The covering laws | the covering is the cap's: `Cap`'s covering law (spec 008's P-CPREG-04, not proven) | the same |

The first three are Kani statements over abstract members (R0); the last two are recorded with
their reasons.

## R6: C++ reference vectors (FR-018)

A new generator, `tools/refgen-cpp/generate_cell_bounds.cc`, writes
`core/tests/data/cpp_cell_bounds_vectors.csv`, checked by `core/tests/cpp_cell_bounds_diff.rs`:

| Section | Rows |
|---------|------|
| `CELL_BOUND_POINTS` | for cells of every level: the cap and rectangle bounds, and for the probe's adversarial points (corners and edge midpoints moved by 0 to 2 ε in u-v, interior points) `Contains(p)`, `GetCapBound().Contains(p)`, `GetRectBound().Contains(p)` |
| `REGION_UNION`, `REGION_INTERSECTION` | mixed members (cap, rectangle, cell, point region; written as their parameters), the bounds, the covering, the tests on a cell and a point |

The adaptors' cap radii are spec 010's documented deviation: the diff test checks them as
`cpp_latlng_diff` does (the centre bit for bit, the radius upstream's widened by the rule).

## R7: Records, CI and the evaluation

- Records: `values/INVENTORY.md` §14 (the adaptors), `PROPERTIES.md` §13 and `Cell`'s new rows in
  §2, `cell_union/PROPERTIES.md`'s P-UREG-04, -05 revisited, the index by kind, `TRUSTED_BASE.md`
  (the abstract region), `MUTANTS.md`, `VERIFIED_TYPES.md` §2.13, `CORE_TRAITS.md` (U-017's
  exception, the adaptors' instances), `COVERAGE.md`, `UPSTREAM.md` (U-017).
- CI: `values-adaptors` per change and weekly; path filters gain
  `region_union.rs`, `region_intersection.rs` and the new CSV.
- Evaluation: a spec-012 part ranking what is left: the shape-backed regions, the missing codecs,
  the two inexact remainders, and U-017's report (the maintainer's call).
