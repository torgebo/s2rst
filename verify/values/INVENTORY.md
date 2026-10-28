# Operation Inventory: the value types (specs 004–007, FR-002, FR-003)

Sections 1–2 are spec 004's (`Cell`, `s1::Interval`); sections 3–4 are spec 005's
(`r1::Interval`, `s2::Rect`; field definitions in
`specs/005-formal-verify-r1-interval-rect/data-model.md` §5); sections 5–6 are spec 006's
(`r2::Rect`, `s2::Point`; `specs/006-formal-verify-r2-rect-point/data-model.md` §5); sections
7–8 are spec 007's (`s1::ChordAngle`, `s1::Angle`;
`specs/007-formal-verify-chord-angle-angle/data-model.md` §5).

Every public operation of `s2rst::s2::Cell` (53 entries: 43 inherent operations, 4
hand-written trait implementations and the 6 methods of the region interface) and of
`s2rst::s1::Interval` (31 entries: 28 inherent operations and 3 hand-written trait
implementations). Field definitions: `specs/004-formal-verify-cell-interval/data-model.md`
§5, which keeps 001's columns. Domain classes: data-model §2.

Each in-scope row has a crash-freedom obligation **C-&lt;entry&gt;**: "the operation completes
without panic, arithmetic overflow, undefined behaviour or a firing debug check for every
input in *domain*, assuming *preconditions*" (FR-004). Both types are fixed-size values, so
the domains carry no size bound.

**Summary** (T026, 2026-10-12): `Cell` 21 in scope (19 integer entries and the 2
stored-bound readers), 32 excluded (31 floating-point geometry, 1 `Display`); `s1::Interval` 30
in scope, 1 excluded (`Display`). Every in-scope row is proven crash-free over its whole
domain: 19 crash harnesses (9 `cl_crash`, 10 `iv_crash`), all covers satisfied, 350 s of
harness time, 2 min 30 s of wall time with 4 jobs on the 8-thread workstation. The binary
codec of `Cell` is spec 003's (`decoding/INVENTORY.md`, `Cell` rows). The property harnesses
that cover each operation are in `PROPERTIES.md`.

**Gap 2** (2026-10-27, section 11): of the 31 floating-point rows, 19 are now proven crash-free
(10 per change, 9 weekly), with the abstract u-v bound and contracts; 12 are not (five out of
memory at 8 GiB, now in the xlarge tier, and the three distances to a point, which fail the NaN
check off the unit sphere). `Cell` has 40 crash-free rows of 52 (`Display` excluded).

