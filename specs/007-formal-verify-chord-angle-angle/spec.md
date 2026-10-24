# Feature Specification: Formal Verification of `ChordAngle` and `Angle`

**Feature Branch**: `004-specs-2/1`

**Created**: 2026-10-16

**Status**: Implemented (2026-10-19); amended at planning (2026-10-16) with research R0's
results (K7, FR-003, FR-008, SC-002, SC-003)

**Input**: User description: "007 : Add verification to the next two types." On 2026-10-16 the
maintainer answered `ChordAngle`'s validity question (spec 006 FR-025) with option B: "Go for
B". The second type, `s1::Angle`, is the recommendation given with that question; the
maintainer did not object.

## Context

### Where the earlier features left off

- **001–003** proved `CellId`, `CellUnion` within size bounds, and the decoding layer.
- **004–006** proved six value types: `Cell`, `s1::Interval`, `r1::Interval`, `s2::Rect`,
  `r2::Rect` and `Point`. Their records are complete. 004's own evaluation is still interim.
- **006 deferred `ChordAngle`** on 2026-10-13 until its validity question was answered. Its
  evaluation ranked it first among the next steps: "Answer `ChordAngle`'s validity question
  and take it next … the order laws are ready, and it is the distance type of the query
  layer."

### Why these two types

- **`ChordAngle`**, the distance between two points on the sphere, is stored as the squared
  length of the straight chord between them.
  - Its squared length runs from 0 (the same point) through 2 (a quarter circle) to 4
    (antipodal points). It rises with the angle, so comparing two distances is one float
    comparison. That is why the query layer measures distance with it: 27 source files and
    the Python bindings use it.
  - Two sentinels lie outside [0, 4]. `NEGATIVE` (−1) is smaller than any distance and is
    the radius of the empty cap. `INFINITY` (+∞) is larger than any distance and means "no
    limit" in queries.
  - Its operations are comparisons, clamps, neighbours in the float order, and short formulas
    with `sqrt` and `sin`. The pilot proved its bit-level core in 0.1–1.5 s per harness.
  - 006 left the generic order laws (`laws::ord`) ready for it.
- **`Angle`**, an angle in radians, is the type `ChordAngle` converts to and from.
  - `LatLng` is two of them, and 57 source files and both bindings use it.
  - Its surface is IEEE-754 arithmetic, the conversions to degrees and to the integer E5, E6
    and E7 forms, and the reduction to (−π, π].
  - It has no validity rule: every `f64` is an angle.
  - The pilot covered it, except the E5/E6/E7 round trips, which did not finish in 25 minutes.

### The validity decision (option B)

Upstream counts every negative squared length as valid. Its own code says "TODO: Consider
stricter check here -- only allow Negative()". So −1, −5 and −0.001 all mean "smaller than any
distance" (an empty cap's radius), but they do not behave as one value:

- −5 sorts below −1 and is not equal to it;
- `predecessor(−5)` returns −1, which is larger than −5;
- two empty caps with radii −1 and −5 are the same set with unequal radii.

The order and neighbour laws hold only on the canonical values {−1} ∪ [0, 4] ∪ {+∞}.

The maintainer chose **B**: every constructor maps every negative value to `NEGATIVE`. The
type then holds exactly the canonical values, every law holds for every value, and validity is
a property of the type. The field is already private, so the constructors can enforce it.

The cost is one documented deviation. A `Cap` encoded by C++ with radius −5 decodes as an
empty cap with radius −1 and re-encodes as −1, where C++ keeps −5. Options A (keep upstream's
rule) and C (only `NEGATIVE` valid, so the decoder rejects such bytes) were not chosen.

### Survey findings (confirmed 2026-10-13 and 2026-10-16)

The probes are kept in `probe/`, with their outputs: the same cases on upstream C++
(`v0.14.0-40`, Release, x86-64) and on the library (release build).

**`ChordAngle`**

- **K1, many negative values.** `from_length2(−5)` is −5, valid, and not `NEGATIVE`. Its
  predecessor is −1. C++ gives the same (observed). Resolved by option B.
- **K2, minus infinity is both.** `from_length2(−∞)` is negative and also infinite in the
  library, since `is_infinity` tests `is_infinite()`. Upstream's `is_infinity` tests `+∞` only
  (read from `s1chord_angle.h:440`). Under option B, −∞ becomes `NEGATIVE`.
- **K3, NaN, a silent deviation.** `from_length2(NaN)` returns NaN, which is invalid. So does
  `plus_error` on a NaN length or with a NaN error. Upstream returns `STRAIGHT` (4) through
  `std::min(4.0, NaN)` (observed). The `Cap` decoder rejects a NaN radius on purpose, a
  documented deviation (003 D-004), and must keep doing so.
