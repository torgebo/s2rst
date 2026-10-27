# Operation Inventory: `s2rst::s2::CellId` (FR-002)

Every inherent public operation of `CellId`, the public module-level functions that construct
a `CellId` or compute its level-derived constants, and its hand-written trait
implementations: **64 rows** (54 methods, 3 free functions, 7 trait impls), checked against
`core/src/s2/cell_id.rs`. Field definitions: `specs/001-formal-verify-base-type/data-model.md`
§3. Domain classes: data-model §2.

Each in-scope row has a crash-freedom obligation **C-&lt;op&gt;**: "*op* completes without
panic, arithmetic overflow or undefined behaviour for every input in *domain*, assuming
*preconditions*" (FR-003).

**Domain rule** (spec Edge Cases, FR-003): every bit pattern (`Any`) only for operations
documented to accept any value; otherwise `Valid`, extended to `ValidOrEnd` (adding `end(l)`
and `sentinel`) where upstream usage passes past-the-end values. Other arguments range over
their whole type, narrowed only by a documented precondition (rustdoc, `debug_assert!`, or
upstream `DCHECK`/doc comment).

**Summary**: 48 in scope (75.0 %, SC-002 floor 48), 16 excluded (12 `float-geometry`,
3 `tool-limit`, 1 `float-geometry: crash-freedom intractable`). Since gap 2 of the 2026-10-26
coverage review, 7 of the 12 float-geometry rows are proven crash-free by the `Cell` harnesses
that run them (`cl_crash::c_fg_*`, shared): 55 in scope. `vertex_neighbors` is at risk
under the R15 budget (research R12).

## 1. Status, domain and crash proof

