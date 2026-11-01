# Feature Specification: `Cell`'s Bounding Laws, and the Region Adaptors

**Feature Branch**: `tb/012-frm/1`

**Created**: 2026-10-29

**Status**: Implemented 2026-10-31. Scope: the first two items of spec 011's ranked list.
Clarified 2026-10-30 by the maintainer: U-017 is recorded and upstream's `Cell` bounds are kept;
the bounding laws are stated for the cell's own points, its `contains_point` margin a documented
exception (FR-016). Every success criterion is met on the workstation; SC-006 on a hosted runner
needs a push (tasks T017). `Cell`'s bounding laws for its own points are not proven (tool limit,
`atan2`'s values), as SC-004 allows. Evaluation: the spec-012 part of
`verify/values/EVALUATION.md`.

**Input**: User description: "next features to verify"

## Context

### Where the earlier features left off

- **001–003** proved `CellId`, `CellUnion` within size bounds, and the decoding layer.
- **004–010** proved the value types and the first `Region` laws (`Cell`, `Cap`, `Rect`).
- **011** proved `PointRegion` (five of its six `Region` laws) and `CellUnion`'s floating-point
  methods, and added two generic laws (the bounding rectangle and the covering contain every
  contained point). `CellUnion`'s bounding laws were left waiting on `Cell`'s own. Nothing is
  pushed.
- **The list** (spec 011's evaluation, "Recommendations"): "1. **`Cell`'s bounding laws** (its cap
  and rectangle bounds contain the points whose leaf it holds, and its own covering): with them,
  `CellUnion`'s P-UREG-04 and -05 follow by composition … 2. **The region adaptors**
  (`RegionUnion`, `RegionIntersection`): their laws compose from their members', now that five
  implementors have instances."

### Why the two together

Both close the `Region` laws by composition. `Cell`'s bounding laws are the missing premise of
`CellUnion`'s (a union's bounds contain each cell's bound, spec 011's P-UBND-01 and -02). The
adaptors' laws are *preservation* statements: if every member satisfies a law, so does their
union and their intersection. Together they finish the `Region` laws for every implementor whose
parts are verified, except the shape-backed ones (`Loop`, `Polygon`, `Polyline`, the shape-index
regions).

### Survey findings (probed 2026-10-29)

The probe is in `probe/`: 20,000 cells with 60 adversarial points each, and 10,000 adaptors over
mixed members, on upstream C++ (v0.14.0-40, Release, x86-64) and on the library, compared line by
line (`probe/README.md`, `probe/compare.txt`). Every `Cell` bound and every point answer agrees bit
for bit; the adaptors agree but for their cap bound's radius, which goes through
`Rect::cap_bound` and its documented widening (spec 010 FR-019, U-016). It finds:

- **K1 (U-017), `Cell`'s bounds miss points the cell contains, upstream too.**
  `Cell::contains_point` accepts points up to `(5/3)·ε` outside the cell in u-v, so that a point's
  own leaf cell always contains it (upstream's `S2Cell::Contains`, issue #463). The cap and
  rectangle bounds cover only the cell itself. Of 1,008,724 contained points, 118,093 lie outside
  the cap bound and 36,315 outside the rectangle bound, every one in that margin; the 240,000
  points at the cell's own corners, edge midpoints and interior all lie in both. The `Region`
  contract's "the bounds contain the region" fails for `Cell` at its margin.
- **K2, `RegionIntersection::region(i)` panics past the end, undocumented.** Upstream indexes
  without a check (undefined behaviour past the end); the library panics, which is safe but not
  documented.
- **No other difference.** `RegionUnion` and `RegionIntersection` are upstream's line for line: a
  rectangle bound folded from the empty (union) or full (intersection) rectangle, the cap bound
  that rectangle's cap bound, the covering the cap's, and the cell and point tests as `any` and
  `all` of the members'. The empty intersection is the whole sphere, the empty union nothing.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `Cell`'s bounding laws (Priority: P1)

As the s2rst maintainer, I want it proven, or recorded with a reason, that a cell's bounding cap
and bounding rectangle contain every point of the cell, so that the `Region` contract holds for
`Cell` as far as upstream's bounds allow, and `CellUnion`'s bounding laws follow.

**Why this priority**: It is the list's first item, and `CellUnion`'s two bounding laws wait on
it.

**Independent Test**: Run the verification command. Each law instance for `Cell` is reported by
name as proven, or not proven with a written reason. The margin exception is recorded with the
probe's counterexamples.

**Acceptance Scenarios**:

1. **Given** a valid cell and a point of its own u-v bound, **When** verification runs, **Then**
   the cell's bounding cap and its bounding rectangle each contain the point, or the instance is
   recorded as not proven with the reason and checked by the probe's points and the C++ rows.
2. **Given** a point within `contains_point`'s margin but outside the cell's u-v bound, **When** a
   reviewer reads the records, **Then** they find U-017, its inputs, and the documented exception,
   with a fixed test that pins one counterexample.
3. **Given** `CellUnion`'s bounding laws (spec 011's P-UREG-04, -05), **When** `Cell`'s laws are
   settled, **Then** `CellUnion`'s are re-stated by composition (for its cells' own points) or
   keep their reason.

