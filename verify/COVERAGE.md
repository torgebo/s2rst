# Crash-freedom coverage: the review of 2026-10-26

A review of the four suites' crash-freedom records (after `074e923`) found three gaps:

1. **Inputs outside the documented domains.** Each crash proof shows that an operation does not
   panic inside its domain. Nothing showed the other half of its `# Panics` section: that it
   panics for the inputs the section names, and only where the section says (debug builds or
   every build).
2. **The value types' floating-point exclusions.** Specs 004–007 excluded 62 entries of `Cell`,
   `s2::Rect`, `s2::Point`, `s1::ChordAngle` and `s1::Angle` as floating-point geometry or as tool
   limits of `asin`, `atan2` and `tan` (and 9 `Display` impls). Spec 008 had since shown how to
   reach them: function contracts for the C library functions the checker does not model.
3. **Tool limits** (`Display` through `core::fmt`, `CellId::all_neighbors`,
   `decode_string_vector`, `CellUnion` beyond 3–4 cells). They stay out: no extension fits the
   budgets, and fuzzing and the C++ vectors remain their evidence.

This record covers what was done about gaps 1 and 2. The inventories carry the per-entry
results: 001's `INVENTORY.md` section 3, `cell_union/INVENTORY.md` section 3,
`decoding/INVENTORY.md` section 2 and `values/INVENTORY.md` sections 10 and 11.

## Gap 1: the documented panics

### Method

- **`n_*`: every input outside the domain panics.** Kani's `#[kani::should_panic]` passes when
  *one* input panics (and every failed check is a panic): a probe showed that it also passes
  when some inputs return, and that `kani::assert(false)` after the call counts as a panic
  (`KANI_ISSUES.md`, limits). Each `n_*` harness therefore ends in one `kani::cover!` after the
  call, which only an input that returns can satisfy; `run-verify.sh` requires it unsatisfied
  (covers `never`) and reports `RETURNS` otherwise.
- **`r_*`: a debug-only panic is absent from release builds.** `make verify-release`
  (`VERIFY_DEBUG_ASSERTIONS=0`) builds without debug assertions, in its own target directory,
  and runs the `r_*` harnesses: on the same inputs, no panic and no arithmetic overflow. Kani
  compiles rustc's overflow checks into every build, and no option turns them off (KI-012), so
  an input where a release build would wrap is outside the proof (two cases, named in the
  inventories). The pass drops CBMC's NaN check (`--no-overflow-checks`'s only other effect):
  the release behaviour of a skipped check is often a NaN, which Rust computes without a panic.
  The same pass runs the every-build `n_*` harnesses, so those panics are proven in both builds.
- **`o_*`: inputs outside a domain that the section says return.** Where a section names such
  inputs (an odd-marker cell id, a NaN margin on a full interval, the empty cap expanded by
  +∞), a plain proof shows that they return, in both builds.

Two lessons: a symbolic panic condition is not simplified to false after the panic, so CBMC
explores the code after it on paths no input takes, and loops there need a small unwind bound
(`crash::n_raw_level`, which spec 009 retired: its operations now take a `Level`); and the invalid cell ids are written as `spec::valid_by_marker`, the
model's validity without its 31-level search, which `tests/spec_vs_cpp.rs` checks against
`spec::valid`.

### Results

71 operations, by suite (39 `n_*`, 21 `r_*` and 4 `o_*` harnesses):

| Suite | Operations | Harnesses |
|-------|------------|-----------|
| `CellId` | 21: every operation taking a raw level, `level`, `child_position`, `parent`, the child operations, the wrapping steps, `vertex_neighbors`, `common_ancestor_level` | 8 `n_*`, 4 `r_*` |
| `CellUnion` | 11: the normalising constructors, the cell queries, `leaf_cells_covered`, `denormalize`, the three range constructors | 7 `n_*`, 2 `r_*` |
| Decoding layer | 3: the fixed-width helpers | 2 `n_*` |
| Value types | 36: `Cell`'s four constructors, `vertex` and `vertex_raw`, `Cell` and `CellId::from_point`, 8 of `s1::Interval`, `r1::Interval::project`, 7 of `s2::Rect` (with its two distances), 4 of `r2::Rect`, `ChordAngle`'s trigonometry and arithmetic (6), `Cap`'s two debug checks | 22 `n_*`, 15 `r_*`, 4 `o_*` |

