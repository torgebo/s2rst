# Research: Formal Verification of `r1::Interval` and `s2::Rect`

Phase 0 of the plan. Each section gives the decision, the rationale and the alternatives
considered. R0 is the planning probe of 2026-10-12, and R1 the re-check of the pinned upstream.
Spec 004's research (`specs/004-formal-verify-cell-interval/research.md`) applies wherever this
file does not change it.

## R0: Planning probe (2026-10-12)

**What ran.** `probe/r0_harnesses.rs` holds 20 Kani harnesses. They ran from a scratch crate
that depends on `s2rst` (`e60a044`) and `s2rst-verify`, so 004's `arc_spec` is reused as it
is. The set-up was a copy of `run-verify.sh`, 4 jobs, a 4 GiB ceiling and 12 minutes per
harness, on Kani 0.68.0 and CBMC 6.11.0. The harnesses measure three things:

- the real interval against a draft real-set description;
- the rectangle against the product of that description and `arc_spec`;
- the expensive operations.

They also test the alternative of proving the rectangle component-wise.

**Results.** ✅ proved, ❌ counterexample, ⏱ hit the 12-minute limit.

| Harness | Measures | Result |
|---------|----------|--------|
| `r0_r1_contains_model` | containment against the real-set description | ✅ 0.2 s |
| `r0_r1_union_least` | the union is the least common superset (three intervals) | ✅ 2.1 s |
| `r0_r1_expanded_finite_contains` | finite margins grow the interval | ✅ 3.4 s |
| `r0_r1_project_crash` | projection lands inside, for every non-NaN point | ✅ 0.7 s |
| `r0_r1_center_inside` | the centre of a finite non-empty interval is in it | ✅ 21 s |
| `r0_r1_expanded_crash` | expansion, every non-NaN interval and every margin | ❌ 0.0 s: "NaN on subtraction" for `[−∞, −2].expanded(−∞)` (K1) |
| `r0_r1_dhd_nonneg` | the directed Hausdorff distance is non-negative | ❌ 0.1 s: NaN on subtraction at infinite bounds (K1) |
| `r0_r1_approx_eq_reflexive` | approximate equality is reflexive | ❌ 0.2 s: `[−1.8e308, ∞]` is not approximately equal to itself, because `\|∞ − ∞\|` is NaN (K1) |
| `r0_rect_valid_model` | `is_valid` against the product description, every bit pattern | ✅ 0.1 s |
| `r0_rect_contains_latlng` | membership of a latitude–longitude, any non-NaN one | ✅ 0.7 s |
| `r0_rect_contains_model` | containment against the product description | ✅ 2.5 s |
| `r0_rect_union_direct` | the union is valid and contains both (product description) | ✅ 48 s |
| `r0_rect_union_componentwise` | the union equals the component unions, bit for bit | ⏱ |
| `r0_rect_intersection_direct` | the intersection is valid and holds every common point | ✅ 28 s |
| `r0_rect_add_point` | adding a point keeps validity and contains the point (if valid) and the original | ✅ 16 s |
| `r0_rect_polar_closure` | the polar closure is valid and contains the original | ✅ 0.5 s |
| `r0_rect_expanded_crash` | expansion keeps validity, every non-NaN margin | ✅ 62 s |
| `r0_rect_expanded_contains` | expansion by finite non-negative margins contains the original | ⏱ |
| `r0_rect_from_center_size` | construction from a centre and size is valid | ✅ 33 s |
| `r0_rect_approx_eq_reflexive` | approximate equality is reflexive | ✅ 256 s |

**What it shows.**

- **Composition pays through the descriptions, not through the proofs.**
  - Every rectangle property stated against the product description proves in 0.1–62 s,
    approximate equality excepted (256 s, from the circular interval's `ieee_remainder`, as in
    004).
  - The component-wise statement, that the union *is* the two component unions bit for bit,
    times out at 12 minutes. Comparing two separate runs of the same float code is
    expensive, even when the code only selects bounds.
  - So the rectangle reuses 004 by reusing `arc_spec`, the generators and the laws. Kani
    re-explores the circular interval's code inside each rectangle proof, cheaply.
- **The real interval's arithmetic meets `∞ − ∞`.**
  - Kani counts a NaN produced by arithmetic as a failed check ("NaN on subtraction"). So the
    crash domain of the arithmetic operations excludes the pairs where `∞ − ∞` can arise, and
    the spec was amended (FR-004, FR-017).
  - The probe also found a corner the survey had not: approximate equality is not reflexive
    for an interval with an infinite bound.
- **Expansion containment is out of per-change reach**, for the rectangle as for the circular
  interval in 004. Validity under expansion proves in 62 s.

