# Research: Formal Verification of `ChordAngle` and `Angle`

Planning decisions for [spec.md](spec.md). Each section gives the decision, the reason and the
alternatives that were rejected. R0 is the planning probe; R1 is the re-reading of upstream.

## R0: Planning probe (run 2026-10-16)

**What runs.** `probe/r0_harnesses.rs` holds 29 Kani harnesses, run from a scratch crate
that depends on `s2rst` alone, through a copy of `run-verify.sh` (4 GiB and 12 minutes per
harness, Kani 0.68.0, CBMC 6.11.0), on 2026-10-16 from 11:49 to 12:01. Nothing else ran. The
first 24 measure the questions below. The last 5, added after the first run, isolate the
checker's models. Times and verdicts are in `probe/r0_results.tsv`, and the concrete
counterexamples in `probe/r0_counterexamples.txt`. Every counterexample was replayed natively
(`probe/rust/src/bin/replay.rs`).

**The modelled functions (FR-003), first by codegen (`cargo kani --only-codegen`).**

- **`asin` is not modelled.** Kani replaces the C library call with a failing check ("call
  to foreign "C" function `asin` is not currently supported"). `to_angle` reaches it, and
  through it `radians`, `degrees`, `e5`, `e6`, `e7`, the conversion to `Angle` and text
  display: 7 in-scope tool-limit rows. `r0_ca_to_angle_crash` and `r0_ca_e7_crash` fail on
  the check in under 1 s.
- **`Angle::tan` is not modelled** (the same check, for `tan`): 1 tool-limit row.
  `ChordAngle::tan` is `sin / cos` through `sqrt`, and proved crash-free in 1.6 s.
- **`sin`, `sin_cos` and `sqrt` are modelled**, as 006 found. `from_angle`'s range [0, 4]
  proved in 0.2 s, and `sin_cos` in 0.0 s.
- **`round_ties_even` and `floor` are modelled correctly.** The concrete ties (0.5, 1.5, 2.5,
  −2.5, 2.4) hold, and the definition through `floor` holds for every value in the `i32`
  range (6.5 s).
- **Finding 1: the checker's `%` on `f64` is wrong.**
  - `r0_frem_concrete` fails on `1.0 % 2.0 == 1.0`, a concrete input.
  - `r0_frem_which`'s covers show that Kani computes `1 % 2` as 0 and `−0 % 2π` as `+0`.
    Natively both are IEEE `fmod`: 1 and −0.
  - Every failure of `r0_ag_normalized_draft_*` and `r0_round_ties_even_model` (which used `%`
    to test evenness) is this artefact. The replay passes natively: the draft maps
    2.358893696265152·10⁵¹ to −1.786… and is idempotent at 1.53·10⁻³⁰⁷.
  - Consequence for FR-021: `normalized` keeps `%` (R6), since it is exact natively. Its crash
    proof stands, because nothing after the `%` can panic whatever its value. Its value
    properties are not provable by the checker; their evidence is the C++ vectors and a
    property test against an exact big-integer remainder (R3). The spec was amended
    (FR-003, SC-003).
  - No proven library code uses `%` on floats (checked 2026-10-16: only test code in
    `s2/rect.rs`). The finding is recorded in `TRUSTED_BASE.md` and memory.

**The library's `normalized` (K7).** `r0_ag_normalized_library_range` fails in 0.3 s: for
−2.462748394469102·10¹⁶ the library returns 4.0, outside (−π, π] (confirmed natively). That
makes K7 worse than the survey found; the spec's K7 was amended, and the input becomes a
regression test of FR-021.

**The E-forms.**

- Crash freedom of `e5`, `e6` and `e7` over every `f64`, NaN and the infinities included:
  0.0 s.
- **Finding 2: the round trips do not finish.** `from_e7(n).e7() == n` over every `i32`
  times out at 12 minutes, with the library's rounding and with the draft's ties to even.
  So do the draft over 0 ≤ n < 2²⁴, and upstream's identity `E6(n) == E7(10 n)` (the pilot:
  over 25 minutes). The domain is the 2³² integers, so `probe/rust/src/bin/exhaust.rs`
  enumerates it natively, in release mode. Every round trip holds (E5, E6 and E7 with ties to
  even, and E7 with the library's rounding), and so do the identity and upstream's `Degrees(n) ==
  E6(10⁶ n)` and `Degrees(n) == E7(10⁷ n)`, added the same day: 69.2 s for the five
  checks. FR-008 now allows "proven by enumeration" for a single-integer domain.

**The chord angle (option B and the laws).**

| Harness | Result | Time |
|---------|--------|------|
| `r0_ca_from_length2_library_canonical` | fails, as expected: NaN gives NaN (K3) | 0.1 s |
| `r0_ca_from_length2_draft_canonical` | FR-017/FR-018's draft gives a canonical value for every `f64` | 0.0 s |
| `r0_ca_order_laws` | 006's order laws on canonical values | 1.0 s |
| `r0_ca_successor_next`, `r0_ca_predecessor_previous` | the neighbours are the adjacent canonical values | 1.0 s, 0.5 s |
| `r0_ca_neighbours_inverse` | `predecessor(successor(c)) == c` below `INFINITY` | 0.3 s |
| `r0_ca_add_range`, `r0_ca_sub_range` | the arithmetic stays in [0, 4] (through `sqrt`) | 4.5 s, 15.4 s |
| `r0_ca_plus_error_canonical` | `plus_error` is canonical for every non-NaN error | 0.7 s |
| `r0_ca_from_angle_range` | `from_angle` of a finite non-negative angle is in [0, 4] | 0.2 s |

The neighbour harnesses ran on the library before FR-017, over the canonical values only, so
they hold today for those values; FR-017 makes those the only values.

**Decisions from R0.**

- Kani covers every chord-angle law in seconds, so the chord angle's catalogue runs per
  change, with nothing weekly (R12).
- The 8 tool-limit rows (`asin`, `tan`) stand, with these measurements (SC-002).
- The E-form round trips and upstream's integer identities are proven by enumeration, in a
  release-mode test of `verify/` (R12).
- `normalized`'s values rest on the C++ vectors and an exact property test (Finding 1).

## R1: Re-reading the pinned upstream (v0.14.0-40)

Read at `/home/t/sources/s2geometry` (`s1angle.{h,cc}`, `s1chord_angle.{h,cc}`,
`util/math/mathutil.h`, `s2cap.cc`), and run where marked (`probe/cpp/survey.cc`, Release,
x86-64, 2026-10-16).

- **`S1ChordAngle`**:
  - `FromLength2(x)` is `S1ChordAngle(std::min(4.0, x))`: NaN and +∞ become 4 (observed for
    NaN).
  - The private constructor checks `is_valid()` in debug builds only.
  - `is_valid` is `(0 ≤ l ≤ 4) ∨ is_special()`; `is_negative` is `l < 0`, with the TODO
    "only allow Negative()"; `is_infinity` is `l == +∞` (`s1chord_angle.h:361–446`).
  - `Successor` and `Predecessor` map every value ≤ 0 and every negative alike
    (`predecessor(−5)` is −1, observed).
  - `operator+` and `operator−` check `!is_special()` in debug builds
    (`s1chord_angle.cc:92–117`); `sin2` and `cos` check it too (`:132`, `:147`), and `sin`
    and `tan` go through them.
  - `PlusError(e)` is `max(0.0, min(4.0, length2_ + e))` for non-special values; NaN becomes
    4 (observed).
  - `e5`, `e6`, `e7` are `ToAngle().e5()` etc., so `Infinity()` meets `S1Angle`'s range check
    (K5).
- **`S1Angle`**:
  - `Degrees(d)` is `(M_PI / 180) * d`; `degrees()` is `(180 / M_PI) * radians_`; `E5(n)` is
    `Degrees(1e-5 * n)`, and likewise E6, E7. The library computes the same products (the
    factors commute, and `degrees * (PI / 180.0)` is the same product).
  - `e5()` is `MathUtil::Round<int32_t>(1e5 * degrees())`, with debug checks that the value
    fits an `int32_t` (`s1angle.h:248–264`) and a TODO to tighten them to (−180, 180]. On x86
    with GCC, `Round<int32_t, double>` is `FastIntRound`, the `cvtsd2si` instruction: ties to
    even under the default rounding mode, and the "integer indefinite" −2147483648 for any
    value out of range, ±∞ and NaN (observed). Elsewhere it is `static_cast<int32_t>(x ± 0.5)`:
    ties away from zero, and undefined behaviour out of range (`mathutil.h:71–79`,
    `:172–192`).
  - `Normalize()` is `remainder(radians_, 2.0 * M_PI)`, then `−π` becomes `π`
    (`s1angle.cc`). `IsNormalized()` is `−π < r ≤ π`.
  - The header guarantees `Degrees(n) == E6(1000000 * n)`, `Degrees(n) == E7(10000000 * n)`
    and `E6(n) == E7(10 * n)` for integers `n`, and lists exact degree–radian identities
    (`s1angle.h:44–72`).
- **`S2Cap::Decode`** builds the radius with `FromLength2` and validates only in debug builds,
  so a negative radius other than −1 decodes as itself (003 R18).

## R2: Suite gating, names and records

**Decision.** The proofs join the `values` feature and 004's suite:

| | `ChordAngle` | `Angle` |
|--|--------------|---------|
| Modules | `ca_crash`, `ca_val`, `ca_cls`, `ca_ord`, `ca_nbr`, `ca_err`, `ca_ari` | `ag_crash`, `ag_val`, `ag_cnv`, `ag_eform`, `ag_ari` |
| Catalogue ids | `P-CAVAL`, `P-CACLS`, `P-CAORD`, `P-CANBR`, `P-CAERR`, `P-CAARI` | `P-AGVAL`, `P-AGCNV`, `P-AGEF`, `P-AGNRM`, `P-AGARI` |
| `make` | `make verify-values P=chord` | `make verify-values P=angle` |
| CI leg | `values-chord` | `values-angle` |

- Neither `ca_` nor `ag_` occurs in any existing harness or module name in `verify/src/`
  (checked 2026-10-16). Since spec 006's final run, `P=<part>` passes each module as
  `--harness <module>::`, so substring matches in either direction no longer select the
  wrong harnesses.
- `P-CA…` and `P-AG…` collide with no catalogue id (`P-ANC` is 001's ancestry; `P-C…` ids of
  004 are `P-CBND`, `P-CCHD`, `P-CEQ` and others, none `P-CA`).
- Records (FR-026, constitution v1.2.0): `verify/values/INVENTORY.md` §7 (`ChordAngle`) and
  §8 (`Angle`); `verify/values/PROPERTIES.md` §8 and §9; a spec-007 part of
  `verify/values/EVALUATION.md`; `PROPERTIES_BY_KIND.md` and its HTML render;
  `TRUSTED_BASE.md`; `MUTANTS.md`; `VERIFIED_TYPES.md` (`ChordAngle` leaves §2.7 for a new
  §2.7, `Angle` joins it, and §2.8 becomes "reached only through other proofs"); 003's
  `verify/decoding/` records for the `Cap` decoder (R5).

**Alternatives rejected.** A new cargo feature: 004–006 share `values`.

## R3: Reference descriptions

**Decision.** Two plain-Rust descriptions, compiled outside Kani, unit-tested in sibling
`*_tests.rs` files, and checked against the new C++ rows by
`verify/tests/values_spec_vs_cpp.rs` (data-model §1):

- **`chord_spec`**: the canonical set as `f64` lengths (`canonical(l)`: `l == −1`,
  `0 ≤ l ≤ 4`, or `l == +∞`), its order (the order of the reals on that set), and its
  neighbours (`next(l)`, `prev(l)`: the adjacent canonical values, computed on the bit
  patterns of non-negative doubles, with −1 below every length and +∞ above). The
  restatements of upstream's formulas (`upstream_from_length2`, `upstream_plus_error`,
  `upstream_sin2`, `upstream_cos`, `upstream_max_point_error`, `upstream_max_angle_error`,
  `upstream_from_angle`) are kept apart.
