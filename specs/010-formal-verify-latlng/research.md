# Research: Formal Verification of `LatLng`, and `s2::Rect` as a `Region`

Planning decisions for [spec.md](spec.md). Each section gives the decision, the reason and the
alternatives that were rejected. R0 is the planning probe; R1 is the re-reading of upstream.

## R0: Planning probe, the bounding cap (run 2026-10-28)

**What runs.** `probe/rust/src/bin/capbound.rs` (native, release build, at `99b67b1`) draws a
million rectangles from six latitude families (random, within 10⁻ᵏ of either pole, the north
pole alone, a single latitude, symmetric about the equator) and three longitude families (full,
random, narrow down to 10⁻¹¹), and for each the points the rectangle contains among 9 latitudes
× 13 longitudes: both latitude bounds and the points 1 and 3 ulps inside them, a random
latitude, both poles; both longitude bounds and the points 1, 2, 4 and 16 ulps inside them, the
middle and two random longitudes. "Contains" is the library's own `Rect::contains_point`, as
the `Region` law states it. `probe/cpp/capbound_cases.cc` runs the worst cases on upstream.
Results: `probe/rust/capbound_results.txt`, `probe/cpp/capbound_cases.txt`.

| Question | Result |
|----------|--------|
| How many contained points lie outside `cap_bound`? | **10,507,883 of 74,647,924**: 10,018,418 in a pole cap, **489,465 in a cap around the centre** |
| Worst pole-cap miss | the north pole as a rectangle (`[π/2, π/2] × full`) and a point one ulp past the pole: the cap has radius 0, the point's chord² is 2.6·10⁻³² |
| Worst miss of the cap around the centre | a point on the low latitude bound, 5 ulps inside the high longitude bound: 1.8·10⁻²⁴ of chord² outside a cap of 6.5·10⁻¹¹ |
| Upstream on the two worst cases | both contained in the rectangle and missed by `GetCapBound()` (`capbound_cases.txt`) |
| Widening the returned cap by 2.5·10⁻¹⁶ rad and 4ε of its length² | 0 misses |
| By 10⁻¹⁵ rad and 8ε | 0 misses |
| By 10⁻¹⁵ rad alone (100,000 rectangles) | 3,069 misses, all pole caps near a full cap: a relative term is needed |
| By 8ε alone | 9.5 million misses: an absolute term is needed |

**Reading.** A rectangle's point test is in latitude and longitude, through `from_point`'s
`atan2`, whose result is within about an ulp of π/2 (2.2·10⁻¹⁶ rad) of the point's true
latitude near the poles. So a point the rectangle contains can lie that far outside the
rectangle's true region, on any side. Upstream's bound has no allowance for it: the pole cap's
`(1 + 2ε)` covers the rounding of `π/2 − lat` relative to itself, not an absolute error, and
the cap around the centre is built from the vertices alone. The survey's G section (114
misses) found only pole caps because it drew no point next to a vertex.

**Decision (FR-019).** Keep upstream's computation and choice of cap (so the centre is
upstream's bit for bit), then widen the chosen cap: `expanded` by `10⁻¹⁵` rad (4.5 ulps of π/2,
four times the 2.5·10⁻¹⁶ that sufficed), then `plus_error(8ε · length²)` for the relative
rounding of the two chord distances near a full cap (twice the 4ε that sufficed). The
rustdoc states the bound and the reason. Both caps are widened: spec FR-019's own clause ("the
cap around the centre stays upstream's unless verification or the vectors find it missing a
point too") makes R0's 489,465 misses a finding, recorded with U-016.