## R1: Re-reading the pinned upstream (v0.14.0-40-g37a4501)

**Decision.**

- **K1 is inherited.** `R1Interval::GetLength`, `Expanded` and `ApproxEquals`
  (`r1interval.h:215–220`) have the same arithmetic.
- **The directed Hausdorff distance at infinite bounds may differ.**
  - Upstream computes `std::max(0.0, std::max(hi() − y.hi(), y.lo() − lo()))`, where
    `std::max(a, b)` returns `a` unless `a < b`, so a NaN first argument wins.
  - The library uses `f64::max`, which ignores a NaN.
  - So `[0, ∞]` against `[5, ∞]` gives 0 upstream (the NaN of `∞ − ∞` wins the inner `max`,
    and loses the outer one) and 5 in the library, which is the true distance. Against
    `[−5, ∞]`, both give 0.
  - The C++ vectors (R9) include these rows. A mismatch is marked `deviation:infinite-corner`
    and documented under FR-017. The library's answer is the true distance, so it is not a
    defect.
  - Upstream's 0 is wrong: upstream finding **U-009** (FR-022), confirmed by the vectors on
    2026-10-13 (`[0, ∞]` against `[5, ∞]`: 0; `[−∞, ∞]` against `[−1, ∞]`: 0, where the
    true distance is ∞).
