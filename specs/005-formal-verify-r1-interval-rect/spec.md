# Feature Specification: Formal Verification of `r1::Interval` and `s2::Rect`

**Feature Branch**: `004-specs-2/1`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Verification of the next two types."

## Context

### Where the earlier features left off

- **001** proved `CellId`: crash freedom and a 49-entry catalogue against an independent model
  of the cell hierarchy.
- **002** proved `CellUnion` within small size bounds.
- **003** proved the decoding layer, including the binary codecs of `Cell` and the
  latitude–longitude rectangle.
- **004** proved `Cell` and the circular interval `s1::Interval`, and is at a checkpoint:
  - every in-scope operation of both is proven crash-free over its whole domain;
  - 31 of 40 properties are proven whole, among them the proof that the seam fix changes the
    length of exactly one interval;
  - its weekly members, mutant campaign and evaluation are still open.

  It introduced the reference description of an arc (`arc_spec`) and the generic trait laws
  (`laws`).

004's evaluation asks for a go or no-go on the remaining value types: the latitude interval,
the latitude–longitude rectangle, `ChordAngle` and `Point` (004 FR-016). This feature takes
the first two.

### Why these two types

- **`r1::Interval`**, the interval of real numbers, is the latitude half of the rectangle and
  both halves of the u-v rectangle. It is used in 16 source files.
  - It has no validity rule: any pair of bounds is an interval, empty when `lo > hi`.
  - Its operations are comparisons, minima and maxima, and single additions.
  - It is the simplest float type left, and the base of two others.
