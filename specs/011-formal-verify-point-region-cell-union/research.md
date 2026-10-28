# Research: Formal Verification of `PointRegion`, and `CellUnion`'s Floating-Point Methods

Phase 0 of the plan. Every NEEDS CLARIFICATION of the technical context is resolved here; the
measurements are R0's.

## R0: Planning probe (run 2026-10-28)

The questions: what each `PointRegion` operation and law costs, which stubs `CellUnion`'s
floating-point methods need and at what union length, and whether the debug-build checks are
reached. `probe/r0_harnesses.rs` (35 harnesses), run inside the verification crate through a
temporary `#[path]` module (not committed) with `run-verify.sh` at 4 GiB and 12 minutes, 4 jobs:
R0 at 21:41–21:55, R0b (the remedies, replays off) 22:06–22:34, R0c 22:34–22:46. Every line is
in `probe/r0_results.txt`. Times are the harnesses' own (memory "kill time, not wall time").

**`PointRegion`**

| Question | Harness | Result | Decision |
|----------|---------|--------|----------|
| `new`, `point`, `cap_bound`, `contains_cell`, `contains_point`, every triple | `r0_pr_basic` | 5.4 s | per change (`pr_crash::c_basic`, P-PRCON-01, P-PRCEL-01, -03) |
| `rect_bound`, every triple, `atan2_contract` | `r0_pr_rect_bound` | **FAILED**, 2.1 s: `s1::Interval`'s debug check `is_valid_pair` for a NaN longitude (a NaN `x` or `y`) | finding K4; `PlainTriple` per change, an `n_` harness for the panic |
| the same over `PlainTriple` | `r0b_pr_rect_bound_plain` | 2.7 s | per change |
| `intersects_cell`, every triple | `r0_pr_intersects` | **FAILED**, 68.4 s: `valid_face_xyz_to_uv`'s debug check, and NaN checks for infinite coordinates | finding K4 (`Cell::contains_point`); `BoxPoint` per change, an `n_` harness |
| the same over `BoxPoint` | `r0b_pr_intersects_box` | 40.5 s | per change |
| `cell_union_bound`, the real `from_point` | `r0_pr_cub_real`, `r0b_pr_cub_real` | unwinding at 8; **TIMEOUT** at 11 | `pure_leaf`, with `from_point`'s own crash proof by composition |
| `cell_union_bound`, `pure_leaf` | `r0b_pr_cub_stub` | 563.4 s (unwind 11) | weekly |
| the vertex law, CBMC's `sqrt` | `r0_pr_law_vertex` | **TIMEOUT** | weekly, split by face (R6) |
| the vertex law, `sqrt_contract` | `r0_pr_law_vertex_sqrtc` | **FAILED**, 104–117 s: the contract lets the root be 10³⁰⁰, so the normalised vertex underflows and leaves the cell; not the real code's behaviour (the probe: 0 of 80,000) | a tighter, true contract: the root lies between `x` and 1 |
| the vertex law, the tighter contract | `r0c_pr_law_vertex` | **TIMEOUT** | weekly, split by face; if it still times out, a tool limit (tested) |
| the bounding cap contains the point, box points | `r0_pr_law_cap` | 1.9 s | per change (P-PRREG-04) |
| the bounding rectangle contains it, `pure_from_point` | `r0_pr_law_rect` | 6.5 s | per change (P-PRREG-05) |
| the covering contains it, `pure_leaf`, `asin_contract` | `r0b_pr_law_cover` | 452.8 s (unwind 11) | weekly (P-PRREG-06) |

**`CellUnion`**