Every `n_*` harness proves in the debug build, and every every-build one in the release pass;
every `r_*` and `o_*` harness proves (2026-10-26). The harnesses take 0.1–20 s, except
`r2::Rect::from_center_size` (196 s, the domain recomputes the bounds) and `Cell::vertex` (72 s).

### Documentation corrected

Twelve `# Panics` sections claimed more panics than the code has, or fewer, and three operations had none although they panic. Each was found by
a harness written to the old text that returned (`RETURNS`), or that passed on a larger domain
than the text named, or, for the floating-point rows, by a native check; the section now says
what the code does, and the harness proves the corrected text. No behaviour changed.

| Operation | The old text | What the code does |
|-----------|--------------|--------------------|
| `s2::from_face_ij` | panics (debug) for `i` or `j` outside `[0, 2^30)` | 36 of 48 sampled such inputs return, many as valid cells; now "some panic" |
| `Cell::from_cell_id` | panics on the null id and faces 6, 7; other invalid ids unspecified | `1 << 62`, `1 << 63` and `3 << 62` panic too (no level marker) |
| `CellUnion::from_cell_ids`, `normalize`, `FromIterator`, `union` | panics (debug) for an invalid id | an invalid id in a valid input cell's leaf range is dropped without a panic |
| `CellUnion`'s queries on an invalid union | panics (debug) | no check reads the union; now "unspecified, may panic" |
| `CellUnion::approx_area`, `exact_area`, the `Region` bounds | panics for an invalid id | only where building the id's `Cell` panics |
| `CellUnion::from_min_max` | panics for `min > max` | `min == max.next()` gives the empty union, as upstream |
| `CellUnion::denormalize` | panics (debug) for an invalid id | ids with a marker are kept (a face-6 leaf); now "may panic" |
| `s1::Interval::expanded` | panics (debug) for a NaN margin | not on the full interval, which it returns, as upstream |
| `s2::Rect::interior_contains_lat_lng` | panics (debug) for a bad longitude | only when the latitude is inside: it is tested first, as upstream |
| `s2::Rect::expanded` | panics (debug) for a NaN longitude margin | not on full longitudes |
| `r2::Rect::from_center_size` | panics (debug) for one negative size component | not when rounding absorbs it (`−5e−324`, or `−1` at a centre of `1e300`) |
| `s2::Cap::expanded` | panics (debug) for +∞ | not on the empty cap, which returns first |
| `CellId::from_point`, `Cell::from_point` (gap 2) | (nothing) | panics (debug) for the zero vector, and may for a NaN coordinate |
| `Cell::vertex`, `vertex_raw`; `s2::Rect::get_distance_to_latlng`, `get_distance` (gap 2) | (nothing) | an index above 3 panics; the two distances debug-check their documented preconditions (upstream only logs the point distance's) |

### Non-vacuity

Eight mutants (`M-801` … `M-808`): four remove a guard (so an `n_*` harness returns or finds no
panic), two promote a debug check to every build (`Debug-Assertions: off`, so an `r_*` harness
fails), one clamps a length, and one drops the full-interval shortcut (so an `o_*` harness
panics). All eight are killed (in place, 2026-10-26, and by `make verify-mutants M='M-8*'`,
2026-10-27). `run-mutants.sh` counts `RETURNS` as a kill and reads the new header. The gap 2
harnesses are crash proofs, whose covers show their domains reached, as everywhere in the suite.

## Gap 2: floating-point geometry

### Method

Each excluded entry got a crash harness (`c_fg_*`), measured at 12 minutes and 4 GiB, then those
over it at 60 minutes and 8 GiB (the weekly budget), as spec 008 did. Four means made them
cheaper:

- **Function contracts** for the C library functions the checker does not support (`tan`,
  `atan`, with 008's `asin`, `atan2` and `cos`), and for two it supports too loosely: `sin` (the
  sign) and `sin_cos` (both near zero at once), whose loose models failed three harnesses that
  should pass (`Cell::from_lat_lng`, `Rect::expanded_by_distance`'s shrink branch,
  `Rect::get_distance_to_latlng`). `tests/libm_contracts.rs` checks every contract against the
  real `libm`.
- **The abstract u-v bound** (`stubs::any_uv_bound`): any rectangle in [−1, 1]² for the bound
  `Cell::from_cell_id` stores. The projection of a symbolic (i, j) was most of the cost: with it,
  `vertex` went from 326 s to 25 s and `rect_bound` from 30 minutes to 280 s.
- **The abstract cell and pair** (`fixed_vertex`, `pair_vertex`, `any_contains_point`) for the
  entries that read a cell through its vertices only.
- **Splits**: one harness per entry group, so a slow entry does not hold back cheap ones.

### Results

43 of the 62 entries are now proven crash-free, 24 per change and 19 weekly; the other 19 are
not. Per type: `Cell` 19 of 31, `s2::Rect` 11 of 18, `s2::Point` 5 of 5, `s1::ChordAngle` 7 of 7,
`s1::Angle` 1 of 1 (`values/INVENTORY.md`, section 11, and each row). Seven of `CellId`'s
floating-point rows are proven too, by the `Cell` harnesses that run them (001's inventory).

| Harness | Entries | Contracts and stubs | Time | Tier |
|---------|---------|---------------------|------|------|
| `ag_crash::c_fg_tan` | `Angle::tan` | `tan` | 0.1 s | per change |
| `rl_crash::c_fg_area` | `Rect::area` | — | 1.2 s | per change |
| `cl_crash::c_fg_average_area` | `average_area_for_level`, `average_area` | u-v | 8.9 s | per change |
| `rl_crash::c_fg_centroid` | `Rect::centroid` | — | 12.3 s | per change |
| `cl_crash::c_fg_size_and_ij` | `size_st`, `ij_coord_of_edge` | u-v | 21.8 s | per change |
| `cl_crash::c_fg_vertex` | `vertex`, `vertex_raw` | u-v | 25.1 s | per change |
| `cl_crash::c_fg_edge` | `edge`, `edge_raw` | u-v | 29.2 s | per change |
| `rl_crash::c_fg_expand_for_subregions` | `Rect::expand_for_subregions` | — | 38.9 s | per change |
| `rl_crash::c_fg_contains_point` | `Rect::contains_point`, `interior_contains_point`, `Region::contains_point` | `atan2` | 40.1 s | per change |
| `cl_crash::c_fg_contains_point` | `Cell::contains_point`, `Region::contains_point` | u-v | 85.0 s | per change |
| `rl_crash::c_fg_expanded_by_distance__neg` | `Rect::expanded_by_distance`, shrinking | `asin`, `sin`, `cos` | 86.8 s | per change |
| `ca_crash::c_fg_conversions` | `to_angle`, `radians`, `degrees`, `e5`, `e6`, `e7`, `From<ChordAngle>` | `asin` | 89.9 s | per change |
| `cl_crash::c_fg_center` | `center`, `center_raw` (and `CellId::to_point`, `to_point_raw`) | u-v | 128.3 s | weekly |
| `cl_crash::c_fg_approx_area` | `approx_area` | u-v, abstract cell | 216.4 s | weekly |
| `rl_crash::c_fg_cap_bound` | `Rect::cap_bound`, `Region::cap_bound` | — | 250.1 s | weekly |
| `cl_crash::c_fg_rect_bound` | `Cell::rect_bound`, `Region::rect_bound` | `atan2`, u-v | 279.8 s | weekly |
| `rl_crash::c_fg_region_cells` | `Region for Rect`: `contains_cell`, `intersects_cell` | `atan2`, u-v | 297.7 s | weekly |
| `cl_crash::c_fg_cap_bound` | `Cell::cap_bound`, `Region::cap_bound` | u-v, abstract cell | 411.2 s | weekly |
| `pt_crash::c_fg_angles` | `distance`, `stable_angle`, the three `approx_eq`s | `atan2` | 459.6 s | weekly |
| `cl_crash::c_fg_from_lat_lng` | `Cell::from_lat_lng` (and `CellId::from_lat_lng`, `From<&LatLng>`) | `sin_cos`, u-v | 496.1 s | weekly |
| `cl_crash::c_fg_from_point` | `Cell::from_point` (and `CellId::from_point`, `From<&Point>`) | u-v | 689.2 s | weekly |
| `rl_crash::c_fg_expanded_by_distance__pos` | `Rect::expanded_by_distance`, growing | `asin`, `atan2`, `cos` | 737.6 s | weekly |

Times are from the measurement runs: per-change members at 4 GiB with up to seven jobs, weekly
ones at 8 GiB with two or three; the 12-minute triage proved some weekly members too. The weekly
members are two new CI legs, `values-fg-a` and `values-fg-b` (about 25 and 30 minutes). The
abstract u-v bound made the difference for most of the `Cell` rows: without it, `vertex` took
326 s, `contains_point` 274 s, `rect_bound` 30 minutes.

### Tool limits

| Entries | Measurement | Where it stays |
|---------|-------------|----------------|
| `Cell::exact_area`; `distance_to_edge`, `max_distance_to_edge`; `distance_to_cell`, `max_distance_to_cell` and the four `is_*distance_less*` | out of memory at 8 GiB weekly, with and without the abstract u-v bound and cell; the robust predicates' exact fallback (`num-bigint`) in `point_area`, `edge_pair_closest_points` and `update_max_distance` | the xlarge tier (`make verify-xlarge`, a 32 GB machine), not run |
| `Rect::boundary_intersects` (the robust crossing test), `get_distance`, `get_hausdorff_distance`, `get_directed_hausdorff_distance` | time out at 60 minutes weekly | `known-timeouts.txt`, the weekly leg `values-limits-h` |
| `Cell::distance_to_point`, `boundary_distance`, `max_distance_to_point` | fail Kani's NaN check after 48 and 37 minutes: on box points off the unit sphere, `edge_distance` takes `sqrt(1 − pq²)` with `pq² > 1` (upstream's formula; a NaN that `from_length2` maps to `STRAIGHT`, no panic). A unit-length domain needs the squared length, which costs more than the checker can afford here (spec 008 research R0) | not proven; the harness is removed |
| `Rect::get_distance_to_latlng` | fails Kani's NaN check after 51 minutes in `interior_dist` (0/0: the edge normal's squared length is 0 on some path), with the `sin_cos` contract | not proven; the harness is removed |
| `Rect::cell_union_bound`, `Region::cell_union_bound` | call `Cap::cell_union_bound`, spec 008's tool limit | no harness |

