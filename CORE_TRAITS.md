# S2RST Core Crate Traits Catalog

This document catalogs all trait types defined within the s2rst core crate, their implementations, hierarchy relationships, and the **invariants** each trait imposes on conforming implementations. Only traits declared with `pub trait` or `trait` in `core/src/` are included; external trait implementations are excluded.

The *invariant* sections below define the contract each trait's implementations must satisfy. These are the properties verified by the generic trait-level property tests in `core/tests/trait_property_tests.rs`.

## Trait Hierarchy Overview

```
DistanceTarget (s2/distance_target.rs) — base target for distance queries
├── closest_edge_query::Target
├── closest_point_query::Target
├── closest_cell_query::Target
└── furthest_edge_query::Target

DistanceTarget (s2/edge_query.rs) — independent, min/max variants

Region          : Debug                    (bounded regions on S²)
Shape           : Send + Sync + Debug      (indexable geometry)
S2Encode                                   (binary encode)
S2Decode        : Sized                    (binary decode)
Projection      : Debug                    (sphere ↔ plane map)
Layer           : Debug                    (S2Builder output adapter)
SnapFunction    : Send + Debug             (vertex snapping policy)

Internal (non-public):
├── RemEuclid           (f64 rem-workaround helper)
├── Accumulator         (Kahan/simple sum dispatch)
├── LoopRelation        (loop containment dispatch)
└── ShapeIndexFactory   (query test plumbing)
```

## Trait Catalog by Topic

### Distance Query Traits

#### DistanceTarget
**Location:** `core/src/s2/distance_target.rs:35`
**Supertraits:** *(none)*

**Methods:**
- `fn cap_bound(&self) -> Cap`
- `fn set_max_error(&mut self, _max_error: ChordAngle) -> bool`

**Invariants:**
- `cap_bound()` returns a cap that covers every point that could affect a distance computation against this target. The returned cap must be non-empty for non-trivial targets.
- `cap_bound()` is deterministic: repeat calls must return identical caps (for targets not mutated in between).
- `set_max_error(e)` accepts any non-negative `ChordAngle` and must not panic.

**Implementations (19):** `PointTarget`, `EdgeTarget`, `CellTarget`, `CellUnionTarget`, `ShapeIndexTarget<'_>` across the four query modules (closest_edge / closest_point / closest_cell / furthest_edge).

---

#### closest_edge_query::Target
**Location:** `core/src/s2/closest_edge_query.rs:174`
**Supertraits:** `DistanceTarget`

**Methods:**
- `fn update_distance_to_point(&self, p: Point, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn update_distance_to_edge(&self, v0: Point, v1: Point, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn visit_containing_shapes(&self, index: &ShapeIndex, visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>) -> ControlFlow<()>`
- `fn max_brute_force_index_size(&self) -> i32` (default: 30)

**Invariants:**
- **Monotone refinement:** for any call `update_distance_to_X(..., dist_limit)` returning `(d', updated)`, if `updated == true` then `d' < dist_limit`; if `updated == false` then the returned chord angle equals `dist_limit`.
- **Cap consistency:** the distance from any point `p` outside `cap_bound().complement()` is ≥ `(1 - cos(cap.radius))`. More concretely, `update_distance_to_point(p, ChordAngle::infinity())` returns a distance ≤ chord-angle-to-cap-boundary plus the cap radius.
- `update_distance_to_cell(&cell, ∞)` returns `(d, true)` with `d ≤ 2` (chord-angle max).

**Implementations (4):** `PointTarget`, `EdgeTarget`, `CellTarget`, `ShapeIndexTarget<'_>`.

---

#### closest_point_query::Target
**Location:** `core/src/s2/closest_point_query.rs:152`
**Supertraits:** `DistanceTarget`

**Methods:**
- `fn update_distance_to_point(&self, p: Point, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn max_brute_force_index_size(&self) -> i32` (default: 200)

**Invariants:** same monotone-refinement contract as `closest_edge_query::Target`.

**Implementations (4):** `PointTarget`, `EdgeTarget`, `CellTarget`, `ShapeIndexTarget<'_>`.

---

#### closest_cell_query::Target
**Location:** `core/src/s2/closest_cell_query.rs:134`
**Supertraits:** `DistanceTarget`

