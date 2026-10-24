# Feature Specification: Formal Verification of `Cell` and `s1::Interval`

**Feature Branch**: `004-specs-2/1`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "004 Specify verification of Cell and s1::Interval."

## Context

### Where the earlier features left off

Feature 001 proved `CellId` with exhaustive bounded model checking: crash-freedom for 48
operations and a 49-entry property catalogue against an independent model of the cell
hierarchy. Feature 002 proved `CellUnion` within small size bounds. Feature 003 proved the
decoding layer, with deductive proofs of restated copies for the unbounded results.

The overview of verified types (`VERIFIED_TYPES.md`, step 3) plans a suite for the value types
next. Today `Cell` and `s1::Interval` are reached only through other proofs. A `Cell` is built
inside the `CellUnion` region proofs, and its binary codec is proven by 003. The circular
interval's validity check runs inside the rectangle decoder's proof. None of their own
operations is proven.

### Why these two types

- **`Cell`** is the layer directly above `CellId`. It is used in 53 source files: the region
  coverer, the cell index, the region interface and the distance queries. Its fields are
  private, and every one of them is derived from its cell identifier. 001's model already
  describes the face, the (i, j) position, the orientation and the level of an identifier,
  so the integer half of `Cell` can be checked against an independent model that exists.
- **`s1::Interval`**, the circular interval, is the longitude half of the latitude–longitude
  rectangle. Its wrap-around representation is the most error-prone among the value types:
  an interval may be inverted, π and −π are the same point, the empty interval is [π, −π]
  and the full one is [−π, π]. It is the first floating-point type this repository proves.
  Its operations are comparisons and additions, which the checker handles with exact
  IEEE-754 semantics. Together with the latitude interval, it opens the way to the rectangle.
- **Runner-up, `ChordAngle`**, is used more widely but is not ready. Its validity check
  accepts every negative value, so the order of its values is not what its documentation
  claims, and its arithmetic uses square roots and trigonometric functions, which are beyond
  the checker. It is a candidate for the evaluation's ranking (FR-016).

### What the pilot branch showed

The branch `claude/formal-methods-type-invariance-ept6u0` piloted both types against release
v0.3.2. `cell.rs` is unchanged since then, and the circular interval differs by one line.

| Type | Pilot result |
|---|---|
| `s1::Interval` | 10 laws proven over every valid interval and every point, 2–141 s each, under 0.5 GB: constructors, classification, membership, complement, adding a point, union, intersection, the relations, projection. Three first drafts failed: the minimal arc of a pair of points can exceed π by one ulp (the library is right, the draft was too strict); a single point's complement is the full circle; and the seam corner K4 below |
| `Cell` | 2 laws over every valid identifier: a cell reports its identifier, face, level and leaf status, and its edge size in (i, j) space (10 s, 0.4 GB); its children agree with the identifier's children (125 s, 0.8 GB) |

### Survey findings (2026-10-12, confirmed with concrete probes)

- **K1, undocumented panics in `Cell` construction.** Building a `Cell` from an invalid
  identifier panics, and no constructor documents it. The null identifier panics with an
  arithmetic overflow in debug builds and with "level must be 0..=30" in release builds. An
  identifier with face 6 or 7 panics with an out-of-bounds index in every build. Upstream's
  constructor checks nothing either, and its header states no precondition.