All of them keep the evidence they had: the `region_*` trait tests, the property tests and the C++
vectors. Three of the harnesses that pass needed a contract only because the checker's own `sin`
and `cos` are too loose (`KANI_ISSUES.md`, limits): they failed until `sin` and `sin_cos` got
contracts, which the real `libm` satisfies.

## Spec 009: bounded types (2026-10-27)

Spec 009 (`BOUNDED_TYPES.md`) made four of gap 1's absolute-range preconditions into types that
are checked when an instance is created. The panics they documented moved to the types'
constructors, so gap 1's complements for them changed:

| Before (gap 1) | After |
|----------------|-------|
| `crash::n_raw_level`: 13 entry points panic for a raw level above 30 (the panicking `From<u8>`) | retired. The operations take a `Level`. `crash::n_level_new` (`Level::new` panics exactly above 30) and `crash::c_level_try_from` (`try_from` exact, with `OutOfRange`) |
| (none: `from_face(7)` returned an invalid id) | `crash::n_face_from_u8`, `crash::c_face_try_from`; `cl_crash::n_from_cell_id` lost its face and raw-level cases, which are no longer expressible |
| `cu_crash::n_denormalize__n2`: a `level_mod` outside 1–3 or a first id without a marker | keeps the id half. `cu_crash::n_level_mod_new` and `cu_crash::c_level_mod_try_from` for the modulus |
| `cl_crash::n_vertex`: an index above 3 panics | retired. `cl_crash::c_cell_vertex_try_from` |