- **K4, the special values in the arithmetic.**
  - Addition and subtraction check for the special values in debug builds only, as
    upstream does.
  - `sin`, `sin2`, `cos` and `tan` do not check at all. Upstream checks `sin2` and `cos`, and
    through them `sin` and `tan`, in debug builds (`s1chord_angle.cc:132,147`). Unchecked,
    `NEGATIVE.sin()` is NaN and `INFINITY.cos()` is −∞.
  - Nothing documents a precondition or a panic.
- **K5, `e5`, `e6` and `e7` of `INFINITY`.** The library returns `i32::MAX`. C++ returns
  −2147483648 in release builds (observed), and its debug builds fail a check: `INFINITY` is a
  valid chord angle, but its angle is outside the range `S1Angle::e5` requires. This is an
  upstream finding (U-014, with K9).
- **K6, deserialisation accepts invalid values** such as 5.0 (observed 2026-10-13). The
  derived deserialiser accepts every `f64` the format carries, −5 included.

**`Angle`**

- **K7, `normalized` is inexact for large angles.**
  - Upstream reduces with the IEEE `remainder`, which is exact.
  - The library computes `x − round(x / 2π) · 2π`, which loses the low bits. It returns
    −0.3575641670031473 for 10⁶ where C++ returns −0.35756416704675331, about 4·10⁻¹¹ off.
  - It returns π for 10¹⁶ and 0 for 10²⁰ and 10³⁰⁰, where C++ returns 2.637…, 1.895… and
    −0.723… (observed).
  - It can leave (−π, π]: for −2.462748394469102·10¹⁶ it returns 4.0. Kani found this
    input in planning (research R0), and a native run confirmed it.
  - This is a library defect.
- **K8, rounding to E5, E6 and E7 at exact ties.** Where degrees × 10ᵏ is exactly a half
  integer:
  - C++ on x86-64 rounds to even (`MathUtil::Round<int32_t>` is `FastIntRound`, the
    `cvtsd2si` instruction). For degrees × 10⁶ = 0.5, 2.5, 4.5 and 6.5 it gives 0, 2, 4 and 6.
  - The library rounds away from zero and gives 1, 3, 5 and 7 (observed).
  - Upstream's generic code, used on other processors, rounds away from zero (`x ± 0.5`
    truncated), so upstream itself depends on the platform.
- **K9, outside the integer range.**
  - Upstream documents that `e5`, `e6` and `e7` need an angle in (−180°, 180°]
    (`s1angle.h:127`) and checks it in debug builds (`s1angle.h:250`).
  - In release builds C++ returns −2147483648 for every value out of range, +∞ and NaN
    included (observed): the sign flips for large positive angles.
  - The library saturates: `i32::MAX` for large positive angles and +∞, `i32::MIN` for large
    negative ones, and 0 for NaN. Nothing documents this. The module's lint justification
    says the values are "clamped to valid range", which no code does.
- **K10, the unsigned forms wrap.** `from_unsigned_e6` and `from_unsigned_e7` reinterpret a
  `u32` above `i32::MAX` as negative, as upstream's `static_cast` does. This is undocumented.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for both types (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope operation of
`ChordAngle` and `Angle` completes without panicking, overflowing or hitting undefined
behaviour, for every input in its documented domain. Both are a single `f64`, so the proofs
cover whole domains with no size bound.

**Why this priority**: Crash freedom is what the constitution already requires (Principle II).
K4 and K9 show that the domains are not yet documented.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope operation as proven free of panics, overflow and undefined behaviour
over its stated domain, with no unproven obligations.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification command,
   **Then** it reports each in-scope operation by name as proven crash-free, states its
   domain, and exits successfully.