| # | name | kind | status | exclusion_reason | domain | preconditions (source) | crash_harness | crash_status | covers |
|---|------|------|--------|------------------|--------|------------------------|---------------|--------------|--------|
| 1 | `CellId::none` | constructor | in-scope |  | — | — | `crash::c_none` | proven | 1/1 |
| 2 | `CellId::sentinel` | constructor | in-scope |  | — | — | `crash::c_sentinel` | proven | 1/1 |
| 3 | `CellId::from_face` | constructor | in-scope |  | Face | `face ≤ 5` (rustdoc "0–5") | `crash::c_from_face` | proven | 3/3 |
| 4 | `CellId::from_face_pos_level` | constructor | in-scope |  | Face × U64 × Level | `pos < 2^61` (rustdoc "61-bit") | `crash::c_from_face_pos_level` | proven | 4/4 |
| 5 | `CellId::from_point` | constructor | in-scope (gap 2, 2026-10-27, shared) |  | BoxPoint but the origin | not the zero vector (`# Panics`, gap 1: `cl_crash::n_from_point`) | `cl_crash::c_fg_from_point` (shared, weekly; stub `any_uv_bound`) | proven | 1/1 |
| 6 | `CellId::from_lat_lng` | constructor | in-scope (gap 2, 2026-10-27, shared) |  | ValidLatLng | — | `cl_crash::c_fg_from_lat_lng` (shared, weekly; `sin_cos_contract`) | proven-under-contract | 1/1 |
| 7 | `CellId::id` | query | in-scope |  | Any | — | `crash::c_id` | proven | 1/1 |
| 8 | `CellId::is_valid` | query | in-scope |  | Any | — | `crash::c_is_valid` | proven | 2/2 |
| 9 | `CellId::face` | query | in-scope |  | Valid | — | `crash::c_face` | proven | 2/2 |
| 10 | `CellId::pos` | query | in-scope |  | ValidOrEnd | — | `crash::c_pos` | proven | 2/2 |
| 11 | `CellId::level` | query | in-scope |  | ValidOrEnd | excludes ids with ≥ 62 trailing zeros, incl. `0` (upstream `ABSL_ASSUME(id_ != 0)`, `s2cell_id.h:599`; upstream comment: defined for end-iterators) | `crash::c_level` | proven | 3/3 |
| 12 | `CellId::lsb` | query | in-scope |  | ValidOrEnd | — | `crash::c_lsb` | proven | 2/2 |
| 13 | `CellId::is_leaf` | query | in-scope |  | ValidOrEnd | — | `crash::c_is_leaf` | proven | 2/2 |
| 14 | `CellId::is_face` | query | in-scope |  | ValidOrEnd | — | `crash::c_is_face` | proven | 2/2 |
| 15 | `CellId::child_position` | navigation | in-scope |  | Valid × Level | `1 ≤ l ≤ level` (`debug_assert!`, rustdoc) | `crash::c_child_position` | proven | 2/2 |
| 16 | `CellId::parent_at_level` | navigation | in-scope |  | Valid × Level | `l ≤ level` (rustdoc) | `crash::c_parent_at_level` | proven | 2/2 |
| 17 | `CellId::parent` | navigation | in-scope |  | Valid | not a face (`debug_assert!`; rustdoc `# Panics`, debug builds) | `crash::c_parent` | proven | 2/2 |
| 18 | `CellId::children` | navigation | in-scope |  | Valid | not a leaf (`debug_assert!`; upstream `DCHECK(!is_leaf())`) | `crash::c_children` | proven | 2/2 |
| 19 | `CellId::child_begin` | navigation | in-scope |  | Valid | not a leaf (`debug_assert!`) | `crash::c_child_begin` | proven | 2/2 |
| 20 | `CellId::child_end` | navigation | in-scope |  | Valid | not a leaf (`debug_assert!`) | `crash::c_child_end` | proven | 2/2 |
| 21 | `CellId::child_begin_at_level` | navigation | in-scope |  | Valid × Level | `l ≥ level` (rustdoc; upstream `DCHECK_GE`) | `crash::c_child_begin_at_level` | proven | 2/2 |
| 22 | `CellId::child_end_at_level` | navigation | in-scope |  | Valid × Level | `l ≥ level` (rustdoc; upstream `DCHECK_GE`) | `crash::c_child_end_at_level` | proven | 2/2 |
| 23 | `CellId::begin` | constructor | in-scope |  | Level | — | `crash::c_begin` | proven | 2/2 |
| 24 | `CellId::end` | constructor | in-scope |  | Level | — | `crash::c_end` | proven | 2/2 |
| 25 | `CellId::range_min` | query | in-scope |  | ValidOrEnd | — (upstream: no `DCHECK`; called on the sentinel by index iterators) | `crash::c_range_min` | proven | 2/2 |
| 26 | `CellId::range_max` | query | in-scope |  | ValidOrEnd | — | `crash::c_range_max` | proven | 2/2 |
| 27 | `CellId::contains` | query | in-scope |  | Valid × Valid | both valid (upstream `DCHECK(is_valid())` ×2) | `crash::c_contains` | proven | 2/2 |
| 28 | `CellId::intersects` | query | in-scope |  | Valid × Valid | both valid (upstream `DCHECK(is_valid())` ×2) | `crash::c_intersects` | proven | 2/2 |
| 29 | `CellId::next` | navigation | in-scope |  | Valid | — | `crash::c_next` | proven | 2/2 |
| 30 | `CellId::prev` | navigation | in-scope |  | ValidOrEnd | — (upstream iteration calls `End(l).prev()`) | `crash::c_prev` | proven | 2/2 |
| 31 | `CellId::next_wrap` | navigation | in-scope |  | Valid | valid (`debug_assert!`) | `crash::c_next_wrap` | proven | 2/2 |
| 32 | `CellId::prev_wrap` | navigation | in-scope |  | Valid | valid (`debug_assert!`) | `crash::c_prev_wrap` | proven | 2/2 |
| 33 | `CellId::advance` | navigation | in-scope |  | Valid × I64 | valid (upstream `DCHECK(is_valid())`) | `crash::c_advance` | proven | 4/4 |
| 34 | `CellId::advance_wrap` | navigation | in-scope |  | Valid × I64 | valid (upstream `DCHECK(is_valid())`) | `crash::c_advance_wrap` | proven | 4/4 |
| 35 | `CellId::distance_from_begin` | query | in-scope |  | ValidOrEnd | — | `crash::c_distance_from_begin` | proven | 2/2 |
| 36 | `CellId::get_center_si_ti` | encoding | in-scope |  | Valid | — | `crash::c_get_center_si_ti` | proven | 2/2 |
| 37 | `CellId::to_point_raw` | encoding | in-scope (gap 2, 2026-10-27, shared) |  | Valid | — | `cl_crash::c_fg_center` (shared, weekly, through `Cell::center_raw`) | proven | 1/1 |
| 38 | `CellId::to_point` | encoding | in-scope (gap 2, 2026-10-27, shared) |  | Valid | — | `cl_crash::c_fg_center` (shared, weekly, through `Cell::center`) | proven | 1/1 |
| 39 | `CellId::to_lat_lng` | encoding | excluded | float-geometry |  |  |  |  |  |
| 40 | `CellId::center_st` | encoding | excluded | float-geometry |  |  |  |  |  |
| 41 | `CellId::center_uv` | encoding | excluded | float-geometry |  |  |  |  |  |
| 42 | `CellId::size_st` | free-fn | in-scope (gap 2, 2026-10-27, shared) |  | Level | — | `cl_crash::c_fg_size_and_ij` (shared, through `Cell::size_st`); a raw level above 30 panics (`crash::n_raw_level`) | proven | 1/1 |
| 43 | `CellId::bound_st` | encoding | excluded | float-geometry |  |  |  |  |  |
| 44 | `CellId::bound_uv` | encoding | excluded | float-geometry |  |  |  |  |  |
| 45 | `CellId::to_face_ij_orientation` | encoding | in-scope |  | Valid | — | `crash::c_to_face_ij_orientation` | proven | 3/3 |
| 46 | `CellId::to_token` | encoding | excluded | tool-limit: `format!("{:016x}")` over 5 min (research R6) |  |  |  |  |  |
| 47 | `CellId::from_token` | encoding | in-scope |  | Text(L), L ≤ 17 | — | `enc::p_enc_06__b{0..=17}` (shared) | proven-within-bounds (input size: byte length ≤ 17; longer texts are rejected by the length check before any loop (FR-007), exercised by the FR-017 tests and fuzz target) | every member satisfied (18/18) |
| 48 | `CellId::edge_neighbors` | neighbour | in-scope (weekly slow tier, R12, R15) |  | Valid | — | `crash::c_edge_neighbors__f{0..5}` (weekly slow tier) | proven (6/6 faces; 229–395 s each, weekly slow tier) | 2/2 per face |
| 49 | `CellId::vertex_neighbors` | neighbour | in-scope (weekly slow tier, R12) |  | Valid × Level | `l < level` (upstream `DCHECK_LT(level, this->level())`) | `crash::c_vertex_neighbors` (weekly slow tier) | proven (857 s, weekly slow tier) | 2/2 |
| 50 | `CellId::all_neighbors` | neighbour | excluded | float-geometry: crash-freedom intractable (OOM above 9.5 GB at gap 0, research R12) |  |  |  |  |  |
| 51 | `CellId::common_ancestor_level` | query | in-scope |  | Any × Any | not (`none`, `none`) (upstream `ABSL_ASSUME(bits != 0)`, `s2cell_id.cc:198`) | `crash::c_common_ancestor_level` | proven | 3/3 |
| 52 | `CellId::maximum_tile` | navigation | in-scope |  | Valid × ValidOrEnd | — | `crash::c_maximum_tile` | proven | 4/4 |
| 53 | `CellId::to_debug_string` | encoding | excluded | tool-limit: `format!` branch OOM above 6 GB; valid branch over 5 min (research R6) |  |  |  |  |  |
| 54 | `CellId::from_debug_string` | encoding | in-scope |  | Text(L), L ≤ 33 | — | `enc::p_enc_09__b{0..=33}` (shared) | proven-within-bounds (input size: byte length ≤ 33; longer texts are rejected by the length check before any loop (FR-007), exercised by the FR-033 tests and fuzz target) | every member satisfied (34/34) |
| 55 | `s2::lsb_for_level` | free-fn | in-scope |  | Level | — | `crash::c_lsb_for_level` | proven | 2/2 |
| 56 | `s2::size_ij` | free-fn | in-scope |  | Level | — | `crash::c_size_ij` | proven | 2/2 |
| 57 | `s2::from_face_ij` | free-fn | in-scope |  | Face × IJ × IJ | `i, j ∈ [0, 2^30 − 1]` (upstream `s2coords.h:44–45`); over all `i32` Rust overflows in `n * 2` (debug) | `crash::c_from_face_ij` | proven | 3/3 |
| 58 | `impl Default for CellId` | trait | in-scope |  | — | — | `crash::c_default` | proven | 1/1 |
| 59 | `impl From<u64> for CellId` | trait | in-scope |  | Any | — | `crash::c_from_u64` | proven | 1/1 |
| 60 | `impl From<CellId> for u64` | trait | in-scope |  | Any | — | `crash::c_u64_from` | proven | 1/1 |
| 61 | `impl From<&Point> for CellId` | trait | in-scope (gap 2, 2026-10-27, shared) |  | BoxPoint but the origin | as `from_point` | `cl_crash::c_fg_from_point` (shared, weekly) | proven | 1/1 |
| 62 | `impl From<&LatLng> for CellId` | trait | in-scope (gap 2, 2026-10-27, shared) |  | ValidLatLng | — | `cl_crash::c_fg_from_lat_lng` (shared, weekly; `sin_cos_contract`) | proven-under-contract | 1/1 |
| 63 | `impl Display for CellId` | trait | excluded | tool-limit: `core::fmt` path over 5 min (research R6) |  |  |  |  |  |
| 64 | `impl FromStr for CellId` | trait | in-scope |  | Text(L), L ≤ 33 | — | `enc::p_enc_10__b{0..=33}` (shared) | proven-within-bounds (input size: byte length ≤ 33; longer texts are rejected by the length check before any loop (FR-007), exercised by the FR-033 tests and fuzz target) | every member satisfied (34/34) |

