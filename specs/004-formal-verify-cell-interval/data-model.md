# Data Model: Formal Verification of `Cell` and `s1::Interval`

This feature adds no runtime data. The entities below are:

- the **verification artifacts** (spec Key Entities);
- the **reference descriptions** of a cell and of a circular interval;
- the **changes to the library** that the survey findings require.

The schemas extend 001's to 003's (`specs/00{1,2,3}-*/data-model.md`). Where a field is
unchanged, this document says so.

## 1. Reference descriptions

Both descriptions are plain Rust, compiled outside Kani, with hand-made unit tests in sibling
`*_tests.rs` files. They are sanity-tested against the new C++ vectors by
`verify/tests/values_spec_vs_cpp.rs` (research R10). Neither calls the code it describes
(FR-007).

### 1.1 A cell (`verify/src/cell_spec.rs`, on top of 001's `spec.rs`)

A cell is its identifier's decomposition, with facts derived from it. Upstream sources:
`s2cell.h`, the `S2Cell(S2CellId)` constructor (`s2cell.cc:64–71`), `Subdivide`
(`s2cell.cc:91–124`), and `S2::STtoUV` and `S2::IJtoSTMin` (`s2coords.h`).

| Item | Definition |
|------|------------|
| `CellModel` | `{ face, level, orientation, i_lo, j_lo, size }` for a well-formed `spec::Decomposed d`: `(face, i, j, orientation) = spec::face_ij_orientation(d)`, `size = spec::size_ij(level)`, `i_lo = i & !(size − 1)`, `j_lo` likewise |
| `child_model(d, k)` | `spec::child(d, k)`, with orientation `o ^ spec::POS_TO_ORIENTATION[k]` |
| `st_to_uv(s)` | upstream's quadratic projection in upstream's operation order: `s ≥ 0.5 ? (1/3)·(4·s·s − 1) : (1/3)·(1 − 4·(1 − s)·(1 − s))` |
| `ij_to_st_min(k)` | `(1 / 2^30) · k`, exact for `0 ≤ k ≤ 2^30` |
| `uv_bounds(m)` | `u = [st_to_uv(ij_to_st_min(i_lo)), st_to_uv(ij_to_st_min(i_lo + size))]`, and `v` likewise with `j_lo` |
| `consistent(c)` | `c.face, c.level, c.orientation, c.id` are those of `CellModel` for `c.id`'s decomposition, and `c.bound_uv()` is `uv_bounds` bit for bit (FR-005) |
| `contains(a, b)`, `intersects(a, b)` | `spec::contains(a, b)`; `contains(a, b) ∨ contains(b, a)` |

`st_to_uv` restates upstream's formula, not the library's code. The bounds are defined by
that floating-point expression, so bit equality with it is parity with upstream (Principle I).
The library's `coords::st_to_uv` uses the same expression today. The model is not derived
from it.

### 1.2 A circular interval (`verify/src/arc_spec.rs`)

An interval denotes a set of points on the circle. Upstream source: the `S1Interval` header
(`s1interval.h`), which defines the representation, the special intervals and each
operation's meaning. Points are `f64` values in [−π, π], and π and −π are one point.