2. **Given** an operation with a documented precondition (for example "neither operand is a
   special value"), **When** verification runs, **Then** the proof assumes exactly that
   precondition and no stronger one.
3. **Given** an internal consistency check that runs only in debug builds, **When**
   verification runs, **Then** the proof shows that it never fires within the domain.
4. **Given** an in-scope operation deliberately changed so that it can panic inside its
   domain, **When** verification runs, **Then** it fails and names the operation and an input.

---

### User Story 2 - A chord angle that holds only canonical values (Priority: P1)

As the maintainer, I want `ChordAngle` to hold only {`NEGATIVE`} ∪ [0, 4] ∪ {`INFINITY`},
whichever way a value is made, and a proof of it, so that its order and neighbour laws hold
for every value it can hold.

**Why this priority**: The maintainer's decision (option B) changes behaviour. It is the
precondition for the order laws in Story 3, and the first type-level invariant in the value
types.

**Independent Test**: Build a chord angle from every constructor, the deserialiser and the
`Cap` decoder with a negative input other than −1, and with −∞. Each yields `NEGATIVE`. The
proof that every operation returns a canonical value passes.

**Acceptance Scenarios**:

1. **Given** any `f64` input, **When** a chord angle is built from it, **Then** the result is
   canonical, and is `NEGATIVE` exactly when the input is negative (−∞ included).
2. **Given** the bytes of a C++-encoded cap whose radius is −5, **When** the library decodes
   them, **Then** it returns an empty cap with radius `NEGATIVE`, and the deviation is
   documented on the decoder and recorded in the C++ vectors.
3. **Given** the bytes of a cap whose radius is NaN, **When** the library decodes them,
   **Then** it still rejects them (003 D-004).

---

### User Story 3 - Proven meaning against independent descriptions (Priority: P2)

As the maintainer, I want a catalogue of functional properties for each type, proven against
independent descriptions:

- a chord angle is a squared chord length in [0, 4], ordered as a real number, with one
  sentinel below every length and one above, so its neighbours are those in that ordered set;
- an angle is an `f64` of radians whose conversions are upstream's formulas, and whose
  reduction is the exact IEEE remainder by 2π.

**Why this priority**: This extends the parity guarantee (Principle I) from sampled inputs to
every input. `successor` and `predecessor` decide the query layer's bounds, and the E7 form is
how latitudes and longitudes are stored.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven or as not proven with a written reason. Each proven property fails when a matching
defect is injected.

**Acceptance Scenarios**:

1. **Given** the property catalogue, **When** verification runs, **Then** every property
   marked proven is discharged with no remaining obligations.
2. **Given** any proven property, **When** a defect that violates it is introduced, **Then**
   verification fails for that property.
3. **Given** a property about a value computed by upstream's arithmetic (the degree
   conversions, the E-forms, the chord formulas), **When** a reviewer reads it, **Then** it
   states the result bit for bit against a restatement of upstream's formula, checked against
   the C++ vectors, or states its tolerance and why.

---

### User Story 4 - Regression protection for future changes (Priority: P3)

As a contributor changing either type, I want the proofs to run automatically in CI, and to be
told which property broke, with a concrete counterexample.

**Why this priority**: Proofs that are not re-checked go stale. This protects Stories 1 to 3.

**Independent Test**: A change that breaks one proven property fails the automated check and
names it. The revert passes.

**Acceptance Scenarios**:

1. **Given** a change to either type, to the `Cap` codec, or to the verification artifacts,
   **When** CI runs, **Then** these proofs run and their result appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the default
   quality gate is not slowed down or made to depend on verification toolchains.

---

### User Story 5 - Evaluation and what next (Priority: P3)

As the maintainer, I want a short written evaluation:

- what the canonical-value invariant cost and gained;
- how far the checker reached into the trigonometric conversions;
- what is trusted but not proven, and the defects found;
- a ranked list of what comes next.

**Why this priority**: `ChordAngle` is the first value type whose invariant the type enforces.
Its result decides whether to do the same for the other value types.

**Independent Test**: A maintainer who did not do the work reads the evaluation and can
answer "did enforcing the invariant pay off, and should the other value types follow?" and
"what next: `LatLng`, `Cap`, or something else?" without asking anyone.

**Acceptance Scenarios**:

1. **Given** the completed feature, **When** the evaluation is read, **Then** it contains an
   assessment of the invariant, a go or no-go for `LatLng` and `Cap`, and a ranked list of
   follow-ups.

---

### Edge Cases

- **Signed zero.** `−0.0` is not negative (`−0 < 0` is false). It stays a zero chord angle,
  equal to `ZERO`, as upstream keeps it. `predecessor(−0)` is `NEGATIVE`, and `successor(−0)`
  is the smallest positive length.
- **The boundaries.** 4 is `STRAIGHT`, and `from_length2` clamps any value above 4, +∞
  included, to it, as upstream. `INFINITY` comes only from the constant, `from_angle(+∞)` and
  `successor(STRAIGHT)`; `predecessor(ZERO)` is `NEGATIVE`.
- **NaN.** For a chord angle, NaN becomes `STRAIGHT` through the constructors, as upstream.
  The `Cap` decoder rejects it (003 D-004). For an angle, NaN is a value like any other: the
  arithmetic propagates it, `normalized` returns NaN, and the E-forms return 0 (K9).
- **Infinite angles.** ±∞ radians are angles. `normalized` of ±∞ is NaN, as upstream's
  `remainder`. `from_angle` maps −∞ to `NEGATIVE` and +∞ to `INFINITY`.
