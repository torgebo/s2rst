# Feature Specification: Formal Verification of `r2::Rect` and `Point`

**Feature Branch**: `004-specs-2/1`

**Created**: 2026-10-13

**Status**: Draft

**Input**: User description: "Verification of the next two types." Revised the same day: "skip
this type [`ChordAngle`], make a note to pick it up later."

## Context

### Where the earlier features left off

- **001–003** proved `CellId`, `CellUnion` within size bounds, and the decoding layer.
- **004** proved `Cell` and the circular interval `s1::Interval`. On 2026-10-13 its weekly
  members found U-011, upstream's `Expanded` crossing its bounds, and the library now
  deviates there (004 FR-023). Its mutant campaign killed all 42 mutants; its evaluation is
  still to be written.
- **005** proved, or is proving, the real interval `r1::Interval` and the latitude–longitude
  rectangle `s2::Rect`. The rectangle is the first composite value type: its description is
  the product of the two interval descriptions. Its re-runs passed (26 of 26 and 37 of 37 on
  2026-10-13); its mutants and weekly members are running.

Both features ask their evaluations for a go or no-go on the remaining value types, in this
order: `r2::Rect`, `ChordAngle`, `Point` (005 FR-016). This feature takes `r2::Rect` and
`Point`. `ChordAngle` is deferred, with its survey findings and its open question kept below.

### Why these two types

- **`r2::Rect`**, the u-v rectangle, is two `r1::Interval`s, one per axis.
  - It is `Cell`'s bound type: 004's bound families (P-CBND) state cells' u-v bounds with it.
    It is also used in 8 other source files: `CellId`'s bounds, the padded cell, the shape
    index, the crossing-edge query and the three edge clippers.
  - Its validity rule is that both intervals are empty or neither is. Its operations are
    comparisons, minima, maxima and single additions, as for `r1::Interval`.
  - It is the second product type, and the first whose two factors are the same proven
    type. It can reuse 005's real-interval description unchanged.
- **`Point`**, the point on the sphere, is the most used type in the library: 111 source
  files name it, and the Python and WASM bindings expose it. It is a 3-vector of `f64` whose
  unit length is a convention, not a rule the type enforces.
  - Its value-level surface is comparisons, selections and exact operations: the
    lexicographic order that the predicates' symbolic perturbation relies on, the
    classifications (`is_unit`, `is_normalizable`), the power-of-two scaling
    (`ensure_normalizable`), negation, addition and subtraction, and the orthogonal
    directions with their sign laws.
  - The rest is floating-point geometry (distances, rotation). This feature proves it free of
    crashes and proves its exact laws (zero distance to itself, symmetry); its accuracy needs
    certified error bounds, a different technique, and stays out of scope.
  - The survey found a library defect in `ensure_normalizable` and a candidate upstream
    finding in the robust cross product, both in the scaling that `ensure_normalizable`
    performs (K5, K6).

### What the pilot branch showed

The branch `claude/formal-methods-type-invariance-ept6u0` piloted both types against v0.3.2
(survey §4.11).

| Type | Pilot result |
|---|---|
| `r2::Rect` | 5 harnesses over every rectangle of two `r1` intervals, 8–41 s each, 0.47 GB: constructors, vertices, adding a point or a rectangle and union as least supersets, intersection against `intersects`, containment as a preorder. Expansion with projection took 545 s |
| `Point` | 3 harnesses over every point without NaN, 7–9 s each, 0.47 GB: `cmp_point` antisymmetric, transitive, lexicographic and consistent with `PartialOrd`, and its NaN behaviour pinned. Classifying with `is_unit` and `is_normalizable` took 1802 s: three products of symbolic doubles, the first harness there bound by solver time rather than memory |

The pilot's rule from the second row: a proof may compute with products of symbolic doubles
freely, but asserting on them is slow.

### Survey findings (2026-10-13, confirmed with concrete probes)

Each was probed on the current tree in release builds, and against upstream C++
(`v0.14.0-40`, Release) where upstream is involved.

**`r2::Rect`**

- **K1, debug-only checks.** `new` and `from_points` check validity only in debug builds:
  `new(empty, [0, 1])` and `from_points((1, 0), (0, 1))` panic in debug builds and return an
  invalid rectangle in release builds. Nothing documents a panic. `project` on the empty
  rectangle panics in every build, through `r1::Interval::project` (U-008), and does not say
  so.
- **K2, `from_center_size` checks nothing.** A size with one negative component,
  `((0, 0), (−1, 1))`, gives an invalid rectangle in every build. Upstream's `FromCenterSize`
  goes through the checking constructor, so its debug builds stop there.
- **K3, deserialisation accepts invalid rectangles.** `{"x": [1, 0], "y": [0, 1]}`, one
  interval empty and the other not, deserialises. 004 (`Cell`) and 005 (`s2::Rect`) closed the
  same gap.
