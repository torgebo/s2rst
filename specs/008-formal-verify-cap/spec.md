# Feature Specification: Formal Verification of `Cap`, with the Trait Laws It Carries

**Feature Branch**: not created yet (`master` is checked out; no branch hook is configured)

**Created**: 2026-10-24

**Status**: Implemented 2026-10-25 (local commits, not pushed); clarified 2026-10-24 (FR-003:
option A, function contracts for `asin` and `atan2`); amended at planning the same day with
research R0–R0c (FR-003's `cos` contract, FR-003a's lemma stubs, SC-004's covering law). SC-001
is met in part and SC-003 is not met (88%); the other 11 criteria are met, SC-006 on the
workstation only (`verify/values/EVALUATION.md`, "Success criteria (spec 008)"). At the
maintainer's choice, D-009 is fixed and K8 matches upstream, and Kani's `thread_local!` crash
is recorded in `KANI_ISSUES.md` (2026-10-25).

**Input**: User description: "Spec 008: Use current work and VERIFIED_TYPES.md to figure out
current state and the natural next step on the way to verify as much of "core" as possible.
Suggest one type to focus on (either one we already have covered partly, or a new type). Focus
on covering that type as much as possible in smart ways. Also use the proofs we have generated
for traits to get verified properties included."

## Context

### Where the earlier features left off

- **001–003** proved `CellId`, `CellUnion` within size bounds, and the decoding layer, which
  includes `Cap`'s binary codec.
- **004–007** proved eight value types: `Cell`, `s1::Interval`, `r1::Interval`, `s2::Rect`,
  `r2::Rect`, `Point`, `ChordAngle` and `Angle`. 007 is complete and committed (`b30f92e`);
  nothing is pushed.
- **Generic trait laws exist and are reused per type** (`verify/src/laws.rs`): the equality
  laws (004), the integer `Region` laws, containment implies intersection and the covering
  covers every contained cell (004, instantiated for `Cell` only), and the total-order laws
  (006, instantiated for `Point`, `ChordAngle` and `Angle`). 003's `Codec` contract states the
  encode/decode laws once over all its codecs. The quickcheck twins of every trait invariant in
  `CORE_TRAITS.md` are in `core/tests/trait_property_tests.rs`.
- **`VERIFIED_TYPES.md` §2.8** lists `Cap` as "reached only through other proofs": its codec
  and validity rule are used, but its containment, union, bounds and `Region` impl have no
  proof. §3 lists `Cap` among the `Region` implementors with no proven method.
- **007's evaluation ranked `LatLng` next, then `Cap`.**

### Why `Cap`, not `LatLng`

This feature takes `Cap` first, against 007's ranking, for four reasons:

1. **It carries the trait laws.** `Cap` implements `Region`, `PartialEq`, `S2Encode` and
   `S2Decode`, so every generic law the project has can be instantiated for it. `LatLng`
   implements only `PartialEq`.
2. **It is what other `Region` laws are judged by.** "The bounding cap contains every point
   the region contains" holds for every `Region` implementor only if `Cap::contains_point`
   means what it should. The query layer's `DistanceTarget::cap_bound` returns a `Cap` too.
3. **Its parts are proven.** A cap is a `Point` (006) and a `ChordAngle` (007). Since 007 the
   chord angle keeps its own canonical-value invariant and its order is total, so a cap's
   containment tests are comparisons of proven values.
4. **More of it is within the checker's reach.** Most of `LatLng`'s surface goes through
   `atan2` (unsupported, 006) or `%` (mis-modelled, 007). `Cap`'s containment, intersection,
   complement, expansion and growth use only comparisons, `sqrt` and `sin`. `LatLng` stays
   next in line (FR-016).

### Survey findings (probed 2026-10-24)

The probe is in `probe/`: a Rust program on the library (release build) and its output. No
upstream C++ build was available, so upstream's behaviour is read from `s2cap.cc` and
`s2metrics.h` at `v0.14.0-40`. The probe transcribes upstream's cell tests over the library's
own `Point`, `Cell` and `ChordAngle` (`probe/README.md`).

- **K1, the covering level can be one too fine.** Upstream picks the level of
  `cell_union_bound` as `kMinWidth.GetLevelForMinValue(r) − 1`, computed with `ilogb`, which is
  exact. The library computes `floor(log2(d / r)) − 1` itself, and `log2` rounds a quotient
  just below 2ᵏ up to k.
  - For 180 of the 1,856 radii probed near the 29 powers of two, the library's level is one
    finer than upstream's. For an angle radius of 0.11785113019775795 rad it is 2, where
    upstream's is 1.
  - Upstream's `− 1` keeps the cells at least twice as wide as the radius, so that the four
    cells around one vertex cover the cap. At the finer level that margin is lost, by about
    one ulp.
  - The library already has upstream's computation (`MIN_WIDTH.max_level`) and does not use it
    here. This is a library defect.
- **K2, `contains_cell` is weaker than upstream.**
  - The library's `Region::contains_cell` tests whether the cap contains the cell's bounding
    cap.
  - Upstream tests the cell's four vertices, then whether the cap's complement meets the
    cell's edges.
  - For face cell 0 and caps centred 0–12° from the face centre, with radii 56–64°, the
    library answers false in 8 of 65 cases where upstream answers true. The first is 4° off
    centre with radius 58°.
  - The answer stays sound: a cap that contains the bounding cap contains the cell. But the
    coverer's results for caps differ from C++'s, which is a parity defect (Principle I).
- **K3, `intersects_cell` is weaker than upstream.**
  - The library's `Region::intersects_cell` tests the cell's bounding cap. Upstream's
    `MayIntersect` tests vertices and edges.
  - For a 2° cap centred at (0°, 50°) on face 1, the library says it intersects face cell 0;
    upstream says it does not. The answer is a false positive, which the trait allows, but it
    differs from C++, which is a parity defect.
- **K4, `complement`'s radius differs from upstream.**
  - Upstream returns the radius `FromLength2(4 − r²)`.
  - The library computes `STRAIGHT − r` through chord-angle subtraction, which squares a
    square root. The result differs in 472,249 of 10⁶ pseudo-random radii, by a few ulps.
  - This is a library defect.
- **K5, `rect_bound` can miss the cap's own centre.**
  - Upstream reduces the longitude bounds with the IEEE `remainder`, which is exact.
  - The library computes `(x + π).rem_euclid(2π) − π`, which rounds `x + π`.
  - A cap centred at longitude 10⁻¹⁴° with radius 10⁻¹⁸ rad gets the longitude interval
    `[0, 0]`, which does not contain its centre. That breaks the `Region` law "the bounding
    rectangle contains every point the region contains" (`CORE_TRAITS.md`). The quickcheck
    twin of that law did not find it, because its generator draws whole degrees.
  - 007 made `Angle::normalized` an exact remainder, which this reduction can reuse. This is
    a library defect.
- **K6, `approx_eq_with` ignores its tolerance for the centres** (read from the code, to be
  confirmed by a test).
  - Upstream compares both the centres and the radii with `max_error`, which defaults to
    10⁻¹⁴.
  - The library compares the centres with a fixed 10⁻¹⁵ and the radii with `max_error`.
  - This is a parity defect.
- **K7, deserialisation accepts invalid caps** (read from the code, to be confirmed by a test).
  - The serde deserialiser of `Cap` is derived. Since 007 the radius is checked, but the
    centre is any `Point`, so a NaN or non-unit centre gives a cap that is not valid.
  - The binary decoder rejects such caps (003). Earlier features closed the same gap for
    `CellUnion`, `Cell`, `s2::Rect`, `r2::Rect` and `ChordAngle`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for `Cap` (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope operation of `Cap`
completes without panicking, overflowing or hitting undefined behaviour, for every input in
its documented domain. A cap is a point and a chord angle, so the proofs cover whole domains
with no size bound.

**Why this priority**: The constitution requires crash freedom (Principle II). Several
operations have debug-build checks whose domains are not documented: `from_center_angle`
checks validity, `expanded` checks for a non-negative distance, and chord-angle addition
checks for special operands.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope `Cap` operation as proven free of panics, overflow and undefined
behaviour over its stated domain, with no unproven obligations.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification command,
   **Then** it reports each in-scope operation by name as proven crash-free, states its
   domain, and exits successfully.
2. **Given** an operation with a documented precondition (for example "the cap is valid" or
   "the distance is not negative"), **When** verification runs, **Then** the proof assumes
   exactly that precondition and no stronger one.
3. **Given** an internal consistency check that runs only in debug builds, such as the
   validity check in `from_center_angle` and `union`, **When** verification runs, **Then** the
   proof shows that it never fires within the domain, or records the input that makes it fire
   as a finding.
4. **Given** an in-scope operation deliberately changed so that it can panic inside its
   domain, **When** verification runs, **Then** it fails and names the operation and an input.

---

### User Story 2 - The generic trait laws, instantiated for `Cap` (Priority: P1)

As the maintainer, I want every generic law the project already states (the equality laws,
the `Region` laws and the codec contract) proven for `Cap`, and the `Region` laws that
`CORE_TRAITS.md` lists but the proofs do not yet state added to the generic laws, so that
`Cap` counts as a verified `Region`, `PartialEq` and codec implementor.

**Why this priority**: This is the cheapest gain: the laws are written once and reused. It
moves `Cap` in `VERIFIED_TYPES.md` §3 from "without" to "with proofs" for three traits. The
laws added here can be instantiated for the other `Region` implementors later.

**Independent Test**: Run the verification command. Each law instance for `Cap` is reported by
name as proven, or as not proven with a written reason. Injecting a defect into
`Cap::equal`, `contains_cell`, `intersects_cell`, `cell_union_bound`, `contains_point` or the
codec fails the matching instance.

**Acceptance Scenarios**:

1. **Given** the equality laws, **When** verification runs, **Then** `Cap`'s equality is
   proven reflexive, symmetric and transitive on valid caps, and any two empty caps, and any
   two full caps, are proven equal whatever their centres.
2. **Given** the `Region` laws, **When** verification runs, **Then** for every valid cap and
   every valid cell, containing the cell implies intersecting it, and a cell the cap does not
   intersect has no vertex the cap contains.
3. **Given** the law "the bounding cap contains every contained point", **When** verification
   runs, **Then** it is proven for `Cap` over every valid cap and every point, and is available
   generically for the other implementors.
4. **Given** 003's codec contract, **When** verification runs, **Then** the round trip of
   every valid cap is stated under `Cap`'s own equality, the trait law `CORE_TRAITS.md` names,
   and 003's existing proofs are reused, not re-run.

---

### User Story 3 - Proven meaning against an independent description (Priority: P2)

As the maintainer, I want a catalogue of functional properties of `Cap`, proven against a
description written from upstream's semantics. In that description, a cap is the set of unit
points whose chord distance from the centre is at most the radius; the empty cap is the empty
set and the full cap is the whole sphere.

**Why this priority**: This extends the parity guarantee (Principle I) from sampled inputs to
every input. The query layer and the coverer depend on a cap's containment and growth being
right.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven, proven within stated bounds, or not proven with a written reason. Each proven property
fails when a matching defect is injected.

**Acceptance Scenarios**:

1. **Given** the property catalogue, **When** verification runs, **Then** every property
   marked proven is discharged with no remaining obligations.
2. **Given** any proven property, **When** a defect that violates it is introduced, **Then**
   verification fails for that property.
3. **Given** a property about a value computed by upstream's arithmetic (height, area,
   centroid, the complement's radius, `add_cap`'s error margin), **When** a reviewer reads it,
   **Then** it states the result bit for bit against a restatement of upstream's formula,
   checked against the C++ vectors, or states its tolerance and why.

---

### User Story 4 - Parity restored where the survey found defects (Priority: P2)

As a library user, I want `Cap`'s coverings, cell tests, complement, bounding rectangle,
approximate equality and deserialisation to behave as upstream's do (K1–K7), each fixed as
its own change with a regression test.

**Why this priority**: K5 breaks a `Region` law that users and the coverer rely on. K1 removes
the covering's safety margin. K2 and K3 make coverings of caps differ from C++'s. These are
the findings of this feature, as earlier features fixed theirs.

**Independent Test**: Each probe case of `probe/` becomes a regression test that fails before
its fix and passes after. The C++ reference vectors for `Cap` (FR-024) match on every row,
except the documented deviations.

**Acceptance Scenarios**:

1. **Given** the cap at longitude 10⁻¹⁴° with radius 10⁻¹⁸ rad, **When** its bounding
   rectangle is computed, **Then** the rectangle contains the cap's centre.
2. **Given** a cap with angle radius 0.11785113019775795 rad, **When** its covering is
   computed, **Then** the cells are at upstream's level 1.
3. **Given** the probe's caps and face cell 0, **When** `contains_cell` and `intersects_cell`
   are evaluated, **Then** they return upstream's answers, and the coverer's result for a cap
   matches the C++ vectors.
4. **Given** any valid cap, **When** its complement is taken, **Then** the radius is
   upstream's `4 − r²` bit for bit.

---

### User Story 5 - Regression protection and evaluation (Priority: P3)

As a contributor changing `Cap`, I want the proofs to run automatically in CI and name the
property that broke, with a concrete counterexample. As the maintainer, I want a short
evaluation of what this feature reached, and a ranked list of what comes next.

**Why this priority**: Proofs that are not re-checked go stale. The evaluation decides the
next type (`LatLng` or another) and whether the generic `Region` laws should be instantiated
for the remaining implementors.

**Independent Test**: A change that breaks one proven property fails the automated check and
names it; the revert passes. A maintainer who did not do the work can answer from the
evaluation alone: "how much of `Cap` is proven, and what next?"

**Acceptance Scenarios**:

1. **Given** a change to `Cap`, its `Region` impl, its codec, or the verification artifacts,
   **When** CI runs, **Then** these proofs run and their result appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the default
   quality gate is not slowed down or made to depend on verification toolchains.
3. **Given** the completed feature, **When** the evaluation is read, **Then** it says which
   `Cap` entries are proven, which are limited and why, which `Region` laws are now generic,
   and ranks the follow-ups.

---

### Edge Cases

- **Empty and full caps with any centre.** `NEGATIVE` makes a cap empty and `STRAIGHT` makes
  it full, whatever the centre. Equality treats all empties as one and all fulls as one, as
  upstream. Their complements are each other. The complement of a single-point cap is also the
  full cap, so the complement is not a bijection (upstream documents this).
- **Zero radius.** `from_point(p)` contains `p` and nothing else in the description. Its
  interior is empty, so `interior_intersects` is false and `interior_contains_point` is false.
- **Antipodal and identical centres.** `contains` and `intersects` with the chord distance 4
  or 0. `union` of caps with identical centres should return the larger cap. With antipodal
  centres the great circle between them is not unique (006's K11 on `point_cross`), and
  `union`'s debug-build validity check may be reachable.
- **Radius at the boundaries.** Radii at 0, at 2 (a hemisphere, upstream's switch point in the
  cell tests), at 4, and the predecessors and successors of each.
- **Special distances in `expanded`.** A negative distance is outside the domain (a debug
  check, as upstream). A distance of +∞ gives the chord angle `INFINITY`, whose addition fails
  a debug check, as upstream. Both go in the domain's documentation.
- **Invalid caps.** A NaN or non-unit centre, and a radius that is a special value other than
  `NEGATIVE`. Every operation except `is_valid` takes a valid cap as its precondition, as
  upstream does.
- **Points that are not unit length.** `contains_point` takes a unit point as its precondition
  (upstream's debug check). The proofs state whether the library's answer still means anything
  without it.
- **The covering's level at powers of two** (K1), and caps large enough to need all six faces.
- **Longitudes near ±π and near 0** in `rect_bound` (K5), and caps that contain a pole.
- **Unmodelled functions.** `angle_radius` and everything after it (`union`, `rect_bound`,
  `cell_union_bound`) go through `asin`. `approx_eq`, `union` and `rect_bound` go through
  `atan2` (in `Vector::angle` and `LatLng`'s conversions). The checker models neither
  (research of 006 and 007), see FR-003.
- **Two `sqrt` results.** A law that recomputes a value its subject computed through `sqrt`
  (a cell vertex, a chord-angle sum) can fail in the checker for no real reason (006). Such a
  law is stated over the values the operation itself computed, and its link to the public
  method is argued and recorded.

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify `s2::Cap` and its trait implementations, and MUST
  instantiate the project's generic trait laws for it.
- **FR-002**: The verification artifacts MUST include an operation inventory for `Cap`:
  - its 32 inherent public functions;
  - its hand-written trait implementations: equality, text display, the six `Region` methods,
    `S2Encode` and `S2Decode`;
  - its derived deserialisation.

  That is 43 entries. Text display is excluded. The two codec entries are in scope, but their
  proofs are 003's; this feature reuses them and adds the law of User Story 2, scenario 4.
- **FR-003**: Entries that go through a function the checker does not model MUST be
  identified, with a measurement. These are `asin` (in `angle_radius`, and through it `union`,
  `rect_bound`, `cell_union_bound` and the two `Region` bounds) and `atan2` (in `approx_eq`,
  `approx_eq_with`, `union` and `rect_bound`). *Clarified 2026-10-24 by the maintainer:
  option A, a stated contract.*
  - In these entries' proofs, `asin` and `atan2` MUST be replaced by a **function contract**:
    a value chosen freely within the function's documented output range ([−π/2, π/2] for
    `asin`, [−π, π] for `atan2`), narrowed further only where the input's sign fixes the
    sign of the result, and NaN where the input is NaN.
  - Crash freedom and the structural properties after them (FR-006, "Bounds") are then proven
    for every value the contract allows, which includes the true one. They get the new status
    **proven under a stated contract**, naming the contract.
  - A property that needs two calls of the same function on the same input to agree (for
    example, "`rect_bound` contains the centre's latitude and longitude") is not provable
    this way, and stays with the C++ vectors and tests.
  - Each contract MUST be tested against the real function: every special value (±0, ±1,
    ±∞, NaN, the signed zeros of `atan2`'s arguments) and at least 10⁶ random inputs, in the
    verification crate's native tests, which CI runs.
  - This is the first replacement of a library dependency in any proof of 001–008. Every
    proof that does not go through `asin`, `atan2` or `cos` still runs the real code of the
    C library.
  - *Amended at planning (2026-10-24, research R0):* `cos` gets a contract too. The checker
    models it, but too loosely: it let `cos` of 1.1·10⁻³⁰⁸ be 0, which faked a division by
    zero in `rect_bound`. Its contract is [−1, 1], 1 at ±0, and at least 6·10⁻¹⁷ (the value
    at the `f64` nearest π/2) wherever |x| ≤ that `f64`. It is tested as the other two.
- **FR-003a** (*added at planning, 2026-10-24, research R0–R0c*): A law whose statement
  recomputes a value the operation already computed (the chord distance, `is_unit`, a
  chord-angle sum, a cell vertex) times out in the checker, though the library computes it
  deterministically. Such a law MAY be proven with that library function replaced by a
  **lemma stub**: a pure function (the same input gives the same value) that returns any
  value satisfying a lemma proven separately on the library's real code.
  - Each lemma MUST have its own harness on the real code, and each stub MUST name the lemmas
    it relies on. A stub that lacks a fact of the real code can only make a law fail, never
    pass, since it allows more behaviours than the real function.
  - The status is **proven by composition**, with the lemmas named. A law that also uses a
    function contract is **proven under a stated contract**.
  - Purity is a fact about the library, recorded in the trusted base: Rust never contracts
    floating-point operations on its own, and the explicit `mul_add` of `r3::Vector` is
    chosen per build, not per call.

**Guarantees**

- **FR-004**: For every in-scope operation, verification MUST establish that it completes
  without panic, arithmetic overflow or undefined behaviour, for every input in its
  documented domain:
  - a `Cap` argument ranges over every valid cap: a centre that passes `Point::is_unit`, and a
    radius that is any canonical chord angle of length² at most 4 (`NEGATIVE` and `STRAIGHT`
    included), narrowed only by a documented precondition;
  - a `Point` argument ranges over every unit point where upstream requires one, and over
    every point otherwise;
  - a `ChordAngle` argument ranges over every canonical value; an `Angle` or `f64` argument
    over every `f64`, NaN and the infinities included, narrowed only by a documented
    precondition;
  - a `Cell` argument ranges over every valid cell.

  A precondition counts as documented if the API documentation, a debug-build check or
  upstream states it. An undocumented precondition that a proof needs MUST be recorded as a
  finding and documented.
- **FR-005**: The cap's invariant MUST be stated formally: a cap is valid when its centre is a
  unit point and its radius is a canonical chord angle of length² at most 4. Verification MUST
  establish that every operation returning a cap returns a valid one for every valid input,
  or record the inputs where it does not as findings.
- **FR-006**: The verification artifacts MUST include a property catalogue for `Cap`, covering
  at least these areas:
  - **Construction**: `from_point`, `from_center_chord_angle`, `from_center_height`,
    `from_center_area` and `from_center_angle` are upstream's formulas, with upstream's clamps
    (a negative height, area or angle gives the empty cap; a height of 2, an area of 4π or an
    angle of π or more gives the full cap); `empty` and `full` are upstream's constants.
  - **Classification**: `is_valid`, `is_empty` and `is_full` as upstream defines them; empty
    and full are exclusive on valid caps.
  - **Measures**: `height`, `area` and `centroid` are upstream's formulas bit for bit; the
    area is never negative.
  - **Point containment**: `contains_point` is the description's membership test;
    `interior_contains_point` implies `contains_point`; the full cap contains every point and
    the empty cap none; `from_point(p)` contains `p`.
  - **Cap relations**: `contains` is reflexive on valid caps; the full cap contains every cap
    and every cap contains the empty cap; `contains` with a non-empty cap implies
    `intersects`; `intersects` is symmetric; `interior_intersects` implies `intersects`; the
    empty cap intersects nothing.
  - **Growth**: after `add_point(p)` the cap contains `p` (upstream's own claim, because both
    compute the same distance); after `add_cap(o)` it contains `o`, the claim upstream's error
    margin is there to keep; neither shrinks the cap; `expanded` by a non-negative distance
    contains the original cap.
  - **Complement**: upstream's centre and radius bit for bit (after FR-020); full and empty
    are each other's complement; the complement of the complement equals the cap wherever the
    radius arithmetic is exact, with the exceptions characterised.
  - **Bounds**: `cap_bound` returns the cap itself; `cell_union_bound` returns at most six
    identifiers, the six faces or the vertex neighbours at upstream's level (after FR-017),
    and that level keeps the cells at least twice as wide as the radius; `rect_bound` returns
    a valid rectangle (after FR-021), and the empty rectangle exactly for the empty cap.
  - **Equality**: `equal` and `==` are upstream's operator; `approx_eq_with` is upstream's
    `ApproxEquals` (after FR-022).
  - **The generic laws** (User Story 2): the equality laws; the `Region` laws; the codec
    round trip under `Cap`'s equality.

  Each entry MUST record a plain-language statement, its kind, the upstream source, the domain,
  and a status: **proven**, **proven within stated bounds**, **proven by composition** (from
  named proven entries, with the argument written down, as 003 did), **proven under a stated
  contract** (FR-003), or **not proven (reason)**.
- **FR-007**: Properties MUST be stated against a reference description written from
  upstream's semantics that does not restate the implementation's code: a cap is a centre and
  a radius in the ordered set of canonical chord angles, and denotes the unit points whose
  chord distance from the centre is at most the radius. Where a property is "bit for bit
  upstream's formula", the restatement of the formula is an explicit part of the description,
  kept apart from the meaning and checked against the C++ vectors, as 004–007 did.
- **FR-008**: Verification MUST be exhaustive and automated over every input in the domain,
  with exact bit-level IEEE-754 semantics, and MUST produce a concrete counterexample on
  failure. Floating-point properties hold exactly, except where upstream promises only an
  approximate result; there the property states its tolerance and the reason.
- **FR-009**: Every proven property MUST be shown to be non-vacuous: at least one injected
  defect MUST make it fail, and the injected defects and outcomes MUST be recorded.
  Crash-freedom proofs MUST show their domains non-empty and their outcomes reachable.

**The generic trait laws**

- **FR-010**: The generic `Region` laws MUST gain, stated once over any implementor, the
  `CORE_TRAITS.md` invariants they lack today, where they can be stated without float
  geometry:
  - the bounding cap contains every point the region contains;
  - a cell the region does not intersect has no vertex the region contains, stated over the
    vertices the region's own cell test computed (Edge Cases, "Two `sqrt` results").

  Each new law MUST be instantiated for `Cap`. Where an existing instance for `Cell`, or a
  new one for `Rect` or `CellUnion`, fits within the per-change budget, it SHOULD be added; if
  not, the evaluation lists it as a follow-up.
- **FR-011**: The quickcheck twins of the `Region` laws (`core/tests/trait_property_tests.rs`)
  MUST draw caps whose centres and radii are not limited to whole degrees, so that K5's kind
  of input is reachable. K5's case MUST be added to them as a fixed example.
- **FR-012**: A property of another type that follows from a proven `Cap` property MUST be
  recorded as proven by composition where the argument is short and checkable. Example:
  `Cell::cap_bound` builds its cap with `from_point` and four `add_point` calls, so the
  growth law shows that it contains the cell's centre and vertices as that function computed
  them. The record goes in that type's catalogue and in `VERIFIED_TYPES.md`.

**Non-intrusiveness and operability**

- **FR-013**: The feature MUST NOT change `Cap`'s public API, and MUST NOT change observable
  behaviour except through FR-017 to FR-023. It MUST NOT add runtime dependencies. It MUST NOT
  slow the library down, except that FR-018's cell tests cost what upstream's do.
  Verification-only code MUST be excluded from normal builds.
- **FR-014**: A defect found by verification MUST be fixed as a separate, recorded change,
  with parity checked against upstream and a regression test that fails without the fix. The
  survey findings K1–K7 are handled inside this feature (FR-017 to FR-023).
- **FR-015**: The proofs MUST run from the project's documented verification command, with a
  documented way to run only this feature's proofs. They MUST run automatically in CI for
  changes to `Cap`, its `Region` impl, its codec, `laws.rs` or the verification artifacts, and
  stay out of the default quality gate. Proofs over the per-change budget run in the weekly
  tier, with a CI leg for `Cap`.
- **FR-016**: The trusted base statement MUST be extended: the checker's models of `sqrt` and
  `sin` as `Cap` uses them; every restatement of upstream's arithmetic; every split of a
  domain; each function contract of FR-003, with the proofs that rely on it and the tests
  that check it against the real function; and each lemma stub of FR-003a, with its lemmas
  and the purity argument. The
  evaluation MUST record the effort, the run time, what was reachable, the limitations and the
  defects found. It MUST also give a go or no-go for `LatLng` next, say which `Region`
  implementors the generic laws can reach next, and rank the follow-ups.

**Known gaps (survey findings)**

- **FR-017** (K1): `cell_union_bound` MUST choose its level as upstream does,
  `MIN_WIDTH.max_level(r) − 1`, through the library's existing exact computation. This fixes
  a library defect, as its own change, with a regression test from the probe's radii.
- **FR-018** (K2, K3): `Region::contains_cell` and `Region::intersects_cell` for `Cap` MUST
  implement upstream's `Contains(const S2Cell&)` and `MayIntersect`: the four vertices first,
  then the edge test of `Intersects(cell, vertices)`.
  - This restores parity, so the coverer's results for caps change to C++'s. Tests that pin
    the old coverings are updated, and the updates are listed with the change.
  - The Python and WebAssembly bindings inherit the change. Their tests are updated in the
    same change if they pin old results.
  - This is its own change, with regression tests from the probe's cases.
- **FR-019** (K4): `complement` MUST return upstream's radius, `FromLength2(4 − r²)`. This
  fixes a library defect, as its own change, with a regression test.
- **FR-020** (K5): `rect_bound` MUST reduce its longitude bounds with the exact IEEE remainder
  by 2π, as upstream, reusing 007's exact reduction. Its result MUST contain the cap's centre
  for the probe's case. This fixes a library defect, as its own change, with a regression
  test.
- **FR-021**: `rect_bound` MUST return a valid rectangle for every valid cap. Verification
  states it (FR-006, "Bounds"); if it finds a counterexample, that is a defect under FR-014.
- **FR-022** (K6): `approx_eq_with` MUST compare the centres within `max_error`, and
  `approx_eq` MUST use upstream's default of 10⁻¹⁴ for both, once a test confirms K6. This
  restores parity, as its own change, with a regression test.
- **FR-023** (K7): Deserialising a cap MUST reject a cap that is not valid. A valid cap MUST
  deserialise unchanged, and the serialised form MUST NOT change. This follows the earlier
  deserialisation fixes, as its own change, with a regression test.

**Reference vectors, upstream findings and records**

- **FR-024**: The feature MUST add committed C++ reference vectors for `Cap`, with
  differential tests, for every operation upstream has: construction, the measures,
  containment, intersection, complement, expansion, union, `AddPoint`, `AddCap`, the three
  bounds, the cell tests and the coverer's result for caps.
  - The vectors cover random caps and the special cases: empty, full, single-point, hemisphere,
    antipodal and identical centres, the probe's cases, and radii next to the covering levels'
    powers of two.
  - Rows where the library deviates are marked as deviations, as the earlier vectors mark
    theirs. After FR-017 to FR-022, none is expected.
  - CI MUST NOT need a C++ toolchain to check them.
- **FR-025**: Undefined or wrong behaviour found in the upstream reference MUST be noted as a
  follow-up task before the affected proof is marked complete, with the upstream location,
  the inputs and the behaviour, and listed in the evaluation and in `UPSTREAM.md`. Reporting
  it upstream needs the maintainer's approval.
- **FR-026**: As the constitution requires (v1.2.0, "Verification records"), the change MUST
  update the records it affects in the same change: the value-type suite's inventory and
  catalogue; the index of properties by kind and its rendered copy; `VERIFIED_TYPES.md`, where
  `Cap` moves from §2.8 to a section of its own and §3's `Region`, `PartialEq` and codec rows
  gain it; `CORE_TRAITS.md`, where the `Region` invariants name their proven instances; and
  003's decoding records, where `Cap`'s codec entries point to the new round-trip law.

### Key Entities

- **`Cap`**: the verification target. A unit centre and a canonical chord-angle radius; the
  empty cap (radius `NEGATIVE`) and the full cap (radius `STRAIGHT`) with any centre.
- **Reference description**: a cap as a set of unit points, by chord distance from its
  centre; restatements of upstream's formulas kept apart (FR-007).
- **Generic trait laws**: the equality laws (004), the `Region` laws (004, extended by FR-010)
  and the codec contract (003), each stated once and instantiated per implementor.
- **Compositions**: properties of other types derived from proven `Cap` properties (FR-012).
- **Operation inventory, property catalogue, trusted base and evaluation**: as in 001–007.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope `Cap` operation is proven free of panics, overflow and undefined
  behaviour over its whole documented domain, with no size bound; the eight entries of FR-003
  under a stated contract for `asin` and `atan2`.
- **SC-002**: All 42 in-scope entries have a crash proof (the two codec entries through 003's),
  or are recorded as tool limits with a measurement. Every exclusion has a written reason.
  Each FR-003 contract passes its tests against the real function.
- **SC-003**: The catalogue covers every area in FR-006. At least 90% of the entries whose
  operations are all in scope and expressible are proven, over the whole domain, within
  stated bounds, by composition, or under a stated contract. Entries that need two calls of
  `asin`, `atan2` or `sqrt` on one input to agree are not counted (FR-003).
- **SC-004**: `Cap` has a proven instance of every generic law that applies to it: three
  equality laws, three `Region` laws (containment implies intersection, and the two of
  FR-010), and the codec round trip under its own equality. The fourth `Region` law, that the
  covering covers every contained cell, needs real geometry between the cap and the cells
  around a vertex; it is attempted in the weekly tier and recorded as not proven if it does
  not finish (amended at planning, research R0).
- **SC-005**: 100% of proven properties have recorded non-vacuity evidence, and 100% of
  crash-freedom proofs have satisfied reachability checks.
- **SC-006**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine, a hosted Linux runner with 4 virtual CPUs and 16 GB.
- **SC-007**: Each regression test of FR-017 to FR-023 fails before its fix and passes after
  it. After the fixes, the probe's five library-versus-upstream differences are gone.
- **SC-008**: The differential tests against the new C++ reference vectors pass on every row,
  and the deviation rows are exactly those the rustdoc documents.
- **SC-009**: Apart from FR-017 to FR-023, `Cap`'s public API and behaviour are unchanged, and
  the full quality gate passes before and after.
- **SC-010**: Every operation the proofs show can panic documents when it panics.
- **SC-011**: Every catalogue entry appears exactly once in the index of properties by kind,
  and the consistency check passes.
- **SC-012**: Every undefined or wrong upstream behaviour met has a follow-up task and an
  entry in the evaluation.
- **SC-013**: A maintainer who did not take part can answer from the evaluation alone: "how
  much of `Cap` is proven, which `Region` implementors can the generic laws reach next, and
  is `LatLng` next?"

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Users benefit through coverings, cell
  tests and bounding rectangles that match upstream and keep their guarantees.
- **The type**: `Cap`, chosen against 007's ranking for the reasons in "Why `Cap`, not
  `LatLng`". `LatLng` is left for the evaluation to rank (FR-016).
- **Builds on 001–007**: the verification setup, the memory-guarded runner, the value-type
  suite, the mutant campaign and its per-spec CI leg, the weekly tier and its CI legs, the
  generic laws, 003's `Cap` codec proofs, and 006's and 007's measurements of the modelled
  functions.
- **Lessons carried over**:
  - the checker mis-models ordered comparisons of booleans, so descriptions compare booleans
    only for equality;
  - it reports `x / ±∞` as NaN, so a domain is narrowed only after the counterexample is
    replayed;
  - two `sqrt` calls on one input may differ in the checker, so no harness compares two
    separately computed `sqrt` results bit for bit;
  - asserting on products of symbolic coordinates is slow (006's chord-angle symmetry timed
    out at 60 minutes), so statements over two centres expect the weekly tier or a split;
  - Kani's `%` on `f64` is wrong, so FR-020's reduction is proven through 007's results, not
    through `%`.
- **Assurance level**: exhaustive model checking of the library's own code, as in 001–007,
  except that the eight entries of FR-003 run with `asin` and `atan2` replaced by their range
  contracts (the maintainer's option A). Options B (keep them as tool limits) and C (contracts
  for crash freedom only) were not chosen.
- **Reference semantics**: upstream C++ at v0.14.0-40, Release build on x86-64, with Go and
  Java secondary. Where the library deviates and documents it, the documented behaviour is
  verified.
- **Parity first** (Principle I): where only the library is wrong or weaker (K1–K6), it is
  fixed to upstream's result. FR-018 changes the coverer's results for caps, because the
  constitution puts parity first.
- **Validity stays a precondition**: `Cap`'s fields are private, but making validity a type
  property would need a checked constructor in place of `from_center_chord_angle`, an API
  change. That is out of scope, as it was for the other value types except `ChordAngle`.
- **Out of scope**: `LatLng` beyond what `rect_bound` calls; `Rect`'s geometry beyond the
  validity of `rect_bound`'s result; the accuracy of `angle_radius`, `union` and the bounds
  beyond their formulas (certified error bounds); text display; the `DistanceTarget`
  implementors.
- **Survey findings**: K1–K5 were confirmed by the probe on 2026-10-24 against upstream read
  from source. K6 and K7 are read from the code and are confirmed by tests before they are
  fixed. The C++ vectors of FR-024 re-check K1–K5 against a running upstream build.
- **Existing tests stay**: the proofs complement the unit, property, differential and generic
  trait tests. No test is removed. Tests that pin old behaviour changed by FR-017 to FR-023
  are updated, and the update is listed with the change.
