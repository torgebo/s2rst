# Data Model: Formal Verification of `LatLng`, and `s2::Rect` as a `Region`

The entities of [spec.md](spec.md) as this feature defines them: the reference description,
the input domains, the stubs, the catalogue, the inventory, the library changes and the
mutants. Research decisions are in [research.md](research.md).

## 1. The reference description (`verify/src/latlng_spec.rs`)

Written from `s2latlng.{h,cc}` (research R1, R4). It never calls the operations it describes.

| Item | Definition | Upstream |
|------|------------|----------|
| `valid(lat, lng)` | `|lat| ≤ π/2 ∧ |lng| ≤ π`; false for NaN | `is_valid`, `s2latlng.h:252` |
| `clamp_lat(lat)` | `π/2` if `π/2 < lat`, `−π/2` if `lat < −π/2`, else `lat` (as `max(−π/2, min(π/2, lat))` for finite `lat`, signed zeros kept) | `Normalized`, `s2latlng.cc:64` |
| `INVALID` | `(π, 2π)` | `Invalid()`, `s2latlng.h:230` |
| `reduce_fmod_copy(r, r4)` | the correction step of `s1::angle::remainder_2pi`, copied, drift-guarded (§6) | `remainder(lng, 2π)` |

**Restatements** (kept apart, checked against the C++ rows by
`verify/tests/values_spec_vs_cpp.rs`):

| Restatement | Formula |
|-------------|---------|
| `upstream_latitude(x, y, z)` | `atan2(z + 0.0, sqrt(x·x + y·y))` |
| `upstream_longitude(x, y)` | `atan2(y + 0.0, x + 0.0)` |
| `upstream_to_point(lat, lng)` | `(cos lng · cos lat, sin lng · cos lat, sin lat)` from two `sin_cos` |
| `upstream_distance(lat1, lng1, lat2, lng2)` | `2 asin(sqrt(min(1, x)))`, `x = dlat² + dlng² · cos lat1 · cos lat2`, `d· = sin(0.5 (·2 − ·1))` |
| `upstream_approx_eq(a, b, m)` | `|a.lat − b.lat| ≤ m ∧ |a.lng − b.lng| ≤ m` |

Booleans are compared with `==` only; there is no `%` anywhere in the description.

## 2. Input domains (`verify/src/value_domain.rs`, extended)