Mutants:

- M-803 and M-806 are re-pointed to the constructors: `n_level_new` and `n_level_mod_new`
  kill them.
- M-809, M-810 and M-811 are new: each `try_from` accepts one value too many.
- Fourteen older patches went stale from the signature edits, and were regenerated with the
  same defects, restated on the new code:
  - M-017, M-018, M-019, M-021, M-024, M-025, M-058, M-128, M-313 and M-720 (the `Face` and
    `Level` edits);
  - M-140 and M-150 (a comment above `denormalize`'s rounding);
  - M-726 and M-740 (`CellVertex`).

The red/green evidence of each binding and core change is `BOUNDED_TYPES.md` §4. Two findings
of the red runs, beyond the spec's list:

- B11: the Python and wasm cell-id snap functions stored a raw level, and panicked only when
  used.
- B12: with `min_level > max_level`, a release build's `covering` did not finish. The check is
  now in every build, at each public coverer and term-indexer method.

### Final runs (spec 009)

- `make verify`, 2026-10-27, 16:20–17:33: 892 of 892 per-change harnesses `SUCCESSFUL`. That is
  the 887 of the gap-2 run, less `n_raw_level` and `n_vertex` (retired), plus `c_level_try_from`,
  `c_face_try_from`, `n_face_from_u8`, `c_level_mod_try_from` and `n_level_mod_new`.
  `n_level_new` and `c_cell_vertex_try_from` replace the two retired harnesses.
- `make verify-release`: 41 of 41 (the every-build `n_*`, the `r_*` and the `o_*`). That is
  gap 1's 40, less the two retired, plus the three new every-build complements.
- `cargo test` in `verify/` (the models, links and the properties-by-kind index): passed.
- `make verify-mutants`, 2026-10-27:
  - all killed: M-803, M-806, M-809, M-810 and M-811 (the constructors);
  - the per-change patches regenerated by the type changes, all killed: M-017, M-018, M-019,
    M-021, M-024, M-025, M-058, M-128, M-313, M-720 and M-726;
  - the weekly-tier ones, all killed: M-140, M-150 and M-740. M-740 had to be regenerated
    once more, after `cap_bound`'s vertex loop changed to iterate by reference (`b1c44d7`).

  All 318 patches apply.

## Spec 010: `LatLng` and `Rect` as a `Region` (2026-10-28)

Spec 010 added a value type in the same pattern as 004–008, so this review's two gaps are
closed for it from the start:

- **Gap 1 (the documented panics):** `LatLng` documents no panic, and none of its operations
  panics anywhere (`ll_crash::c_*` over every pair of `f64`). Its two documented preconditions,
  `to_point`'s finite coordinates and `get_distance`'s normalised inputs, are not panics:
  `c_to_point` proves the operation over every pair (NaN coordinates outside), and
  `o_get_distance` proves `get_distance` over every pair without an infinity (NaN where upstream
  gives NaN).
- **Gap 2 (floating-point geometry):** every entry through `atan2`, `sin_cos`, `sin`, `cos` and
  `asin` has a crash proof under the contracts; none is excluded as floating-point geometry.
  `Display` and `to_string_in_degrees`' formatting stay out with the other `Display`s (gap 3).

`values/INVENTORY.md` section 12 has the rows. `Rect`'s three `Region` methods that the new law
instances exercise (`contains_cell`, `intersects_cell`, `contains_point`) already had crash
proofs from gap 2.

## Spec 011: `PointRegion` and `CellUnion`'s floating-point methods (2026-10-28)

Spec 011 closed both gaps for `PointRegion` from the start, and gap 2 for the nine
floating-point rows that spec 002 had excluded from `CellUnion`:

- **Gap 1 (the documented panics):** the planning probe found four debug-build panics that the
  documentation did not state, each upstream's `DCHECK` too (spec 011 K4): `PointRegion::rect_bound`
  for a NaN `x` or `y`, `Cell::contains_point` (and so `PointRegion::intersects_cell`) off its
  face, `CellId::from_point` with two or more infinite coordinates, and
  `CellUnion::expand_by_radius` on a NaN radius. FR-019 documented each, with a `should_panic`
  test; `pr_crash::n_rect_bound_nan`, `n_intersects_cell` and `uf_exp::n_expand_by_radius_nan`
  prove the NaN cases exact, and `r_rect_bound`, `r_intersects_cell`, `r_from_point_infinities`
  the release builds. The checker cannot show the infinite cases' panics (it reports `∞ · 0` as
  a NaN check first), so those rest on the native tests and the probe's sweep.
