# Data Model: Formal Verification of `PointRegion`, and `CellUnion`'s Floating-Point Methods

The entities of [spec.md](spec.md) as this feature defines them: the reference description,
the input domains, the stubs, the catalogue, the inventory, the library changes and the
mutants. Research decisions are in [research.md](research.md); measurements in its R0.

## 1. The reference description (`verify/src/region_spec.rs`)

Written from `s2point_region.{h,cc}` and `s2cell_union.{h,cc}` (research R1, R4). It never calls
the operations it describes.

| Item | Definition | Upstream |
|------|------------|----------|
| `pr_contains(p, q)` | `p.x == q.x ∧ p.y == q.y ∧ p.z == q.z` (IEEE equality: ±0 agree, NaN contains nothing) | `Contains(p)`, `s2point_region.h:64` |
| `pr_contains_cell` | `false` | `Contains(cell)`, `s2point_region.h:62` |
| `union_contains_leaf(ds, leaf)` | `union_spec::contains_id(ds, leaf)` (002) | `Contains(S2CellId(p))`, `s2cell_union.cc:564` |
| `AVG_LEAF_AREA` | `ldexp(4π/6, −60)`, upstream's `AverageArea(30)` | `s2metrics.h` `kAvgArea` |

**Drift guards** (`region_spec_tests.rs`, on the library's source text, as 010's):

| Guard | The library's text is |
|-------|-----------------------|
| `cap_bound_is_upstreams` | `CellUnion::cap_bound`: the empty cap for no ids; the centroid as the sum of `average_area() · to_point()`; `(1, 0, 0)` when it is the zero vector; `add_cap` of each cell's `cap_bound()` |
| `rect_bound_is_upstreams` | the fold of `union` from `Rect::empty()` |
| `cell_union_bounds_are_the_caps` | `cell_union_bound` is `self.cap_bound().cell_union_bound()`, for both `PointRegion` and `CellUnion` |
| `areas_sum_from_zero` | `approx_area` and `exact_area` fold from `0.0` (after FR-018) |
| `expand_level_is_upstreams` | `expand_by_radius`'s level: `MIN_WIDTH.max_level(r)`, the face-width double expansion, `min(min_level + diff, radius_level)` clamped to 30 |

## 2. Input domains

