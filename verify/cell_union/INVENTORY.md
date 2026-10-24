# Operation Inventory: `s2rst::s2::CellUnion` (FR-002)

Every inherent public operation of `CellUnion` and its hand-written trait implementations:
**42 rows** (29 inherent methods, 13 trait methods), checked against
`core/src/s2/cell_union.rs` and `core/src/s2/encoding.rs`. Field definitions:
`specs/002-formal-verify-cell-union/data-model.md` §3 (which extends 001's §3). Domain
classes: data-model §2.

Each in-scope row has a crash-freedom obligation **C-CellUnion::&lt;op&gt;**: "*op* completes
without panic, arithmetic overflow or undefined behaviour for every input in *domain* within
*bounds*, assuming *preconditions*" (FR-003).

**Domain rule** (spec FR-003, Edge Cases). A union argument ranges over one of four domains:
- `Seq`: any sequence of ids, for operations documented to accept any sequence;
- `ValidSeq`: sequences of valid cells, for the normalising operations (upstream
  `Normalize` requires valid ids);
- `ValidUnion`: valid unions (sorted, non-overlapping, valid cells), for every operation on
  an existing union whose upstream contract rests on the class invariant (`FromVerbatim`
  REQUIRES `IsValid()`, `s2cell_union.h:142`);
- `NormUnion`: normalised unions, used only for the "normalised when the inputs are"
  claims (FR-004).

A cell argument follows 001 (`Valid`). Other arguments range over their whole type, narrowed
only by a documented precondition: rustdoc, `debug_assert!`, or upstream `DCHECK`/`REQUIRES`.
Every bound is a concrete-length family (`__n<k>`, `__b<L>`), so "`n ≤ 3`" means one harness
per length 0…3 (research R4).

**Summary**: 27 in scope (64.3 %, SC-002 floor 60 %), 15 excluded (9 `float-geometry`, 1
`excluded: depends on CellId::all_neighbors`, 5 `tool-limit`). Every in-scope row is proven
crash-free: 2 over the whole domain (`new`, `whole_sphere`) and 25 within the stated
bounds, which all meet or exceed SC-001's minimums. The bounds count the weekly slow tier
(`make verify-slow F=union`), which runs the largest members; the inventory marks them.

**Shared harnesses** (data-model §3). A property harness runs its operation with every Kani
check on, so a panic, overflow or undefined behaviour fails it like a crash harness. Where a
per-change property member covers an operation over the same domain and size, it discharges
that crash obligation, and the crash family keeps only the larger members. That removed 50
duplicate harnesses from the per-change run (T046), whose fixed cost per harness dominated.

**Debug-only checks** (spec Edge Cases). Where a precondition is guarded by a
`debug_assert!` (a valid cell id, `level_mod` in 1–3, the range constructors' bounds), the
harness assumes the guarded condition, so no proof relies on the check firing in either
build.

## 1. Status, domain and crash proof

| # | name | kind | status | exclusion_reason | union_domain | domain | preconditions (source) | crash_harness | bounds | crash_status | covers |
|---|------|------|--------|------------------|--------------|--------|------------------------|---------------|--------|--------------|--------|
| 1 | `CellUnion::new` | constructor | in-scope |  | — | — | — | `cu_acc::p_uacc_01__n0` (shared) | whole domain (no argument) | proven | 1/1 (`__n0`) |
| 2 | `CellUnion::from_cell_ids` | constructor | in-scope |  | `ValidSeq` | `ValidSeq(n)` | every id valid (upstream `Normalize`; not in rustdoc) | `cu_norm::p_unorm_01__n{0,1}` (shared) and `cu_crash::c_from_cell_ids__n2` | `n ≤ 2`. Unreached: the sibling merge and its cascades (4 cells; 3 cells cost about 14 GB, R0). Exercised by `normalize_matches_cpp` (C++ vectors) and `prop_cell_union_normalize_preserves_coverage` (up to 20 cells) | proven-within-bounds | 6/6 (3 harnesses) |
| 3 | `CellUnion::from_verbatim` | constructor | in-scope |  | `Seq` | `Seq(n)` | — | `cu_acc::p_uacc_01__n{0..=3}` (shared) | `n ≤ 3`; longer unions: unit tests and every other proof, which build their inputs through `from_verbatim` | proven-within-bounds | 7/7 |
| 4 | `CellUnion::from_range` | constructor | excluded | tool-limit: since the D-002 fix it calls `from_begin_end` (row 18), so it inherits that measurement. Deprecated in favour of `from_begin_end` on 2026-10-11 | — | `LeafOrEnd` × `LeafOrEnd`, `b ≤ e` | half-open leaf range (rustdoc); leaves or `end(30)`, `begin ≤ end` through `from_begin_end`'s `debug_assert!`s |  | covered instead by `prop_cell_union_ranges_cover_exactly` and `CELLUNION_FROM_BEGIN_END` (`from_range_matches_cpp`) |  |  |
| 5 | `CellUnion::num_cells` | access | in-scope |  | `Seq` | `Seq(n)` | — | `cu_acc::p_uacc_01__n{0..=3}` (shared) | `n ≤ 3`; longer unions: unit tests and every other proof, which build their inputs through `from_verbatim` | proven-within-bounds | 7/7 |
| 6 | `CellUnion::cell_ids` | access | in-scope |  | `Seq` | `Seq(n)` | — | `cu_acc::p_uacc_01__n{0..=3}` (shared) | `n ≤ 3`; longer unions: unit tests and every other proof, which build their inputs through `from_verbatim` | proven-within-bounds | 7/7 |
| 7 | `CellUnion::is_valid` | query | in-scope |  | `Seq` | `Seq(n)` | — | `cu_inv::p_uinv_01__n{0..=4}` (shared; `__n3`, `__n4` weekly) | `n ≤ 4`. Longer sequences: `cell_union.rs` unit tests and the property tests, which check `is_normalized` on every result | proven-within-bounds | 18/18 |
| 8 | `CellUnion::is_normalized` | query | in-scope |  | `Seq` | `Seq(n)` | — | `cu_inv::p_uinv_02__n{0..=4}` (shared; `__n3`, `__n4` weekly) | `n ≤ 4`, which reaches the sibling check (covered). Longer sequences: as `is_valid` | proven-within-bounds | 19/19 |
| 9 | `CellUnion::normalize` | transform | in-scope |  | `ValidSeq` | `ValidSeq(n)` | every id valid (upstream `Normalize`) | `cu_norm::p_unorm_01__n{0,1}` (shared) and `cu_crash::c_normalize__n2` | `n ≤ 2`. Unreached: the sibling merge and its cascades; exercised as for `from_cell_ids` | proven-within-bounds | 6/6 (3 harnesses) |
| 10 | `CellUnion::contains_cell_id` | query | in-scope |  | `ValidUnion` | `ValidUnion(n)` × `Valid` | `id` valid (`debug_assert!`) | `cu_mem::p_umem_01__n{0,1}` (shared) and `cu_crash::c_contains_cell_id__n{2,3}` | `n ≤ 3`. Longer unions: `prop_cell_union_queries_match_covered_sets` (up to 20 cells) | proven-within-bounds | 8/8 (4 harnesses) |
| 11 | `CellUnion::intersects_cell_id` | query | in-scope |  | `ValidUnion` | `ValidUnion(n)` × `Valid` | `id` valid (`debug_assert!`) | `cu_mem::p_umem_02__n{0,1}` (shared) and `cu_crash::c_intersects_cell_id__n{2,3}` | `n ≤ 3`; longer unions as `contains_cell_id` | proven-within-bounds | 8/8 (4 harnesses) |
| 12 | `CellUnion::contains_point` | query | excluded | float-geometry |  |  |  |  |  |  |  |
| 13 | `CellUnion::contains_union` | query | in-scope |  | `ValidUnion` | `ValidUnion(a)` × `ValidUnion(b)` | — | `cu_mem::p_umem_04__n{a}_n{b}`, `a, b ≤ 1` (shared), and `cu_crash::c_contains_union__n{a}_n{b}` for the 12 pairs with a union of 2 or 3 cells | `3 × 3` (raised from the planned `2 × 2`, T020). Larger: `set_ops_match_cpp` and `prop_cell_union_queries_match_covered_sets` | proven-within-bounds | 28/28 (16 harnesses) |
| 14 | `CellUnion::intersects_union` | query | in-scope |  | `ValidUnion` | `ValidUnion(a)` × `ValidUnion(b)` | — | `cu_mem::p_umem_04__n{a}_n{b}`, `a, b ≤ 1` (shared), and `cu_crash::c_intersects_union__n{a}_n{b}` for the 12 pairs with a union of 2 or 3 cells | `3 × 3`; larger as `contains_union` | proven-within-bounds | 28/28 (16 harnesses) |
| 15 | `CellUnion::leaf_cells_covered` | measure | in-scope |  | `ValidUnion` | `ValidUnion(n)` | the union is valid (upstream `LeafCellsCovered` bound and `FromVerbatim` REQUIRES; FR-016) | `cu_crash::c_leaf_cells_covered__n{0..=3}` | `n ≤ 3`; any size by the written argument (disjoint cells, `6·4^30 < 2^63`; FR-016). `leaf_cells_covered_matches_cpp` checks larger unions | proven-within-bounds | 7/7 |
| 16 | `CellUnion::whole_sphere` | constructor | in-scope |  | — | — | — | `cu_range::p_urange_04` (shared) | whole domain (a constant) | proven | 1/1 (1 harness) |
| 17 | `CellUnion::from_min_max` | constructor | excluded | tool-limit: calls `from_begin_end` (row 18), so it inherits that measurement | — | `Leaf` × `Leaf`, `b ≤ e` | leaves, `min ≤ max` (upstream `FromMinMax` REQUIRES; `debug_assert!`) |  | covered instead by `prop_cell_union_ranges_cover_exactly` and `CELLUNION_FROM_BEGIN_END` |  |  |
| 18 | `CellUnion::from_begin_end` | constructor | excluded | tool-limit: out of memory at 6 GB in 123 s for a range inside one level-29 cell (unwind 32), and at 5.7 GB in 43 s with a per-loop unwind of 9 (R0) | — | `LeafOrEnd` × `LeafOrEnd`, `b ≤ e` | leaves or `end(30)`, `begin ≤ end` (upstream `FromBeginEnd` REQUIRES; `debug_assert!`) |  | covered instead by `prop_cell_union_ranges_cover_exactly` and `CELLUNION_FROM_BEGIN_END` |  |  |
| 19 | `CellUnion::is_empty` | access | in-scope |  | `Seq` | `Seq(n)` | — | `cu_acc::p_uacc_01__n{0..=3}` (shared) | `n ≤ 3`; longer unions: unit tests and every other proof, which build their inputs through `from_verbatim` | proven-within-bounds | 7/7 |
| 20 | `CellUnion::union` | set-op | in-scope |  | `ValidUnion` | `ValidUnion(a)` × `ValidUnion(b)` | — | `cu_set::p_uset_01__n{0_n0,0_n1,1_n0}` (shared) and `cu_crash::c_union__n1_n1` | `1 × 1` (`2 × 1` normalises three cells, about 14 GB). Unreached: merges. Exercised by `set_ops_match_cpp` and `prop_cell_union_set_ops_match_covered_sets` | proven-within-bounds | 5/5 (4 harnesses) |
| 21 | `CellUnion::intersection` | set-op | in-scope |  | `ValidUnion` | `ValidUnion(a)` × `ValidUnion(b)` | — | `cu_set::p_uset_02__n{0_n0,0_n1,1_n0}` (shared) and `cu_crash::c_intersection__n1_n1` (weekly) | `1 × 1`. Unreached: interleavings of more than one cell per side; exercised as for `union` | proven-within-bounds | 5/5 (4 harnesses) |
| 22 | `CellUnion::intersection_with_cell_id` | set-op | in-scope |  | `ValidUnion` | `ValidUnion(n)` × `Valid` | `id` valid (via `contains_cell_id`'s `debug_assert!`) | `cu_set::p_uset_03__n{0..=2}` (shared) and `cu_crash::c_intersection_with_cell_id__n3` | `n ≤ 3` (raised from the planned 2, T020). Longer: `prop_cell_union_queries_match_covered_sets` | proven-within-bounds | 6/6 (4 harnesses) |
| 23 | `CellUnion::difference` | set-op | excluded | tool-limit: over 5 min at `1 × 1` cells in a one-level window (unwind 3), and out of memory at 6 GB in 156 s in a two-level window (R0) | — | `ValidUnion` × `ValidUnion` | — |  | covered instead by `prop_cell_union_set_ops_match_covered_sets` and `CELLUNION_SETOPS` |  |  |
| 24 | `CellUnion::expand_at_level` | transform | excluded | excluded: depends on `CellId::all_neighbors` (001) |  |  |  |  |  |  |  |
| 25 | `CellUnion::expand_by_radius` | transform | excluded | float-geometry |  |  |  |  |  |  |  |
| 26 | `CellUnion::average_based_area` | measure | excluded | float-geometry |  |  |  |  |  |  |  |
| 27 | `CellUnion::approx_area` | measure | excluded | float-geometry |  |  |  |  |  |  |  |
| 28 | `CellUnion::exact_area` | measure | excluded | float-geometry |  |  |  |  |  |  |  |
| 29 | `CellUnion::denormalize` | transform | in-scope |  | `ValidUnion` | `ValidUnion(n)` × `Level` × `LevelMod` | `level_mod` 1–3 (`debug_assert!`, upstream `DCHECK`) | `cu_den::p_uden_01__n0_ids` (shared) and `cu_crash::c_denormalize__n{1,2}` (`__n2` weekly) | `n ≤ 2` (raised from the planned 1, T020); loop bound: a level gap of at most 2 (16 descendants per cell). Unreached: more cells, gaps 3–30. Exercised by `denormalize_clamps_to_max_level`, `denormalize_matches_cpp` and `prop_cell_union_denormalize_preserves_coverage` | proven-within-bounds | 7/7 (3 harnesses) |
| 30 | `impl Deref for CellUnion` | trait | in-scope |  | `Seq` | `Seq(n)` | — | `cu_acc::p_uacc_01__n{0..=3}` (shared) | `n ≤ 3`; longer unions: unit tests and every other proof, which build their inputs through `from_verbatim` | proven-within-bounds | 7/7 |
| 31 | `impl Display for CellUnion` | trait | excluded | tool-limit: goes through `CellId::to_token`, over 5 min in 001 (001 R6) |  |  |  |  |  |  |  |
| 32 | `impl FromIterator<CellId> for CellUnion` | trait | in-scope |  | `ValidSeq` | `ValidSeq(n)` | every id valid (upstream `Normalize`) | `cu_norm::p_unorm_01__n{0,1}` (shared) and `cu_crash::c_from_iter__n2` | `n ≤ 2`. Unreached: the sibling merge; exercised as for `from_cell_ids` | proven-within-bounds | 6/6 (3 harnesses) |
| 33 | `impl IntoIterator for CellUnion` | trait | in-scope |  | `Seq` | `Seq(n)` | — | `cu_acc::p_uacc_01__n{0..=3}` (shared) | `n ≤ 3`; longer unions: unit tests and every other proof, which build their inputs through `from_verbatim` | proven-within-bounds | 7/7 |
| 34 | `impl IntoIterator for &CellUnion` | trait | in-scope |  | `Seq` | `Seq(n)` | — | `cu_acc::p_uacc_01__n{0..=3}` (shared) | `n ≤ 3`; longer unions: unit tests and every other proof, which build their inputs through `from_verbatim` | proven-within-bounds | 7/7 |
| 35 | `Region::cap_bound` | trait | excluded | float-geometry |  |  |  |  |  |  |  |
| 36 | `Region::rect_bound` | trait | excluded | float-geometry |  |  |  |  |  |  |  |
| 37 | `Region::cell_union_bound` | trait | excluded | float-geometry (through `cap_bound`) |  |  |  |  |  |  |  |
| 38 | `Region::contains_cell` | trait | in-scope |  | `ValidUnion` | `ValidUnion(n)` × `Cell` of a `Valid` id | the cell's id valid (via `contains_cell_id`) | `cu_mem::p_umem_05__n{0,1}` (shared) and `cu_crash::c_region_contains_cell__n{2,3}` | `n ≤ 3` (building the `Cell` needs an unwind of 9, T020) | proven-within-bounds | 8/8 (4 harnesses) |
| 39 | `Region::intersects_cell` | trait | in-scope |  | `ValidUnion` | `ValidUnion(n)` × `Cell` of a `Valid` id | the cell's id valid (via `intersects_cell_id`) | `cu_mem::p_umem_05__n{0,1}` (shared) and `cu_crash::c_region_intersects_cell__n{2,3}` | `n ≤ 3` | proven-within-bounds | 8/8 (4 harnesses) |
| 40 | `Region::contains_point` | trait | excluded | float-geometry |  |  |  |  |  |  |  |
| 41 | `impl S2Encode for CellUnion` | encoding | in-scope |  | `Seq` | `Seq(n)` | — | `cu_enc::p_uenc_01__n{0..=3}` (shared) | `n ≤ 3`. Longer: `encode_matches_cpp` and every decode round trip | proven-within-bounds | 5/5 |
| 42 | `impl S2Decode for CellUnion` | encoding | in-scope |  | — | `Bytes(L)` | — | `cu_enc::p_uenc_03__b{0..=16}` (shared) and `cu_enc::p_uenc_04__b{17..=24}`; `__b25` in the weekly slow tier (8 GB) | `L ≤ 24` per change (one cell), `L = 25` weekly (two cells, which reaches the FR-017 order check). Longer inputs: `decoder_robustness`, `decode_matches_cpp`, the nightly `decode_cell_union` fuzz target | proven-within-bounds | 43/43 (26 harnesses, `__b25` weekly) |

## 2. Properties, result validity, panics and upstream findings

`undocumented_panic` records a precondition the proof had to assume although the Rust docs
do not state it (FR-003). Each one gets a `# Panics` section in the documentation-only commit
(research R14, `panics_doc`). That commit covers every public function, excluded ones too
(Constitution II).

| # | name | properties | result_validity (FR-004) | undocumented_panic | panics_doc | upstream_findings |
|---|------|------------|--------------------------|--------------------|------------|-------------------|
| 1 | `CellUnion::new` | P-UACC-01 | the empty union (P-UACC-01) |  | — (cannot panic) |  |
| 2 | `CellUnion::from_cell_ids` | P-UNORM-01, P-UNORM-02, P-UNORM-04 | normalised: P-UNORM-01 | an invalid id panics in debug builds (`range_min`/`range_max` underflow on id 0; the final `debug_assert!(is_normalized())`); the proof assumes `ValidSeq`, from upstream `Normalize` | added (T030) |  |
| 3 | `CellUnion::from_verbatim` | P-UACC-01 | none: stores its input (FR-016 documents the valid-union requirement) | — (stores its input; FR-016 documents the valid-union requirement) | added (T030): the FR-016 requirement |  |
| 4 | `CellUnion::from_range` | P-URANGE-03 | not proven (tool limit); normalised in `from_begin_end_matches_cpp` (C++) and `prop_cell_union_ranges_cover_exactly` | excluded row: in debug builds, a non-leaf bound or `begin > end` | added (T030) | D-002 (defect, fixed) |
| 5 | `CellUnion::num_cells` | P-UACC-01 | — (returns no union) |  | — (cannot panic) |  |
| 6 | `CellUnion::cell_ids` | P-UACC-01 | — (returns no union) |  | — (cannot panic) |  |
| 7 | `CellUnion::is_valid` | P-UINV-01 | — (returns no union) |  | — (cannot panic) |  |
| 8 | `CellUnion::is_normalized` | P-UINV-02 | — (returns no union) |  | — (cannot panic) |  |
| 9 | `CellUnion::normalize` | P-UNORM-01, P-UNORM-02, P-UNORM-03, P-UNORM-04 | normalised: P-UNORM-01 | as `from_cell_ids` | added (T030) |  |
| 10 | `CellUnion::contains_cell_id` | P-UMEM-01, P-UMEM-03 | — (returns no union) | in debug builds: an invalid `id` (`debug_assert!`, not in rustdoc before), or an invalid union (id 0 underflows in `range_max`) | added (T030) |  |
| 11 | `CellUnion::intersects_cell_id` | P-UMEM-02 | — (returns no union) | as `contains_cell_id` | added (T030) |  |
| 12 | `CellUnion::contains_point` | — | — (returns no union) | excluded row: in debug builds, an invalid union | added (T030) |  |
| 13 | `CellUnion::contains_union` | P-UMEM-04 | — (returns no union) | in debug builds, an invalid id in either union | added (T030) |  |
| 14 | `CellUnion::intersects_union` | P-UMEM-04 | — (returns no union) | as `contains_union` | added (T030) |  |
| 15 | `CellUnion::leaf_cells_covered` | P-UMEAS-01 | — (returns no union) | an id without a level marker panics in all builds (`CellId::level`); in debug builds the `i64` sum overflows for eight or more overlapping face cells (FR-016) | added (T030), with the FR-016 precondition and deviation note |  |
| 16 | `CellUnion::whole_sphere` | P-URANGE-04 | normalised: P-URANGE-04 (the six faces in order) |  | — (cannot panic) |  |
| 17 | `CellUnion::from_min_max` | P-URANGE-02 | not proven (tool limit); as `from_range` | excluded row: in debug builds, a non-leaf bound or `min > max` | added (T030) |  |
| 18 | `CellUnion::from_begin_end` | P-URANGE-01 | not proven (tool limit); normalised in `from_begin_end_matches_cpp` and `prop_cell_union_ranges_cover_exactly` | excluded row: in debug builds, a non-leaf bound or `begin > end` | added (T030) |  |
| 19 | `CellUnion::is_empty` | P-UACC-01 | — (returns no union) |  | — (cannot panic) |  |
| 20 | `CellUnion::union` | P-USET-01 | normalised: P-USET-01 | as `from_cell_ids`, for an invalid id in either union | added (T030) |  |
| 21 | `CellUnion::intersection` | P-USET-02 | normalised when the inputs are: P-USET-02 | in debug builds, an invalid id in either union | added (T030) |  |
| 22 | `CellUnion::intersection_with_cell_id` | P-USET-03 | normalised when the union is: P-USET-03 | as `contains_cell_id` | added (T030) |  |
| 23 | `CellUnion::difference` | P-USET-04 | not proven (tool limit); normalised when the first input is, in `set_ops_match_cpp` and `prop_cell_union_set_ops_match_covered_sets` | excluded row: in debug builds, an invalid union | added (T030) |  |
| 24 | `CellUnion::expand_at_level` | — | — (returns no union) | excluded row: some invalid ids in all builds, others in debug builds | added (T030) |  |
| 25 | `CellUnion::expand_by_radius` | — | — (returns no union) | excluded row: an id without a level marker in all builds | added (T030) |  |
| 26 | `CellUnion::average_based_area` | — | — (returns no union) | excluded row: as `leaf_cells_covered` | added (T030) |  |
| 27 | `CellUnion::approx_area` | — | — (returns no union) | excluded row: an invalid id, in all builds | added (T030) |  |
| 28 | `CellUnion::exact_area` | — | — (returns no union) | excluded row: an invalid id, in all builds | added (T030) |  |
| 29 | `CellUnion::denormalize` | P-UDEN-01, P-UDEN-02 | valid: P-UDEN-01 (`p_uden_01__n*_valid`) | in debug builds, `level_mod` outside 1–3 (`debug_assert!`) and any invalid id; in all builds, an id without a level marker | added (T030) |  |
| 30 | `impl Deref for CellUnion` | P-UACC-01 | — (returns no union) |  | — (cannot panic) |  |
| 31 | `impl Display for CellUnion` | — | — (returns no union) |  | — (cannot panic) |  |
| 32 | `impl FromIterator<CellId> for CellUnion` | P-UNORM-01, P-UNORM-02 | normalised: P-UNORM-01 | as `from_cell_ids` | added (T030, on the impl) |  |
| 33 | `impl IntoIterator for CellUnion` | P-UACC-01 | — (returns no union) |  | — (cannot panic) |  |
| 34 | `impl IntoIterator for &CellUnion` | P-UACC-01 | — (returns no union) |  | — (cannot panic) |  |
| 35 | `Region::cap_bound` | — | — (returns no union) | excluded row: an invalid id, in all builds | added (T030, on the impl) |  |
| 36 | `Region::rect_bound` | — | — (returns no union) | excluded row: an invalid id, in all builds | added (T030, on the impl) |  |
| 37 | `Region::cell_union_bound` | — | — (returns no union) | excluded row: an invalid id, in all builds | added (T030, on the impl) |  |
| 38 | `Region::contains_cell` | P-UMEM-05 | — (returns no union) | in debug builds, an invalid union | added (T030, on the impl) |  |
| 39 | `Region::intersects_cell` | P-UMEM-05 | — (returns no union) | in debug builds, an invalid union | added (T030, on the impl) |  |
| 40 | `Region::contains_point` | — | — (returns no union) | excluded row: in debug builds, an invalid union | added (T030, on the impl) |  |
| 41 | `impl S2Encode for CellUnion` | P-UENC-01, P-UENC-02 | — (returns no union) |  | — (cannot panic) |  |
| 42 | `impl S2Decode for CellUnion` | P-UENC-02, P-UENC-03, P-UENC-04 | a valid union: P-UENC-03 (exactly the encodings of valid unions are accepted, FR-017) |  | — (cannot panic) | U-003 |

## Outside FR-002's definition

- The derived impls `Clone`, `Debug`, `Default`, `PartialEq`, `Eq` and `Hash`, and the
  optional `serde` derive of `Serialize`: compiler-generated (spec Assumptions). `Default` is
  still checked alongside `new` in `cu_acc::p_uacc_01__n0`.
- The optional `serde` `Deserialize`, hand-written since 2026-10-11, after this inventory:
  it reads the derived form and rejects ids that are not a valid union through `is_valid`
  (row 7), as `S2Decode` does (row 42). It is covered by unit tests
  (`serde_rejects_invalid_unions`, `prop_serde_roundtrip`), not by a proof.
