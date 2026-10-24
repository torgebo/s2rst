# Data Model: Formal Verification of `r1::Interval` and `s2::Rect`

This feature adds no runtime data. The entities are the verification artifacts, the reference
descriptions and the changes to the library. The schemas are 004's
(`specs/004-formal-verify-cell-interval/data-model.md`), except where this document says
otherwise.

## 1. Reference descriptions

Both are plain Rust, compiled outside Kani, unit-tested in sibling `*_tests.rs` files, and
checked against the new C++ rows by `verify/tests/values_spec_vs_cpp.rs`. They never call the
operations they describe. They compare booleans only with `==` (004 R13).

### 1.1 The real interval (`verify/src/real_spec.rs`)

Written from upstream `r1interval.h`.

| Item | Definition |
|------|------------|
| `empty(x)` | `x.lo > x.hi` |
| `member(x, p)` | `x.lo ≤ p ≤ x.hi` |
| `interior(x, p)` | `x.lo < p < x.hi` |
| `contains(x, y)` | `empty(y) ∨ (x.lo ≤ y.lo ∧ y.hi ≤ x.hi)`: closed real sets, exact |
| `interior_contains(x, y)` | `empty(y) ∨ (x.lo < y.lo ∧ y.hi < x.hi)` |
| `intersects(x, y)` | `¬empty(x) ∧ ¬empty(y) ∧ x.lo ≤ y.hi ∧ y.lo ≤ x.hi` |
| `interior_intersects(x, y)` | `¬empty(x) ∧ ¬empty(y) ∧ x.lo < y.hi ∧ y.lo < x.hi ∧ x.lo < x.hi` (upstream's rule: the interior of `x` meets `y`) |
| `finite(x)` | both bounds finite: the domain of the arithmetic properties (FR-004, FR-017) |

### 1.2 The rectangle (`verify/src/rect_spec.rs`)

Written from upstream `s2latlng_rect.h`, on top of `real_spec` and 004's `arc_spec`.

| Item | Definition |
|------|------------|
| `valid(r)` | `\|lat.lo\| ≤ π/2 ∧ \|lat.hi\| ≤ π/2 ∧ arc_spec::valid(lng) ∧ (real_spec::empty(lat) == arc_spec::empty(lng))` |
| `ll_valid(lat, lng)` | `\|lat\| ≤ π/2 ∧ \|lng\| ≤ π` |
| `member(r, lat, lng)` | `ll_valid ∧ real_spec::member(r.lat, lat) ∧ arc_spec::member(r.lng, lng)` |
| `contains(a, b)` | `empty(b) ∨ (real_spec::contains(a.lat, b.lat) ∧ arc_spec::contains(a.lng, b.lng))` |
| `intersects`, `interior_contains`, `interior_intersects` | the products of the two descriptions' relations, as upstream's headers define them |
| `touches_pole(r)` | `r.lat.lo == −π/2 ∨ r.lat.hi == π/2` |

## 2. Input domain classes

As research R4: `Real`, `FiniteReal`, `Number`, `ValidRect`, `AnyLatLng` and `ValidLatLng`,
plus 004's `ValidInterval`, `Point` and `AnyF64`.

## 3. Invariants (FR-005)

| Invariant | Proved by |
|-----------|-----------|
| The real interval: no invalid non-NaN value; emptiness is `lo > hi` | P-RIV-01 |
| Rectangle validity | P-LVAL-01 (the check), P-LVAL-02 (every operation that returns a rectangle) |

## 4. Property catalogue (`verify/values/PROPERTIES.md`, sections 4 and 5)

Columns as in 004. The R0 column gives the probe result where one ran.

### 4.1 `r1::Interval`

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-RIV-01 | correct result | An interval is empty exactly when its low bound exceeds its high one. The constructors give the documented intervals: `[lo, hi]`, a single point, the shorter-first pair, the pair conversion, and the empty default `[1, 0]` | `new`, `empty`, `from_point`, `from_point_pair`, `From<(f64, f64)>`, `Default`, `is_empty` | `Real`, `Number²` | — |
| P-RIV-02 | law | The generic equality laws hold, and every empty interval equals every other | `PartialEq` | `Real³` | — |
| P-RIV-03 | correct result | `bound(0)` and `bound(1)` are the low and high bounds | `bound` | `Real` | — |
| P-RMEM-01 | correct result | A number is in an interval exactly when it lies between the bounds | `contains` | `Real × Number` | — |
| P-RMEM-02 | correct result | A number is in the interior exactly when it lies strictly between the bounds | `interior_contains` | `Real × Number` | — |
| P-RREL-01 | correct result | Containment and interior containment are those of real sets | `contains_interval`, `interior_contains_interval` | `Real²` | 0.2 s |
| P-RREL-02 | correct result | Intersection and interior intersection are those of real sets; intersection is symmetric | `intersects`, `interior_intersects` | `Real²` | — |
| P-RSET-01 | correct result | The union is the least interval containing both | `union` | `Real³` | 2.1 s |
| P-RSET-02 | correct result | The intersection is the set intersection: it holds exactly the common numbers | `intersection` | `Real² × Number` | — |
| P-RSET-03 | correct result | Adding a number or an interval gives the least interval containing both; adding an interval is the union | `add_point`, `add_interval` | `Real² × Number` | — |
| P-RMEA-01 | correct result | The centre of a finite non-empty interval lies in it; its length is `hi − lo`, and non-negative | `center`, `length` | `FiniteReal` | 21 s |
| P-RMEA-02 | correct result | The directed Hausdorff distance is non-negative, 0 when the second interval contains the first, and infinite when only the second is empty | `directed_hausdorff_distance` | `FiniteReal²` | — |
| P-REXP-01 | correct result | Expanding by a finite non-negative margin contains the original; by a negative one is contained in it; the empty interval stays empty | `expanded` | `FiniteReal × finite Number` | 3.4 s (growth) |
| P-REXP-02 | correct result | Projection gives a number of a non-empty interval: the number itself if inside, otherwise the nearer bound | `project` | non-empty `Real × Number` | 0.7 s |
| P-REXP-03 | law | Approximate equality is reflexive and symmetric for every finite non-negative error bound, and the default uses 1e-15 | `approx_eq_with`, `approx_eq` | `FiniteReal² × finite Number` | — |

### 4.2 `s2::Rect`

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-LVAL-01 | correct result | The validity check agrees with the description for every bit pattern | `is_valid` | four `AnyF64` | 0.1 s |
| P-LVAL-02 | validity | Every in-scope operation that returns a rectangle returns a valid one: the constructors, union, intersection, adding a point, the polar closure, expansion | the 13 such operations | `ValidRect`, `ValidLatLng`/`AnyLatLng`, margins | 0.5–62 s |
| P-LVAL-03 | correct result | Empty, full, a single point, inverted longitudes, and the full latitude and longitude ranges agree with the description | `is_empty`, `is_full`, `is_point`, `is_inverted`, `full_lat`, `full_lng`, `empty`, `full`, `Default` | `ValidRect` | — |
| P-LMEM-01 | correct result | A latitude–longitude is in a rectangle exactly when it is valid and in both intervals | `contains_lat_lng` | `ValidRect × AnyLatLng` | 0.7 s |
| P-LMEM-02 | correct result | Interior membership is the product of the interiors | `interior_contains_lat_lng` | `ValidRect × ValidLatLng` | — |
| P-LREL-01 | correct result | Containment and interior containment are the products of the intervals' | `contains`, `interior_contains` | `ValidRect²` | 2.5 s |
| P-LREL-02 | correct result | Intersection and interior intersection are the products of the intervals'; intersection is symmetric | `intersects`, `interior_intersects` | `ValidRect²` | — |
| P-LSET-01 | correct result | The union contains both rectangles | `union` | `ValidRect²` | 48 s |
| P-LSET-02 | correct result | The intersection holds every common point, and is empty exactly when the two do not intersect | `intersection` | `ValidRect² × AnyLatLng` | 28 s |
| P-LSET-03 | correct result | Adding a valid point gives a rectangle containing it and the original; an out-of-range point changes nothing (K2) | `add_point`, `from_lat_lng`, `from_point_pair` | `ValidRect × AnyLatLng` | 16 s |
| P-LSET-04 | correct result | The polar closure contains the original, makes the longitudes full when a pole is touched, and changes nothing otherwise | `polar_closure` | `ValidRect` | 0.5 s |
| P-LBND-01 | correct result | The low and high corners, the four vertices, the centre and the size are the corresponding combinations of the two intervals' bounds, centres and lengths | `lo`, `hi`, `vertex`, `center`, `size` | `ValidRect × RectVertex` | — |
| P-LEXP-01 | correct result | Expanding keeps a valid rectangle for every non-NaN margin; by finite non-negative margins it contains the original, and by a finite non-positive latitude margin with a finite negative longitude margin it is contained in it (a zero longitude margin grows, as in P-IEXP-01). Construction from a centre and size is valid and contains the centre | `expanded`, `from_center_size` | `ValidRect × margins` | validity 62 s, centre-size 33 s, containment ⏱ (weekly) |
| P-LEXP-02 | law | Approximate equality is reflexive and symmetric | `approx_eq` | `ValidRect²` | 256 s (reflexive) |
| P-LEQ-01 | law | The generic equality laws hold for the rectangle, and equality is upstream's: every two empty rectangles are equal, otherwise the four bounds are (added during implementation: the laws alone let a mutant that compares only the latitudes survive) | `PartialEq` | `ValidRect³` | — |
| P-LEQ-02 | correct result | The rectangle's bound, inherent and through the region interface, is the rectangle itself | `rect_bound`, `Region::rect_bound` | `ValidRect` | — |

004's `laws::region` laws take a cell and go through `contains_cell` and `intersects_cell`,
which are excluded for the rectangle (floating-point geometry). They are not instantiated
here; the quickcheck twins `region_rect_*` in `core/tests/trait_property_tests.rs` cover them,
and `PROPERTIES.md` §5 says so.

**Totals:** 15 real-interval entries and 16 rectangle entries, 31 in all. Spec 004's 40 entries
are unchanged.

## 5. Operation inventories (`verify/values/INVENTORY.md`, sections 3 and 4)

- **`r1::Interval` (27 entries)**:
  - in scope: the 23 inherent operations, `Default`, `PartialEq` and `From<(f64, f64)>`;
  - excluded: `Display` (tool limit);
  - domains as R4: the arithmetic operations take `FiniteReal`, and `project` needs a
    non-empty interval.
- **`s2::Rect` (53 entries)**:
  - in scope: 34, as in spec FR-003;
  - excluded: 18 for floating-point geometry (the area, point membership in 3-D, the cap and
    covering bounds, the centroid, the boundary test, the expansions by distance and for
    sub-regions, the four distances, and the five region methods through points or caps),
    and `Display`.

  The float-geometry rows point to the existing property and generic trait tests
  (`region_rect_*` in `core/tests/trait_property_tests.rs`), and to D-006's
  `rect_tests.rs`.

## 6. Changes to the library

| Change | Files | Kind | Test (fails without the change) |
|--------|-------|------|--------------------------------|
| FR-017 (K1): the infinite corners of `length`, `center`, `expanded`, `approx_eq_with`, `approx_eq` and `directed_hausdorff_distance` | `core/src/r1/interval.rs` | documentation | — |
| FR-018 (K2): `add_point` and `contains_lat_lng` ignore or refuse an out-of-range point, a deviation from C++ that follows Go | `core/src/s2/rect.rs` | documentation | — |
| FR-019 (K3, U-008): `# Panics` on `Rect::from_lat_lng`, `from_point_pair` and `interior_contains_lat_lng` (debug builds), and on `r1::Interval::project` (every build) | both files | documentation | — |
| FR-020 (K4): a hand-written `Deserialize` for `s2::Rect` that rejects an invalid rectangle (research R6) | `core/src/s2/rect.rs`, `core/src/s2/rect_tests.rs` | behaviour | the invalid rectangles are rejected; valid ones, the boundary values included, round-trip valid |
| Records: U-008 (a candidate), and the K2 deviation | `UPSTREAM.md` | documentation | — |

## 7. Mutants (`verify/mutants/M-4NN-<slug>.patch`)

**As implemented: 43 patches, `M-401` … `M-443`**, at least one per catalogue entry (SC-004),
generated by one exact-string edit each. P-REXP-03's laws cannot catch "compare one bound", so
its mutants break symmetry and the default error bound instead. The planned list:

- **The real interval:**
  - `is_empty` with `>=`;
  - `contains` with `<` at the high end;
  - `interior_contains` with `<=`;
  - `contains_interval` reversed;
  - `intersects` missing a case;
  - `union` taking the max of the lows;
  - `intersection` taking the min of the highs wrong;
  - `add_point` not extending;
  - `add_interval` ignoring an empty argument;
  - `center` not halved;
  - `length` reversed;
  - the Hausdorff distance without the clamp at 0;
  - `expanded` moving both bounds the same way;
  - `project` not clamping;
  - `approx_eq_with` comparing one bound;
  - equality not treating all empties as equal;
  - `bound` swapped;
  - the default not empty.
- **The rectangle:**
  - `is_valid` without the emptiness rule;
  - `contains_lat_lng` without the validity screen;
  - `add_point` adding an invalid point;
  - `contains` checking only latitudes;
  - `intersects` checking only longitudes;
  - `union` keeping one longitude range;
  - `intersection` dropping the latitude check;
  - `polar_closure` testing only the north pole;
  - `vertex` swapping two corners;
  - `size` swapping axes;
  - `expanded` not clipping latitudes to [−π/2, π/2];
  - `rect_bound` returning an empty rectangle;
  - equality comparing latitudes only.
- **The descriptions** (two): `real_spec::contains` with a strict bound, and
  `rect_spec::valid` without the emptiness rule.

## 8. Records and states

As in 004. `verify/values/EVALUATION.md` gains a spec-005 part with SC-010's composition numbers.