- **`angle_spec`**: radians as `f64`; the restatements of upstream's conversions
  (`upstream_degrees`, `upstream_from_degrees`, `upstream_e(k, v)` rounding ties to even and
  returning the saturated value where the library documents it). The reduction's
  description, `remainder_2pi(x)`, lives in `core/src/s1/angle_tests.rs` beside the property
  test that uses it, because it needs `num-bigint`, a dependency of `core` but not of
  `verify/`. It is stated from IEEE-754's definition: the `r = x − n·y` with `n` the integer
  nearest `x / y` (ties to even), `y` the `f64` nearest 2π, mapped to π at −π. It is computed
  exactly with big integers (`num-bigint`, already a dependency of `core`), so it shares
  nothing with the library's `%`. Kani cannot check `normalized` against it (R0, Finding 1):
  the C++ vectors and a quickcheck property in `core/src/s1/angle_tests.rs` (random
  magnitudes from 10⁻³⁰⁰ to 10³⁰⁰) compare the library with it.

**Rationale.** As in 004–006: properties compare the library with an independent description,
and the "bit for bit upstream's formula" statements are explicit restatements checked against
C++.

**Alternatives rejected.** Describing the neighbours through `f64::next_up`: it is stable only
from Rust 1.86 and is what the library would use, so it would not be independent.