**Rejected.** Widening only the pole cap (the misses around the centre stay); choosing the cap
after widening (it moves the choice away from upstream's for no gain); an analytic bound from
the error terms alone (the margins of `from_point`, `to_point` and the two chord distances
compound in ways the measurement shows but a short rustdoc cannot carry; the chosen margins are
the measured ones doubled, with the analytic reason in one paragraph).

**Kani.** The law needs the values of `atan2` and `sin_cos` (the point's `LatLng` and the
vertices' points must be near each other), which the contracts do not fix. P-LREG-03 is
recorded as not proven (needs the functions' values), checked by a native test (R0's probe at
20,000 rectangles, as a test), the quickcheck twin's boundary points and the C++ rows.

## R1: Upstream, read and run (v0.14.0-40-g37a4501)

Read in `/home/t/sources/s2geometry/src/s2/s2latlng.{h,cc}` and `s2latlng_rect.cc`; run
through `probe/cpp/survey.cc` and `capbound_cases.cc`.

- **Constructors**: `S2LatLng(S1Angle, S1Angle)` keeps its arguments; `FromRadians`,
  `FromDegrees`, `FromE5/E6/E7`, `FromUnsignedE6/E7` are the pairs of `S1Angle`'s.
  `Invalid()` is `(π, 2π)`; the default is `(0, 0)`.
- **`Latitude(p)`**: `atan2(p[2] + 0.0, sqrt(p[0]² + p[1]²))`; **`Longitude(p)`**:
  `atan2(p[1] + 0.0, p[0] + 0.0)`; `S2LatLng(p)` is the pair, with a log (not a check) when
  invalid, and the comment "already normalized".
- **`is_valid`**: `|lat| ≤ π/2 && |lng| ≤ π`.
- **`Normalized()`**: `Invalid()` if either coordinate is not finite, else
  `(max(−π/2, min(π/2, lat)), remainder(lng, 2π))`. The C++ `min`/`max` of a finite latitude
  equal Rust's `clamp`, signed zeros included.
- **`ToPoint()`**: `(cos θ cos φ, sin θ cos φ, sin φ)` from `SinCos`, with debug checks that
  both coordinates are finite. The library has no check: non-finite input gives NaN
  coordinates. Not a parity defect (upstream's check is debug-only and the release result is
  the same NaN), but the precondition is documented (spec FR-004).
- **`GetDistance`**: the haversine, `2 asin(sqrt(min(1, x)))`, with logs for invalid input.
- **`ApproxEquals(o, max_error = 1e-15)`**: `R2Point::aequal`, `|Δlat| ≤ m && |Δlng| ≤ m`.
- **`ToStringInDegrees`**: `"%f,%f"` of the normalised degrees.
- **Arithmetic**: `+`, `−` and the two scalings on `coords_`, coordinate by coordinate.
- **`S2LatLngRect::GetCapBound`** (`s2latlng_rect.cc:298–332`): the pole cap with
  `(1 + 2ε) · pole_angle`, the cap around the centre from the four vertices when the
  longitude length is at most π, the smaller by height. R0: both miss contained points (U-016).
- **`Contains(const S2Cell&)`** and **`MayIntersect(const S2Cell&)`**: through
  `cell.GetRectBound()`, as the library (`region.rs:117–124`).

U-016 is the only wrong upstream behaviour met (spec FR-022).

## R2: Suite gating, names and records

**Decision.** The proofs join the `values` feature and 004's suite.

| Item | Value |
|------|-------|
| Modules | `ll_crash`, `ll_con`, `ll_fp` (from a point), `ll_tp` (to a point), `ll_nrm`, `ll_val`, `ll_dst`, `ll_apx`, `ll_ari`, `ll_eq`, `ll_lem`; `rl_reg` in `Rect`'s part |
| Shared verification code | `verify/src/latlng_spec.rs` (R4), new stubs in `verify/src/stubs.rs` (R5, R6) |
| Catalogue ids | `P-LLCON`, `P-LLFP`, `P-LLTP`, `P-LLNRM`, `P-LLVAL`, `P-LLDST`, `P-LLAPX`, `P-LLARI`, `P-LLEQ`, `P-LLGEO`, `P-LLDES`; `Rect`'s `P-LREG`; lemmas `L-LL-NN` |
| `make` | `make verify-values P=latlng` |
| CI leg | `values-latlng` |
| Mutants | `M-9NN` (series 9: the CI comment's rule "series N holds spec 00(N+1)'s"; M-8NN are the documented panics') |

- No module name in `verify/src/` ends in `ll_<x>` or contains `ll_crash::` and the like
  (checked 2026-10-28), so `--harness ll_crash::` selects only the new modules.
- `rl_reg` matches `P=rect`'s `values_part` pattern (`rl_*`), so `Rect`'s leg runs it.

## R3: The shared exact reduction (FR-017)

**Decision.** `core/src/s1/angle.rs` gains

```text
pub(crate) fn remainder_2pi(x: f64) -> f64   // C's remainder(x, 2π): exact, ties to even
    r  = x % 2π         // IEEE fmod: exact, the sign of x, |r| < 2π
    if r >  π: r − 2π   // exact (Sterbenz: π < |r| < 2·2π)
    if r < −π: r + 2π
    if |r| == π:        // a tie: the quotient's parity decides
        if x % 4π == r: r          // the quotient is even (4π = 2·2π exactly)
        else: r − copysign(2π, r)  // odd: π ↔ −π, exact
    else r
```

and `Angle::normalized` becomes `remainder_2pi(x)` with −π mapped to π (upstream's
`S1Angle::Normalize`), `LatLng::normalized` uses it as it is. NaN and ±∞ give NaN through
every branch (`Angle`), and `LatLng::normalized` returns `invalid()` before it.

- **Ties.** `x = (2k + 1)·π_f64` exactly, which `f64` can represent only for small `k`; the
  remainder is π for even `k`, −π for odd `k`, with the sign of `x`. The second `%` runs only
  at a tie.
- **Signed zeros.** `fmod` keeps the sign of `x` on a zero result, as `remainder` does: −0 and
  −2π give −0 (upstream; K1).
- **Kani.** `%` is mis-modelled (`KANI_ISSUES.md`), so the reduction's statement is split: the
  correction step (everything after the `%`s) is proven on a drift-guarded copy in
  `latlng_spec` over every `r` with `|r| < 2π` and both possible `r4` (P-LLNRM-03); that `%` is
  IEEE `fmod` is a trusted fact (`TRUSTED_BASE.md`), and the whole is checked against the
  exact `num-bigint` remainder by a property test over every binade and against the C++ rows.
- **`Cap::rect_bound`'s `reduce_longitude`** (008 FR-020) stays: its domain is |x| ≤ 3π/2,
  where one correction is the remainder; folding it in would add a `%` to a function that
  proves without one.

**Rejected.** `x − round(x / 2π) · 2π` (the defect); `f64::rem_euclid` (rounds when it adds
2π); the BigInt remainder at run time (a dependency in `core`'s hot path).

## R4: The reference description (`verify/src/latlng_spec.rs`)

**Decision.** Written from `s2latlng.{h,cc}`, never calling the operations it describes; it
reads `lat` and `lng` only.

- `valid(lat, lng)`: `|lat| ≤ π/2 ∧ |lng| ≤ π` (NaN fails both).
- `clamp_lat(lat)`: `max(−π/2, min(π/2, lat))` written as upstream's two comparisons.
- `INVALID = (π, 2π)`.
- `in_turns(x, r)`: `r` differs from `x` by a whole number of turns, as far as the checker can
  state it without `%`: used only on the correction step's copy (R3).
- Restatements of upstream's arithmetic, kept apart and checked against the C++ rows:
  `upstream_latitude(x, y, z)`, `upstream_longitude(x, y)`, `upstream_to_point(lat, lng)`,
  `upstream_distance(a, b)`, `upstream_approx_eq(a, b, m)`.
- `reduce_fmod_copy(r, r4)`: the drift-guarded copy of the reduction's correction step.

**Rejected.** A set-of-points description: a `LatLng` is a pair of numbers until it is
converted, and the conversions' meaning is upstream's formulas.

## R5: Pure function contracts

**Problem.** A restatement that recomputes `sin`, `cos`, `sin_cos` or `asin` gets a different
contract value from the one the library's call got: 008's contracts return any value in range
on every call. 008 avoided restatements through contracts; `LatLng`'s exact-output entries
(`to_point`, `get_distance`) are nothing but such calls.

**Decision.** `stubs.rs` gains `pure_sin`, `pure_cos`, `pure_sin_cos`, `pure_asin` and
`pure_atan2`: each returns the stored value for an input it has seen, and otherwise a fresh
value satisfying the existing contract (`sin_contract`, …), stored in a `Table`. Soundness is
008's lemma-stub argument: the real function returns one value its contract allows, the same
every time (the C library is deterministic, a trusted fact already in `TRUSTED_BASE.md` for
the lemma stubs), so every behaviour of the real code is one of the stubbed code. The status
of a harness using them is `proven-under-contract`.

**Rejected.** Comparing results within a tolerance (the contracts give no accuracy); leaving the
exact-output entries to the C++ vectors alone (they are the formulas; the vectors check the
values, the proofs that the code is the formula for every value).

## R6: `Rect`'s `Region` laws and the `Cell::rect_bound` contract

`Region for Rect` tests a cell through `cell.rect_bound()` (`region.rs:117–124`), upstream's
`Contains(S2Cell)` and `MayIntersect`; its point test is `contains_lat_lng(from_point(p))`.

- **P-LREG-01, containment implies intersection.** `contains(b) ⇒ intersects(b)` holds for
  every valid non-empty `b` (005's `rect_spec` relations); for an empty `b` it fails (the empty
  rectangle is contained in every rectangle and intersects none). So the harness stubs
  `Cell::rect_bound` with `any_cell_bound` (a pure stub: one valid, non-empty rectangle per
  cell), resting on the lemma **L-LL-01**: `Cell::rect_bound` is valid and non-empty for every
  valid cell, proven under `atan2_contract` with gap 2's u-v abstraction (`any_uv_bound`).
- **P-LREG-02, no contained vertex in a cell it does not intersect.** It needs the bound to
  contain the vertices' `LatLng`s. That relation goes through `atan2` on both sides
  (`Cell::rect_bound`'s and `from_point`'s), so it is a **function contract on
  `Cell::rect_bound`**: `cell_rect_bound_contract` returns a valid rectangle containing
  `pure_from_point(fixed_vertex(k))` for the four `k`. `pure_from_point` is a lemma stub (one
  valid `LatLng` per point, by P-LLFP-02), `fixed_vertex` 008's abstract cell. The contract is
  tested natively (`verify/tests/libm_contracts.rs`): every cell to level 5 (8,190 cells,
  each face's children) and 10⁶ random cells at every level, the real `rect_bound` against the
  real `from_point` of the real vertices. Status `proven-under-contract`.
- **P-LREG-03, the bounding cap.** Not proven: it needs `atan2`'s and `sin_cos`'s values (R0).
- **P-LREG-04, the covering.** Not proven: `Rect::cell_union_bound` is
  `cap_bound().cell_union_bound()`, and 008's P-CPREG-04 reason carries over (the covering's
  level depends on `asin`'s value). Not attempted in Kani.

**Rejected.** A symbolic real cell (out of memory at 4 GiB in 008's R0); an unconstrained
`rect_bound` stub for P-LREG-02 (the law is false for a bound that misses the vertices, so the
proof would fail on a behaviour the real code does not have).

## R7: Input domains

| Class | Definition | Used by |
|-------|------------|---------|
| `AnyLatLng2` | every pair of `f64`, NaN and the infinities included (struct literal through `from_radians`) | constructors, validity, `normalized`, equality, `approx_eq`'s symmetry |
| `NoNanLatLng` | neither coordinate NaN | `approx_eq`'s reflexivity (finite only), the equality laws |
| `ValidLatLng` | 005's (`rect_spec::ll_valid`) | `to_point`'s range, `get_distance`, the composition |
| `FiniteLatLng` | both coordinates finite | `to_point`'s finiteness, `normalized`'s validity |
| `AnyTriple`, `NoNanTriple` | 006's, and 006's without NaN | `latitude`, `longitude`, `from_point` |
| `AnyF64`, `AnyI32²`, `AnyU32²` | 004's, 007's | scalings, E-forms |
| `ValidRect`, the abstract cell | 005's, 008's | `rl_reg` |

## R8: Entries outside the checker (`geo-types`, serde, formatting)

- **`geo-types`** (4 entries): each conversion is one `from_degrees` call or two `degrees`
  calls (`geo_types_interop.rs:114–155`), so their crash freedom and meaning follow from 007's
  `Angle` entries (P-AGCNV-01): **proven by composition**, the argument written in the
  inventory row. Enabling the `geo-types` feature in the verification crate would put
  `geo-types` and `num-traits` into every Kani build for four one-line functions.
- **`Deserialize`** (derived): tested (`latlng_tests::serde_round_trip`), as 008's
  `Deserialize for Cap`. A `LatLng` has no validity invariant, so the derived impl is right.
- **`Display`** and **`to_string_in_degrees`'s formatting**: excluded (`core::fmt`), as every
  type's. `to_string_in_degrees`'s normalisation is `normalized`'s, and the vectors check the
  string.

## R9: C++ reference vectors (FR-021)

**Decision.** A new generator, `tools/refgen-cpp/generate_latlng.cc`, writes
`core/tests/data/cpp_latlng_vectors.csv`, checked by a new `core/tests/cpp_latlng_diff.rs`
(one file per type, 008's pattern). Sections, each row's doubles as hex bits:

| Section | Rows |
|---------|------|
| `LATLNG_CTOR` | `FromRadians`, `FromDegrees`, `FromE5/E6/E7`, `FromUnsignedE6/E7`, `Invalid`, default; random and the integer edges |
| `LATLNG_POINT` | `Latitude`, `Longitude`, `S2LatLng(p)` of random points (unit and not), the poles, ±0 coordinates, the zero vector, subnormals, huge and non-finite coordinates |
| `LATLNG_NORM` | `Normalized()` of the survey's longitudes (special, then random at six scales), the ties ±π, ±3π, ±5π, ±7π, out-of-range and non-finite latitudes |
| `LATLNG_OPS` | `ToPoint`, `GetDistance` (valid pairs and the survey's out-of-range ones), `ApproxEquals` (default and given margins), `+`, `−`, both scalings, `is_valid` |
| `LATLNG_STR` | `ToStringInDegrees` of special and random values |
| `RECT_REGION` | `Contains(cell)`, `MayIntersect(cell)` for random rectangles × cells of levels 0–12 |
| `RECT_CAPBOUND` | `GetCapBound()` (centre and length² bits) and `Contains(p)`, `GetCapBound().Contains(p)` for R0's adversarial points of random and polar rectangles |

- The library matches every row after FR-017 and FR-018. The deviation rows are exactly the
  `RECT_CAPBOUND` radii (FR-019): the test checks the centre bit for bit and that the
  library's radius is upstream's widened by the documented rule, recomputed from upstream's
  value, and that the library's cap contains every point the rectangle contains.
- `verify/tests/values_spec_vs_cpp.rs` checks `latlng_spec`'s restatements on the same rows.
- **Size**: about 30,000 rows, under 3 MB (`cpp_values_vectors.csv` stays under 10 MB, 007).

## R10: The fixes and the tests they change

| Change | Design | Test (fails without it) |
|--------|--------|------------------------|
| FR-017 (K1) | R3 | `latlng_tests`: 10¹⁶ gives 2.637242432414304 (upstream), −2.462748394469102·10¹⁶ and 10³⁰⁰ upstream's bits, ±π and 5π kept, −0 and −2π give −0, `to_string_in_degrees` of (−0, −0) is `-0.000000,-0.000000`; a property test: `normalized` is the exact remainder at every binade and valid for every finite input. `angle_tests` keeps passing (the shared function changes nothing for `Angle`) |
| FR-018 (K3) | `2.0 * x.min(1.0).sqrt().asin()` | the survey's pair gives NaN |
| FR-019 (K4) | R0 | `rect_tests`: R0's two worst cases and the survey's 114 points lie in the bound; a native sweep (20,000 rectangles of R0's families, R0's points) finds no miss; the centre of the bound is upstream's |

Tests that pin the old behaviour, to be updated and listed with the change: none known for
FR-017 and FR-018 (the library's tests compare `normalized` within 10⁻¹³ degrees); FR-019
changes coverings where the wider cap moves a level, so `make test`'s Python and wasm suites
and `core/tests/` are run, and any pinned covering is listed.

## R11: Mutants (`M-9NN`)

At least one per proven entry and one per law instance, one exact-string edit each, 004's
format. Two patch `latlng_spec.rs`, two patch `stubs.rs` (a pure contract that drops its
purity, and the `Cell::rect_bound` contract without the vertex clause, each expected to fail
its law). FR-017 to FR-019's fixes get a reverting mutant each, killed by their tests through
`run-mutants.sh`'s `Expect-Fail` of a `cargo test` target where the proof cannot see them.

## R12: Records, CI and the evaluation

- **Records** (constitution v1.2.0): `verify/values/INVENTORY.md` §12 and `PROPERTIES.md` §11
  for `LatLng`, `Rect`'s §5 gains P-LREG-01 to -04 and the composition; the index by kind and
  its HTML render; `TRUSTED_BASE.md` (pure contracts, the `Cell::rect_bound` contract, IEEE
  `fmod`); `MUTANTS.md`; `VERIFIED_TYPES.md` §2.11 for `LatLng`, §3's `Region` row gains
  `Rect`, and the two missing upstream operations (FR-020); `CORE_TRAITS.md`'s `Region`
  invariants name `Rect`'s instances; `verify/COVERAGE.md`; `UPSTREAM.md` U-016.
- **CI**: `verify.yml`'s `verify` matrix gains `values-latlng`; `core/src/s2/latlng.rs` and
  `core/src/s1/angle.rs` join the path filters if missing; the `mutants` matrix gains series
  `9`.
- **Evaluation**: a spec-010 part of `verify/values/EVALUATION.md` answering SC-013, ranking
  the follow-ups: the remaining `Region` implementors (`Cell`'s float methods, `PointRegion`,
  `CellUnion`'s bounds), the next type, the codec and `Intersects(S2Cell)` (FR-020), the two
  other inexact remainders (`s1::Interval`'s `ieee_remainder`, 008's T058, and the
  projections' `remainder`), and making validity a bounded type (spec Assumptions).