| Generator | Domain |
|-----------|--------|
| `any_lat_lng_bits()` | `AnyLatLng2`: every pair of `f64` bit patterns |
| `any_finite_lat_lng()` | `FiniteLatLng`: both coordinates finite |
| `any_valid_lat_lng()` | `ValidLatLng` (005's, unchanged) |
| `any_triple()`, `any_plain_triple()` | `AnyTriple`, `NoNanTriple` (006's) |
| `any_rect()` | `ValidRect` (005's) |
| `stubs::draw_abstract_cell()` | the abstract cell (008's): four box points as vertices |

## 3. Stubs (`verify/src/stubs.rs`, extended)

| Stub | Replaces | Returns | Rests on |
|------|----------|---------|----------|
| `pure_sin`, `pure_cos`, `pure_asin`, `pure_atan2`, `pure_sin_cos` | `f64::sin`, … | one value per input, from a table; a new input gets any value the existing contract (`sin_contract`, …) allows | the contracts (tested, `libm_contracts.rs`) and the C library's determinism (trusted) |
| `sqrt_contract`, `pure_sqrt` | `f64::sqrt` | NaN for NaN and `x < 0`; `x` for ±0 and +∞; otherwise a positive finite value, at most 1 when `x ≤ 1` and at least 1 when `x ≥ 1`; `pure_sqrt` one value per input | IEEE `sqrt` is correctly rounded (trusted), tested |
| `pure_from_point` | `LatLng::from_point` | one valid `LatLng` per point without NaN | P-LLFP-02, purity |
| `any_cell_bound` | `Cell::rect_bound` | one valid, non-empty rectangle per cell | L-LL-01 |
| `cell_rect_bound_contract` | `Cell::rect_bound` | one valid rectangle per cell that contains `pure_from_point(fixed_vertex(c, k))` for the four `k` | tested on the real functions (research R6) |

**Lemma** (harness `ll_lem::l_ll_01`):

| ID | Statement | Stubs |
|----|-----------|-------|
| L-LL-01 | `Cell::rect_bound` is a valid, non-empty rectangle for every valid cell | `atan2_contract`, `any_uv_bound` (gap 2) |

## 4. The property catalogue

### 4.1 `LatLng` (`verify/values/PROPERTIES.md` §11)

| ID | Kind | Statement | Domain | Harness | Expected status |
|----|------|-----------|--------|---------|-----------------|
| P-LLCON-01 | correct result | `new` and `from_radians` keep their arguments bit for bit; `from_degrees`, `from_e5/e6/e7`, `from_unsigned_e6/e7` are the pairs of `Angle`'s constructors; `invalid()` is `(π, 2π)` and the default `(+0, +0)` | `AnyF64²`, `AnyI32²`, `AnyU32²` | `ll_con::p_llcon_01` | proven |
| P-LLFP-01 | exact output | `latitude` and `longitude` are upstream's formulas, `+ 0.0` included, bit for bit; `from_point` and `From<Point>` are the pair | `AnyTriple` | `ll_fp::p_llfp_01` | proven-under-contract (`pure_atan2`, `pure_sqrt`) |
| P-LLFP-02 | validity | `from_point(p)` is valid exactly when no coordinate of `p` is NaN (the infinities included: K2 and more) | `AnyTriple` | `ll_fp::p_llfp_02` | proven-under-contract (`atan2_contract`) |
| P-LLTP-01 | exact output | `to_point` is upstream's formula bit for bit, and `From<LatLng> for Point` is it | `AnyLatLng2` | `ll_tp::p_lltp_01` | proven-under-contract (`pure_sin_cos`) |
| P-LLTP-02 | correct result | for finite coordinates each coordinate of `to_point` is finite and in [−1, 1]; a non-finite coordinate gives NaN in every coordinate it reaches | `AnyLatLng2` | `ll_tp::p_lltp_02` | proven-under-contract (`sin_cos_contract`) |
| P-LLNRM-01 | correct result | `normalized`'s latitude is `clamp_lat`, bit for bit; a non-finite coordinate gives `invalid()` | `AnyLatLng2` | `ll_nrm::p_llnrm_01` | proven |
| P-LLNRM-02 | exact output | `normalized`'s longitude is `remainder(lng, 2π)`, exactly: ties to even, the sign of zero kept | `FiniteLatLng` | by composition: P-LLNRM-03 and IEEE `fmod` (trusted); `latlng_tests::normalized_is_the_exact_remainder` | proven-by-composition |
| P-LLNRM-03 | exact output | the correction step: for every `r` with `|r| < 2π` and each `r4` the quotient's parity allows, the result is in [−π, π], differs from `r` by exactly 0 or ±2π, and at `|r| = π` keeps `r` exactly when `r4 = r` | `|r| < 2π` | `ll_nrm::p_llnrm_03` on `reduce_fmod_copy` | proven |
| P-LLNRM-04 | validity | `normalized` is valid for every finite input; `invalid()` is not valid | `FiniteLatLng` | by composition: P-LLNRM-01, -03, `fmod`'s range (trusted) | proven-by-composition |
| P-LLNRM-05 | law | `normalized` is the identity on valid input, bit for bit | `ValidLatLng` | by composition: `fmod(x, 2π) = x` for `|x| < 2π` (trusted), P-LLNRM-01, -03 | proven-by-composition |
| P-LLVAL-01 | correct result | `is_valid` is `latlng_spec::valid` for every pair; the default is valid | `AnyLatLng2` | `ll_val::p_llval_01` | proven |
| P-LLDST-01 | exact output | `get_distance` is upstream's formula bit for bit (after FR-018) | `AnyLatLng2²` without `∞ − ∞` | `ll_dst::p_lldst_01` | proven-under-contract (`pure_sin`, `pure_cos`, `pure_sqrt`, `pure_asin`) |
| P-LLDST-02 | correct result | for valid inputs the distance is in [0, π], never NaN | `ValidLatLng²` | `ll_dst::p_lldst_02` | proven-under-contract |
| P-LLDST-03 | correct result | the distance from a valid `LatLng` to itself is +0 | `ValidLatLng` | `ll_dst::p_lldst_03` | proven-under-contract |
| P-LLDST-04 | law | symmetric within 10⁻¹⁴ rad (not bit for bit: the formula multiplies the cosines in argument order) | `ValidLatLng²` | — | not-proven (needs the functions' values); `prop_distance_symmetric`, the vectors |
| P-LLAPX-01 | correct result | `approx_eq_with` is upstream's `aequal`, bit for bit; `approx_eq` is it at 10⁻¹⁵ | `AnyLatLng2²` without `∞ − ∞` × `AnyF64` | `ll_apx::p_llapx_01` | proven |
| P-LLAPX-02 | law | `approx_eq_with` is symmetric for every input, and reflexive for finite coordinates and a non-negative margin | as P-LLAPX-01 | `ll_apx::p_llapx_02` | proven |
| P-LLARI-01 | exact output | `+`, `−` and both scalings are `Angle`'s operations coordinate by coordinate, bit for bit; `k * ll` is `ll * k` | without `∞ − ∞` and `0 · ∞` | `ll_ari::p_llari_01` | proven |
| P-LLEQ-01 | law | `==` is coordinate-wise `f64` equality, and an equivalence on pairs without NaN (`laws::eq::all`) | `AnyLatLng2³` | `ll_eq::p_lleq_01` | proven |
| P-LLGEO-01 | correct result | the `geo-types` conversions are degrees with x the longitude and y the latitude, both ways | every `f64` pair | by composition: one `from_degrees` or two `degrees` calls (`geo_types_interop.rs:114–155`), 007's P-AGCNV-01; `geo_types_interop` tests | proven-by-composition |
| P-LLDES-01 | correct result | deserialisation accepts every pair and round-trips | every serialised `LatLng` | — | not-proven (tested) |

**Counted for SC-003**: the 19 entries other than P-LLDST-04 (needs the functions' values) and
P-LLDES-01 (outside the checker). The bar is 18.

### 4.2 `s2::Rect` as a `Region` (`PROPERTIES.md` §5, appended)

| ID | Kind | Statement | Domain | Harness | Expected status |
|----|------|-----------|--------|---------|-----------------|
| P-LREG-01 | law | containing a cell implies intersecting it (`laws::region::contains_implies_intersects`) | `ValidRect` × every valid cell | `ll_reg::p_lreg_01` (stub `any_cell_bound`, L-LL-01) | proven-under-contract |
| P-LREG-02 | law | a cell the rectangle does not intersect has no vertex it contains (`laws::region::no_intersect_no_vertex`) | `ValidRect` × the abstract cell | `ll_reg::p_lreg_02` (stubs `fixed_vertex`, `pure_from_point`, `cell_rect_bound_contract`) | proven-under-contract |
| P-LREG-03 | law | the bounding cap contains every contained point (`laws::region::cap_bound_contains_points`, FR-019's bound) | `ValidRect` × every point | — | not-proven (needs `atan2`'s and `sin_cos`'s values); `rect_tests::cap_bound_contains_boundary_points`, the quickcheck twin, `RECT_CAPBOUND` |
| P-LREG-04 | law | every contained cell is covered (`laws::region::covering_covers`) | `ValidRect` × every valid cell | — | not-proven (needs `asin`'s value, as P-CPREG-04) |
| P-LMEM-03 | correct result | `contains_point(p)` is `contains_lat_lng` of `from_point(p)`, a valid `LatLng` for every point without NaN (FR-012) | `ValidRect` × `NoNanTriple` | `ll_reg::p_lmem_03` (stub `pure_from_point`) | proven-under-contract |

## 5. The operation inventory (`verify/values/INVENTORY.md` §12)

33 entries, grouped as in §9's style. Domains are research R7's.

| # | Entries | Domain | Crash harness | Expected |
|---|---------|--------|---------------|----------|
| 1 | `new`, `from_radians`, `from_degrees`, `invalid`, `Default`, `is_valid` | `AnyF64²` | `ll_crash::c_construction` | proven |
| 2 | `from_e5`, `from_e6`, `from_e7`, `from_unsigned_e6`, `from_unsigned_e7` | `AnyI32²`, `AnyU32²` | `ll_crash::c_e_forms` | proven |
| 3 | `latitude`, `longitude`, `from_point`, `From<Point> for LatLng` | `AnyTriple` | `ll_crash::c_from_point` (`atan2_contract`) | proven-under-contract |
| 4 | `to_point`, `From<LatLng> for Point` | `AnyLatLng2` (finite documented) | `ll_crash::c_to_point` (`sin_cos_contract`) | proven-under-contract |
| 5 | `normalized` | `AnyLatLng2` | `ll_crash::c_normalized` (does not depend on `%`'s value) | proven |
| 6 | `get_distance` | `ValidLatLng²` (documented); every pair without infinities outside it (`o_get_distance`) | `ll_crash::c_get_distance`, `o_get_distance` (`sin_contract`, `cos_contract`, `asin_contract`) | proven-under-contract |
| 7 | `approx_eq`, `approx_eq_with` | `AnyLatLng2²` without `∞ − ∞` × `AnyF64` | `ll_crash::c_approx_eq` | proven |
| 8 | `Add`, `Sub`, `Mul<LatLng> for f64`, `Mul<f64> for LatLng` | without `∞ − ∞` and `0 · ∞` | `ll_crash::c_arithmetic` | proven |
| 9 | `PartialEq` | `AnyLatLng2²` | `ll_crash::c_construction` | proven |
| 10 | `to_string_in_degrees` | — | excluded: `core::fmt` (its normalisation is entry 5's) | — |
| 11 | `Display` | — | excluded: `core::fmt` | — |
| 12 | `Deserialize` | every serialised `LatLng` | `latlng_tests::serde_round_trip` | tested |
| 13 | `geo-types`: `From<Coord> for LatLng`, `From<LatLng> for Coord`, `From<geo::Point> for LatLng`, `From<LatLng> for geo::Point` | every `f64` pair | by composition (007's `ag_crash::c_construction_and_accessors`) | proven-by-composition |

That is 26 entries with Kani crash proofs (rows 1–9), 4 by composition, 1 tested and 2
excluded.

## 6. Library changes

| Change | Where | What |
|--------|-------|------|
| FR-017 | `core/src/s1/angle.rs` | `pub(crate) fn remainder_2pi(x: f64) -> f64` (research R3); `Angle::normalized` maps its −π to π |
| FR-017 | `core/src/s2/latlng.rs` | `normalized` uses `remainder_2pi`; the rustdoc names the ties and the signed zeros |
| FR-018 | `core/src/s2/latlng.rs` | `get_distance`: `2.0 * x.min(1.0).sqrt().asin()` |
| FR-019 | `core/src/s2/rect.rs` | `cap_bound`: upstream's cap, then `expanded(1e-15 rad)` and `plus_error(8ε · length²)`; the rustdoc states the deviation (U-016) |
| Tests | `core/src/s2/latlng_tests.rs` (new), `rect_tests.rs`, `angle_tests.rs` | research R10 |
| Docs | `core/src/s2/latlng.rs` | `to_point`'s finite precondition, `get_distance`'s NaN outside its domain (spec FR-004, SC-010) |

The drift guard (`latlng_spec_tests::reduce_fmod_copy_is_the_librarys`) compares the copy's
body with the correction step's in `angle.rs`, which is factored as
`fn reduce_fmod(r: f64, x: f64) -> f64` so that the copy has a body to compare; the copy takes
`r4` (the value of `x % 4π`) instead of `x`, and the guard compares the lines after that
substitution.

## 7. Mutants (`verify/mutants/M-9NN-<slug>.patch`)

| Id | Defect | Target |
|----|--------|--------|
| M-901 | `from_degrees` swaps the coordinates | P-LLCON-01 |
| M-902 | `invalid()` is `(π, π)` | P-LLCON-01, P-LLVAL-01 |
| M-903 | `latitude` drops `+ 0.0` | P-LLFP-01 |
| M-904 | `longitude` takes `atan2(x, y)` | P-LLFP-01 |
| M-905 | `latitude` uses `x·x + x·y` | P-LLFP-01 |
| M-906 | `latitude` takes `atan2(sqrt(…), z)`, which leaves the range | P-LLFP-02 |
| M-907 | `to_point`'s `y` is `sin lng · sin lat` | P-LLTP-01 |
| M-908 | `to_point`'s `x` is `cos lng + cos lat` (leaves [−1, 1]) | P-LLTP-02 |
| M-909 | `normalized` clamps to [−π/2, π] | P-LLNRM-01 |
| M-910 | `normalized` returns the input for non-finite coordinates | P-LLNRM-01 |
| M-911 | the correction step keeps `r` above π | P-LLNRM-03 |
| M-912 | the tie keeps `r` for an odd quotient | P-LLNRM-03 |
| M-913 | `is_valid` uses `< π` for the longitude | P-LLVAL-01 |
| M-914 | `get_distance` drops the `min(1)` | P-LLDST-01 |
| M-915 | `get_distance` uses `lat1` twice in the cosines | P-LLDST-01 |
| M-916 | `get_distance` omits the factor 2 | P-LLDST-01 |
| M-917 | `get_distance` returns `asin` of `x` without the root | P-LLDST-01 |
| M-918 | `get_distance` uses `0.5 · (lat2 + lat1)` | P-LLDST-03 |
| M-919 | `approx_eq_with` uses `<` | P-LLAPX-01 |
| M-920 | `approx_eq` uses 10⁻¹⁴ | P-LLAPX-01 |
| M-921 | `approx_eq_with` compares `self.lat − other.lng` | P-LLAPX-02 |
| M-922 | `Sub` adds the longitudes | P-LLARI-01 |
| M-923 | `Mul<LatLng> for f64` scales only the latitude | P-LLARI-01 |
| M-924 | `Rect::contains`'s cell test through `contains_lat_lng` of the bound's low corner | P-LREG-01 |
| M-925 | `intersects_cell` is `contains_cell` | P-LREG-01 |
| M-926 | `intersects_cell` tests only the latitudes | P-LREG-02 |
| M-927 | `contains_point` tests a `LatLng` with the coordinates swapped | P-LMEM-03 |
| M-928 | spec mutant: `latlng_spec::valid` uses `< π/2` | P-LLVAL-01 |
| M-929 | spec mutant: `upstream_to_point` swaps `sin` and `cos` of the latitude | P-LLTP-01 |
| M-930 | stub mutant: `pure_from_point` may return an invalid `LatLng` | P-LMEM-03 |
| M-931 | stub mutant: `cell_rect_bound_contract` drops the vertex clause | P-LREG-02 |
| M-932 | FR-017 reverted: `normalized` rounds the quotient | `latlng_tests` |
| M-933 | FR-018 reverted: `get_distance` clamps after the root | `latlng_tests`, P-LLDST-01 |
| M-934 | FR-019 reverted: `cap_bound` without the widening | `rect_tests` |

The final list follows the harnesses as written; changes are recorded in `MUTANTS.md`.
