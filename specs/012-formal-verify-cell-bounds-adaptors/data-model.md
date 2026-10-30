# Data Model: `Cell`'s Bounding Laws, and the Region Adaptors

The entities of [spec.md](spec.md) as this feature defines them. Decisions and measurements are
in [research.md](research.md).

## 1. The description

- **A union contains what some member contains; an intersection what every member contains.**
  The empty union contains nothing; the empty intersection everything.
- **The members are abstract** (`verify/src/any_region.rs`, research R4): any answers, one per
  input and region, so a statement over them holds for every implementor.
- **`Cell`'s own points** are the normalised points `face_uv_to_xyz(face, u, v)` with `(u, v)` in
  the cell's u-v bound; `contains_point` also accepts a `(5/3)·ε` margin around it (U-017).

## 2. Input domains

| Generator | Domain |
|-----------|--------|
| `any_region::members::<N>()` | `N` abstract regions, `N ∈ {0, 1, 2, 3}` (2 for the vertex law) |
| `any_cell()` with `any_uv_bound` | `ValidCell` (the cell is read by id and, for the vertex law, through `fixed_vertex`) |
| `any_triple()`, `any_plain_triple()` | 006's |

## 3. Stubs

| Stub | Replaces | Returns | Rests on |
|------|----------|---------|----------|
| `AnyRegion` (a type, not a stub) | every member | any answers, one per input | nothing: it allows every implementor's answers |
| `pure_from_point`, `atan2_contract`, `asin_contract`, `any_uv_bound`, `fixed_vertex` | as in 008–011 | as recorded | as recorded |

## 4. The property catalogue

### 4.1 The adaptors (`verify/values/PROPERTIES.md` §13)

| ID | Kind | Statement | Domain | Harness | Expected |
|----|------|-----------|--------|---------|----------|
| P-ADCON-01 | correct result | `new()`/`Default` are empty; `from_regions` keeps its members in order (`len`, `num_regions`, `iter`, `region(i)`, both `IntoIterator`s); `add` appends one | `N ≤ 3` | `ad_mean::p_adcon_01__ru`, `__ri` | proven-within-bounds |
| P-ADBND-01 | correct result | the union's rectangle bound contains each member's; the intersection's is contained in each member's | `N ≤ 3` | `ad_mean::p_adbnd_01__{ru,ri}__n{1..3}` | proven-within-bounds |
| P-ADBND-02 | correct result | the cap bound is the rectangle bound's cap bound; the covering is the cap's | — | drift guards (`region_spec_tests::adaptors_are_upstreams`) | proven-by-composition |
| P-ADBND-03 | correct result | the empty union's bounds are empty; the empty intersection's full | `N = 0` | `ad_mean::p_adbnd_03` | proven |
| P-ADTST-01 | correct result | the union's cell and point tests are `any` of the members', the intersection's `all` | `N ≤ 3` | `ad_mean::p_adtst_01__{ru,ri}__n{0..3}` | proven-within-bounds, any `N` by the argument |
| P-ADREG-01 | law | containment implies intersection is preserved | `N ≤ 3` | `ad_reg::p_adreg_01__{ru,ri}__n{1..3}` | proven-within-bounds, any `N` by the argument |
| P-ADREG-02 | law | the vertex law is preserved | `N ≤ 2` | `ad_reg::p_adreg_02__{ru,ri}__n1` per change, `__n2` weekly | proven-within-bounds, any `N` by the argument |
| P-ADREG-03 | law | the bounding-rectangle law is preserved | `N ≤ 3` | `ad_reg::p_adreg_03__{ru,ri}__n1` per change, `__n2`, `__n3` weekly | proven-within-bounds, any `N` by the argument |
| P-ADREG-04 | law | the bounding-cap law holds when the members satisfy the rectangle law | — | — | not proven: by composition it needs `Rect`'s bounding-cap law (spec 010's P-LREG-03, not proven) |
| P-ADREG-05 | law | the covering laws | — | — | not proven: `Cap`'s covering law (spec 008's P-CPREG-04) |

**The argument for any `N`:** every adaptor answer is a fold over the members (`any`, `all`,
`Rect::union` from the empty rectangle, `Rect::intersection` from the full one), and the folds
are associative; spec 005 proves that `Rect::union` contains both operands (P-LSET-01) and that
`Rect::intersection` holds every common point (P-LSET-02). A law for `N + 1` members is the law for an adaptor of `N`
members, which the induction gives, combined with one more member, which the `N = 1, 2` proofs
cover.

### 4.2 `Cell`'s bounding laws (`verify/values/PROPERTIES.md` §2)

| ID | Kind | Statement | Status (R0) |
|----|------|-----------|-------------|
| P-CREG-01 | law | the cap bound contains every point of the cell's own u-v bound | not proven (tool limit: timeouts at 12 minutes; tested by 240,000 own points, C++ alike) |
| P-CREG-02 | law | the rectangle bound contains every point of the cell's own u-v bound | not proven (needs `atan2`'s monotonicity; tested as P-CREG-01) |
| P-CREG-03 | law | the covering contains every contained point, in its closed form | proven by composition: the covering is the cell's own id, whose cell is the cell |
| P-CREG-04 | law | the margin: a point of `contains_point`'s margin can lie outside both bounds (U-017) | an exception, pinned by `cell_tests::bounds_miss_the_margin` |

`CellUnion`'s P-UREG-04, -05 keep their reason (they wait on P-CREG-01, -02).

## 5. The inventory (`verify/values/INVENTORY.md` §14)

| # | Entries | Crash harness | Expected |
|---|---------|---------------|----------|
| 1 | `RegionUnion::new`, `from_regions`, `add`, `len`, `is_empty`, `iter`, both `IntoIterator`s, `Default` | `ad_crash::c_union_api__n{0..3}` | proven-within-bounds |
| 2 | `RegionIntersection::new`, `from_regions`, `num_regions`, `region` (in range), `Default` | `ad_crash::c_intersection_api__n{0..3}`; `n_region_past_end` | proven-within-bounds |
| 3 | `Region for RegionUnion` and `for RegionIntersection`: the tests | `ad_crash::c_tests__{ru,ri}__n{0..3}` | proven-within-bounds |
| 4 | the same: the bounds and the covering | `ad_crash::c_bounds__{ru,ri}__n0` per change, `__n1`, `__n2` weekly (`atan2`, `asin` contracts) | proven-within-bounds, under contract |
| 5 | `Debug` (derived) | excluded: `core::fmt` | — |

## 6. Library changes (documentation only)

| Change | Where |
|--------|-------|
| FR-016 (U-017) | `Cell::contains_point`, `cap_bound`, `rect_bound` rustdoc; `cell_tests::bounds_miss_the_margin`; `UPSTREAM.md` U-017; `CORE_TRAITS.md` |
| FR-017 (K2) | `RegionIntersection::region`'s `# Panics`; a `should_panic` test |

## 7. Mutants (`M-974` …)

One per proven entry: the union's `contains_cell` as `all`, `intersects_cell` returning
`contains_cell`, `contains_point` skipping the first member, `rect_bound` from `Rect::full()`;
the intersection's `contains_point` as `any`, `rect_bound` skipping a member, `region(i)` with
`get().unwrap_or` of the first (so no panic); `from_regions` dropping the last; `add` inserting at
the front (order); the abstract region without its tables (a stub mutant).