## 1. `Cell` (53 entries)

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `Cell::from_cell_id` | in scope | | `ValidCell` | id is a valid cell (`# Panics`, FR-017) | `cl_crash::c_from_cell_id` (13.0 s), `c_from_cell_id_largest` | proven |
| 2 | `Cell::from_point` | in scope (gap 2) |  | `BoxPoint` but the origin | not the zero vector (`# Panics`; `cl_crash::n_from_point`) | `cl_crash::c_fg_from_point` (689.2 s, weekly); stub `any_uv_bound` | proven |
| 3 | `Cell::from_lat_lng` | in scope (gap 2, under contract) |  | `ValidLatLng` |  | `cl_crash::c_fg_from_lat_lng` (496.1 s, weekly); `sin_cos_contract`; stub `any_uv_bound` | proven-under-contract |
| 4 | `Cell::from_face` | in scope | | faces 0–5 | face ≤ 5 (`# Panics`, FR-017) | `cl_crash::c_from_face` (1.6 s) | proven |
| 5 | `Cell::from_face_pos_level` | in scope | | faces 0–5 × `pos < 2^61` × levels 0–30 | as `CellId::from_face_pos_level` (`# Panics`) | `cl_crash::c_from_face_pos_level` (13.1 s) | proven |
| 6 | `Cell::face` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 7 | `Cell::level` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 8 | `Cell::id` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 9 | `Cell::orientation` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 10 | `Cell::is_leaf` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 11 | `Cell::size_ij` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 12 | `Cell::size_st` | in scope (gap 2) |  | `ValidCell` |  | `cl_crash::c_fg_size_and_ij` (21.8 s, per change); stub `any_uv_bound` | proven |
| 13 | `Cell::bound_uv` | in scope (stored-bound reader) | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 14 | `Cell::vertex` | in scope (gap 2) |  | `ValidCell` × `k < 4` | `k ≤ 3` (an index; `# Panics` added) | `cl_crash::c_fg_vertex` (25.1 s, per change); stub `any_uv_bound` | proven |
| 15 | `Cell::vertex_raw` | in scope (gap 2) |  | `ValidCell` × `k < 4` | `k ≤ 3` (an index; `# Panics` added) | `cl_crash::c_fg_vertex` (25.1 s, per change); stub `any_uv_bound` | proven |
| 16 | `Cell::edge` | in scope (gap 2) |  | `ValidCell` × `Edge` |  | `cl_crash::c_fg_edge` (29.2 s, per change); stub `any_uv_bound` | proven |
| 17 | `Cell::edge_raw` | in scope (gap 2) |  | `ValidCell` × `Edge` |  | `cl_crash::c_fg_edge` (29.2 s, per change); stub `any_uv_bound` | proven |
| 18 | `Cell::center` | in scope (gap 2) |  | `ValidCell` |  | `cl_crash::c_fg_center` (128.3 s, weekly); stub `any_uv_bound` | proven |
| 19 | `Cell::center_raw` | in scope (gap 2) |  | `ValidCell` |  | `cl_crash::c_fg_center` (128.3 s, weekly); stub `any_uv_bound` | proven |
| 20 | `Cell::uv_coord_of_edge` | in scope (stored-bound reader) | | `ValidCell` × `Edge` | | `cl_crash::c_uv_coord_of_edge` (10.3 s) | proven |
| 21 | `Cell::ij_coord_of_edge` | in scope (gap 2) |  | `ValidCell` × `Edge` |  | `cl_crash::c_fg_size_and_ij` (21.8 s, per change); stub `any_uv_bound` | proven |
| 22 | `Cell::children` | in scope | | `ValidCell` | | `cl_crash::c_children` (35.6 s) | proven |
| 23 | `Cell::average_area_for_level` | in scope (gap 2) |  | `Level` |  | `cl_crash::c_fg_average_area` (8.9 s, per change); stub `any_uv_bound` | proven |
| 24 | `Cell::average_area` | in scope (gap 2) |  | `ValidCell` |  | `cl_crash::c_fg_average_area` (8.9 s, per change); stub `any_uv_bound` | proven |
| 25 | `Cell::approx_area` | in scope (gap 2) |  | `ValidCell`, the abstract cell's vertices |  | `cl_crash::c_fg_approx_area` (216.4 s, weekly); stub `fixed_vertex`; stub `any_uv_bound` | proven |
| 26 | `Cell::exact_area` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell`, the abstract cell's vertices |  | `cl_crash::c_fg_exact_area` (xlarge); `atan2_contract`, `tan_contract`, `atan_contract`; stub `fixed_vertex`; stub `any_uv_bound` | not proven (tool limit) |
| 27 | `Cell::contains_point` | in scope (gap 2) |  | `ValidCell` × `BoxPoint` |  | `cl_crash::c_fg_contains_point` (85.0 s, per change); stub `any_uv_bound` | proven |
| 28 | `Cell::contains_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` (32.6 s) | proven |
| 29 | `Cell::intersects_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` | proven |
| 30 | `Cell::cap_bound` | in scope (gap 2) |  | `ValidCell`, the abstract cell's vertices |  | `cl_crash::c_fg_cap_bound` (411.2 s, weekly); stub `fixed_vertex`; stub `any_uv_bound` | proven |
| 31 | `Cell::rect_bound` | in scope (gap 2, under contract) |  | `ValidCell` |  | `cl_crash::c_fg_rect_bound` (279.8 s, weekly); `atan2_contract`; stub `any_uv_bound` | proven-under-contract |
| 32 | `Cell::cell_union_bound` | in scope | | `ValidCell` | | `cl_crash::c_containment` | proven |
| 33 | `Cell::distance_to_point` | not proven | the harness `cl_crash::c_fg_distance_to_point` failed Kani's NaN check after 2874 s weekly (gap 2, 2026-10-27): on box points off the unit sphere `edge_distance` takes the square root of `1 − pq²` with `pq² > 1`, as upstream's formula does; a unit domain needs the squared length, which the checker cannot afford here. A NaN is not a panic in Rust; the harness is removed (`verify/COVERAGE.md`). Tested as before | `ValidCell` × `BoxPoint` |  |  | not proven (NaN check) |
| 34 | `Cell::distance_to_edge` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell` × `BoxPoint`², the abstract cell |  | `cl_crash::c_fg_distance_to_edge` (xlarge); stubs `fixed_vertex`, `any_contains_point`; stub `any_uv_bound` | not proven (tool limit) |
| 35 | `Cell::distance_to_cell` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell`², the abstract pair × `Chord` |  | `cl_crash::c_fg_distance_to_cell` (xlarge); stubs `pair_vertex`, `any_contains_point`; stub `any_uv_bound` | not proven (tool limit) |
| 36 | `Cell::max_distance_to_point` | not proven | the harness `cl_crash::c_fg_max_distance_to_point` failed Kani's NaN check after 2246 s weekly (gap 2, 2026-10-27): on box points off the unit sphere `edge_distance` takes the square root of `1 − pq²` with `pq² > 1`, as upstream's formula does; a unit domain needs the squared length, which the checker cannot afford here. It also reaches `distance_to_point`. A NaN is not a panic in Rust; the harness is removed (`verify/COVERAGE.md`). Tested as before | `ValidCell` × `BoxPoint` |  |  | not proven (NaN check) |
| 37 | `Cell::max_distance_to_edge` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell` × `BoxPoint`², the abstract cell |  | `cl_crash::c_fg_max_distance_to_edge` (xlarge); stubs `fixed_vertex`, `any_contains_point`; stub `any_uv_bound` | not proven (tool limit) |
| 38 | `Cell::max_distance_to_cell` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell`², the abstract pair × `Chord` |  | `cl_crash::c_fg_max_distance_to_cell` (xlarge); stub `pair_vertex`; stub `any_uv_bound` | not proven (tool limit) |
| 39 | `Cell::boundary_distance` | not proven | the harness `cl_crash::c_fg_distance_to_point` failed Kani's NaN check after 2874 s weekly (gap 2, 2026-10-27): on box points off the unit sphere `edge_distance` takes the square root of `1 − pq²` with `pq² > 1`, as upstream's formula does; a unit domain needs the squared length, which the checker cannot afford here. A NaN is not a panic in Rust; the harness is removed (`verify/COVERAGE.md`). Tested as before | `ValidCell` × `BoxPoint` |  |  | not proven (NaN check) |
| 40 | `Cell::is_distance_less` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell`², the abstract pair × `Chord` |  | `cl_crash::c_fg_distance_to_cell` (xlarge); stubs `pair_vertex`, `any_contains_point`; stub `any_uv_bound` | not proven (tool limit) |
| 41 | `Cell::is_distance_less_or_equal` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell`², the abstract pair × `Chord` |  | `cl_crash::c_fg_distance_to_cell` (xlarge); stubs `pair_vertex`, `any_contains_point`; stub `any_uv_bound` | not proven (tool limit) |
| 42 | `Cell::is_max_distance_less` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell`², the abstract pair × `Chord` |  | `cl_crash::c_fg_max_distance_to_cell` (xlarge); stub `pair_vertex`; stub `any_uv_bound` | not proven (tool limit) |
| 43 | `Cell::is_max_distance_less_or_equal` | tool-limit | out of memory at 8 GiB in the weekly tier (gap 2, 2026-10-27: the robust predicates' exact arithmetic), so in the xlarge tier, which has not run it; tested as before | `ValidCell`², the abstract pair × `Chord` |  | `cl_crash::c_fg_max_distance_to_cell` (xlarge); stub `pair_vertex`; stub `any_uv_bound` | not proven (tool limit) |
| 44 | `impl PartialEq for Cell` | in scope | | `ValidCell`² | | `cl_crash::c_eq` (24.3 s) | proven |
| 45 | `impl Default for Cell` | in scope | | (no input) | | `cl_crash::c_from_and_default` (36.3 s) | proven |
| 46 | `impl From<CellId> for Cell` | in scope | | `ValidCell` | id is a valid cell (`# Panics`, FR-017) | `cl_crash::c_from_and_default` | proven |
| 47 | `impl Display for Cell` | excluded | tool limit: formats through `core::fmt`, as 001's `Display` (P-ENC-12) | | | | n/a |
| 48 | `impl Region for Cell::cap_bound` | in scope (gap 2) |  | `ValidCell`, the abstract cell's vertices |  | `cl_crash::c_fg_cap_bound` (411.2 s, weekly); stub `fixed_vertex`; stub `any_uv_bound` | proven |
| 49 | `impl Region for Cell::rect_bound` | in scope (gap 2, under contract) |  | `ValidCell` |  | `cl_crash::c_fg_rect_bound` (279.8 s, weekly); `atan2_contract`; stub `any_uv_bound` | proven-under-contract |
| 50 | `impl Region for Cell::cell_union_bound` | in scope | | `ValidCell` | | `cl_crash::c_containment` | proven |
| 51 | `impl Region for Cell::contains_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` | proven |
| 52 | `impl Region for Cell::intersects_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` | proven |
| 53 | `impl Region for Cell::contains_point` | in scope (gap 2) |  | `ValidCell` × `BoxPoint` |  | `cl_crash::c_fg_contains_point` (85.0 s, per change); stub `any_uv_bound` | proven |

## 2. `s1::Interval` (31 entries)

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `s1::Interval::new` | in scope | | `Point`² | both point in [−π, π] (debug-checked; `# Panics`, FR-019) | `iv_crash::c_constructors` (0.9 s) | proven |
| 2 | `s1::Interval::empty` | in scope | | (no input) | | `iv_crash::c_constructors` | proven |
| 3 | `s1::Interval::full` | in scope | | (no input) | | `iv_crash::c_constructors` | proven |
| 4 | `s1::Interval::from_point` | in scope | | `Point` | | `iv_crash::c_constructors` | proven |
| 5 | `s1::Interval::from_point_pair` | in scope | | `Point`² | both point in [−π, π] (debug-checked; `# Panics`, FR-019) | `iv_crash::c_constructors` | proven |
| 6 | `s1::Interval::is_valid` | in scope | | `AnyF64`² (struct literals) | | `iv_crash::c_is_valid` (0.1 s) | proven |
| 7 | `s1::Interval::is_full` | in scope | | `ValidInterval` | | `iv_crash::c_classifiers_and_measures` (0.9 s) | proven |
| 8 | `s1::Interval::is_empty` | in scope | | `ValidInterval` | | `iv_crash::c_classifiers_and_measures` | proven |
| 9 | `s1::Interval::is_inverted` | in scope | | `ValidInterval` | | `iv_crash::c_classifiers_and_measures` | proven |
| 10 | `s1::Interval::center` | in scope (measured first, FR-003) | | `ValidInterval` | | `iv_crash::c_classifiers_and_measures` | proven |
| 11 | `s1::Interval::length` | in scope | | `ValidInterval` | | `iv_crash::c_classifiers_and_measures` | proven |
| 12 | `s1::Interval::complement` | in scope | | `ValidInterval` | | `iv_crash::c_classifiers_and_measures` | proven |
| 13 | `s1::Interval::complement_center` | in scope (measured first, FR-003) | | `ValidInterval` | | `iv_crash::c_classifiers_and_measures` | proven |
| 14 | `s1::Interval::contains` | in scope | | `ValidInterval` × `Point` | point in [−π, π] (debug-checked; `# Panics`, FR-019) | `iv_crash::c_point_operations` (1.6 s) | proven |
| 15 | `s1::Interval::interior_contains` | in scope | | `ValidInterval` × `Point` | point in [−π, π] (debug-checked; `# Panics`, FR-019) | `iv_crash::c_point_operations` | proven |
| 16 | `s1::Interval::contains_interval` | in scope | | `ValidInterval`² | | `iv_crash::c_relations` (0.4 s) | proven |
| 17 | `s1::Interval::interior_contains_interval` | in scope | | `ValidInterval`² | | `iv_crash::c_relations` | proven |
| 18 | `s1::Interval::intersects` | in scope | | `ValidInterval`² | | `iv_crash::c_relations` | proven |
| 19 | `s1::Interval::interior_intersects` | in scope | | `ValidInterval`² | | `iv_crash::c_relations` | proven |
| 20 | `s1::Interval::directed_hausdorff_distance` | in scope (measured first, FR-003) | | `ValidInterval`² | | `iv_crash::c_directed_hausdorff_distance` (24.2 s); D-005 fixed first | proven |
| 21 | `s1::Interval::add_point` | in scope | | `ValidInterval` × `Point` | point in [−π, π] (debug-checked; `# Panics`, FR-019) | `iv_crash::c_point_operations` | proven |
| 22 | `s1::Interval::project` | in scope | | non-empty `ValidInterval` × `Point` | non-empty and point in [−π, π] (debug-checked; `# Panics`, FR-019) | `iv_crash::c_project` (0.3 s) | proven |
| 23 | `s1::Interval::expanded` | in scope (measured first, FR-003) | | `ValidInterval` × `AnyF64` except NaN | margin not NaN (debug-checked through `new`; `# Panics`, K5) | `iv_crash::c_expanded` (70.3 s) | proven |
| 24 | `s1::Interval::union` | in scope | | `ValidInterval`² | | `iv_crash::c_union_intersection` (16.5 s) | proven |
| 25 | `s1::Interval::intersection` | in scope | | `ValidInterval`² | | `iv_crash::c_union_intersection` | proven |
| 26 | `s1::Interval::approx_eq_with` | in scope (measured first, FR-003) | | `ValidInterval`² × `AnyF64` | | `iv_crash::c_approx_eq` (67.3 s) | proven |
| 27 | `s1::Interval::approx_eq` | in scope | | `ValidInterval`² | | `iv_crash::c_approx_eq` | proven |
| 28 | `s1::Interval::bound` | in scope | | `ValidInterval` × `i ≤ 1` | `i ≤ 1` (`# Panics`) | `iv_crash::c_classifiers_and_measures` | proven |
| 29 | `impl Default for s1::Interval` | in scope | | (no input) | | `iv_crash::c_constructors` | proven |
| 30 | `impl PartialEq for s1::Interval` | in scope | | `ValidInterval`² | | `iv_crash::c_classifiers_and_measures` | proven |
| 31 | `impl Display for s1::Interval` | excluded | tool limit: formats through `core::fmt`, as 001's `Display` (P-ENC-12) | | | | n/a |

## 3. `r1::Interval` (27 entries)

**Summary** (spec 005 T020, 2026-10-14): 26 in scope, 1 excluded (`Display`). Every in-scope
row is proven crash-free over its whole documented domain, the infinite corners of FR-017
excepted as the domains state: 9 crash harnesses (`ri_crash`), all covers satisfied, all per
change, 5.2 s of harness time (`make verify-values P=real`, 4 jobs, after the module-name
selection fix). The runner records no peak memory; every harness stayed under the 4 GiB
ceiling. The property harnesses that cover each operation are in `PROPERTIES.md` §4.

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `r1::Interval::new` | in scope | | `Number`² |  | `ri_crash::c_constructors` (0.1 s) | proven |
| 2 | `r1::Interval::empty` | in scope | | (no input) |  | `ri_crash::c_constructors` | proven |
| 3 | `r1::Interval::from_point` | in scope | | `Number` |  | `ri_crash::c_constructors` | proven |
| 4 | `r1::Interval::from_point_pair` | in scope | | `Number`² |  | `ri_crash::c_constructors` | proven |
| 5 | `r1::Interval::is_empty` | in scope | | `Real` |  | `ri_crash::c_classifiers` (0.2 s) | proven |
| 6 | `r1::Interval::center` | in scope | | `Real`, except `[−∞, ∞]` and `[∞, −∞]` | the infinite corner gives NaN (documented, FR-017) | `ri_crash::c_measures` (0.5 s) | proven |
| 7 | `r1::Interval::length` | in scope | | `Real`, except `[∞, ∞]` and `[−∞, −∞]` | the infinite corner gives NaN (documented, FR-017) | `ri_crash::c_measures` | proven |
| 8 | `r1::Interval::contains` | in scope | | `Real` × `Number` |  | `ri_crash::c_point_operations` (0.1 s) | proven |
| 9 | `r1::Interval::interior_contains` | in scope | | `Real` × `Number` |  | `ri_crash::c_point_operations` | proven |
| 10 | `r1::Interval::contains_interval` | in scope | | `Real`² |  | `ri_crash::c_relations_and_set_operations` (0.5 s) | proven |
| 11 | `r1::Interval::interior_contains_interval` | in scope | | `Real`² |  | `ri_crash::c_relations_and_set_operations` | proven |
| 12 | `r1::Interval::intersects` | in scope | | `Real`² |  | `ri_crash::c_relations_and_set_operations` | proven |
| 13 | `r1::Interval::interior_intersects` | in scope | | `Real`² |  | `ri_crash::c_relations_and_set_operations` | proven |
| 14 | `r1::Interval::directed_hausdorff_distance` | in scope | | `Real`², no `∞ − ∞` on its path (FR-017): not the same infinity as both high or both low bounds of two non-empty intervals | documented, FR-017; U-009 | `ri_crash::c_directed_hausdorff_distance` (0.4 s) | proven |
| 15 | `r1::Interval::add_point` | in scope | | `Real` × `Number` |  | `ri_crash::c_point_operations` | proven |
| 16 | `r1::Interval::add_interval` | in scope | | `Real`² |  | `ri_crash::c_relations_and_set_operations` | proven |
| 17 | `r1::Interval::project` | in scope | | non-empty `Real` × `AnyF64` | non-empty (`# Panics` in every build, FR-019; U-008) | `ri_crash::c_project` (0.8 s) | proven |
| 18 | `r1::Interval::expanded` | in scope | | `Real` × `Number`, except an infinite margin against an infinite bound of the opposite sign | the corner gives a NaN bound (documented, FR-017) | `ri_crash::c_expanded` (0.1 s) | proven |
| 19 | `r1::Interval::union` | in scope | | `Real`² |  | `ri_crash::c_relations_and_set_operations` | proven |
| 20 | `r1::Interval::intersection` | in scope | | `Real`² |  | `ri_crash::c_relations_and_set_operations` | proven |
| 21 | `r1::Interval::approx_eq_with` | in scope | | `Real`² × `AnyF64`, no `∞ − ∞` on its path (FR-017) | documented, FR-017 | `ri_crash::c_approx_eq` (2.5 s) | proven |
| 22 | `r1::Interval::approx_eq` | in scope | | `Real`², no `∞ − ∞` on its path (FR-017) | documented, FR-017 | `ri_crash::c_approx_eq` | proven |
| 23 | `r1::Interval::bound` | in scope | | `Real` × both endpoints |  | `ri_crash::c_classifiers` | proven |
| 24 | `impl Default for r1::Interval` | in scope | | (no input) |  | `ri_crash::c_constructors` | proven |
| 25 | `impl PartialEq for r1::Interval` | in scope | | `Real`² |  | `ri_crash::c_classifiers` | proven |
| 26 | `impl From<(f64, f64)> for r1::Interval` | in scope | | `Number`² |  | `ri_crash::c_constructors` | proven |
| 27 | `impl Display for r1::Interval` | excluded | tool limit: the formatter (`core::fmt`) is outside what Kani handles in the budget, as for 001's and 004's `Display` | | | | n/a |

## 4. `s2::Rect` (53 entries)

**Gap 2** (2026-10-27, section 11): of the 18 floating-point rows, 11 are now proven crash-free
(6 per change, 5 weekly or split), 7 are not (three timeouts at 60 minutes, the NaN check in
`get_distance_to_latlng`, and the two `cell_union_bound`s through `Cap`'s). `s2::Rect` has 45
crash-free rows of 52.

**Summary** (spec 005 T020, 2026-10-14): 34 in scope, 19 excluded (18 floating-point geometry,
1 `Display`). Every in-scope row is proven crash-free over its whole documented domain: 12
crash harnesses (`rl_crash`), all covers satisfied, all per change, 183 s of harness time
(`make verify-values P=rect`, 4 jobs), the longest `c_expanded` (87.8 s). The runner records
no peak memory; every harness stayed under the 4 GiB ceiling. The serde implementations
(feature `serde`) are outside the inventory, as for the other types; FR-020's deserialiser is
tested by `core/src/s2/rect_tests.rs`. The property harnesses that cover each operation are
in `PROPERTIES.md` §5.

The binary codec of `s2::Rect` is spec 003's (`decoding/INVENTORY.md`, `Rect` rows).

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `s2::Rect::new` | in scope | | the two intervals of a `ValidRect` |  | `rl_crash::c_constructors` (0.2 s) | proven |
| 2 | `s2::Rect::empty` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 3 | `s2::Rect::full` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 4 | `s2::Rect::from_lat_lng` | in scope | | `ValidLatLng` | a valid longitude (debug-checked; `# Panics`, FR-019); the latitude is unchecked | `rl_crash::c_from_points` (0.9 s) | proven |
| 5 | `s2::Rect::from_center_size` | in scope | | `ValidLatLng` × a size with non-NaN components | a valid centre, a non-NaN longitude size (debug-checked; `# Panics`, FR-019) | `rl_crash::c_from_center_size` (41.8 s) | proven |
| 6 | `s2::Rect::is_valid` | in scope | | `AnyF64`⁴ (struct literals) |  | `rl_crash::c_is_valid` (0.1 s) | proven |
| 7 | `s2::Rect::is_empty` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` (0.8 s) | proven |
| 8 | `s2::Rect::is_full` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 9 | `s2::Rect::is_point` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 10 | `s2::Rect::vertex` | in scope | | `ValidRect` × the four vertices |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 11 | `s2::Rect::lo` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 12 | `s2::Rect::hi` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 13 | `s2::Rect::center` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 14 | `s2::Rect::size` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 15 | `s2::Rect::area` | in scope (gap 2) |  | `ValidRect` |  | `rl_crash::c_fg_area` (1.2 s, per change) | proven |
| 16 | `s2::Rect::contains_lat_lng` | in scope | | `ValidRect` × every `LatLng`, NaN included | none: an invalid point is not contained (K2, FR-018) | `rl_crash::c_screened_point_operations` (2.1 s) | proven |
| 17 | `s2::Rect::contains_point` | in scope (gap 2, under contract) |  | `ValidRect` × `BoxPoint` |  | `rl_crash::c_fg_contains_point` (40.1 s, per change); `atan2_contract` | proven-under-contract |
| 18 | `s2::Rect::contains` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` (23.2 s) | proven |
| 19 | `s2::Rect::intersects` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 20 | `s2::Rect::from_point_pair` | in scope | | `ValidLatLng`² | a valid longitude (debug-checked; `# Panics`, FR-019); the latitude is unchecked | `rl_crash::c_from_points` | proven |
| 21 | `s2::Rect::full_lat` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 22 | `s2::Rect::full_lng` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 23 | `s2::Rect::is_inverted` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 24 | `s2::Rect::interior_contains_lat_lng` | in scope | | `ValidRect` × `ValidLatLng` | a valid longitude (debug-checked; `# Panics`, FR-019) | `rl_crash::c_interior_contains_lat_lng` (1.3 s) | proven |
| 25 | `s2::Rect::interior_contains_point` | in scope (gap 2, under contract) |  | `ValidRect` × `BoxPoint` |  | `rl_crash::c_fg_contains_point` (40.1 s, per change); `atan2_contract` | proven-under-contract |
| 26 | `s2::Rect::interior_contains` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 27 | `s2::Rect::interior_intersects` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 28 | `s2::Rect::add_point` | in scope | | `ValidRect` × every `LatLng`, NaN included | none: an invalid point is ignored (K2, FR-018) | `rl_crash::c_screened_point_operations` | proven |
| 29 | `s2::Rect::polar_closure` | in scope | | `ValidRect` |  | `rl_crash::c_polar_closure` (0.1 s) | proven |
| 30 | `s2::Rect::expanded` | in scope | | `ValidRect` × a margin with non-NaN components | a non-NaN longitude margin (debug-checked; `# Panics`, FR-019) | `rl_crash::c_expanded` (87.8 s) | proven |
| 31 | `s2::Rect::expanded_by_distance` | in scope (gap 2, under contract) |  | `ValidRect` × every finite angle |  | `rl_crash::c_fg_expanded_by_distance__neg` (86.8 s, per change); `rl_crash::c_fg_expanded_by_distance__pos` (737.6 s, weekly); `asin_contract`, `sin_contract`, `cos_contract` (negative); `asin_contract`, `atan2_contract`, `cos_contract` (non-negative) | proven-under-contract |
| 32 | `s2::Rect::union` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 33 | `s2::Rect::intersection` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 34 | `s2::Rect::expand_for_subregions` | in scope (gap 2) |  | `ValidRect` |  | `rl_crash::c_fg_expand_for_subregions` (38.9 s, per change) | proven |
| 35 | `s2::Rect::cap_bound` | in scope (gap 2) |  | `ValidRect` |  | `rl_crash::c_fg_cap_bound` (250.1 s, weekly) | proven |
| 36 | `s2::Rect::rect_bound` | in scope | | `ValidRect` |  | `rl_crash::c_eq_and_bound` (1.2 s) | proven |
| 37 | `s2::Rect::cell_union_bound` | tool-limit | calls `s2::Cap::cell_union_bound` (`cap_bound().cell_union_bound()`), a tool limit: it timed out at 60 minutes weekly (spec 008, `cp_crash::c_cell_union_bound`); covered by the `region_rect_*` tests | `ValidRect` | | | not proven (tool limit) |
| 38 | `s2::Rect::centroid` | in scope (gap 2) |  | `ValidRect` |  | `rl_crash::c_fg_centroid` (12.3 s, per change) | proven |
| 39 | `s2::Rect::boundary_intersects` | tool-limit | timed out at 60 minutes in the weekly tier (gap 2, 2026-10-27; `known-timeouts.txt`); tested as before | `ValidRect` × `BoxPoint`² |  | `rl_crash::c_fg_boundary_intersects` (weekly); `atan2_contract` | not proven (tool limit) |
| 40 | `s2::Rect::get_distance_to_latlng` | not proven | the harness `rl_crash::c_fg_get_distance_to_latlng` failed Kani's NaN check after 3054 s weekly (gap 2, 2026-10-27): `edge_distances::interior_dist` divides by the squared length of the edge's normal, which was 0 on some path (0/0). A NaN is not a panic in Rust; the harness is removed (`verify/COVERAGE.md`). Tested as before | non-empty `ValidRect` × `ValidLatLng` | non-empty, a valid point (debug-checked; `# Panics` added; `rl_crash::n_get_distance_to_latlng`) |  | not proven (NaN check) |
| 41 | `s2::Rect::get_distance` | tool-limit | timed out at 60 minutes in the weekly tier (gap 2, 2026-10-27; `known-timeouts.txt`); tested as before | non-empty `ValidRect`² | both non-empty (debug-checked; `# Panics` added; `rl_crash::n_get_distance`) | `rl_crash::c_fg_get_distance` (weekly); `asin_contract`, `atan2_contract`, `sin_cos_contract` | not proven (tool limit) |
| 42 | `s2::Rect::get_hausdorff_distance` | tool-limit | timed out at 60 minutes in the weekly tier (gap 2, 2026-10-27; `known-timeouts.txt`); tested as before | `ValidRect`² |  | `rl_crash::c_fg_hausdorff` (weekly); `asin_contract`, `atan2_contract`, `sin_cos_contract` | not proven (tool limit) |
| 43 | `s2::Rect::get_directed_hausdorff_distance` | tool-limit | timed out at 60 minutes in the weekly tier (gap 2, 2026-10-27; `known-timeouts.txt`); tested as before | `ValidRect`² |  | `rl_crash::c_fg_hausdorff` (weekly); `asin_contract`, `atan2_contract`, `sin_cos_contract` | not proven (tool limit) |
| 44 | `s2::Rect::approx_eq` | in scope | | `ValidRect`² |  | `rl_crash::c_approx_eq` (23.2 s) | proven |
| 45 | `impl PartialEq for s2::Rect` | in scope | | `ValidRect`² |  | `rl_crash::c_eq_and_bound` | proven |
| 46 | `impl Default for s2::Rect` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 47 | `impl Display for s2::Rect` | excluded | tool limit: the formatter (`core::fmt`) is outside what Kani handles in the budget, as for 001's and 004's `Display` | | | | n/a |
| 48 | `impl Region for s2::Rect::cap_bound` | in scope (gap 2) |  | `ValidRect` |  | `rl_crash::c_fg_cap_bound` (250.1 s, weekly) | proven |
| 49 | `impl Region for s2::Rect::rect_bound` | in scope | | `ValidRect` |  | `rl_crash::c_eq_and_bound` | proven |
| 50 | `impl Region for s2::Rect::cell_union_bound` | tool-limit | calls `s2::Cap::cell_union_bound` (`cap_bound().cell_union_bound()`), a tool limit: it timed out at 60 minutes weekly (spec 008, `cp_crash::c_cell_union_bound`); covered by the `region_rect_*` tests | `ValidRect` | | | not proven (tool limit) |
| 51 | `impl Region for s2::Rect::contains_cell` | in scope (gap 2, under contract) |  | `ValidRect` × `ValidCell` |  | `rl_crash::c_fg_region_cells` (297.7 s, weekly); `atan2_contract`; stub `any_uv_bound` | proven-under-contract |
| 52 | `impl Region for s2::Rect::intersects_cell` | in scope (gap 2, under contract) |  | `ValidRect` × `ValidCell` |  | `rl_crash::c_fg_region_cells` (297.7 s, weekly); `atan2_contract`; stub `any_uv_bound` | proven-under-contract |
| 53 | `impl Region for s2::Rect::contains_point` | in scope (gap 2, under contract) |  | `ValidRect` × `BoxPoint` |  | `rl_crash::c_fg_contains_point` (40.1 s, per change); `atan2_contract` | proven-under-contract |

## 5. `r2::Rect` (33 entries)

**Summary** (T026, 2026-10-14): 32 in scope, 1 excluded (`Display`). Every in-scope row is
proven crash-free over its whole documented domain: 10 crash harnesses (`ur_crash`), all
covers satisfied, all per change, 35 s of harness time. The serde implementations (feature
`serde`) are outside the inventory, as for the other types; FR-019's deserialiser is tested by
`core/src/r2/rect_tests.rs`. The property harnesses that cover each operation are in
`PROPERTIES.md` §6.

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `r2::Rect::new` | in scope | | the two intervals of a `UvRect` | both intervals empty or both not (debug-checked; `# Panics`, FR-017) | `ur_crash::c_constructors` (3.7 s) | proven |
| 2 | `r2::Rect::from_points` | in scope | | `UvPoint`² | corners whose axes are both ordered or both reversed (debug-checked; `# Panics`, FR-017) | `ur_crash::c_constructors` | proven |
| 3 | `r2::Rect::empty` | in scope | | (no input) |  | `ur_crash::c_constructors` | proven |
| 4 | `r2::Rect::from_point` | in scope | | `UvPoint` |  | `ur_crash::c_constructors` | proven |
| 5 | `r2::Rect::from_point_pair` | in scope | | `UvPoint`² |  | `ur_crash::c_constructors` | proven |
| 6 | `r2::Rect::from_center_size` | in scope | | `UvPoint` × a finite size `≥ 0` | a non-negative, non-NaN size (debug-checked; `# Panics`, FR-017) | `ur_crash::c_from_center_size` (20.8 s) | proven |
| 7 | `r2::Rect::lo` | in scope | | `UvRect` |  | `ur_crash::c_accessors` (0.4 s) | proven |
| 8 | `r2::Rect::hi` | in scope | | `UvRect` |  | `ur_crash::c_accessors` | proven |
| 9 | `r2::Rect::is_valid` | in scope | | `AnyF64`⁴ (struct literals) |  | `ur_crash::c_is_valid` (0.1 s) | proven |
| 10 | `r2::Rect::is_empty` | in scope | | `UvRect` |  | `ur_crash::c_accessors` | proven |
| 11 | `r2::Rect::vertex` | in scope | | `UvRect` × `i32` |  | `ur_crash::c_accessors` | proven |
| 12 | `r2::Rect::vertex_ij` | in scope | | `UvRect` × `Endpoint`² |  | `ur_crash::c_accessors` | proven |
| 13 | `r2::Rect::vertices` | in scope | | `UvRect` |  | `ur_crash::c_accessors` | proven |
| 14 | `r2::Rect::center` | in scope | | `FiniteUvRect` | finite bounds (FR-004; the infinite corners are documented, FR-018) | `ur_crash::c_measures` (1.1 s) | proven |
| 15 | `r2::Rect::size` | in scope | | `FiniteUvRect` | finite bounds (FR-004, FR-018) | `ur_crash::c_measures` | proven |
| 16 | `r2::Rect::contains_point` | in scope | | `UvRect` × `UvPoint` |  | `ur_crash::c_point_operations` (0.5 s) | proven |
| 17 | `r2::Rect::interior_contains_point` | in scope | | `UvRect` × `UvPoint` |  | `ur_crash::c_point_operations` | proven |
| 18 | `r2::Rect::contains` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` (2.0 s) | proven |
| 19 | `r2::Rect::interior_contains` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` | proven |
| 20 | `r2::Rect::intersects` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` | proven |
| 21 | `r2::Rect::interior_intersects` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` | proven |
| 22 | `r2::Rect::add_point` | in scope | | `UvRect` × `UvPoint` |  | `ur_crash::c_point_operations` | proven |
| 23 | `r2::Rect::add_rect` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` | proven |
| 24 | `r2::Rect::project` | in scope | | non-empty `UvRect` × `UvPoint` | a non-empty rectangle without NaN bounds (`# Panics` in every build, FR-017; U-008) | `ur_crash::c_project` (1.0 s) | proven |
| 25 | `r2::Rect::expanded` | in scope | | `FiniteUvRect` × finite margins | finite bounds and margins (FR-004, FR-018) | `ur_crash::c_expanded` (3.1 s) | proven |
| 26 | `r2::Rect::expanded_by_margin` | in scope | | `FiniteUvRect` × a finite margin | as `expanded` | `ur_crash::c_expanded` | proven |
| 27 | `r2::Rect::union` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` | proven |
| 28 | `r2::Rect::intersection` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` | proven |
| 29 | `r2::Rect::approx_eq_with` | in scope | | `FiniteUvRect`² × a finite error `≥ 0` | finite bounds (FR-018: an infinite bound compares through `∞ − ∞`) | `ur_crash::c_approx_eq` (2.5 s) | proven |
| 30 | `r2::Rect::approx_eq` | in scope | | `FiniteUvRect`² | as `approx_eq_with` | `ur_crash::c_approx_eq` | proven |
| 31 | `impl Default for r2::Rect` | in scope | | (no input) |  | `ur_crash::c_constructors` | proven |
| 32 | `impl PartialEq for r2::Rect` | in scope | | `UvRect`² |  | `ur_crash::c_relations_and_set_operations` | proven |
| 33 | `impl Display for r2::Rect` | excluded | tool limit: formats through `core::fmt`, as 001's and 004's `Display` | | | | n/a |

## 6. `s2::Point` (31 entries)

**Gap 2** (2026-10-27, section 11): the 5 rows through `atan2` are proven crash-free under its
contract (`pt_crash::c_fg_angles`, weekly), so 30 of 31 (`Display` excluded).

**Summary** (T026, 2026-10-14): 24 in scope and `rotate` (25 rows with crash proofs), 5 tool
limits through `atan2`, 1 excluded (`Display`). 23 rows are proven crash-free over their whole
documented domain; `from_coords` and `normalize` are proven within bounds, over every triple
whose coordinates are at most `2⁵¹¹` (the band up to `2⁵¹²` timed out at 60 minutes; beyond it
the checker's spurious `1 / ∞` NaN makes the rest a tool limit). 11 crash harnesses
(`pt_crash`): 8 per change and 3 weekly (`c_ortho`, `c_rotate`, and `__band`, which timed out),
all covers satisfied where they finished, 905 s of harness time for the ten that finished
(the per-change ones 298 s). The serde implementations are outside the
inventory. The property harnesses that cover each operation are in `PROPERTIES.md` §7.

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `s2::Point::new` | in scope | | `AnyTriple` |  | `pt_crash::c_construction_and_accessors` (0.1 s) | proven |
| 2 | `s2::Point::from_coords` | in scope | | `BoundedTriple` (every coordinate at most `2⁵¹¹`); the band `(2⁵¹¹, 2⁵¹²)` of `NormableTriple` timed out at 60 minutes (weekly `__band`) | a finite squared length: the checker reports the `1 / ∞` of an overflowing one as NaN, so those triples are a tool limit, tested by `point_tests.rs` and the C++ vectors (FR-022, K7) | `pt_crash::c_from_coords_normalize__bounded` (61.0 s) | proven-within-bounds |
| 3 | `s2::Point::origin` | in scope | | (no input) |  | `pt_crash::c_construction_and_accessors` | proven |
| 4 | `s2::Point::vector` | in scope | | `AnyTriple` |  | `pt_crash::c_construction_and_accessors` | proven |
| 5 | `s2::Point::x` | in scope | | `AnyTriple` |  | `pt_crash::c_construction_and_accessors` | proven |
| 6 | `s2::Point::y` | in scope | | `AnyTriple` |  | `pt_crash::c_construction_and_accessors` | proven |
| 7 | `s2::Point::z` | in scope | | `AnyTriple` |  | `pt_crash::c_construction_and_accessors` | proven |
| 8 | `s2::Point::is_unit` | in scope | | `AnyTriple` |  | `pt_crash::c_classifications` (2.9 s) | proven |
| 9 | `s2::Point::normalize` | in scope | | as `from_coords` | as `from_coords` | `pt_crash::c_from_coords_normalize__bounded` | proven-within-bounds |
| 10 | `s2::Point::chord_angle` | in scope | | `BoxPoint`² | unit points (FR-022); the box `[−1, 1]³` keeps every intermediate finite | `pt_crash::c_chord_angle` (2.8 s) | proven |
| 11 | `s2::Point::point_cross` | in scope | | `BoxPoint`² | points in the box (FR-022) | `pt_crash::c_point_cross` (224.3 s) | proven |
| 12 | `s2::Point::reference_dir` | in scope | | `BoxPoint` | a point in the box (FR-022) | `pt_crash::c_ortho` (260.0 s, weekly) | proven |
| 13 | `s2::Point::cmp_point` | in scope | | `AnyTriple`² |  | `pt_crash::c_order` (0.3 s) | proven |
| 14 | `s2::Point::is_normalizable` | in scope | | `AnyTriple` |  | `pt_crash::c_classifications` | proven |
| 15 | `s2::Point::ensure_normalizable` | in scope | | `AnyTriple` |  | `pt_crash::c_ensure_normalizable` (6.1 s) | proven |
| 16 | `s2::ortho` | in scope | | `BoxPoint` | a point in the box (FR-022) | `pt_crash::c_ortho` | proven |
| 17 | `impl Neg for s2::Point` | in scope | | `AnyTriple` |  | `pt_crash::c_classifications` | proven |
| 18 | `impl Add for s2::Point` | in scope | | `PlainTriple`², except opposite infinities in one coordinate | no `∞ − ∞` (FR-004) | `pt_crash::c_add_sub` (1.0 s) | proven |
| 19 | `impl Sub for s2::Point` | in scope | | `PlainTriple`², except the same infinity in one coordinate | no `∞ − ∞` (FR-004) | `pt_crash::c_add_sub` | proven |
| 20 | `impl From<r3::Vector> for s2::Point` | in scope | | `AnyTriple` |  | `pt_crash::c_construction_and_accessors` | proven |
| 21 | `impl From<s2::Point> for r3::Vector` | in scope | | `AnyTriple` |  | `pt_crash::c_construction_and_accessors` | proven |
| 22 | `impl PartialOrd for s2::Point` | in scope | | `AnyTriple`² |  | `pt_crash::c_order` | proven |
| 23 | `impl PartialEq for s2::Point` | in scope | | `AnyTriple`² |  | `pt_crash::c_order` | proven |
| 24 | `impl Default for s2::Point` | in scope | | (no input) |  | `pt_crash::c_construction_and_accessors` | proven |
| 25 | `s2::rotate` | in scope (trigonometry: `sin`, `cos` modelled) | | `BoxPoint`² × `FiniteAngle` | unit points (`ABSL_DCHECK`, `s2pointutil.cc:63–64`); `sin` and `cos` are CBMC's models, in [−1, 1] | `pt_crash::c_rotate` (346.0 s, weekly) | proven |
| 26 | `s2::Point::distance` | in scope (gap 2, under contract) |  | `BoxPoint`² |  | `pt_crash::c_fg_angles` (459.6 s, weekly); `atan2_contract` | proven-under-contract |
| 27 | `s2::Point::stable_angle` | in scope (gap 2, under contract) |  | `BoxPoint`² |  | `pt_crash::c_fg_angles` (459.6 s, weekly); `atan2_contract` | proven-under-contract |
| 28 | `s2::Point::approx_eq` | in scope (gap 2, under contract) |  | `BoxPoint`² |  | `pt_crash::c_fg_angles` (459.6 s, weekly); `atan2_contract` | proven-under-contract |
| 29 | `s2::Point::approx_eq_angle` | in scope (gap 2, under contract) |  | `BoxPoint`² × `AnyAngle` |  | `pt_crash::c_fg_angles` (459.6 s, weekly); `atan2_contract` | proven-under-contract |
| 30 | `s2::Point::approx_eq_with` | in scope (gap 2, under contract) |  | `BoxPoint`² × `AnyAngle` |  | `pt_crash::c_fg_angles` (459.6 s, weekly); `atan2_contract` | proven-under-contract |
| 31 | `impl Display for s2::Point` | excluded | tool limit: formats through `core::fmt`, as 001's and 004's `Display` | | | | n/a |

## 7. `s1::ChordAngle` (36 entries)

**Gap 2** (2026-10-27, section 11): the 7 rows through `asin` are proven crash-free under its
contract (`ca_crash::c_fg_conversions`, per change), so all 35.

**Summary** (T031, 2026-10-17): 28 in scope with crash proofs, 7 tool limits through `asin`
(`to_angle` and the operations that read it), 1 excluded (`Display`). Every in-scope row is
proven crash-free over its whole documented domain by 9 crash harnesses (`ca_crash`), all
covers satisfied, 39.8 s of harness time (`make verify-values P=chord`, 5 jobs). Since option
B (FR-017) every chord angle is canonical, so `Chord` (−1, every length in [0, 4], +∞) is every
value the type can hold. The arithmetic and trigonometry take non-special operands, their
documented precondition (FR-019): their debug checks are proven never to fire there. The serde
implementations are outside the inventory. The property harnesses that cover each operation
are in `PROPERTIES.md` §8.

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `s1::ChordAngle::from_length2` | in scope |  | `AnyF64` |  | `ca_crash::c_from_numbers` (1.0 s) | proven |
| 2 | `s1::ChordAngle::from_angle` | in scope |  | `AnyAngle` |  | `ca_crash::c_from_angle` (0.5 s) | proven |
| 3 | `s1::ChordAngle::from_radians` | in scope |  | `AnyF64` |  | `ca_crash::c_from_numbers` | proven |
| 4 | `s1::ChordAngle::from_degrees` | in scope |  | `AnyF64` |  | `ca_crash::c_from_numbers` | proven |
| 5 | `s1::ChordAngle::from_e5` | in scope |  | every `i32` |  | `ca_crash::c_from_e_forms` (1.1 s) | proven |
| 6 | `s1::ChordAngle::from_e6` | in scope |  | every `i32` |  | `ca_crash::c_from_e_forms` | proven |
| 7 | `s1::ChordAngle::from_e7` | in scope |  | every `i32` |  | `ca_crash::c_from_e_forms` | proven |
| 8 | `s1::ChordAngle::length2` | in scope |  | `Chord` |  | `ca_crash::c_accessors_classes_and_order` (0.5 s) | proven |
| 9 | `s1::ChordAngle::to_angle` | in scope (gap 2, under contract) |  | `Chord` |  | `ca_crash::c_fg_conversions` (89.9 s, per change); `asin_contract` | proven-under-contract |
| 10 | `s1::ChordAngle::radians` | in scope (gap 2, under contract) |  | `Chord` |  | `ca_crash::c_fg_conversions` (89.9 s, per change); `asin_contract` | proven-under-contract |
| 11 | `s1::ChordAngle::degrees` | in scope (gap 2, under contract) |  | `Chord` |  | `ca_crash::c_fg_conversions` (89.9 s, per change); `asin_contract` | proven-under-contract |
| 12 | `s1::ChordAngle::e5` | in scope (gap 2, under contract) |  | `Chord` |  | `ca_crash::c_fg_conversions` (89.9 s, per change); `asin_contract` | proven-under-contract |
| 13 | `s1::ChordAngle::e6` | in scope (gap 2, under contract) |  | `Chord` |  | `ca_crash::c_fg_conversions` (89.9 s, per change); `asin_contract` | proven-under-contract |
| 14 | `s1::ChordAngle::e7` | in scope (gap 2, under contract) |  | `Chord` |  | `ca_crash::c_fg_conversions` (89.9 s, per change); `asin_contract` | proven-under-contract |
| 15 | `s1::ChordAngle::is_zero` | in scope |  | `Chord` |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 16 | `s1::ChordAngle::is_negative` | in scope |  | `Chord` |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 17 | `s1::ChordAngle::is_infinity` | in scope |  | `Chord` |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 18 | `s1::ChordAngle::is_special` | in scope |  | `Chord` |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 19 | `s1::ChordAngle::is_valid` | in scope |  | `Chord` |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 20 | `s1::ChordAngle::successor` | in scope |  | `Chord` |  | `ca_crash::c_neighbours` (0.2 s) | proven |
| 21 | `s1::ChordAngle::predecessor` | in scope |  | `Chord` |  | `ca_crash::c_neighbours` | proven |
| 22 | `s1::ChordAngle::plus_error` | in scope |  | `Chord` × `AnyF64` |  | `ca_crash::c_plus_error` (0.3 s) | proven |
| 23 | `s1::ChordAngle::max_point_error` | in scope |  | `ChordLength` |  | `ca_crash::c_error_bounds` (0.2 s) | proven |
| 24 | `s1::ChordAngle::max_angle_error` | in scope |  | `ChordLength` |  | `ca_crash::c_error_bounds` | proven |
| 25 | `s1::ChordAngle::sin` | in scope |  | `ChordLength` | non-special (FR-019; upstream's `ABSL_DCHECK` in `sin2`) | `ca_crash::c_trigonometry` (16.3 s) | proven |
| 26 | `s1::ChordAngle::sin2` | in scope |  | `ChordLength` | non-special (FR-019; `s1chord_angle.cc:132`) | `ca_crash::c_trigonometry` | proven |
| 27 | `s1::ChordAngle::cos` | in scope |  | `ChordLength` | non-special (FR-019; `s1chord_angle.cc:147`) | `ca_crash::c_trigonometry` | proven |
| 28 | `s1::ChordAngle::tan` | in scope |  | `ChordLength` | non-special (FR-019, through `sin` and `cos`) | `ca_crash::c_trigonometry` | proven |
| 29 | `impl Default for s1::ChordAngle` | in scope |  | (no input) |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 30 | `impl From<s1::Angle> for s1::ChordAngle` | in scope |  | `AnyAngle` |  | `ca_crash::c_from_angle` | proven |
| 31 | `impl From<s1::ChordAngle> for s1::Angle` | in scope (gap 2, under contract) |  | `Chord` |  | `ca_crash::c_fg_conversions` (89.9 s, per change); `asin_contract` | proven-under-contract |
| 32 | `impl Add for s1::ChordAngle` | in scope |  | `ChordLength`² | neither operand special (FR-019; `s1chord_angle.cc:92–93`) | `ca_crash::c_add_sub` (19.7 s) | proven |
| 33 | `impl Sub for s1::ChordAngle` | in scope |  | `ChordLength`² | neither operand special (FR-019; `s1chord_angle.cc:116–117`) | `ca_crash::c_add_sub` | proven |
| 34 | `impl Display for s1::ChordAngle` | excluded | tool limit: formats through `core::fmt` and `to_angle` (`asin`), as 001's and 004's `Display` |  |  |  | n/a |
| 35 | `PartialEq (derived)` | in scope |  | `Chord`² |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 36 | `PartialOrd (derived)` | in scope |  | `Chord`² |  | `ca_crash::c_accessors_classes_and_order` | proven |

## 8. `s1::Angle` (31 entries)

The 20 inherent operations count `approx_eq`; the two constants (`ZERO`, `INFINITY`) are read by
the crash harnesses but are not entries. The derived `PartialEq` and `PartialOrd` share a row,
so the table has 30 rows for 31 entries.

**Gap 2** (2026-10-27, section 11): `tan` is proven crash-free under a new contract
(`ag_crash::c_fg_tan`, per change), so all 30.

**Summary** (T031, 2026-10-17): 30 in scope, 29 of them with crash proofs and 1 a tool limit (`tan`, a C library
call), 1 excluded (`Display`). Every in-scope row is proven crash-free by 9 crash harnesses
(`ag_crash`), all covers satisfied, 2.9 s of harness time (`make verify-values P=angle`, 5
jobs). An angle has no validity rule, so the domains are every `f64` of radians, every `i32`
and every `u32`, narrowed only where IEEE-754 makes a new NaN, and for the quotients by an
infinity, a tool limit. The property harnesses and the enumeration tests that cover each
operation are in `PROPERTIES.md` §9.

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `s1::Angle::from_radians` | in scope |  | `AnyF64` |  | `ag_crash::c_construction_and_accessors` (0.1 s) | proven |
| 2 | `s1::Angle::from_degrees` | in scope |  | `AnyF64` |  | `ag_crash::c_construction_and_accessors` | proven |
| 3 | `s1::Angle::from_e5` | in scope |  | every `i32` |  | `ag_crash::c_from_e_forms` (0.3 s) | proven |
| 4 | `s1::Angle::from_e6` | in scope |  | every `i32` |  | `ag_crash::c_from_e_forms` | proven |
| 5 | `s1::Angle::from_e7` | in scope |  | every `i32` |  | `ag_crash::c_from_e_forms` | proven |
| 6 | `s1::Angle::from_unsigned_e6` | in scope |  | every `u32` |  | `ag_crash::c_from_e_forms` | proven |
| 7 | `s1::Angle::from_unsigned_e7` | in scope |  | every `u32` |  | `ag_crash::c_from_e_forms` | proven |
| 8 | `s1::Angle::radians` | in scope |  | `AnyAngle` |  | `ag_crash::c_construction_and_accessors` | proven |
| 9 | `s1::Angle::degrees` | in scope |  | `AnyAngle` |  | `ag_crash::c_construction_and_accessors` | proven |
| 10 | `s1::Angle::e5` | in scope |  | `AnyAngle` (NaN and ±∞ included; saturating, FR-023) | upstream's (−180°, 180°] is not required: every angle is computed (FR-023) | `ag_crash::c_e_forms` (1.2 s) | proven |
| 11 | `s1::Angle::e6` | in scope |  | `AnyAngle` | as `e5` | `ag_crash::c_e_forms` | proven |
| 12 | `s1::Angle::e7` | in scope |  | `AnyAngle` | as `e5` | `ag_crash::c_e_forms` | proven |
| 13 | `s1::Angle::abs` | in scope |  | `AnyAngle` |  | `ag_crash::c_construction_and_accessors` | proven |
| 14 | `s1::Angle::normalized` | in scope |  | every angle but ±∞ (whose remainder is NaN, documented) |  | `ag_crash::c_normalized` (0.3 s); the proof does not depend on the value of `%`, which Kani models wrongly (research R0): nothing after it can panic. Its value is P-AGNRM-01's tests | proven |
| 15 | `s1::Angle::sin` | in scope |  | `AnyAngle` |  | `ag_crash::c_trigonometry` (0.1 s) | proven |
| 16 | `s1::Angle::cos` | in scope |  | `AnyAngle` |  | `ag_crash::c_trigonometry` | proven |
| 17 | `s1::Angle::tan` | in scope (gap 2, under contract) |  | `AnyAngle` |  | `ag_crash::c_fg_tan` (0.1 s, per change); `tan_contract` | proven-under-contract |
| 18 | `s1::Angle::sin_cos` | in scope |  | `AnyAngle` |  | `ag_crash::c_trigonometry` | proven |
| 19 | `s1::Angle::is_infinite` | in scope |  | `AnyAngle` |  | `ag_crash::c_construction_and_accessors` | proven |
| 20 | `s1::Angle::approx_eq` | in scope |  | `AnyAngle`² |  | `ag_crash::c_order` (0.1 s) | proven |
| 21 | `impl Neg for s1::Angle` | in scope |  | `AnyAngle` |  | `ag_crash::c_construction_and_accessors` | proven |
| 22 | `impl Add for s1::Angle` | in scope |  | `AnyAngle`² without opposite infinities | IEEE-754: `∞ − ∞` is NaN | `ag_crash::c_add_sub` (0.1 s) | proven |
| 23 | `impl Sub for s1::Angle` | in scope |  | `AnyAngle`² without equal infinities | IEEE-754, as `Add` | `ag_crash::c_add_sub` | proven |
| 24 | `impl Mul<f64> for s1::Angle` | in scope |  | `AnyAngle` × `AnyF64` without `0 · ∞` | IEEE-754 | `ag_crash::c_mul` (0.3 s) | proven |
| 25 | `impl Mul<s1::Angle> for f64` | in scope |  | as `Mul<f64>` | IEEE-754 | `ag_crash::c_mul` | proven |
| 26 | `impl Div<f64> for s1::Angle` | in scope |  | `AnyAngle` × a finite divisor, without `0 / 0` | an infinite divisor is a tool limit (the checker reports `x / ±∞` as NaN; IEEE-754 gives ±0), checked by the C++ vectors | `ag_crash::c_div` (0.4 s) | proven |
| 27 | `impl Div<s1::Angle> for s1::Angle` | in scope |  | as `Div<f64>` | as `Div<f64>` | `ag_crash::c_div` | proven |
| 28 | `impl Display for s1::Angle` | excluded | tool limit: formats through `core::fmt`, as 001's and 004's `Display` |  |  |  | n/a |
| 29 | `PartialEq, PartialOrd (derived)` | in scope |  | `AnyAngle`², NaN included |  | `ag_crash::c_order` | proven |
| 30 | `Default (derived)` | in scope |  | (no input) |  | `ag_crash::c_construction_and_accessors` | proven |

## 9. `s2::Cap` (43 entries)

Spec 008 (`specs/008-formal-verify-cap/data-model.md` §5). The 32 inherent operations; the
hand-written `PartialEq` and `Display`; the six `Region` methods; `S2Encode` and `S2Decode`
(proven by spec 003, `verify/decoding/`); the hand-written `Deserialize` (FR-023). The derived
`Clone`, `Copy` and `Debug` are not entries.

Statuses as section 8's, plus **under contract** (the proof replaces `asin`, `atan2` or `cos`
by its range contract, spec 008 FR-003) and **by composition** (lemma stubs, FR-003a); the
stubs a harness uses are named in its row.

**Summary** (T027, 2026-10-24): 42 in scope, 1 excluded (`Display`). The 32 rows group the 43
entries (several share a row). 31 entries have crash proofs on the real code or with lemma stubs,
8 under the function contracts (5 per change, 3 weekly), and the two codec entries are spec
003's; `Deserialize` is tested. The per-change crash harnesses: 14 in `cp_crash`, every one
`SUCCESSFUL`, all covers satisfied (`make verify-values P=cap`; seven gained their covers at
T052, 2026-10-25). The weekly members, one at a
time at 60 minutes and 8 GiB (2026-10-25): `c_cell_tests__edges_contains` (704.2 s) and
`c_cell_tests__edges_intersects` (402.0 s) prove the edge test over the abstract cell;
`c_cell_tests__real` and `c_cell_union_bound` time out (`verify/known-timeouts.txt`); `c_union` runs
out of memory, and alone at 16 GiB times out. So 3 of the 8 contract entries (`union`, both `cell_union_bound`s) are tool limits.

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `s2::Cap::from_center_chord_angle` | in scope |  | `BoxPoint` × every `f64` (through `from_length2`) |  | `cp_crash::c_construction_and_measures` (9.0 s) | proven |
| 2 | `s2::Cap::from_center_angle` | in scope |  | a unit centre × every non-NaN angle | a unit centre (upstream's debug check, `# Panics`) | `cp_crash::c_from_center_angle` (6.7 s; stub `pure_is_unit`) | proven-by-composition |
| 3 | `s2::Cap::from_point` | in scope |  | `BoxPoint` |  | `cp_crash::c_construction_and_measures` | proven |
| 4 | `s2::Cap::from_center_height` | in scope |  | `BoxPoint` × every `f64` |  | `cp_crash::c_construction_and_measures` | proven |
| 5 | `s2::Cap::from_center_area` | in scope |  | `BoxPoint` × every `f64` |  | `cp_crash::c_construction_and_measures` | proven |
| 6 | `s2::Cap::empty`, `full` | in scope |  | — |  | `cp_crash::c_construction_and_measures` | proven |
| 7 | `s2::Cap::center`, `chord_radius` | in scope |  | `BoxCap` |  | `cp_crash::c_construction_and_measures` | proven |
| 8 | `s2::Cap::angle_radius` | in scope (under contract) |  | `BoxCap` |  | `cp_crash::c_angle_radius` (1.1 s; `asin_contract`) | proven-under-contract |
| 9 | `s2::Cap::height`, `area`, `centroid` | in scope |  | `BoxCap` |  | `cp_crash::c_construction_and_measures` | proven |
| 10 | `s2::Cap::is_valid`, `is_empty`, `is_full` | in scope |  | `BoxCap` |  | `cp_crash::c_classifications_and_equality` (1.8 s) | proven |
| 11 | `s2::Cap::contains_point`, `interior_contains_point` | in scope |  | `BoxCap` × `BoxPoint` | a unit point (upstream's debug check; documented) | `cp_crash::c_point_containment` (5.2 s) | proven |
| 12 | `s2::Cap::contains` | in scope |  | `BoxCap²` |  | `cp_crash::c_contains` (6.2 s) | proven |
| 13 | `s2::Cap::intersects`, `interior_intersects` | in scope |  | `BoxCap²` |  | `cp_crash::c_intersects` (36.7 s) | proven |
| 14 | `s2::Cap::complement` | in scope |  | `BoxCap` |  | `cp_crash::c_complement` (0.8 s) | proven |
| 15 | `s2::Cap::expanded` | in scope |  | `BoxCap` × `FiniteNonNegAngle` | a finite non-negative distance (upstream's debug check; `# Panics`) | `cp_crash::c_expanded` (10.7 s) | proven |
| 16 | `s2::Cap::union` | in scope (under contract) |  | `ValidCap²` |  | weekly: `cp_crash::c_union` (`asin_contract`, `atan2_contract`, `pure_is_unit`; out of memory at 8 GiB, timeout at 60 minutes at 16 GiB, 2026-10-25) | not proven (tool limit) |
| 17 | `s2::Cap::add_point` | in scope |  | `BoxCap` × `BoxPoint` |  | `cp_crash::c_add_point` (1.4 s) | proven |
| 18 | `s2::Cap::add_cap` | in scope |  | `BoxCap²` |  | `cp_crash::c_add_cap` (15.1 s) | proven |
| 19 | `s2::Cap::cap_bound` | in scope |  | `BoxCap` |  | `cp_crash::c_classifications_and_equality` | proven |
| 20 | `s2::Cap::rect_bound` | in scope (under contract) |  | `BoxCap` |  | `cp_crash::c_rect_bound` (180.2 s; `asin_contract`, `atan2_contract`, `cos_contract`) | proven-under-contract |
| 21 | `s2::Cap::cell_union_bound` | in scope (under contract) |  | `ValidCap` |  | weekly: `cp_crash::c_cell_union_bound` (`asin_contract`, `pure_is_unit`; timeout at 60 minutes, 2026-10-25) | not proven (tool limit) |
| 22 | `s2::Cap::equal`, `PartialEq` | in scope |  | `BoxCap²` |  | `cp_crash::c_classifications_and_equality` | proven |
| 23 | `s2::Cap::approx_eq`, `approx_eq_with` | in scope (under contract) |  | `BoxCap²` × every `f64` |  | `cp_crash::c_approx_eq` (31.5 s; `atan2_contract`) | proven-under-contract |
| 24 | `Region for Cap::cap_bound` | in scope |  | `BoxCap` |  | `cp_crash::c_classifications_and_equality` | proven |
| 25 | `Region for Cap::rect_bound` | in scope (under contract) |  | `BoxCap` |  | `cp_crash::c_rect_bound` | proven-under-contract |
| 26 | `Region for Cap::cell_union_bound` | in scope (under contract) |  | `ValidCap` |  | weekly: `cp_crash::c_cell_union_bound` (timeout at 60 minutes) | not proven (tool limit) |
| 27 | `Region for Cap::contains_cell` | in scope |  | `BoxCap` × the abstract cell |  | `cp_crash::c_cell_tests__vertices` (170.6 s; the edge test abstracted); weekly: `cp_crash::c_cell_tests__edges_contains` (704.2 s; the abstract cell's edges), `cp_crash::c_cell_tests__real` (timeout at 60 minutes) | proven-by-composition (vertex part and edge test, over the abstract cell) |
| 28 | `Region for Cap::intersects_cell` | in scope |  | `BoxCap` × the abstract cell |  | `cp_crash::c_cell_tests__vertices`; weekly: `cp_crash::c_cell_tests__edges_intersects` (402.0 s), `cp_crash::c_cell_tests__real` (timeout at 60 minutes) | proven-by-composition (vertex part and edge test, over the abstract cell) |
| 29 | `Region for Cap::contains_point` | in scope |  | `BoxCap` × `BoxPoint` |  | `cp_crash::c_point_containment` | proven |
| 30 | `S2Encode for Cap`, `S2Decode for Cap` | in scope (spec 003) |  | every valid cap; every byte string |  | `cd_fx::p_fx_03__*`, `cd_fx::p_fx_08__*` (spec 003) | proven (spec 003) |
| 31 | `Deserialize for Cap` | in scope (tested) |  | every serialised cap |  | `cap_tests::deserialise_rejects_invalid_caps` (not a Kani entry) | tested |
| 32 | `Display for Cap` | excluded | `core::fmt` is too slow for Kani, and `asin` (as 004–007) |  |  |  | — |

## 10. Outside the domain: the documented panics (2026-10-26)

The other half of each `# Panics` section, as in 001's `INVENTORY.md` section 3: for every input
outside the domain that the section names, the operation panics (`n_*`, `#[kani::should_panic]`
with one cover after the call that must stay unsatisfied), and where the panic is debug-only,
the release build completes on the same inputs without a panic or an arithmetic overflow (`r_*`,
`make verify-release`, which also drops Kani's NaN check: a release build's skipped check often
leaves a NaN, which Rust computes without a panic). Where the section names inputs outside the
domain that return, an `o_*` harness proves that they do, in every build. The `n_*` and `o_*`
harnesses run in each part's per-change leg; the every-build `n_*`, the `r_*` and the `o_*` run
in the release pass.

| Type | Operations | Outside the domain | Build | Harness | Result |
|------|------------|--------------------|-------|---------|--------|
| `Cell` | `from_cell_id`, `From<CellId>` (and, before spec 009, `from_face` and `from_face_pos_level`) | an id with no level marker (62 or more trailing zero bits) or on face 6 or 7. Before spec 009 also faces 6 and 7, and a raw level above 30: `from_face` and `from_face_pos_level` take a `Face` and a `Level` now, so neither can be handed one | every | `cl_crash::n_from_cell_id` | panics (both builds). The `# Panics` named the null id and faces 6 and 7, and called every other invalid id unspecified: `1 << 62`, `1 << 63` and `3 << 62` panic too. Corrected |
| `Cell` | `from_cell_id` | the other invalid ids: an odd lowest set bit up to 61, faces 0–5 | every | `cl_crash::o_from_cell_id_unspecified` | returns (unspecified fields) |
| `Cell` | `from_point`, and `CellId::from_point` | the zero vector, any signs | debug | `cl_crash::n_from_point`; release `r_from_point` | panics in debug builds (no face contains it); returns a leaf in release builds. Undocumented until gap 2 (section 11): `# Panics` added, which also says that some points with a NaN coordinate panic |
| `Cell` | `vertex`, `vertex_raw` | an index above 3 (until spec 009; they take a `CellVertex` now, so none can be passed) | every | `cl_crash::c_cell_vertex_try_from`: `CellVertex::try_from` is exact; before spec 009, `cl_crash::n_vertex` (72 s, retired) | the panic is gone: an index above 3 is rejected where it is converted, with `OutOfRange`. Undocumented until gap 2, and then documented as `# Panics`, now removed |
| `s1::Interval` | `new`, `from_point_pair` | an endpoint outside [−π, π] or NaN | debug | `iv_crash::n_constructors`; release `r_constructors` | panics in debug builds; returns in release builds |
| `s1::Interval` | `contains`, `interior_contains`, `add_point` | every valid interval, a point outside [−π, π] or NaN | debug | `iv_crash::n_point_operations`; release `r_point_operations` | panics in debug builds; returns in release builds |
| `s1::Interval` | `project` | an empty interval, or a point outside [−π, π] or NaN | debug | `iv_crash::n_project`; release `r_project` | panics in debug builds; returns in release builds |
| `s1::Interval` | `expanded` | a NaN margin, every valid interval but the full one | debug | `iv_crash::n_expanded`; release `r_expanded` | panics in debug builds; returns in release builds. Over every valid interval the first version returned: the full interval, which a NaN margin returns as it is, as upstream (`iv_crash::o_expanded_nan_full`). Corrected |
| `s1::Interval` | `bound` | an index above 1 | every | `iv_crash::n_bound` | panics (both builds) |
| `r1::Interval` | `project` | an empty interval or a NaN bound, any point | every | `ri_crash::n_project` | panics (both builds: `f64::clamp`'s assertion) |
| `s2::Rect` | `from_lat_lng`, `from_point_pair` | a longitude outside [−π, π] or NaN, any latitude | debug | `rl_crash::n_from_points`; release `r_from_points` | panics in debug builds; returns in release builds |
| `s2::Rect` | `from_center_size` | a centre longitude outside [−π, π] or NaN, or a NaN size longitude (finite latitudes) | debug | `rl_crash::n_from_center_size`; release `r_from_center_size` | panics in debug builds; returns in release builds. Infinite latitudes give `∞ − ∞` first, which Kani reports as NaN, so they are outside the debug proof |
| `s2::Rect` | `interior_contains_lat_lng` | a longitude outside [−π, π] or NaN, with the latitude strictly inside the rectangle's | debug | `rl_crash::n_interior_contains_lat_lng`; release `r_interior_contains_lat_lng` | panics in debug builds; returns in release builds. Over every point the first version returned: the latitude is tested first and a latitude outside answers `false`, as upstream. Corrected |
| `s2::Rect` | `expanded` | a NaN longitude margin, longitudes not full | debug | `rl_crash::n_expanded`; release `r_expanded` | panics in debug builds; returns in release builds. Full longitudes return (`rl_crash::o_expanded_nan_full_lng`), as `s1::Interval::expanded`. Corrected |
| `s2::Rect` | `get_distance_to_latlng` | an empty rectangle with any point, or an invalid point (NaN included) | debug | `rl_crash::n_get_distance_to_latlng` | panics in debug builds. Undocumented until gap 2: `# Panics` added (upstream only logs both) |
| `s2::Rect` | `get_distance` | a pair of rectangles of which one is empty | debug | `rl_crash::n_get_distance` | panics in debug builds, as upstream's `ABSL_DCHECK`s. Undocumented until gap 2: `# Panics` added |
| `r2::Rect` | `new`, `from_points` | exactly one empty axis, NaN bounds included | debug | `ur_crash::n_constructors`; release `r_constructors` | panics in debug builds; returns the invalid rectangle in release builds |
| `r2::Rect` | `from_center_size` | a finite centre and size with exactly one axis empty, `c − s/2 > c + s/2` | debug | `ur_crash::n_from_center_size` (196 s); release `r_from_center_size` | panics in debug builds; returns the invalid rectangle in release builds. The `# Panics` said "one size component negative and the other not": the first version returned, for a negative size that rounding absorbs (`−5e−324`, or `−1` at a centre of `1e300`). Corrected |
| `r2::Rect` | `project` | an empty axis or a NaN bound, any point | every | `ur_crash::n_project` | panics (both builds) |
| `s1::ChordAngle` | `sin`, `sin2`, `cos`, `tan` | `NEGATIVE`, `INFINITY` | debug | `ca_crash::n_trigonometry`; release `r_trigonometry` | panics in debug builds; in release builds returns the values the `# Panics` states (NaN, −1.25, 1.5; NaN, −∞, −∞) |
| `s1::ChordAngle` | `Add`, `Sub` | an operand `NEGATIVE` or `INFINITY` | debug | `ca_crash::n_add_sub`; release `r_add_sub` | panics in debug builds; returns in release builds |
| `s2::Cap` | `from_center_angle` | a centre that is not unit length, any angle | debug | `cp_crash::n_from_center_angle`; release `r_from_center_angle` (both with `pure_is_unit`) | panics in debug builds; returns an invalid cap in release builds |
| `s2::Cap` | `expanded` | a negative or NaN distance; +∞ with a non-empty cap | debug | `cp_crash::n_expanded`; release `r_expanded` | panics in debug builds; returns in release builds. Over every box cap and +∞ the first version returned: the empty cap returns before the sum (`cp_crash::o_expanded_empty_infinite`). Corrected |

`s2::rotate` states unit points (upstream's `ABSL_DCHECK`) but checks nothing, so there is no
panic to prove.

## 11. Floating-point geometry (gap 2, 2026-10-27)

The 62 entries that specs 004–007 excluded as floating-point geometry or as tool limits of
`asin`, `atan2` and `tan`, each now with a crash harness (`c_fg_*`, rows above). The method, the
measurements and the tool limits are in `verify/COVERAGE.md`; what the proofs trust is in
`TRUSTED_BASE.md`, "Floating-point geometry".

| Type | Entries | Proven per change | Proven weekly | Not proven |
|------|---------|-------------------|---------------|------------|
| `Cell` | 31 | 10 | 9 | 12: `exact_area` and the 8 distances to an edge or a cell (out of memory at 8 GiB, the robust predicates' exact arithmetic: the xlarge tier); `distance_to_point`, `boundary_distance`, `max_distance_to_point` (the NaN check, off the unit sphere) |
| `s2::Rect` | 18 | 6 | 5 | 7: `boundary_intersects`, `get_distance`, the two Hausdorff distances (60-minute timeouts), `get_distance_to_latlng` (the NaN check), the two `cell_union_bound`s (`Cap`'s tool limit) |
| `s2::Point` | 5 | 0 | 5 | 0 |
| `s1::ChordAngle` | 7 | 7 | 0 | 0 |
| `s1::Angle` | 1 | 1 | 0 | 0 |
| All | 62 | 24 | 19 | 19 |

`expanded_by_distance` counts as weekly: its shrinking branch proves per change, its growing one
weekly. `Display` (9 rows) stays out (gap 3).

## 12. `s2::LatLng` (33 entries)

Spec 010 (`specs/010-formal-verify-latlng/data-model.md` §5). The 19 inherent operations; the
operators `Add`, `Sub`, `Mul<LatLng> for f64`, `Mul<f64> for LatLng`; `Default`, `Display`,
`PartialEq`; `From<Point> for LatLng` and `From<LatLng> for Point`; the derived `Deserialize`;
the four `geo-types` conversions (an optional integration). The derived `Clone`, `Copy`, `Debug`
and `Serialize` are not entries. A `LatLng` has no validity invariant, so most domains are every
pair of `f64` (`AnyLatLng2`), narrowed only where an operation makes a new NaN (`∞ − ∞`, `0 · ∞`)
or by a documented precondition.

Statuses as section 9's. The rows group the entries.

**Summary** (T020, 2026-10-28): 31 in scope, 2 excluded (`Display`, and
`to_string_in_degrees`' formatting). 26 entries have Kani crash proofs over their whole
documented domains, every one `SUCCESSFUL` with its covers satisfied: 5 rows on the real code
and 3 under the function contracts (`atan2`, `sin_cos`, `sin`, `cos`, `asin`; `sqrt` too outside
`get_distance`'s domain), 9 harnesses, 35.5 s of harness time (`make verify-values P=latlng`).
The four `geo-types` conversions are crash-free by composition, `Deserialize` is tested. No
undocumented precondition was needed: `to_point`'s finite coordinates and `get_distance`'s
normalised inputs were documented already, and the proofs show what happens outside them (NaN,
no panic; FR-004, SC-010). `o_get_distance` first failed on CBMC's built-in `sqrt`, which
computes the NaN of a negative argument through a division its NaN check reports; under
`sqrt_contract` it proves.

| # | entries | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|---------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `s2::LatLng::new`, `from_radians`, `from_degrees`, `invalid`, `is_valid`, `Default`, `PartialEq` | in scope |  | `AnyF64²` |  | `ll_crash::c_construction` (0.3 s) | proven |
| 2 | `s2::LatLng::from_e5`, `from_e6`, `from_e7`, `from_unsigned_e6`, `from_unsigned_e7` | in scope |  | `AnyI32²`, `AnyU32²` |  | `ll_crash::c_e_forms` (0.3 s) | proven |
| 3 | `s2::LatLng::latitude`, `longitude`, `from_point`, `From<Point> for LatLng` | in scope (under contract) |  | `AnyTriple` |  | `ll_crash::c_from_point` (10.7 s; `atan2_contract`) | proven-under-contract |
| 4 | `s2::LatLng::to_point`, `From<LatLng> for Point` | in scope (under contract) |  | `AnyLatLng2` | finite coordinates for a meaningful result (upstream's debug check; documented): outside it the coordinates are NaN, no panic | `ll_crash::c_to_point` (0.4 s; `sin_cos_contract`) | proven-under-contract |
| 5 | `s2::LatLng::normalized` | in scope |  | `AnyLatLng2` |  | `ll_crash::c_normalized` (0.9 s; independent of `%`'s value) | proven |
| 6 | `s2::LatLng::get_distance` | in scope (under contract) |  | `ValidLatLng²`; outside it every pair without an infinite coordinate (`o_get_distance`) | both normalised (documented; upstream logs) | `ll_crash::c_get_distance` (14.7 s; `sin_contract`, `cos_contract`, `asin_contract`), `ll_crash::o_get_distance` (7.2 s; also `sqrt_contract`) | proven-under-contract |
| 7 | `s2::LatLng::approx_eq`, `approx_eq_with` | in scope |  | `AnyLatLng2²` without `∞ − ∞` × `AnyF64` |  | `ll_crash::c_approx_eq` (0.4 s) | proven |
| 8 | `Add`, `Sub`, `Mul<LatLng> for f64`, `Mul<f64> for LatLng` | in scope |  | `AnyLatLng2²` without `∞ − ∞`; every factor without `0 · ∞` |  | `ll_crash::c_arithmetic` (0.6 s) | proven |
| 9 | `s2::LatLng::to_string_in_degrees` | excluded | `core::fmt` is too slow for Kani, as every type's text; its normalisation is row 5's, and the `LATLNG_STR` rows check the string |  |  |  | — |
| 10 | `Display for LatLng` | excluded | `core::fmt` |  |  |  | — |
| 11 | `Deserialize for LatLng` (derived) | in scope (tested) |  | every serialised `LatLng` |  | `latlng_tests::prop_serde_accepts_every_pair` (not a Kani entry) | tested |
| 12 | `From<Coord> for LatLng`, `From<LatLng> for Coord`, `From<geo_types::Point> for LatLng`, `From<LatLng> for geo_types::Point` | in scope (by composition) |  | every `f64` pair |  | each is one `from_degrees` or two `degrees` calls (`geo_types_interop.rs:114–155`), crash-free for every `f64` by `ag_crash::c_construction_and_accessors` (spec 007); the verification crate does not enable `geo-types` | proven-by-composition |
