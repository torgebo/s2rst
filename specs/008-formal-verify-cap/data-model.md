# Data Model: Formal Verification of `Cap`, with the Trait Laws It Carries

This feature adds no runtime data. The entities are the verification artifacts, the reference
description, the stubs and the changes to the library. The schemas are 004's
(`specs/004-formal-verify-cell-interval/data-model.md`), except where this document says
otherwise. The R0 column of §4 holds the planning probe's times (research R0, all three
rounds; "R0c 6.8 s" means the R0c harness of that law).

## 1. The reference description (`verify/src/cap_spec.rs`)

Plain Rust, compiled outside Kani too, unit-tested in `cap_spec_tests.rs`, checked against the
new C++ rows by `verify/tests/values_spec_vs_cpp.rs`. It reads `center()` and
`chord_radius()` only and never calls the operations it describes (research R3).

| Item | Definition |
|------|------------|
| `chord2(c, p)` | `min(4, (c − p).norm2())`, the squared chord, computed by the description |
| `empty(cap)`, `full(cap)` | radius length² < 0; radius length² == 4 |
| `valid(cap)` | `centre.is_unit()` and radius length² ≤ 4 (upstream's `is_valid`) |
| `member(cap, p)` | `full(cap)`, or not `empty(cap)` and `chord2(c, p) ≤ r²` |
| `interior_member(cap, p)` | `full(cap)`, or `chord2(c, p) < r²` |

**Restatements of upstream's arithmetic**, kept apart (spec FR-007):

| Item | Upstream | Checked against |
|------|----------|-----------------|
| `upstream_height(l)`, `upstream_area(l)` | `0.5 · l`; `2π · max(0, 0.5 · l)` | `CAP_OPS` height, area |
| `upstream_centroid(c, l)` | empty → origin; else `c · ((1 − 0.5 h) · area)` | `CAP_OPS` centroid |
| `upstream_from_height(h)`, `upstream_from_area(a)` | `FromLength2(2 h)`, `FromLength2(a / π)` | `CAP_OPS` constructors |
| `upstream_complement_length2(l)` | `FromLength2(4 − l)` (after FR-019) | `CAP_OPS` complement |
| `upstream_add_cap_margin(d2)` | `(2 ε + 2.02 ε) · d2` | `CAP_OPS` `AddCap` |
| `upstream_level(r)` | `ilogb(d / r)` clamped to [0, 30], minus 1; `d = 2√2 / 3` | `CAP_BOUNDS` ids' level |

## 2. Input domain classes (research R4)

| Class | Definition |
|-------|------------|
| `BoxCap` | centre in [−1, 1]³; radius `NEGATIVE` or a length in [0, 4] |
| `ValidCap` | `BoxCap` with `is_unit(centre)` through `pure_is_unit` |
| `NonEmptyCap` | `BoxCap` with a length in [0, 4] |
| `BoxPoint` | 006's |
| `AbstractCell` | `Cell::vertex`, `Cell::edge_raw` from a fixed table of four box points and four box vectors |
| `ValidCell` | 001's model (`any_cell`) |
| `AnyF64`, `AnyAngle`, `FiniteNonNegAngle` | 004's, 007's; the last for `expanded` |
| `LngSum` | \|x\| ≤ 3π/2, the reduction's domain (FR-020) |

## 3. Stubs (`verify/src/stubs.rs`; spec FR-003, FR-003a; research R5, R6)

| Stub | Kind | Replaces | Allows | Rests on |
|------|------|----------|--------|----------|
| `asin_contract` | function contract | `f64::asin` | NaN outside [−1, 1]; ±0 at ±0; else [−π/2, π/2] with the argument's sign | `libm_contracts::asin` (native) |
| `atan2_contract` | function contract | `f64::atan2` | NaN with NaN; else [−π, π], `y`'s sign, \|r\| ≤ π/2 for a clear-signed `x` | `libm_contracts::atan2` |
| `cos_contract` | function contract | `f64::cos` | NaN for non-finite; 1 at ±0; [−1, 1]; ≥ 6·10⁻¹⁷ for \|x\| ≤ `FRAC_PI_2` | `libm_contracts::cos` |
| `pure_chord` | lemma stub | `Point::chord_angle` | 0 for `(p, p)`; any length in [0, 4], one per ordered pair | L-CP-01, L-CP-02 |
| `pure_chord_sym` | lemma stub | same | as above, one per unordered pair | L-CP-01–04 |
| `pure_add` | lemma stub | `<ChordAngle as Add>::add` | the other operand for a zero operand (`+0` or −0); else [0, 4] and ≥ each operand | L-CP-08, L-CP-05 |
| `pure_add_sym` | lemma stub | same | as above, one per unordered pair | also L-CP-06 |
| `pure_is_unit` | lemma stub | `Point::is_unit` | true at (±1, 0, 0); one value per point, signs ignored | L-CP-07, L-CP-04 |
| `pure_approx_eq_angle` | lemma stub | `Point::approx_eq_angle` | one boolean per pair | purity only |
| `fixed_vertex`, `fixed_edge` | abstraction | `Cell::vertex`, `Cell::edge_raw` | the `AbstractCell` table | none |
| `any_edge_result` | abstraction | `Cap::intersects_cell_edges` (private, FR-018) | any boolean | none |

**Lemmas** (module `cp_lem`, each on the real code):

| Id | Statement | R0 |
|----|-----------|----|
| L-CP-01 | `p.chord_angle(p)` is 0 for every box point | R0c 1.7 s |
| L-CP-02 | `a.chord_angle(b)` is a length in [0, 4] for box points | R0b 0.9 s |
| L-CP-03 | `b − a` is `−(a − b)` bit for bit for finite `a`, `b` (a zero difference aside) | R0b 54.6 s |
| L-CP-04 | `(−u) · (−u)` is `u · u` bit for bit for finite `u` | R0b 94.1 s |
| L-CP-05 | for lengths `a`, `b`: `a + b` is in [0, 4] and at least each | R0b 626.1 s (weekly) |
| L-CP-06 | for lengths `a`, `b`: `a + b` is `b + a` bit for bit | not probed (R13) |
| L-CP-07 | `Cap::empty()` and `Cap::full()` are valid | R0c 0.0 s |

## 4. Property catalogue (`verify/values/PROPERTIES.md`, section 10)

Statuses as spec FR-006: **proven**, **by composition** (lemma stubs, named), **under
contract** (function contracts, named), **not proven (reason)**.

| Id | Kind | Plain statement | Operations | Domain | Expected status | R0 |
|----|------|-----------------|------------|--------|-----------------|----|
| P-CPCON-01 | correct result | `from_point`, `from_center_chord_angle` keep their arguments; `from_center_height` and `from_center_area` are upstream's `FromLength2(2 h)` and `FromLength2(a / π)` bit for bit; `empty()` and `full()` are upstream's constants | the five constructors | `BoxPoint × AnyF64` | proven | not probed |
| P-CPCON-02 | correct result | `from_center_angle` clamps the angle to π and is `from_center_chord_angle` of `ChordAngle::from_angle`; its validity check never fires on a unit centre and a non-NaN angle | `from_center_angle` | `ValidCap`'s centre × `AnyAngle` without NaN | by composition (`pure_is_unit`) | R0b 1.7 s |
| P-CPCLS-01 | correct result | `is_valid`, `is_empty`, `is_full` are upstream's; on valid caps empty and full exclude each other; the constants are valid | the three classifications | `BoxCap` | proven | L-CP-07 0.0 s |
| P-CPMEA-01 | exact output | `height`, `area`, `centroid` are upstream's formulas bit for bit; the area is never negative | `height`, `area`, `centroid` | `BoxCap` | proven | not probed |
| P-CPMEM-01 | correct result | `contains_point` is `cap_spec::member`; `interior_contains_point` is `interior_member` and implies `contains_point`; `from_point(p)` contains `p` | `contains_point`, `interior_contains_point`, `from_point` | `BoxCap × BoxPoint` | proven (one chord each side; measured first) | L-CP-01 1.7 s |
| P-CPREL-01 | law | `contains` is reflexive on non-empty caps; the full cap contains every cap and every cap the empty cap | `contains` | `BoxCap` | proven | R0 13.5 s |
| P-CPREL-02 | law | containing a non-empty cap implies intersecting it; `interior_intersects` implies `intersects`; the empty cap intersects nothing | `contains`, `intersects`, `interior_intersects` | `BoxCap × NonEmptyCap` | by composition (`pure_chord`, `pure_add`) | R0b 20.3 s |
| P-CPREL-03 | law | `intersects` is symmetric | `intersects` | `BoxCap²` | by composition (`pure_chord_sym`, `pure_add_sym`; L-CP-06) | R0c 27.0 s |
| P-CPGRO-01 | correct result | after `add_point(p)` the cap contains `p`, and its radius did not shrink | `add_point` | `BoxCap × BoxPoint` | by composition (`pure_chord`) | R0c 6.8 s |
| P-CPGRO-02 | correct result | after `add_cap(b)` the cap contains `b`, and its radius did not shrink | `add_cap` | `BoxCap²` | by composition (`pure_chord`, `pure_add`) | R0c 17.0 s |
| P-CPGRO-03 | correct result | `expanded` by a finite non-negative distance contains the original; an empty cap stays empty | `expanded` | `BoxCap × FiniteNonNegAngle` | proven (weekly) or by composition per change | R0 336.9 s |
| P-CPCMP-01 | exact output | `complement` is upstream's: centre `−c`, radius `FromLength2(4 − r²)` bit for bit; the full and empty caps are each other's | `complement` | `BoxCap` | proven (after FR-019) | the library fails, R0 3.3 s |
| P-CPCMP-02 | validity | the complement of a valid cap is valid | `complement` | `ValidCap` | by composition (`pure_is_unit`) | R0c 4.7 s |
| P-CPCMP-03 | correct result | the complement of the complement is the cap where `4 − r²` is exact, in particular for every r² ≥ 2; elsewhere its length² differs by at most 2⁻⁵¹ | `complement` | `NonEmptyCap`, not full | proven | not probed |
| P-CPBND-01 | correct result | `cap_bound` is the cap itself; the `Region` methods delegate to the inherent ones | `cap_bound`, `Region::{cap_bound, contains_point, rect_bound, cell_union_bound}` | `BoxCap` | proven | not probed |
| P-CPBND-02 | correct result | the covering's level is `upstream_level` and keeps the cells at least twice as wide as the radius | `cell_union_bound` (its level) | every finite `r > 0` | proven (after FR-017) | R0 13.8 s |
| P-CPBND-03 | correct result | `cell_union_bound` returns the six faces or four vertex neighbours, at most six ids | `cell_union_bound` | `ValidCap` | under contract (`asin`), weekly attempt | R0 timeout |
| P-CPBND-04 | validity | `rect_bound` returns a valid rectangle, empty exactly for the empty cap | `rect_bound` | `BoxCap` | under contract (`asin`, `atan2`, `cos`) | R0b 84.6 s |
| P-CPBND-05 | exact output | the longitude reduction is exact: the result differs from `x` by 0 or ±2π exactly and lies in [−π, π] | the reduction (FR-020) | `LngSum` | proven | R0 2.9 s |
| P-CPEQ-01 | law | `==` is an equivalence on box caps (004's `laws::eq`); any two empty caps are equal, and any two full caps | `PartialEq`, `equal` | `BoxCap³` | proven | R0 1.9 s |
| P-CPEQ-02 | correct result | `approx_eq_with` is upstream's `ApproxEquals`; `approx_eq` is it at 10⁻¹⁴ | `approx_eq`, `approx_eq_with` | `BoxCap² × AnyF64` | by composition (`pure_approx_eq_angle`) | not probed |
| P-CPREG-01 | law | `contains_cell` implies `intersects_cell` (`laws::region::contains_implies_intersects`) | the two `Region` cell tests | `BoxCap × AbstractCell` | by composition (edge test abstracted); weekly with the real edge test | R0c 53.0 s; 696.9 s |
| P-CPREG-02 | law | a cell the cap does not intersect has no vertex the cap contains (new `laws::region::no_intersect_no_vertex`) | `Region::intersects_cell`, `contains_point` | `BoxCap × AbstractCell` | by composition | R0c 51.1 s |
| P-CPREG-03 | law | the bounding cap contains every point the region contains (new `laws::region::cap_bound_contains_points`) | `Region::cap_bound`, `contains_point` | `BoxCap × BoxPoint` | proven | not probed (`cap_bound` is the identity) |
| P-CPREG-04 | law | every cell the cap contains is covered by its covering (`laws::region::covering_covers::<Cap, 6>`) | `contains_cell`, `cell_union_bound` | `ValidCap × ValidCell` | weekly attempt; not proven if it does not finish (SC-004) | not probed |
| P-CPCOD-01 | law | every valid cap round-trips through its encoding under `Cap`'s equality (the `CORE_TRAITS.md` codec law; 003's P-FX-03 proves the bytes) | `S2Encode`, `S2Decode` | `ValidCap` | by composition (`pure_is_unit`) | R0b 8.8 s |
| P-CPDES-01 | correct result | deserialisation accepts exactly the valid caps, unchanged | `Deserialize` | — | tested (serde is out of the checker's reach, as 004–007) | — |

**Crash proofs** (`cp_crash`, `C-s2::Cap::<method>`): every in-scope entry of §5, over its
domain; the cell tests over `ValidCell` with `fixed_vertex` (weekly) and over `AbstractCell`;
`union` under the `asin` and `atan2` contracts with `interpolate_at_distance` real (weekly
attempt, R0's timeout).

**Totals:** 26 `Cap` entries (25 for SC-003, P-CPDES-01 being tested only, as before), 7
lemmas, and 2 compositions in other catalogues (§4.1).

### 4.1 Entries in other types' catalogues (spec FR-012, research R12)

| Id | Section | Statement | Status |
|----|---------|-----------|--------|
| P-CCAP-01 (new) | `Cell` (spec 004) | `Cell::cap_bound` contains the cell's centre and four vertices as it computed them | by composition from P-CPGRO-01 (`Cell::vertex` pure) |
| P-PTDST-01 (update) | `Point` (spec 006) | its chord-symmetry clause | timeout → by composition from L-CP-03, L-CP-04; the known-timeout member is retired |

## 5. Operation inventory (`verify/values/INVENTORY.md`, section 9)

43 entries (spec FR-002):

- **Crash proofs** (31, and `Deserialize`, which is tested): `from_center_chord_angle`, `from_center_angle`
  (with `pure_is_unit` for its debug check), `from_point`, `from_center_height`,
  `from_center_area`, `empty`, `full`, `center`, `chord_radius`, `height`, `area`, `is_valid`,
  `is_empty`, `is_full`, `contains_point`, `interior_contains_point`, `contains`,
  `intersects`, `interior_intersects`, `complement`, `expanded`, `add_point`, `add_cap`,
  `cap_bound`, `centroid`, `equal`, `PartialEq`, `Region::cap_bound`,
  `Region::contains_point`, `Region::contains_cell`, `Region::intersects_cell` (the cell
  tests with the vertices stubbed: a cell's vertex computation is 006's normalisation),
  `Deserialize` (tested, not a Kani entry).
- **Under contract** (8): `angle_radius` (`asin`), `rect_bound` and `Region::rect_bound`
  (`asin`, `atan2`, `cos`), `cell_union_bound` and `Region::cell_union_bound` (`asin`;
  weekly attempt), `union` (`asin`, `atan2`; weekly attempt), `approx_eq`, `approx_eq_with`
  (`atan2`).
- **003's proofs** (2): `S2Encode`, `S2Decode`.
- **Excluded** (1): `Display`.

## 6. Changes to the library

| Change | Files | Kind | Test (fails without the change) |
|--------|-------|------|--------------------------------|
| FR-017 (K1) | `core/src/s2/cap.rs`, `core/src/s2/cap_tests.rs` (new sibling) | behaviour (defect) | the probe's radius gives level 1; 180 radii near the powers of two match `MIN_WIDTH.max_level − 1` |
| FR-018 (K2, K3): private `intersects_cell_edges`; `Region::{contains_cell, intersects_cell}` as upstream | `core/src/s2/cap.rs`, `core/src/s2/region.rs`, `cap_tests.rs` | behaviour (parity) | the probe's grid: 38 of 65; K3's cap: no intersection |
| FR-019 (K4) | `cap.rs`, `cap_tests.rs` | behaviour (defect) | length² 1 → 3, 1.7 → 2.3 |
| FR-020 (K5): private `reduce_longitude` | `cap.rs`, `cap_tests.rs`; the exact remainder moved from `core/src/s1/angle_tests.rs` to a shared test helper | behaviour (defect) | K5's cap contains its centre; the reduction equals the exact remainder on 10⁶ inputs |
| FR-022 (K6) | `cap.rs`, `cap_tests.rs` | behaviour (parity) | K6's pair is approximately equal |
| FR-023 (K7): hand-written `Deserialize` | `cap.rs`, `cap_tests.rs` | behaviour | NaN and non-unit centres rejected; valid caps unchanged |
| Documentation: the domains of `contains_point` (unit point), `expanded` (non-negative, finite: +∞ fails the sum's check), `union`'s error margin (a documented deviation) | `cap.rs` | documentation | — |
| FR-011: the quickcheck generator and K5's example | `core/tests/trait_property_tests.rs` | tests | K5's example fails before FR-020 |
| FR-024: a C++ generator with four sections (research R9) | `tools/refgen-cpp/generate_cap.cc` (new), `tools/refgen-cpp/CMakeLists.txt`, `core/tests/data/cpp_cap_vectors.csv` (new), `core/tests/cpp_cap_diff.rs` (new), `verify/tests/values_spec_vs_cpp.rs` | tests | — |

## 7. Mutants (`verify/mutants/M-7NN-<slug>.patch`)

Candidates, one exact-string edit each:

- **Construction:** `from_center_height` with `height` for `2 · height`; `from_center_area`
  dividing by 2π; `from_center_angle` without the π clamp; `empty()` with `ZERO`.
- **Classification:** `is_full` with `>=`; `is_valid` without the radius bound.
- **Measures:** `height` with 0.25; `area` without `max(0, …)`; `centroid` with `1 + 0.5 h`.
- **Membership:** `contains_point` with `<`; `interior_contains_point` without `is_full()`.
- **Relations:** `contains` without the `other.is_empty()` case; `intersects` with `>`;
  `interior_intersects` with `<` for `<=` on zero.
- **Growth:** `add_point` with `>=`… returning the old radius; `add_cap` without the margin
  (expected to survive: the margin is upstream's float safety, not a law in exact terms; then
  recorded, as 007 did); `expanded` returning the cap unchanged.
- **Complement:** the old `STRAIGHT − r`; the centre not negated.
- **Bounds:** the level without `− 1`; `reduce_longitude` with `>=`; `rect_bound` without the
  pole clamp.
- **Equality:** `equal` without the empty case; `approx_eq_with` with the fixed 10⁻¹⁵ again.
- **Cell tests:** `Region::contains_cell` without the complement test; `intersects_cell`
  without the vertex loop; `intersects_cell_edges` with `dot < 0`.
- **The description** (two): `member` with `<`; `upstream_complement_length2` as `4 + l`.
- **The stubs** (two): `pure_chord` without the `(p, p)` case (P-CPGRO-01 must fail, which
  shows the stub's fact is needed); `pure_add` without the zero case.
- **The lemmas:** `ChordAngle::add` returning `rhs`; `Point::chord_angle` without the clamp.

## 8. Records and states

As 004 to 007, under the constitution's rule (v1.2.0). New statuses: **proven by
composition** (with lemma stubs named; 003 used it for the round trip by composition) and
**proven under a stated contract** (FR-003). `verify/values/EVALUATION.md` gains a spec-008
part (research R15).
