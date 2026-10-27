# Contract: Signature Audit, `T` versus `impl Into<T>`

Answers the maintainer's planning question: "Check if related function signatures containing
type T should be rewritten to `impl Into<T>`." The rule and its rationale are in research R1.
This file applies the rule to every public signature in `core/src` that takes a bounded type, or
a raw integer standing in for one. The source is grep on 2026-10-27 (research P6), with line
numbers at that date.

## The rule

1. **A parameter of a bounded type takes `T`, never `impl Into<T>`.** A bounded type's sources
   are raw integers, and converting one can fail by definition, so it is `TryFrom`, not `From`.
   `impl Into<T>` would then accept only `T` itself.
2. **`impl Into<T>` stays appropriate where `T` has infallible sources that callers hold**, such
   as the typed identifiers of the shape index and the builder (`ShapeId`, `VertexId`, `EdgeId`,
   `InputEdgeId`). Each wraps an integer with no range. Their validity (an existing shape or
   vertex) is checked where they are used.
3. **A raw integer that stands in for a bounded type is replaced by the type** when it is one of
   the four adopted candidates (spec FR-012). Otherwise it is listed as a follow-up.

## Verdicts

### `impl Into<Level>`: 25 signatures, all become `Level` (breaking, FR-013)

*Corrected at implementation (2026-10-27): the compiler found two more,
`point_compression::encode_points_compressed` and `decode_points_compressed`, whose parameter is
`impl Into<crate::s2::coords::Level>` on a line of its own, which the one-line grep missed.*

| Item | Location | New signature |
|------|----------|---------------|
| `Metric::value` | `s2/metric.rs:61` | `value(&self, level: Level)` |
| `CellId::from_face_pos_level` | `s2/cell_id.rs:239` | `from_face_pos_level(face: Face, pos: u64, level: Level)` |
| `CellId::child_position` | `s2/cell_id.rs:338` | `child_position(self, level: Level)` |
| `CellId::parent_at_level` | `s2/cell_id.rs:356` | `parent_at_level(self, level: Level)` |
| `CellId::child_begin_at_level` | `s2/cell_id.rs:426` | `child_begin_at_level(self, level: Level)` |
| `CellId::child_end_at_level` | `s2/cell_id.rs:436` | `child_end_at_level(self, level: Level)` |
| `CellId::begin` | `s2/cell_id.rs:446` | `begin(level: Level)` |
| `CellId::end` | `s2/cell_id.rs:456` | `end(level: Level)` |
| `CellId::size_st` | `s2/cell_id.rs:659` | `size_st(level: Level)` |
| `CellId::vertex_neighbors` | `s2/cell_id.rs:768` | `vertex_neighbors(self, level: Level)` |
| `CellId::all_neighbors` | `s2/cell_id.rs:807` | `all_neighbors(self, level: Level)` |
| `lsb_for_level` | `s2/cell_id.rs:962` | `lsb_for_level(level: Level)` |
| `size_ij` | `s2/cell_id.rs:973` | `size_ij(level: Level)` |
| `ij_level_to_bound_uv` | `s2/cell_id.rs:1037` | `ij_level_to_bound_uv(i: i32, j: i32, level: Level)` |
| `testing::random_cell_id_at_level` | `s2/testing.rs:89` | `random_cell_id_at_level(rng, level: Level)` |
| `RegionCoverer::min_level` | `s2/region_coverer.rs:98` | `min_level(self, level: Level)` |
| `RegionCoverer::max_level` | `s2/region_coverer.rs:104` | `max_level(self, level: Level)` |
| `Polygon::snapped` | `s2/polygon.rs:716` | `snapped(polygon, snap_level: Level)` |
| `Cell::from_face_pos_level` | `s2/cell.rs:181` | `from_face_pos_level(face: Face, pos: u64, level: Level)` |
| `Cell::average_area_for_level` | `s2/cell.rs:355` | `average_area_for_level(level: Level)` |
| `CellIdSnapFunction::new` | `s2/builder/snap.rs:104` | `new(level: Level)` |
| `CellIdSnapFunction::with_snap_radius` | `s2/builder/snap.rs:111` | `with_snap_radius(level: Level, snap_radius: Angle)` |
| `CellIdSnapFunction::min_snap_radius_for_level` | `s2/builder/snap.rs:132` | `min_snap_radius_for_level(level: Level)` |
| `point_compression::encode_points_compressed` | `s2/point_compression.rs:346` | `encode_points_compressed(w, vertices, level: Level)` |
| `point_compression::decode_points_compressed` | `s2/point_compression.rs:415` | `decode_points_compressed(r, level: Level, num_points)` |