- **Exact ties in the E-forms** (K8), and multiples of π in `normalized`. An odd multiple of
  the `f64` nearest π reduces to π, as upstream, never −π.
- **The integer extremes.** `from_e7(i32::MIN)`, `from_unsigned_e7(u32::MAX)` (wraps to −1,
  K10), and E-forms of angles whose degrees × 10ᵏ exceed the `i32` range (K9).
- **Special values in the arithmetic.** Addition and subtraction of `NEGATIVE` or `INFINITY`,
  and the trigonometry of special values (K4).
- **The trigonometric functions.** `from_angle` and its forms go through `sin`; `to_angle` and
  everything after it (`radians`, `degrees`, the E-forms, the conversion to `Angle`, text
  display) through `asin` and `sqrt`; `Angle`'s `sin`, `cos`, `tan` and `sin_cos` through
  their functions. `sin`, `cos` and `sin_cos` are modelled, and `sqrt` is under-determined
  (006). `asin` and `tan` are not modelled, and the checker's `%` is wrong (research R0,
  FR-003).
- **The build.** No operation of either type uses fused multiply-add, so 006's K9 does not
  apply.

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify the chord angle `s1::ChordAngle` and the angle
  `s1::Angle` of the core library.
- **FR-002**: The verification artifacts MUST include an operation inventory for `Angle`:
  - its 20 inherent public functions;
  - its 8 hand-written trait implementations (negation, addition, subtraction, the two
    multiplications by a scalar, the two divisions, text display);
  - its 3 derived traits (equality, ordering, the default value).

  That is 31 entries. Every entry except text display is in scope. `sin`, `cos` and `sin_cos`
  get crash proofs; `tan` calls the C library, which the checker does not model, and is a
  tool-limit exclusion (FR-003). The conversions with the earth's units
  (`earth.rs`) belong to those units and are not in this inventory.
- **FR-003**: The verification artifacts MUST include an operation inventory for
  `ChordAngle`:
  - its 28 inherent public functions;
  - its 6 hand-written trait implementations (the default value, the two conversions with
    `Angle`, addition, subtraction, text display);
  - its 2 derived traits (equality, ordering).

  That is 36 entries, of which 35 are in scope; text display is excluded. Within the 35:

  - **Through `asin` and `sqrt`: 7 entries.** These are `to_angle`, `radians`, `degrees`,
    `e5`, `e6`, `e7` and the conversion to `Angle`.
  - **Through `sin`: 7 entries.** These are `from_angle`, `from_radians`, `from_degrees`, the
    three E-form constructors and the conversion from `Angle`.
  - **Measured in planning (research R0, 2026-10-16).** `asin` and `Angle`'s `tan` are C
    library calls that the checker does not model.
    - The 7 entries through `asin`, and `Angle::tan` (FR-002), are tool-limit exclusions,
      with the measurement.
    - `sin`, `sin_cos`, `sqrt`, `floor` and `round_ties_even` are modelled, so the other
      entries get crash proofs.
    - The values of the excluded entries are checked against the C++ vectors (FR-024).
  - **Through `%`.** After FR-021, `normalized` reduces with `f64`'s `%`, whose model in the
    checker is wrong: it gives 0 for `1 % 2` (research R0).
    - `normalized` gets a crash proof: no operation after the `%` can panic, whatever value
      it returns.
    - Its value properties are not provable by the checker. The C++ vectors and property
      tests check them.
  - **Properties that compare two results of `sqrt`** cannot be stated, because the checker
    does not fix `sqrt`'s result (006 research R0). They are left to the C++ vectors and the
    property tests.

**Guarantees**

- **FR-004**: For every in-scope operation, verification MUST establish that it completes
  without panic, arithmetic overflow or undefined behaviour, for every input in its
  documented domain:
  - a `ChordAngle` argument ranges over every value the type can hold, which after FR-017 is
    every canonical value, narrowed only by a documented precondition (non-special operands
    for the arithmetic and trigonometry, FR-019);
  - an `f64` argument of a constructor or of `plus_error` ranges over every `f64`, NaN and the
    infinities included;
  - an `Angle` argument ranges over every `f64` of radians, NaN and the infinities included,
    since the type has no validity rule. The E-forms are proven over the whole domain,
    including the out-of-range values K9 documents;
  - an `i32` or `u32` argument ranges over every value of its type.

  A precondition counts as documented if the library's API documentation, a debug-build check
  or the upstream reference states it. An internal consistency check that runs only in debug
  builds MUST be proven never to fire within the domain. An undocumented precondition that a
  proof needs MUST be recorded as a finding and documented.