## R4: Domains and generators

**Decision.** New generators in `verify/src/value_domain.rs`:

- `any_chord()`: every canonical chord angle (−1, every length in [0, 4], +∞), built through
  the public constants and `from_length2` (after FR-017, every `f64` input gives one);
- `any_chord_length()`: the non-special ones, [0, 4];
- `any_angle()`: every `f64` of radians; `any_finite_angle()` (006) where an operation's
  arithmetic would meet `∞ − ∞`;
- 004's `any_f64()` for the constructors' bit-pattern inputs, and `i32`/`u32` `kani::any()`
  for the E-form constructors.

The arithmetic and trigonometry of a chord angle assume non-special operands (their documented
precondition, FR-019). The E-forms are proven over every angle, NaN and the infinities
included, since FR-023 documents what they return there.

## R5: The FR-017 and FR-018 change (canonical chord angles)

**Decision.**

- `from_length2(x)`: `x < 0` gives `NEGATIVE`; `x ≤ 4` gives `x` unchanged; otherwise (above
  4, +∞, NaN) `STRAIGHT`. Upstream's `std::min(4.0, x)` for every `x ≥ 0` and NaN; the one
  deviation is the negatives (option B).
- `plus_error(e)`: `max(0, min(4, l + e))` with Rust's NaN-ignoring `f64::min` and `f64::max`,
  so a NaN sum becomes 4, as upstream's `std::min(4.0, NaN)`; `clamp` is replaced because it
  propagates NaN.