- **K4, infinite and NaN corners**, inherited from `r1::Interval` (005 K1): the size of a
  rectangle with an `[∞, ∞]` side has a NaN component. A NaN coordinate gives a rectangle that
  counts as valid and non-empty but contains nothing, not even the point it was built from.
  Nothing on `r2::Rect` documents either.

**`Point`**

- **K5, `ensure_normalizable` breaks for the smallest inputs.** Its documentation and
  upstream's comment say it scales a non-zero vector by a power of two so that the largest
  component lands in [1, 2). It builds that power of two from exponent bits, and when the
  largest component is below 2⁻¹⁰²³ the biased exponent reaches zero or below. For
  `(x, 0, −x/2)`:
  - at x = 2⁻¹⁰²⁴ it returns `(∞, NaN, −∞)`;
  - from 2⁻¹⁰²⁵ down to 2⁻¹⁰⁷⁴ it returns the zero vector with every sign flipped.

  Neither result is normalizable. Upstream's `EnsureNormalizable` is private to
  `s2edge_crossings.cc`, and Go does not port it. A library defect, in a public function that
  nothing in the library calls.
- **K6, the robust cross product overflows, in upstream too.** The library has a second,
  private copy of the scaling for `robust_cross_prod`, written differently again:
  - It scales into [0.5, 1), not [1, 2) as its comment and upstream say, so
    `robust_cross_prod` differs from C++ by a factor of two whenever it scales: for the
    antipodal unit points `b = (2⁻³⁰⁰, 0, 1)` and `−b` it returns `(−0, 0.5, 0)` where C++
    returns `(−0, 1, 0)`. The direction is the same.
  - Upstream computes the scale as `ldexp(2, −1 − ilogb(p_max))`, which overflows to infinity
    when the largest component is below 2⁻¹⁰²³. For `b = (2⁻¹⁰⁷⁴, 0, 1)`, a unit point, C++
    `S2::RobustCrossProd(−b, b)` returns `(NaN, ∞, NaN)` (observed for `b = (2⁻ᵏ, 0, 1)` at
    k = 1024 and k = 1074). The library's copy overflows one power of two later, from k = 1025,
    and then returns the same.

  A candidate upstream finding: a NaN cross product from two valid unit points.
- **K7, `from_coords` leaves the sphere for huge and tiny coordinates.** Its documentation
  says only that the zero vector gives `origin`. When the squared length overflows (a
  coordinate above about 1.34·10¹⁵⁴), `from_coords(1e200, 1, 1)` is the zero vector; when it
  underflows, `from_coords(1e-170, 1e-170, 0)` is the zero vector too, not `origin`; and when
  it is subnormal, `from_coords(1e-160, 0, 0)` has x = 1.0000055664551362, not unit length.
  `normalize` behaves the same way. Upstream's `Normalize` (C++, observed) and Go's
  `PointFromCoords` compute the same: an inherited, undocumented limit.
- **K8, NaN in the order.** `cmp_point` treats a NaN coordinate as equal and moves on to the
  next, so `(1, 0, 0) < (NaN, 5, 0) < (0, 9, 0) < (1, 0, 0)` is a cycle. C++'s
  `operator<` gives the same cycle (observed). `partial_cmp` reports a NaN point equal to
  itself, while `==` says it differs, which breaks the contract of Rust's `PartialOrd`.
  `is_normalizable` ignores a NaN coordinate; upstream's `std::max` chain passes a NaN on or
  drops it depending on its position (read from the code, as the function is private).
  Nothing documents any of this. Signed zeros are equal under both `==` and the order, as in C++.
- **K9, the arithmetic depends on the build.** `r3::Vector`'s dot and cross products use
  fused multiply-add when the target has it, which only the `bench-native` tuning target
  enables. In 100,000 random pairs of unit points, the default build's `distance` was
  symmetric bit for bit in every pair and the FMA build's in all but 7,963. `point_cross`
  stayed antisymmetric in both. The proofs cover the default build.
- **K10, `point_cross` of a point with itself or its antipode.** `point_cross` ports Go's
  `PointCross`, but where the cross product is exactly zero (`p == q`, `p == −q`) it returns
  S2's perturbed `ortho(p)`, as C++'s `RobustCrossProd(a, a)` returns `S2::Ortho(a)`. Go
  returns the plain axis `Ortho` of `r3.Vector`: for `p = (1, 0, 0)`, Go gives `(0, −1, 0)`
  and the library `(0, −0.999986, 0.0053)`. For `p == −q`, C++ perturbs symbolically instead.
  The callers (the edge crosser, the edge distances) follow C++, so the library's choice is
  C++'s for `p == q`, but nothing documents the difference from Go.