Note on row 42: `CellId::size_st` is an associated function without `self`; it is listed with
the methods because it is defined in `impl CellId`.

## 2. Properties, result validity, panics and upstream findings

`undocumented_panic` records a precondition the proof had to assume although the Rust docs
do not state it (FR-003); each one gets a `# Panics` section in the documentation-only commit
(research R17, `panics_doc`). Every `impl Into<Level>` argument panics in the conversion for a
raw `u8 > 30` (`impl From<u8> for Level`), inside the operation (spec Edge Cases); that is
recorded once here for rows 4, 15, 16, 21, 22, 23, 24, 42, 49, 50, 55, 56.

| # | name | properties | result_validity (FR-004) | undocumented_panic | panics_doc | upstream_findings |
|---|------|------------|--------------------------|--------------------|------------|-------------------|
| 1 | `none` | P-VAL-05 | — (returns the invalid `none` by definition) |  |  |  |
| 2 | `sentinel` | P-VAL-05 | — (returns the invalid `sentinel` by definition) |  |  |  |
| 3 | `from_face` | P-VAL-09 | P-VAL-09 (a `compose` result, valid by P-VAL-01/03) |  |  |  |
| 4 | `from_face_pos_level` | P-VAL-10 | P-VAL-10 (a `compose` result) | raw level `> 30` | added (8a06089) |  |
| 7 | `id` | P-VAL-04 | — |  |  |  |
| 8 | `is_valid` | P-VAL-03 | — |  |  |  |
| 9 | `face` | P-VAL-06 | — |  |  |  |
| 10 | `pos` | P-VAL-06 | — |  |  |  |
| 11 | `level` | P-VAL-06 | — | ids with ≥ 62 trailing zeros, incl. `0` | added (8a06089) | U-001 |
| 12 | `lsb` | P-VAL-07 | — |  |  |  |
| 13 | `is_leaf` | P-VAL-07 | — |  |  |  |
| 14 | `is_face` | P-VAL-07 | — |  |  |  |
| 15 | `child_position` | P-HIER-05 | — | raw level `> 30`; debug: not valid, or `l ∉ 1..=level` | added (8a06089) |  |
| 16 | `parent_at_level` | P-HIER-04 | P-HIER-04 | raw level `> 30` | added (8a06089) |  |
| 17 | `parent` | P-HIER-03 | P-HIER-03 | debug: a face cell (the rustdoc said "Panics if `self.is_face()`" without the debug-only qualifier) | added (8a06089) |  |
| 18 | `children` | P-HIER-01, P-HIER-02 | P-HIER-01 | debug: not valid, or a leaf | added (8a06089) |  |
| 19 | `child_begin` | P-HIER-06 | P-HIER-06 | debug: not valid, or a leaf | added (8a06089) |  |
| 20 | `child_end` | P-HIER-06 | P-HIER-06 pins it; may be past-the-end (`end(level+1)` after the last cell), as upstream | debug: not valid, or a leaf | added (8a06089) |  |
| 21 | `child_begin_at_level` | P-HIER-07 | P-HIER-07 | raw level `> 30` | added (8a06089) |  |
| 22 | `child_end_at_level` | P-HIER-07 | P-HIER-07 pins it; may be past-the-end, as upstream | raw level `> 30` | added (8a06089) |  |
| 23 | `begin` | P-HIER-08 | P-HIER-08 | raw level `> 30` | added (8a06089) |  |
| 24 | `end` | P-HIER-08 | — (past-the-end by definition) | raw level `> 30` | added (8a06089) |  |
| 25 | `range_min` | P-RANGE-01 | P-RANGE-01 (a valid leaf, for valid input) |  |  |  |
| 26 | `range_max` | P-RANGE-01 | P-RANGE-01 (a valid leaf, for valid input) |  |  |  |
| 27 | `contains` | P-RANGE-02, P-RANGE-04, P-RANGE-05 | — |  |  |  |
| 28 | `intersects` | P-RANGE-03, P-RANGE-05 | — |  |  |  |
| 29 | `next` | P-TRAV-01, P-TRAV-02, P-TRAV-07 | P-TRAV-01 pins it; may be `end(level)`, as upstream |  |  |  |
| 30 | `prev` | P-TRAV-02, P-TRAV-07 | P-TRAV-02 (valid for `pos ≥ 1` and for `end(l)`); `begin(l).prev()` is not a valid cell, the same arithmetic as upstream |  |  |  |
| 31 | `next_wrap` | P-TRAV-03 | P-TRAV-03 (always valid) | debug: not valid | added (8a06089) |  |
| 32 | `prev_wrap` | P-TRAV-03 | P-TRAV-03 (always valid) | debug: not valid | added (8a06089) |  |
| 33 | `advance` | P-TRAV-04, P-TRAV-07 | P-TRAV-04 pins it; may be `end(level)`, as upstream |  |  |  |
| 34 | `advance_wrap` | P-TRAV-05 | P-TRAV-05 (always valid) |  |  |  |
| 35 | `distance_from_begin` | P-TRAV-06 | — |  |  |  |
| 36 | `get_center_si_ti` | P-ENC-04 | — |  |  |  |
| 45 | `to_face_ij_orientation` | P-ENC-01, P-ENC-03 | — |  |  |  |
| 47 | `from_token` | P-ENC-06 (P-ENC-05 outside the SC-003 denominator) | — (any `u64`, as upstream `FromToken`) |  |  |  |
| 48 | `edge_neighbors` | P-NBR-01, P-NBR-02 | P-NBR-01 (four valid cells; weekly slow tier) |  |  |  |
| 49 | `vertex_neighbors` | P-NBR-03 | P-NBR-03 (3 or 4 valid cells; weekly slow tier) | `l = 30` (`Level::new(31)`); raw level `> 30` | added (8a06089) |  |
| 51 | `common_ancestor_level` | P-ANC-01, P-ANC-02 | — | (`none`, `none`): `ilog2(0)` | added (8a06089) | U-002 |
| 52 | `maximum_tile` | P-RANGE-06 | P-RANGE-06 (valid, or `limit` itself, which may be past-the-end, as upstream) |  |  |  |
| 54 | `from_debug_string` | P-ENC-09 (P-ENC-08 outside the SC-003 denominator) | P-ENC-09 (`Some` only for a valid cell: the model builds it with `compose`) |  |  |  |
| 55 | `lsb_for_level` | P-VAL-08 | — | raw level `> 30` | added (8a06089) |  |
| 56 | `size_ij` | P-VAL-08 | — | raw level `> 30` | added (8a06089) |  |
| 57 | `from_face_ij` | P-ENC-02, P-ENC-03 | P-ENC-02 (the result is a valid leaf) | debug: `n * 2` overflows for some `i` or `j` outside `[0, 2^30)` (not all: section 3); C++ wraps silently: defect D-001 (task T056) | added (8a06089), corrected 2026-10-26 (section 3) |  |
| 58 | `Default` | P-VAL-05 | — (returns `none`) | | | |
| 59 | `From<u64>` | P-VAL-04 | — (any `u64`) | | | |
| 60 | `From<CellId> for u64` | P-VAL-04 | — | | | |
| 64 | `FromStr` | P-ENC-10 | P-ENC-10 (`Ok` only for a valid cell, as `from_debug_string`) | | | |

