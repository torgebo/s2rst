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

## 1. `Cell` (53 entries)

| # | entry | status | exclusion reason | domain | preconditions (source) | crash harness | crash status |
|---|-------|--------|------------------|--------|------------------------|---------------|--------------|
| 1 | `Cell::from_cell_id` | in scope | | `ValidCell` | id is a valid cell (`# Panics`, FR-017) | `cl_crash::c_from_cell_id` (13.0 s), `c_from_cell_id_largest` | proven |
| 2 | `Cell::from_point` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 3 | `Cell::from_lat_lng` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 4 | `Cell::from_face` | in scope | | faces 0–5 | face ≤ 5 (`# Panics`, FR-017) | `cl_crash::c_from_face` (1.6 s) | proven |
| 5 | `Cell::from_face_pos_level` | in scope | | faces 0–5 × `pos < 2^61` × levels 0–30 | as `CellId::from_face_pos_level` (`# Panics`) | `cl_crash::c_from_face_pos_level` (13.1 s) | proven |
| 6 | `Cell::face` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 7 | `Cell::level` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 8 | `Cell::id` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 9 | `Cell::orientation` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 10 | `Cell::is_leaf` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 11 | `Cell::size_ij` | in scope | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 12 | `Cell::size_st` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 13 | `Cell::bound_uv` | in scope (stored-bound reader) | | `ValidCell` | | `cl_crash::c_from_cell_id` | proven |
| 14 | `Cell::vertex` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 15 | `Cell::vertex_raw` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 16 | `Cell::edge` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 17 | `Cell::edge_raw` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 18 | `Cell::center` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 19 | `Cell::center_raw` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 20 | `Cell::uv_coord_of_edge` | in scope (stored-bound reader) | | `ValidCell` × `Edge` | | `cl_crash::c_uv_coord_of_edge` (10.3 s) | proven |
| 21 | `Cell::ij_coord_of_edge` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 22 | `Cell::children` | in scope | | `ValidCell` | | `cl_crash::c_children` (35.6 s) | proven |
| 23 | `Cell::average_area_for_level` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 24 | `Cell::average_area` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 25 | `Cell::approx_area` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 26 | `Cell::exact_area` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 27 | `Cell::contains_point` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 28 | `Cell::contains_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` (32.6 s) | proven |
| 29 | `Cell::intersects_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` | proven |
| 30 | `Cell::cap_bound` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 31 | `Cell::rect_bound` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 32 | `Cell::cell_union_bound` | in scope | | `ValidCell` | | `cl_crash::c_containment` | proven |
| 33 | `Cell::distance_to_point` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 34 | `Cell::distance_to_edge` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 35 | `Cell::distance_to_cell` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 36 | `Cell::max_distance_to_point` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 37 | `Cell::max_distance_to_edge` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 38 | `Cell::max_distance_to_cell` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 39 | `Cell::boundary_distance` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 40 | `Cell::is_distance_less` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 41 | `Cell::is_distance_less_or_equal` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 42 | `Cell::is_max_distance_less` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 43 | `Cell::is_max_distance_less_or_equal` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 44 | `impl PartialEq for Cell` | in scope | | `ValidCell`² | | `cl_crash::c_eq` (24.3 s) | proven |
| 45 | `impl Default for Cell` | in scope | | (no input) | | `cl_crash::c_from_and_default` (36.3 s) | proven |
| 46 | `impl From<CellId> for Cell` | in scope | | `ValidCell` | id is a valid cell (`# Panics`, FR-017) | `cl_crash::c_from_and_default` | proven |
| 47 | `impl Display for Cell` | excluded | tool limit: formats through `core::fmt`, as 001's `Display` (P-ENC-12) | | | | n/a |
| 48 | `impl Region for Cell::cap_bound` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 49 | `impl Region for Cell::rect_bound` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |
| 50 | `impl Region for Cell::cell_union_bound` | in scope | | `ValidCell` | | `cl_crash::c_containment` | proven |
| 51 | `impl Region for Cell::contains_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` | proven |
| 52 | `impl Region for Cell::intersects_cell` | in scope | | `ValidCell`² | | `cl_crash::c_containment` | proven |
| 53 | `impl Region for Cell::contains_point` | excluded | floating-point geometry (FR-002). Tested for `Cell` by the `prop_cell_*` tests of `core/tests/property_tests.rs` and the `region_cell_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`'s float `Region` laws) | | | | n/a |

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
| 15 | `s2::Rect::area` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 16 | `s2::Rect::contains_lat_lng` | in scope | | `ValidRect` × every `LatLng`, NaN included | none: an invalid point is not contained (K2, FR-018) | `rl_crash::c_screened_point_operations` (2.1 s) | proven |
| 17 | `s2::Rect::contains_point` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 18 | `s2::Rect::contains` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` (23.2 s) | proven |
| 19 | `s2::Rect::intersects` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 20 | `s2::Rect::from_point_pair` | in scope | | `ValidLatLng`² | a valid longitude (debug-checked; `# Panics`, FR-019); the latitude is unchecked | `rl_crash::c_from_points` | proven |
| 21 | `s2::Rect::full_lat` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 22 | `s2::Rect::full_lng` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 23 | `s2::Rect::is_inverted` | in scope | | `ValidRect` |  | `rl_crash::c_classifiers_and_bounds` | proven |
| 24 | `s2::Rect::interior_contains_lat_lng` | in scope | | `ValidRect` × `ValidLatLng` | a valid longitude (debug-checked; `# Panics`, FR-019) | `rl_crash::c_interior_contains_lat_lng` (1.3 s) | proven |
| 25 | `s2::Rect::interior_contains_point` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 26 | `s2::Rect::interior_contains` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 27 | `s2::Rect::interior_intersects` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 28 | `s2::Rect::add_point` | in scope | | `ValidRect` × every `LatLng`, NaN included | none: an invalid point is ignored (K2, FR-018) | `rl_crash::c_screened_point_operations` | proven |
| 29 | `s2::Rect::polar_closure` | in scope | | `ValidRect` |  | `rl_crash::c_polar_closure` (0.1 s) | proven |
| 30 | `s2::Rect::expanded` | in scope | | `ValidRect` × a margin with non-NaN components | a non-NaN longitude margin (debug-checked; `# Panics`, FR-019) | `rl_crash::c_expanded` (87.8 s) | proven |
| 31 | `s2::Rect::expanded_by_distance` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 32 | `s2::Rect::union` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 33 | `s2::Rect::intersection` | in scope | | `ValidRect`² |  | `rl_crash::c_relations_and_set_operations` | proven |
| 34 | `s2::Rect::expand_for_subregions` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 35 | `s2::Rect::cap_bound` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 36 | `s2::Rect::rect_bound` | in scope | | `ValidRect` |  | `rl_crash::c_eq_and_bound` (1.2 s) | proven |
| 37 | `s2::Rect::cell_union_bound` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 38 | `s2::Rect::centroid` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 39 | `s2::Rect::boundary_intersects` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 40 | `s2::Rect::get_distance_to_latlng` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 41 | `s2::Rect::get_distance` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 42 | `s2::Rect::get_hausdorff_distance` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 43 | `s2::Rect::get_directed_hausdorff_distance` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 44 | `s2::Rect::approx_eq` | in scope | | `ValidRect`² |  | `rl_crash::c_approx_eq` (23.2 s) | proven |
| 45 | `impl PartialEq for s2::Rect` | in scope | | `ValidRect`² |  | `rl_crash::c_eq_and_bound` | proven |
| 46 | `impl Default for s2::Rect` | in scope | | (no input) |  | `rl_crash::c_constructors` | proven |
| 47 | `impl Display for s2::Rect` | excluded | tool limit: the formatter (`core::fmt`) is outside what Kani handles in the budget, as for 001's and 004's `Display` | | | | n/a |
| 48 | `impl Region for s2::Rect::cap_bound` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 49 | `impl Region for s2::Rect::rect_bound` | in scope | | `ValidRect` |  | `rl_crash::c_eq_and_bound` | proven |
| 50 | `impl Region for s2::Rect::cell_union_bound` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 51 | `impl Region for s2::Rect::contains_cell` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 52 | `impl Region for s2::Rect::intersects_cell` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |
| 53 | `impl Region for s2::Rect::contains_point` | excluded | floating-point geometry (FR-003). Tested by the `region_rect_*` tests of `core/tests/trait_property_tests.rs` (`CORE_TRAITS.md`) and, for `cap_bound`, D-006's `core/src/s2/rect_tests.rs` | | | | n/a |

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
| 26 | `s2::Point::distance` | excluded | tool limit: reaches `f64::atan2` through `r3::Vector::angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_pt_distance_symmetric` failed on it in 14.9 s). Equal points return early, so P-PTDST-01 proves them for `p` and `p`; `core/tests/cpp_values_diff.rs` checks `distance` against `S1Angle` on every C++ row | `BoxPoint`² | | | n/a |
| 27 | `s2::Point::stable_angle` | excluded | tool limit: reaches `f64::atan2` through `r3::Vector::angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_pt_distance_symmetric` failed on it in 14.9 s). Equal points return early, so P-PTDST-01 proves them for `p` and `p`; `core/tests/cpp_values_diff.rs` checks `distance` against `S1Angle` on every C++ row | `BoxPoint`² | | | n/a |
| 28 | `s2::Point::approx_eq` | excluded | tool limit: reaches `f64::atan2` through `r3::Vector::angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_pt_distance_symmetric` failed on it in 14.9 s). Equal points return early, so P-PTDST-01 proves them for `p` and `p`; `core/tests/cpp_values_diff.rs` checks `distance` against `S1Angle` on every C++ row | `BoxPoint`² | | | n/a |
| 29 | `s2::Point::approx_eq_angle` | excluded | tool limit: reaches `f64::atan2` through `r3::Vector::angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_pt_distance_symmetric` failed on it in 14.9 s). Equal points return early, so P-PTDST-01 proves them for `p` and `p`; `core/tests/cpp_values_diff.rs` checks `distance` against `S1Angle` on every C++ row | `BoxPoint`² | | | n/a |
| 30 | `s2::Point::approx_eq_with` | excluded | tool limit: reaches `f64::atan2` through `r3::Vector::angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_pt_distance_symmetric` failed on it in 14.9 s). Equal points return early, so P-PTDST-01 proves them for `p` and `p`; `core/tests/cpp_values_diff.rs` checks `distance` against `S1Angle` on every C++ row | `BoxPoint`² | | | n/a |
| 31 | `impl Display for s2::Point` | excluded | tool limit: formats through `core::fmt`, as 001's and 004's `Display` | | | | n/a |