| Question | Harness | Result | Decision |
|----------|---------|--------|----------|
| point test, `pure_leaf` | `r0_cu_contains_point__n1`, `__n3` | 5.6 s, 15.3 s | per change, `n ≤ 3` (P-UPT-01) |
| `rect_bound` contains each cell's, `any_cell_bound` | `r0_cu_rect_bound__n1`, `__n2`, `__n3` | 27.7 s, 81.1 s, 333.7 s | per change `n ≤ 2`, weekly `n = 3` (P-UBND-01) |
| `rect_bound`, the real `Cell::rect_bound`, `atan2_contract` | `r0_cu_rect_bound_real__n1` | 93.1 s (unwind 11) | per change (`c_rect_bound__n1`) |
| `cap_bound` crash, `pure_cell_cap`, `any_center` | `r0_cu_cap_bound_crash__n2` | 462.7 s | weekly `n = 2`; `n = 1` per change |
| `cap_bound` contains each cell's | `r0_cu_cap_bound__n1`, `__n2` | **TIMEOUT** both | — |
| the same with 008's `pure_chord`, `pure_add` | `r0c_cu_cap_bound__n1`, `__n2` | 357.4 s; **TIMEOUT** | by composition for every length: P-CPGRO-02 and a new lemma L-PR-02 (a cap that contains `b` still contains it after `add_cap`), each over `BoxCap`, as 008's |
| `average_based_area`, the same expression | `r0_cu_avg_area__n1`, `__n3` | **FAILED**, 4.0 s, 16.8 s: the two copies differ | finding: the checker treats `powi` as it treats `sqrt` (KI-013) |
| is `powi` a function? | `r0b_powi_deterministic` | **FAILED**, 0.1 s | KI-013 in `KANI_ISSUES.md`, with this reproducer |
| `average_based_area`, `pure_avg_area` | `r0b_cu_avg_area__n1`, `r0c_…__n2`, `r0b_…__n3` | 10.5 s; **TIMEOUT**; **TIMEOUT** | per change `n = 1` (P-UAREA-01 within `n ≤ 1`) |
| `approx_area` is the in-order sum, `pure_area` | `r0_cu_approx_area__n3` | 429.7 s (unwind 11) | per change `n ≤ 2`, weekly `n = 3` |
| the empty union's areas are +0 | `r0_cu_area_empty` | **FAILED**, 1.8 s: −0 | K2 confirmed in the checker; passes after FR-018 |
| containment implies intersection | `r0_cu_law_contains__n3` | 29.1 s (unwind 11) | per change, `n ≤ 3` (P-UREG-01) |
| the vertex law, abstract cell, `pure_leaf` | `r0_cu_law_vertex__n1` | **FAILED**, 8.2 s | expected: not a law (the probe's counterexample is the fixed test) |

**Lessons:**

- **Unwinding**: building a `Cell` needs 9 (`to_face_ij_orientation`), a stub table 11; the
  covering's `vertex_neighbors` needs 11. Ten of R0's 17 failures were only that.
- **The debug-build checks** of K4 are upstream's `DCHECK`s; the checker reached two in seconds.
  A native sweep (`probe/rust/src/bin/leaf.rs`) found a third: `CellId::from_point` panics in
  debug builds for every point with two or more infinite coordinates (the face check's
  `p · n` is `∞ · 0`); release builds return a valid leaf for all 1,001,331 triples tried, the
  specials included.
- **Replays**: `run-verify.sh` re-runs each failure for a counterexample, which can take the
  whole limit; probes run with `VERIFY_PLAYBACK=0`.

## R1: Upstream, read and run (v0.14.0-40-g37a4501)

Read in `/home/t/sources/s2geometry/src/s2/s2point_region.{h,cc}`, `s2cell_union.{h,cc}`,
`s2cell.cc`, `s2cap.cc` and `s2metrics.h`; run through `probe/cpp/survey.cc`.

- **`S2PointRegion(p)`** keeps `p`, with `ABSL_DCHECK(S2::IsUnitLength(point))`.
  `GetCapBound()` is `S2Cap::FromPoint(point_)`; `GetRectBound()` is
  `S2LatLngRect(S2LatLng(point_), S2LatLng(point_))`, whose `S1Interval(lng, lng)` constructor
  `DCHECK`s validity (so a point with a NaN `x` or `y` fails a debug check upstream, as in the
  library: R0); `GetCellUnionBound` is the cap's; `Contains(cell)` is `false`;
  `MayIntersect(cell)` is `cell.Contains(point_)`; `Contains(p)` is `point_ == p`.
- **`S2PointRegion::Encode`/`Decode`**: a version byte (`kCurrentLosslessEncodingVersionNumber`,
  1) and three doubles; `Decode` fails on fewer than 25 bytes, a version above 1, or a point
  that is not unit length (`S2::IsUnitLength`, `|‖p‖² − 1| ≤ 5ε`, the library's `is_unit`).
  Missing in the library (K5, FR-020).
- **`S2CellUnion::GetCapBound`** (`s2cell_union.cc:235–259`): the empty cap for the empty union;
  otherwise the centroid `Σ AverageArea(level) · ToPoint()`, `(1, 0, 0)` when it is exactly
  zero, normalised; then `AddCap` of each cell's `GetCapBound()`. `S2Cap::AddCap` rounds up
  with a "TODO: error analysis"; the library's `add_cap` is proven to contain its argument
  (008's P-CPGRO-02).
- **`GetRectBound`**: `Union` of each cell's `GetRectBound()` from the empty rectangle.
  **`GetCellUnionBound`**: the cap bound's. **`Contains(cell)`, `MayIntersect(cell)`**: by id.
  **`Contains(p)`**: `Contains(S2CellId(p))`.
- **Areas**: `AverageBasedArea` is `S2Cell::AverageArea(kMaxLevel) · LeafCellsCovered()`;
  `ApproxArea` and `ExactArea` sum each cell's from `double area = 0` (K2).
- **`Expand(min_radius, max_level_diff)`**: `min_level` over the cells (30 for the empty
  union, where the library takes 0, unobservable: expanding the empty union is empty);
  `radius_level = kMinWidth.GetLevelForMinValue(r)`, which `DCHECK`s `!isnan(r)` (K4); a radius
  above the face width expands at level 0 first; then
  `Expand(min(min_level + max_level_diff, radius_level))`. The library's `u8` difference
  saturates and clamps to 30, the same level for every value an `int` difference could reach
  above it.
- **`S2Cell::GetRectBound`, level 0** (`s2cell.cc:226–275`): the polar faces' latitude bound is
  `kPoleMinLat = asin(sqrt(1./3)) − 0.5 · DBL_EPSILON`, computed (K1).
- **`S2Cell::Contains(p)`**: the u-v bound expanded by `(5/3) · DBL_EPSILON`, as the library.

No upstream defect was met (spec FR-022): K1 and K2 are the library's.

## R2: Suite gating, names and records

**Decision.** The proofs join the `values` feature and 004's suite, as one part, `P=regions`.

| Item | Value |
|------|-------|
| Modules | `pr_crash`, `pr_con`, `pr_bnd`, `pr_cel`, `pr_eq`, `pr_reg` (`PointRegion`); `uf_crash`, `uf_pt`, `uf_bnd`, `uf_area`, `uf_exp`, `uf_reg` (`CellUnion`'s floating-point half, also gated on `cellunion`, for `union_domain`) |
| Shared verification code | `verify/src/region_spec.rs` (R4), new stubs in `verify/src/stubs.rs` (R5), two laws in `verify/src/laws.rs` (R6) |
| Catalogue ids | `P-PRCON`, `P-PRBND`, `P-PRCEL`, `P-PREQ`, `P-PRREG`, `P-PRDES`; `P-UPT`, `P-UBND`, `P-UAREA`, `P-UEXP`, `P-UREG`; lemmas `L-PR-NN` |
| `make` | `make verify-values P=regions` (both prefixes) |
| CI leg | `values-regions`, and a `values-regions` slow leg for the weekly members |
| Mutants | `M-950` onwards, in series 9 |

- No file in `verify/src/` starts with `pr_` or `uf_` (checked 2026-10-28), so the module
  filters select only the new modules.
- **Why not the union suite (`make verify-union`)**: its per-change leg holds 120 harnesses in
  7.9 minutes (002), and these harnesses need the value suite's stubs and domains.
- **Why series 9 for the mutants**: `run-mutants.sh` reads a mutant's id as `M-[0-9]+`, and the
  CI legs select `M-<series>*`. A series `10` (`M-10NN`) would be selected by series 1's
  `M-1*`, which holds 002's `M-100` … `M-108`. Spec 010 used `M-901` … `M-937`; spec 011 takes
  `M-950` …, and the comment in `verify.yml` says series 9 holds both.

## R3: The fixes (FR-017 to FR-019)

| Change | Design | Test (fails without it) |
|--------|--------|------------------------|
| FR-017 (K1) | `POLE_MIN_LAT = 0.615_479_708_670_387_3`, the shortest literal of the `f64` nearest `asin(sqrt(1/3))` (`0x3fe3b2028082e8d3`, what upstream computes at start-up with glibc), with a comment saying so | the two polar face cells' bounds have upstream's latitude bits (`0x3fe3b2028082e8d0` after the expansion); `POLE_MIN_LAT == (1.0 / 3.0).sqrt().asin()` at run time |
| FR-018 (K2) | `fold(0.0, \|a, id\| a + …)` in `approx_area` and `exact_area`: upstream's sum, from +0, in order | the empty union's areas are +0 bit for bit; a non-empty union's are unchanged (the C++ vectors) |
| FR-019 (K3) | `PointRegion::new` documents upstream's precondition (a unit point) and what each method does otherwise; `rect_bound` documents its debug-build panic for a point with a NaN `x` or `y` (R0); a hand-written `Deserialize` that rejects a point that is not unit length, as `Cap`'s since 008 (K7 there) and upstream's `Decode` | `point_region` tests: (3, 4, 0) and a NaN point are rejected, a unit point round-trips |
| FR-019 (K4) | `expand_by_radius` documents the debug-build panic on a NaN radius (the metric's check, upstream's `DCHECK`) | a `#[should_panic]` test in debug builds; an `n_` harness |

Tests that pin the old behaviour: none for FR-017 in `core/` (no test pins the polar faces'
bound bits; checked 2026-10-28), and the coverer does not read cell rectangle bounds. The
Python and wasm suites are run after each change.

## R4: The reference description (`verify/src/region_spec.rs`)

**Decision.** A small module, with unit tests and drift guards:

- **A point region is its point**: `pr_contains(p, q) = (p == q)` on coordinates (so ±0 agree,
  and NaN contains nothing).
- **A union is the set of points whose leaf cell it contains**: through 002's
  `union_spec::contains_id` on the point's leaf.
- **The bounds' meaning is containment**: a bound of a union contains each cell's bound (the
  rectangle by `Rect::contains`, the cap by `Cap::contains`); the covering is the cap's.
- **The constructions' text** (the centroid, the fallback centre, the fold from `Rect::empty`,
  the covering through `cap_bound`, the sums from +0, `expand_by_radius`'s level arithmetic) is
  not restated as arithmetic, since a restatement that recomputes float products times out
  (008, 010 R13). Drift guards on the library's source text tie them to upstream's statements,
  and the C++ vectors check the values (FR-021).

## R5: Stubs

| Stub | Replaces | Allows | Rests on |
|------|----------|--------|----------|
| `pure_leaf` | `CellId::from_point` | any valid leaf, one per point | the real function returns a valid leaf for every point, NaN included (its weekly crash proof `cl_crash::c_fg_from_point` gains the validity assertion, L-PR-01; native test on the specials and 10⁶ random points) |
| `pure_cell_cap` | `Cell::cap_bound` | any non-empty cap with a centre in the unit box, one per cell | the real bound's centre is a normalised point and its radius in [0, 4] (native test on every cell to level 5 and 10⁶ random cells, `libm_contracts.rs`) |
| `any_center` | `CellId::to_point` | any point in the unit box | the same test for the real centre |
| `pure_cell_area`, `pure_avg_area` | `Cell::approx_area`, `exact_area`; `Cell::average_area_for_level` | any finite non-negative (positive) value, one per cell (level) | uninterpreted: the statements are about the sums and the product, not the values; `average_area_for_level` is pinned to upstream's `ldexp` by enumeration (P-UAREA-04) |
| `sqrt_between` | `f64::sqrt` | `sqrt_contract`'s values between `x` and 1 | the correctly rounded root lies there (`x` and 1 are representable); R0: `sqrt_contract` alone lets the vertex underflow; `libm_contracts::sqrt` gains the clause |
| 008's and 010's | `asin`, `atan2`, `sqrt`; `LatLng::from_point` (`pure_from_point`); `Cell::rect_bound` (`any_cell_bound`); the u-v bound (`any_uv_bound`) | as recorded in `TRUSTED_BASE.md` | as recorded |

**Why a pure leaf**: the covering law and the union's point test call `CellId::from_point` twice
on one point (the library once, the statement once). Its `st` projection takes a square root,
which the checker does not treat as one value per input (KI-007), and its crash proof alone takes
689 s weekly (coverage review). **Why a pure leaf area**: R0 found that the checker treats
`powi` as it treats `sqrt` (R0, KI-013).

## R6: The `Region` laws

| Law | `PointRegion` | `CellUnion` |
|-----|---------------|-------------|
| Containment implies intersection | P-PRREG-01, Kani (trivial: `contains_cell` is false) | P-UREG-01, Kani within bounds (integer: by id) |
| The covering covers every contained cell | P-PRREG-02, by composition: vacuous, since `contains_cell` is false (P-PRCEL-01) | P-UREG-02, not proven: the covering is the cap bound's, whose level needs `asin`'s value (as 008's P-CPREG-04) |
| The vertex law | P-PRREG-03: weekly, one harness per face, `sqrt_between` (R0: the unsplit harness times out with CBMC's `sqrt` and with `sqrt_between`); a tool limit if a face still times out, tested by the probe's 80,000 vertices | not a law (P-UREG-03): the probe's 20,573 contained vertices of 39,636 non-intersected neighbours; a fixed test pins one |
| The bounding cap contains every contained point | P-PRREG-04, Kani | P-UREG-04, not proven: by P-UBND-02 (by composition with P-CPGRO-02 and L-PR-02, for every length) the cap contains each cell's cap bound, but that a cell's cap bound contains the points whose leaf lies in the cell is `Cell`'s bounding-cap law, not proven (it needs the projections' values); tested (the probe's 262,154 points, the quickcheck twins) |
| The bounding rectangle contains every contained point (new, FR-010) | P-PRREG-05, Kani (pure `from_point`) | P-UREG-05, not proven, as P-UREG-04 with `Cell::rect_bound` |
| The covering contains every contained point (new, FR-010) | P-PRREG-06, weekly (452.8 s) | P-UREG-06, not proven, as P-UREG-02 |

**Measured in implementation (T017):** the covering law in its leaf form ("some id of the
covering contains `CellId::from_point(p)`") fails for `Cell` at its vertices: the closed cell
contains them, and their leaves lie in a neighbour (cell `14` and its vertex 1). The law is
stated in the leaf form, which `PointRegion` and `CellUnion` satisfy by construction, with the
closed form (`Cell::contains_point` of some covering cell) for a closed region's boundary; the
twins check both.

The two new laws go in `laws::region` with doc comments naming their instances, and gain
quickcheck twins for `Cap`, `Rect`, `Cell`, `CellUnion` and `PointRegion` in
`core/tests/trait_property_tests.rs` (FR-010, FR-013).

## R7: Input domains

| Class | Definition | Used by |
|-------|------------|---------|
| `AnyTriple`, `PlainTriple`, `BoxPoint` | 006's | `PointRegion`'s crash proofs (every triple), its laws (box points hold every unit point; `PlainTriple` for the rectangle law) |
| `ValidCell` with the abstract u-v bound | 004's and the coverage review's | the cell tests, the vertex law |
| `ValidUnion(N)`, `N ≤ 3` | 002's (`union_domain::any_valid_union`), concrete lengths | every `uf_*` harness |
| `AnyAngle`, `AnyU8` | 007's, every `u8` | `expand_by_radius`'s NaN panic and level |

## R8: Entries outside the checker

- **`Deserialize for PointRegion`** (after FR-019): tested (`point_region` tests), as `Cap`'s.
- **`exact_area`**: through `Cell::exact_area`, which runs out of memory at 8 GiB (coverage
  review, xlarge tier): crash freedom inherits that limit; its sum (P-UAREA-03) is proven with
  `pure_cell_area`.
- **`cell_union_bound` of a union**: through `Cap::cell_union_bound` of a cap of any radius,
  spec 008's tool limit (60-minute timeout); inherited. `PointRegion`'s, a zero-radius cap, is
  R0's question.
- **`expand_by_radius`**: through `expand_at_level`, whose `all_neighbors` is 001's tool limit
  (out of memory above 9.5 GB, 001 R12); inherited. Its NaN panic is reached before, so its
  `n_` harness runs.
- **`Debug`**: excluded (`core::fmt`), as every type's.

## R9: C++ reference vectors (FR-021)

**Decision.** A new generator, `tools/refgen-cpp/generate_region.cc`, writes
`core/tests/data/cpp_region_vectors.csv`, checked by a new `core/tests/cpp_region_diff.rs`
(one file per feature, 008's and 010's pattern). Sections, each double as hex bits:

| Section | Rows |
|---------|------|
| `POINTREGION` | `GetCapBound`, `GetRectBound`, `GetCellUnionBound` of random unit points and the probe's special vectors that are unit length or harmless (the axes, (−1, −0, 0), (−0, −0, −1)) |
| `POINTREGION_CELL` | `Contains(cell)`, `MayIntersect(cell)` for the point's own cell at each level and random cells; each cell's vertices as points |
| `CELLUNION_BOUND` | `GetCapBound`, `GetRectBound`, `GetCellUnionBound` of random, clustered and special unions (empty, whole sphere, each face, the polar faces, opposite faces, one leaf) |
| `CELLUNION_AREA` | `AverageBasedArea`, `ApproxArea`, `ExactArea` of the same unions |
| `CELLUNION_POINT` | `Contains(p)` for points in, on the boundary of and outside random unions |
| `CELLUNION_EXPAND` | `Expand(radius, diff)` of small clustered unions: the probe's special radii and random ones, `diff` up to 40 (the ids, not a hash) |
| `CELL_FACE_RECT` | `S2Cell::GetRectBound()` of the six face cells |

- The library matches every row after FR-017 and FR-018; there is no deviation.
- **Size**: about 20,000 rows, under 3 MB.

## R10: Mutants (`M-950` …)

At least one per proven entry and one per proven law instance, one exact-string edit each,
004's format:

- `point_region.rs`: `contains_point` by `!=`; `intersects_cell` always true; `contains_cell`
  true; `rect_bound` from a fixed `LatLng`; `cap_bound` from a widened cap and from the
  antipode; `point` returning a normalised copy;
- `cell_union.rs`: `contains_point` on the leaf's parent; `rect_bound` skipping the last cell;
  `cap_bound`'s fallback centre and `add_cap` replaced by `Cap::from_point`; the sums starting
  at −0 (FR-018's revert) and skipping a cell; `average_based_area` at level 29;
- `cell.rs`: FR-017's revert (killed by the `cargo test` target);
- `region_spec.rs`, `stubs.rs`: `pure_leaf` without its purity (the covering law fails), the
  point description by `!=`;
- the two new laws in `laws.rs` (an `!` dropped), killed by their instances.

## R11: Records, CI and the evaluation

- **Records** (constitution v1.2.0): `verify/values/INVENTORY.md` §13 and `PROPERTIES.md` §12
  for `PointRegion`; `verify/cell_union/INVENTORY.md`'s nine rows and new entries in
  `verify/cell_union/PROPERTIES.md`; the index by kind and its HTML render; `TRUSTED_BASE.md`
  (the new stubs and contracts, KI-013); `MUTANTS.md`; `VERIFIED_TYPES.md` (a §2.12 for
  `PointRegion`, `CellUnion`'s float half, §3's `Region` row, the missing codec); `CORE_TRAITS.md`
  (the two new laws' instances, `CellUnion`'s vertex-law exception); `verify/COVERAGE.md`;
  `KANI_ISSUES.md` (KI-013, `powi`, with a reproducer).
- **CI**: `verify.yml`'s `verify` matrix gains `values-regions`; `core/src/s2/point_region.rs`
  and the new CSV join the path filters; the `slow` job gains a `values-regions` leg for the
  weekly members; the `mutants` comment says series 9 holds 010's and 011's.
- **Evaluation**: a spec-011 part of `verify/values/EVALUATION.md` answering SC-013, ranking the
  follow-ups: `Cell`'s remaining floating-point methods (its bounding-cap and rectangle laws,
  which P-UREG-04 and -05 wait on), the region adaptors, the `S2PointRegion` codec (FR-020),
  010's open items (the `S2LatLng` codec, `Intersects(S2Cell)`, the two inexact remainders).
