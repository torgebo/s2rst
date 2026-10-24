# Data Model: Formal Verification of `r2::Rect` and `Point`

This feature adds no runtime data. The entities are the verification artifacts, the reference
descriptions and the changes to the library. The schemas are 004's
(`specs/004-formal-verify-cell-interval/data-model.md`), except where this document says
otherwise. The R0 column of §4 holds the planning probe's times and findings (research R0,
2026-10-14). Implementation amended four entries (research R0, findings 4–7): P-PTVAL-02's
normalising clause and P-PTORT-01 are tool limits (the checker's `sqrt`), P-PTORT-03's
`ortho` branch likewise, and P-QSET-01 excepts two empty rectangles; the catalogue in
`verify/values/PROPERTIES.md` holds the final statements, harnesses and statuses.

## 1. Reference descriptions

Both are plain Rust, compiled outside Kani, unit-tested in sibling `*_tests.rs` files, and
checked against the new C++ rows by `verify/tests/values_spec_vs_cpp.rs`. They read public
fields only and never call the operations they describe. They compare booleans only with `==`
(004 R13).

### 1.1 The u-v rectangle (`verify/src/uv_spec.rs`)

Written from upstream `r2rect.h`, on top of 005's `real_spec`.

| Item | Definition |
|------|------------|
| `valid(r)` | `real_spec::empty(r.x) == real_spec::empty(r.y)`, no bound NaN |
| `empty(r)` | `real_spec::empty(r.x)` |
| `member(r, p)` | `real_spec::member(r.x, p.x) ∧ real_spec::member(r.y, p.y)` |
| `interior(r, p)` | `real_spec::interior(r.x, p.x) ∧ real_spec::interior(r.y, p.y)` |
| `contains(a, b)` | `empty(b) ∨ (real_spec::contains(a.x, b.x) ∧ real_spec::contains(a.y, b.y))` |
| `interior_contains(a, b)` | `empty(b) ∨ (real_spec::interior_contains(a.x, b.x) ∧ real_spec::interior_contains(a.y, b.y))` |
| `intersects(a, b)`, `interior_intersects(a, b)` | the products of `real_spec`'s relations |
| `vertex(r, k)` | for `k mod 4` = 0, 1, 2, 3: `(x.lo, y.lo)`, `(x.hi, y.lo)`, `(x.hi, y.hi)`, `(x.lo, y.hi)`, upstream's counter-clockwise order |
| `finite(r)` | all four bounds finite: the domain of the arithmetic properties (FR-004, FR-018) |

### 1.2 The point (`verify/src/point_spec.rs`)

Written from upstream `util/math/vector.h`, `s2pointutil.{h,cc}` and `s2edge_crossings.cc`.

| Item | Definition |
|------|------------|
| `lex_less(a, b)` | C++'s `std::lexicographical_compare` over `(x, y, z)` with `<`: the first coordinate where `a_i < b_i` or `b_i < a_i` decides; a NaN pair decides nothing |
| `order(a, b)` | `Less` if `lex_less(a, b)`, `Greater` if `lex_less(b, a)`, `Equal` otherwise |
| `same(a, b)` | `a.x == b.x ∧ a.y == b.y ∧ a.z == b.z` (IEEE equality: signed zeros equal, NaN unequal) |
| `max_abs(p)` | the largest of `\|x\|`, `\|y\|`, `\|z\|` over a NaN-free triple |
| `normalizable(p)` | `max_abs(p) ≥ 2⁻²⁴²` |
| `normable(p)` | every coordinate finite, and `x·x + y·y + z·z` (upstream's `Norm2` order) finite: the domain where `normalize` never divides by an infinity (R0 finding 1) |
| `exponent(v)` | `ilogb(v)` for a non-zero finite `v`, from IEEE-754's encoding (subnormals included) |
| `scaled_by(p, k)` | `p · 2^k` computed in two exact steps of at most `2^537` each |
| `in_box(p)` | every coordinate in `[−1, 1]` |
| `zero(p)` | every coordinate `== 0` (either sign) |
| `ORIGIN` | upstream's `S2::Origin()` coordinates, as bit patterns |

**Restatements of upstream's arithmetic**, kept apart from the meaning (FR-007):

| Item | Upstream | Checked against |
|------|----------|-----------------|
| `upstream_is_unit(p)` | `fabs(p.Norm2() - 1) <= 5 * DBL_EPSILON`, products summed in upstream's order | `S2POINT_*` rows' `IsUnitLength` |
| `upstream_normalize(p)` | `n = Norm(); if (n != 0) n = 1 / n; return p * n` | `S2POINT_*` rows' `Normalize` |
| `upstream_scaled(p)` | `ldexp(2, −1 − ilogb(max_abs(p))) * p` where it is finite | `S2POINT_SPECIAL`'s robust cross product rows |

## 2. Input domain classes

As research R4:

- the u-v rectangle: `UvRect`, `FiniteUvRect`, `UvPoint`, and 005's `Number` for margins and
  error bounds;
- the point: `AnyTriple`, `PlainTriple` (no NaN), `FiniteTriple`, `NormableTriple`
  (`normable`), `BoxPoint`, and `NormalizableBoxPoint` (`in_box ∧ normalizable`).
  `NormableTriple` is proven in two parts (research R0, finding 5): `BoundedTriple` (every
  coordinate at most `2⁵¹¹`) per change and `NormableBand` (the rest) weekly;
- `FiniteAngle` for `rotate` and the approximate equalities, and 004's `AnyF64` for the
  bit-pattern checks.

## 3. Invariants (FR-005)

| Invariant | Proved by |
|-----------|-----------|
| u-v rectangle validity: the check | P-QVAL-01 |
| u-v rectangle validity: every operation that returns a rectangle | P-QVAL-02 |
| The point has no validity rule; unit length is `upstream_is_unit` | P-PTCLS-02 (the test), P-PTCLS-03 (the operations that promise it) |

## 4. Property catalogue (`verify/values/PROPERTIES.md`, sections 6 and 7)

Columns as in 004. The generic order laws join section 3.

### 4.1 `r2::Rect`

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-QVAL-01 | correct result | The validity check is upstream's emptiness rule (`real_spec::empty(x) == real_spec::empty(y)`) for every bit pattern, NaN included, so it agrees with `uv_spec::valid` wherever no bound is NaN; a NaN bound passes it (K4, documented by FR-018; FR-019's deserialiser rejects it) | `is_valid` | four `AnyF64` | 0.1 s (non-NaN bounds) |
| P-QVAL-02 | validity | Every in-scope operation that returns a rectangle returns a valid one | the constructors, `add_point`, `add_rect`, `union`, `intersection`, `expanded`, `expanded_by_margin`, `Default` | `UvRect`, `UvPoint`, finite margins; `from_center_size` with a finite non-negative size | expansion 0.4 s |
| P-QVAL-03 | correct result | The constructors give the documented rectangles: the two intervals, `[lo, hi]` per axis, the empty `[1, 0]²`, a single point, the shorter-first pairs, centre ± half the size; the default is empty; a rectangle is empty exactly when its x-interval is | `new`, `from_points`, `empty`, `from_point`, `from_point_pair`, `from_center_size`, `Default`, `is_empty` | `UvRect`, `UvPoint²` | centre and size 19.3 s |
| P-QVAL-04 | correct result | The corners and the vertices are the description's, for every `i32` index | `lo`, `hi`, `vertex`, `vertex_ij`, `vertices` | `UvRect × i32` | `vertex` 0.2 s |
| P-QMEM-01 | correct result | A point is in a rectangle exactly when each coordinate is in its interval | `contains_point` | `UvRect × UvPoint` | 0.3 s |
| P-QMEM-02 | correct result | Interior membership is the product of the interiors | `interior_contains_point` | `UvRect × UvPoint` || not probed (as P-QMEM-01) |
| P-QREL-01 | correct result | Containment and interior containment are the products of the intervals' | `contains`, `interior_contains` | `UvRect²` | `contains` 0.6 s |
| P-QREL-02 | correct result | Intersection and interior intersection are the products of the intervals'; intersection is symmetric | `intersects`, `interior_intersects` | `UvRect²` || not probed (as P-QREL-01) |
| P-QSET-01 | correct result | The union is the least rectangle containing both; adding a rectangle is the union | `union`, `add_rect` | `UvRect³` | 2.2 s |
| P-QSET-02 | correct result | The intersection holds exactly the common points, and is empty exactly when the two do not intersect | `intersection` | `UvRect² × UvPoint` | 6.0 s |
| P-QSET-03 | correct result | Adding a point gives the least rectangle containing the original and the point | `add_point` | `UvRect² × UvPoint` | 1.8 s |
| P-QMEA-01 | correct result | The centre of a finite non-empty rectangle lies in it; the size is the two intervals' lengths, non-negative | `center`, `size` | `FiniteUvRect` || not probed |
| P-QEXP-01 | correct result | Expanding by finite margins keeps a valid rectangle; by non-negative margins it contains the original, by non-positive ones it is contained in it; if either axis empties, the result is empty; expanding by one margin is expanding by it on both axes | `expanded`, `expanded_by_margin` | `FiniteUvRect × finite margins` | growth 28.2 s |
| P-QEXP-02 | correct result | Projection gives a point of a non-empty rectangle: the point itself if inside, otherwise the nearest bound per axis | `project` | non-empty `UvRect × UvPoint` | 1.2 s |
| P-QEXP-03 | law | Approximate equality is reflexive and symmetric for every finite non-negative error bound, and the default uses 1e-15 | `approx_eq_with`, `approx_eq` | `FiniteUvRect² × finite Number` | reflexive 3.1 s |
| P-QEQ-01 | law | The generic equality laws hold, and equality is upstream's: two rectangles are equal exactly when both intervals are, so every two empty rectangles are equal | `PartialEq` | `UvRect³` || not probed |

### 4.2 `Point`

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-PTVAL-01 | correct result | Building from a vector and the two conversions keep the bits; the accessors return the coordinates; the default is `(0, 0, 0)`; `origin` is upstream's point, and unit length | `new`, `From<Vector>`, `From<Point> for Vector`, `vector`, `x`, `y`, `z`, `Default`, `origin` | `AnyTriple` || not probed |
| P-PTVAL-02 | correct result | `from_coords` maps every zero triple to `origin` and every other normable triple to `normalize`'s result; `normalize` is upstream's formula bit for bit and maps zero to zero. The finite triples whose squared length overflows are a tool limit (the checker reports `1 / ∞` as NaN); ordinary tests check their zero vector (K7) | `from_coords`, `normalize` | `NormableTriple` | `FiniteTriple`: spurious NaN on division, 4.0 s (finding 1) |
| P-PTVAL-03 | law | The generic equality laws hold for points without NaN | `PartialEq` | `PlainTriple³` || not probed |
| P-PTCLS-01 | correct result | A point is normalizable exactly when its largest absolute coordinate is at least 2⁻²⁴² | `is_normalizable` | `PlainTriple` | 0.2 s |
| P-PTCLS-02 | exact output | `is_unit` is upstream's test, bit for bit | `is_unit` | `FiniteTriple` | 644.5 s: weekly |
| P-PTCLS-03 | correct result | `from_coords` and `normalize` give a unit point for coordinates within the documented range (K7); `ortho` and `reference_dir` give a unit point | `from_coords`, `normalize`, `ortho`, `reference_dir` | `NormableTriple` within bounds, `NormalizableBoxPoint` | not probed: weekly (the pilot) |
| P-PTSCL-01 | correct result | `ensure_normalizable` returns a normalizable vector for every non-zero finite input, the input itself when it already is, and the zero vector for zero | `ensure_normalizable` | `FiniteTriple` | the library fails (K5), 8.6 s; FR-020's draft holds |
| P-PTSCL-02 | exact output | Otherwise the result is the input times a power of two, exactly, with its largest coordinate in [1, 2), and it equals upstream's result wherever that is finite | `ensure_normalizable` | `FiniteTriple`, not normalizable | the draft, 113.8 s |
| P-PTORD-01 | correct result | `cmp_point` is upstream's lexicographic order for every triple, NaN included, and the ordering agrees with it | `cmp_point`, `PartialOrd` | `AnyTriple²` | 0.5 s |
| P-PTORD-02 | law | Without NaN the order is a total preorder whose ties are exactly equality (the generic order laws) | `cmp_point`, `PartialOrd`, `PartialEq` | `PlainTriple³` | 0.4–0.9 s |
| P-PTARI-01 | correct result | Negation, addition and subtraction are the coordinate-wise IEEE-754 operations, bit for bit; negation undoes itself, and subtracting is adding the negation | `Neg`, `Add`, `Sub` | `FiniteTriple²` | 150.5 s |
| P-PTORT-01 | law | `ortho(−a)` is `−ortho(a)` as values (`same`: the zeros' signs may differ), and `reference_dir` is `ortho` | `ortho`, `reference_dir` | `BoxPoint` | bit for bit fails on zeros' signs, 108.3 s (finding 2) |
| P-PTORT-02 | correct result | `ortho` and `point_cross` never return the zero vector for a normalizable point | `ortho`, `point_cross` | `NormalizableBoxPoint`, `BoxPoint` | `ortho` 358.0 s |
| P-PTORT-03 | law | `point_cross` has its three sign laws as values (`same`) wherever `(p + q) × (q − p)` is not the zero vector; where it is (equal, antipodal or parallel points, or an underflow, K11) the result is `ortho(p)` | `point_cross` | `BoxPoint²` | "unless equal or antipodal" fails, 142.3 s (finding 3) |
| P-PTDST-01 | correct result | The distance and the chord angle from a point to itself are zero, the chord angle is symmetric and its squared length is within [0, 4], and the approximate equalities are reflexive | `distance`, `chord_angle`, `approx_eq`, `approx_eq_angle`, `approx_eq_with` | `BoxPoint²`, `FiniteAngle` | 3.2 s, 6.2 s |
| P-PTDST-02 | law | The distance and the stable angle are symmetric bit for bit (default build) | `distance`, `stable_angle` | `BoxPoint²` | not proven (tool limit: `atan2` unsupported, R0); the C++ vectors and a property test check it |

**Generic order laws** (`verify/src/laws.rs`, new module `ord`, PROPERTIES.md §3): `reflexive`
(`Equal` with itself), `dual` (`cmp(b, a)` is `cmp(a, b)` reversed), `transitive`, and
`ties_are_eq`. Stated over any type with a comparison function, instantiated by P-PTORD-02, and
ready for `ChordAngle` (FR-025).

**Totals:** 16 u-v rectangle entries and 16 point entries, 32 in all. Specs 004's and 005's
entries are unchanged.

## 5. Operation inventories (`verify/values/INVENTORY.md`, sections 5 and 6)

- **`r2::Rect` (33 entries)**: the 30 inherent operations, `Default`, `PartialEq` in scope;
  `Display` excluded (tool limit). Domains as R4: the arithmetic operations (`center`,
  `size`, `expanded`, `expanded_by_margin`, `approx_eq_with`, `approx_eq`, `from_center_size`)
  take finite bounds, and `project` a non-empty rectangle.
- **`Point` (31 entries)**:
  - in scope (24): `new`, `from_coords`, `origin`, `vector`, `x`, `y`, `z`, `is_unit`,
    `normalize`, `chord_angle`, `point_cross`, `reference_dir`, `cmp_point`,
    `is_normalizable`, `ensure_normalizable`, `ortho`, `Neg`, `Add`, `Sub`, the two `From`
    conversions, `PartialOrd`, `PartialEq`, `Default`;
  - through trigonometry (6): `rotate` gets a crash proof (`sin` and `cos` are modelled);
    `distance`, `stable_angle`, `approx_eq`, `approx_eq_angle` and `approx_eq_with` reach
    `atan2`, which Kani does not support, so they are tool-limit exclusions (research R0),
    with their values checked against the C++ vectors;
  - excluded: `Display` (tool limit).

  Domains as R4: the order, comparisons, conversions and classifications take every triple;
  the arithmetic takes every triple where it cannot produce a new NaN (FR-004's maximal
  domain; at least the finite ones), `from_coords` and `normalize` normable ones (the rest of
  the finite triples is a tool limit, R0 finding 1); the geometric operations take
  `BoxPoint`.

## 6. Changes to the library

| Change | Files | Kind | Test (fails without the change) |
|--------|-------|------|--------------------------------|
| FR-017 (K1, K2): `# Panics` on `new`, `from_points` (debug builds) and `project` (every build); `from_center_size`'s precondition, with a debug check that the result is valid | `core/src/r2/rect.rs` | documentation; debug builds only | a negative size panics in debug builds (`#[cfg(debug_assertions)]` test) |
| FR-018 (K4): the infinite and NaN corners of `center`, `size`, `expanded`, `expanded_by_margin`, `approx_eq_with` | `core/src/r2/rect.rs` | documentation | — |
| FR-019 (K3): a hand-written `Deserialize` that rejects disagreeing emptiness and NaN bounds (R6) | `core/src/r2/rect.rs`, `core/src/r2/rect_tests.rs` | behaviour | invalid rectangles are rejected with the rule named; valid ones round-trip bit for bit; the format is unchanged |
| FR-020 (K5): `ensure_normalizable` scales into [1, 2) for every non-zero finite input (R8) | `core/src/s2/point.rs`, `core/src/s2/point_tests.rs` | behaviour (a defect fix) | `(x, 0, −x/2)` for x = 2⁻¹⁰²⁴ and 2⁻¹⁰²⁵ gives `(1, 0, −0.5)`, and `(2⁻¹⁰⁷⁴, 0, 0)` gives `(1, 0, 0)` |
| FR-021 (K6, U-012): `edge_crossings` uses `Point`'s scaling; deviation section on `robust_cross_prod` | `core/src/s2/edge_crossings.rs`, `core/src/s2/edge_crossings_tests.rs` | behaviour (parity, and a documented deviation) | `robust_cross_prod(−b, b)` is `(−0, 1, 0)` for `b = (2⁻³⁰⁰, 0, 1)` and for `b = (2⁻¹⁰⁷⁴, 0, 1)` |
| FR-022 (K7, K8, K10, K11): `from_coords`, `normalize`, the order with NaN, `is_normalizable`, the geometric operations' preconditions, `point_cross` against Go, and its sign laws' true exception | `core/src/s2/point.rs` | documentation | — |
| FR-023: four C++ vector sections and their differential tests (R9) | `tools/refgen-cpp/*`, `core/tests/data/cpp_values_vectors.csv`, `core/tests/cpp_values_diff.rs`, `verify/tests/values_spec_vs_cpp.rs` | tests | — |
| FR-024: U-012 (K6), U-013 (K11) | `UPSTREAM.md` | documentation | — |

## 7. Mutants (`verify/mutants/M-5NN-<slug>.patch`)

At least one per catalogue entry (SC-004), one exact-string edit each:

- **The u-v rectangle:** `is_valid` without the emptiness rule; `is_empty` reading y;
  `from_points` swapping an axis; `from_point_pair` not ordering; `from_center_size` with the
  full size; `vertex` swapping two corners; `vertex_ij` swapping axes; `contains_point`
  checking x only; `interior_contains_point` with `contains`; `contains` checking x only;
  `interior_contains` with `contains`; `intersects` checking y only; `union` keeping one
  x-interval; `intersection` without the emptiness check; `add_point` not extending y;
  `center` swapping axes; `size` reversed; `expanded` not emptying both axes; `project`
  without clamping y; `approx_eq_with` comparing x only; equality comparing x only.
- **The point:** `from_coords` without the zero case; `is_unit` with `<`; `is_normalizable`
  with `>`; the FR-020 fix removed (expected killed by P-PTSCL-01); the scale's two factors
  unbalanced so one overflows; `cmp_point` skipping y; `PartialOrd` reversed; `Neg` negating
  two coordinates; `Sub` as addition; `point_cross` without the zero branch; `chord_angle`
  measuring the sum (as planned, "without the clamp at 4" was equivalent: `from_length2`
  clamps too); `distance` without the equality short-cut (killed by P-PTDST-01, which then
  reaches `atan2`); `origin` rounded; equality not symmetric; `normalize` dividing by the
  squared length; `point_cross` dropping a sign (P-PTORT-03). The planned `ortho` at the largest
  axis and `reference_dir` not `ortho` were dropped: their target, P-PTORT-01, is a tool limit
  (research R0, finding 4).
- **The descriptions** (three): `uv_spec::valid` without the emptiness rule,
  `point_spec::lex_less` deciding on a NaN pair, `point_spec::normalizable` with `>`.

## 8. Records and states

As in 004. `verify/values/EVALUATION.md` gains a spec-006 part with SC-014's composition
numbers, the reach into the point's geometry, and the `ChordAngle` follow-up (FR-025).