- `is_infinity`: `l == +∞`, as upstream. After the constructors no value is −∞, so this is
  not observable; it keeps the definition upstream's.
- `from_angle` already maps a negative angle to `NEGATIVE`, and NaN through
  `angle.min(π)` to 4, as C++ (observed). Unchanged.
- Deserialisation (FR-020): a hand-written `Deserialize` that reads the `f64` and rejects it
  unless canonical, with the rule named; the format stays a bare number.
- **The `Cap` decoder** reads the raw radius; it rejects NaN before calling `from_length2`
  (003 D-004), and otherwise uses `from_length2`, so a negative radius becomes `NEGATIVE`.
  Its rustdoc gains the option-B deviation. 003's model `wire_spec::lib_cap` maps a negative
  length to −1, and its rustdoc says why; `ref_cap` (upstream) is unchanged; the decoding
  harnesses that compare the two (`cd_fx::p_fx_08__b32`, `__b33`, and their crash members)
  are re-run, and P-FX-08's row names the deviation. P-FX-03's round trip of valid caps is
  unchanged: a valid cap now has a canonical radius, which encodes and decodes to itself.

**Rationale.** One change at the root (`from_length2`) carries every other constructor, the
arithmetic (which already returns `ZERO` or `STRAIGHT` at its ends) and the decoder.

**Alternatives rejected.** Mapping in `is_valid` or the comparisons: that would keep −5 in
memory and make equality disagree with the bits; option B maps at construction.

## R6: The FR-021 change (`normalized`)

**Decision.** `normalized` computes `r = x % y` with `y = 2.0 * PI`, then `r − y` if `r > π`,
`r + y` if `r ≤ −π`, else `r`.

- `%` on `f64` is IEEE `fmod`, which is exact; `r` has the sign of `x` and `|r| < y`.
- The IEEE remainder differs from `r` by `0` or `±y`. Where `|r| > y/2 = π`, the subtraction
  is exact (Sterbenz: `y/2 ≤ r ≤ 2y`).
- At `r = ±π` (a tie), IEEE's choice between `+π` and `−π` does not matter, since upstream
  maps `−π` to `π`; the branch gives `π` in both cases. So the result equals upstream's
  `remainder` then `−π → π`, for every input, with no dependency and no parity bit.