| Item | Definition |
|------|------------|
| `in_range(p)` | `−π ≤ p ≤ π` (false for NaN) |
| `canon(p)` | `π` if `p = −π`, else `p` |
| `valid(lo, hi)` | `in_range(lo) ∧ in_range(hi)`, and `−π` occurs only as `[−π, π]` (full) or `[π, −π]` (empty) |
| `empty`, `full`, `inverted` | `lo = π ∧ hi = −π`; `lo = −π ∧ hi = π`; `lo > hi` |
| `member(x, p)` | for `q = canon(p)`: if not inverted, `lo ≤ q ≤ hi`; if inverted, `(q ≥ lo ∨ q ≤ hi) ∧ ¬empty` |
| `interior(x, p)` | full: true. Otherwise, for `q = canon(p)`: if not inverted, `lo < q < hi`; if inverted, `q > lo ∨ q < hi` |
| `ccw_le(a, q, r)`, `ccw_lt(a, q, r)` | whether `q` comes no later (strictly before) than `r`, walking counter-clockwise from `a`: the order of the exact real distances, decided by float comparisons and boolean equality only (never by ordering booleans: Kani 0.68.0 mis-models `<` and `<=` on symbolic `bool`s, found in T037) |
| `contains(x, y)`, `interior_contains(x, y)` | the relations of **real** arcs: `y` empty or `x` full; otherwise, from `x.lo`, `y` starts no later than it ends and ends no later than (strictly before, for the interior) `x` does: `ccw_le(x.lo, y.lo, y.hi) ∧ ccw_le(x.lo, y.hi, x.hi)` |
| `intersects(x, y)`, `interior_intersects(x, y)` | two closed arcs meet when one contains a bound of the other; `y` meets `x`'s interior when it starts inside it, or contains `x.lo` and runs on past it |
| `upstream_length(x)` | upstream's `GetLength` (`s1interval.cc:41–47`), restated in its operation order: `d = hi − lo; if d ≥ 0 {d} else {e = d + 2π; if e > 0 {e} else {−1}}` |
| `upstream_expanded(x, m)` | upstream's `Expanded` (`s1interval.cc:208–224`), restated in its operation order from the fields (FR-023, added 2026-10-13): the early returns on the length tests, then `remainder(lo − m, 2π)`, `remainder(hi + m, 2π)`, the constructor's −π normalisation and `lo ≤ −π ⇒ π`. The length is the library's documented one (FR-020), and `remainder` the library's `x − round(x / y)·y`, which agrees with C's on the arguments that reach it |
| `CORNER` | `[π, next_up(−π)]`: `lo = π`, `hi = f64::from_bits((−π).to_bits() − 1)` |
| `ULP_PI` | `2^−51 = 4.440892098500626e−16`, one ulp of π, the corner's exact length |
| `ulps_apart(a, b)` | the distance between two finite floats in units in the last place |

**Why the relations are of real arcs (found in T010).** A float-free gap is still a gap. `[next_up(−π), π]` holds every float, π included, but C++ says it does not contain the full circle, because the reals strictly between −π and `next_up(−π)` lie outside it. So the relations between intervals cannot be read off float points, and the description decides them by the cyclic order of the bounds. That needs only exact float comparisons. Membership stays a property of float points. The first draft characterised containment by finite float witnesses; the C++ vectors refuted it on exactly this pair.

**Library deviations** are named predicates, kept apart from upstream's meaning, as in 003:

| Deviation | Applies to | Predicate |
|-----------|------------|-----------|
| `DEV_SEAM_LENGTH` (FR-020) | `length`, and the results computed from it | `x = CORNER` ⇒ the length is `ULP_PI`, not `upstream_length(x) = −1` |
| `DEV_EXPANDED_CROSSING` (FR-023) | `expanded` | with `u = upstream_expanded(x, m)`: `m < 0 ∧ ¬contains(x, u)` ⇒ the result is empty; `m ≥ 0 ∧ ¬contains(u, x)` ⇒ full; otherwise it is `u` |

**Validation rules**: `cell_spec_tests.rs` and `arc_spec_tests.rs` hold hand-made unit tests
(Constitution III). They cover upstream's s2cell_test and s1interval_test fixtures (the
quadrant intervals and the `IntervalOps` table). `tests/values_spec_vs_cpp.rs` checks both
descriptions against every row of `cpp_values_vectors.csv`.

## 2. Input domain classes

The cell domains are 001's. This feature adds:

| Class | Meaning | Generator |
|-------|---------|-----------|
| `ValidCell` | `Cell::from_cell_id(id)` for every valid `id` (001's `Valid`) | `any_cell()` |
| `ValidCell(L)` | the same at level `L`; a family `__l0 … __l30` covers `ValidCell` | `any_cell_at(L)` |
| `ValidInterval` | every pair of `f64` bounds with `arc_spec::valid`: empty, full, inverted and single-point intervals included | `any_interval()` |
| `Point` | every `f64` with `in_range` | `any_point()` |
| `AnyF64` | every `f64` bit pattern, NaN and the infinities included | `any_f64()` |
| `Edge` | the four `CellEdge` values | `any_edge()` |
| `BoundIndex` | `{0, 1}`, `bound`'s documented domain | inline |

These are the FR-004 domains. A margin and an error bound range over `AnyF64` for crash
freedom. A property may narrow them, for example to finite non-negative margins, and it says
so in its statement.

## 3. Invariants (FR-005)

| Invariant | Statement | Proved by |
|-----------|-----------|-----------|
| Cell consistency | `cell_spec::consistent(c)` for every cell a constructor or `children()` returns | P-CID-01 (integer fields), P-CBND-01 (bounds), P-CCHD-01 and P-CCHD-02 (children) |
| Interval validity | `arc_spec::valid(lo, hi)` for every interval an in-scope operation returns, whenever its inputs are in the domain | P-IVAL-02 (constructors), and the validity clause of each P-ISET and P-IEXP entry |

## 4. Property catalogue (`verify/values/PROPERTIES.md`)

The columns are 001's: id, kind, plain statement, exact statement, upstream source, domain,
harness, status, mutants. The status is `proven`, `proven within stated bounds`, or
`not proven (reason)`. The R0 column gives the planning probe's result where it ran
(research R0).

### 4.1 `Cell`

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-CID-01 | correct result | A cell built from an identifier reports that identifier, and the face, level and orientation the model gives for it | `from_cell_id`, `id`, `face`, `level`, `orientation` | `ValidCell` | pilot: 10 s |
| P-CID-02 | correct result | A cell is a leaf exactly when its level is 30, and its edge in (i, j) space is 2^(30 − level) long | `is_leaf`, `size_ij` | `ValidCell` | pilot: 10 s |
| P-CID-03 | correct result | The face constructor, the face-position-level constructor, conversion from an identifier and the default value give the cell of the identifier the model gives (face 0 for the default) | `from_face`, `from_face_pos_level`, `From<CellId>`, `Default` | faces 0–5; valid positions and levels | — |
| P-CCHD-01 | correct result | A cell has children exactly when it is not a leaf. Child `k` is the cell of the model's child `k`, on the same face, one level down, with the orientation the model gives | `children` | `ValidCell` | pilot: 125 s |
| P-CCHD-02 | exact output | Each child's u-v bounds, which `children()` computes from the parent's centre, are bit for bit those of the child built from its own identifier (upstream's `TestSubdivide`) | `children`, `bound_uv` | `ValidCell(L)`, `L ≤` measured (R8) | L=3 109 s, L=5 638 s, L≥7 ⏱ |
| P-CCNT-01 | correct result | One cell contains another exactly when the model's identifier containment holds, and two cells intersect exactly when one contains the other | `contains_cell`, `intersects_cell` | `ValidCell` × `ValidCell` | — |
| P-CCNT-02 | correct result | The region interface agrees with the inherent operations, and a cell's covering is the union of its own identifier | `Region::{contains_cell, intersects_cell, cell_union_bound}`, `cell_union_bound` | `ValidCell` × `ValidCell` | — |
| P-CCNT-03 | law | **The generic `Region` laws (§4.3), for `Cell`.** Containing a cell implies intersecting it, and every cell a region contains is covered by its covering bound | `Region::{contains_cell, intersects_cell, cell_union_bound}` | `ValidCell` × `ValidCell` | — |
| P-CBND-01 | exact output | The stored u-v bounds are bit for bit upstream's projection of the identifier's (i, j) range | `bound_uv` | `ValidCell(L)`, `L ≤` measured (R8) | L=3 9.5 s, L=10 575 s, L=20 ⏱ |
| P-CBND-02 | validity | The bounds are ordered and lie in [−1, 1] | `bound_uv` | `ValidCell(L)`, `L ≤` measured (R8) | L=3 9 s, L=10 98 s, L≥15 ⏱ |
| P-CBND-03 | correct result | Edge k's u-v coordinate is the matching side of the bounds: bottom is v low, right is u high, top is v high, left is u low | `uv_coord_of_edge` | `ValidCell` × `Edge` | 26 s |
| P-CEQ-01 | law | **The generic equality laws (§4.3), for `Cell`:** equality is an equivalence. Two cells are equal exactly when their identifiers are | `PartialEq` | `ValidCell` × `ValidCell` × `ValidCell` | — |
| P-CEQ-02 | law | Two cells built from the same identifier have identical fields, bounds included, so equal cells cannot differ in any field | `PartialEq`, `from_cell_id` | `ValidCell` | — |

### 4.2 The circular interval

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-IVAL-01 | correct result | The validity check agrees with the model's for every pair of floats, NaN and the infinities included | `is_valid` | `AnyF64²` | — |
| P-IVAL-02 | validity | Every constructor returns a valid interval for its documented inputs, and `new` turns −π into π except in the full and empty intervals. The default interval is the empty one, as documented | `new`, `empty`, `full`, `from_point`, `from_point_pair`, `Default` | `Point²` | pilot |
| P-IVAL-03 | correct result | Empty, full and inverted agree with the model. The empty and full intervals are the only ones of their kind, the empty one is inverted, and none is both | `is_empty`, `is_full`, `is_inverted` | `ValidInterval` | pilot |
| P-IVAL-04 | correct result | `bound(0)` is the low bound and `bound(1)` the high one; equality compares both bounds with float `==` | `bound`, `PartialEq` | `ValidInterval` × `BoundIndex` | — |
| P-IVAL-05 | law | **The generic equality laws (§4.3), for the interval:** equality is an equivalence on valid intervals. It is not bit identity: `[0, 0] == [−0, −0]`, although their lengths are `0.0` and `−0.0` | `PartialEq` | `ValidInterval³` | — |
| P-IMEM-01 | correct result | A point is in an interval exactly when the model says it is, and π and −π are one point | `contains` | `ValidInterval` × `Point` | pilot |
| P-IMEM-02 | correct result | A point is in an interval's interior exactly when the model says it is | `interior_contains` | `ValidInterval` × `Point` | pilot |
| P-IREL-01 | correct result | If one interval contains another, every point of the second is in the first | `contains_interval` | `ValidInterval²` × `Point` | pilot |
| P-IREL-02 | correct result | Containment is exactly the containment of real arcs, `arc_spec::contains` | `contains_interval` | `ValidInterval²` | — |
| P-IREL-03 | correct result | Interior containment is exactly `arc_spec::interior_contains`, and then every point of the second interval is in the first's interior | `interior_contains_interval` | `ValidInterval²` × `Point` | — |
| P-IREL-04 | correct result | Two intervals intersect exactly when one contains a bound of the other (`arc_spec::intersects`); a shared point implies it; it is symmetric | `intersects` | `ValidInterval²` × `Point` | pilot |
| P-IREL-05 | model check | The real-arc relations are consistent with membership: a point is in an interval exactly when the interval contains that point's single-point interval; containment implies every point of `y` is in `x`; interior containment implies every point of `y` is in `x`'s interior | (`arc_spec` only) | `ValidInterval²` × `Point` | — |
| P-IREL-06 | correct result | Interior intersection is exactly `arc_spec::interior_intersects`: the second interval meets the first's interior. It is not symmetric, as upstream documents | `interior_intersects` | `ValidInterval²` × `Point` | pilot |
| P-ISET-01 | correct result | The complement holds every point outside the interval. Apart from a single point, it shares no interior point with the interval, and taking it twice gives the interval back | `complement` | `ValidInterval` × `Point` | pilot |
| P-ISET-02 | correct result | The union is valid and contains both intervals; its bounds are bounds of the two, or it is full | `union` | `ValidInterval²` | pilot |
| P-ISET-03 | correct result | The intersection is valid, holds every common point, and is empty exactly when the two do not intersect | `intersection` | `ValidInterval²` × `Point` | pilot |
| P-ISET-04 | correct result | Adding a point gives a valid interval that contains the point and the original, and changes nothing if the point was already in it | `add_point` | `ValidInterval` × `Point` | pilot |
| P-ISET-05 | correct result | The arc through two points contains both and is the shorter arc, within one ulp of π | `from_point_pair` | `Point²` | pilot |
| P-ISET-06 | correct result | Projection gives a point of a non-empty interval, the point itself if it is in the interval, and otherwise the nearer bound | `project` | non-empty `ValidInterval` × `Point` | pilot |
| P-IMEA-01 | exact output | **The FR-020 containment proof.** For every valid interval except the corner, the length is bit for bit upstream's arithmetic. At the corner, it is one ulp of π | `length` | `ValidInterval` | 217 s (against the restated fix) |
| P-IMEA-02 | law | Every non-empty interval has a non-negative length, positive unless it is a single point; the empty interval's is −1 and the full one's is 2π | `length` | `ValidInterval` | pilot (with the corner as an exception) |
| P-IMEA-03 | model check | The corner is the only valid, non-empty interval for which upstream's arithmetic gives a negative length | (`arc_spec` only) | `ValidInterval` | 0.2 s |
| P-IMEA-04 | correct result | The centre of an interval that is neither empty nor full lies in it | `center` | `ValidInterval` | 30 s |
| P-IMEA-05 | correct result | The complement's centre lies in the complement, for an interval that is neither empty nor full | `complement_center` | `ValidInterval` | 30 s |
| P-IMEA-06 | correct result | The directed Hausdorff distance lies in [0, π]. It is 0 when the second interval contains the first (the converse holds up to rounding: across a one-ulp gap the distance rounds to 0, found in T037), and π when only the second is empty | `directed_hausdorff_distance` | `ValidInterval²` | the 0/π clauses whole; the range clause split, two members weekly |
| P-IEXP-01 | correct result | Expanding by a finite non-negative margin gives a valid interval that contains the original; shrinking gives one it contains; empty and full stay so | `expanded` | `ValidInterval` × finite `AnyF64` | validity 91 s; containment: the corner fails until FR-020, off the corner ⏱, split (R9) |
| P-IEXP-02 | law | Approximate equality is reflexive and symmetric for every finite non-negative error bound, and the default form uses 1e-15 | `approx_eq_with`, `approx_eq` | `ValidInterval²` × finite `AnyF64` | 254 s (reflexive) |
| P-IEXP-03 | exact output | The crossing fix (FR-023) changes an expansion only where upstream's bounds cross: every other result is bit for bit `upstream_expanded`, and there it is empty (shrinking) or full (growing). With P-IREL-02 it implies P-IEXP-01's containment clause | `expanded` | `ValidInterval` × `AnyF64` (no NaN) | added 2026-10-13, after `p_iexp_01_shrink__xinv` found U-011; split by the margin's sign |

### 4.3 Generic trait laws (`verify/src/laws.rs`)

Each law is stated once, generically, and instantiated per type, as 003's `Codec` contract
is (VERIFIED_TYPES §3 and step 4). The same laws exist as quickcheck properties in
`core/tests/trait_property_tests.rs` (Constitution III, `CORE_TRAITS.md`). The Kani
versions prove them for every value of the instance's domain.

| Law module | Laws | Instances in this feature | Earlier instances |
|------------|------|---------------------------|-------------------|
| `laws::region` | `contains_implies_intersects(r, c)`: `r.contains_cell(c) ⇒ r.intersects_cell(c)`. `covering_covers(r, c)`: `r.contains_cell(c) ⇒` some id in `r.cell_union_bound()` contains `c.id()`. Both integer-only | `Cell` (P-CCNT-03) | `CellUnion`: 002's P-UMEM-05 states the model agreement it implies. A follow-up task adds the generic instance within 002's size bounds |
| `laws::eq` | `reflexive(a)`, `symmetric(a, b)`, `transitive(a, b, c)` for `T: PartialEq` | `Cell` (P-CEQ-01), `s1::Interval` (P-IVAL-05) | none: `CellId` and `CellUnion` derive equality on integers |

The instance-specific meaning (for example, "agrees with the identifier model") stays in each
type's own entries. The laws module holds only what every implementor must satisfy.

**Totals:** 13 `Cell` entries and 27 interval entries, 40 in all. P-IREL-05 and P-IMEA-03
are model checks: they prove facts about the reference description that other entries rely
on, without calling the library.

## 5. Operation inventory (`verify/values/INVENTORY.md`)

001's columns: name, scope, reason, domain, crash result, crash harness.

### 5.1 `Cell` (53 entries, FR-002)

| Group | Entries | Scope |
|-------|---------|-------|
| Integer constructors | `from_cell_id`, `from_face`, `from_face_pos_level`, `impl From<CellId>`, `impl Default` | in scope |
| Accessors | `face`, `level`, `id`, `orientation`, `is_leaf`, `size_ij` | in scope |
| Children | `children` | in scope |
| Containment | `contains_cell`, `intersects_cell`, `cell_union_bound`, `impl Region::{contains_cell, intersects_cell, cell_union_bound}` | in scope |
| Equality | `impl PartialEq` | in scope |
| Stored bounds | `bound_uv`, `uv_coord_of_edge` | in scope. `uv_coord_of_edge`'s property is proved whole. `bound_uv`'s are proved within measured levels (research R8) |
| Floating-point geometry | `from_point`, `from_lat_lng`, `size_st`, `ij_coord_of_edge`, `vertex`, `vertex_raw`, `edge`, `edge_raw`, `center`, `center_raw`, `average_area_for_level`, `average_area`, `approx_area`, `exact_area`, `contains_point`, `cap_bound`, `rect_bound`, the 11 distance operations, `impl Region::{cap_bound, rect_bound, contains_point}` (31) | excluded: depends on floating-point geometry (FR-002). The floating-point `Region` invariants of `CORE_TRAITS.md` are tested for `Cell` by `core/tests/trait_property_tests.rs` (`region_cell_cap_contains`, `region_cell_contains_self_center`) and the `prop_cell_*` tests of `core/tests/property_tests.rs` |
| Text | `impl Display` (through `Debug`) | excluded: tool limit, as 001's formatter |

19 + 2 in scope, 32 excluded. The binary codec of `Cell` is 003's (`decoding/INVENTORY.md`),
and the table points to it.

### 5.2 `s1::Interval` (31 entries, FR-003)

All 28 inherent operations and `impl Default` and `impl PartialEq` are in scope. `impl
Display` is excluded as a tool limit. The five operations planning measured first (FR-003)
are in scope on the R0 evidence (research R9).

| Precondition (FR-004) | Operations |
|------------------------|------------|
| none: every `ValidInterval` | the classifiers, `length`, `center`, `complement`, `complement_center`, the relations, `union`, `intersection`, `directed_hausdorff_distance` |
| point in [−π, π] (debug-checked, documented by FR-019) | `new`, `from_point_pair`, `contains`, `interior_contains`, `add_point`, `project` |
| non-empty (debug-checked, documented by FR-019) | `project` |
| `i ≤ 1` (documented `# Panics`) | `bound` |
| margin not NaN (debug-checked through `new`, documented by FR-019, K5) | `expanded` |
| none on the error bound (every `f64`, NaN included) | `approx_eq_with` |

## 6. Changes to the library

| Change | Files | Kind | Test (fails without the change) |
|--------|-------|------|--------------------------------|
| FR-017 (K1): `# Panics` on `from_cell_id`, `from_face`, `from_face_pos_level` and `From<CellId>` | `core/src/s2/cell.rs` | documentation | none needed |
| FR-018 (K2): a hand-written `Deserialize` for `Cell` that reads the same five fields, rejects an invalid identifier or a face, level or orientation that differs from the identifier's, rejects a u-v bound more than one ulp from the identifier's, and returns `Cell::from_cell_id(id)` (research R6). `Serialize` stays derived | `core/src/s2/cell.rs` | behaviour | `cell_tests.rs`: the level-12 edit and an invalid identifier are rejected; every cell to level 7 and 2 million random cells round-trip bit for bit through `serde_json` |
| FR-019 (K3, K5): the debug-only preconditions documented on `new`, `from_point_pair`, `contains`, `interior_contains`, `add_point`, `project` and `expanded` (a NaN margin), saying what release builds do | `core/src/s1/interval.rs` | documentation | none needed |
| FR-020 (K4): the narrow seam fix in `length`, with its rustdoc deviation note (research R5) | `core/src/s1/interval.rs` | behaviour | `interval_tests.rs`: the corner's length, `expanded(3.2)` and the rectangle's area; `python/tests/test_interval.py`: the corner's length |
| D-005 (FR-012): `directed_hausdorff_distance` builds its helper arcs with `Interval::new`, as upstream does (research R9) | `core/src/s1/interval.rs` | behaviour in debug builds only | `interval_tests.rs`: `x = [1, 2]`, `y = [0, 1e−17]` panics without the fix and returns 2 with it |
| The generic trait property tests (research R15) | `core/tests/trait_property_tests.rs` (brought in from `tb/python-bindings/1`, commit `f9805f8`, with an SPDX header), plus `Cell`'s missing instances of the integer `Region` laws (`region_cell_contains_implies_intersects`, `region_cell_union_bound_covers`) | tests | they are the tests |
| FR-023 (D-007, U-011; found by the weekly member `p_iexp_01_shrink__xinv`, 2026-10-13): after upstream's arithmetic, `expanded` returns the empty interval when shrinking gives one the original does not contain, and the full interval when growing gives one that does not contain the original | `core/src/s1/interval.rs` | behaviour | `interval.rs`: Kani's counterexample and three found by search give the empty interval; `rect_tests.rs`: the rectangle with those longitudes shrinks to empty |
| D-006 (found when the generic tests were brought in, research R15): `Rect::cap_bound` takes the centre cap only when `lng.length() <= π`, as upstream does. The port's `rem_euclid` test let every rectangle crossing the antimeridian through | `core/src/s2/rect.rs`, `core/src/s2/rect_tests.rs` (new) | behaviour | `rect_tests.rs`: the 248° rectangle's cap contains its point, and is the pole cap; `region_union_cap_contains` |
| Records | `UPSTREAM.md`: U-007 for the corner, moved from the inherited behaviours, and a note on U-004 that the constructor itself takes the same ids (research R1). `VERIFIED_TYPES.md` | documentation | — |

## 7. Mutants (`verify/mutants/M-3NN-<slug>.patch`)

Each proven entry is the target of at least one mutant (SC-004). Planned, about 34:

- **`Cell`:**
  - the orientation of child `k` taken from the wrong table entry;
  - `is_leaf` true at level 29;
  - `size_ij` off by one level;
  - `contains_cell` reversed;
  - `intersects_cell` reduced to `contains_cell`;
  - the region's covering bound using the parent's identifier;
  - a child's u bound taken from the wrong half;
  - the `y` bounds swapped with `x`;
  - edge `Right` returning `x.lo`;
  - equality comparing levels only;
  - the region's `intersects_cell` dropping the "this contains that" direction, which breaks
    `laws::region` (P-CCNT-03);
  - the covering bound using the first child's identifier, which breaks the covering law
    (P-CCNT-03);
  - equality as `id <= other.id`, which breaks symmetry (P-CEQ-01).
- **The interval:**
  - `is_valid` accepting `−π` as a low bound;
  - `new` not normalising `−π`;
  - `contains` ignoring the seam;
  - `interior_contains` using `≤`;
  - `contains_interval`'s inverted case dropping `¬is_empty`;
  - `intersects` missing one endpoint case;
  - `complement` of a single point being empty;
  - `union` returning the longer of two disjoint gaps;
  - `intersection` returning the longer piece;
  - `add_point` extending the wrong side;
  - `project` returning the farther bound;
  - `center` without the inverted adjustment;
  - `complement_center` off by π;
  - the directed Hausdorff distance swapping `lo` and `hi`;
  - `expanded` testing the full case with `>` for `≥`;
  - `approx_eq_with` comparing only the low bounds;
  - equality as `lo == other.lo && hi <= other.hi`, which breaks symmetry (P-IVAL-05).
- **FR-020 (research R5):**
  - `M-3NN-seam-fix-widened`: the stable form for every inverted interval. It must fail
    P-IMEA-01, which is what contains the correction;
  - `M-3NN-seam-fix-removed`: the fix taken out. It must fail P-IMEA-01 and P-IMEA-02.
- **FR-023 (added 2026-10-13):**
  - `M-343-crossing-fix-removed`: the shrink guard taken out. It must fail P-IEXP-03
    (`p_iexp_03__mneg`); Kani has to find a crossing, so it is in the weekly tier;
  - `M-344-crossing-fix-widened`: the shrink guard testing interior containment. It must fail
    P-IEXP-03, which is what contains the correction;
  - `M-345-grow-guard-reversed`: the grow guard testing the reverse containment. It must fail
    P-IEXP-03 (`p_iexp_03__mpos`).
  - The grow guard removed is not a mutant: no input is known where it acts.

## 8. Records and states

Unchanged from 001:

- the inventory's scope values (`in scope`, `excluded (<reason>)`, `tool-limit`);
- the property statuses;
- the mutant outcomes;
- the evaluation sections.

`verify/values/EVALUATION.md` adds two sections:

- **Floating-point cost**: harness time and memory against the operation mix, comparisons
  and additions versus products.
- **The decision questions of FR-016**: the go/no-go for the remaining value types, whether
  type-level invariants come first, and the ranked follow-ups.
