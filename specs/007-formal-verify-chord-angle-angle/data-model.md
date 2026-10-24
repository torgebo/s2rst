# Data Model: Formal Verification of `ChordAngle` and `Angle`

This feature adds no runtime data. The entities are the verification artifacts, the reference
descriptions and the changes to the library. The schemas are 004's
(`specs/004-formal-verify-cell-interval/data-model.md`), except where this document says
otherwise. The R0 column of §4 holds the planning probe's times and findings (research R0,
2026-10-16).

## 1. Reference descriptions

Both are plain Rust, compiled outside Kani, unit-tested in sibling `*_tests.rs` files, and
checked against the new C++ rows by `verify/tests/values_spec_vs_cpp.rs`. They read only public
accessors (`length2`, `radians`) and never call the operations they describe. They compare
booleans only with `==` (004 R13), and use no `f64` `%` (research R0, Finding 1).

### 1.1 The chord angle (`verify/src/chord_spec.rs`)

Written from upstream `s1chord_angle.{h,cc}`.

| Item | Definition |
|------|------------|
| `canonical(l)` | `l == −1`, or `0 ≤ l ≤ 4` (either zero), or `l == +∞` |
| `special(l)` | `l == −1` or `l == +∞` |
| `less(a, b)` | the order of the reals on canonical lengths (−1 below every length, +∞ above) |
| `next(l)` | the least canonical value above `l`: `−1 → +0`; a length below 4 → the next double up (from the bit pattern; `−0` and `+0` → the smallest subnormal); `4 → +∞`; `+∞ → +∞` |
| `prev(l)` | the greatest canonical value below `l`: `+∞ → 4`; a length above the smallest subnormal → the next double down; the smallest subnormal → `+0`; `±0 → −1`; `−1 → −1` |
| `NEGATIVE`, `ZERO`, `RIGHT`, `STRAIGHT`, `INFINITY` | −1, 0, 2, 4, +∞ (upstream's `Negative()`, `Zero()`, `Right()`, `Straight()`, `Infinity()`) |

**Restatements of upstream's arithmetic**, kept apart from the meaning (FR-007):

| Item | Upstream | Checked against |
|------|----------|-----------------|
| `upstream_from_length2(x)` | `std::min(4.0, x)`, NaN → 4; the deviation: `x < 0` → −1 (FR-017) | `CHORD_ANGLE` rows' `FromLength2` (deviation rows marked) |
| `upstream_plus_error(l, e)` | special → `l`; else `max(0.0, min(4.0, l + e))` with C++'s `std::min`/`std::max` on NaN | `PlusError` rows |
| `upstream_sin2(l)`, `upstream_cos(l)` | `l * (1 - 0.25 * l)`, `1 - 0.5 * l` | `sin2`, `cos` rows |
| `upstream_max_point_error(l)`, `upstream_max_angle_error(l)` | `4.5 * DBL_EPSILON * l + 16 * DBL_EPSILON * DBL_EPSILON`, `DBL_EPSILON * l` | the error rows |

### 1.2 The angle (`verify/src/angle_spec.rs`)

Written from upstream `s1angle.{h,cc}` and `util/math/mathutil.h`.

| Item | Definition |
|------|------------|
| `upstream_from_degrees(d)`, `upstream_degrees(r)` | `(π / 180) · d`, `(180 / π) · r` |
| `upstream_from_e(k, n)` | `upstream_from_degrees(10⁻ᵏ · n)` for k = 5, 6, 7 |
| `upstream_e(k, r)` | `10ᵏ · upstream_degrees(r)` rounded to the nearest integer, ties to even (C++ on x86-64, FR-022); outside the `i32` range `i32::MAX` or `i32::MIN` by sign, NaN → 0 (the library's documented deviation, FR-023) |
| `remainder_2pi(x)` (in `core/src/s1/angle_tests.rs`, not here) | the IEEE-754 remainder of `x` by `y`, the `f64` nearest 2π: `x − n·y` with `n` the integer nearest `x / y`, ties to even, computed exactly with `num-bigint`; `−π` → `π`; NaN for ±∞ and NaN |

`remainder_2pi` lives with the property test in `core`, whose dependency `num-bigint` it uses,
so `verify/` needs no new dependency. Kani cannot run through `%`, and the big-integer
computation is out of its reach (research R0, R6).

## 2. Input domain classes

As research R4:

- the chord angle: `Chord` (every canonical value: `NEGATIVE`, every length in [0, 4],
  `INFINITY`), `ChordLength` (the non-special ones, [0, 4]), and 004's `AnyF64` for the
  constructors' and `plus_error`'s inputs;
- the angle: `AnyAngle` (every `f64` of radians), `FiniteAngle` (006), `AnyF64` for the scalar
  arguments, and every `i32` and `u32` for the E-form constructors;
- `Int32` for the enumeration tests: every `i32`, enumerated.

## 3. Invariants (FR-005)

| Invariant | Proved by |
|-----------|-----------|
| Chord angle: only canonical values are built, from every `f64` input | P-CAVAL-02 |
| Chord angle: every operation that returns one returns a canonical value | P-CAVAL-02 |
| Chord angle: `is_valid` holds for every value | P-CACLS-01 |
| Angle: none; every `f64` is an angle | — |

## 4. Property catalogue (`verify/values/PROPERTIES.md`, sections 8 and 9)

Columns as in 004. "Enumeration" in the R0 column means proven by enumerating the integer
domain natively (FR-008).

### 4.1 `ChordAngle`

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-CAVAL-01 | correct result | The constants are upstream's (−1, 0, 2, 4, +∞), the default is `ZERO`, and `length2` returns the stored length | the constants, `Default`, `length2` | — | not probed |
| P-CAVAL-02 | validity | Every way to build a chord angle gives a canonical value: the constructors for every input, and every operation that returns one on its domain | `from_length2`, `from_angle`, `from_radians`, `from_degrees`, `from_e5`–`from_e7`, `From<Angle>`, `successor`, `predecessor`, `plus_error`, `Add`, `Sub` | `AnyF64`, `AnyAngle`, `i32`, `Chord`, `ChordLength²` | the draft 0.0 s; `plus_error` 0.7 s; `Add` 4.5 s, `Sub` 15.4 s |
| P-CAVAL-03 | correct result | `from_length2` is `NEGATIVE` for a negative input (−∞ included, option B), keeps every length in [0, 4] bit for bit, and gives `STRAIGHT` above 4, at +∞ and for NaN, as upstream | `from_length2` | `AnyF64` | the library fails on NaN (K3), 0.1 s |
| P-CAVAL-04 | correct result | `from_angle` is `NEGATIVE` for a negative angle, `INFINITY` at +∞, and a length in [0, 4] otherwise (NaN gives 4, as C++); the other constructors are `from_angle` of their angle | `from_angle`, `from_radians`, `from_degrees`, `from_e5`–`from_e7`, `From<Angle>` | `AnyAngle`, `AnyF64`, `i32` | range 0.2 s |
| P-CACLS-01 | correct result | `is_zero`, `is_negative`, `is_infinity`, `is_special` and `is_valid` are upstream's definitions; `is_infinity` holds at +∞ only, and `is_valid` for every value | the five classifications | `Chord` | not probed |
| P-CAORD-01 | law | Equality and the ordering are a total order on the canonical values (006's order laws, 004's equality laws) | `PartialOrd`, `PartialEq` | `Chord³` | 1.0 s |
| P-CANBR-01 | correct result | `successor` is `chord_spec::next`: the least canonical value above, `INFINITY` at the top | `successor` | `Chord` | 1.0 s |
| P-CANBR-02 | correct result | `predecessor` is `chord_spec::prev`: the greatest canonical value below, `NEGATIVE` at the bottom | `predecessor` | `Chord` | 0.5 s |
| P-CANBR-03 | law | The neighbours undo each other: `predecessor(successor(c)) == c` below `INFINITY`, and `successor(predecessor(c)) == c` above `NEGATIVE` | `successor`, `predecessor` | `Chord` | 0.3 s (the first) |
| P-CAERR-01 | correct result | `plus_error` leaves a special value unchanged and is upstream's clamp otherwise, bit for bit; a NaN sum gives `STRAIGHT` | `plus_error` | `Chord × AnyF64` | 0.7 s (non-NaN errors) |
| P-CAERR-02 | exact output | `max_point_error` and `max_angle_error` are upstream's formulas, bit for bit | `max_point_error`, `max_angle_error` | `ChordLength` | not probed |
| P-CAARI-01 | correct result | The sum of two non-special values is in [0, 4]; adding `ZERO` returns the other operand; a sum whose lengths add up to at least 4 is `STRAIGHT` | `Add` | `ChordLength²` | 4.5 s |
| P-CAARI-02 | correct result | The difference of two non-special values is in [0, 4]; subtracting `ZERO` returns the first; subtracting a value at least as large gives `ZERO` | `Sub` | `ChordLength²` | 15.4 s |
| P-CAARI-03 | exact output | `sin2` and `cos` are upstream's formulas bit for bit; `sin` is in [0, 1] | `sin2`, `cos`, `sin` | `ChordLength` | not probed |

### 4.2 `Angle`

| Id | Kind | Plain statement | Operations | Domain | R0 |
|----|------|-----------------|------------|--------|----|
| P-AGVAL-01 | correct result | `from_radians` and `radians` keep the bits; `ZERO`, `INFINITY` and the default are upstream's; `abs`, negation and `is_infinite` are the IEEE-754 operations, and negation undoes itself | `from_radians`, `radians`, the constants, `Default`, `abs`, `Neg`, `is_infinite` | `AnyF64` | not probed |
| P-AGVAL-02 | law | Without NaN, equality and the ordering are a total order (006's order laws, 004's equality laws) | `PartialEq`, `PartialOrd` | `AnyAngle³` without NaN | not probed (as P-CAORD-01) |
| P-AGCNV-01 | exact output | `from_degrees`, `degrees` and the E-form constructors are upstream's products, bit for bit; the unsigned constructors are the signed ones of the wrapped value | `from_degrees`, `degrees`, `from_e5`–`from_e7`, `from_unsigned_e6`, `from_unsigned_e7` | `AnyF64`, `i32`, `u32` | not probed |
| P-AGCNV-02 | correct result | Upstream's documented identities hold: `Degrees(n) == E6(10⁶ n)` and `Degrees(n) == E7(10⁷ n)` wherever the product fits, `E6(n) == E7(10 n)` for every `n` with `10 n` in range, and the exact degree–radian identities of `s1angle.h:58–72` | `from_degrees`, `from_e6`, `from_e7`, `from_radians`, `degrees` | `Int32` (enumeration); the listed constants | Kani timeout (12 min); enumeration holds |
| P-AGEF-01 | exact output | `e5`, `e6` and `e7` are `angle_spec::upstream_e`, bit for bit, for every angle: ties to even, saturated outside the `i32` range, 0 for NaN | `e5`, `e6`, `e7` | `AnyAngle` | crash 0.0 s; the statement not probed |
| P-AGEF-02 | correct result | The round trips hold: `from_e5(n).e5()`, `from_e6(n).e6()` and `from_e7(n).e7()` return `n` for every `i32` | the E-forms and their constructors | `Int32` (enumeration) | Kani timeout (12 min, and over 2²⁴); enumeration holds, 69.2 s with P-AGCNV-02 |
| P-AGNRM-01 | correct result | `normalized` is upstream's: the exact IEEE remainder by the `f64` nearest 2π, π for −π, in (−π, π] for every finite angle, NaN otherwise, and idempotent | `normalized` | `AnyAngle` | **not provable by Kani** (its `%` is wrong, R0 Finding 1); the C++ vectors and a property test against `remainder_2pi`. The library before FR-021 returns 4.0 for −2.462748394469102·10¹⁶ (0.3 s) |
| P-AGARI-01 | exact output | Addition, subtraction, the two scalar products and the two divisions are the IEEE-754 operations on radians, bit for bit | `Add`, `Sub`, `Mul<f64>`, `Mul<Angle> for f64`, `Div<f64>`, `Div<Angle>` | `AnyAngle²`, `AnyAngle × AnyF64` | not probed |

**Totals:** 14 chord-angle entries and 8 angle entries, 22 in all. P-AGNRM-01 does not count
toward SC-003 (spec FR-003), so the bar is 19 of 21.

## 5. Operation inventories (`verify/values/INVENTORY.md`, sections 7 and 8)

- **`ChordAngle` (36 entries)**:
  - with crash proofs (28): `from_length2`, `from_angle`, `from_radians`, `from_degrees`,
    `from_e5`, `from_e6`, `from_e7`, `length2`, `is_zero`, `is_negative`, `is_infinity`,
    `is_special`, `is_valid`, `successor`, `predecessor`, `plus_error`, `max_point_error`,
    `max_angle_error`, `sin`, `sin2`, `cos`, `tan`, `Default`, `From<Angle>`, `Add`, `Sub`,
    `PartialEq`, `PartialOrd`;
  - tool limits through `asin` (7): `to_angle`, `radians`, `degrees`, `e5`, `e6`, `e7`,
    `From<ChordAngle> for Angle` (research R0), with their values checked against the C++
    vectors;
  - excluded: `Display` (tool limit: `core::fmt`, and `asin`).

  Domains: `Chord` for the classifications, the order, the neighbours and `plus_error`;
  `ChordLength` for the arithmetic and trigonometry (their documented precondition, FR-019);
  every input for the constructors.
- **`Angle` (31 entries)**:
  - with crash proofs (29): the 20 inherent functions except `tan`, plus `Neg`, `Add`, `Sub`,
    both `Mul`s, both `Div`s, `PartialEq`, `PartialOrd`, `Default`. `normalized`'s crash proof
    rests on the argument in research R6;
  - tool limit (1): `tan` (a C library call, research R0);
  - excluded: `Display`.

  Domains: every `f64`, NaN and the infinities included; every `i32` and `u32`.

## 6. Changes to the library

| Change | Files | Kind | Test (fails without the change) |
|--------|-------|------|--------------------------------|
| FR-017 (K1, K2, option B): `from_length2` maps every negative to `NEGATIVE`; `is_infinity` tests +∞ only; the type's rustdoc states the canonical set and the `Cap` consequence | `core/src/s1/chord_angle.rs`, `core/src/s1/chord_angle_tests.rs` (new sibling) | behaviour (a documented deviation) | `from_length2(−5)` and `from_length2(−∞)` are `NEGATIVE`; `predecessor(from_length2(−5))` is `NEGATIVE` |
| FR-017, the `Cap` decoder: negative radii decode as `NEGATIVE`; NaN still rejected before `from_length2` | `core/src/s2/encoding.rs`, `core/src/s2/encoding_tests.rs` | behaviour (a documented deviation) | C++'s bytes for a cap of radius −5 decode to radius `NEGATIVE`; a NaN radius is rejected |
| FR-018 (K3): `from_length2(NaN)` and `plus_error` with a NaN sum give `STRAIGHT` | `core/src/s1/chord_angle.rs`, `chord_angle_tests.rs` | behaviour (parity) | both give `STRAIGHT` |
| FR-019 (K4): preconditions of `Add`, `Sub`, `sin`, `sin2`, `cos`, `tan`; debug checks in `sin2` and `cos` | `core/src/s1/chord_angle.rs`, `chord_angle_tests.rs` | documentation; debug builds only | `NEGATIVE.cos()` panics in debug builds (`#[cfg(debug_assertions)]` test) |
| FR-020 (K6): a hand-written `Deserialize` that rejects non-canonical values | `core/src/s1/chord_angle.rs`, `chord_angle_tests.rs` | behaviour | 5.0, −5 and NaN are rejected with the rule named; canonical values round-trip bit for bit; the format is unchanged |
| FR-021 (K7): `normalized` is the exact remainder (research R6) | `core/src/s1/angle.rs`, `core/src/s1/angle_tests.rs` (new sibling) | behaviour (a defect fix) | 10⁶, 10¹⁶, 10²⁰, 10³⁰⁰ and −2.462748394469102·10¹⁶ give C++'s results; a quickcheck property against an exact big-integer remainder |
| FR-022 (K8): the E-forms round ties to even | `core/src/s1/angle.rs`, `angle_tests.rs` | behaviour (parity with the reference build) | the probe's ties: `e6` of the angle with degrees × 10⁶ = 2.5 is 2, and `e7` at −2.5 is −2 |
| FR-023 (K5, K9, K10): the E-forms' domain and saturation, the unsigned forms' wrapping, the lint justification | `core/src/s1/angle.rs`, `core/src/s1/chord_angle.rs` | documentation | — |
| FR-024: two C++ vector sections and their differential tests (research R9) | `tools/refgen-cpp/generate_values.cc`, `core/tests/data/cpp_values_vectors.csv`, `core/tests/cpp_values_diff.rs`, `verify/tests/values_spec_vs_cpp.rs` | tests | — |
| FR-025: U-014 (K5, K9, with K8's platform dependence) | `UPSTREAM.md` | documentation | — |
| 003's model of the `Cap` decoder (research R5) | `verify/src/wire_spec.rs`, `verify/decoding/PROPERTIES.md` (P-FX-08) | verification | `cd_fx::p_fx_08__*` re-run |

## 7. Mutants (`verify/mutants/M-6NN-<slug>.patch`)

At least one per proven catalogue entry (SC-004), one exact-string edit each:

- **The chord angle:** `from_length2` keeping negatives (the FR-017 fix removed); `from_length2`
  keeping NaN; `from_length2` clamping at `>=`; `from_angle` without the negative case;
  `is_infinity` with `is_infinite()`; `is_special` without infinity; `successor` of `STRAIGHT`
  stepping by one ulp; `predecessor` of the smallest subnormal returning `NEGATIVE`;
  `predecessor` stepping up; `plus_error` without the special check; `plus_error` with `clamp`
  (NaN); `max_point_error` with 4.0; `Add` without the `STRAIGHT` branch; `Add` returning `rhs`
  for a zero `rhs`; `Sub` without the `ZERO` branch; `sin2` with 0.5; `cos` with `+`;
  `PartialOrd` reversed; `RIGHT` as 1.
- **The angle:** `from_radians` negating; `abs` as identity; `Neg` as identity; `from_degrees`
  with `180 / π`; `degrees` with `π / 180`; `from_e6` with 1e-7; `from_unsigned_e7` through
  `from_e6`; `e7` with `round()` (killed by P-AGEF-01); `e5` with 1e6; `e6` truncating; `Add`
  as subtraction; `Mul<Angle> for f64` dividing; `Div<Angle>` reversed; `from_e7` with a
  perturbed scale (killed by the enumeration, P-AGEF-02); equality not symmetric.
- **`normalized`:** `r <= PI` for `r > PI` and the old formula restored, killed by the C++
  vectors and the property test, not by Kani (R0, Finding 1); recorded with that outcome.
- **The descriptions** (two): `chord_spec::next` skipping `+0`; `angle_spec::upstream_e`
  rounding away from zero.

*After implementation (2026-10-19):* this list was the candidate set; `verify/MUTANTS.md` is the
record (`M-601` … `M-648`, all 39 killed). Three items changed. The perturbed `from_e7` scale
(`M-644`) does not break the round trip: the rounding in `e7` absorbs it, so it survived
`p_agef_02_e7` and is killed by `p_agcnv_01_e_forms` (the product bit for bit); P-AGEF-02's
mutant is `M-648` (`e7` truncates). "Equality not symmetric" was not written: `Angle`'s
equality and order are derived from `f64`'s, so no single edit of the library breaks P-AGVAL-02,
whose evidence is its cover (±0 equal). `is_infinity` with `is_infinite()`, `PartialOrd`
reversed, `plus_error` with `clamp` and `from_length2` at `>=` were not written either; each
entry they aimed at has other killed mutants.

## 8. Records and states

As in 004, plus the constitution's rule (v1.2.0): the inventory, the catalogue, the index of
properties by kind and its HTML render, `TRUSTED_BASE.md`, `MUTANTS.md` and
`VERIFIED_TYPES.md` change in the same change as the harnesses. `verify/values/EVALUATION.md`
gains a spec-007 part (research R13). A new status, **proven by enumeration**, is used for
P-AGCNV-02's integer identities and P-AGEF-02, with the test's name and run time.