**Methods:**
- `fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn max_brute_force_index_size(&self) -> i32` (default: 200)

**Invariants:** same monotone-refinement contract.

**Implementations (5):** `PointTarget`, `EdgeTarget`, `CellTarget`, `CellUnionTarget`, `ShapeIndexTarget<'_>`.

---

#### furthest_edge_query::Target
**Location:** `core/src/s2/furthest_edge_query.rs:148`
**Supertraits:** `DistanceTarget`

**Methods:**
- `fn update_distance_to_point(&self, p: Point, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn update_distance_to_edge(&self, v0: Point, v1: Point, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool)`
- `fn visit_containing_shapes(&self, _index: &ShapeIndex, _visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>)` (default: no-op)
- `fn max_brute_force_index_size(&self) -> i32` (default: 100)

**Invariants:**
- **Reverse monotone-refinement:** `update_distance_to_X(..., dist_limit)` returning `(d', updated)`. If `updated` then `d' > dist_limit` (furthest is a *maximum* search).

**Implementations (4):** `PointTarget`, `EdgeTarget`, `CellTarget`, `ShapeIndexTarget<'_>`.

---

#### edge_query::DistanceTarget
**Location:** `core/src/s2/edge_query.rs:136`
**Supertraits:** *(none)*

**Methods:**
- `fn update_min_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle>`
- `fn update_min_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle>`
- `fn update_max_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle>`
- `fn update_max_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle>`
- `fn interior_point(&self) -> Option<Point>` (default: `None`)

**Invariants:**
- **Min methods:** return `Some(d')` only if `d' < dist`; else `None`.
- **Max methods:** return `Some(d')` only if `d' > dist`; else `None`.
- Min and max are never larger / smaller than each other: for the same inputs, `min_distance ≤ max_distance`.

**Implementations (2):** `PointTarget`, `EdgeTarget`.

---

### Geometry Traits

#### Region
**Location:** `core/src/s2/region.rs:41`
**Supertraits:** `std::fmt::Debug`

**Methods:**
- `fn cap_bound(&self) -> Cap`
- `fn rect_bound(&self) -> Rect`
- `fn cell_union_bound(&self) -> Vec<CellId>`
- `fn contains_cell(&self, cell: &Cell) -> bool`
- `fn intersects_cell(&self, cell: &Cell) -> bool`
- `fn contains_point(&self, p: &Point) -> bool`

