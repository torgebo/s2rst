# Evaluation: Formal Verification of `CellUnion` (FR-014)

Spec: `specs/002-formal-verify-cell-union/`. Records: [INVENTORY.md](INVENTORY.md),
[PROPERTIES.md](PROPERTIES.md), [../MUTANTS.md](../MUTANTS.md),
[../TRUSTED_BASE.md](../TRUSTED_BASE.md). The 001 evaluation is
[../EVALUATION.md](../EVALUATION.md).

## Summary

The second verification feature applied the pilot's method (Kani, an independent model,
mutants, CI) to `s2rst::s2::CellUnion`, a sorted collection of cells. Its length is
unbounded, so every proof holds only up to a stated size.

- **Crash-freedom:** all 27 in-scope operations are proven free of panics, overflow and
  undefined behaviour ([`INVENTORY.md`](INVENTORY.md)): 2 over their whole domain, 25 within
  union-size or byte-length bounds. Read-only operations reach 3 to 4 cells, operations
  that build a union by normalising reach 2, and the decoder is crash-free on every byte
  string of up to 25 bytes (its functional property reaches 24). 15 operations are out of scope: 10 by design (floating-point geometry, and one
  that depends on an operation 001 excluded) and 5 tool limits, `difference` and the three
  range constructors among them.
- **Correctness:** all 23 catalogue properties whose operations are in scope are proven,
  22 of them within bounds, against an independent model of the leaf cells a union covers
  ([`PROPERTIES.md`](PROPERTIES.md)). Every proven property has a mutant that breaks it: 25
  mutants, all killed ([`../MUTANTS.md`](../MUTANTS.md)).
- **Cost:** the per-change proofs take 7.9 minutes on 4 CPUs (20.9 CPU-minutes). The larger
  property members and three crash members, 45 harnesses, run in the weekly tier.
- **Findings:** the C++ parity tests the feature added found a serious new defect, D-002.
  `CellUnion::from_range` was wrong on 301 of 307 reference ranges, and through it
  `ShapeIndex` built indexes with gaps and overlapping cells, so a point-in-polygon query
  could answer wrongly. It is fixed, as are the three defects found before the feature
  (FR-015 `denormalize` panic, FR-016 undocumented leaf-count precondition, FR-017
  decoding). The proofs led to `# Panics` documentation on 23 items, and to one
  upstream finding: C++ decodes unions that break its own invariant (U-003).

**Recommendation:** continue with Kani for integer code, including collections, but plan
on small bounds and pair every collection proof with differential and property tests. That
pairing found D-002; the proofs alone would not have. Next: the integer coding layer of the
index encodings ("Recommendations" below).

## Effort log