- **FR-005**: Each type's invariant MUST be stated formally:
  - **chord angle**: its squared length is `NEGATIVE`'s −1, a value in [0, 4] (both zeros
    included), or +∞. Verification MUST establish that every in-scope operation returning a
    chord angle returns a canonical one for every input in its domain, the constructors and
    `plus_error` for every `f64`;
  - **angle**: none; every `f64` is an angle.
- **FR-006**: The verification artifacts MUST include a property catalogue for each type,
  covering at least these areas:
  - **`ChordAngle`**:
    - **Construction**: `from_length2` maps a negative input (−∞ included) to `NEGATIVE`, a
      value above 4 (+∞ included, as upstream's `std::min(4.0, x)`) and NaN to `STRAIGHT`, and
      keeps every value in [0, 4] bit for bit. `from_angle` maps a negative angle to `NEGATIVE` and +∞ to
      `INFINITY`, and is upstream's formula elsewhere. The constants are upstream's.
    - **Classification**: `is_zero`, `is_negative`, `is_infinity`, `is_special` and
      `is_valid` as upstream defines them on canonical values. `is_valid` holds for every
      value the type can hold.
    - **Order**: equality and the ordering are a total order on canonical values, instances
      of 006's generic order laws.
    - **Neighbours**: `successor` and `predecessor` return the next canonical value above and
      below, with upstream's results at the sentinels and the boundaries. Each undoes the
      other except at the ends.
    - **Error bounds**: `plus_error` clamps to [0, 4] and leaves special values unchanged.
      `max_point_error` and `max_angle_error` are upstream's formulas bit for bit.
    - **Arithmetic and trigonometry**: addition and subtraction return a canonical value in
      [0, 4] for non-special operands. Adding or subtracting zero returns the other operand.
      Subtracting a larger operand returns zero. `sin2` and `cos` are upstream's formulas bit
      for bit.
    - **The generic laws**: the equality laws, instances of 004's generic laws.
  - **`Angle`**:
    - **Representation**: `from_radians` and `radians` keep the bits; the default is zero;
      `abs` and negation are the IEEE operations; negation undoes itself.
    - **Conversions**: `from_degrees`, `degrees` and the E-form constructors are upstream's
      formulas bit for bit. The exact identities upstream documents (`s1angle.h:46–72`) hold
      for every integer they name.
    - **E-forms**: `e5`, `e6` and `e7` are the rounding FR-022 settles. For every in-range
      integer n, `from_e7(n).e7()` returns n, and likewise for E6 and E5, or the catalogue
      states the exceptions. Out of range they saturate as documented (FR-023).
    - **Reduction**: `normalized` returns the exact IEEE remainder of the angle by the
      `f64` nearest 2π, mapped to π where upstream maps it, so it lies in (−π, π] for every
      finite angle. It is idempotent.
    - **Arithmetic**: addition, subtraction, the scalar products and the two divisions are
      the IEEE-754 operations on radians, bit for bit.
    - **The generic laws**: the equality laws, and the order laws on angles without NaN.

  Each entry MUST record:
  - a plain-language statement;
  - its kind;
  - the upstream source;
  - the domain;
  - a status: **proven** (the whole domain is covered), **proven within stated bounds**, or
    **not proven (reason)**.
- **FR-007**: Properties MUST be stated against reference descriptions written from upstream
  S2 semantics that do not restate the implementation's code:
  - for the chord angle, the ordered set {−1} ∪ [0, 4] ∪ {+∞} of `f64` values, with
    neighbours as the adjacent values of that set;
  - for the angle, an `f64` of radians, with the reduction stated as the IEEE remainder.

  Where a property is "bit for bit upstream's formula", the restatement of the formula is an
  explicit part of the description, kept apart from the meaning and checked against the C++
  vectors, as 004 and 006 did. Applies to the degree factor, the E-form scale, the chord
  formulas and the error bounds.
- **FR-008**: Verification MUST be exhaustive and automated over every input in the domain,
  with exact bit-level IEEE-754 semantics, and MUST produce a concrete counterexample on
  failure. A property whose whole domain is one `i32` or `u32` MAY instead be proven by
  enumerating every input against the library's code compiled in release mode, recorded as
  "proven by enumeration". The checker's runs of the E-form round trips time out (research
  R0), and the enumeration takes about 70 s. Floating-point properties hold exactly, except where upstream promises only an
  approximate result. There the property states its tolerance in ulps and the reason.
- **FR-009**: Every proven property MUST be shown to be non-vacuous: at least one injected
  defect MUST make it fail, and the injected defects and outcomes MUST be recorded.
  Crash-freedom proofs MUST show their domains non-empty and their outcomes reachable.

**Readability**