Excluded rows keep their catalogue properties as `not-proven`: rows 46, 53, 63 →
P-ENC-05, P-ENC-07, P-ENC-08, P-ENC-11, P-ENC-12 (`tool-limit`); row 50 → P-NBR-04
(`excluded: all_neighbors`). They stay outside the SC-003 denominator (spec SC-003).

## 3. Outside the domain: the documented panics (2026-10-26)

The crash proofs above show that an operation does not panic inside its domain. These show
the other half of its `# Panics` section, so that the section is exact: for every input
outside the domain that the section names, the operation panics (`n_*`, a
`#[kani::should_panic]` harness whose one cover after the call must stay unsatisfied, so no
input returns; `run-verify.sh` reports it as covers `never`, and `RETURNS` otherwise). A panic
the section calls debug-only is proven in the debug build, and the release build's behaviour
on the same inputs separately (`r_*`, `make verify-release`, built without debug assertions:
no panic and no arithmetic overflow). Every-build panics are proven in both builds. The
`n_*` harnesses run in the per-change `CellId` leg; `make verify-release` runs the release
pass.

| Operations | Outside the domain | Build | Harness | Result |
|------------|--------------------|-------|---------|--------|
| `impl From<u8> for Level`, and every operation taking `impl Into<Level>`: `from_face_pos_level`, `child_position`, `parent_at_level`, `child_begin_at_level`, `child_end_at_level`, `begin`, `end`, `lsb_for_level`, `size_ij`, `size_st`, `vertex_neighbors`, `all_neighbors` | a raw level above 30 | every | `crash::n_raw_level` | panics (both builds) |
| `level` | 62 or more trailing zero bits (`0`, `1 << 62`, `1 << 63`, `3 << 62`) | every | `crash::n_level` | panics (both builds) |
| `child_position` | an invalid id with any level, or a valid cell with a level outside `1..=level` | debug | `crash::n_child_position`; release `crash::r_child_position` | panics in debug builds; returns in release builds |
| `parent` | a face cell | debug | `crash::n_parent`; release `crash::r_parent` | panics in debug builds; returns in release builds |
| `children`, `child_begin`, `child_end` | an invalid id or a leaf | debug | `crash::n_children`; release `crash::r_children` | panics in debug builds; returns in release builds, except that `child_end`'s sum wraps for the ids within 5/4 of their lowest set bit of 2^64 (`sentinel` among them), which Kani checks as an overflow, so those ids are outside the release proof |
| `next_wrap`, `prev_wrap` | an invalid id | debug | `crash::n_wrap`; release `crash::r_wrap` | panics in debug builds; returns in release builds |
| `vertex_neighbors` | level 30 | every | `crash::n_vertex_neighbors` | panics (both builds) |
| `common_ancestor_level` | (`none`, `none`) | every | `crash::n_common_ancestor_level` | panics (both builds) |
| `from_face_ij` | `i` or `j` outside `[0, 2^30)` | debug | none: not every such input panics | the `# Panics` said it panics; the first `n_from_face_ij` returned (36 of 48 sampled inputs return, many as valid cells: `i = 2^30` on face 0 gives face 1's first leaf). The section now says that some such inputs panic and the others give an unspecified id |

Two harness lessons: a symbolic panic condition (`raw > 30`) is not simplified to false, so
CBMC explores the code after the panic on paths no input takes, and `n_raw_level` needs a small
unwind bound for the neighbour operations' loops (10 minutes without it, 1.4 s with it); and
the domain of the invalid ids is `spec::valid_by_marker`, the model's validity without its
level search, which `tests/spec_vs_cpp.rs` checks against `spec::valid`.

## Outside FR-002's definition

- `s2::ij_level_to_bound_uv` and `s2::expanded_by_distance_uv`: floating-point (u,v)
  helpers, not `CellId` constructors or level constants.
- The derived impls `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`
  (and the `serde` derives): compiler-generated.