---

### User Story 2 - The region adaptors' laws (Priority: P1)

As the maintainer, I want it proven that `RegionUnion` and `RegionIntersection` preserve every
`Region` law their members satisfy, so that a union or intersection of verified regions is
verified.

**Why this priority**: The adaptors compose every other implementor; a preservation proof covers
all of their combinations at once.

**Independent Test**: Run the verification command. For each law and each adaptor, a preservation
statement is reported as proven (for members that satisfy the law, whatever they are), or not
proven with a reason. Injecting a defect into either adaptor's methods fails a matching statement.

**Acceptance Scenarios**:

1. **Given** members that satisfy "containment implies intersection", **When** verification runs,
   **Then** their union and their intersection satisfy it too.
2. **Given** members that satisfy the vertex law, or the bounding-rectangle law, **When**
   verification runs, **Then** their union and their intersection satisfy it too.
3. **Given** the bounding-cap and covering laws, **When** verification runs, **Then** each is
   proven to follow from the adaptor's rectangle bound and `Rect`'s own law, or recorded with the
   reason (the adaptors' cap bound is their rectangle's).

---

### User Story 3 - Crash freedom of the adaptors (Priority: P2)

As the maintainer, I want every operation of `RegionUnion` and `RegionIntersection` proven free of
panics, overflow and undefined behaviour, given members that are, with `region(i)`'s panic
documented and proven exactly where it happens.

**Why this priority**: The constitution requires crash freedom (Principle II); the adaptors hold
any regions, so their own code must not add a crash.

**Independent Test**: The verification command reports every adaptor operation crash-free, for
members whose operations do not panic, and `region(i)` panicking exactly for `i` past the end.

**Acceptance Scenarios**:

1. **Given** an adaptor of any number of well-behaved members (within stated bounds), **When**
   verification runs, **Then** every operation completes.
2. **Given** `RegionIntersection::region(i)` with `i` past the end, **When** verification runs,
   **Then** it panics, as its documentation now says (K2).

---

### User Story 4 - Records, CI and evaluation (Priority: P3)

As a contributor, I want the new proofs in CI, and as the maintainer a short evaluation of what is
proven and what comes next.

**Why this priority**: Proofs that are not re-checked go stale.

**Independent Test**: A change that breaks a proven property fails the automated check; the
evaluation answers "which `Region` laws hold for `Cell` and the adaptors, and what next?"

**Acceptance Scenarios**:

1. **Given** a change to `Cell`'s bounds, the adaptors or the laws, **When** CI runs, **Then**
   these proofs run.
2. **Given** the completed feature, **When** the evaluation is read, **Then** it answers which laws
   hold, which are limited and why, and ranks the follow-ups.

---

### Edge Cases