### Already plain `Level`: 2 signatures, unchanged in kind

| Item | Location | Change |
|------|----------|--------|
| `CellUnion::expand_at_level` | `s2/cell_union.rs:420` | none |
| `CellUnion::denormalize` | `s2/cell_union.rs:529` | `level_mod: u8` becomes `LevelMod` (R6) |

### Faces: 4 raw-integer signatures become `Face`; 12 already take `Face`

| Item | Location | New signature |
|------|----------|---------------|
| `CellId::from_face` | `s2/cell_id.rs:224` | `from_face(face: Face)` |
| `Cell::from_face` | `s2/cell.rs:169` | `from_face(face: Face)` |
| both `from_face_pos_level` | as above | `face: Face` |
| `face_uv_to_xyz`, `valid_face_xyz_to_uv`, `face_xyz_to_uv`, `face_xyz_to_uvw`, `face_si_ti_to_xyz`, `get_u_norm`, `get_v_norm`, `get_norm`, `get_u_axis`, `get_v_axis`, `get_uvw_axis`, `get_uvw_face`; `from_face_ij`; `UVEdgeClipper::from_face_rect`, `init` | `s2/coords.rs`, `s2/cell_id.rs:986`, `s2/uv_edge_clipper.rs` | unchanged: plain `Face` is already right |

### Vertex and edge indices

| Item | Location | Verdict |
|------|----------|---------|
| `Cell::vertex(k: usize)`, `vertex_raw(k: usize)` | `s2/cell.rs:243`, `:253` | become `CellVertex` (R5, gated) |
| `Cell::edge`, `edge_raw`, `uv_coord_of_edge`, `ij_coord_of_edge` (`CellEdge`) | `s2/cell.rs:260–303` | unchanged: plain enum |
| `s2::Rect::vertex(v: RectVertex)` | `s2/rect.rs:287` | unchanged: plain enum |
| `r2::Rect::vertex(k: i32)` | `r2/rect.rs:191` | follow-up: it wraps modulo 4, as upstream does, so nothing panics; a typed vertex would be consistency only |

### Other raw integers that stand in for a bounded type (follow-ups, not adopted)

| Item | Location | Range | Why not now |
|------|----------|-------|-------------|
| `get_uvw_axis(face, axis: u8)`, `get_uvw_face(face, axis: u8, direction: u8)` | `s2/coords.rs:690`, `:702` | `axis` 0..=2 (`r3::Axis` exists), `direction` 0..=1 | low-level coordinate API; not in FR-005 |
| `face_si_ti_to_xyz(face, si: u32, ti: u32)` | `s2/coords.rs:582` | 0..=2³¹ | floating-point domain; ranked with the class (d) invariants |
| `fractal::set_max_level(i32)`, `num_vertices_at_level(i32)` | `s2/fractal.rs:172`, `:345` | a test-data generator's level | testing utility |
| `DensityTree::new(…, max_level: u8)` | `s2/density_tree.rs:444` | 0..=30 | uses `Level` after the follow-up; not prototyped |
| `RegionCoverer::max_cells(usize)` | `s2/region_coverer.rs:116` | clamped to ≥ 1, where upstream does not clamp | a deviation of its own, recorded |

### `impl Into<Id>` for typed identifiers: appropriate, unchanged

`ContainsPointQuery::shape_contains`, `boolean_operation::…::new`,
`ShapeIndex::find_by_shape_id`, `shape`, `clipped`, `ShapeNestingQuery::compute_shape_nesting`
and `ShapeEdgeId::new` (`impl Into<ShapeId>`); `Graph::vertex`, `degree`, `edge_ids`
(`impl Into<VertexId>`); `Graph::edge`, `input_edge_ids`, `input_edge_id_set_id`,
`min_input_edge_id`, `LabelFetcher::fetch` (`impl Into<EdgeId>`); `Builder::input_edge`,
`Graph::labels` (`impl Into<InputEdgeId>`); `S2Error::new` (`impl Into<String>`).

Rule 2 applies: these are infallible wrappers, so `impl Into` is the idiom. Whether their
indices should be checked at construction is a different question (validity relative to a
container, class c) and is ranked as a follow-up.

## Return types

The ten functions that return `Level` or `Option<Level>` (`CellId::level`,
`common_ancestor_level`, `Cell::level`, `PaddedCell::level`, `Metric::min_level`, `max_level`,
`closest_level`, `Polygon::get_snap_level`, `CellIdSnapFunction::level`,
`level_for_max_snap_radius`) are unchanged. Returning the bounded type is what lets callers pass
it on without converting.