- **FR-010**: Each catalogue entry MUST have a plain-language statement and one kind from the
  kinds the project uses.
  - It MUST appear in the index of properties by kind.
  - Each proof's documentation MUST start with the entry's identifier, kind and plain
    statement.
  - The consistency check between the catalogues, the index and the proofs MUST cover the new
    entries.

**Non-intrusiveness**

- **FR-011**: The feature MUST NOT change the public API of either type, and MUST NOT change
  observable behaviour, except for FR-017 to FR-023. It MUST NOT add runtime dependencies or
  slow the library down. Verification-only code MUST be excluded from normal builds.
- **FR-012**: A defect found by verification MUST be fixed as a separate, recorded change,
  with parity checked against upstream and a regression test that fails without the fix. The
  survey findings K1–K10 are the exception: they are handled inside this feature (FR-017 to
  FR-023).

**Operability**

- **FR-013**: The proofs MUST run from the project's documented verification command, which
  reports pass or fail per operation and per property, with a documented way to run only this
  feature's proofs.
- **FR-014**: The proofs MUST run automatically in CI, through the same command, for changes
  to either type, to the `Cap` codec, or to the verification artifacts. They MUST stay out of
  the default quality gate. Proofs over the per-change budget run in the weekly tier, which
  has a CI leg per value-type module.
- **FR-015**: The trusted base statement MUST be extended, including:
  - the checker's model of IEEE-754 arithmetic, of rounding to an integer and of the
    remainder;
  - its models of `sin`, `asin`, `sqrt`, `tan` and `sin_cos`, as FR-003's measurement finds
    them;
  - every restatement of upstream's arithmetic, and any split of a domain.

**Evaluation**

- **FR-016**: The feature MUST produce a written evaluation. It records the effort, the run
  time, what was reachable, the limitations and the defects found. It MUST also:
  - assess the canonical-value invariant: which properties it made provable over the whole
    type, what it cost, and whether the other value types should enforce theirs (type-level
    invariants);
  - say how far the checker reached into the trigonometric conversions;
  - give a go or no-go for `LatLng` and `Cap`;
  - give a ranked list of follow-ups.

**Known gaps: `ChordAngle`**