- ±∞ and NaN give NaN, as upstream's `remainder`.

**Rationale.** Exact, short, dependency-free, and equal to upstream bit for bit, including
the sign of a zero result (both `fmod` and `remainder` return a zero with the sign of `x`).

**Verification.** Kani's model of `%` is wrong (R0, Finding 1). `normalized` gets a crash proof,
since every operation after the `%` is a comparison, an addition or a subtraction of finite
values. Its value is checked by:
- the C++ vectors, with the probe's large angles and R0's −2.462748394469102·10¹⁶;
- a quickcheck property against `remainder_2pi`, the exact big-integer remainder in
  `core/src/s1/angle_tests.rs` (R3);
- regression tests that fail on the old code.

**Alternatives rejected.** The `libm` crate's `remainder`: a new runtime dependency
(constitution, technical constraints) for a three-line function. A wider `x − round(x/y)·y`
with error compensation: still inexact for large `x`.

## R7: The FR-022 change (ties to even)

**Decision.** `e5`, `e6`, `e7` compute `(degrees * 10ᵏ).round_ties_even() as i32`.

- `f64::round_ties_even` is stable since Rust 1.77, inside the MSRV (1.88).
- In range, `cvtsd2si` under the default rounding mode is exactly round-half-to-even, so the
  result equals C++ on x86-64 everywhere in range, not only at ties.
- `as i32` saturates and maps NaN to 0 (FR-023's documented deviation).

**Alternatives rejected.** Rounding through `rint` via the C library: a foreign call, which
Kani would not model.

## R8: Documentation (FR-019, FR-023)

**Decision.**

- `ChordAngle`: the canonical set on the type (option B, with the `Cap` consequence); on
  `Add`, `Sub`, `sin`, `sin2`, `cos`, `tan` the precondition "neither operand special", the
  debug-build panic and the release result. `sin2` and `cos` gain `debug_assert!`s matching
  upstream's; `sin` and `tan` inherit them.
- `Angle`: the E-forms' domain (upstream's (−180°, 180°]), the rounding (ties to even, as C++
  on x86-64; C++ elsewhere rounds them away from zero), and the saturation outside the `i32`
  range with NaN to 0, as a deviation from C++'s −2147483648; the unsigned constructors'
  wrapping; `normalized`'s domain (NaN for ±∞ and NaN). The module's lint justification says
  "saturating cast; out-of-range values documented on `e5`".
- `ChordAngle`'s E-forms: `INFINITY`'s are `i32::MAX`; `NEGATIVE`'s are the E-forms of −1
  radian.

## R9: C++ reference vectors and differential tests

**Decision.** Two sections in `tools/refgen-cpp/generate_values.cc` and
`core/tests/data/cpp_values_vectors.csv`, diffed by `core/tests/cpp_values_diff.rs` and
checked against the descriptions by `verify/tests/values_spec_vs_cpp.rs`:

- `CHORD_ANGLE`: random lengths in [0, 4] and the special cases (−1, −5, −0.001, −∞, ±0,
  4, 4 + ε, +∞, NaN, the smallest subnormal), each with `FromLength2`, `is_*`, `Successor`,
  `Predecessor`, `PlusError` (with ±ε, NaN), the two error bounds, `sin2`, `cos`, `ToAngle`,
  `e5`–`e7`, and pairwise `+`, `−` and `<` for non-special pairs. Rows with a negative input
  other than −1 carry `deviation:negative-canonical`; NaN rows of `S2Cap::Decode` go to 003's
  existing rows.
- `ANGLE`: random angles across magnitudes (10⁻³⁰⁰ to 10³⁰⁰), the exact ties of the probe, the
  integer extremes for `E5`–`E7` and the unsigned forms, multiples of π, ±∞ and NaN, each with
  `Degrees`, `degrees()`, `e5`–`e7`, `Normalized`, `IsNormalized`, `abs`, `sin`, `cos`, `tan`
  and pairwise arithmetic. Out-of-range E-form rows carry `deviation:eform-saturate`.

The C++ generator is built against the local s2geometry install, not in CI (as 004–006).

## R10: Non-vacuity