- **The margin.** A point beyond the cell's u-v bound but within `(5/3)·ε` of it: contained, and
  possibly outside the bounds (K1). Points exactly on the bound, at a corner or an edge, lie in
  both bounds (the probe's 160,000).
- **Face cells and leaf cells.** Level-0 cells have special rectangle bounds (spec 011's K1) and
  the largest caps; leaf cells the smallest, where the margin is largest relative to the cell.
- **Empty adaptors.** The empty union contains nothing (empty rectangle and cap); the empty
  intersection is the whole sphere (full rectangle and cap). An adaptor of one member is that
  member, with the adaptor's own bounds.
- **Members that break a law.** A `Cell` member breaks the vertex law and, at its margin, the
  bounding laws; `CellUnion` breaks the vertex law. Preservation says nothing about a law a member
  breaks; the records say which members a preserved law then covers.
- **Nested adaptors.** An adaptor can hold adaptors; preservation applies to them, since they
  are members like any other.
- **Unmodelled functions.** `Cell`'s bounds go through `atan2` (the rectangle) and normalisation
  (`sqrt`); a statement that needs their values beyond their contracts is recorded as not proven,
  with the probe and the C++ rows as its evidence.

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST instantiate the bounding-cap and bounding-rectangle laws for `Cell`,
  for the points of the cell's own u-v bound (FR-016), and record the covering law's instance
  (its closed form holds by construction: the covering is the cell itself).
- **FR-002**: The feature MUST verify `RegionUnion` and `RegionIntersection`: an operation inventory
  (their constructors, accessors, iterators, `Default` and the six `Region` methods; `Debug`
  excluded), crash freedom, a property catalogue, and the `Region` laws by preservation.
- **FR-003**: Statements that go through `atan2`, `asin`, `sqrt`, `sin` or `cos` MUST use the
  existing tested contracts and stubs (status **proven under a stated contract**); a new contract
  or stub MUST be tested against the real function and recorded in the trusted base.

**Guarantees**

- **FR-004**: For every in-scope adaptor operation, verification MUST establish crash freedom for
  every input in its documented domain: any members whose own operations do not panic on the
  arguments, adaptors of up to a stated number of members, and every cell and point.
- **FR-005**: For each adaptor and each of the six generic `Region` laws, the catalogue MUST hold a
  preservation entry: if every member satisfies the law (for the cell and points at hand), the
  adaptor does; proven, proven by composition (with the argument), or not proven with a reason.
- **FR-006**: The catalogue MUST also hold the adaptors' meaning: the rectangle bound is the union
  (intersection) of the members' rectangle bounds; the cap bound is that rectangle's cap bound;
  the covering is the cap's; the cell and point tests are `any` (`all`) of the members'; the empty
  adaptors' answers.
- **FR-007**: Properties MUST be stated against a description written from upstream's semantics:
  a union contains what some member contains, an intersection what every member contains; a
  member is any region (an abstract region in the proofs), so a statement holds for every
  implementor.
- **FR-008**: Verification MUST be exhaustive and automated over its domain, with exact IEEE-754
  semantics, and MUST produce a counterexample on failure.
- **FR-009**: Every proven property MUST be shown non-vacuous by an injected defect, recorded with
  its outcome; crash proofs MUST show their domains non-empty.

**`Cell`'s laws and `CellUnion`'s**

- **FR-010**: `Cell`'s bounding-cap and bounding-rectangle laws MUST be attempted for every valid
  cell and every point of its u-v bound, and recorded as proven or not proven with the reason and
  the measurement.
- **FR-011**: `CellUnion`'s bounding laws (spec 011's P-UREG-04, -05) MUST be revisited: re-stated by
  composition from P-UBND-01, -02 and `Cell`'s laws for the points of its cells' own u-v bounds, or
  keep their reason.
- **FR-012**: The quickcheck twins MUST draw points at a cell's corners, on its edges and in its
  interior (the bounding laws) and adaptors of mixed members, including nested adaptors.

**Non-intrusiveness and operability**

- **FR-013**: The feature MUST NOT change the public API or the behaviour of `Cell`, `CellUnion`,
  `RegionUnion` or `RegionIntersection`, apart from documentation (FR-016, FR-017). No runtime
  dependency; verification-only code excluded from normal builds.
- **FR-014**: A defect found by verification MUST be fixed as a separate, recorded change with a
  regression test that fails without the fix, after the maintainer's go-ahead if it is not in this
  spec.
- **FR-015**: The proofs MUST run from the documented verification command with a way to run only
  this feature's, in CI for changes to the files they cover, outside the default quality gate;
  proofs over the per-change budget run weekly.

**Known gaps (survey findings)**

- **FR-016** (K1, U-017): *Clarified 2026-10-30: keep upstream's bounds.* `Cell::contains_point`,
  `cap_bound` and `rect_bound` MUST document that the bounds contain the cell's own points but not
  every point of `contains_point`'s margin, as upstream; `CORE_TRAITS.md`'s bounding invariants
  MUST name `Cell`'s margin as an exception; U-017 MUST be recorded in `UPSTREAM.md` with the
  probe's inputs, and a fixed test MUST pin one counterexample.
- **FR-017** (K2): `RegionIntersection::region(i)` MUST document its panic for `i` past the end,
  with a test.

**Reference vectors, upstream findings and records**

- **FR-018**: The feature MUST add committed C++ reference vectors, with differential tests: for
  `S2Cell`, the bounds and `Contains(p)` at the probe's adversarial points; for `S2RegionUnion`
  and `S2RegionIntersection`, the bounds, covering and tests over mixed members. The cap radii of
  the adaptors are the documented deviation of spec 010's FR-019, checked as its rule says.
- **FR-019**: U-017 MUST have a follow-up task, an entry in `UPSTREAM.md` and in the evaluation;
  reporting it upstream needs the maintainer's approval.
- **FR-020**: As the constitution requires, the records the change affects MUST be updated in the
  same change: inventories, catalogues, the index by kind and its render, the trusted base,
  `MUTANTS.md`, `VERIFIED_TYPES.md`, `CORE_TRAITS.md`, `COVERAGE.md`, and the evaluation, which
  ranks the follow-ups.

### Key Entities

- **`Cell`'s bounds**: the cap and rectangle bounds, judged against the cell's own points; the
  `contains_point` margin a documented exception (U-017).
- **`RegionUnion`, `RegionIntersection`**: the adaptors, verified over abstract member regions.
- **The abstract region**: a member whose answers are any values consistent with the law at
  hand; a preservation proof over it holds for every implementor.
- **Generic `Region` laws**: the six of `laws::region` (specs 004, 008, 011).
- **Inventory, catalogue, trusted base, evaluation**: as in 001–011.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope adaptor operation is proven crash-free over its documented domain
  (within a stated member count), and `region(i)`'s panic proven exact.
- **SC-002**: Every adaptor entry has a crash proof or a written exclusion; every new contract
  passes its tests against the real function.
- **SC-003**: At least 90% of the counted catalogue entries (the adaptors' meaning and preservation
  entries whose operations are in scope and expressible) are proven, within bounds, by
  composition, or under a stated contract.
- **SC-004**: For each adaptor, "containment implies intersection", the vertex law and the
  bounding-rectangle law are preserved, proven; the bounding-cap and covering laws are proven by
  composition or recorded with a reason. For `Cell`, the bounding-cap and bounding-rectangle laws
  for its own points are proven or recorded with a reason, and the margin exception is recorded
  with its counterexample.
- **SC-005**: 100% of proven properties have recorded non-vacuity evidence, and 100% of crash
  proofs satisfied reachability checks.
- **SC-006**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine (a hosted Linux runner, 4 vCPUs, 16 GB).
- **SC-007**: The documentation changes of FR-016 and FR-017 each have a test, and the probe's
  library-versus-upstream differences are exactly spec 010's documented cap radii.
- **SC-008**: The differential tests against the new C++ vectors pass on every row, the deviation
  rows being exactly the documented ones.
- **SC-009**: The public API and behaviour are unchanged, and the full quality gate passes.
- **SC-010**: Every operation the proofs show can panic documents when it panics.
- **SC-011**: Every catalogue entry appears once in the index of properties by kind, and the
  consistency check passes.
- **SC-012**: U-017 has a follow-up task, an entry in the evaluation and in `UPSTREAM.md`.
- **SC-013**: From the evaluation alone, a maintainer can answer "which `Region` laws hold for
  `Cell` and the adaptors, and what comes next?"

## Assumptions

- **Scope**: the first two items of spec 011's list (`Cell`'s bounding laws; the region adaptors),
  read from the user's "next features"; the shape-backed regions are out of scope.
- **Parity first** (Principle I): U-017 is upstream's own; the maintainer chose to keep upstream's
  bounds and document the margin (2026-10-30), rather than widen them as spec 010 did for `Rect`
  (whose bound missed points of the rectangle itself, not of a margin).
- **Builds on 001–011**: the suite, the runner, the stubs and contracts (008, 010, 011), the
  abstract u-v bound and cell, the laws and their twins, 002's union domains.
- **Lessons carried over**: products recomputed in a statement time out (lemma or uninterpreted
  stubs); `sqrt` and `powi` are not functions to the checker; the NaN check reports `∞ · 0` before
  a debug panic; per-change parts are measured as a whole and split if over the budget.
- **Assurance level**: exhaustive model checking of the library's own code, with members as
  abstract regions and the unmodelled functions under their tested contracts.
- **Reference semantics**: upstream C++ at v0.14.0-40, Release, x86-64.
- **Out of scope**: `Cell`'s other floating-point methods (its distances, proven or limited in the
  coverage review); widening any bound; `Loop`, `Polygon`, `Polyline` and the shape-index regions.
- **Existing tests stay**; tests are added, none removed.