**Invariants (verified by generic properties):**
- **Bounding cap contains contained points.** If `contains_point(p)` is `true`, then `cap_bound().contains_point(p)` is `true`.
- **Bounding rect contains contained points.** If `contains_point(p)` is `true`, then `rect_bound().contains_latlng(LatLng::from_point(p))` is `true`.
- **Cell-union cover covers the region.** Every cell in `cell_union_bound()` either intersects the region or is redundant; taken together, they cover every contained point.
- **Containment implies intersection.** `contains_cell(c) ⇒ intersects_cell(c)` for all cells `c`.
- **Non-intersection implies non-containment.** If `!intersects_cell(c)`, then `!contains_cell(c)`. For a region whose cell test checks the vertices first (`Cap`, `Rect`), also `!contains_point(&v)` for every vertex `v` of `c`; this second part does not hold for `Cell`, whose `intersects_cell` compares ids while its closed region contains the shared vertices of a neighbour (as upstream's `S2Cell`; spec 008 T022).
- **Determinism.** All methods are pure functions of `(&self, inputs)`; repeated calls yield identical results.
- **Debug is implemented** (from supertrait).

**Proven instances (Kani, `verify/src/laws.rs`):** containment implies intersection for `Cell`
(P-CCNT-03) and `Cap` (P-CPREG-01, by composition, spec 008); the vertex part of
non-intersection for `Cap` (P-CPREG-02); the bounding cap for `Cap` (P-CPREG-03); the
cell-union cover for `Cell` (P-CCNT-03; for `Cap` not proven, P-CPREG-04). Every invariant
also has quickcheck instances (below).

**Implementations (12):** `Cap`, `Rect`, `Cell`, `PointRegion`, `RegionIntersection`, `RegionUnion`, `Polyline`, `Loop`, `Polygon`, `CellUnion`, `ShapeIndexRegion<'_>`, `ShapeIndexBufferedRegion<'_>`.

---

#### Shape
**Location:** `core/src/s2/shape.rs:379`
**Supertraits:** `Send + Sync + std::fmt::Debug`

**Methods:**
- `fn num_edges(&self) -> usize`
- `fn edge(&self, id: usize) -> Edge`
- `fn reference_point(&self) -> ReferencePoint`
- `fn num_chains(&self) -> usize`
- `fn chain(&self, chain_id: usize) -> Chain`
- `fn chain_edge(&self, chain_id: usize, offset: usize) -> Edge`
- `fn chain_position(&self, edge_id: usize) -> ChainPosition`
- `fn dimension(&self) -> Dimension`
- `fn is_empty(&self) -> bool` (default)
- `fn is_full(&self) -> bool` (default)
- `fn has_interior(&self) -> bool` (default: `dimension() == Polygon`)
- `fn type_tag(&self) -> u32` (default: 0)
- `fn encode_tagged(&self, w, hint) -> io::Result<()>` (default: `Unsupported`)

**Invariants:**
- **Chain edge counts sum.** `Σ chain(i).length == num_edges()` for `i in 0..num_chains()`.
- **Chain/position round-trip.** For every `edge_id ∈ 0..num_edges()`, let `ChainPosition{chain_id, offset} = chain_position(edge_id)`; then `chain_edge(chain_id, offset) == edge(edge_id)`.
- **Chain edges are a partition.** Concatenating `chain_edge(c, 0..chain(c).length)` for `c in 0..num_chains()` yields the sequence `edge(0..num_edges())`.
- **is_empty consistency.** `is_empty() ⇒ num_edges() == 0` (and for dim-2, also `num_chains() == 0`).
- **has_interior matches dimension.** `has_interior() == (dimension() == Dimension::Polygon)`.
- **is_full implies dim-2.** `is_full() ⇒ dimension() == Dimension::Polygon && !is_empty()`.
- **reference_point is on the unit sphere.** `|reference_point().point|² ≈ 1.0`.
- **Send + Sync + Debug** are implementable (trait-level bound).

**Implementations (12):** `PointVector`, `LaxPolyline`, `LaxLoop`, `Polyline`, `LoopShape`, `Loop`, `PolygonShape`, `Polygon`, `LaxPolygon`, `EdgeVectorShape`, `GraphShape`, `WrappedShape`.

---

### Encoding Traits

#### S2Encode
**Location:** `core/src/s2/encoding.rs:54`
**Supertraits:** *(none)*

**Methods:**
- `fn encode(&self, w: &mut dyn Write) -> io::Result<()>`

**Invariants:**
- **Determinism.** Two calls on the same value to two `Vec<u8>` writers produce the same bytes.
- **Non-empty output.** `encode` writes at least one byte (no type is encoded as zero bytes in S2).

---

#### S2Decode
**Location:** `core/src/s2/encoding.rs:64`
**Supertraits:** `Sized`

**Methods:**
- `fn decode(r: &mut dyn Read) -> io::Result<Self>`

**Invariants (paired with `S2Encode`):**
- **Round-trip.** For every value `v: T` where `T: S2Encode + S2Decode`, decoding the bytes produced by `v.encode(&mut buf)` yields a value equal (by the type's domain equality, e.g., set-equality for `CellUnion`) to `v`.
- **Decode rejects empty/truncated input** by returning `Err`.

**Proven instances (Kani):** the round trip of every codec of spec 003 (`verify/decoding/`),
bytes for bytes; for `Cap`, also under `Cap`'s own equality (P-CPCOD-01, spec 008).

**Implementations (12 each):** `CellId`, `Point`, `Cap`, `Rect`, `CellUnion`, `Polyline`, `Loop`, `Polygon`, `LaxPolyline`, `LaxPolygon`, `PointVector`, `Cell`.

---

### Map Projection Trait

#### Projection
**Location:** `core/src/s2/projections.rs:22`
**Supertraits:** `std::fmt::Debug`

**Methods:**
- `fn project(&self, p: Point) -> r2::Point`
- `fn unproject(&self, p: r2::Point) -> Point`
- `fn from_lat_lng(&self, ll: LatLng) -> r2::Point`
- `fn to_lat_lng(&self, p: r2::Point) -> LatLng`
- `fn interpolate(&self, f: f64, a: r2::Point, b: r2::Point) -> r2::Point`
- `fn wrap_distance(&self) -> r2::Point`
- `fn wrap_destination(&self, a, b) -> r2::Point` (default)

**Invariants:**
- **Project/unproject round-trip (approx).** For any unit point `p` that maps to a non-wrapped projected point, `unproject(project(p))` is approximately `p` (within a small ε tolerance).
- **LatLng round-trip.** `from_lat_lng(to_lat_lng(project(p)))` ≈ `project(p)` for valid latitudes.
- **Interpolation endpoints.** `interpolate(0, a, b) ≈ a` and `interpolate(1, a, b) ≈ b`.
- **Interpolation linearity.** `interpolate(0.5, a, b)` is ε-close to the midpoint of `a` and `b` (for non-wrapping projections) or to the wrap-destination midpoint.
- **Wrap distance is non-negative componentwise.**
- `wrap_destination(a, a) == a`.

**Implementations (2):** `PlateCarreeProjection`, `MercatorProjection`.

---

### Builder Traits

#### Layer
**Location:** `core/src/s2/builder/layer.rs:29`
**Supertraits:** `std::fmt::Debug`

**Methods:**
- `fn graph_options(&self) -> GraphOptions`
- `fn build(&mut self, graph: &Graph, error: &mut S2Error)`
- `fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>`

**Invariants:**
- **`graph_options()` is deterministic** and returns the same value across calls until the layer is mutated.
- **`build` does not panic** on an empty graph (must fail via the `error` out-parameter instead).
- **After build, `into_any` is safe** (tested structurally via the type system).

**Implementations (16):** `S2PointVectorLayer`, `S2PolylineLayer`, `S2PolylineVectorLayer`, `LaxPolylineLayer`, `LaxPolygonLayer`, `S2PolygonLayer`, `IndexedLaxPolygonLayer`, `IndexedLaxPolylineLayer`, `IndexedS2PolygonLayer`, `IndexedS2PolylineLayer`, `IndexedS2PolylineVectorLayer`, `IndexedS2PointVectorLayer`, `SharedEdgeClippingLayer`, `EdgeCollectorLayer`, `NormalizingLayer`, `WindingLayer`.

---

#### SnapFunction
**Location:** `core/src/s2/builder/snap.rs:29`
**Supertraits:** `Send + std::fmt::Debug`

**Methods:**
- `fn snap_radius(&self) -> Angle`
- `fn snap_point(&self, point: Point) -> Point`
- `fn min_vertex_separation(&self) -> Angle`
- `fn min_edge_vertex_separation(&self) -> Angle`
- `fn clone_snap(&self) -> Box<dyn SnapFunction>`

**Invariants:**
- **Snap radius bound.** For every input `p`, the angle between `p` and `snap_point(p)` is ≤ `snap_radius()`.
- **Output is unit-length.** `|snap_point(p)|² ≈ 1.0`.
- **Idempotence.** `snap_point(snap_point(p)) == snap_point(p)` — snapping an already-snapped point is a no-op.
- **Separation bounds.** `min_vertex_separation() ≥ 0` and `min_vertex_separation() ≤ 2 · snap_radius()`.
- **`min_edge_vertex_separation() ≤ min_vertex_separation()`** (edge-vertex separation is at most the vertex separation).
- **Clone fidelity.** `clone_snap()` produces a snap function with identical `snap_radius()`, `min_vertex_separation()`, `min_edge_vertex_separation()`, and behaviour of `snap_point` on any point.

**Implementations (3):** `IdentitySnapFunction`, `S2CellIdSnapFunction`, `IntLatLngSnapFunction`.

---

### Internal Helper Traits (Non-Public)

#### RemEuclid — `core/src/s1/interval.rs:515`
Private utility. `fn rem_euclid_workaround(x: f64, y: f64) -> f64`. Impl: `f64`.

#### Accumulator — `core/src/s2/loop_measures.rs:111`
Private loop-measurement helper. `fn acc_add(&mut self, value: f64)`. Impls: `f64`, `KahanSum`.

#### LoopRelation — `core/src/s2/s2loop.rs:974`
Private loop-relation dispatch. Methods: `a_crossing_target`, `b_crossing_target`, `wedges_cross`.

---

## Trait Hierarchy Summary

### Distance Query Trait Hierarchy
```
DistanceTarget (s2/distance_target.rs) — base
├── closest_edge_query::Target
├── closest_point_query::Target
├── closest_cell_query::Target
└── furthest_edge_query::Target

edge_query::DistanceTarget — independent (min + max variants)
```

### Geometry Trait Hierarchy
```
Region       : Debug
Shape        : Send + Sync + Debug
```

### Builder Trait Hierarchy
```
Layer        : Debug
SnapFunction : Send + Debug
```

### Encoding Trait Hierarchy
```
S2Encode     (independent)
S2Decode     : Sized
```

### Projection Trait Hierarchy
```
Projection   : Debug
```

---

## Generic Property-Test Harness

The invariants above are encoded as generic functions in
`core/tests/trait_property_tests.rs`, each parameterized by:

- a concrete type `T` implementing the trait, and
- a **data generator** that produces arbitrary `T` values from a
  `quickcheck::Gen` source (or from seed integers).

For composite types (for example, `RegionIntersection<R1, R2>`), the
generators compose: a generator for the composite is built from
generators for its constituents. Each concrete type therefore only
has to define a generator and satisfy the trait — the invariant
tests come for free via the shared harness.

See `core/tests/trait_property_tests.rs` for the generic property
functions (e.g., `region_properties::cap_bound_contains_points`)
and their instantiations for each concrete type.

The same laws are stated once more for the Kani model checker, in
`verify/src/laws.rs` (`laws::region`, `laws::eq`, `laws::ord`), and
instantiated over symbolic values in the `values` suite: the `Region`
laws for `Cell` (P-CCNT-03) and `Cap` (P-CPREG-01 to -03), equality for
`Cell`, `s1::Interval`, `r2::Rect`, `Point` and `Cap`. Spec 008 added two
`Region` laws to both harnesses: a cell the region does not intersect
has no vertex the region contains (`no_intersect_no_vertex`; a law for
`Cap` and `Rect`, not for `Cell`), and the bounding cap contains every
contained point (`cap_bound_contains_points`). Each Kani law names its
quickcheck twin in its doc comment; the catalogue is
`verify/values/PROPERTIES.md` §3.

---

## Index (Alphabetical)

### Public Traits
- **DistanceTarget** (distance_target module) — base for distance query targets
- **DistanceTarget** (edge_query module) — min/max point & edge distances
- **Layer** — S2Builder layer interface
- **Projection** — sphere ↔ plane mapping
- **Region** — bounded geometric regions on S²
- **S2Decode** — binary decoding for S2 types
- **S2Encode** — binary encoding for S2 types
- **Shape** — geometry with edges, chains, and dimension
- **SnapFunction** — vertex snapping policies
- **Target** (closest_cell_query)
- **Target** (closest_edge_query)
- **Target** (closest_point_query)
- **Target** (furthest_edge_query)

### Private / Internal Traits
- **Accumulator** — loop measurement accumulator
- **LoopRelation** — loop containment relation dispatch
- **RemEuclid** — IEEE 754 remainder workaround

### Implementation Count by Trait
| Trait | Count |
|-------|-------|
| DistanceTarget (distance_target) | 19 |
| Target (closest_edge_query) | 4 |
| Target (closest_point_query) | 4 |
| Target (closest_cell_query) | 5 |
| Target (furthest_edge_query) | 4 |
| DistanceTarget (edge_query) | 2 |
| Region | 12 |
| Shape | 12 |
| S2Encode | 12 |
| S2Decode | 12 |
| Projection | 2 |
| Layer | 16 |
| SnapFunction | 3 |

**Total public trait definitions:** 13
**Total private trait definitions:** 3
**Total implementations:** ~120+

---

*Document generated from s2rst core crate. This catalog excludes
external trait implementations (From, Debug, Display, etc.) and
focuses on in-crate geometry, query, builder, and encoding
abstractions.*
