# Feature Specification: Formal Verification of `LatLng`, and `s2::Rect` as a `Region`

**Feature Branch**: not created yet (`009-bounded-types/2` is checked out; no branch hook is
configured)

**Created**: 2026-10-27

**Status**: Draft; clarified 2026-10-27 by the maintainer: `s2::Rect` as a `Region` is in scope
(FR-001a); U-016 is fixed by widening the pole cap, a documented deviation (FR-019); the two
missing upstream operations are follow-ups (FR-020).

**Input**: User description: "Continue with formally verifying the next type on the list"

## Context

### Where the earlier features left off

- **001–003** proved `CellId`, `CellUnion` within size bounds, and the decoding layer.
- **004–008** proved nine value types: `Cell`, `s1::Interval`, `r1::Interval`, `s2::Rect`,
  `r2::Rect`, `Point`, `ChordAngle`, `Angle` and `Cap`. 008 added function contracts for
  `asin`, `atan2` and `cos`, and lemma stubs for laws that recompute a value.
- **The coverage review of 2026-10-26** proved most of the floating-point entries 004–007 had
  excluded, under contracts for `tan`, `atan`, `sin` and `sin_cos` (`verify/COVERAGE.md`).
- **009** made `Level`, `Face`, `LevelMod` and `CellVertex` bounded types. Nothing is pushed.
- **The list**: 008's evaluation ranks, first, "`Rect` next for the `Region` laws, reusing
  `laws::region`, the abstract cell and the stubs; then `LatLng` as the next value type, with
  the `atan2` contract" (`verify/values/EVALUATION.md`, "Recommendations (spec 008)").
  `VERIFIED_TYPES.md` (step 3) records `LatLng` as the next value type.

### Why `LatLng`, together with `s2::Rect` as a `Region`

`LatLng` is the next *type* on the list: `s2::Rect` is already proven as a value type (005,
and the coverage review), and what the list's first item adds for it is four law instances.
The two belong together:

1. **`Rect`'s point test is `LatLng`'s conversion.** `Rect::contains_point(p)` is
   `contains_lat_lng(LatLng::from_point(p))`, and `Rect::cap_bound` builds its cap from
   `LatLng::to_point` of the centre and the vertices. Every `Region` law for `Rect` goes
   through `LatLng`.
2. **`LatLng` has no proof of its own.** Proven `Rect` and `Cell` entries take it as an
   argument (the domain `ValidLatLng`), but its own 19 functions have no inventory, no
   catalogue and no C++ vectors.
3. **The contracts it needs exist.** `from_point` goes through `atan2`, `to_point` through
   `sin_cos`, `get_distance` through `sin`, `cos`, `sqrt` and `asin`; each has a tested
   contract since 008 and the coverage review.

### Survey findings (probed 2026-10-27)

The probe is in `probe/`: the same 132,043 cases on upstream C++ (v0.14.0-40, Release, x86-64)
and on the library, compared line by line (`probe/README.md`). It agrees bit for bit on
`to_point` (20,004 cases), `from_point` (20,009, signed zeros and NaN included), `get_distance`
of valid pairs (20,000), and `Rect`'s `Contains(cell)` and `MayIntersect(cell)` (20,000). It
finds:

- **K1, `normalized` is not upstream's reduction, and can return an invalid `LatLng`.**
  - Upstream reduces the longitude with `remainder(lng, 2π)`, the exact IEEE remainder. The
    library computes `lng − round(lng / 2π) · 2π`, which rounds twice.
  - 8,378 of 12,014 longitudes differ. Up to |lng| = 4 only ±π differ; at |lng| ≤ 100, 372 of
    2,000; beyond 10⁶, nearly all.
  - **293 results lie outside [−π, π]**: `LatLng::from_radians(0.3, 1e16).normalized()` has
    longitude −4.0, so it is not valid, though `normalized` documents a result in [−π, π].
  - ±π come back as ∓π (upstream's remainder keeps π and −π, ties to even), and −0 and −2π
    lose the sign of zero, which `to_string_in_degrees` shows: (−0, −0) prints
    `-0.000000,0.000000`, where upstream prints `-0.000000,-0.000000`.
  - 007 fixed the same rounding in `Angle::normalized` (K7 there), but `Angle::normalized`
    maps −π to π, as upstream's `S1Angle::Normalize` does, and `S2LatLng::Normalized` does not.
    This is a library defect.
- **K2, `from_point` is always valid** (a law, not a defect): 200,000 random finite vectors
  whose coordinates span 600 decades all give a valid `LatLng`, as upstream's constructor
  comment says ("already normalized"). Upstream's `+ 0.0` makes the longitude of (−1, −0, 0)
  π, not −π; the library does the same.
- **K3, `get_distance` clamps after the square root.** Upstream computes `sqrt(min(1, x))`, the
  library `min(sqrt(x), 1)`. They differ only where rounding makes the haversine term `x`
  negative, which needs a latitude outside [−π/2, π/2]: for one point written twice,
  (3.124793493547795, 0) and (0.0167991600419979, π), upstream returns NaN and the library π,
  the largest distance there is. Outside the documented domain ("both must be normalized"),
  but a parity defect.
- **K4 (U-016), `s2::Rect::cap_bound` misses points of the rectangle, upstream too.**
  - For 20,000 random rectangles and 7 points on and inside each one's boundary, 114 of 129,595
    contained points lie outside the bounding cap, in C++ and in the library alike (the answers
    agree on every case).
  - Every miss is a rectangle whose bound is the pole cap, and a point on its far latitude
    bound, outside by up to 18.5ε of the cap's length². The pole cap's radius is widened by a
    relative 2ε, too little for the error of converting the point and measuring its chord.
  - It breaks the `Region` invariant "the bounding cap contains every contained point"
    (`CORE_TRAITS.md`), which 008 stated generically. Its quickcheck twin did not find it,
    because it does not draw points on a rectangle's boundary.
- **K5, two upstream operations are missing** (not defects of existing code):
  - `S2LatLng::Encode` and `Init`, a 16-byte codec (two doubles, decoded without validation).
    Every other value type with an upstream codec has one in the library.
  - `S2LatLngRect::Intersects(const S2Cell&)`, upstream's exact cell test. The library's
    `Region::intersects_cell` is upstream's `MayIntersect` (the bounding rectangles), which is
    correct; the exact test differs from it on 158 of 20,000 random pairs. The library has its
    edge helpers privately, for `boundary_intersects`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for `LatLng` (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope operation of
`LatLng` completes without panicking, overflowing or hitting undefined behaviour, for every
input in its documented domain, NaN and the infinities included where the operation accepts
them.

**Why this priority**: The constitution requires crash freedom (Principle II). `LatLng` takes
any pair of numbers (`new` and the constructors do not check), so its domain is every pair of
`f64`, and every operation that does not document a precondition must be safe on all of them.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope `LatLng` operation as proven free of panics, overflow and undefined
behaviour over its stated domain, with no unproven obligations.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification command,
   **Then** it reports each in-scope operation by name as proven crash-free, states its
   domain, and exits successfully.
2. **Given** an operation with a documented precondition (`to_point`: finite coordinates;
   `get_distance`: both normalised), **When** verification runs, **Then** the proof assumes
   exactly that precondition, and a separate proof shows what the operation does outside it.
3. **Given** an in-scope operation deliberately changed so that it can panic inside its
   domain, **When** verification runs, **Then** it fails and names the operation and an input.

---

### User Story 2 - Proven meaning against an independent description (Priority: P1)

As the maintainer, I want a catalogue of functional properties of `LatLng`, proven against a
description written from upstream's semantics: a `LatLng` is a latitude and a longitude; it is
valid when both lie in their principal ranges; normalising clamps the latitude and moves the
longitude by a whole number of turns into [−π, π]; the conversions to and from a point are
upstream's formulas.

**Why this priority**: Every `Rect`, `Cell` and `Cap` proof that takes a `LatLng` assumes it is
valid and means what upstream's does. K1 shows that `normalized`, the operation users call to
make one valid, can fail to.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven, proven within stated bounds, proven by composition, proven under a stated contract,
or not proven with a written reason. Each proven property fails when a matching defect is
injected.

**Acceptance Scenarios**:

1. **Given** any finite `LatLng`, **When** it is normalised, **Then** the result is valid, its
   latitude is the clamped latitude, and its longitude differs from the input's by an exact
   whole number of turns, upstream's choice at the ties (after FR-017).
2. **Given** any finite point, **When** it is converted to a `LatLng`, **Then** the result is
   valid (K2).
3. **Given** a valid `LatLng`, **When** it is normalised, **Then** it is returned unchanged.
4. **Given** a property about a value computed through a function the checker does not model
   exactly (`atan2`, `sin`, `cos`, `asin`), **When** a reviewer reads it, **Then** it states
   what holds for every value the function's contract allows, and leaves the value itself to
   the C++ vectors.

---

### User Story 3 - The `Region` laws, instantiated for `s2::Rect` (Priority: P2)

As the maintainer, I want the generic `Region` laws proven for `s2::Rect`, so that it counts as
a verified `Region` implementor, as `Cell` and `Cap` do.

**Why this priority**: It is the list's first item, and the probe found the one `Region` law
that fails for `Rect`, upstream too (K4). The coverer relies on the bounding cap.

**Independent Test**: Run the verification command. Each law instance for `Rect` is reported
by name as proven, or as not proven with a written reason. Injecting a defect into
`Rect`'s `contains_cell`, `intersects_cell`, `contains_point` or `cap_bound` fails the
matching instance.

**Acceptance Scenarios**:

1. **Given** the `Region` laws, **When** verification runs, **Then** for every valid rectangle
   and every valid cell, containing the cell implies intersecting it, and a cell the rectangle
   does not intersect has no vertex the rectangle contains.
2. **Given** the law "the bounding cap contains every contained point", **When** verification
   runs on the widened bound (FR-019), **Then** it is proven for `Rect`, or, where it needs
   more of `atan2` than its contract gives, recorded as not proven with the reason and checked
   by the C++-vector rows and the quickcheck twin's boundary points.
3. **Given** the law "the covering bound covers every contained cell", **When** verification
   runs, **Then** it is attempted, and recorded as a tool limit if `Cap::cell_union_bound`'s
   timeout (008) carries over.

---

### User Story 4 - Parity restored where the survey found defects (Priority: P2)

As a library user, I want `LatLng::normalized`, `get_distance` and `to_string_in_degrees` to
give upstream's answers (K1, K3), and `Rect::cap_bound` to contain every point the rectangle
contains (K4), each fixed as its own change with a regression test.

**Why this priority**: K1 makes `normalized` return invalid `LatLng`s, which every operation
that requires a valid one then mishandles. These are this feature's findings, fixed as the
earlier features fixed theirs.

**Independent Test**: Each probe case becomes a regression test that fails before its fix and
passes after. The new C++ reference vectors (FR-021) match on every row, except the
documented deviations.

**Acceptance Scenarios**:

1. **Given** the longitude 10¹⁶, **When** `LatLng::from_radians(0.3, 1e16).normalized()` runs,
   **Then** its longitude is upstream's, 2.6372…, and the result is valid.
2. **Given** the longitudes π, −π, 5π, −0 and −2π, **When** they are normalised, **Then** each
   result is upstream's bit for bit, sign of zero included.
3. **Given** the point written twice with an out-of-range latitude, **When** `get_distance`
   runs, **Then** it returns upstream's NaN.
4. **Given** the probe's 114 boundary points, **When** `Rect::cap_bound` is computed, **Then**
   every one lies in the bound (FR-019), where upstream's bound misses them.

---

### User Story 5 - Regression protection and evaluation (Priority: P3)

As a contributor changing `LatLng` or `Rect`'s `Region` impl, I want the proofs to run
automatically in CI and name the property that broke. As the maintainer, I want a short
evaluation of what this feature reached and a ranked list of what comes next.

**Why this priority**: Proofs that are not re-checked go stale. The evaluation picks the next
step among the remaining `Region` implementors (`Cell`'s float methods, `PointRegion`,
`CellUnion`'s bounds) and the next type.

**Independent Test**: A change that breaks one proven property fails the automated check and
names it; the revert passes. A maintainer who did not do the work can answer from the
evaluation alone: "how much of `LatLng` is proven, which `Region` laws hold for `Rect`, and
what next?"

**Acceptance Scenarios**:

1. **Given** a change to `LatLng`, `Rect`'s `Region` impl or the verification artifacts,
   **When** CI runs, **Then** these proofs run and their result appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the default
   quality gate is not slowed down or made to depend on verification toolchains.
3. **Given** the completed feature, **When** the evaluation is read, **Then** it says which
   entries are proven, which are limited and why, and ranks the follow-ups.

---

### Edge Cases

- **Any pair of numbers.** `new`, `from_radians` and `from_degrees` accept NaN, the infinities
  and out-of-range values; `is_valid` is a predicate, not an invariant. `invalid()` is
  (π, 2π), upstream's. Operations without a documented precondition take every pair.
- **The ties of the remainder.** A longitude whose quotient by 2π is exactly half-way (±π, ±3π
  as `f64`s) keeps upstream's tie rule: π stays π and −π stays −π, unlike `Angle::normalized`,
  which maps −π to π as upstream's `S1Angle::Normalize` does.
- **Signed zeros.** The longitude −0 normalises to −0 upstream. `latitude` and `longitude` add
  `+ 0.0` to their arguments, so a −0 coordinate counts as +0 and the longitude of (−1, −0, 0)
  is π.
- **Huge and non-finite longitudes.** 10³⁰⁰ reduces exactly (to −0.7234… upstream; the library
  gives 0). NaN or ±∞ in either coordinate normalises to `invalid()`.
- **Points that are not unit length.** `from_point` takes any direction vector, the zero vector
  (latitude 0, longitude 0), subnormal coordinates and infinite ones included.
- **Asymmetric distance.** `get_distance(a, b)` and `get_distance(b, a)` differ bit for bit on
  about 8% of random valid pairs, in upstream's formula as in the library: it multiplies the
  two cosines in argument order. Symmetry holds only within a tolerance.
- **Unnormalised inputs to `get_distance`.** Latitudes outside [−π/2, π/2] make the haversine
  term negative after rounding (K3).
- **Rectangles at the poles and wider than π.** `Rect::cap_bound` switches between the pole cap
  and a cap around the centre; K4's misses are all pole caps. Full, empty, single-point and
  inverted rectangles, and longitude intervals of exactly π (the switch).
- **Unmodelled functions.** `atan2` (in `latitude`, `longitude`, `from_point`, and so in
  `Rect::contains_point`), `sin` and `cos` (in `to_point` and `get_distance`) and `asin` (in
  `get_distance`) run under the contracts of 008 and the coverage review. A property that
  needs two calls of one of them on the same input to agree, or needs its value, stays with the
  C++ vectors.
- **The remainder through `%`.** The checker's `f64` remainder is wrong (KANI_ISSUES.md), so
  FR-017's reduction is proven as 007 proved `Angle::normalized`'s: by C++ vectors and an
  exact arbitrary-precision property test, with the checker proving only what follows from it.

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify `s2::LatLng` and its trait implementations, and MUST
  instantiate the project's generic equality laws for it.
- **FR-001a**: The feature MUST instantiate the generic `Region` laws for `s2::Rect` (User
  Story 3). *Clarified 2026-10-27: in scope, since every one of them goes through `LatLng`.*
- **FR-002**: The verification artifacts MUST include an operation inventory for `LatLng`:
  - its 19 inherent public functions;
  - its trait implementations: addition, subtraction, the two scalings, the default, text
    display, equality, and the two conversions to and from `Point`;
  - its derived deserialisation;
  - its four `geo-types` conversions (an optional integration).

  That is 33 entries. Text display is excluded, as for every other type (the formatter is
  outside the checker's budget); `to_string_in_degrees`'s normalisation is in scope through
  `normalized`, its formatting is not.
- **FR-003**: Entries that go through `atan2`, `sin`, `cos`, `sin_cos`, `sqrt` or `asin` MUST be
  proven under the existing tested function contracts (status **proven under a stated
  contract**), as 008 and the coverage review did. A new contract, or a stronger one than a
  range (for example monotonicity of `atan2`, which K4's law may need), MUST be tested against
  the real function as the existing ones are (every special value and at least 10⁶ random
  inputs) and recorded in the trusted base.

**Guarantees**

- **FR-004**: For every in-scope operation, verification MUST establish that it completes
  without panic, arithmetic overflow or undefined behaviour, for every input in its
  documented domain:
  - a `LatLng` argument ranges over every pair of `f64`, NaN and the infinities included,
    narrowed only by a documented precondition;
  - a `Point` argument ranges over every triple of `f64`;
  - an `f64` scale factor over every `f64`.

  A precondition counts as documented if the API documentation, a debug-build check or
  upstream states it. An undocumented precondition that a proof needs MUST be recorded as a
  finding and documented.
- **FR-005**: The validity predicate MUST be stated formally (latitude in [−π/2, π/2],
  longitude in [−π, π], neither NaN). Verification MUST establish which operations return a
  valid `LatLng` for which inputs: at least `from_point` for every finite point (K2),
  `normalized` for every finite input (after FR-017), the default, and the E5/E6/E7
  constructors within their degree ranges; and that `invalid()` is not valid.
- **FR-006**: The verification artifacts MUST include a property catalogue for `LatLng`,
  covering at least these areas:
  - **Construction**: each constructor is the pair of 007's `Angle` constructors, by
    composition with 007's proofs (the E-forms proven by enumeration there); `invalid` and
    the default are upstream's constants.
  - **From a point**: `latitude` and `longitude` are upstream's formulas, `+ 0.0` included;
    `from_point` is valid for every finite point; NaN in, NaN out.
  - **To a point**: `to_point` is upstream's formula bit for bit over the values its
    contracts allow; each coordinate lies in [−1, 1]; finite for finite input.
  - **Normalisation** (after FR-017): the latitude is the clamp; the longitude differs from
    the input by an exact whole number of turns and lies in [−π, π], with upstream's ties and
    signed zeros; the result is valid; `invalid()` for non-finite input; the identity on valid
    input.
  - **Validity**: `is_valid` is upstream's predicate.
  - **Distance** (after FR-018): upstream's formula bit for bit over the values its contracts
    allow; in [0, π] for valid inputs; zero for equal valid inputs; symmetric within a stated
    tolerance (Edge Cases).
  - **Approximate equality**: upstream's `ApproxEquals`, both coordinates within the tolerance
    inclusively; reflexive and symmetric on non-NaN input.
  - **Arithmetic**: addition, subtraction and the two scalings are 007's `Angle` operations,
    coordinate by coordinate.
  - **Equality**: coordinate by coordinate; the equality laws on `LatLng`s without NaN.
  - **The `geo-types` conversions**: the coordinates in degrees, x the longitude and y the
    latitude, both ways.

  Each entry MUST record a plain-language statement, its kind, the upstream source, the domain,
  and a status: **proven**, **proven within stated bounds**, **proven by composition** (from
  named proven entries, with the argument written down), **proven under a stated contract**,
  or **not proven (reason)**.
- **FR-007**: Properties MUST be stated against a reference description written from
  upstream's semantics that does not restate the implementation's code: a `LatLng` is a pair
  of angles in radians; validity is membership in the principal ranges; normalisation is the
  clamp and the exact reduction by whole turns with upstream's tie rule. Where a property is
  "bit for bit upstream's formula", the restatement is an explicit part of the description,
  kept apart from the meaning and checked against the C++ vectors.
- **FR-008**: Verification MUST be exhaustive and automated over every input in the domain,
  with exact bit-level IEEE-754 semantics, and MUST produce a concrete counterexample on
  failure. Floating-point properties hold exactly, except where upstream promises only an
  approximate result; there the property states its tolerance and the reason.
- **FR-009**: Every proven property MUST be shown to be non-vacuous: at least one injected
  defect MUST make it fail, and the injected defects and outcomes MUST be recorded.
  Crash-freedom proofs MUST show their domains non-empty and their outcomes reachable.

**`s2::Rect` as a `Region`** (FR-001a)

- **FR-010**: The four generic `Region` laws MUST be instantiated for `s2::Rect` over every
  valid rectangle and every valid cell: containment implies intersection; a cell the
  rectangle does not intersect has no vertex the rectangle contains; the bounding cap contains
  every contained point (on the widened bound of FR-019); the covering bound covers every
  contained cell (attempted; a tool limit if `Cap::cell_union_bound`'s timeout carries over).
  A law instance that needs the value of `atan2`, beyond its contract, is recorded as not
  proven, with the reason, as 008's P-CPREG-04 was.
- **FR-011**: The quickcheck twins of the `Region` laws MUST draw rectangles near the poles and
  points on a rectangle's boundary (corners and both latitude bounds), so that K4's inputs are
  reachable, and the probe's cases MUST be added as fixed examples.
- **FR-012**: A property of another type that follows from a proven `LatLng` property MUST be
  recorded as proven by composition where the argument is short and checkable (for example,
  that `Rect::contains_point` and `Cell::from_lat_lng` receive a valid `LatLng` from
  `from_point` and `normalized`), in that type's catalogue and in `VERIFIED_TYPES.md`.

**Non-intrusiveness and operability**

- **FR-013**: The feature MUST NOT change `LatLng`'s or `Rect`'s public API, and MUST NOT
  change observable behaviour except through FR-017 to FR-019. It MUST
  NOT add runtime dependencies or slow the library down. Verification-only code MUST be
  excluded from normal builds.
- **FR-014**: A defect found by verification MUST be fixed as a separate, recorded change,
  with parity checked against upstream and a regression test that fails without the fix (red
  first, then green). The survey findings are handled inside this feature (FR-017 to FR-020).
- **FR-015**: The proofs MUST run from the project's documented verification command, with a
  documented way to run only this feature's proofs. They MUST run automatically in CI for
  changes to `LatLng`, `Rect`'s `Region` impl, `laws.rs` or the verification artifacts, and
  stay out of the default quality gate. Proofs over the per-change budget run in the weekly
  tier, with a CI leg.
- **FR-016**: The trusted base statement MUST be extended with every contract, stub, lemma and
  restatement this feature uses. The evaluation MUST record the effort, the run time, what was
  reachable, the limitations and the defects found, and rank the follow-ups: the remaining
  `Region` implementors, the next type, and the two other inexact remainders in the library
  (`s1::Interval`'s private `ieee_remainder`, 008's T058, and the projections' `remainder`).

**Known gaps (survey findings)**

- **FR-017** (K1): `LatLng::normalized` MUST reduce the longitude with the exact IEEE remainder
  by 2π, ties to even, as upstream's `remainder(lng, 2π)`: π stays π, −π stays −π, and a zero
  keeps its sign. It MUST share one exact reduction with `Angle::normalized` (which then maps
  −π to π) rather than add a third copy. Its result MUST be valid for every finite input. Its
  own change, with regression tests from the probe (10¹⁶, −2.462748394469102·10¹⁶, 10³⁰⁰, ±π,
  5π, −0, −2π, and `to_string_in_degrees` of (−0, −0)). The Python binding inherits it.
- **FR-018** (K3): `get_distance` MUST clamp before the square root, as upstream, so it returns
  upstream's result for every input, NaN where rounding makes the haversine term negative. Its
  own change, with the probe's case as its regression test.
- **FR-019** (K4, U-016): `Rect::cap_bound` MUST contain every point the rectangle contains.
  *Clarified 2026-10-27: widen the pole cap, a documented deviation, as 008 did for
  `Cap::union`'s U-015.*
  - The pole cap's radius MUST be widened by a stated error bound for converting a point and
    measuring its chord distance, justified in the rustdoc, in place of upstream's relative
    2ε; the cap around the centre stays upstream's unless verification or the vectors find it
    missing a point too, which is then a finding under FR-014.
  - The probe's 114 points MUST lie in the bound; its regression test fails before the change.
  - The coverings of rectangles may change where the wider cap moves a level; tests that pin
    them are updated and listed with the change, and the Python and wasm bindings inherit it.
  - U-016 MUST be recorded in `UPSTREAM.md` with its inputs, and the C++-vector rows of
    `GetCapBound` that differ are marked as deviations.
- **FR-020** (K5): Upstream's `S2LatLng` codec and `S2LatLngRect::Intersects(const S2Cell&)`
  are missing from the library. *Clarified 2026-10-27: follow-ups, not ported here.* The
  evaluation MUST rank both, and `VERIFIED_TYPES.md` MUST list them as missing upstream
  operations.

**Reference vectors, upstream findings and records**

- **FR-021**: The feature MUST add committed C++ reference vectors for `LatLng`, with
  differential tests: the constructors, `Latitude`, `Longitude`, the point constructor,
  `Normalized`, `ToPoint`, `GetDistance`, `ApproxEquals`, `ToStringInDegrees` and the
  arithmetic; and, for `Rect` as a region, `Contains(cell)`, `MayIntersect(cell)`,
  `GetCapBound` and `Contains(point)` for points on the boundary.
  - The vectors cover random values and the special cases: ±0, ±π, the ties, huge and
    non-finite longitudes, out-of-range latitudes, the poles, and the probe's cases.
  - Rows where the library deviates are marked as deviations; after FR-017 and FR-018, only
    FR-019's widened pole caps.
  - CI MUST NOT need a C++ toolchain to check them.
- **FR-022**: Undefined or wrong behaviour found in the upstream reference MUST be noted as a
  follow-up task before the affected proof is marked complete, with the upstream location,
  the inputs and the behaviour, and listed in the evaluation and in `UPSTREAM.md` (U-016 at
  least). Reporting it upstream needs the maintainer's approval.
- **FR-023**: As the constitution requires ("Verification records"), the change MUST update
  the records it affects in the same change: the value-type suite's inventory and catalogue;
  the index of properties by kind and its rendered copies; `VERIFIED_TYPES.md`, where
  `LatLng` gets a section of its own and §3's `Region` row gains `Rect`; `CORE_TRAITS.md`,
  where the `Region` invariants name their instances; and `verify/COVERAGE.md`.

### Key Entities

- **`LatLng`**: the verification target. A latitude and a longitude in radians, any pair of
  numbers; valid when both lie in their principal ranges.
- **`s2::Rect` as a `Region`**: the second target (FR-001a): its cell tests, point test and
  bounding cap, judged by the generic laws; its pole cap widened (FR-019).
- **Reference description**: angles in radians, validity, normalisation by whole turns with
  upstream's tie rule; restatements of upstream's formulas kept apart (FR-007).
- **Function contracts**: the tested contracts of `atan2`, `sin`, `cos`, `sin_cos` and `asin`
  (008, the coverage review), and any added under FR-003.
- **Generic trait laws**: the equality laws (004) and the `Region` laws (004, 008), stated once
  and instantiated per implementor.
- **Operation inventory, property catalogue, trusted base and evaluation**: as in 001–008.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope `LatLng` operation is proven free of panics, overflow and undefined
  behaviour over its whole documented domain, with no size bound; entries through an
  unmodelled function under a stated contract.
- **SC-002**: All in-scope entries of the 33 have a crash proof, or are recorded as tool limits
  with a measurement. Every exclusion has a written reason. Every contract passes its tests
  against the real function.
- **SC-003**: The catalogue covers every area in FR-006. At least 90% of the entries whose
  operations are all in scope and expressible are proven, over the whole domain, within
  stated bounds, by composition, or under a stated contract. Entries that need the value of
  an unmodelled function, or two of its calls on one input to agree, are not counted.
- **SC-004**: `Rect` has a proven instance of containment implies intersection and of "no
  contained vertex in a cell it does not intersect"; the bounding-cap and covering laws are
  proven or have a written reason; and after FR-019 no point of the probe or of the C++
  vectors that a rectangle contains lies outside its bounding cap.
- **SC-005**: 100% of proven properties have recorded non-vacuity evidence, and 100% of
  crash-freedom proofs have satisfied reachability checks.
- **SC-006**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine, a hosted Linux runner with 4 virtual CPUs and 16 GB.
- **SC-007**: Each regression test of FR-017 to FR-019 fails before its fix and passes after
  it. After the fixes, the probe's library-versus-upstream differences are 0 in every section
  but G, where they are exactly the widened pole caps, and G's misses are 0 in the library.
- **SC-008**: The differential tests against the new C++ reference vectors pass on every row,
  and the deviation rows are exactly those the rustdoc documents.
- **SC-009**: Apart from FR-017 to FR-019, the public API and behaviour of `LatLng` and `Rect`
  are unchanged, and the full quality gate passes before and after.
- **SC-010**: Every operation the proofs show can panic documents when it panics.
- **SC-011**: Every catalogue entry appears exactly once in the index of properties by kind,
  and the consistency check passes.
- **SC-012**: Every undefined or wrong upstream behaviour met has a follow-up task and an
  entry in the evaluation and in `UPSTREAM.md`.
- **SC-013**: A maintainer who did not take part can answer from the evaluation alone: "how
  much of `LatLng` is proven, which `Region` laws hold for `Rect`, and what comes next?"

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Users benefit through a `normalized`
  that always returns a valid `LatLng`, distances and bounds that match upstream, and a
  `Region` contract they can rely on.
- **The type**: `LatLng`, the next type on 008's list, with the list's first item (`Rect`'s
  `Region` laws) because every one of them goes through `LatLng` (FR-001a).
- **Builds on 001–009**: the verification setup, the memory-guarded runner, the value-type
  suite, the mutant campaign and its per-spec CI leg, the weekly tier, the generic laws, the
  function contracts and lemma stubs of 008 and the coverage review, 007's exact reduction and
  E-form enumeration, and 009's bounded types (`CellVertex` in the vertex law).
- **Lessons carried over**:
  - the checker's `f64` remainder is wrong, so FR-017's reduction is proven through C++ vectors
    and an exact property test, as 007's was;
  - CBMC's `cos` and `log2` are loose, and `x / ±∞` is reported as NaN, so a counterexample
    through them is replayed natively before a domain is narrowed;
  - two `sqrt` or `atan2` calls on one input may differ in the checker, so no harness compares
    two separately computed results bit for bit;
  - laws that recompute a value use lemma stubs (008).
- **Assurance level**: exhaustive model checking of the library's own code, as in 001–008,
  with the unmodelled functions under their tested contracts.
- **Reference semantics**: upstream C++ at v0.14.0-40, Release build on x86-64, with Go and Java
  secondary. Where the library deviates and documents it, the documented behaviour is
  verified.
- **Parity first** (Principle I): where only the library is wrong (K1, K3), it is fixed to
  upstream's result. Where upstream breaks its own contract (K4), the library deviates and
  documents it, at the maintainer's choice, as for U-011 and U-015.
- **No new public API**: the missing codec and exact cell test (K5) are follow-ups.
- **Validity stays a predicate**: `LatLng`'s fields are public and its constructors take any
  numbers, as upstream's do; making validity a type property would be a bounded type in 009's
  sense and an API change, out of scope here (a candidate for 009's follow-ups).
- **Out of scope**: `Rect`'s other operations (proven in 005 and the coverage review), except
  `cap_bound`'s widening; porting the missing upstream operations (K5); the
  accuracy of `to_point`, `from_point` and `get_distance` beyond their formulas (certified
  error bounds); text display; the other two inexact remainders (FR-016 ranks them); the
  `LatLng`-taking operations of other types beyond FR-012.
- **Survey findings**: K1–K5 were confirmed by the probe on 2026-10-27 against a running
  upstream build; the C++ vectors of FR-021 re-check them.
- **Existing tests stay**: the proofs complement the unit, property, differential and generic
  trait tests. No test is removed. Tests that pin old behaviour changed by FR-017 to FR-019 are
  updated, and the update is listed with the change.
