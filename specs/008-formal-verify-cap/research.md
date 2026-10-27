# Research: Formal Verification of `Cap`, with the Trait Laws It Carries

Planning decisions for [spec.md](spec.md). Each section gives the decision, the reason and the
alternatives that were rejected. R0 is the planning probe, in three rounds; R1 is the
re-reading and running of upstream.

## R0: Planning probe (three rounds, run 2026-10-24)

**What runs.** `probe/r0_harnesses.rs` (24 harnesses), `probe/r0b_harnesses.rs` (18) and
`probe/r0c_harnesses.rs` (9), from a scratch crate that depends on `s2rst` alone, through a
copy of `run-verify.sh` with `-Z stubbing` (4 GiB and 12 minutes per harness, 3 jobs, Kani
0.68.0, CBMC 6.11.0). R0 ran 13:21–14:12, R0b 14:19–14:40, R0c 14:47–15:00; nothing else ran.
Times and verdicts are in `probe/r0_results.tsv` and `probe/r0bc_results.tsv`. Every failure
was replayed natively before it was read as a finding.

### R0: the questions as first asked

| Question | Harnesses | Result |
|----------|-----------|--------|
| Can `#[kani::stub]` replace `f64::asin` and `f64::atan2`? | `r0_stub_asin_to_angle`, `r0_stub_atan2_latitude`, control `r0_nostub_asin_to_angle` | **Yes.** 1.4 s and 2.7 s; the control fails on "call to foreign "C" function `asin` is not currently supported" (0.9 s) |
| K4 on the library | `r0_complement_library_vs_upstream` | fails in 3.3 s (length² 3.657754: the library's radius is not `4 − r²`) |
| FR-017's level | `r0_level_draft_margin` | proven, 13.8 s: `MIN_WIDTH.max_level(r) − 1` keeps the cells at least `2r` wide, for every finite `r > 0` |
| K1 on the library's formula | `r0_level_library_margin` | fails in 7.7 s, but **the counterexample is the checker's**: at r = 0.4866111182066655, `d / r` = 1.9375 and `log2` is 0.954 natively, so the level is −1 and nothing is asserted. CBMC's `log2` is loose. K1's evidence stays the native probe; FR-017 removes `log2` |
| FR-020's reduction | `r0_reduce_draft_exact` | proven, 2.9 s: for every \|x\| ≤ 3π/2 the result is in [−π, π] and differs from `x` by exactly 0 or ±2π (the `f64` nearest) |
| `rect_bound` valid under the two contracts | `r0_rect_bound_draft_valid`, `_library_valid` | both fail (46 s, 169 s) with "NaN on division" at `sin_a / sin_c`. **Checker artefact:** CBMC's `cos` returned 0 for cos(1.1·10⁻³⁰⁸) and cos(−0), where the real value is 1 (replayed). Hence the `cos` contract (spec FR-003, amended) |
| The equality laws | `r0_eq_laws` | proven, 1.9 s |
| `contains` reflexive | `r0_contains_reflexive` | proven, 13.5 s |
| `expanded` contains the original | `r0_expanded_contains` | proven, 336.9 s (weekly) |
| `add_point`, `add_cap`, `contains ⇒ intersects`, `intersects` symmetric | four harnesses | **all time out** at 12 minutes |
| A unit centre in the domain | `r0_codec_round_trip_eq`, `r0_complement_draft_valid`, `r0_from_center_angle_crash` | **all time out** |
| `cell_union_bound` has at most six ids (asin contract) | `r0_cell_union_bound_len` | times out (`CellId::from_point` and `vertex_neighbors`; 001 proved the latter only weekly) |
| `union` crash (both contracts) | `r0_union_crash` | times out |
| Upstream's cell tests over a symbolic cell | `r0_region_*` (3) | out of memory at 4 GiB in 16 s (two), timeout (one) |

**Reading.** Every timeout but `union` and `cell_union_bound` has one cause: the statement
recomputes float products that the operation already computed. `add_point(p)` stores the
chord `d(c, p)` as the radius, and `contains_point(p)` computes `d(c, p)` again; the checker
does not see that the two are one value, and must prove two copies of three products equal.
The same holds for `is_unit` assumed then checked, for `contains` against `intersects` (two
chord-angle sums, each through `sqrt`), and for 006's chord symmetry, which timed out at 60
minutes.

### R0b: three remedies

1. **Lemma stubs** (spec FR-003a): the recomputed library function is replaced by a pure
   function (one value per input, from a table in a `static Mutex`) that returns any value
   satisfying lemmas proven on the real code.
2. **The `cos` contract.**
3. **Decomposing the chord's symmetry** into lemmas of one or two variables.

| Harness | Result |
|---------|--------|
| `lemma_chord_range`: the chord between two box points is in [0, 4] | proven, 0.9 s |
| `lemma_add_bounds`: a sum of two lengths is in [0, 4] and at least each operand | proven, 626.1 s (weekly) |
| `lemma_neg_sub`: `b − a` is `−(a − b)` bit for bit, finite operands (except a zero difference's sign) | proven, 54.6 s |
| `lemma_sq_neg`: `(−u)·(−u)` is `u·u` bit for bit | proven, 94.1 s |
| `lemma_norm2_neg`: the squared length of `−p` is that of `p` | timeout (it follows from `lemma_sq_neg` by composition: the three sums take equal operands) |
| `r0b_contains_implies_intersects` (chord and sum stubbed) | proven, 20.3 s |
| `r0b_from_center_angle_crash` (`is_unit` stubbed) | proven, 1.7 s |
| `r0b_codec_round_trip_eq` (`is_unit` stubbed) | proven, 8.8 s |
| `r0b_rect_bound_draft_valid` (asin, atan2, cos contracts) | **proven, 84.6 s** |
| `r0b_rect_bound_library_valid` (same) | fails, 259.8 s: **checker artefact**, the library's `rem_euclid` goes through `f64 %`, which Kani computes wrongly (007 R0). With the stub values the native reduction gives the valid [0.733, 1.608]. FR-020 removes it |
| `r0b_edge_test_crash`: upstream's edge test, every box cap, vertex and edge | proven, 385.0 s (weekly) |
| `r0b_add_point_contains`, `r0b_add_cap_contains`, `r0b_complement_draft_valid` | **fail: the stubs lacked facts of the real code** — `chord(p, p) = 0` (an empty cap's `add_point` makes `from_point(p)`), adding zero returns the other operand, and the constants' centre (1, 0, 0) is unit. A stub that lacks a fact can only fail a law, never pass one (R6) |
| The cell laws with growable tables | out of memory: a `Vec` in a static is expensive to model |
| `r0b_private_stub_union` | the private `cap::interpolate_at_distance` **can be stubbed** (the path resolves); `union` still times out |

Two tool findings came out of R0b's set-up:

- **Kani 0.68.0's compiler panics** ("intrinsics.rs:243 … `IntTy::I32`") when a stub uses
  `thread_local!` with `RefCell`. A `static Mutex<…>` works. Recorded in memory and the
  trusted base.
- **Comparing arrays or tuples with `==` or `<` inside a stub** is avoided (element-wise
  comparisons), as 002's lessons already advise.

### R0c: complete stubs, fixed tables

| Harness | Result |
|---------|--------|
| `lemma_chord_self`: `chord(p, p) = 0` for every box point | proven, 1.7 s |
| `lemma_axis_unit`: `empty()` and `full()` are valid | proven, 0.0 s |
| `r0c_add_point_contains` | **proven, 6.8 s** |
| `r0c_add_cap_contains` | **proven, 17.0 s** |
| `r0c_complement_draft_valid` | **proven, 4.7 s** |
| `r0c_intersects_symmetric` (the chord and sum stubs key unordered pairs, by the symmetry lemmas) | **proven, 27.0 s** |
| `r0c_region_contains_implies_intersects_abstract` (vertices any four box points, fixed table; edge test abstracted) | **proven, 53.0 s** |
| `r0c_region_no_intersect_no_vertex` (same) | **proven, 51.1 s** |
| `r0c_region_contains_implies_intersects_real_edges` (upstream's real edge test over an abstract cell) | **proven, 696.9 s** (weekly) |

### Decisions from R0

- **Stubbing works** for `f64` functions, the library's public functions, trait methods
  (`<ChordAngle as Add>::add`) and private functions. The proofs use it in three ways: the
  function contracts (R5), lemma stubs (R6) and the abstract cell (R7).
- **Per change:** every law of R0c except the real-edge variant, the reflexivity and
  equality laws, the lemmas under 2 minutes, the level and the reduction, and `rect_bound`'s
  validity (85 s).
- **Weekly:** `lemma_add_bounds` (626 s), the edge test's crash proof (385 s), the cell law
  with real edges (697 s), `expanded`'s containment (337 s; per change if a lemma-stub variant
  is cheap, measured in implementation).
- **Attempted weekly, not proven yet:** `union`'s crash proof and `cell_union_bound`'s length;
  if they do not finish at 60 minutes they are recorded as timeouts (tool limits).
- **Spec amendments:** `cos` joins the contracts (FR-003), lemma stubs are allowed with their
  lemmas named (FR-003a), and the covering law is attempted, not counted (SC-004).

## R1: Upstream, read and run (v0.14.0-40-g37a4501)

Read in `/home/t/sources/s2geometry/src/s2/s2cap.{h,cc}`, `s2metrics.h`, `s1chord_angle.cc`;
run through `probe/cpp/survey.cc`, built against the existing release build in
`/home/t/sources/s2geometry/build` (installed to a scratch prefix).

- `S2Cap(center, S1Angle)` clamps the angle to π and converts; `S2Cap::FromCenterHeight` and
  `FromCenterArea` use `FromLength2(2 h)` and `FromLength2(area / π)`; `Empty()` and `Full()`
  have centre (1, 0, 0) and radii `Negative()` and `Straight()`.
- `is_valid` is `IsUnitLength(center) && radius.length2() <= 4`; `is_empty` is
  `radius.is_negative()`, `is_full` is `radius.length2() == 4`.
- `Contains(S2Point)` is `S1ChordAngle(center, p) <= radius` with a unit-length debug check;
  `InteriorContains` adds `is_full() ||` and uses `<`.
- `Contains(S2Cap)`, `Intersects`, `InteriorIntersects`: as the library (`s2cap.cc:73–87`).
- `Complement()`: `FromLength2(4 − r²)` around `−center` (`:65–71`). Observed: length² 1
  gives 3, the library 2.99999999999999956 (K4).
- `AddPoint`: `max(radius, d(center, p))`, with the comment that `Contains(p)` must then hold
  because it computes the same distance; `AddCap`: `d + other.radius` plus
  `(2 ε + kRelativeSumError) · d²`, with `kRelativeSumError = 2.02 ε` (`s1chord_angle.h:129`).
- `Expanded`: `radius + S1ChordAngle(distance)`, empty stays empty, a debug check on a
  negative distance.
- `Union`: through `GetRadius()` (`asin`) and `S1Angle(a, b)` (`atan2`), and
  `S2::GetPointOnLine`; the library adds an error margin (`cap.rs:315–321`), a deviation that
  makes the union contain both caps.
- `GetRectBound()`: `remainder(lng ∓ A, 2π)` (`:186–188`); observed: contains the probe's
  centre where the library's does not (K5).
- `GetCellUnionBound()`: `kMinWidth.GetLevelForMinValue(GetRadius().radians()) − 1`, six faces
  below 0, else `AppendVertexNeighbors` (`:193–213`); observed level 1 where the library's is 2
  (K1).
- `Contains(S2Cell)`, `MayIntersect(S2Cell)`, `Intersects(cell, vertices)` (private):
  vertices, then the edge test (`:225–303`). Observed on K2's grid: 38 of 65 true (library
  30); `MayIntersect` false for K3's cap (library true).
- `ApproxEquals(other, max_error = 1e-14)`: `S2::ApproxEquals(center, other.center,
  max_error)` and the radius terms. Observed: true for centres 5.02·10⁻¹⁵ apart (library
  false, K6).
- `Decode` accepts what it reads (003 R18); upstream has no serde.

No undefined or wrong upstream behaviour was met: FR-025 has nothing to record so far.

## R2: Suite gating, names and records

**Decision.** The proofs join the `values` feature and 004's suite.

| Item | Value |
|------|-------|
| Modules | `cp_crash`, `cp_con`, `cp_cls`, `cp_mea`, `cp_mem`, `cp_rel`, `cp_gro`, `cp_cmp`, `cp_bnd`, `cp_eq`, `cp_reg`, `cp_lem` (the lemmas) |
| Shared verification code | `verify/src/stubs.rs` (the function contracts and the lemma stubs, R5, R6), `verify/src/cap_spec.rs` (R3) |
| Catalogue ids | `P-CPCON`, `P-CPCLS`, `P-CPMEA`, `P-CPMEM`, `P-CPREL`, `P-CPGRO`, `P-CPCMP`, `P-CPBND`, `P-CPEQ`, `P-CPREG`, `P-CPCOD`, `P-CPDES`; lemmas `L-CP-NN` |
| `make` | `make verify-values P=cap` |
| CI leg | `values-cap`; weekly `values-cap` in the slow job |
| Mutants | `M-7NN` |

- `cp_` occurs in no module or harness name in `verify/src/` (checked 2026-10-24); `P=cap`
  passes each module as `--harness <module>::`, as spec 006's final run settled.
- **Every `verify` build gains `-Z stubbing`.** A crate with any `#[kani::stub]` attribute
  needs the flag at code generation. It changes nothing for harnesses without stubs (Kani
  applies stubs per harness). `run-verify.sh` adds it to every `cargo kani` call, the
  codegen, the main run, the single-harness re-runs and the playback.

**Rejected.** A separate `cap` feature: the value types share one suite, one runner and one
CI pattern.

## R3: The reference description (`verify/src/cap_spec.rs`)

**Decision.** Written from `s2cap.{h,cc}`, never calling the operations it describes; it reads
`center()` and `chord_radius()` only.

- `member(cap, p)`: `cap` is not empty and the chord length² from the centre to `p`, computed
  by the description as `(c − p).norm2()` clamped at 4, is at most the radius's length²; every
  point for the full cap. The library's `contains_point` is checked against it.
- `empty(cap)`, `full(cap)`, `valid(cap)`: upstream's definitions.
- Restatements of upstream's arithmetic (kept apart, checked against the C++ rows):
  `upstream_height(l) = 0.5 · l`, `upstream_area(l) = 2π · max(0, 0.5 · l)`,
  `upstream_centroid`, `upstream_complement_length2(l) = min(4, 4 − l)` (as
  `FromLength2(4 − l)`), `upstream_add_cap_margin(d) = (2 ε + 2.02 ε) · d`,
  `upstream_level(r) = ilogb(d / r)` clamped to [0, 30], minus 1.
- Booleans compared with `==` only (Kani's bool-ordering bug); no `f64 %`.

**Rejected.** A set-of-points description with real arithmetic: no harness can decide
membership of a real sphere point exactly; the chord comparison is upstream's own meaning of a
cap and the description recomputes it independently.

## R4: Input domains

| Class | Definition | Used by |
|-------|------------|---------|
| `BoxCap` | centre in [−1, 1]³ (006's box), radius `NEGATIVE` or a length in [0, 4] | the relations, growth, membership: every law whose meaning does not need unit length |
| `ValidCap` | `BoxCap` with `is_unit(centre)`, through the pure `is_unit` stub (R6) | validity preservation, the codec, `from_center_angle`, the bounds |
| `NonEmptyCap` | radius a length in [0, 4] | `contains` reflexive, `contains ⇒ intersects`' second cap |
| `BoxPoint` | 006's | points |
| `AbstractCell` | four box points as vertices and four box vectors as edges, from a fixed table (R7) | the cell laws |
| `ValidCell` | 001's model, as `any_cell` | crash proofs of the cell tests, with vertices stubbed |
| `AnyF64`, `AnyAngle` | 004's, 007's | constructors |

- Assuming `is_unit` on the real code costs a symbolic squared length and timed out three
  times in R0; with the pure stub it costs nothing (R0b: 1.7 s, 8.8 s). It is sound because
  the stub agrees with itself, and a stubbed `is_unit` that answers more freely only allows
  more caps (R6).
- A centre outside the box is not a valid cap (`is_unit` fails for any coordinate above
  1 + 5ε in magnitude), so `BoxCap` covers every valid cap.

## R5: The function contracts (spec FR-003)

**Decision.** `verify/src/stubs.rs` holds `asin_contract`, `atan2_contract` and `cos_contract`,
exactly as `probe/r0b_harnesses.rs` states them:

- `asin`: NaN outside [−1, 1] and for NaN; ±0 for ±0; otherwise any value in [−π/2, π/2]
  with the argument's sign.
- `atan2(y, x)`: NaN if either is NaN; otherwise any value in [−π, π], with `y`'s sign
  (sign bit), within [−π/2, π/2] when `x` has a clear sign bit (C11 F.10.1.4).
- `cos`: NaN for a non-finite argument; 1 at ±0; otherwise in [−1, 1], and at least
  6·10⁻¹⁷ when |x| ≤ `FRAC_PI_2`.

They are tested natively in `verify/tests/libm_contracts.rs`: each special value (±0, ±1,
±∞, NaN, subnormals, `FRAC_PI_2` and its neighbours, `atan2`'s sixteen signed-zero and
infinity combinations) and 10⁶ random inputs per function from a fixed seed (uniform bits and
uniform values in the function's domain), on the platform's `libm`. The test asserts that each
real result satisfies its contract.

- The contracts state ranges only, so no proof can depend on `asin` being exact or monotone.
- **Rejected:** contracts as Kani function contracts (`#[kani::ensures]` with
  `stub_verified`): they need annotations in `core`, and the real functions are C code that
  Kani cannot verify against the contract anyway.

## R6: Lemma stubs (spec FR-003a)

**Decision.** `verify/src/stubs.rs` also holds the pure-function stubs, built on one
fixed-size table type (`Table<K>`: ten slots of `K` 64-bit keys, in a `static Mutex`, keys
compared element-wise), as R0c measured:

| Stub | Replaces | Returns | Lemmas |
|------|----------|---------|--------|
| `pure_chord` | `Point::chord_angle` | 0 for a point and itself; otherwise any length in [0, 4], one per ordered pair | L-CP-01 (`chord(p, p) = 0`), L-CP-02 (range) |
| `pure_chord_sym` | same | as `pure_chord`, keyed by the unordered pair | also L-CP-03, L-CP-04 (the symmetry, by `b − a = −(a − b)` and `(−u)² = u²`) |
| `pure_add` | `<ChordAngle as Add>::add` | the other operand when one is zero; otherwise any length in [0, 4] at least each operand | 007's P-CAARI-01 (zero), L-CP-05 (bounds, weekly) |
| `pure_add_sym` | same | as `pure_add`, keyed by the unordered pair | also L-CP-06 (`a + b` is `b + a`, to be measured; R13) |
| `pure_is_unit` | `Point::is_unit` | true for (±1, 0, 0); one value per point with the coordinates' signs ignored | L-CP-07 (the constants are valid), L-CP-04 (negation keeps the squared length) |
| `pure_approx_eq_angle` | `Point::approx_eq_angle` | any boolean, one per pair | none (only purity) |
| `fixed_vertex`, `fixed_edge` | `Cell::vertex`, `Cell::edge_raw` | the abstract cell's table (R7) | none: any values |

- **Soundness.** A stub returns, for each input, some value the lemmas allow; the real
  function returns one such value (by the lemma) and returns it every time (purity). So every
  behaviour of the real code is a behaviour of the stubbed code, and a law proven with the
  stub holds for the real code. A stub that is missing a fact only adds behaviours: R0b's
  three failures were of that kind, never a false pass.
- **Purity** is a property of the library, recorded in `TRUSTED_BASE.md`: Rust does not
  contract float expressions, and `r3::Vector`'s `mul_add` is chosen by
  `cfg!(target_feature = "fma")`, once per build (`vector.rs:99–120`).
- **The table's capacity** (ten keys) is a cost bound, not a soundness one: an eleventh key
  gets a fresh, unrecorded value, again only more behaviours.
- **Rejected:** a `thread_local!` table (crashes Kani 0.68.0's compiler, R0b); a growable
  `Vec` table (out of memory on the cell laws, R0b); running the laws without stubs in the
  weekly tier (R0's timeouts at 12 minutes; 006's chord symmetry did not finish in 60).

## R7: The `Region` laws over upstream's cell tests

**Decision (FR-018's shape).** `cap.rs` gains private
`fn intersects_cell_edges(self, cell: &Cell, vertices: &[Point; 4]) -> bool` (upstream's
`Intersects(cell, vertices)`), and `Region::contains_cell` and `Region::intersects_cell`
compute the four vertices once each and call it, as upstream's `Contains(S2Cell)` and
`MayIntersect`. The laws are then proven over an **abstract cell**: `Cell::vertex` and
`Cell::edge_raw` stubbed to a table of four box points and four box vectors drawn at the start
of the harness, `Point::chord_angle` pure.

- **Contains ⇒ intersects** (`laws::region::contains_implies_intersects`): with the edge test
  abstracted to any result (`kani::stub` of the private helper), per change (53 s); with the
  real edge test, weekly (697 s).
- **No intersection ⇒ no contained vertex** (new `laws::region::no_intersect_no_vertex`),
  edge test abstracted: per change (51 s).
- **Crash freedom** of the edge test over every box value: weekly (385 s); over `ValidCell`
  with the vertices from the real code, attempted weekly.
- The abstract cell has any four vertices, so the laws hold for every real cell a fortiori:
  they depend only on the order in which the tests are made, which is upstream's.

**Rejected.** A symbolic real cell (out of memory at 4 GiB, R0); concrete cells only (a few
faces and children would say little about the rest).

## R8: The fixes (FR-017 to FR-023)

| Change | Design | Test (fails without it) |
|--------|--------|------------------------|
| FR-017 (K1) | `cell_union_bound`: `i32::from(MIN_WIDTH.max_level(radius).as_u8()) − 1` | the probe's radius 0.11785113019775795 gives level 1, and 180 radii near the powers of two match `MIN_WIDTH.max_level − 1` |
| FR-018 (K2, K3) | R7's shape; `Region for Cap` calls the new private tests | the probe's grid (38 of 65 contain face 0) and K3's cap (no intersection) |
| FR-019 (K4) | `complement`: `ChordAngle::from_length2(4.0 − r²)` around `−center` | length² 1 gives 3, 1.7 gives 2.3 |
| FR-020 (K5) | private `fn reduce_longitude(x)`: `x − 2π` above π, `x + 2π` below −π, else `x` — exact for \|x\| ≤ 3π/2 (Sterbenz, R0's `r0_reduce_draft_exact`), which holds since `lng ∈ [−π, π]` and `A ∈ [0, π/2]`; no `%` | K5's cap contains its centre; the reduction equals 007's exact `remainder_2pi` (moved to a shared test helper, without its `−π → π` step) on 10⁶ inputs in [−3π/2, 3π/2] and the edges ±π, ±3π/2 |
| FR-021 | none expected: R0b proved the draft valid under the contracts | — |
| FR-022 (K6) | `approx_eq_with`: `center.approx_eq_angle(other.center, Angle::from_radians(max_error))`; `approx_eq`: `approx_eq_with(other, 1e-14)` | the probe's centres 5.02·10⁻¹⁵ apart are approximately equal |
| FR-023 (K7) | a hand-written `Deserialize` through a shadow struct, as `Cell` (`cell.rs:810`) and `Rect`, rejecting a cap that is not valid with the rule named | a NaN centre and a centre of length 2 are rejected; valid caps round-trip bit for bit; the JSON form is unchanged |

- The level change also removes the library's own NaN path: `max_level` debug-checks NaN, and
  a valid cap's `angle_radius` is never NaN (`to_angle` of [0, 4]).
- FR-018 costs four vertex computations per call, as upstream.

## R9: C++ reference vectors

**Decision.** A new generator, `tools/refgen-cpp/generate_cap.cc` (a target in its
`CMakeLists.txt`), writes `core/tests/data/cpp_cap_vectors.csv`, checked by a new
`core/tests/cpp_cap_diff.rs`. `cpp_values_vectors.csv` is already 9.6 MB, and 007 kept it under
10 MB; a file per type is the pattern of `cellid`, `cellunion` and `coding`. Its sections:

- `CAP_OPS` (random valid caps and points): construction from height, area and angle;
  height, area, centroid; `Contains(p)`, `InteriorContains(p)`; `Contains`, `Intersects`,
  `InteriorIntersects` with a second cap; `Complement`, `Expanded`, `Union`, `AddPoint`,
  `AddCap`; `ApproxEquals`.
- `CAP_SPECIAL`: empty, full, single-point, hemisphere, antipodal and identical centres, the
  radii next to the covering levels' powers of two (K1), K5's cap, K6's pair.
- `CAP_BOUNDS`: `GetRectBound`, `GetCellUnionBound` (the ids), and `S2RegionCoverer`'s
  covering with `max_cells = 8`.
- `CAP_CELL`: `Contains(S2Cell)` and `MayIntersect(S2Cell)` for random caps against cells at
  levels 0–10 near the cap, K2's grid and K3's cap.

`core/tests/cpp_cap_diff.rs` checks them bit for bit (ids exactly); deviation rows are
marked (none expected after FR-017 to FR-022; `Union` keeps the library's documented margin,
compared by containment). `verify/tests/values_spec_vs_cpp.rs` checks `cap_spec`'s
restatements. The build uses the existing release build of `/home/t/sources/s2geometry`,
installed to a prefix (`tools/refgen-cpp/README.md` gains the step).

## R10: The catalogue, its tiers and its counts

The catalogue is data-model §4: 24 entries for `Cap`, 7 lemmas, and 2 compositions recorded
in other types' catalogues (R12). Tiers from R0's times:

- **Per change**, about 40 harnesses: the crash proofs of the cheap entries (seconds, by
  006's and 007's experience), the R0c laws but one, the lemmas under 2 minutes, the level,
  the reduction and `rect_bound`'s validity.
- **Weekly**, about 8: `lemma_add_bounds`, the edge test's crash proofs, the real-edge law,
  `expanded`'s containment (unless its lemma-stub variant is cheap), `union`'s crash and
  `cell_union_bound`'s length (attempted), the covering law (attempted).

SC-006's 10 minutes: R0c's per-change set took under 3 minutes at 3 jobs, and the remaining
cheap harnesses are expected in seconds each, plus 5 s of set-up apiece (VERIFIED_TYPES §5).

## R11: Mutants (`M-7NN`)

At least one per proven entry, one exact-string edit each, 004's format. The candidates are
data-model §7. Two patch `cap_spec.rs`, two patch `stubs.rs` (a lemma stub that drops a fact,
expected to fail its law, which shows the stub is not vacuous), and the lemma harnesses get
their own (for example `ChordAngle::add` returning `rhs` unconditionally kills L-CP-05).

## R12: Compositions recorded in other types (spec FR-012)

- **`Cell::cap_bound` contains the cell's centre and four vertices**, as it computed them:
  `from_point(centre)` then four `add_point` calls, so P-CPGRO-01 (with `Cell::vertex` pure)
  gives it. A new entry in `Cell`'s catalogue (spec 004's section), status "proven by
  composition", with a harness instantiating it over `ValidCell` with the vertices stubbed.
- **006's chord-angle symmetry** (`Point::chord_angle(a, b) == chord_angle(b, a)`, part of
  P-PTDST-01, a timeout at 60 minutes): proven by composition from L-CP-03 and L-CP-04 — the
  three differences negate exactly, their squares are equal, and the sum, the clamp and
  `from_length2` take equal operands. 006's entry moves from "not proven (timeout)" to
  "proven by composition", and its known-timeout member is retired.

## R13: Not measured yet, measured first in implementation

- **L-CP-06, `a + b` is `b + a` bit for bit** for chord angles: the formula is symmetric in
  its operands, and the one `sqrt` takes `x · y` against `y · x`, equal by IEEE
  commutativity. The checker may need `sqrt` pure too (a lemma stub on `f64::sqrt`, 006 R0's
  under-determined model). If it does not prove per change, it goes weekly; P-CPREL-03 rests
  on it.
- **`expanded`'s containment with lemma stubs** (R0: 337 s on the real code).
- **The crash proofs** of the 30 cheap entries, which R0 did not probe one by one.
- **`cell_union_bound`'s length and `union`'s crash** at 60 minutes.

## R14: The quickcheck twins (spec FR-011)

`core/tests/trait_property_tests.rs` gains a generator of caps from arbitrary `f64` latitudes,
longitudes and radii (finite, clamped into range), alongside the whole-degree one, and K5's
cap as a fixed example in the `Region` rect-bound law. The `Region` laws of FR-010 get
quickcheck twins named as in `CORE_TRAITS.md`.

## R15: Records, CI and the evaluation

- **Records** (constitution v1.2.0): `verify/values/INVENTORY.md` §9 and `PROPERTIES.md` §10
  for `Cap`; the new `Cell` entry and 006's `Point` entry updated (R12); the index by kind and
  the HTML renders; `TRUSTED_BASE.md` (the three contracts, the lemma stubs, purity, the Kani
  compiler crash); `MUTANTS.md`; `VERIFIED_TYPES.md` §2.9 for `Cap` and §3's trait rows;
  `CORE_TRAITS.md`'s `Region` invariants name their instances; 003's records for `Cap`'s codec
  point to P-CPCOD-01.
- **CI**: `verify.yml`'s `verify` matrix gains `values-cap`; the `slow` job a `values-cap`
  leg with R10's weekly filters; the `mutants` matrix series `7`. `core/src/s2/cap.rs`,
  `core/src/s2/region.rs` and `verify/src/stubs.rs` join the path filters.
- **Evaluation**: a spec-008 part of `verify/values/EVALUATION.md`, answering SC-013: how much
  of `Cap` is proven and how, what the lemma stubs cost and gained (they reach laws that
  three earlier specs left as timeouts), which `Region` implementors the generic laws can
  reach next (`Rect` and `CellUnion` through their cap bounds, `Cell` through R12), and
  whether `LatLng` is next.