- **K2, deserialisation breaks the `Cell` invariant.** With the optional serialisation
  support, a `Cell` deserialises from any field values. The text form of a level-5 cell with
  its level changed to 12 deserialises into a cell that reports level 12 while its
  identifier has level 5. It still compares equal to the original, because equality compares
  identifiers only. This is the hole that the code review closed for `CellUnion` (finding
  #9, 2026-10-11). Planning found a second effect (research R6): the JSON library the tests
  use parses floating-point text to within one ulp unless its exact mode is enabled, so an
  honest round trip through JSON changed the stored u-v bounds of 375,836 of 1,598,279
  cells by one ulp. Such a cell's bounds then disagree with its identifier's.
- **K3, debug-only precondition checks in `s1::Interval`.** The constructor and the point
  operations check their arguments only in debug builds, and none documents a panic. A
  constructor call with a bound of 4.0, or a membership query for 4.0, panics in a debug
  build and returns silently in a release build, the constructor with an invalid interval.
- **K4, the seam corner of the length.** The interval from π to the next value above −π is
  valid, non-empty and one ulp long (4.4e-16), but its length is reported as −1, the value
  reserved for the empty interval: the difference of its bounds rounds to exactly −2π, and
  adding 2π back gives 0. Upstream has the same arithmetic, and `UPSTREAM.md` records it as
  inherited. The public API reaches the interval as the arc through π and the next value
  above −π, and the wrong length spreads to the operations built on it. Expanding the
  interval by 3.2 gives a short arc around 0, on the opposite side of the circle, instead of
  the full circle. At zero error it counts as approximately equal to the empty interval. A
  rectangle with it as its longitude range, and latitudes [0, 0.1], reports an area of
  −0.0998 and a longitude size of −1. A one-ulp interval anywhere else shows none of this.
  An existing property test claims that every non-empty interval has a non-negative length;
  random inputs never reach the corner.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for `Cell` and `s1::Interval` (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope operation of
`Cell` and of the circular interval completes without panicking, overflowing or hitting
undefined behaviour, for every input in its documented domain. Both types are fixed-size
values, so the proofs cover whole domains, with no size bound.

**Why this priority**: Crash-freedom is what the constitution already requires (Principle II).
K1 and K3 show that the documented domains are not yet stated, and the proofs make them
explicit. It is useful even if no functional property is proven.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope operation as proven free of panics, overflow and undefined behaviour
over its stated domain, with no unproven obligations remaining.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification
   command, **Then** it reports each in-scope operation by name as proven crash-free, states
   its domain, and exits successfully.
2. **Given** an operation with a documented precondition (for example "the identifier must be
   a valid cell" or "the point must lie in [−π, π]"), **When** verification runs, **Then** the
   proof assumes exactly that precondition and no stronger one, and the report states it.
3. **Given** an internal consistency check that runs only in debug builds, **When**
   verification runs, **Then** the proof shows that the check never fires for any input in
   the domain.
4. **Given** an in-scope operation deliberately changed so that it can panic inside its
   domain, **When** verification runs, **Then** it fails and names the operation and a
   concrete input that triggers the failure.

---

### User Story 2 - Proven meaning of a cell and of a circular interval (Priority: P2)

As the maintainer, I want a catalogue of functional properties for each type, each proven
against an independent description of what the type means:

- A `Cell` is its identifier, with precomputed facts. Its face, level, orientation and
  children must be those the identifier gives, and containment between cells must be
  containment between their identifiers.
- A circular interval stands for a set of points on the circle. Membership must agree with
  that set. The relations between intervals, and complement, union, intersection, adding a
  point and projection, must have the meaning upstream documents for them.

Each property is traced to upstream S2 semantics, which extends the parity guarantee
(Principle I) from sampled inputs to every input.

**Why this priority**: This is where formal methods add the most over the existing tests,
which sample inputs and never reached K4. It depends on the scaffolding from Story 1.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven or as not proven with a written reason. Each proven property fails when a matching
defect is injected into the implementation.

**Acceptance Scenarios**:

1. **Given** the property catalogue, **When** verification runs, **Then** every property
   marked proven is discharged with no remaining obligations.
2. **Given** any proven property, **When** a defect that violates it is introduced into the
   implementation, **Then** verification fails for that property.
3. **Given** any property, **When** a reviewer reads the catalogue or the index of properties
   by kind, **Then** they find a plain-language statement, its kind, the upstream source, the
   domain and the proof status.

---

### User Story 3 - Regression protection for future changes (Priority: P3)

As a contributor changing `Cell`, the circular interval, or the `CellId` and coordinate code
they build on, I want the proofs to run automatically in CI. If my change breaks a proven
property, I am told which property broke and given a concrete counterexample before the change
is merged.

**Why this priority**: Proofs that are not re-checked go stale. This protects Stories 1 and 2,
but it only matters once they exist.

**Independent Test**: Open a change that breaks one proven property of either type. The
automated verification check fails and names the property. Revert the change and the check
passes.

**Acceptance Scenarios**:

1. **Given** a proposed change to either type, to `CellId`, to the coordinate code, or to the
   verification artifacts, **When** CI runs, **Then** these proofs run and their result
   appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the existing
   quality gate is not slowed down or made to depend on verification toolchains.

---

### User Story 4 - Evaluation of floating-point value types (Priority: P3)

As the maintainer, I want a short written evaluation of this feature. It should cover the
effort, the run time, what was reachable, what is trusted but not proven, and the defects
found. It should answer whether exact checking of floating-point value types gives enough
assurance to justify the cost, and which value type or decision should come next.

**Why this priority**: 001–003 avoided floating-point code. This feature tests whether the
method extends to value types whose operations are comparisons and additions of floats. The
answer decides the shape of the rest of the value-type work.

**Independent Test**: A maintainer who did not do the work reads the evaluation and can answer
"should we continue with the value types?", "with which approach?" and "what next?" without
asking anyone.

**Acceptance Scenarios**:

1. **Given** the completed feature, **When** the evaluation is read, **Then** it contains a
   go or no-go recommendation for the remaining value types, the rationale, a comparison with
   001–003, and a ranked list of candidates for follow-up work.

---

### Edge Cases

- **Special intervals**: the empty interval [π, −π], the full interval [−π, π], single
  points, and intervals of every length from one ulp to the full circle.
- **The seam**: π and −π are the same point. −π appears only in the empty and full
  intervals, and every constructor turns it into π elsewhere. Inverted intervals cross the
  seam, and K4 is a one-ulp interval across it. By analysis, it is the only valid, non-empty
  interval whose length upstream's arithmetic gets wrong; a proof confirms this (FR-020).
- **Floating-point values**: negative zero next to positive zero, subnormal bounds, and the
  largest values below π. Point arguments outside [−π, π], NaN and the infinities are
  outside the documented domain of the point operations (K3). Other floating-point
  arguments, such as an expansion margin or an error bound, range over every value unless a
  precondition is documented.
- **Documented asymmetries**: a single point's complement is the full circle, so an
  interval and its complement can share interior points. Interior intersection is not
  symmetric. The properties state these as upstream documents them.
- **Approximate results**: the minimal arc through two points can exceed π by one ulp,
  because two operations evaluate the same quantity in different orders. Where upstream
  promises only an approximate result, the property states the tolerance and why.
- **Face and leaf cells**: the six face cells (level 0) and leaf cells (level 30), which have
  no children. The default cell is the face-0 cell.
- **Orientation**: all four orientations, which decide the order of a cell's children.
- **Invalid identifiers**: the null identifier, identifiers with face 6 or 7, and identifiers
  with no level marker. They are outside the domain of the `Cell` constructors and panic
  (K1). The proofs state the domain, and the documentation states the panics.
- **Inconsistent cells**: a `Cell` whose fields disagree with its identifier cannot come from
  any constructor, but it can come from deserialisation today (K2).
- **Equality**: cells compare equal when their identifiers do. Once K2 is fixed, equal
  identifiers imply equal fields, and a property states that.
- **Debug-only checks**: several interval operations guard their arguments, and one guards an
  internal consistency condition of the directed Hausdorff distance, with checks that run
  only in debug builds. Each must be proven safe for its domain, without relying on the
  check firing in release builds.

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify the `Cell` type and the circular interval type
  (`s1::Interval`) of the core library.
- **FR-002**: The verification artifacts MUST include an operation inventory for `Cell`. It
  lists its 43 inherent public operations, its 4 hand-written trait implementations
  (equality, the default value, conversion from an identifier, text display) and the 6
  methods of the region interface: 53 entries. Each entry is marked **in scope** or
  **excluded**, with a written reason for each exclusion. Excluded by default:
  - the 31 entries whose results depend on floating-point geometry: construction from a
    point or a latitude–longitude, the edge size in (s, t) space, the (i, j) coordinate of an
    edge, vertices, edges, centre, the four area measures, point containment, the cap and
    rectangle bounds, and the eleven distance operations, with their region-interface
    counterparts;
  - text display, which goes through the formatter that 001 excluded as a tool limit.

  The 19 integer entries are in scope: the identifier-based constructors, the accessors, leaf
  status, edge size in (i, j) space, children, cell containment and intersection, the
  covering bound, equality, the default value, conversion from an identifier, and the three
  region-interface methods that only read identifiers. The 2 entries that return the stored
  u-v bounds are also in scope. Their properties compare floating-point values computed two
  ways; if a check does not fit the budget, the entry becomes a tool-limit exclusion with its
  measurements. The binary codec of `Cell` is 003's, and the inventory refers to it.
- **FR-003**: The verification artifacts MUST include an operation inventory for the circular
  interval: its 28 inherent public operations and 3 hand-written trait implementations (the
  default value, equality, text display), 31 entries. Every entry except text display is in
  scope. An operation whose check does not fit the budget becomes a tool-limit exclusion with
  its measurements, and its properties stay in the catalogue as "not proven". Planning
  measures first the operations that the pilot left out: the centre, the complement's
  centre, expansion by a margin, the directed Hausdorff distance and approximate equality.

**Guarantees**

- **FR-004**: For every in-scope operation, verification MUST establish that the operation
  completes without panic, arithmetic overflow or undefined behaviour, for every input in its
  documented domain:
  - A `Cell` argument ranges over every cell built from a valid identifier. After FR-018,
    those are the only cells there are.
  - An identifier argument ranges over the valid cells, unless the operation documents a
    wider domain.
  - An interval argument ranges over every pair of 64-bit floating-point bounds that passes
    the validity check, including empty, full, inverted and single-point intervals.
  - A point argument ranges over every floating-point value in [−π, π].
  - Every other argument (a margin, an error bound, a face, a position, a level, an index)
    ranges over all values of its type, narrowed only by a documented precondition.

  A precondition counts as documented if the library's API documentation, a debug-build
  check or the upstream reference states it for the same operation. An internal consistency
  check that runs only in debug builds MUST be proven never to fire within the domain. If a
  proof must assume a precondition that the library's own documentation does not state, the
  undocumented panic MUST be recorded as a finding and documented (FR-017, FR-019).
- **FR-005**: Each type's invariant MUST be stated formally:
  - **Cell consistency**: a cell's face, level and orientation are those its identifier
    gives, and its u-v bounds are those computed from the identifier's (i, j) range and
    level.
  - **Interval validity**: neither bound exceeds π in absolute value, and −π appears only in
    the empty and full intervals.

  Verification MUST establish that every in-scope operation that returns a cell returns a
  consistent one, and that every in-scope operation that returns an interval returns a valid
  one whenever its inputs are in the domain.
- **FR-006**: The verification artifacts MUST include a property catalogue for each type,
  covering at least these areas:
  - **`Cell`**:
    - **Identity**: a cell reports the identifier it was built from, and the face, level,
      orientation and leaf status the reference model gives for it. Its edge size in (i, j)
      space is 2^(30 − level).
    - **Construction**: the face constructor, the position constructor, the default value and
      conversion from an identifier give the cell of the identifier the model gives.
    - **Children**: a cell has children exactly when it is not a leaf. They are the cells of
      the identifier's children, in curve order, and each one is consistent. Each child's
      u-v bounds equal those of the same cell built directly from its identifier. Upstream's
      test checks the vertices, edges and centre derived from them bit for bit.
    - **Containment**: cell containment and intersection agree with containment and
      intersection of the identifiers, and the region interface agrees with the inherent
      operations. The covering bound is the cell's own identifier.
    - **Bounds**: the stored u-v bounds are those the identifier's (i, j) range gives, and
      each edge coordinate is the matching side of those bounds.
    - **Equality**: two cells are equal exactly when their identifiers are, and equal
      cells have equal fields.
  - **The circular interval**:
    - **Validity and representation**: the validity check agrees with its formal statement
      for every pair of floats. Every constructor returns a valid interval for its
      documented inputs. The empty and full intervals are unique. Classification as empty,
      full and inverted is exhaustive and consistent.
    - **Membership**: point containment and interior containment agree with the reference
      description of the arc, and π and −π are treated as one point.
    - **Relations**: interval containment, interior containment, intersection and interior
      intersection agree with the reference description, including the documented
      asymmetry of interior intersection.
    - **Set operations**: the complement covers every point outside the interval. Union,
      intersection, adding a point and the arc through two points give the smallest interval
      that upstream documents for each. Projection returns the closest point of the
      interval.
    - **Measures**: the length, the centre, the complement's centre and the directed
      Hausdorff distance match their upstream definitions, apart from the deviation at the
      seam corner (FR-020). The length is non-negative for every non-empty interval, and
      positive unless the interval is a single point.
    - **Expansion and approximate equality**: expanding contains the original by the
      margin, and approximate equality is reflexive and agrees with its upstream definition.

  Each entry MUST record a plain-language statement, its kind, the upstream source, the
  domain and a status: **proven** (the whole domain is covered) or **not proven (reason)**. A
  loop bound that leaves part of a domain uncovered follows 001's rule, and the entry is then
  **proven within stated bounds**.
- **FR-007**: Properties MUST be stated against reference descriptions written from upstream
  S2 semantics that do not restate the implementation's code:
  - for `Cell`, 001's reference model of the cell hierarchy, extended with the u-v bounds of
    an (i, j) range as upstream defines them;
  - for the circular interval, a description of the set of points on the circle that an
    interval denotes, written from upstream's header.

  The proof then establishes agreement between two independent descriptions (Constitution
  Principle I).
- **FR-008**: Verification MUST be exhaustive and automated. For every in-scope operation and
  property, the checker MUST explore every input in the domain, not a sample. It MUST use
  exact bit-level machine-arithmetic semantics, including IEEE-754 floating point (rounding,
  signed zeros, infinities, NaN and subnormal values), and it MUST produce a concrete
  counterexample on failure. Floating-point properties hold exactly, except where upstream
  promises only an approximate result. There the property states the tolerance in ulps and
  the reason.
- **FR-009**: Every proven catalogue property MUST be shown to be non-vacuous: at least one
  injected defect MUST make its verification fail, and the injected defects and outcomes MUST
  be recorded. Crash-freedom proofs MUST show that their assumed domain is non-empty and that
  the outcomes they cover are reachable, for example an inverted and a non-inverted interval,
  an empty and a full one, a leaf cell and one with children.

**Readability**

- **FR-010**: Each catalogue entry MUST have a plain-language statement and one kind, from the
  kinds the project uses to browse its proofs (no crash, correct result, round trip, exact
  output, exact input, validity, law, model check). Each entry MUST appear in the project's
  index of properties by kind, and each proof's documentation MUST start with the entry's
  identifier, its kind and the plain-language statement. The existing consistency check
  between the catalogues, the index and the proofs MUST cover the new catalogues.

**Non-intrusiveness**

- **FR-011**: The feature MUST NOT change the public API of either type. It MUST NOT change
  observable behaviour either, except for FR-018 and FR-020. It
  MUST NOT add runtime dependencies to the published library, and it MUST NOT slow it down.
  Verification-only code MUST be excluded from normal builds.
- **FR-012**: A defect found by verification MUST be fixed as a separate, recorded change
  under the constitution, with parity checked against upstream (Principle I) and a regression
  test that fails without the fix (Principle III). The survey findings K1–K4 are the
  exception: they were found before this feature, and they are handled inside it
  (FR-017 to FR-020).

**Operability**

- **FR-013**: The proofs MUST run from the project's documented verification command, which
  reports pass or fail per operation and per property. A documented way to run only this
  feature's proofs MUST also exist.
- **FR-014**: The proofs MUST run automatically in CI, through the same command, for changes
  to either type, to `CellId`, to the coordinate code, or to the verification artifacts. They
  MUST stay out of the default quality gate. Proofs over the per-change budget MAY run in the
  weekly tier, and their results count like any other proof.
- **FR-015**: The trusted base statement MUST be extended to cover this feature, including:
  - every `CellId` result from 001 and every codec result from 003 that a proof relies on
    instead of re-proving;
  - the checker's model of IEEE-754 arithmetic, and any standard-library floating-point
    function taken as given rather than checked;
  - any loop bound used.

**Evaluation**

- **FR-016**: The feature MUST produce a written evaluation. It records the effort, the run
  time, what was reachable, the limitations hit and the defects found. It MUST also:
  - compare the cost and value with 001–003;
  - say whether exact checking of floating-point value types is worth its cost, and give a
    go or no-go recommendation for the remaining value types (the latitude interval, the
    latitude–longitude rectangle, `ChordAngle`, `Point`);
  - say whether type-level invariants (private fields with validating constructors) should
    come first for the types that lack them, as 003 recorded;
  - give a ranked list of candidates for follow-up work.

**Known defects and gaps**

- **FR-017** (K1): Every `Cell` constructor that takes an identifier, directly or through
  conversion, MUST document that the identifier must be a valid cell and when it panics
  (Constitution II). This changes no behaviour. If upstream's behaviour for such an
  identifier is undefined, that is assessed under FR-022.
- **FR-018** (K2): Deserialising a `Cell` MUST reject input whose identifier is not a valid
  cell, or whose other fields are not those the identifier gives. The face, level and
  orientation MUST match exactly. Each u-v bound MUST be within one ulp of the bound the
  identifier gives, so that text formats that parse floating-point numbers to within one ulp
  still read back what they wrote (research R6). The deserialised cell MUST carry the exact
  fields that the identifier gives, so a consistent cell deserialises unchanged. The
  serialised form MUST NOT change. This follows the decision for `CellUnion` (code review
  #9). The fix MUST be its own identifiable change, with a regression test that fails
  without it, and it is listed in the evaluation.
- **FR-019** (K3): Every interval operation that checks its arguments only in debug builds
  MUST document the precondition, that it panics in debug builds, and what happens in
  release builds. This changes no behaviour.
- **FR-020** (K4): The length of the one-ulp interval across the seam MUST be its true
  length, one ulp of π (4.4e-16), computed in a form that does not lose the ulp. The fix
  MUST be narrow:
  - every other interval's length MUST stay bit for bit what upstream's arithmetic gives, so
    that the rest of the library keeps exact parity, and a proof MUST show this for every
    valid interval (computing every length in the stable form would change the last bit for
    about 22% of inverted intervals, measured on 10⁶ random ones);
  - the operations that use the length (expansion, approximate equality, and the
    rectangle's size and area) MUST NOT change otherwise; they follow the corrected length.

  This is a deliberate deviation from upstream (Constitution I). The rustdoc of the length
  MUST state it and the reason, and `UPSTREAM.md` MUST record the corner as a numbered
  finding whose s2rst status is the deviation, instead of an inherited behaviour (FR-022).
  The existing property test that claims a non-negative length for every non-empty interval
  then holds as stated. Regression tests MUST cover the length, the expansion by 3.2 and the
  rectangle's area at the corner, each failing without the fix (Principle III). The fix is
  its own identifiable change, listed in the evaluation.

**Reference vectors and upstream findings**

- **FR-021**: The feature MUST add committed C++ reference vectors, with differential tests:
  - for the circular interval, every in-scope operation over random valid intervals and
    points, plus the special cases (empty, full, single points, both representations of the
    seam point, inverted intervals, the K4 corner). The vectors for the K4 corner, and for
    the results that follow from its length, are marked as the FR-020 deviation, as the
    existing vectors mark documented deviations;
  - for `Cell`, the fields of each cell, the identifiers of its children and the exact bit
    patterns of their u-v bounds.

  As for the earlier vectors, CI MUST NOT need a C++ toolchain to check them (Constitution
  I).
- **FR-022**: If the work shows undefined or wrong behaviour in the upstream reference, the
  finding MUST be noted as a follow-up task in the feature's task list before the affected
  proof is marked complete. The task records the upstream location, the inputs and the
  observed behaviour, and the finding is listed in the evaluation and in `UPSTREAM.md`. The
  feature does not change upstream code.

**Found by the proofs** (amendment, 2026-10-13)

- **FR-023** (U-011, D-007): Expansion MUST keep its containment: shrinking gives an
  interval the original contains, growing one that contains the original. Upstream's
  `Expanded` breaks it: shrinking an inverted arc to almost nothing can give almost the full
  circle, because rounding the bounds and reducing them across the seam exceeds its 2·ε
  allowance and the bounds cross. The weekly member `p_iexp_01_shrink__xinv` found it
  (2026-10-13), and C++ and Go give the same result. The user chose a fix in both
  directions. The fix MUST be narrow:
  - where upstream's result is not contained in the interval (shrinking), the result is the
    empty interval; where it does not contain the interval (growing), the full interval;
  - every other result MUST stay bit for bit upstream's, and a proof MUST show this for every
    valid interval and every margin except NaN (P-IEXP-03). No growing case is known, so that
    half is a guard.

  It is a deliberate deviation from upstream (Constitution I), stated in the rustdoc of the
  expansion with the reason, and recorded in `UPSTREAM.md` as U-011 with a follow-up task
  (FR-022). Regression tests MUST cover Kani's counterexample and the rectangle that inherits
  it, each failing without the fix (Principle III). The C++ vectors MUST include the
  counterexample, marked as the deviation (FR-021).

### Key Entities

- **`Cell`**: a verification target. A cell identifier with precomputed facts: face, level,
  orientation and u-v bounds.
- **Circular interval**: a verification target. A closed arc of the unit circle, given by two
  bounds in [−π, π], which may wrap across the seam.
- **Cell consistency and interval validity**: the formal invariants of the two types
  (FR-005).
- **Reference descriptions**: 001's model of the cell hierarchy, extended with u-v bounds, and
  the set of points an interval denotes (FR-007).
- **Operation inventories**: each operation listed in FR-002 and FR-003, with its status, the
  reason for any exclusion, its domain and its crash-freedom result.
- **Property catalogues**: the functional properties, each with a statement, a kind, an
  upstream source, a domain, a status and its non-vacuity evidence, also listed in the index
  of properties by kind.
- **Trusted base statement**: what the proofs rely on without proving it, including the
  results reused from 001 and 003 and the model of floating-point arithmetic.
- **Evaluation**: the findings, the comparison with 001–003 and the recommendation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope operation is proven free of panics, overflow and undefined
  behaviour over its whole documented domain (FR-004), with no size bound. A loop bound, if
  any, is documented under FR-008's rule.
- **SC-002**: All 19 integer `Cell` entries are in scope, and the 2 bound-reading entries
  are in scope or recorded as tool limits with measurements. At least 25 of the 31
  circular-interval entries (80%) are in scope. Every excluded entry has a written reason,
  and every tool-limit exclusion has its measurements.
- **SC-003**: Each catalogue has at least one functional property for each in-scope
  operation, and covers every area in FR-006. At least 90% of the entries whose operations
  are all in scope are proven; the rest are listed with a reason.
- **SC-004**: 100% of proven catalogue properties have recorded evidence of non-vacuity, and
  100% of crash-freedom proofs have satisfied reachability checks.
- **SC-005**: This feature's per-change proofs complete in under 10 minutes on the standard
  CI machine, a hosted Linux runner with 4 virtual CPUs and 16 GB of memory. The existing
  suites stay within their budgets. Proofs over the budget run in the weekly tier and finish
  within its time limit.
- **SC-006**: Apart from the documentation in FR-017 and FR-019 and the behaviour changes in
  FR-018 and FR-020, the public API and behaviour of both types are unchanged. No runtime
  dependencies are added, and the project quality gate passes before and after the feature.
- **SC-007**: A maintainer who did not take part can answer from the evaluation alone:
  "continue with the value types or stop?", "with which approach?", "are type-level
  invariants needed first?" and "what next?"
- **SC-008**: Every public operation in the inventories that the proofs show can panic
  documents when it panics, and the evaluation lists each documentation addition.
- **SC-009**: After the feature, deserialising a `Cell` with fields that disagree with its
  identifier fails, and every consistent cell deserialises unchanged, bit for bit, through
  JSON with the default float parsing (every cell to level 7 and 2 million random deeper
  cells). The regression test fails before the fix and passes after it.
- **SC-010**: After the feature, the one-ulp interval across the seam reports a length of
  4.4e-16. Expanding it by 3.2 gives the full circle, it no longer counts as approximately
  equal to the empty interval at zero error, and the rectangle in K4 reports a positive area.
  The length of every other valid interval is unchanged bit for bit. The regression tests
  fail before the fix and pass after it, the existing property test passes unchanged, and
  the rustdoc and `UPSTREAM.md` state the deviation.
- **SC-011**: The differential tests against the new C++ reference vectors pass for every
  vector, with the vectors marked under FR-020 matching the documented deviation.
- **SC-012**: Every catalogue entry appears exactly once in the index of properties by kind,
  and the consistency check between the catalogues, the index and the proofs passes.
- **SC-013**: Every undefined or wrong upstream behaviour met during the work has a follow-up
  task, created before the affected proof was marked complete, and an entry in the evaluation.

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Downstream users benefit through
  stronger guarantees about the cells and longitude ranges that coverings, indexes, bounds
  and queries are built from.
- **Builds on 001–003**: the feature reuses their verification setup: the reference model of
  the cell hierarchy, the input generators, the memory-guarded command runner, the mutant
  campaign, the CI jobs and the weekly tier, and the index of properties by kind. A proof may
  rely on a `CellId` property proven by 001 or a codec property proven by 003 instead of
  re-proving it, and the trusted base records each such reliance (FR-015).
- **Assurance level**: exhaustive model checking of the library's own code, as in 001–003.
  The deductive track of 003 is not used: its verifier cannot state floating-point
  arithmetic, and the integer half of `Cell` needs no unbounded proof because a cell is a
  fixed-size value.
- **Budget**: the pilot measured both types well within the per-harness budget of 4 GB and 12
  minutes used since 001, at whole domains. Planning re-measures on the current tree.
- **Reference semantics**: upstream C++ defines intended behaviour, with Go and Java as
  secondary references (Constitution I). The reference is the pinned upstream checkout
  (v0.14.0-40), as for 003. Where the library deliberately deviates and documents it, the
  documented behaviour is what gets verified.
- **The deserialisation fix (FR-018)** follows the decision already made for `CellUnion`:
  reject what is not a valid value, keep the format. The circular interval's fields are
  public, so deserialisation adds no way to build an interval that its constructor-free
  literal form does not already allow; it is left as it is.
- **Out of scope**:
  - making the circular interval's fields private, or adding validating constructors (a
    public API change that 003 recorded as a candidate for its own feature);
  - the floating-point half of `Cell` beyond the stored bounds (FR-002), the latitude
    interval, the rectangles, `ChordAngle` and `Point`, which the evaluation ranks;
  - the binary codec of `Cell`, already proven by 003;
  - validating input in the language bindings (Constitution IV); the bindings inherit the
    documentation and the FR-018 fix.
- **Survey findings**: K1–K4 were confirmed with concrete probes on 2026-10-12, in debug and
  release builds, before this specification was written.
- **Existing tests stay**: the proofs complement the existing unit, property, differential
  and robustness tests. No existing test is removed or weakened; the one property test that
  K4 contradicts holds once FR-020 is in place.