- **K11, `point_cross`'s sign laws fail more widely than documented** (found by the planning
  probe, 2026-10-14). The documentation, copied from Go's `PointCross`, promises
  `f(q, p) == −f(p, q)` and the laws for `−p` and `−q` unless `p == q` or `p == −q`. But the
  `ortho` branch is taken whenever `(p + q) × (q − p)` rounds to the zero vector, which also
  happens for distinct parallel points and for points so small that the products underflow.
  For `p = (1, 0, 0)` and `q = (1 + 2ε, 0, 0)`, both of which pass `is_unit`, `f(q, p)` is
  `f(p, q)` rather than its negation: within an ulp in the library, exactly in Go (observed).
  C++'s `RobustCrossProd` perturbs parallel points symbolically and keeps the laws. Where the
  laws hold, they hold as values: the zeros' signs may differ, as they do for `ortho(−a)` and
  `−ortho(a)`.

### Deferred: `ChordAngle`

`ChordAngle` was this feature's second type until 2026-10-13, when the maintainer deferred it
while its validity question is open. It is the distance type of the query layer (28 source
files and the Python bindings). The pilot proved its bit-level core in 0.1–1.5 s per harness.
The survey found, in the library at 2026-10-13:

- **NaN, a silent deviation**: `from_length2(NaN)` and `plus_error(NaN)` return NaN, an
  invalid value; upstream returns the straight angle (`std::min(4.0, NaN)` is 4.0).