As 004–006: each proven entry gets at least one `M-6NN` mutant, a single exact-string edit in
`core/` or a description, run by `make verify-mutants`, and the per-spec CI mutant leg gains
`M='M-6*'`. Crash proofs carry `kani::cover!` for each branch of the operation (the sentinels,
the clamps, both sides of the E-form range). `normalized`'s branches after the `%` get
ordinary tests rather than covers, since the checker's `%` decides which it reaches. The
enumeration tests' non-vacuity is a mutant each (an E-form constructor with a perturbed scale,
which breaks the round trip; rounding away from zero does not, as R0's enumeration showed, and
is killed by P-AGEF-01's Kani proof instead).

*After implementation (2026-10-19):* a perturbed scale does not break the round trip either.
`M-644` (`from_e7`'s scale two ulps off) survived `p_agef_02_e7`, because `e7` rounds the
error away, and is killed by `p_agcnv_01_e_forms`, which compares the product bit for bit. The
round trips' mutant is `M-648` (`e7` truncates, 53,623 failures in the first 200,000 values).

## R11: Trusted base additions

`verify/TRUSTED_BASE.md` gains:

- CBMC's `rint`/`nearbyint` for `round_ties_even` and its `sin` (over-approximated within
  [−1, 1]) for `from_angle`;
- that Kani's `%` on `f64` is wrong (R0, Finding 1), so nothing is proven through it, and
  `normalized`'s crash proof rests on the argument that no operation after the `%` can
  panic;
- that the enumeration tests run the library's release build on the workstation's or the
  runner's x86-64 IEEE-754 arithmetic (these functions use no fused multiply-add);
- that `asin` and `tan` are not modelled (R0), so the entries through them rest on the C++
  vectors and the property tests;
- `chord_spec` and `angle_spec`, and every restatement of upstream's arithmetic;
- the split of any domain the tiers need (R12).

## R12: Tiers and budget

**Decision.** Per change: every Kani harness of both types, in two legs (`values-chord`,
`values-angle`). R0's longest is 15.4 s (`Sub`'s range), so each leg is expected under 3
minutes on 4 CPUs, most of it Kani's per-harness set-up (about 5 s each).

- **Enumeration** (`verify/tests/angle_enumeration.rs`, `#[ignore]`d so that the debug
  `cargo test` stays fast): the E5, E6 and E7 round trips and upstream's identities
  (`Degrees(n) == E6(10⁶ n)`, `Degrees(n) == E7(10⁷ n)`, `E6(n) == E7(10 n)`), each over its
  whole integer domain. It runs in release mode as part of `make verify-values P=angle`
  (about 70 s in R0) and of the `values-angle` CI leg. The precedent is 004's `verify/tests/uv_projection.rs`,
  which checks every one of `2³⁰ + 1` boundary values in the plain `cargo test` (004 research
  R8); this one is 16 times larger and runs several checks, so it needs release mode.
- **Weekly:** nothing planned. A member the first run finds over 6 minutes moves to a
  `values-chord-angle` leg of the `slow` job.
- **Not run by Kani:** the 8 tool-limit rows, and `normalized`'s values (R0, Finding 1).

**Alternatives rejected.** Kani with the E-form round trips split by range: the 2²⁴ slice
alone timed out at 12 minutes, so the 2³² domain would need hundreds of weekly members, where
enumeration takes about 70 s.

## R13: The evaluation (FR-016, SC-013)

A spec-007 part of `verify/values/EVALUATION.md`:

- the effort and run times;
- the canonical-value invariant: which laws became whole-type properties, what option B cost
  (the `Cap` deviation, the code touched), and whether the other value types should enforce
  their invariants in the type;
- how far the checker reached into the trigonometric conversions;
- the go or no-go for `LatLng` and `Cap`;
- the ranked follow-ups (U-014's report, the next types).

## R14: Lessons carried over

- Booleans are compared only with `==` (Kani 0.68.0's ordering bug).
- `x / ±∞` is reported as NaN; replay before narrowing a domain.
- Two `sqrt` calls on one input may differ; no harness compares two of them.
- Kani's `--harness` is a substring filter; the `make` parts pass `<module>::`.
- The kill time of a mutant is the run-verify status line's, not the wall time.
- Kani's `%` on `f64` is wrong (R0, Finding 1): no harness or description uses it.
- Upstream findings get a follow-up task before the proof is complete (U-014).