## 7. `s1::ChordAngle` (36 entries)

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
| 9 | `s1::ChordAngle::to_angle` | excluded | tool limit: reaches `f64::asin` through `to_angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_ca_to_angle_crash` failed on it in 0.8 s); `core/tests/cpp_values_diff.rs` checks `to_angle` and the E-forms against `S1ChordAngle` on every `CHORD_ANGLE` row | `Chord` |  |  | n/a |
| 10 | `s1::ChordAngle::radians` | excluded | tool limit: reaches `f64::asin` through `to_angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_ca_to_angle_crash` failed on it in 0.8 s); `core/tests/cpp_values_diff.rs` checks `to_angle` and the E-forms against `S1ChordAngle` on every `CHORD_ANGLE` row | `Chord` |  |  | n/a |
| 11 | `s1::ChordAngle::degrees` | excluded | tool limit: reaches `f64::asin` through `to_angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_ca_to_angle_crash` failed on it in 0.8 s); `core/tests/cpp_values_diff.rs` checks `to_angle` and the E-forms against `S1ChordAngle` on every `CHORD_ANGLE` row | `Chord` |  |  | n/a |
| 12 | `s1::ChordAngle::e5` | excluded | tool limit: reaches `f64::asin` through `to_angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_ca_to_angle_crash` failed on it in 0.8 s); `core/tests/cpp_values_diff.rs` checks `to_angle` and the E-forms against `S1ChordAngle` on every `CHORD_ANGLE` row | `Chord` |  |  | n/a |
| 13 | `s1::ChordAngle::e6` | excluded | tool limit: reaches `f64::asin` through `to_angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_ca_to_angle_crash` failed on it in 0.8 s); `core/tests/cpp_values_diff.rs` checks `to_angle` and the E-forms against `S1ChordAngle` on every `CHORD_ANGLE` row | `Chord` |  |  | n/a |
| 14 | `s1::ChordAngle::e7` | excluded | tool limit: reaches `f64::asin` through `to_angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_ca_to_angle_crash` failed on it in 0.8 s); `core/tests/cpp_values_diff.rs` checks `to_angle` and the E-forms against `S1ChordAngle` on every `CHORD_ANGLE` row | `Chord` |  |  | n/a |
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
| 31 | `impl From<s1::ChordAngle> for s1::Angle` | excluded | tool limit: reaches `f64::asin` through `to_angle`, a foreign C call Kani replaces with a failing check (research R0: `r0_ca_to_angle_crash` failed on it in 0.8 s); `core/tests/cpp_values_diff.rs` checks `to_angle` and the E-forms against `S1ChordAngle` on every `CHORD_ANGLE` row | `Chord` |  |  | n/a |
| 32 | `impl Add for s1::ChordAngle` | in scope |  | `ChordLength`² | neither operand special (FR-019; `s1chord_angle.cc:92–93`) | `ca_crash::c_add_sub` (19.7 s) | proven |
| 33 | `impl Sub for s1::ChordAngle` | in scope |  | `ChordLength`² | neither operand special (FR-019; `s1chord_angle.cc:116–117`) | `ca_crash::c_add_sub` | proven |
| 34 | `impl Display for s1::ChordAngle` | excluded | tool limit: formats through `core::fmt` and `to_angle` (`asin`), as 001's and 004's `Display` |  |  |  | n/a |
| 35 | `PartialEq (derived)` | in scope |  | `Chord`² |  | `ca_crash::c_accessors_classes_and_order` | proven |
| 36 | `PartialOrd (derived)` | in scope |  | `Chord`² |  | `ca_crash::c_accessors_classes_and_order` | proven |

## 8. `s1::Angle` (31 entries)

The 20 inherent operations count `approx_eq`; the two constants (`ZERO`, `INFINITY`) are read by
the crash harnesses but are not entries. The derived `PartialEq` and `PartialOrd` share a row,
so the table has 30 rows for 31 entries.

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
| 17 | `s1::Angle::tan` | excluded | tool limit: calls the C library's `tan`, a foreign C call Kani replaces with a failing check (research R0: `r0_ag_tan_crash` failed on it in 0.0 s); `core/tests/cpp_values_diff.rs` checks it against `S1Angle` on every `ANGLE` row | `AnyAngle` |  |  | n/a |
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