- **Every negative value is valid**, as in upstream (whose `is_negative` carries a "TODO:
  Consider stricter check here -- only allow Negative()"). `from_length2(−5)` is not the
  `NEGATIVE` sentinel and `predecessor(−5)` is −1. Minus infinity counts as infinite, where
  upstream's `is_infinity` tests `+∞` only.
- **Special values in the arithmetic**: `Add` and `Sub` check them in debug builds only, and
  `sin`, `sin2`, `cos` and `tan` not at all (`NEGATIVE.sin()` is NaN, `INFINITY.cos()` is
  −∞), with no documented panic.
- **`e5`, `e6` and `e7` of `INFINITY`** are `i32::MAX`. Upstream's are undefined behaviour in
  release builds (observed: −2147483648) and a check failure in debug builds: a candidate
  upstream finding, to be numbered when the type is picked up.
- **Deserialisation accepts invalid values** such as 5.0.

**The open question** is which values are valid:

- (A, recommended) keep upstream's rule, proving on the canonical values and pinning the
  others;
- (B) make the constructors map every negative to `NEGATIVE`, a documented deviation that
  changes 003's `Cap` round trip;
- (C) make only `NEGATIVE` valid among the negatives, so the `Cap` decoder would reject bytes
  C++ accepts.

The type is picked up once that is answered (FR-025).

*Resolved 2026-10-16: the maintainer chose (B), and spec 007
(`specs/007-formal-verify-chord-angle-angle/`) verifies `ChordAngle` with `Angle`.*

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for the u-v rectangle and the point (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope operation of
`r2::Rect` and `Point` completes without panicking, overflowing or hitting undefined
behaviour, for every input in its documented domain. Both are fixed-size values, so the proofs
cover whole domains, with no size bound.

**Why this priority**: Crash freedom is what the constitution already requires (Principle II),
and K1, K2 and K5 show that the domains and results are not yet documented. It is useful even
if no functional property is proven.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope operation as proven free of panics, overflow and undefined behaviour
over its stated domain, with no unproven obligations.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification command,
   **Then** it reports each in-scope operation by name as proven crash-free, states its
   domain, and exits successfully.
2. **Given** an operation with a documented precondition (for example "both points must be
   unit length"), **When** verification runs, **Then** the proof assumes exactly that
   precondition and no stronger one.
3. **Given** an internal consistency check that runs only in debug builds, **When**
   verification runs, **Then** the proof shows that it never fires within the domain.
4. **Given** an in-scope operation deliberately changed so that it can panic inside its
   domain, **When** verification runs, **Then** it fails and names the operation and an input.

---

### User Story 2 - Proven meaning: a product of proven intervals, and an ordered vector (Priority: P2)

As the maintainer, I want a catalogue of functional properties for each type, proven against
independent descriptions:

- a u-v rectangle is the product of two closed real intervals, built from 005's real-interval
  description, so that what 005 proved is reused, not repeated;
- a point is a triple of IEEE-754 values, ordered lexicographically, classified by upstream's
  unit-length and normalizability tests, scaled by exact powers of two, with the sign laws
  upstream documents for its orthogonal directions.

**Why this priority**: This extends the parity guarantee (Principle I) from sampled inputs to
every input. The point's order decides the predicates' symbolic perturbation, and its scaling
decides whether the robust cross product returns a usable direction.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven or as not proven with a written reason. Each proven property fails when a matching
defect is injected.

**Acceptance Scenarios**:

1. **Given** the property catalogue, **When** verification runs, **Then** every property
   marked proven is discharged with no remaining obligations.
2. **Given** any proven property, **When** a defect that violates it is introduced, **Then**
   verification fails for that property.
3. **Given** a u-v rectangle property, **When** a reviewer reads its statement, **Then** it is
   stated through 005's real-interval description, and the trusted base names each result of
   005 it relies on.
4. **Given** a point property about a value computed by upstream's arithmetic (the unit-length
   test, the scaling), **When** a reviewer reads it, **Then** it states the result bit for bit
   against a restatement of upstream's formula, checked against the C++ vectors, or states its
   tolerance and why.

---

### User Story 3 - Regression protection for future changes (Priority: P3)

As a contributor changing either type, or the real interval and vector code they build on, I
want the proofs to run automatically in CI, and to be told which property broke, with a
concrete counterexample.

**Why this priority**: Proofs that are not re-checked go stale. This protects Stories 1 and 2.

**Independent Test**: A change that breaks one proven property fails the automated check and
names it. The revert passes.

**Acceptance Scenarios**:

1. **Given** a change to either type, to `r1::Interval` or `r3::Vector`, or to the
   verification artifacts, **When** CI runs, **Then** these proofs run and their result
   appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the default
   quality gate is not slowed down or made to depend on verification toolchains.

---

### User Story 4 - Evaluation of the value types as a whole (Priority: P3)

As the maintainer, I want a short written evaluation:

- what reusing 005's description cost and saved for the u-v rectangle;
- how far exhaustive checking reached into the point's floating-point geometry, and what
  needs another technique;
- what was reachable, and what is trusted but not proven;
- the defects found;
- a closing assessment of the value types (004–006), and what next, `ChordAngle` included.

**Why this priority**: After three features on value types, the project needs one place that
says whether the approach was worth it and where to go next.

**Independent Test**: A maintainer who did not do the work reads the evaluation and can
answer "is `ChordAngle` next, and what must be decided first?", "type-level invariants
first?" and "what next?" without asking anyone.

**Acceptance Scenarios**:

1. **Given** the completed feature, **When** the evaluation is read, **Then** it contains a go
   or no-go for `ChordAngle`, an assessment of 004–006 together, and a ranked list of
   follow-ups.

---

### Edge Cases

- **Empty rectangles**: every pair of empty intervals is an empty rectangle, with many
  representations. Upstream's is `[1, 0] × [1, 0]`. A rectangle with exactly one empty
  interval is invalid (K1–K3).
- **Infinite and signed-zero bounds**: infinite bounds are valid in a u-v rectangle, as in its
  intervals (K4). `0.0` and `−0.0` compare equal.
- **NaN**: a NaN bound or coordinate has no meaning in upstream. It is outside the domain of
  every rectangle operation and every point operation's functional properties, unless an
  operation documents otherwise (K4, K8). The point's crash proofs include NaN.
- **The vertex index**: `vertex(k)` takes any `i32`, `i32::MIN` included, and reduces it
  modulo 4.
- **The zero vector**: `from_coords` maps every signed zero triple to `origin`;
  `ensure_normalizable` returns the zero vector unchanged; `normalize` returns it.
- **Tiny, huge and infinite coordinates**: subnormal coordinates, a largest component at
  2⁻²⁴², at 2⁻¹⁰²³ and below it (K5, K6), coordinates whose squared length overflows or
  underflows (K7), and infinite coordinates, where `normalize` computes `∞ · 0` and addition
  `∞ − ∞` (FR-004).
- **Antipodal and equal points**: `point_cross(p, p)` and `point_cross(p, −p)` take the
  `ortho` branch, and so do distinct parallel points and points whose cross product
  underflows (K11); the robust cross product of antipodal points takes the symbolic branch
  (K6).
- **Ties in the largest component**: `ortho` chooses its axis by the largest absolute
  component, and ties pick the later axis.
- **Debug-only checks**: the rectangle's constructors (K1).
- **Trigonometry**: `distance`, `stable_angle`, the approximate equalities and `rotate` go
  through `atan2`, `sin` and `cos`. Whether the checker models these is measured at planning
  (FR-003).
- **The build**: the default build computes dot and cross products without fused
  multiply-add; a build for a CPU with it computes others (K9).

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify the u-v rectangle `r2::Rect` and the point on the
  sphere `s2::Point` of the core library.
- **FR-002**: The verification artifacts MUST include an operation inventory for `r2::Rect`:
  its 30 inherent public operations and its 3 hand-written trait implementations (the default
  value, equality, text display), 33 entries. Every entry except text display is in scope.
- **FR-003**: The verification artifacts MUST include an operation inventory for `Point`: its
  20 inherent public functions, the 2 public free functions of its module (`ortho`,
  `rotate`), its 7 hand-written trait implementations (negation, addition, subtraction, the
  two conversions with `r3::Vector`, the ordering, text display) and its derived equality and
  default value, 31 entries.
  - **In scope: 24 entries** that compare, select, scale by powers of two, or compute with
    `sqrt` and products: `new`, `from_coords`, `origin`, `vector`, `x`, `y`, `z`, `is_unit`,
    `normalize`, `chord_angle`, `point_cross`, `reference_dir`, `cmp_point`,
    `is_normalizable`, `ensure_normalizable`, `ortho`, negation, addition, subtraction, the
    two conversions, the ordering, equality and the default value.
  - **Through trigonometry: 6 entries**, `distance`, `stable_angle`, `approx_eq`,
    `approx_eq_angle`, `approx_eq_with` and `rotate`. **Planning measures first** whether the
    checker models `atan2`, `sin` and `cos`. Where it does, they get crash proofs; where it
    does not, they are tool-limit exclusions with measurements. Their values are checked
    against the C++ vectors (FR-023) either way.
  - **Excluded: text display** (1 entry).

  The encoding and decoding of points were proven by 003, and the conversions from `LatLng`
  and from `geo-types` belong to those types; none is in this inventory.

**Guarantees**

- **FR-004**: For every in-scope operation, verification MUST establish that it completes
  without panic, arithmetic overflow or undefined behaviour, for every input in its documented
  domain:
  - an `r2::Rect` argument ranges over every valid rectangle with non-NaN bounds, infinities
    and every representation of the empty rectangle included. For the operations whose
    arithmetic meets `∞ − ∞` (centre, size, expansion, approximate equality), it ranges over
    the rectangles where that cannot happen; the infinite corners are documented (FR-018);
  - a `Point` argument ranges over every triple of `f64`, NaN and the infinities included,
    since the type has no validity rule, narrowed only by a documented precondition (unit
    length where an operation states it; a non-zero vector for `ensure_normalizable`). For
    the operations whose arithmetic can meet `∞ − ∞`, `0 · ∞` or an overflow into them
    (addition, subtraction, `from_coords`, `normalize`, the cross products, `ortho`, the
    distances, `rotate`), it ranges over the triples where that cannot happen; those corners
    are documented (FR-022). `from_coords` and `normalize` range over the triples whose
    squared length is finite: the checker reports their division by an infinite length as
    NaN, though IEEE-754 gives 0 there (research R0). Ordinary tests check the zero vector
    they return for the rest (K7);
  - a point, margin or error-bound argument of `r2::Rect` ranges over every value of its type,
    narrowed only by a documented precondition;
  - an `Angle` argument over every value, and an `i32` argument (`vertex`) over every value.

  A precondition counts as documented if the library's API documentation, a debug-build check
  or the upstream reference states it. An internal consistency check that runs only in debug
  builds MUST be proven never to fire within the domain. An undocumented precondition that a
  proof needs MUST be recorded as a finding and documented (FR-017, FR-022).
- **FR-005**: Each type's invariant MUST be stated formally:
  - **u-v rectangle validity**: both intervals empty or neither, with non-NaN bounds;
  - **point**: no validity rule; the unit-length convention is stated as upstream's
    unit-length test, and every operation that promises a unit result (`from_coords` and
    `normalize` within their documented range, `ortho`, `reference_dir`, `rotate`) is checked
    against it wherever the checker reaches.

  Verification MUST establish that every in-scope operation that returns a u-v rectangle
  returns a valid one whenever its inputs are in the domain.
- **FR-006**: The verification artifacts MUST include a property catalogue for each type,
  covering at least these areas:
  - **`r2::Rect`**:
    - **Validity and representation**: emptiness, the constructors (from two intervals, from
      two corners, from one point, from two points, from a centre and size), the bounds and
      the four vertices in upstream's order.
    - **Membership**: point and interior membership, each the product of the intervals'.
    - **Relations**: containment, interior containment, intersection and interior
      intersection.
    - **Set operations**: adding a point or a rectangle, union and intersection, as the
      smallest rectangles upstream documents.
    - **Measures**: the centre and the size.
    - **Expansion, projection and approximate equality.**
    - **The generic laws**: the equality laws, instances of 004's generic laws.
  - **`Point`**:
    - **Representation**: `new` and the two conversions keep the bits; the accessors return
      the coordinates; `origin` is upstream's point, and unit; the default is the zero vector.
    - **Construction**: `from_coords` maps every zero triple to `origin` and every other
      triple to its normalised vector; the result is unit length within the documented range
      (K7).
    - **Classification**: `is_unit` is upstream's test bit for bit; `is_normalizable` holds
      exactly when the largest absolute coordinate is at least 2⁻²⁴².
    - **Scaling**: `ensure_normalizable` returns a normalizable vector for every non-zero,
      non-NaN input, unchanged when it already is, and otherwise the input times a power of
      two, exactly, with the largest component in [1, 2).
    - **Order**: `cmp_point` is lexicographic; on points without NaN it is a total preorder
      whose ties are exactly equality (so signed zeros tie); the ordering agrees with it; the
      behaviour with NaN is pinned (K8).
    - **Arithmetic**: negation, addition and subtraction are the coordinate-wise IEEE-754
      operations, bit for bit; negation undoes itself.
    - **Orthogonal directions**: `ortho(−a)` is `−ortho(a)` as values (upstream's
      documented law; the zeros' signs may differ); `reference_dir` is `ortho`; neither
      `ortho` nor `point_cross` returns the zero vector for a unit point; `point_cross` has
      its three sign laws, as values, wherever its cross product is non-zero (K11).
    - **Distances**: the distance and chord angle from a point to itself are zero, both are
      symmetric bit for bit, the chord angle's squared length is within [0, 4], and the
      approximate equalities are reflexive, as far as the checker reaches (FR-003).
    - **The generic laws**: the equality laws, instances of 004's generic laws, and the order
      laws (a total preorder on a set of values), stated generically so that later types
      such as `ChordAngle` can instantiate them.

  Each entry MUST record a plain-language statement, its kind, the upstream source, the
  domain and a status: **proven** (the whole domain is covered), **proven within stated
  bounds**, or **not proven (reason)**.
- **FR-007**: Properties MUST be stated against reference descriptions written from upstream
  S2 semantics that do not restate the implementation's code:
  - for the u-v rectangle, the product of 005's real-interval description for each axis,
    valid when both are empty or neither is;
  - for the point, the coordinate triple, the lexicographic order of IEEE-754 values, and
    scaling by a power of two as a change of exponent.

  Where a property is "bit for bit upstream's formula" (the unit-length test, the scale's
  exponent), the restatement of the formula is an explicit part of the description, kept apart
  from the meaning and checked against the C++ vectors, as 004 did for the length and the
  expansion. A rectangle property MAY rely on a real-interval result proven by 005 instead of
  re-proving it; the trusted base records each such reliance.
- **FR-008**: Verification MUST be exhaustive and automated over every input in the domain,
  with exact bit-level IEEE-754 semantics, and MUST produce a concrete counterexample on
  failure. Floating-point properties hold exactly, except where upstream promises only an
  approximate result (unit length); there the property states its tolerance in ulps and the
  reason.
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
  observable behaviour, except for FR-017 to FR-022. It MUST NOT add runtime dependencies or
  slow the library down. Verification-only code MUST be excluded from normal builds.
- **FR-012**: A defect found by verification MUST be fixed as a separate, recorded change,
  with parity checked against upstream and a regression test that fails without the fix. The
  survey findings K1–K10 are the exception: they are handled inside this feature (FR-017 to
  FR-022, FR-015).

**Operability**

- **FR-013**: The proofs MUST run from the project's documented verification command, which
  reports pass or fail per operation and per property, with a documented way to run only this
  feature's proofs.
- **FR-014**: The proofs MUST run automatically in CI, through the same command, for changes
  to either type, to `r1::Interval`, to `r3::Vector`, to the robust cross product, or to the
  verification artifacts. They MUST stay out of the default quality gate. Proofs over the
  per-change budget MAY run weekly.
- **FR-015**: The trusted base statement MUST be extended, including:
  - the results of 005 that the u-v rectangle's proofs rely on;
  - the checker's model of IEEE-754 arithmetic, `sqrt` included, and of the trigonometric
    functions if FR-003's measurement finds them modelled;
  - that the proofs cover the default build, whose dot and cross products are not fused, and
    that a build with fused multiply-add computes different values (K9);
  - every restatement of upstream's arithmetic, and any split of a domain.

**Evaluation**

- **FR-016**: The feature MUST produce a written evaluation. It records the effort, the run
  time, what was reachable, the limitations and the defects found. It MUST also:
  - measure what reusing 005's description saved for the u-v rectangle;
  - say how far the checker reached into the point's floating-point geometry, and which
    properties need certified error bounds instead;
  - assess the value types of 004–006 together, against 001–003;
  - give a go or no-go for `ChordAngle`, with the decision it waits on (FR-025), and say
    whether type-level invariants (private fields with validating constructors) should come
    first for the value types;
  - give a ranked list of follow-ups.

**Known gaps: `r2::Rect`**

- **FR-017** (K1, K2): The constructors' preconditions MUST be documented, with the panic and
  what release builds do: `new` and `from_points` (debug-only validity check) and `project` on
  an empty rectangle (a panic in every build). `from_center_size` MUST check its precondition
  as upstream does, a debug-build check that the result is valid, and document it. This
  changes behaviour in debug builds only.
- **FR-018** (K4): The infinite and NaN corners MUST be documented on the operations they
  reach: the size and centre of a rectangle with an infinite side, expansion by an infinite
  margin, approximate equality at infinite bounds, and a NaN coordinate. They MUST say that
  upstream computes the same, or document the difference if the C++ vectors show one. This
  changes no behaviour.
- **FR-019** (K3): Deserialising an `r2::Rect` MUST reject a rectangle that is not valid. A
  valid rectangle MUST deserialise unchanged, and the serialised form MUST NOT change. The fix
  MUST be its own identifiable change, with a regression test that fails without it.

**Known gaps: `Point`**

- **FR-020** (K5): `ensure_normalizable` MUST scale every non-zero, non-NaN vector by a power
  of two so that its largest component lies in [1, 2), as its documentation and upstream's
  comment say, for every largest component down to 2⁻¹⁰⁷⁴. Where the result was already
  correct it MUST NOT change. Its rustdoc MUST state the domain. This fixes a library defect,
  as its own identifiable change, with a regression test that fails without it.
- **FR-021** (K6): The library MUST have one implementation of the scaling, FR-020's, used by
  both `Point` and the robust cross product. Then:
  - wherever upstream's result is finite, the robust cross product MUST equal upstream's bit
    for bit, which restores parity where the private copy scaled into [0.5, 1);
  - where upstream's overflows (a largest component below 2⁻¹⁰²³), the library MUST return the
    correctly scaled vector, a documented deviation, as for U-007, U-009 and U-010.

  This is its own identifiable change, with regression tests for both cases that fail without
  it.
- **FR-022** (K7, K8, K10, K11): The limits MUST be documented, with upstream's matching
  behaviour:
  - `from_coords` and `normalize` for coordinates whose squared length overflows, underflows
    or is subnormal (the zero vector, or a vector off unit length);
  - the infinite coordinates, and the coordinates large enough to overflow, for which the
    arithmetic of FR-004's second bullet produces NaN, with the precondition each such
    operation needs;
  - the order's behaviour with NaN (not transitive; `partial_cmp` reports equal where `==`
    does not), and `is_normalizable` on a NaN coordinate;
  - `point_cross` of a point with itself or its antipode (K10), as a deviation from Go's
    `PointCross` that follows C++'s choice for equal points;
  - `point_cross`'s sign laws with their true exception, a cross product that rounds to zero
    (K11), as Go's behaviour (U-013).

  This changes no behaviour.

**Reference vectors, upstream findings and the deferred type**

- **FR-023**: The feature MUST add committed C++ reference vectors, with differential tests:
  - every in-scope operation of the u-v rectangle over random rectangles and the special
    cases (empty, infinite, signed-zero and single-point sides, and every cell's u-v bound at
    a few levels);
  - every operation of the point that upstream has, the trigonometric ones included, over
    random unit points and the special cases (the zero vector, signed zeros, subnormal, tiny
    and huge coordinates, ties in the largest component, equal and antipodal pairs, NaN and
    the infinities), with the robust cross product's scaled results. Rows where the library
    deviates (FR-021) are marked as deviations, as the earlier vectors mark theirs.

  CI MUST NOT need a C++ toolchain to check them.
- **FR-024**: Undefined or wrong behaviour found in the upstream reference MUST be noted as a
  follow-up task before the affected proof is marked complete, with the upstream location, the
  inputs and the behaviour, and listed in the evaluation and in `UPSTREAM.md`. K6 (upstream's
  `RobustCrossProd` returning NaN for two unit points) is the first, as U-012. Go's
  `PointCross` breaking its documented sign laws for distinct parallel points (K11) is the
  second, as U-013.
- **FR-025**: The deferral of `ChordAngle` MUST stay recorded: the survey findings and the open
  question in this specification's Context, and a follow-up in the evaluation's ranked list
  that names the question to answer before the type is picked up.

### Key Entities

- **`r2::Rect`**: a verification target. Two closed real intervals, one per axis, both empty
  or neither.
- **`Point`**: a verification target. A triple of `f64` that by convention has unit length.
- **Reference descriptions**: the product of 005's real-interval description (FR-007); the
  point's coordinate triple, lexicographic order and power-of-two scaling, with the
  restatements of upstream's formulas kept apart.
- **Generic order laws**: a total preorder on a set of values, stated once, as 004's equality
  laws are.
- **The scaling**: one implementation, shared by `Point` and the robust cross product
  (FR-021).
- **Operation inventories, property catalogues, trusted base and evaluation**: as in 001–005.
- **The deferred type note**: `ChordAngle`'s findings and open question (FR-025).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope operation is proven free of panics, overflow and undefined
  behaviour over its whole documented domain, with no size bound.
- **SC-002**:
  - All 32 in-scope `r2::Rect` entries have a crash proof.
  - All 24 in-scope `Point` entries have one. The 6 trigonometric entries have one, or are
    recorded as tool limits with measurements.
  - Every exclusion has a written reason.
- **SC-003**: Each catalogue covers every area in FR-006. At least 90% of the entries whose
  operations are all in scope are proven, over the whole domain or within stated bounds.
  Entries that compare two normalisations are not counted: the checker's `sqrt` does not fix
  its result, so it cannot state them (research R0), and the C++ vectors and property tests
  check them. *Amended 2026-10-14, after implementation, by the maintainer: this leaves out
  P-PTVAL-02 and P-PTORT-01.*
- **SC-004**: 100% of proven properties have recorded non-vacuity evidence, and 100% of
  crash-freedom proofs have satisfied reachability checks.
- **SC-005**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine, a hosted Linux runner with 4 virtual CPUs and 16 GB.
- **SC-006**: Apart from FR-017 to FR-022, the public API and behaviour of both types are
  unchanged, and the quality gate passes before and after.
- **SC-007**: A maintainer who did not take part can answer from the evaluation alone: "is
  `ChordAngle` next, and what must be decided first?", "type-level invariants first?" and
  "what next?".
- **SC-008**: Every operation the proofs show can panic documents when it panics.
- **SC-009**: After the feature, deserialising an invalid u-v rectangle fails, every valid one
  deserialises unchanged, and each regression test fails before its fix and passes after it.
- **SC-010**: `ensure_normalizable` returns a normalizable vector for every non-zero, non-NaN
  input (proven), and the robust cross product returns a finite, non-zero vector for the
  antipodal unit points of K6 (tested against the C++ vectors, with the deviation marked).
- **SC-011**: The differential tests against the new C++ reference vectors pass for every
  vector.
- **SC-012**: Every catalogue entry appears exactly once in the index of properties by kind,
  and the consistency check passes.
- **SC-013**: Every undefined or wrong upstream behaviour met has a follow-up task and an
  entry in the evaluation.
- **SC-014**: The evaluation states, for the u-v rectangle, the share of its properties that
  rely on 005's results and the time saved against proving them again.

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Users benefit through stronger
  guarantees about cell bounds, the order the predicates rely on and the robust cross
  product.
- **Builds on 001–005.** The feature reuses:
  - the verification setup, the memory-guarded runner, the mutant campaign, CI and the weekly
    tier;
  - 004's generic equality laws and generators, and its practice of restating upstream's
    arithmetic and checking the restatement against the C++ vectors (004 FR-020, FR-023);
  - 005's real-interval description, unchanged.

  004's and 005's open runs do not block writing this feature. Verification runs from all
  three go one at a time.
- **Lessons carried over**:
  - the checker mis-models ordered comparisons of booleans, so descriptions compare booleans
    only for equality;
  - relations between intervals are those of real sets, not of float points;
  - properties through approximate equality of two intervals were over the per-change budget
    in 004 and 005; the u-v rectangle's are expected to need the weekly tier too;
  - weekly members can find real defects (U-011), so they are run, not only written;
  - a harness may compute with float products freely, but asserting on products of symbolic
    values is slow: the bit-for-bit statements compare the library's expression with an
    identical restatement, which the checker shares. The point's unit-length properties are
    the pilot's slowest (1802 s) and are expected to need bounds or the weekly tier.
- **Assurance level**: exhaustive model checking of the library's own code, as in 001–005.
- **Budget**: the pilot measured both types within the per-harness budget (4 GB, 12 minutes),
  except the u-v rectangle's expansion with projection (545 s) and the point's
  classification (1802 s). Planning re-measures on the current tree, and measures the
  trigonometric functions (FR-003).
- **Reference semantics**: upstream C++ at v0.14.0-40, with Go and Java secondary. Where the
  library deviates and documents it, the documented behaviour is verified.
- **Parity first**: where upstream and the library agree on a limit (K7, K8), or the library
  follows C++ where Go differs (K10), the library keeps its behaviour and documents it, as 005
  did for the real interval's infinite corners. Where upstream's result is wrong for valid
  input (K6), the library returns the correct value as a documented deviation, as for U-007,
  U-009 and U-010. Where only the library is wrong (K5, and K6's factor of two), it is fixed
  to upstream's result.
- **The deserialisation fix (FR-019)** follows the decisions for `CellUnion`, `Cell` and
  `s2::Rect`: reject what is not valid, keep the format. A point has no validity rule, so its
  deserialisation is unchanged, as upstream's decoding is.
- **Out of scope**:
  - making the fields private (type-level invariants, a public API change for its own
    feature). Both types' fields are public;
  - implementing a total `Ord` for `Point`, or changing its `PartialOrd` with NaN (K8 is
    documented, not changed);
  - the accuracy of the point's geometry (distances, rotation, the robust cross product's
    direction), which needs certified error bounds;
  - the build with fused multiply-add (K9);
  - `ChordAngle` (deferred, FR-025), `r2::Point` arithmetic and `Angle`, except as arguments;
  - validating input in the language bindings, which inherit the documentation and the fixes.
- **Survey findings**: K1–K10 were confirmed with concrete probes on 2026-10-13, in release
  builds; K2, K6, K7 and K8 against upstream C++ in a Release build, and K10 against Go. The
  `ChordAngle` findings in the Context were confirmed the same day.
- **Existing tests stay**: the proofs complement the existing unit, property, differential
  and generic trait tests. No test is removed.