| Date | Phase | Person-hours | Notes |
|------|-------|--------------|-------|
| 2026-10-02 | Specification, clarification, planning (incl. the memory-capped Kani probe, R0) | — | Not separately timed |
| 2026-10-03 | Phase 1 setup (`cellunion` feature, Makefile targets, runner) and Phase 2 (model, generators, C++ vectors, inventory) | — | |
| 2026-10-03 | Phase 3: parity tests, D-002 (found by them, fixed with the user's go-ahead), FR-015, FR-017, R17 property tests | — | Five commits, each with `make test` passing |
| 2026-10-03 – 2026-10-04 | Phase 4 (US1): bound measurements, crash families, `# Panics` docs, M-100 | — | Most time went to unwinding and memory issues in the harnesses (see "Notes from implementation") |
| 2026-10-04 | Phase 5 (US2): property families, the cost-trap fixes, 25 mutants | — | The first property run failed on memory for 30 of 93 harnesses; about half a day of re-runs |
| 2026-10-04 | Phase 6–8 (US3, US4, polish): CI legs, the per-change budget (tier split, shared harnesses), weekly run, evaluation, quickstart | — | |

**Who did the work:** as in 001, an AI coding agent wrote the harnesses, scripts and
documents. The maintainer made the decisions: the R0 bounds and exclusions, the FR-017
behaviour, and fixing D-002 within the feature. Person-hours were not timed. **Machine time**
dominated: one full property run took about an hour of CPU time, and the mutant campaign
about 50 minutes of wall-clock time.

## Exclusions and tool limits (FR-002)

Measured in the 2026-10-02 planning probe, each harness alone in a memory-capped scope; the
full table is research R0.

| Operation | Reason | Measurement |
|-----------|--------|-------------|
| 9 floating-point operations (`contains_point`, `expand_by_radius`, `average_based_area`, `approx_area`, `exact_area`, `Region::cap_bound`, `Region::rect_bound`, `Region::cell_union_bound`, `Region::contains_point`) | `float-geometry` (FR-002 default) | — |
| `expand_at_level` | excluded: depends on `CellId::all_neighbors`, which 001 found intractable | — |
| `Display` | `tool-limit`: `CellId::to_token` | over 5 min in 001 (001 R6) |
| `difference` | `tool-limit`: recursion into children, four ways per level | over 5 min at `1 × 1` cells in a one-level window (unwind 3); out of memory at 6 GB in 156 s in a two-level window |
| `from_begin_end` | `tool-limit`: `maximum_tile`'s 30-level loop per output cell | out of memory at 6 GB in 123 s for a range inside one level-29 cell (unwind 32); out of memory at 5.7 GB in 43 s with a per-loop unwind of 9 |
| `from_min_max` | `tool-limit`: calls `from_begin_end` | inherits `from_begin_end`'s measurement |
| `from_range` | `tool-limit`: since the D-002 fix it calls `from_begin_end` | inherits `from_begin_end`'s measurement |

## Bounds reached

Per operation group, the largest size each kind of proof reaches (per change, or weekly in
the slow tier). `INVENTORY.md` and `PROPERTIES.md` give each row's exact family, and the
behaviour each bound leaves unreached with the tests that exercise it.

| Operations | Crash-freedom | Properties | Cost of the largest member (alone, workstation) |
|------------|---------------|------------|-------------------------------------------------|
| Accessors, `new`, `from_verbatim`, encode | 3 cells | 3 cells | a few seconds |
| `is_valid`, `is_normalized` | 4 ids (a sibling group) | 4 ids | 48 s at 4 ids |
| Cell queries, `Region` cell queries | 3 cells | 3 cells | 18 s (crash), 153 s (P-UMEM-05) |
| Union queries | `3 × 3` | `3 × 3` | 9 s (crash), 396 s (P-UMEM-04, in parallel) |
| `leaf_cells_covered` | 3 cells; any size by argument | 3 cells | a few seconds |
| `normalize`, `from_cell_ids`, `FromIterator` | 2 cells | 2 cells | 16 s; 3 cells about 14 GB (R0) |
| `union`, `intersection` | `1 × 1` | `1 × 1` | 13 s / 25 s (crash), 90 s (P-USET-02) |
| `intersection_with_cell_id` | 3 cells | 2 cells | 8 s (crash), 20 s (property) |
| `denormalize` | 2 cells, gap ≤ 2 | 1 cell, gap ≤ 2 | 64 s at 2 cells |
| Decoding | 25 bytes | 24 bytes (round trip: 4 cells) | 92 s at 7.8 GB for 25 bytes |
| `whole_sphere`, `new` | whole domain | whole domain | seconds |
| `difference`, range constructors | none (tool limit) | none | over the budget at 1 cell (R0) |

## Upstream findings (FR-019, SC-012)

| Id | Upstream | Inputs | Observed upstream | Rust | Follow-up |
|----|----------|--------|-------------------|------|-----------|
| U-003 | C++ `S2CellUnion::Decode` (`s2cell_union.cc:544–562`) with `S2CellId::Decode` (`s2cell_id.cc:261–265`): neither validates the ids, although `FromVerbatim` REQUIRES `IsValid()` (`s2cell_union.h:142`) | version 1, count 2, ids `[0x3000000000000000, 0x1000000000000000]`; or count 1 with id `0` | `Decode` returns `true` and a union whose `IsValid()` is `false`. The `CELLUNION_DECODE` vectors hold 10 such rows (invalid, unsorted, duplicate and overlapping ids) | rejected with `InvalidData`, a documented deviation (FR-017) | tasks.md T054 |

## `# Panics` documentation added (SC-011)

One documentation-only commit (`ce53834`, T030) added a `# Panics` section to every public
item of `core/src/s2/cell_union.rs` that can panic, excluded operations included
(Constitution II). Each condition was confirmed with concrete inputs in debug and release
builds. The same commit added a "Validity" section to the type, and the FR-016
documentation of `from_verbatim` and `leaf_cells_covered`. The 23 items:

`from_cell_ids`, `from_range`, `normalize`, `contains_cell_id`, `intersects_cell_id`, `contains_point`, `contains_union`, `intersects_union`, `leaf_cells_covered`, `from_min_max`, `from_begin_end`, `union`, `intersection`, `intersection_with_cell_id`, `difference`, `expand_at_level`, `expand_by_radius`, `average_based_area`, `approx_area`, `exact_area`, `denormalize`, `impl FromIterator<CellId> for CellUnion`, `impl Region for CellUnion`.

Most panic only in debug builds (`debug_assert!`s and overflow checks). `leaf_cells_covered`,
`average_based_area`, `denormalize`, `expand_at_level`, `expand_by_radius`, `approx_area`,
`exact_area` and the `Region` bounds also panic in release builds, on ids without a valid
level marker. No panic differs from upstream in a way that needed a `D-NNN`: C++ `DCHECK`s
the same preconditions or has undefined behaviour there.

## Defects found

Records: `denormalize`, `leaf_cells_covered`, `from_range` and the decoder are rows 29, 15, 4
and 42 of [`INVENTORY.md`](INVENTORY.md); their properties are P-UDEN-01, P-UMEAS-01,
P-URANGE-03 and P-UENC-02/03 in [`PROPERTIES.md`](PROPERTIES.md); their mutants are M-150,
M-100 and M-152 to M-155 in [`../MUTANTS.md`](../MUTANTS.md). U-003's follow-up is tasks.md
T054.

| Id | Defect | Fix | Evidence |
|----|--------|-----|----------|
| FR-015 | `denormalize` panics (`level must be 0..=30`) where C++ clamps to level 30, for example a level-29 cell with `min_level` 1 and `level_mod` 3 (found by the survey before the feature) | `a4ecbb4`: the rounded level is a saturating `u8` add clamped to 30, as C++ (`s2cell_union.cc:215–223`) | `denormalize_clamps_to_max_level` and `denormalize_matches_cpp` (`CELLUNION_DENORMALIZE`, 1,865 rows), both failing without the fix (re-checked 2026-10-04 with M-150 applied); P-UDEN-01 and mutant M-150 |
| FR-016 | `leaf_cells_covered` and `from_verbatim` do not state upstream's valid-union precondition; the `i64` count overflows on an overlapping union of eight face cells (survey) | `ce53834`, documentation only: the precondition, the `# Panics` section, and the `i64` result as a deliberate deviation | P-UMEAS-01 (no overflow up to 3 cells), the written argument for any size, mutant M-100 |
| FR-017 | decoding normalises (four encoded siblings decode as their parent) where C++ keeps the ids verbatim; neither behaviour was documented (survey) | `15e45e6`: the ids are kept as encoded, input that is not a valid union is rejected (a documented deviation; U-003), and the error messages are formatted only when displayed | `cell_union_tests::decode_*` (five of six fail without the fix, re-checked 2026-10-04), `decode_matches_cpp` (`CELLUNION_DECODE`), new robustness seeds, a pytest and a wasm test; P-UENC-02 (four siblings round-trip) and P-UENC-03 (up to 24 bytes) |
| D-002 | `CellUnion::from_range(begin, end)` does not cover `[begin, end)`. It climbs from `begin` to the largest fitting cell and then steps with `next()` at that level, never shrinking near `end`, and it stops by comparing ids of different levels. It is wrong on 301 of the 307 `CELLUNION_FROM_BEGIN_END` rows (141 overshoot past `end`, 160 miss leaves), and 5 leaves give one level-29 cell of 4. `ShapeIndex::skip_cell_range` calls it where C++ `MutableS2ShapeIndex::SkipCellRange` uses `FromBeginEnd`, so built indexes get gaps and overlapping cells. The sphere minus a small hole near a face corner gives 38 cells, 2 of them overlapping, against 63 disjoint cells with `from_begin_end`. `CellIteratorJoin::compute_covering` also calls it, where C++ uses `FromMinMax`. Found by T010's `from_range` parity check on 2026-10-03. (The fix commit, `b52833e`, calls it D-001; it was renumbered because 001's `from_face_ij` defect is already D-001.) The same index answers `ContainsPointQuery` with `false` for a point well inside the polygon, (25°, −90°). | Its own commit (FR-010, the user's decision of 2026-10-03). `from_range` delegates to `from_begin_end`. `ShapeIndex` calls `from_begin_end` and `CellIteratorJoin` calls `from_min_max`, as C++ does. | `from_range_matches_cpp` (C++ vectors); `cell_union_tests::from_range_covers_exactly_the_range`; `shape_index_tests::skipped_ranges_give_disjoint_cells_without_gaps` (fails before the fix on both an overlap and a gap); a pytest and a wasm-bindgen test |

## Measurements

### Baseline: the `CellId` suite alone (`make verify-cellid`, T003)

Workstation: Intel i7-8665U (4 cores, 8 threads), 32 GB, of which about 21 GB was available.
`run-verify.sh` chose `VERIFY_JOBS=5` from available memory. Run alone on 2026-10-03, with the
`cellunion` feature off (`--no-default-features`):

| Harnesses | Result | Wall clock | Sum of harness times | Slowest |
|-----------|--------|------------|----------------------|---------|
| 196 | 196 SUCCESSFUL, all covers satisfied | 582.6 s (9.7 min, including the model's `cargo test`) | 1682.8 s (28.0 min) | `range::p_range_06`, 271.6 s |

This reproduces 001's final result (196 harnesses, all successful). The feature gate leaves
the `CellId` suite unchanged.

### The US1 crash suite (`make verify-union`, T032)

Run alone on 2026-10-04 on the workstation above, with `VERIFY_JOBS=6` chosen from available
memory. Per-harness times are with six harnesses in parallel, so they run above the
alone-times in the next table (for example `c_denormalize__n2`: 139.8 s here, 64.3 s alone).

| Harnesses | Result | Wall clock | Sum of harness times | Slowest |
|-----------|--------|------------|----------------------|---------|
| 120 (the crash families and the shared `cu_acc`, `cu_inv`, `p_uenc_01` and `p_uenc_04` families) | 120 SUCCESSFUL, all covers satisfied | 428.8 s (7.1 min, including the model's `cargo test`) | 1649.2 s (27.5 min) | `cu_crash::c_denormalize__n2`, 139.8 s |

By family (sum of harness times): the decode family `p_uenc_04__b0…b24` 465 s (25 members,
slowest 54 s), `p_uinv_02` 143 s, `c_denormalize` 158 s, `p_uinv_01` 118 s, the two union
queries 106 s and 110 s (16 members each), the two `Region` queries 100 s each. Every other
family is under 50 s. Against R15's budget of 35 CPU-minutes for the whole per-change
`CellUnion` suite, US1 uses 27.5; the US2 properties are measured in T044.

### The US2 property suite (T044)

The first run of the US2 families (2026-10-04) had 30 of 93 harnesses fail, almost all on
memory. The causes and fixes are under "Notes from implementation". After the fixes, the 94
property harnesses ran on the workstation with `VERIFY_JOBS=6`:

| Harnesses | Result | Wall clock | Sum of harness times | Slowest |
|-----------|--------|------------|----------------------|---------|
| 94 (all US2 property families, before the tier split) | 94 SUCCESSFUL, all covers satisfied | 802.0 s (13.4 min) | 3669.6 s (61.2 min) | `cu_mem::p_umem_04__n3_n3`, 396.2 s |

Bounds the properties reach (the crash families' bounds are in the inventory): P-UMEM-04
`3 × 3`; P-UMEM-01/02/03/05 `n ≤ 3`; P-UNORM-01…04 `n ≤ 2`; P-USET-01/02 `1 × 1`; P-USET-03
`n ≤ 2` (3 cells ran out of memory); P-UDEN-01/02 `n ≤ 1`, gap ≤ 2; P-UMEAS-01 `n ≤ 3`;
P-UENC-02 `n ≤ 4` (four siblings included); P-UENC-03 `L ≤ 24`. At 25 bytes (two cells, which
reaches FR-017's order check) P-UENC-03 ran out of memory at 8 GB, both as one harness and
split by the count field; only the crash proof reaches 25 bytes.

### The per-change budget and the weekly tier (SC-005)

US1 and US2 together summed to about 89 CPU-minutes, against R15's 35 for the per-change
`CellUnion` suite. Without a change, the per-change leg would have taken about 20 minutes
on the workstation, against SC-005's 10. As in 001, the members over the budget moved to the
weekly slow tier (FR-012), where their results count like any other proof:
- **Per change (120 harnesses):** every crash family except the largest members of three
  (`c_denormalize__n2`, `c_intersection__n1_n1`, the 25-byte decode), and the smaller
  members of each property family. Where a per-change property member already covers an
  operation over the same domain and size, it discharges the crash obligation as a shared
  harness, and the duplicate crash member was removed (50 harnesses; see "Kani's fixed cost
  per harness" below). The per-change property members are:
  - P-UMEM-04 up to `1 × 1`, and P-UMEM-01/02/03/05 up to 1 cell;
  - P-UNORM up to 1 cell, and P-UDEN at 0 cells;
  - P-USET-01/02 up to `1 × 0`, and P-USET-03 up to 2 cells;
  - P-UINV up to 2 ids;
  - P-UENC-02 up to 3 cells, and P-UENC-03 up to 16 bytes;
  - P-UACC, P-UMEAS, P-URANGE and P-UENC-01 in full.
- **Weekly (`make verify-slow F=union`, 45 more harnesses):** the larger members, up to the
  bounds above, at an 8 GB ceiling and one harness at a time.

**Measurements (T046).** Kani spends a fixed few seconds per harness outside the times it
reports (about 5 s here). Each 4-CPU run below went through `make verify-union`, pinned with
`taskset -c 0-3` in a 16 GB scope as the reference runner's stand-in:

| Per-change set | Harnesses | Result | Wall clock (4 CPUs) | Sum of harness times |
|----------------|-----------|--------|---------------------|----------------------|
| Before the split (8 threads, 6 jobs) | 176 | all SUCCESSFUL | 9.9 min | 37.5 min |
| After moving the larger members to the weekly tier | 175 | all SUCCESSFUL | 10.9 min | 28.6 min |
| Five more members weekly | 170 | all SUCCESSFUL | 10.0 min | 24.5 min |
| Duplicate crash members removed (final) | 120 | all SUCCESSFUL | **7.9 min** | **20.9 min** |

The final per-change leg meets SC-005 (under 10 minutes) and R15's 35 CPU-minutes on the
4-CPU stand-in. A GitHub runner's 4 vCPUs are two cores with hyperthreading, so CI may be
slower; T046's live check measures that.

### The final runs (T046, T052)

On the workstation above, one suite at a time, on 2026-10-04:

| Run | Harnesses | Result | Wall clock | Sum of harness times | Slowest | Peak memory |
|-----|-----------|--------|------------|----------------------|---------|-------------|
| `make verify-union`, pinned to 4 CPUs | 120 | all SUCCESSFUL | 7.9 min | 20.9 min | see the T046 table | under 4 GB per harness |
| `make verify-cellid` (`VERIFY_JOBS=6`) | 196 | all SUCCESSFUL | 10.8 min | — | as 001 | under 4 GB per harness |
| `make verify-slow F=union` (8 GB, one job) | 165 (120 + 45 weekly) | all SUCCESSFUL | 49.4 min | 44.2 min | `cu_mem::p_umem_04__n3_n3`, 198.9 s | 7.8 GB (`cu_enc::p_uenc_04__b25`) |
| The same on 2026-10-09 (60 minutes per harness) | 165 | all SUCCESSFUL | 50.6 min | 44.8 min | `cu_mem::p_umem_04__n3_n3`, 205.5 s | not recorded |

The `CellId` leg is unchanged by the feature and stays within its 15-minute budget. The weekly
`union` leg fits the slow job's 360-minute limit with room to spare. **CI times:** not yet
measured on a hosted runner. The maintainer deferred T046's live check (a draft pull
request) on 2026-10-04.

### Bounds measured first (T020, T021)

Each harness alone through `make verify-harness` (4 GB ceiling unless noted), on the
workstation above. "Time" is Kani's verification time; "peak" is the largest process in the
run, which includes the build.

| Harness | Result | Time | Peak | Decision |
|---------|--------|------|------|----------|
| `cu_enc::p_uenc_04__b17` (decode, 1 cell) | SUCCESSFUL | 24.7 s | 2.6 GB | Decoding stays in scope after the R11 message change (SC-001) |
| `cu_enc::p_uenc_04__b24` | SUCCESSFUL | 25.6 s | 2.7 GB | Per-change family `__b0 … __b24` |
| `cu_enc::p_uenc_04__b25` (2 cells, reaches the FR-017 order check) | OOM at 4 GB in about 38 s; SUCCESSFUL at 8 GB | 91.9 s | 7.8 GB | Weekly slow tier with an 8 GB ceiling |
| `cu_crash::c_region_contains_cell__n0` (cost of building the `Cell`) | SUCCESSFUL, after raising the unwind to 9 for `to_face_ij_orientation` | 15.7 s | 0.6 GB | Both `Region` cell queries stay in scope (SC-002 stays at 64 %) |
| `cu_crash::c_region_contains_cell__n3` | SUCCESSFUL | 17.8 s | 0.7 GB | `n ≤ 3` as planned |
| `cu_crash::c_intersection_with_cell_id__n2` / `__n3` | SUCCESSFUL | 6.1 s / 7.9 s | 0.3 / 0.5 GB | Raised from `n ≤ 2` to `n ≤ 3` |
| `cu_crash::c_contains_union__n2_n2` / `__n3_n3` | SUCCESSFUL | 5.1 s / 8.6 s | 0.3 / 0.4 GB | Raised from `2 × 2` to `3 × 3`, for both union queries |
| `cu_crash::c_union__n1_n1` | SUCCESSFUL | 12.7 s | 0.5 GB | `1 × 1` as planned; `2 × 1` normalises three cells (about 14 GB, R0) |
| `cu_crash::c_intersection__n1_n1` | SUCCESSFUL | 24.7 s | 1.0 GB | `1 × 1` as planned |
| `cu_crash::c_from_cell_ids__n2` | SUCCESSFUL | 15.5 s | 0.5 GB | `n ≤ 2` as planned |
| `cu_crash::c_denormalize__n1` / `__n2` | SUCCESSFUL | 6.6 s / 64.3 s | 0.3 / 2.0 GB | Raised from `n ≤ 1` to `n ≤ 2` |

`from_range` (T021) needed no measurement of its own: since the D-002 fix it calls
`from_begin_end`, so it inherits that tool limit.

Two harness-writing lessons, both from unwinding assertions:
- A cover that calls the model's `spec::decompose` (a 31-level loop) forces an unwind of 32
  on everything else in the harness. Covers only show reachability, so they use the
  implementation's loop-free `CellId::is_valid` instead.
- Building a `Cell` needs an unwind of 9 for `to_face_ij_orientation`'s lookup loop, as in
  001.

## Comparison with 001

| | 001 `CellId` | 002 `CellUnion` |
|---|---|---|
| Inventory in scope | 48 of 64 (75 %) | 27 of 42 (64 %) |
| Crash proofs over the whole domain | 45 of 48 | 2 of 27; 25 within size bounds |
| Properties proven (entries whose operations are in scope) | 42 of 43 (3 within an input bound) | 23 of 23 (22 within size bounds) |
| Harnesses | 196 per change, plus the weekly neighbour tier | 120 per change, plus 45 weekly |
| Per-change CPU time (workstation) | 28 min (baseline run, T003) | 21 min (final, 4 CPUs) |
| Mutants | 38 | 25, all killed |
| New defects | D-001 (debug-build panic, fix deferred) | D-002 (wrong results, `ShapeIndex` corruption; fixed) |
| Known defects fixed in the feature | 2 (FR-015, FR-016) | 3 (FR-015, FR-016, FR-017) |
| Upstream findings | 2 (U-001, U-002: UB) | 1 (U-003: invariant not enforced) |
| `# Panics` sections added | 22 | 23 (21 functions, 2 trait impls) |
| Elapsed time, specification to completion | about 4 days | about 3 days |
| New defects per elapsed day | 0.25 (D-001 in about 4 days) | 0.33 (D-002 in about 3 days), found by tests rather than proofs |

What changed in kind: 001 proved most operations for **every** input, and its
counterexamples were real defects. For collections the proofs are **bounded** (one to four
cells), and the cost grows very steeply with size: normalising takes 16 s at two cells and
about 14 GB at three (R0). The bugs that matter in a collection (merges, cascades,
interleavings) need four or more cells, which the proofs cannot reach. D-002 was found by
the C++ differential tests written for the tool-limit exclusions, not by a proof.

## Limitations

- **Small bounds.** The sibling merge and its cascades (4+ cells), interleavings of more
  than one cell per side in `union` and `intersection`, multi-cell `denormalize` with large
  level gaps, and multi-cell decoding are unreached by the proofs. Unit tests, the C++
  vectors and the R17 property tests exercise them; `INVENTORY.md` names the test for each.
- **Tool-limit exclusions.** `difference` (exponential recursion), the three range
  constructors (`maximum_tile` per output cell) and `Display` (`core::fmt`) have no proofs.
- **The weekly tier.** 42 property harnesses (the larger members) and 3 crash harnesses (the
  25-byte decode, `denormalize` at 2 cells, `intersection` at `1 × 1`) run weekly, so a
  regression only they would catch can merge and be found up to a week later. Every
  operation keeps a smaller crash proof per change.
- **Mutants that cannot be killed within the bounds:**
  - None of the collapse loop's lines (`ci = ci.parent()`, `are_siblings`) are reachable
    below four cells, so no mutant targets them.
  - M-151 reversed FR-017: decoding normalised again and stopped checking order. Only a
    two-cell input distinguishes it, and the 25-byte decode property runs out of memory at
    8 GB (as does the mutated run). The patch was dropped from `mutants/`. FR-017's evidence
    is its regression tests (`cell_union_tests::decode_*`, which fail without the fix),
    `decode_matches_cpp` and the fuzz target.
- **Reliance on 001.** Outputs are read through `CellId` methods 001 proved (`range_min`,
  `range_max`, `is_valid`, `face`, `level`, `pos`); the decode property reads ids the same
  way ([`../TRUSTED_BASE.md`](../TRUSTED_BASE.md)).
- **Written arguments.** The leaf count of any valid union fits in `i64` (FR-016); the
  order of `normalize`'s input never matters (P-UNORM-04, by transpositions); and
  `denormalize` cannot panic for gaps the regression test cannot materialise.
- **Kani issues met:** a driver panic while parsing CBMC output ("data did not match any
  variant of untagged enum ParserItem") on a run that was near its memory limit, and the
  cost traps in "Notes from implementation". None needed a workaround beyond making the
  harness cheaper.

## Recommendations

### Are bounded proofs over collections worth it? Partly

- **Read-only operations: yes.** Queries, invariant checks, the leaf count and the
  encoding verify at 3 to 4 cells in seconds, and exhaustively cover the edge cases at
  small sizes (empty unions, nesting, face boundaries, every byte string a decoder can see).
- **Operations that build a union: only as a crash-freedom net.** Normalising operations
  stop at two cells, `union` and `intersection` at `1 × 1`, and `difference` and the range
  constructors not at all. The behaviour that makes these operations hard is out of reach,
  so the functional proofs add little over the property tests, at a high cost.
- **Decoding untrusted bytes: yes.** The decoder is proven crash-free on every input of up
  to 25 bytes and correct against the model up to 24, which is where the header logic
  lives. The FR-017 order check needs two cells, so only the crash proof reaches it. The
  proofs complement the fuzz target.
- **The differential tests paid off most.** They were written because the proofs could not
  cover `from_range`, and they found the feature's one serious defect.

### Go or no-go: go, with a narrower target

Continue with Kani for integer and byte-level code, but choose targets whose interesting
behaviour fits in small inputs: fixed-size values, parsers and decoders, and read-only
queries. For collection-building algorithms, invest in differential tests against C++ and
in property tests first, and use Kani only for crash-freedom at small sizes.

### Assurance level

Bounded model checking is the wrong tool for `normalize`'s merge loop: the cost is in the
propositional encoding, not the solver (R0). Removing the size bound needs loop invariants:
Kani's loop contracts (unstable in 0.68.0, and they need annotations in `core/`) or a
deductive verifier such as Verus or Creusot
([`../research/rust_formalmethods.md`](../research/rust_formalmethods.md) §2). That is a
larger investment than the evaluation recommends for now, and only for `normalize`.

### What next (ranked)

1. **The integer coding layer** (`encoded_uint_vector`, `encoded_s2cell_id_vector`,
   varint): untrusted input, fixed-width integer arithmetic, and byte-level formats. These
   are the strengths this feature confirmed. 001 also ranked them first among the decoders.
2. **The other untrusted-input decoders** (`Cap`, `Rect`, `Cell`, polylines): crash-freedom
   over short byte strings, like the `CellUnion` decoder here. Their `format!` error
   messages need the same deferred formatting first.
3. **More C++ differential coverage** of operations built on `CellUnion` (`ShapeIndex`
   construction, `RegionCoverer`), since D-002 reached `ShapeIndex` unnoticed.
4. **Loop contracts for `normalize`**, once Kani's are stable, to remove its size bound.

## Notes from implementation

- **Iterator loops under a large unwind.** The model's predicates on raw ids need an unwind of
  32 for `spec::decompose`. Written with iterator `collect`s over slices, they made CBMC
  unroll the collect 32 times, each with a 31-level search: `p_uinv_01__n0` and `__n2` ran
  out of memory at 4 GB, while `__n1` passed in 15 s. Rewritten as indexed loops over the
  concrete length (`for i in 0..ids.len()`), the same harnesses take 0.2 s, 12 s and, at
  four cells, 48 s. The generators index their arrays the same way.
- **`memcmp` in assertions.** Comparing two `Vec<u8>` runs CBMC's `memcmp`, a byte loop, so
  the encoding harnesses need an unwind of the encoding's length (`9 + 8·n`) plus 2.
- **Loops over an implementation's output.** An output's length is symbolic to CBMC, so a
  loop over it, a slice `==`, a `collect` or an `assert_eq!` on `Vec`s made the set,
  normalisation and decode properties run out of memory even at 0 or 1 cell. The output
  readers now take a concrete maximum (`out_covers::<MAX>`, `out_valid::<MAX>`,
  `out_ids_equal::<MAX>`) and loop over it with a length guard.
- **Decoding what `encode` wrote.** Decoding a buffer that `encode` had written into a `Vec`
  ran out of memory even for one cell, while decoding a symbolic stack array took seconds.
  P-UENC-02 therefore decodes the model's byte layout built on the stack. Together with
  P-UENC-01 (`encode(U)` is exactly that layout) this gives `decode(encode(U)) = U`.
- **Kani's fixed cost per harness.** Each harness costs a few seconds outside its reported
  verification time, about 5 s on the workstation. At 170 harnesses that was 3.5 of the 10
  per-change minutes. Removing crash members that a per-change property member already
  covers (shared harnesses) saved more than moving large members to the weekly tier did.
- **Splitting a property by conjunct.** P-UDEN-01 (equal to the model, and valid) did not fit
  4 GB as one harness, but each conjunct did (`p_uden_01__n1_ids`, `__n1_valid`). Splitting
  P-UENC-03's 25-byte member by the count field did not help: both halves still ran out of
  memory at 8 GB.
- **Reading outputs without the model's search.** Checking an output's validity or
  normalisation needs its decomposition. Instead of `spec::decompose`, outputs are decomposed
  through `CellId::is_valid`, `face`, `level` and `pos`, which 001 proved equal to the model
  (P-VAL-03, P-VAL-06). That keeps the unwind of the normalising harnesses small. The
  reliance is recorded in `TRUSTED_BASE.md`.
- **The runner's out-of-memory classification (R18).** R0 reported that `run-verify.sh`
  showed CBMC's pre-solver "Out of memory" exit (status 6) as `ERROR`. On 2026-10-03 this did
  not reproduce. A throwaway 3-cell `from_cell_ids` harness, run at ceilings from 2 GB down
  to 128 MB, ran out of memory both in the solver ("Solver ran out of memory during
  propositional reduction") and before it ("Out of memory", "CBMC failed with status 6").
  `run-verify.sh` reported `OOM` every time, because its re-run log always contained the
  message. The runner now also matches the status-6 line alone, `std::bad_alloc`, and Rust's
  "memory allocation of N bytes failed", so an exit without the message is still reported
  as `OOM`.
- **The runner's memory backstop.** The first full weekly union run stopped at
  `p_uenc_04__b25` with "memory backstop hit". That harness uses 7.8 of its 8 GiB ceiling,
  and the whole-run cgroup cap was exactly one ceiling (`jobs × ceiling`), leaving nothing
  for Kani's driver. The 86 harnesses before it had passed. 001's runner design (its T003)
  called for `jobs × ceiling + 2G`, and `run-verify.sh` now does that, still capped below
  available memory.

## Acceptance (SC-001 … SC-012)

Checked 2026-10-04 against the final runs.

| Criterion | Result | Evidence |
|-----------|--------|----------|
| SC-001 crash-freedom | **Met.** 27/27 in-scope operations proven; 2 over their whole domain, 25 within size bounds, each at or above its minimum: read-only operations 3 cells (validity checks 4, union queries `3 × 3`), normalising operations 2, `intersection_with_cell_id` 3, `denormalize` 2, `union` and `intersection` `1 × 1`, decoding 25 bytes (minimum 17). The largest members run weekly | `INVENTORY.md` §1; "Bounds reached" |
| SC-002 scope ≥ 60% | **Met.** 27/42 = 64.3%. Each of the 15 excluded rows has a reason, and each of the 5 tool limits has its measurement | `INVENTORY.md` §1; "Exclusions and tool limits" |
| SC-003 catalogue | **Met.** Every in-scope operation has a `P-U*` property, and every FR-005 area is covered. 23 of 23 entries whose operations are in scope are proven (22 within bounds that meet SC-001, P-URANGE-04 in full). The 4 others belong to tool-limit operations, each listed with the tests that check it instead | `PROPERTIES.md` |
| SC-004 non-vacuity | **Met.** 25 mutants, 25 killed, and every proven property has at least one (M-104 re-run after moving to the weekly tier: killed). Every harness's covers are satisfied in both tiers (165/165 in the weekly run) | `../MUTANTS.md`; `PROPERTIES.md`, "Mutants" column; "The final runs" |
| SC-005 run time | **Met on the reference stand-in; CI not yet measured.** Per-change `union` leg: 120 harnesses in 7.9 min on 4 pinned CPUs (20.9 CPU-min). The `cellid` leg is unchanged (196 harnesses, 12.5 min on 001's 4-vCPU emulation). Weekly `union` leg: 49.4 min, one job at 8 GB, against the slow job's 360-minute limit. Neither leg has run on a hosted runner; the maintainer deferred T046's live check | "Measurements" |
| SC-006 no API or dependency change, gate unchanged | **Met, with one approved behaviour change.** Since the spec commit, `core` changed only `cell_union.rs`, `encoding.rs`, `shape_index.rs` and `cell_iterator_join.rs` plus tests and the C++ vectors. No `pub` signature or manifest changed, and `make test` passes (2026-10-04). Besides FR-015 and FR-017, D-002's fix changes `from_range` to match C++, by the maintainer's decision (FR-010) | `git diff 8cfeaf2^ --stat -- core python wasm`; "Defects found" |
| SC-007 self-contained evaluation | **Met** (T048 read-through). "Summary" and "Recommendations" answer continue or stop (continue), with which approach (Kani for integer and byte-level code, differential tests for collection-building code), whether bounded collection proofs are worth it (partly, by operation kind) and what next (the integer coding layer) | This document |
| SC-008 `denormalize` parity | **Met.** No panic for any minimum level 0–30, level step 1–3 and cell level 0–30 (the regression test sweeps them, for gaps of up to two levels); the regression and differential tests fail without the fix (re-checked); 1,865 C++ vectors match | "Defects found" |
| SC-009 leaf-count precondition | **Met.** `from_verbatim` and `leaf_cells_covered` document the precondition, the panic and the `i64` result (`ce53834`); P-UMEAS-01 proves no overflow up to 3 cells, and the written argument covers any size | "Defects found"; `../TRUSTED_BASE.md` |
| SC-010 decoding | **Met.** Valid unions decode verbatim, four siblings included (P-UENC-02; regression test). Every input up to 24 bytes that is not a valid union is rejected (P-UENC-03), and the tests cover longer ones. The deviation is documented. The regression and differential tests fail without the fix (re-checked); 523 C++ decode vectors match; robustness tests pass; fuzz smoke run of 60 s: 8.1 M executions, no crash | "Defects found"; `core/tests/decoder_robustness.rs` |
| SC-011 `# Panics` documented | **Met.** 23 items, in one documentation-only commit (`ce53834`) | "`# Panics` documentation added" |
| SC-012 upstream findings | **Met.** U-003 has a follow-up task (T054), written with the task list before any proof, and an entry here | "Upstream findings" |