- **`s2::Rect`**, the latitude–longitude rectangle, bounds every loop, polygon and polyline,
  and drives region coverings. It is used in 25 files of `s2`.
  - It is the product of a latitude `r1::Interval` and a longitude `s1::Interval`. So it is the
    first composite value type, and it can build on 004's proofs instead of repeating them.
  - It already showed real defects: D-006 (its cap bound missed part of some rectangles across
    the antimeridian, fixed on 2026-10-12), and K4 of 004 (a rectangle with the seam corner as
    its longitude range had a negative area, fixed by 004's FR-020).
- **The runners-up:**
  - **`r2::Rect`, the u-v rectangle**, has pilot results and is `Cell`'s bound type, but it is
    less exposed to users. It is a candidate for the evaluation's ranking.
  - **`ChordAngle`** is a no-go until its validity check is settled: it accepts every negative
    value. That is a question of type-level invariants.
  - **`Point`** is open only for its ordering.

### What the pilot branch showed

The branch `claude/formal-methods-type-invariance-ept6u0` piloted both types against v0.3.2
(survey §4.6 and §4.11).

| Type | Pilot result |
|---|---|
| `r1::Interval` | 6 harnesses over every non-NaN pair of bounds and every point, 0.3–25 s each, under 0.8 GB: constructors and emptiness, membership, adding a point or an interval as least supersets, union and intersection laws, containment as a preorder, expansion and projection for finite values |
| `s2::Rect` | 8 harnesses over every valid rectangle and every valid latitude–longitude, 1–37 s each, 0.45 GB, all passing on the first run: constructors, classification, membership, adding a point, union, intersection, the relations, the polar closure. The operations through trigonometry or angle arithmetic were left to property tests |

### Survey findings (2026-10-12, confirmed with concrete probes)

- **K1, `r1::Interval`'s infinite corners.**
  - The length of the non-empty intervals `[∞, ∞]` and `[−∞, −∞]` is NaN, because `hi − lo`
    is `∞ − ∞`.
  - Expanding an interval with an infinite bound by an infinite margin of the opposite sign
    gives a NaN bound: `[−∞, 1].expanded(−∞)` is `[NaN, −∞]`, neither empty nor contained in
    anything.
  - Approximate equality of an interval with an infinite bound is false even with itself:
    `|∞ − ∞|` is NaN (found by the planning probe).
  - Upstream computes the same: `GetLength`, `Expanded` and `ApproxEquals` have the same
    arithmetic. Nothing documents any of these cases.
  - The directed Hausdorff distance meets `∞ − ∞` too. There the library's maximum ignores the
    NaN while upstream's depends on the argument order, so the two may differ; the C++ vectors
    check it (FR-021).
- **K2, out-of-range points in `s2::Rect`, a silent deviation.**
  - `add_point` ignores a latitude–longitude outside the valid range and returns the
    rectangle unchanged. `contains_lat_lng` returns false for one.
  - Upstream C++ only logs in debug builds, and then adds or tests the coordinates anyway.
  - The library follows Go here, a secondary reference. The deviation from C++ is not
    documented (Constitution I).
- **K3, debug-only panics in `s2::Rect`.** `from_lat_lng`, `from_point_pair` and
  `interior_contains_lat_lng` pass their longitude to the circular interval's checks. A longitude outside [−π, π]
  panics in a debug build and gives an invalid rectangle in a release build. Nothing
  documents a panic.
- **K4, two deserialisers that disagree.** The binary decoder of `s2::Rect` rejects an invalid
  rectangle (a documented deviation of 003). The serialisation support accepts it: a
  rectangle with a latitude bound of 2.0 deserialises. That is the inconsistency the code
  review closed for `CellUnion`, and 004's FR-018 closed for `Cell`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for the real interval and the rectangle (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope operation of
`r1::Interval` and `s2::Rect` completes without panicking, overflowing or hitting undefined
behaviour, for every input in its documented domain. Both are fixed-size values, so the
proofs cover whole domains, with no size bound.

**Why this priority**: Crash freedom is what the constitution already requires (Principle II),
and K3 shows that the rectangle's domains are not yet documented. It is useful even if no
functional property is proven.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope operation as proven free of panics, overflow and undefined behaviour
over its stated domain, with no unproven obligations.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification
   command, **Then** it reports each in-scope operation by name as proven crash-free, states
   its domain, and exits successfully.
2. **Given** an operation with a documented precondition (for example "the latitude–longitude
   must be valid"), **When** verification runs, **Then** the proof assumes exactly that
   precondition and no stronger one.
3. **Given** an internal consistency check that runs only in debug builds, **When** verification
   runs, **Then** the proof shows that it never fires within the domain.
4. **Given** an in-scope operation deliberately changed so that it can panic inside its
   domain, **When** verification runs, **Then** it fails and names the operation and an input.

---

### User Story 2 - Proven meaning, with the rectangle built on its intervals (Priority: P2)

As the maintainer, I want a catalogue of functional properties for each type, proven against
independent descriptions:

- an `r1::Interval` is the closed set of numbers between its bounds;
- a rectangle is the product of a latitude interval and a longitude arc, restricted to valid
  latitudes.

The rectangle's description must be built from the two interval descriptions, 004's arc
description among them, so that what 004 proved about the circular interval is reused, not
repeated.

**Why this priority**: This extends the parity guarantee (Principle I) from sampled inputs to
every input. It also tests whether the method composes: whether a product type is cheap once
its parts are proven.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven or as not proven with a written reason. Each proven property fails when a matching
defect is injected.

**Acceptance Scenarios**:

1. **Given** the property catalogue, **When** verification runs, **Then** every property
   marked proven is discharged with no remaining obligations.
2. **Given** any proven property, **When** a defect that violates it is introduced, **Then**
   verification fails for that property.
3. **Given** a rectangle property, **When** a reviewer reads its statement, **Then** it is
   stated through the two interval descriptions, and the trusted base names each result of
   004 it relies on.

---

### User Story 3 - Regression protection for future changes (Priority: P3)

As a contributor changing either type, or the circular interval and coordinate code they
build on, I want the proofs to run automatically in CI, and to be told which property broke,
with a concrete counterexample.

**Why this priority**: Proofs that are not re-checked go stale. This protects Stories 1 and 2.

**Independent Test**: A change that breaks one proven property fails the automated check and
names it. The revert passes.

**Acceptance Scenarios**:

1. **Given** a change to either type, the circular interval, or the verification artifacts,
   **When** CI runs, **Then** these proofs run and their result appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the default
   quality gate is not slowed down or made to depend on verification toolchains.

---

### User Story 4 - Evaluation of composite value types (Priority: P3)

As the maintainer, I want a short written evaluation:

- what composing proven types cost;
- what was reachable, and what is trusted but not proven;
- the defects found;
- whether the remaining value types should follow, and in which order.

**Why this priority**: The rest of the geometry is built from composite values, so the answer
decides how the verification scales.

**Independent Test**: A maintainer who did not do the work reads the evaluation and can answer
"continue with the value types?", "with which approach?" and "what next?" without asking anyone.

**Acceptance Scenarios**:

1. **Given** the completed feature, **When** the evaluation is read, **Then** it contains a
   go or no-go for the remaining value types (`r2::Rect`, `ChordAngle`, `Point`), the rationale, a
   comparison with 001–004, and a ranked list of follow-ups.

---

### Edge Cases

- **Empty intervals**: every pair with `lo > hi` is an empty real interval, with many
  representations. Upstream's empty interval is `[1, 0]`.
- **Infinite and signed-zero bounds**: infinities are valid bounds of a real interval (K1).
  `0.0` and `−0.0` compare equal.
- **NaN**: a NaN bound has no meaning in upstream. It is outside the domain of every
  operation, unless an operation documents otherwise.
- **The rectangle's validity**: latitudes within [−π/2, π/2], a valid longitude arc, and the
  latitude interval empty exactly when the longitude arc is. Struct literals reach invalid
  rectangles, because the fields are public. The proofs take valid ones, as upstream's
  operations require.
- **The poles and the antimeridian**: a rectangle touching a pole, and the polar closure that
  widens its longitudes to full; a longitude arc across ±π (inverted), and the seam corner of
  004.
- **Out-of-range points**: latitudes beyond ±π/2, longitudes beyond ±π, and NaN, as arguments
  to the operations that take a point (K2, K3).
- **Approximate results**: where upstream promises only an approximate result (for example
  approximate equality), the property states the tolerance and why.
- **Debug-only checks**: the circular interval's argument checks, reached through the
  rectangle's constructors (K3).

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify the real interval `r1::Interval` and the
  latitude–longitude rectangle `s2::Rect` of the core library.
- **FR-002**: The verification artifacts MUST include an operation inventory for
  `r1::Interval`:
  - its 23 inherent public operations;
  - its 4 hand-written trait implementations: the default value, equality, conversion from a
    pair of numbers, and text display.

  That is 27 entries. Every entry except text display is in scope.
- **FR-003**: The verification artifacts MUST include an operation inventory for `s2::Rect`:
  its 44 inherent public operations, its 3 hand-written trait implementations (equality, the
  default value, text display) and the 6 methods of the region interface, 53 entries.
  - **In scope: 34 entries.** These are the operations that compare, add or select the bounds:
    - construction from two intervals, the empty and full rectangles, construction from a
      point, from a centre and size, and from two points;
    - classification (valid, empty, full, a single point, inverted longitudes);
    - the bounds, vertices, centre and size;
    - membership of a latitude–longitude, interior membership, containment and intersection,
      interior containment and interior intersection;
    - adding a point, the polar closure, expansion by a latitude–longitude margin, union and
      intersection;
    - approximate equality, the full latitude and longitude ranges;
    - the rectangle as its own bound (inherent and through the region interface);
    - equality and the default value.
  - **Excluded: 18 entries whose results depend on floating-point geometry, and text display
    (19 in all).** These are the
    area, membership of a point given in 3-D, interior membership of such a point, expansion
    by a distance, the expansion for sub-regions, the cap bound, the covering bound, the
    centroid, the boundary-intersection test, the four distance operations, and the five region
    methods that go through points or caps.
  - **Planning measures first** the operations that go through the circular interval's
    expansion and approximate equality, which were expensive in 004: expansion, construction
    from a centre and size, and approximate equality.
  - The binary codec of `s2::Rect` is 003's, and the inventory refers to it.

**Guarantees**

- **FR-004**: For every in-scope operation, verification MUST establish that it completes
  without panic, arithmetic overflow or undefined behaviour, for every input in its documented
  domain:
  - an `r1::Interval` argument ranges over every pair of non-NaN bounds, infinities and empty
    intervals included, for the operations that compare and select bounds. For the ones whose
    arithmetic meets `∞ − ∞` (length, centre, expansion, the directed Hausdorff distance,
    approximate equality), it ranges over the pairs where that cannot happen. The infinite
    corners are documented by FR-017;
  - a number argument (a point, a margin, an error bound) ranges over every value of its
    type, narrowed only by a documented precondition;
  - an `s2::Rect` argument ranges over every valid rectangle;
  - a latitude–longitude argument ranges over every value for the operations that handle any
    value (adding a point, membership), and over the valid ones for the constructors that
    require them.

  A precondition counts as documented if the library's API documentation, a debug-build check
  or the upstream reference states it. An internal consistency check that runs only in debug
  builds MUST be proven never to fire within the domain. An undocumented precondition that a
  proof needs MUST be recorded as a finding and documented (FR-019).
- **FR-005**: Each type's invariant MUST be stated formally:
  - **The real interval** has no invalid non-NaN value. Its emptiness rule is `lo > hi`.
  - **Rectangle validity**: both latitude bounds within [−π/2, π/2], a valid longitude arc,
    and the latitude interval empty exactly when the longitude arc is.

  Verification MUST establish that every in-scope operation that returns a rectangle returns
  a valid one whenever its inputs are in the domain.
- **FR-006**: The verification artifacts MUST include a property catalogue for each type,
  covering at least these areas:
  - **`r1::Interval`**:
    - **Representation**: emptiness, and the constructors, including the arc through two
      numbers and conversion from a pair.
    - **Membership**: point and interior membership agree with the description.
    - **Relations**: containment, interior containment, intersection and interior
      intersection agree with the description.
    - **Set operations**: adding a point or an interval, union and intersection give the
      smallest interval upstream documents.
    - **Measures**: the centre, the length and the directed Hausdorff distance.
    - **Expansion, projection and approximate equality.**
  - **`s2::Rect`**:
    - **Validity and classification**, including the polar closure.
    - **Membership of a latitude–longitude**, and interior membership.
    - **Relations** and **set operations**, each the product of the two intervals'
      operations.
    - **The bounds, vertices, centre and size.**
    - **Expansion and approximate equality.**
    - **The generic laws**: the equality laws, and the region laws that apply to the
      rectangle as its own bound. They are instances of 004's generic laws, not restatements.

  Each entry MUST record a plain-language statement, its kind, the upstream source, the
  domain and a status: **proven** (the whole domain is covered), **proven within stated
  bounds**, or **not proven (reason)**.
- **FR-007**: Properties MUST be stated against reference descriptions written from upstream
  S2 semantics that do not restate the implementation's code:
  - for the real interval, the closed set of numbers between its bounds, empty when the low
    bound exceeds the high one;
  - for the rectangle, the product of that description for latitudes and 004's description
    of an arc for longitudes, restricted to valid latitudes.

  A rectangle property MAY rely on a circular-interval result proven by 004 instead of
  re-proving it. The trusted base records each such reliance.
- **FR-008**: Verification MUST be exhaustive and automated over every input in the domain,
  with exact bit-level IEEE-754 semantics, and MUST produce a concrete counterexample on
  failure. Floating-point properties hold exactly, except where upstream promises only an
  approximate result; there the property states its tolerance in ulps and the reason.
- **FR-009**: Every proven property MUST be shown to be non-vacuous: at least one injected
  defect MUST make it fail, and the injected defects and outcomes MUST be recorded.
  Crash-freedom proofs MUST show their domains non-empty and their outcomes reachable.

**Readability**

- **FR-010**: Each catalogue entry MUST have a plain-language statement and one kind from the
  kinds the project uses. It MUST appear in the index of properties by kind, and each proof's
  documentation MUST start with the entry's identifier, kind and plain statement. The
  consistency check between the catalogues, the index and the proofs MUST cover the new
  catalogue.

**Non-intrusiveness**

- **FR-011**: The feature MUST NOT change the public API of either type, and MUST NOT change
  observable behaviour, except for FR-020. It MUST NOT add runtime dependencies or slow the
  library down. Verification-only code MUST be excluded from normal builds.
- **FR-012**: A defect found by verification MUST be fixed as a separate, recorded change,
  with parity checked against upstream and a regression test that fails without the fix. The
  survey findings K1–K4 are the exception: they are handled inside this feature (FR-017 to
  FR-020).

**Operability**

- **FR-013**: The proofs MUST run from the project's documented verification command, which
  reports pass or fail per operation and per property, with a documented way to run only this
  feature's proofs.
- **FR-014**: The proofs MUST run automatically in CI, through the same command, for changes
  to either type, to the circular interval, to the coordinate types they take, or to the
  verification artifacts. They MUST stay out of the default quality gate. Proofs over the
  per-change budget MAY run weekly.
- **FR-015**: The trusted base statement MUST be extended, including:
  - the results of 004 that the rectangle's proofs rely on;
  - the checker's model of IEEE-754 arithmetic;
  - any split of a domain.

**Evaluation**

- **FR-016**: The feature MUST produce a written evaluation. It records the effort, the run
  time, what was reachable, the limitations and the defects found. It MUST also:
  - measure what reusing 004's results saved, against proving the rectangle on its own;
  - give a go or no-go for `r2::Rect`, `ChordAngle` and `Point`;
  - say whether type-level invariants should come first, for the rectangle's public fields;
  - give a ranked list of follow-ups.

**Known gaps**

- **FR-017** (K1): The real interval's arithmetic operations MUST document their infinite
  corners:
  - the length of `[∞, ∞]` and `[−∞, −∞]` is NaN;
  - the centre of `[−∞, ∞]` is NaN;
  - an infinite margin against an infinite bound of the opposite sign gives a NaN bound;
  - approximate equality is false for an interval with an infinite bound, even with itself;
  - the directed Hausdorff distance at infinite bounds.

  They MUST also say that upstream computes the same, or document the difference if the C++
  vectors show one. This changes no behaviour.
- **FR-018** (K2): `add_point` and `contains_lat_lng` MUST document that an out-of-range
  latitude–longitude is ignored or not contained. This is a deviation from upstream C++ that
  follows Go, with the reason (Constitution I). This changes no behaviour.
- **FR-019** (K3): Every rectangle operation that checks its arguments only in debug builds
  MUST document the precondition, the debug-build panic, and what release builds do. These are
  the constructors from points, and interior membership of a latitude–longitude, which, unlike
  membership, does not screen out an out-of-range one. The same
  holds for `r1::Interval::project` on an empty interval, which states its precondition but
  not its panic. This changes no behaviour.
- **FR-020** (K4): Deserialising an `s2::Rect` MUST reject a rectangle that is not valid, as
  its binary decoder does (003). A valid rectangle MUST deserialise unchanged, and the
  serialised form MUST NOT change. The fix MUST be its own identifiable change, with a
  regression test that fails without it.

**Reference vectors and upstream findings**

- **FR-021**: The feature MUST add committed C++ reference vectors, with differential tests:
  - every in-scope operation of the real interval over random intervals and the special cases
    (empty, infinite, signed-zero and single-point intervals);
  - every in-scope operation of the rectangle over random valid rectangles and the special
    cases (the poles, the antimeridian, the empty and full rectangles, and 004's seam corner
    as a longitude range).

  CI MUST NOT need a C++ toolchain to check them.
- **FR-022**: Undefined or wrong behaviour found in the upstream reference MUST be noted as a
  follow-up task before the affected proof is marked complete, with the upstream location, the
  inputs and the behaviour, and listed in the evaluation and in `UPSTREAM.md`.

### Key Entities

- **`r1::Interval`**: a verification target. A closed interval of real numbers, empty when
  its low bound exceeds its high bound.
- **`s2::Rect`**: a verification target. A latitude interval and a longitude arc.
- **Rectangle validity**: the formal invariant of the rectangle (FR-005).
- **Reference descriptions**: the set of numbers an interval denotes, and the rectangle as the
  product of that and 004's arc description (FR-007).
- **Operation inventories, property catalogues, trusted base and evaluation**: as in 001–004.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope operation is proven free of panics, overflow and undefined
  behaviour over its whole documented domain, with no size bound.
- **SC-002**:
  - All 26 in-scope `r1::Interval` entries are in scope with a crash proof.
  - At least 30 of the 34 in-scope rectangle entries (88%) have one. The rest are recorded
    as tool limits with measurements.
  - Every exclusion has a written reason.
- **SC-003**: Each catalogue covers every area in FR-006. At least 90% of the entries whose
  operations are all in scope are proven, over the whole domain or within stated bounds.
- **SC-004**: 100% of proven properties have recorded non-vacuity evidence, and 100% of
  crash-freedom proofs have satisfied reachability checks.
- **SC-005**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine, a hosted Linux runner with 4 virtual CPUs and 16 GB.
- **SC-006**: Apart from the documentation in FR-017 to FR-019 and the change in FR-020, the
  public API and behaviour of both types are unchanged, and the quality gate passes before
  and after.
- **SC-007**: A maintainer who did not take part can answer from the evaluation alone:
  "continue with the value types?", "with which approach?", "type-level invariants first?" and
  "what next?".
- **SC-008**: Every operation the proofs show can panic documents when it panics.
- **SC-009**: After the feature, deserialising an invalid rectangle fails, every valid one
  deserialises unchanged, and the regression test fails before the fix and passes after it.
- **SC-010**: The evaluation states, for the rectangle, the share of its properties that rely
  on 004's results and the time saved against proving them again.
- **SC-011**: The differential tests against the new C++ reference vectors pass for every
  vector.
- **SC-012**: Every catalogue entry appears exactly once in the index of properties by kind,
  and the consistency check passes.
- **SC-013**: Every undefined or wrong upstream behaviour met has a follow-up task and an
  entry in the evaluation.

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Users benefit through stronger
  guarantees about the bounds that coverings, indexes and queries are built from.
- **Builds on 001–004.** The feature reuses:
  - the verification setup, the memory-guarded runner, the mutant campaign, CI and the weekly
    tier;
  - 004's arc description, generic laws and generators.

  004's open tasks (its weekly members, mutants and evaluation) do not block this feature.
  Verification runs from both go one at a time.
- **Lessons from 004 carried over**:
  - the checker mis-models ordered comparisons of booleans, so descriptions compare booleans
    only for equality;
  - relations between intervals are those of real sets, not of float points;
  - two-interval properties through the circular interval's expansion and approximate
    equality were over the per-change budget.
- **Assurance level**: exhaustive model checking of the library's own code, as in 001–004.
- **Budget**: the pilot measured both types well within the per-harness budget (4 GB, 12
  minutes). Planning re-measures on the current tree.
- **Reference semantics**: upstream C++ at v0.14.0-40, with Go and Java secondary. Where the
  library deviates and documents it, the documented behaviour is verified.
- **The deserialisation fix (FR-020)** follows the binary decoder of the same type and the
  decisions for `CellUnion` and `Cell`: reject what is not valid, keep the format. 004 left
  the circular interval's deserialisation as it is. That type has no validity rule beyond
  its bounds' range, and its fields are public. The evaluation revisits it.
- **Out of scope**:
  - making the rectangle's or the intervals' fields private (type-level invariants, a public
    API change for its own feature);
  - the rectangle's floating-point geometry (FR-003);
  - `r2::Rect`, `ChordAngle` and `Point`, which the evaluation ranks;
  - the rectangle's binary codec, proven by 003;
  - validating input in the language bindings, which inherit the documentation and FR-020.
- **Survey findings**: K1–K4 were confirmed with concrete probes on 2026-10-12, in debug and
  release builds.
- **Existing tests stay**: the proofs complement the existing unit, property, differential
  and generic trait tests. No test is removed.