- **The vectors found U-010** (2026-10-13): upstream's `GetCenter` is `0.5 * (lo() + hi())`
  (`r1interval.h:97`), which overflows to ±∞ for finite bounds beyond `DBL_MAX / 2`:
  `[DBL_MAX, DBL_MAX]` has centre ∞. The library's `f64::midpoint` halves first and returns
  the true centre. This is a documented deviation (FR-017's documentation task) and an
  upstream finding.
- **K2 is a deviation that follows Go.**
  - C++ `AddPoint` and `Contains(S2LatLng)` only log an out-of-range point
    (`ABSL_DLOG_IF`, `s2latlng_rect.cc:156–158`, `214–220`) and then use it.
  - Go's `AddPoint` and `ContainsLatLng` return the rectangle or `false`
    (`golang/geo s2/rect.go:139–145`, `284–289`), as the library does.
  - FR-018 documents this as a deviation from C++ that follows Go: an invalid point should not
    silently widen a bound.
- **K3 matches upstream's debug-only checks.** C++ `FromPointPair` logs an invalid point and
  then calls `S1Interval::FromPointPair`, whose `ABSL_DCHECK` fires in debug builds. The
  library's panic is in the same place.
- **A new upstream finding, U-008, a candidate.**
  - `R1Interval::Project` (`r1interval.h:173–176`) calls `std::clamp(p, lo(), hi())` behind an
    `ABSL_DCHECK(!is_empty())`.
  - `std::clamp` is undefined behaviour when `hi < lo`, so in a release build an empty interval
    reaches undefined behaviour.
  - The library's `f64::clamp` asserts `min <= max` in every build, so it panics instead, in
    release builds too.
  - FR-019 documents the panic. Under the upstream-findings rule, a follow-up task records
    U-008.
- **K4 has no upstream counterpart.** Upstream has no serde. The binary decoder's rejection
  of an invalid rectangle is 003's documented deviation (`DEV` in `wire_spec`).

**Rationale.** As in 003 and 004, re-reading the pinned oracle confirms each finding and sorts
it as inherited, a deviation, or upstream's own bug.

## R2: Suite gating, names and records

**Decision.**

- **The feature:** the same `values` feature as 004. The value types are one suite.
- **Prefixes:** `ri_` (the real interval) and `rl_` (the latitude–longitude rectangle). Neither
  matches an existing harness or module name (checked 2026-10-12; `rc_`, `re_` and `rt_` did).
- **Make target:** `make verify-values` gains `P=real` and `P=rect`, and runs all four parts
  without `P`.
- **CI:** the `verify` matrix gains `values-real` and `values-rect`.
- **Records:** `verify/values/` gains sections 3 and 4 in `INVENTORY.md` and sections 4 and 5 in `PROPERTIES.md` (its section 3 is 004's generic laws),
  and a spec-005 part in `EVALUATION.md`.
- **Mutants:** `M-4NN`.
- **Catalogue areas:**
  - the real interval: `P-RIV`, `P-RMEM`, `P-RREL`, `P-RSET`, `P-RMEA`, `P-REXP`;
  - the rectangle: `P-LVAL`, `P-LMEM`, `P-LREL`, `P-LSET`, `P-LBND`, `P-LEXP`, `P-LEQ`.

  None collides with an existing area (`P-RANGE` is 001's).

**Rationale.**

- 004 set up the suite for "the value types" so that later ones join it.
- One catalogue file keeps `property_kinds.rs`'s list short.

**Alternatives.**

- A new feature and records directory per spec: more wiring for the same suite.

## R3: Reference descriptions

**Decision.**

- **`real_spec.rs`** describes the real interval: the closed set of numbers `[lo, hi]`, empty
  when `lo > hi`. It defines:
  - membership and interior membership;
  - containment, intersection, interior containment and interior intersection of real sets:
    by the bounds, which is exact for closed real intervals.
- **`rect_spec.rs`** describes the rectangle:
  - validity (FR-005);
  - membership of a latitude–longitude: valid, latitude in the real interval and longitude in
    the arc;
  - containment, intersection and the interior relations, as products of `real_spec` and
    004's `arc_spec`;
  - the polar closure's meaning: a rectangle touching a pole contains every longitude at that
    pole.
- Both are written from upstream's headers (`r1interval.h`, `s2latlng_rect.h`), never call the
  library's operations, and compare booleans only with `==` (004's Kani finding).

**Rationale.**

- Products of proven descriptions keep the rectangle's statements short and independent of its
  code (FR-007).
- R0 measured them cheap.

**Alternatives.**

- Component-wise statements (the rectangle's operation equals its components' operations):
  they restate the code, and R0 measured them at a timeout.

## R4: Domains and generators

**Decision.**

| Class | Meaning | Generator |
|-------|---------|-----------|
| `Real` | every pair of non-NaN `f64` bounds, infinities and empty intervals included | `any_real()` |
| `FiniteReal` | both bounds finite | `any_finite_real()` |
| `Number` | every non-NaN `f64` | `any_number()` |
| `ValidRect` | struct literals with `rect_spec::valid` | `any_rect()` |
| `AnyLatLng` | any non-NaN latitude and longitude, valid or not | `any_lat_lng()` |
| `ValidLatLng` | latitude in [−π/2, π/2], longitude in [−π, π] | `any_valid_lat_lng()` |

- **The real interval's arithmetic operations** (length, centre, expansion, the directed
  Hausdorff distance, approximate equality) take `FiniteReal` and finite arguments. Their
  infinite corners are documented (FR-017), not proven.
- **Comparison operations** take `Real`.
- **The rectangle's operations** take `ValidRect`.
  - Point arguments are `AnyLatLng` for `add_point` and `contains_lat_lng`, which screen out
    invalid ones (K2).
  - They are `ValidLatLng` for the constructors and `interior_contains_lat_lng` (K3).

**Rationale.** These are the documented domains. Kani's NaN checks rule out `∞ − ∞` (R0).

## R5: How the rectangle reuses 004

**Decision.** The rectangle reuses:

- 004's `arc_spec`, unchanged, for the longitudes;
- the generic equality laws (`laws::eq`);
- the generators `any_interval` and `any_point`, through `rect_spec`.

Rectangle properties are stated directly against the product description, and do not rely on
004's proofs as lemmas.

- The trusted base records the reliance on 004's descriptions, and on its proofs of the
  circular interval's crash freedom, which cover the code every rectangle operation calls.
- The evaluation reports, for SC-010:
  - the share of rectangle properties whose statements use `arc_spec`;
  - the cost compared with the circular interval's own proofs in 004;
  - R0's finding that component-wise statements are the expensive path.

**Rationale.** R0. A proof that leaned on 004's results instead of re-exploring the code would
need stubbing, which the project forbids, or component-wise statements, which time out.

## R6: The FR-020 fix (deserialising a `Rect`)

**Decision.** A hand-written `Deserialize` for `s2::Rect`:

- it reads the same two fields through a private shadow struct, so the format is unchanged;
- it rejects a rectangle that is not valid (`Rect::is_valid`), with an error that says which
  rule failed: a latitude out of range, an invalid longitude arc, or emptiness that disagrees;
- it returns the rectangle as read. There is no canonical form to rebuild, unlike 004's `Cell`.
- `Serialize` stays derived.

**Rationale.**

- **The evidence (2026-10-12).** Through `serde_json`'s default parsing, 1,001,444 valid
  rectangles were round-tripped: random ones, and every combination of the boundary values
  ±π/2, ±π, the floats just inside them, ±0 and 1e-300. 391,960 came back with a bound one ulp
  off, and **none** came back invalid. So exact validity needs no tolerance, unlike `Cell`'s
  stored bounds in 004 R6.
- It follows the same type's binary decoder (003) and the `CellUnion` and `Cell` decisions.

**Alternatives.**

- A tolerance at the boundaries: no measured need.
- Clamping into range: that would silently change data.

## R7: The FR-017 to FR-019 documentation

**Decision.**

- **`r1::Interval`** (FR-017, FR-019):
  - `length`: NaN for `[∞, ∞]` and `[−∞, −∞]`;
  - `center`: NaN for `[−∞, ∞]` (`f64::midpoint` halves each bound, then adds; upstream's
    `0.5 * (lo + hi)` gives NaN too). Found while writing the tasks;
  - `expanded`: a NaN bound for an infinite bound and an infinite margin of the opposite
    sign;
  - `approx_eq_with` and `approx_eq`: false for an interval with an infinite bound, even
    against itself;
  - `directed_hausdorff_distance` at infinite bounds, as R9's rows show;
  - `project`: `# Panics` on an empty interval, in every build (`f64::clamp`), where upstream
    reaches undefined behaviour in release builds (U-008).
- **`s2::Rect`** (FR-018, FR-019):
  - `add_point` and `contains_lat_lng`: an out-of-range latitude–longitude is ignored or not
    contained, a deviation from C++ that follows Go;
  - `from_lat_lng`, `from_point_pair` and `interior_contains_lat_lng`: `# Panics` in debug
    builds on a longitude outside [−π, π] (through the circular interval's checks), and an
    unspecified result in release builds.

## R8: Tiers and budget

**Decision.**

- **Per change:** every harness except the ones below.
- **Weekly (`F=values`):**
  - `rl_exp::p_lexp_01_contains__*`, split by the margins' signs;
  - approximate equality's symmetry and default form, for both types, if they time out as in
    004;
  - a member that times out at 60 minutes becomes "not proven (tool limit)".
- **CI legs and estimates** (R0's times plus 004's crash-harness ratio, on 4 vCPUs):
  - `values-real`: about 2 minutes;
  - `values-rect`: about 6–8 minutes (approximate equality's 256 s is the largest).

**Rationale.** SC-005's 10 minutes per leg, as in 004.

## R9: C++ reference vectors and differential tests

**Decision.** Extend 004's `tools/refgen-cpp/generate_values.cc` with four sections. Appending
keeps every earlier line byte-identical.

- **`R1INTERVAL_OPS`:** about 5,000 random rows with every in-scope result.
- **`R1INTERVAL_SPECIAL`:** empty, infinite, signed-zero and single-point intervals, including
  `[0, ∞]` against `[5, ∞]` and `[−5, ∞]` for the Hausdorff distance.
- **`RECT_OPS`:** about 5,000 random valid rectangles and latitude–longitudes.
- **`RECT_SPECIAL`:** the poles, the antimeridian, the empty and full rectangles, and 004's seam
  corner as a longitude range.

The rows extend:

- `core/tests/cpp_values_diff.rs` for the library;
- `verify/tests/values_spec_vs_cpp.rs` for `real_spec` and `rect_spec`.

Infinite-corner rows where C++ and the library differ are marked `deviation:infinite-corner`
and documented (FR-017). Any other mismatch is a finding.

## R10: Non-vacuity

**Decision.** About 30 `M-4NN` mutants, at least one per proven entry, in 004's format. Two
mutants target the descriptions (`real_spec`, `rect_spec`). Crash harnesses carry covers:

- empty, infinite and single-point real intervals;
- rectangles at a pole, across the antimeridian, empty and full;
- an out-of-range point that `add_point` ignores.

## R11: Trusted base additions

**Decision.** `TRUSTED_BASE.md` gains a section for this feature:

- the reuse of 004's `arc_spec`, laws and generators;
- the circular interval's code, which every rectangle proof executes rather than stubs;
- Kani's NaN checks as part of the crash definition;
- `f64::min`, `f64::max` and `f64::clamp`, as Kani models them.

## R12: The evaluation (FR-016, SC-010)

**Decision.** A spec-005 part of `verify/values/EVALUATION.md`:

- the composition result (R0, R5) and SC-010's numbers;
- the findings: K1–K4 and U-008;
- go or no-go for `r2::Rect`, which is two real intervals and so likely cheap, for
  `ChordAngle` and for `Point`;
- type-level invariants: the rectangle's public fields let any caller build an invalid
  rectangle, which FR-020 only closes for deserialisation.

## R13: Lessons carried over from 004

**Decision.**

- Descriptions compare booleans only with `==`. Kani 0.68.0 mis-models ordered comparisons of
  booleans.
- Relations are those of real sets, not of float points.
- A Kani counterexample that does not reproduce concretely is checked for a tool model first.
- Two-interval properties through the circular interval's `ieee_remainder` are weekly from the
  start.
- Verification jobs of 004 (its weekly members and mutants) and of this feature run one after
  another.