- **Gap 2 (floating-point geometry):** every `PointRegion` entry has a crash proof (the
  rectangle under `atan2`'s contract, the covering under `CellId::from_point`'s, weekly). Of
  `CellUnion`'s nine rows, six are now proven within 002's size bounds, given the crash proofs of
  the `Cell` functions they call (`uf_crash`); three inherit a measured limit: `exact_area`
  (`Cell::exact_area`, the xlarge tier), `cell_union_bound` (`Cap::cell_union_bound`, spec 008)
  and `expand_by_radius`'s expansion (`all_neighbors`, 001). `cell_union/INVENTORY.md`'s counts are
  33 in scope and 9 excluded.

`values/INVENTORY.md` section 13 and `cell_union/INVENTORY.md` rows 12, 25–28, 35–37, 40 have the
rows.

## Spec 012: `Cell`'s bounding laws and the region adaptors (2026-10-30)

- **Gap 1 (the documented panics):** `RegionIntersection::region(i)` panicked past the end without
  saying so (spec 012 K2); it is documented, and `ad_crash::n_region_past_end` proves the panic
  exact in every build.
- **Gap 2 (floating-point geometry):** the adaptors' 16 in-scope entries have crash proofs over
  abstract members, the bounds under `atan2`'s and `asin`'s contracts; `cell_union_bound` inherits
  `Cap::cell_union_bound`'s limit. `values/INVENTORY.md` section 14 has the rows.