- **FR-017** (K1, K2; the maintainer's option B): Every way to make a chord angle MUST yield a
  canonical value. That covers `from_length2`, `from_angle` and its forms, `plus_error`,
  `successor`, `predecessor`, the arithmetic, deserialisation (FR-020) and the `Cap` decoder.
  - Every negative input, −∞ included, MUST become `NEGATIVE`.
  - `is_infinity` MUST hold for +∞ only, as upstream.
  - `is_valid` keeps upstream's definition, which every canonical value satisfies.
  - The rustdoc of `ChordAngle` and of the `Cap` decoder MUST document the deviation and its
    reason: a cap encoded with a negative radius other than −1 decodes as an empty cap with
    radius `NEGATIVE`, so its bytes do not round-trip, where C++ keeps them.
  - 003's records MUST be updated. The `Cap` decoder's agreement with C++ (P-FX-08) gains
    this deviation; the round trip of valid caps (P-FX-03) is unchanged.
  - This is its own identifiable change, with regression tests that fail without it.
- **FR-018** (K3): `from_length2` of NaN, and `plus_error` with a NaN length or a NaN error,
  MUST return `STRAIGHT`, as upstream. The `Cap` decoder MUST still reject a NaN radius (003
  D-004), by checking the raw value before it builds the chord angle. This restores parity, as
  its own identifiable change, with a regression test that fails without it.
- **FR-019** (K4): The preconditions of addition, subtraction, `sin`, `sin2`, `cos` and `tan`
  (neither operand special) MUST be documented, with the panic in debug builds and what
  release builds return.
  - `sin2` and `cos` MUST gain upstream's debug-build checks, and with them `sin` and `tan`.
  - This changes behaviour in debug builds only.
- **FR-020** (K6): Deserialising a chord angle MUST reject a value that is not canonical. That
  includes NaN, values above 4 other than +∞, and negatives other than −1.
  - A canonical value MUST deserialise unchanged.
  - The serialised form MUST NOT change.
  - This follows the decisions for `CellUnion`, `Cell`, `s2::Rect` and `r2::Rect`, as its own
    identifiable change, with a regression test that fails without it.

**Known gaps: `Angle`**

- **FR-021** (K7): `normalized` MUST return the exact IEEE-754 remainder of the angle by the
  `f64` nearest 2π, mapped to π where it is −π, as upstream's `Normalize`, for every input.
  - Its result is in (−π, π] for every finite angle and NaN for ±∞ and NaN.
  - Where the result was already exact it MUST NOT change.
  - Its rustdoc MUST state the domain.
  - This fixes a library defect, as its own identifiable change, with regression tests from
    the probe that fail without it. It adds no dependency.
- **FR-022** (K8): `e5`, `e6` and `e7` MUST round exact ties to even, as C++ does on x86-64,
  the platform of the reference build. Everywhere else the result MUST equal upstream's.
  - This changes the library's results at exact ties only: an angle whose degrees × 10⁶ is
    exactly 2.5 gets `e6` = 2, not 3. `ChordAngle`'s E-forms follow through `to_angle`.
  - The rustdoc MUST state the rounding, and that C++'s generic code on other processors rounds
    ties away from zero (U-014).
  - This is its own identifiable change, with regression tests from the probe that fail
    without it.
  - *Clarified 2026-10-16 by the maintainer: option A, ties to even.*
- **FR-023** (K5, K9, K10): The E-forms' domain MUST be documented: upstream's (−180°, 180°],
  and what the library returns outside it.
  - Outside the domain they saturate: `i32::MAX` above the range and for +∞, `i32::MIN` below
    it and for −∞, and 0 for NaN. This is a documented deviation from C++'s −2147483648.
  - The same applies to `ChordAngle`'s E-forms, where `INFINITY`'s are `i32::MAX`.
  - The wrapping of `from_unsigned_e6` and `from_unsigned_e7` MUST be documented, as
    upstream's.
  - The module's lint justification MUST describe what the code does.
  - Apart from FR-022, this changes no behaviour.

**Reference vectors, upstream findings and records**

- **FR-024**: The feature MUST add committed C++ reference vectors, with differential tests,
  for every operation of both types that upstream has, the trigonometric ones included.
  - The vectors cover random values and the special cases:
    - the sentinels and the boundaries;
    - signed zeros, NaN and the infinities;
    - negatives other than −1;
    - exact ties in the E-forms;
    - multiples of π, and large angles for `normalized`;
    - the integer extremes.
  - Rows where the library deviates (FR-017, FR-018 for the decoder, FR-023) are marked as
    deviations, as the earlier vectors mark theirs. The ties of FR-022 are not deviations.
  - CI MUST NOT need a C++ toolchain to check them.
- **FR-025**: Undefined or wrong behaviour found in the upstream reference MUST be noted as a
  follow-up task before the affected proof is marked complete. The note gives the upstream
  location, the inputs and the behaviour, and is listed in the evaluation and in
  `UPSTREAM.md`.
  - K5 and K9 together are the first, as U-014: `S1ChordAngle::e5`, `e6` and `e7` of the
    valid value `Infinity()` break `S1Angle`'s precondition. They fail a check in debug builds
    and return −2147483648 in release builds; out-of-range `S1Angle` E-forms do the same.
  - Upstream's tie rounding depending on the processor (K8) is recorded with it.
  - Reporting them upstream needs the maintainer's approval.
- **FR-026**: As the constitution requires (v1.2.0, "Verification records"), the change MUST
  update the records it affects in the same change:
  - the value-type suite's inventory and catalogue;
  - the index of properties by kind and its rendered copy;
  - `VERIFIED_TYPES.md`, where `ChordAngle` moves from "reached only through other proofs"
    to a section of its own, and `Angle` is added;
  - 003's decoding records for the `Cap` deviation (FR-017).

  006's deferral note (its FR-025) MUST be marked as resolved by this feature.

### Key Entities

- **`ChordAngle`**: a verification target. A squared chord length in [0, 4], or one of two
  sentinels below and above every length; after FR-017 the type holds nothing else.
- **`Angle`**: a verification target. An `f64` of radians with no validity rule.
- **Reference descriptions**: the ordered set of canonical chord-angle values and its
  neighbours; radians with the IEEE remainder; the restatements of upstream's formulas kept
  apart (FR-007).
- **Generic order laws**: 006's total-order laws, instantiated for both types.
- **The canonical-value invariant**: the first value-type invariant the type itself enforces
  (FR-017).
- **Operation inventories, property catalogues, trusted base and evaluation**: as in 001–006.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope operation is proven free of panics, overflow and undefined
  behaviour over its whole documented domain, with no size bound.
- **SC-002**:
  - All 30 in-scope `Angle` entries have a crash proof, or are recorded as tool limits with
    measurements, which is allowed for `tan` only.
  - All 35 in-scope `ChordAngle` entries have one, or are recorded as tool limits with
    measurements, which is allowed for the 7 entries through `asin` only.
  - Every exclusion has a written reason.
- **SC-003**: Each catalogue covers every area in FR-006. At least 90% of the entries whose
  operations are all in scope and expressible are proven, over the whole domain or within
  stated bounds (by the checker or by enumeration). Entries that compare two results of
  `sqrt`, or state the value of `normalized`, whose `%` the checker mis-models, are not counted
  (FR-003).
- **SC-004**: 100% of proven properties have recorded non-vacuity evidence, and 100% of
  crash-freedom proofs have satisfied reachability checks.
- **SC-005**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine, a hosted Linux runner with 4 virtual CPUs and 16 GB.
- **SC-006**: Apart from FR-017 to FR-023, the public API and behaviour of both types are
  unchanged, and the quality gate passes before and after.
- **SC-007**: After the feature, no input to any constructor, deserialiser or decoder yields a
  chord angle outside the canonical set (proven for the constructors, tested for the
  deserialiser and the decoder), and each regression test of FR-017 to FR-022 fails before its
  fix and passes after it.
- **SC-008**: `normalized` equals upstream's result for every row of the C++ vectors, the
  probe's large angles included.
- **SC-009**: Every operation the proofs show can panic documents when it panics.
- **SC-010**: The differential tests against the new C++ reference vectors pass for every
  vector, and the deviation rows are exactly those FR-024 names.
- **SC-011**: Every catalogue entry appears exactly once in the index of properties by kind,
  and the consistency check passes.
- **SC-012**: Every undefined or wrong upstream behaviour met has a follow-up task and an
  entry in the evaluation.
- **SC-013**: A maintainer who did not take part can answer from the evaluation alone: "did
  enforcing the invariant pay off, and should the other value types follow?" and "what
  next?".

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Users benefit through a distance type
  whose comparisons and neighbours hold for every value, and through exact angle reduction.
- **The second type** is `Angle`. It was recommended alongside the validity question on
  2026-10-16, over `Cap` and `LatLng`:
  - `ChordAngle` converts to and from it;
  - `LatLng` is built from two of them;
  - it is the smallest candidate.

  `LatLng` and `Cap` are left for the evaluation to rank.
- **Builds on 001–006.** The feature reuses:
  - the verification setup, the memory-guarded runner, the mutant campaign, the per-spec
    mutant leg, the weekly tier and its CI legs;
  - 004's generic equality laws;
  - 006's generic order laws and its measurements of the modelled functions.
- **Lessons carried over**:
  - the checker mis-models ordered comparisons of booleans, so descriptions compare booleans
    only for equality;
  - it reports `x / ±∞` as NaN, so a domain is narrowed only after the counterexample is
    replayed;
  - two `sqrt` calls on one input may differ in the checker, so no harness compares two
    separately computed `sqrt` results bit for bit;
  - asserting on products of symbolic values is slow, so the bit-for-bit statements compare
    the library's expression with an identical restatement;
  - the pilot's E5/E6/E7 round trips did not finish in 25 minutes, so they are expected to
    need the weekly tier, bounds or a split by range.
- **Assurance level**: exhaustive model checking of the library's own code, as in 001–006.
- **Reference semantics**: upstream C++ at v0.14.0-40, Release build on x86-64, with Go and
  Java secondary. Where the library deviates and documents it, the documented behaviour is
  verified.
- **Parity first**: where only the library is wrong (K3, K7), or differs from the reference
  build (K8), it is fixed to upstream's result. Where upstream's result is wrong or undefined
  (K5, K9), the library returns a sensible value as a documented deviation, as for U-007 to
  U-012. Option B is the one
  deliberate deviation for a valid input, decided by the maintainer.
- **Deserialisation (FR-020) rejects** a non-canonical chord angle rather than mapping it,
  following the earlier deserialisation fixes: reject what is not valid, keep the format.
  Only the binary `Cap` decoder maps a negative radius to `NEGATIVE`, as option B describes,
  because C++ writes such bytes.
- **An angle has no validity rule**, so its deserialisation is unchanged, as upstream's
  decoding is.
- **Out of scope**:
  - `LatLng`, `Cap` beyond its decoder's radius, and the earth-unit conversions;
  - the accuracy of `to_angle` and `from_angle` beyond the formulas (certified error bounds);
  - text display;
  - private fields for `Angle`, which has no invariant to keep;
  - validating input in the language bindings, which inherit the documentation and the
    fixes.
- **Survey findings**: K1–K4 and K6 were confirmed on 2026-10-13, and K1–K3 re-checked on
  2026-10-16. K5 and K7–K10 were confirmed on 2026-10-16 against upstream C++ (`probe/`). The
  library files of both types are unchanged since the initial commit.
- **Existing tests stay**: the proofs complement the existing unit, property, differential
  and generic trait tests. No test is removed. Tests that pin the old behaviour of a value
  that FR-017, FR-018, FR-021 or FR-022 changes are updated, and the update is listed with the
  change.