| Generator | Domain |
|-----------|--------|
| `any_triple()`, `any_plain_triple()`, `any_box_point()` | `AnyTriple`, `PlainTriple`, `BoxPoint` (006's) |
| `any_cell()` with `any_uv_bound` | `ValidCell` (004's, the coverage review's abstraction) |
| `union_domain::any_valid_union::<N>()` | `ValidUnion(N)` (002's), `N ∈ {0, 1, 2, 3}`, concrete |
| `any_angle()`, `kani::any::<u8>()` | `AnyAngle` (007's), every `u8` |

## 3. Stubs (`verify/src/stubs.rs`, extended)

| Stub | Replaces | Returns | Rests on |
|------|----------|---------|----------|
| `pure_leaf` | `CellId::from_point` | one valid leaf per point | L-PR-01 (weekly), the native test |
| `pure_cell_cap` | `Cell::cap_bound` | one non-empty cap per cell, centre in the unit box, length² in [0, 4] | native test on real cells (`libm_contracts.rs`) |
| `any_center` | `CellId::to_point` | any point in the unit box | the same test |
| `pure_cell_area` | `Cell::approx_area`, `Cell::exact_area` | one finite non-negative value per cell | uninterpreted |
| `pure_avg_area` | `Cell::average_area_for_level` | one positive finite value per level | uninterpreted; P-UAREA-04 pins the real function (KI-013: `powi` is not one value per input to the checker) |
| `sqrt_between` | `f64::sqrt` | `sqrt_contract`'s values that also lie between `x` and 1 | true of the correctly rounded root (`x` and 1 are representable); `libm_contracts::sqrt` gains the clause |

**Lemma**:

| ID | Statement | Harness |
|----|-----------|---------|
| L-PR-01 | `CellId::from_point(p)` is a valid leaf wherever it returns | `cl_crash::c_fg_from_point`'s assertion (weekly, its box-point domain); native: every one of 1,001,331 triples in release builds (`probe/rust/src/bin/leaf.rs`) |
| L-PR-02 | if `a` contains `b` and `a` is not empty, `a.add_cap(c)` contains `b` | `uf_bnd::l_pr_02` (`pure_chord`, `pure_add`, as P-CPGRO-02) |

## 4. The property catalogue

### 4.1 `PointRegion` (`verify/values/PROPERTIES.md` §12)

| ID | Kind | Statement | Domain | Harness | Expected |
|----|------|-----------|--------|---------|----------|
| P-PRCON-01 | correct result | `point()` returns `new`'s argument bit for bit | `AnyTriple` | `pr_con::p_prcon_01` | proven |
| P-PRBND-01 | correct result | `cap_bound()` is `Cap::from_point(p)`: centre `p` bit for bit, radius zero | `AnyTriple` | `pr_bnd::p_prbnd_01` | proven |
| P-PRBND-02 | correct result | `rect_bound()` is the single `LatLng` of `p`: `lat.lo == lat.hi == ll.lat`, `lng.lo == lng.hi == ll.lng` (π for −π), valid | `PlainTriple` | `pr_bnd::p_prbnd_02` (`pure_from_point`) | proven-under-contract |
| P-PRBND-03 | correct result | `cell_union_bound()` is the four (three at a face corner) cells around `p`'s leaf at level 29, so it holds 3 or 4 ids | `AnyTriple` | `pr_bnd::p_prbnd_03` (`pure_leaf`, `asin_contract`), weekly (R0: 563.4 s) | proven-under-contract |
| P-PRCEL-01 | correct result | `contains_cell(c)` is false | `AnyTriple × ValidCell` | `pr_cel::p_prcel_01` | proven |
| P-PRCEL-02 | correct result | `intersects_cell(c) == c.contains_point(p)` | `BoxPoint × ValidCell` | `pr_cel::p_prcel_02` | proven |
| P-PRCEL-03 | correct result | `contains_point(q) == pr_contains(p, q)` | `AnyTriple²` | `pr_cel::p_prcel_03` | proven |
| P-PREQ-01 | law | equality is the point's, and the equality laws hold on points without NaN | `PlainTriple³` | `pr_eq::p_preq_01` | proven |
| P-PRDES-01 | correct result | deserialisation rejects a point that is not unit length and keeps a unit one (FR-019) | every serialised `PointRegion` | `point_region` tests | tested (not counted) |
| P-PRREG-01 | law | containment implies intersection | `BoxPoint × ValidCell` | `pr_reg::p_prreg_01` | proven |
| P-PRREG-02 | law | the covering covers every contained cell | — | by composition: vacuous by P-PRCEL-01 | proven-by-composition |
| P-PRREG-03 | law | the vertex law: `c.contains_point(c.vertex(k))` for every cell and `k`, so a cell the region of `p` misses has no vertex equal to `p` | `ValidCell × CellVertex` | `pr_reg::p_prreg_03__f{0..5}` (`sqrt_between`), weekly, one per face (R0: 12-minute timeout unsplit) | proven-under-contract, or a tool limit (tested: the probe's 80,000 vertices) |
| P-PRREG-04 | law | the bounding cap contains the point | `BoxPoint` | `pr_reg::p_prreg_04` | proven |
| P-PRREG-05 | law | the bounding rectangle contains the point (FR-010) | `PlainTriple` | `pr_reg::p_prreg_05` (`pure_from_point`) | proven-under-contract |
| P-PRREG-06 | law | the covering contains the point (FR-010) | `BoxPoint` | `pr_reg::p_prreg_06` (`pure_leaf`, `asin_contract`), weekly (R0b: 452.8 s) | proven-under-contract |

### 4.2 `CellUnion`'s floating-point half (`verify/cell_union/PROPERTIES.md`, appended)

| ID | Kind | Statement | Domain | Harness | Expected |
|----|------|-----------|--------|---------|----------|
| P-UPT-01 | correct result | `contains_point(p)` (inherent and `Region`) is `union_contains_leaf` of `p`'s leaf | `ValidUnion(n ≤ 3) × AnyTriple` | `uf_pt::p_upt_01__n{0..3}` (`pure_leaf`) | proven-within-bounds, under contract |
| P-UBND-01 | correct result | `rect_bound()` contains each cell's `rect_bound()` | `ValidUnion(n ≤ 3)` | `uf_bnd::p_ubnd_01__n{1,2}` (`any_cell_bound`), `__n3` weekly (R0: 27.7 s, 81.1 s, 333.7 s) | proven-within-bounds |
| P-UBND-02 | correct result | `cap_bound()` contains each cell's `cap_bound()` | every valid union | by composition: the fold starts from a non-empty cap (`Cap::from_point`); P-CPGRO-02 (after `add_cap(b)` the cap contains `b`) and L-PR-02 (a cap that contains `b` still contains it after another `add_cap`); `uf_bnd::p_ubnd_02__n1` weekly as a direct check (R0c: 357.4 s) | proven-by-composition |
| P-UBND-03 | correct result | the empty union's bounds are the empty cap and the empty rectangle | `n = 0` | `uf_bnd::p_ubnd_03` | proven |
| P-UBND-04 | correct result | the constructions are upstream's text (the centroid, the fallback, the fold, the covering through the cap) | — | drift guards (§1) | proven-by-composition |
| P-UAREA-01 | correct result | `average_based_area()` is the leaf area times `leaf_cells_covered()` | `ValidUnion(n ≤ 3)` | `uf_area::p_uarea_01__n1` (`pure_avg_area`; R0: `n = 2` and `3` time out) | proven-within-bounds (`n ≤ 1`) |
| P-UAREA-02 | correct result | `approx_area()` is the sum of the cells' areas in order from +0; +0 for the empty union (after FR-018) | `ValidUnion(n ≤ 3)` | `uf_area::p_uarea_02__n{0..2}` (`pure_cell_area`), `__n3` weekly (R0b: 429.7 s) | proven-within-bounds |
| P-UAREA-03 | correct result | `exact_area()` likewise | `ValidUnion(n ≤ 3)` | `uf_area::p_uarea_03__n{0..2}` (`pure_cell_area`), `__n3` weekly | proven-within-bounds |
| P-UAREA-04 | correct result | `Cell::average_area_for_level(l)` is upstream's `ldexp(4π/6, −2l)` bit for bit | the 31 levels | native enumeration (`verify/tests/`) | proven-by-enumeration |
| P-UEXP-01 | correct result | `expand_by_radius`'s level is upstream's | every radius, every `u8` | by composition: 008's P-CPBND-02 (the metric's level) and the drift guard | proven-by-composition |
| P-UEXP-02 | correct result | the result contains the original union | — | not proven: `expand_at_level`'s `all_neighbors` (001 R12) | not counted |
| P-UREG-01 | law | containment implies intersection | `ValidUnion(n ≤ 3) × ValidCell` | `uf_reg::p_ureg_01__n{0..3}` | proven-within-bounds |
| P-UREG-02 | law | the covering covers every contained cell | — | not proven: `Cap::cell_union_bound`'s level needs `asin`'s value (008's P-CPREG-04) | not counted |
| P-UREG-03 | law | the vertex law | — | does not hold, by design (a fixed test pins the probe's counterexample) | not a law |
| P-UREG-04 | law | the bounding cap contains every contained point | — | not proven: P-UBND-02 and `Cell`'s bounding-cap law, which is not proven | not counted |
| P-UREG-05 | law | the bounding rectangle contains every contained point | — | not proven: P-UBND-01 and `Cell`'s rectangle law | not counted |
| P-UREG-06 | law | the covering contains every contained point | — | not proven, as P-UREG-02 | not counted |

## 5. The operation inventory

### 5.1 `PointRegion` (`verify/values/INVENTORY.md` §13)

| # | Entries | Domain | Crash harness | Expected |
|---|---------|--------|---------------|----------|
| 1 | `new`, `point`, `cap_bound`, `contains_cell`, `contains_point`, `PartialEq` | `AnyTriple` (× `ValidCell`, `AnyTriple`) | `pr_crash::c_basic` | proven |
| 2 | `rect_bound` | `PlainTriple`; a NaN `x` or `y` panics in debug builds (documented, FR-019) | `pr_crash::c_rect_bound` (`atan2_contract`), `n_rect_bound_nan` | proven-under-contract |
| 3 | `intersects_cell` | `BoxPoint × ValidCell` (`Cell::contains_point`'s proven domain); a NaN on the cell's face axis panics in debug builds (documented) | `pr_crash::c_intersects_cell`, `n_intersects_cell_nan` | proven |
| 4 | `cell_union_bound` | `AnyTriple`; `from_point`'s documented debug panics (the zero vector, NaN, two infinite coordinates) are its own | `pr_crash::c_cell_union_bound` (`pure_leaf`, `asin_contract`), weekly, with `cl_crash::c_fg_from_point` by composition | proven-under-contract |
| 5 | `Deserialize` | every serialised `PointRegion` | `point_region` tests | tested |
| 6 | `Debug` | — | excluded: `core::fmt` | — |

That is 10 entries with Kani crash proofs (rows 1–4: `new`, `point`, the six `Region`
methods, `PartialEq`), 1 tested and 1 excluded.

### 5.2 `CellUnion` (`verify/cell_union/INVENTORY.md`, rows 12, 25–28, 35–37, 40)

| Row | Entry | Crash harness | Expected |
|-----|-------|---------------|----------|
| 12, 40 | `contains_point`, `Region::contains_point` | `uf_crash::c_contains_point__n{0..3}` (`pure_leaf`), with `c_fg_from_point` by composition | proven-within-bounds, under contract |
| 25 | `expand_by_radius` | `uf_crash::n_expand_by_radius_nan` (the debug panic, reached before `expand_at_level`); the rest inherits `all_neighbors`' limit | partly; tool limit (inherited) |
| 26 | `average_based_area` | `uf_crash::c_average_based_area__n{1..3}` (no value compared, so `powi` needs no stub) | proven-within-bounds |
| 27 | `approx_area` | `uf_crash::c_approx_area__n{1..3}` (`pure_cell_area`), with `cl_crash::c_fg_approx_area` by composition | proven-within-bounds, by composition |
| 28 | `exact_area` | inherits `Cell::exact_area`'s out-of-memory limit (coverage review) | tool limit (inherited) |
| 35 | `Region::cap_bound` | `uf_crash::c_cap_bound__n1` per change, `__n2` weekly (R0b: 462.7 s) (`pure_cell_cap`, `any_center`), with `cl_crash::c_fg_cap_bound`, `c_fg_center` by composition | proven-within-bounds, by composition |
| 36 | `Region::rect_bound` | `uf_crash::c_rect_bound__n1` with the real `Cell::rect_bound` (`atan2_contract`; R0b: 93.1 s), `__n{2,3}` with `any_cell_bound` and `cl_crash::c_fg_rect_bound` by composition | proven-within-bounds |
| 37 | `Region::cell_union_bound` | inherits `Cap::cell_union_bound`'s timeout (008) | tool limit (inherited) |

## 6. Library changes

| Change | Where | What |
|--------|-------|------|
| FR-017 | `core/src/s2/cell.rs` | `POLE_MIN_LAT = 0.615_479_708_670_387_3` (research R3) |
| FR-018 | `core/src/s2/cell_union.rs` | `approx_area`, `exact_area` fold from `0.0` |
| FR-019 | `core/src/s2/point_region.rs` | `new`'s precondition and the methods' behaviour outside it documented; `rect_bound`'s and `intersects_cell`'s debug panics documented; a hand-written `Deserialize` rejecting a point that is not unit length |
| FR-019 | `core/src/s2/cell_union.rs`, `core/src/s2/cell.rs` | `expand_by_radius`'s NaN panic documented; `Cell::contains_point`'s debug panic for a NaN on the face axis documented (R0) |
| Tests | `cell.rs` tests, `cell_union_tests.rs`, `point_region.rs` tests | research R3 |

## 7. Mutants (`verify/mutants/M-95N-<slug>.patch`)

| Id | Defect | Target |
|----|--------|--------|
| M-950 | `PointRegion::contains_point` uses `!=` | P-PRCEL-03 |
| M-951 | `contains_cell` returns true | P-PRCEL-01, P-PRREG-01 |
| M-952 | `intersects_cell` returns true | P-PRCEL-02 |
| M-953 | `rect_bound` from `LatLng::default()` | P-PRBND-02, P-PRREG-05 |
| M-954 | `cap_bound` is `Cap::from_point(-p)` | P-PRBND-01, P-PRREG-04 |
| M-955 | `point` returns a normalised copy | P-PRCON-01 |
| M-956 | `cell_union_bound` from `Cap::empty()` | P-PRBND-03, P-PRREG-06 |
| M-957 | a hand-written `PartialEq` comparing `x` and `y` only | P-PREQ-01 |
| M-958 | `CellUnion::contains_point` tests the leaf's parent | P-UPT-01 |
| M-959 | `rect_bound` skips the last cell | P-UBND-01 |
| M-960 | `cap_bound` adds `Cap::from_point` of each centre instead of its cap bound | P-UBND-02 |
| M-961 | `cap_bound` of the empty union is `Cap::full()` | P-UBND-03 |
| M-962 | `average_based_area` at level 29 | P-UAREA-01 |
| M-963 | `approx_area` skips the first cell | P-UAREA-02 |
| M-964 | FR-018 reverted: `approx_area` sums with `sum()` | P-UAREA-02 (`__n0`), `cell_union_tests` |
| M-965 | `exact_area` sums `approx_area` | `cpp_region_diff` |
| M-966 | `contains_cell` by `intersects_cell_id` | P-UREG-01 |
| M-967 | FR-017 reverted: Go's literal | `cell` tests |
| M-968 | FR-019 reverted: the derived `Deserialize` | `point_region` tests |
| M-969 | spec mutant: `pr_contains` by `!=` on `z` | P-PRCEL-03 |
| M-970 | stub mutant: `pure_leaf` without its table (any leaf each call) | P-PRREG-06 |
| M-971 | law mutant: `rect_bound_contains_points` asserts the negation for contained points | P-PRREG-05 |

The final list follows the harnesses as written; changes are recorded in `MUTANTS.md`.