## What changed outside `verify/`

Only documentation: the `# Panics` sections of gap 1, in `core/src/s2/cell_id.rs`, `cell.rs`,
`cell_union.rs`, `rect.rs`, `cap.rs`, `core/src/s1/interval.rs` and `core/src/r2/rect.rs`. No
behaviour changed, and the Python and WASM bindings check their inputs before these calls, so
their documentation stands.

## Final run (2026-10-27)

- `make verify`: 887 of 887 per-change harnesses `SUCCESSFUL` (5 jobs, 1 h 36 min), the new
  `n_*`, `o_*` and per-change `c_fg_*` members included, with the models' and links' tests. The
  edits after it moved weekly members between tiers, removed three failing weekly harnesses,
  and reformatted; no per-change harness changed.
- `make verify-release`: 40 of 40 (`n_*` in every build, `r_*`, `o_*`).
- The weekly members of gap 2: as in the results table, each in its final form.
- `make test`: passed (`core/`'s documentation builds with warnings as errors, the bindings'
  tests and stubs unchanged).
- `cargo test` in `verify/` (130 tests: the models, `valid_by_marker_is_valid`, the libm
  contracts, the page index) and `verify/deductive/` (7, the drift guard included): passed.
- Mutants: `M-801` … `M-808` killed. Eleven older patches had gone stale from the documentation
  edits (their context lines moved); they were regenerated with the same mutation lines, and
  the six that are not weekly-tier re-checked in place, all killed. All 315 patches apply.
- `run-verify.sh` now reads CBMC's out-of-memory status from the main run, so an `OOM` no
  longer waits for a re-run alone (one took 30 minutes on 2026-10-27).
