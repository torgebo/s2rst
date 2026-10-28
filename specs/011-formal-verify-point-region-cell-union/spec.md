# Feature Specification: Formal Verification of `PointRegion`, and `CellUnion`'s Floating-Point Methods

**Feature Branch**: `tb/011-frm/1`

**Created**: 2026-10-28

**Status**: Draft. Scope chosen by the maintainer on 2026-10-28: `PointRegion` together with
`CellUnion`'s floating-point methods (the first item of spec 010's ranked list).

**Input**: User description: "Continue with formally verifying the next type on the list"

## Context

### Where the earlier features left off

- **001–003** proved `CellId`, `CellUnion` within size bounds, and the decoding layer. 002
  excluded `CellUnion`'s nine floating-point entries (`float-geometry`).
- **004–008** proved nine value types, with function contracts and lemma stubs since 008; the
  coverage review of 2026-10-26 proved most of the floating-point entries 004–007 had excluded.
- **009** made `Level`, `Face`, `LevelMod` and `CellVertex` bounded types.
- **010** proved `LatLng` and two of `s2::Rect`'s four `Region` laws. Nothing is pushed.
- **The list**: 010's evaluation ranks first "**`PointRegion`** next for the `Region` laws (it
  is a cap), then **`CellUnion`'s float bounds**; and `Cell`'s remaining float methods"
  (`verify/values/EVALUATION.md`, "Recommendations (FR-016) (spec 010)").

### Why `PointRegion`, together with `CellUnion`'s floating-point methods

`PointRegion` is the next type: a single point as a `Region`. It is small (two functions and
the six `Region` methods), and each method is one call into a type already proven: `Cap`,
`LatLng`, `Rect`, `Cell`. `CellUnion` is the `Region` implementor whose integer half is proven
(002) and whose floating-point half is not. Together they finish the `Region` laws for every
implementor whose parts are proven, and they share the work:

1. **Both bound through `Cap::cell_union_bound`.** Their covering bounds are the covering of
   their bounding cap, so the covering laws of both rest on the same function, which spec 008
   measured as a tool limit.
2. **Both reach the cell's geometry through `Cell`.** `PointRegion::intersects_cell` is
   `Cell::contains_point`; `CellUnion`'s bounds union `Cell::cap_bound` and `Cell::rect_bound`
   over its cells. The abstract u-v bound and the `Cell::rect_bound` contract of 010 apply.
3. **Two `Region` invariants have no generic law yet.** `CORE_TRAITS.md` states "bounding rect
   contains contained points" and "the cell-union cover covers every contained point", but
   `laws.rs` states neither. These two regions are where they are cheapest to state first: a
   point's rectangle is one `LatLng`, and a union's point test is its integer cell test.

### Survey findings (probed 2026-10-28)

The probe is in `probe/`: the same cases on upstream C++ (v0.14.0-40, Release, x86-64) and on
the library, compared line by line (`probe/README.md`, `probe/compare.txt`). It agrees bit for
bit on `S2PointRegion`'s three bounds and two cell tests (49,022 cases, special vectors
included), on `S2CellUnion`'s cap bound, covering, areas and `Expand` (33,000 cases), and on
every law count. It finds:

- **K1, `Cell::rect_bound` of the two polar face cells is not upstream's.** The library writes
  the latitude of a polar face's corners as `0.615479708670387`, Go's 15-digit literal, which
  is 2 ulps below upstream's computed `asin(sqrt(1/3))`. The bound of face cells 2 and 5 is 2
  ulps wider than upstream's, conservative but different, and every `CellUnion::rect_bound` of a
  union with one of those faces inherits it (3 of the probe's 10,000 unions). A library defect in
  spec 004's type, found through this feature's.
- **K2, the empty union's area is −0.** `CellUnion::approx_area` and `exact_area` of the empty
  union return −0, where upstream returns +0: the library sums with a fold whose starting value
  is −0. A parity defect (the sign of zero, as 008's D-009).
- **K3, `PointRegion` takes any point, undocumented.** Upstream's constructor requires a unit
  point (a debug-build check), and upstream's decoder rejects a point that is not unit length.
  The library's `new` neither checks nor documents the precondition, and its derived
  deserialiser accepts any three numbers, where `Cap`'s rejects a centre that is not unit length
  since 008 (K7 there).
- **K4, debug-build panics that the documentation does not state.** Each is a debug-build
  check that upstream makes too (a `DCHECK`), so release builds agree with upstream's; the
  library's documentation omits them. Found by the probe and by the planning probe's proofs
  (research R0):
  - `expand_by_radius` on a NaN radius (the level lookup's check);
  - `PointRegion::rect_bound` for a point with a NaN `x` or `y`, through `Rect::from_lat_lng`
    and `s1::Interval::from_point`, which check that the longitude is in range;
  - `PointRegion::intersects_cell`, through `Cell::contains_point`, for a point whose
    coordinate on the cell's face axis is NaN, or is on the face's side while another
    coordinate is infinite (the face projection's check);
  - `CellId::from_point` (so `PointRegion::cell_union_bound` and `CellUnion::contains_point`)
    for a point with two or more infinite coordinates: its documentation names the zero vector
    and NaN coordinates only (112 of the 1,331 special triples the planning probe tried
    panic without a NaN or a zero vector).
- **K5, upstream's `S2PointRegion` codec is missing** (not a defect of existing code): a
  25-byte encoding (a version byte and three doubles), whose decoder rejects a point that is not
  unit length.
- **Laws, not defects:**
  - For `PointRegion`, every law holds on the probe's cases: the cap and the rectangle bound
    contain the point, the covering contains it (2,000 of 2,000), and `Cell::contains_point`
    accepts each of a cell's own vertices (80,000 of 80,000), so the vertex law holds.
  - For `CellUnion`, the bounding cap and the bounding rectangle contain each of 262,154
    contained points, and the covering contains every cell of the union, in both.
  - **The vertex law does not hold for `CellUnion`**, in both: of 39,636 edge neighbours a
    union does not intersect, the union contains 20,573 of their vertices. Its cell test is by
    id and its point test by leaf cell, so a shared vertex can belong to the union while the
    neighbour does not; `laws.rs` already records the same for `Cell`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope operation of
`PointRegion`, and every floating-point operation of `CellUnion`, completes without panicking,
overflowing or hitting undefined behaviour, for every input in its documented domain.

**Why this priority**: The constitution requires crash freedom (Principle II). `PointRegion`
takes any point, so its domain is every triple of numbers unless a precondition is documented
(K3); `CellUnion`'s floating-point entries are the last of its 42 inventory rows without a crash
proof or a measured limit.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope operation as proven free of panics, overflow and undefined behaviour
over its stated domain (within stated size bounds for unions), with no unproven obligations,
or as a tool limit with a measurement.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification command,
   **Then** it reports each in-scope operation by name as proven crash-free or as a measured
   tool limit, states its domain, and exits successfully.
2. **Given** an operation with a documented panic (the debug-build panics of K4, after
   FR-019; the bounds on an id that has no `Cell`), **When** verification runs,
   **Then** a proof shows the panic happens exactly where documented, and a separate proof
   shows the operation returns everywhere else in its domain.
3. **Given** an in-scope operation deliberately changed so that it can panic inside its
   domain, **When** verification runs, **Then** it fails and names the operation and an input.

---

### User Story 2 - The `Region` laws for `PointRegion` (Priority: P1)

As the maintainer, I want every generic `Region` law proven for `PointRegion`, so that it counts
as a verified `Region` implementor.

**Why this priority**: It is the list's first item, and every law should hold (the survey found
no counterexample), so this is where the laws can be proven in full.

**Independent Test**: Run the verification command. Each law instance for `PointRegion` is
reported by name as proven (over the whole domain, under a stated contract or by composition),
or as not proven with a written reason. Injecting a defect into any of its six `Region` methods
fails the matching instance.

**Acceptance Scenarios**:

1. **Given** any unit point and any valid cell, **When** verification runs, **Then** containing
   the cell implies intersecting it, and a cell the region does not intersect has no vertex the
   region contains.
2. **Given** any unit point, **When** verification runs, **Then** the region's bounding cap,
   its bounding rectangle and its covering each contain the point.
3. **Given** a property that needs the value of a function the checker does not model exactly
   (`atan2` in the rectangle, `asin` in the covering's level), **When** a reviewer reads it,
   **Then** it states what holds for every value the function's contract allows, and leaves the
   value itself to the C++ vectors.

---

### User Story 3 - `CellUnion`'s floating-point methods and its `Region` laws (Priority: P2)

As the maintainer, I want `CellUnion`'s bounds, point test, areas and radius expansion verified,
and the generic `Region` laws instantiated for it, so that `CellUnion` is verified as a `Region`
and not only as a set of ids.

**Why this priority**: The coverer and the region adaptors rely on its bounds; 002 left them
out as floating-point geometry.

**Independent Test**: Run the verification command. Each in-scope entry and law instance is
reported by name as proven (within stated size bounds), or as not proven with a written reason.

**Acceptance Scenarios**:

1. **Given** a valid union within the stated size bounds, **When** verification runs, **Then**
   its point test is its cell test on the point's leaf cell, its rectangle bound is the union of
   its cells' rectangle bounds, and its average-based area is the leaf count times the leaf
   area.
2. **Given** the bounding cap, bounding rectangle and covering laws, **When** verification runs,
   **Then** each is proven within bounds or recorded with its reason and checked by the C++
   vectors and the quickcheck twins.
3. **Given** the vertex law, **When** verification runs, **Then** it is recorded as not a law of
   `CellUnion`, with the probe's counterexample as a fixed test, as for `Cell`.

---

### User Story 4 - Parity restored where the survey found defects (Priority: P2)

As a library user, I want the polar face cells' rectangle bounds and the empty union's areas to
be upstream's (K1, K2), and `PointRegion`'s and `expand_by_radius`'s preconditions documented
(K3, K4), each fixed as its own change with a regression test.

**Why this priority**: These are this feature's findings, fixed as the earlier features fixed
theirs.

**Independent Test**: Each probe case becomes a regression test that fails before its fix and
passes after. The new C++ reference vectors (FR-020) match on every row.

**Acceptance Scenarios**:

1. **Given** face cells 2 and 5, **When** their rectangle bounds are computed, **Then** each is
   upstream's bit for bit, and so is the rectangle bound of every union of the probe.
2. **Given** the empty union, **When** its approximate and exact areas are computed, **Then**
   each is +0.
3. **Given** a serialised `PointRegion` whose point is not unit length, **When** it is
   deserialised, **Then** it is rejected, as upstream's decoder and `Cap`'s deserialiser reject
   theirs.

---

### User Story 5 - Regression protection and evaluation (Priority: P3)

As a contributor changing `PointRegion`, `CellUnion`'s floating-point methods or the laws, I
want the proofs to run automatically in CI and name the property that broke. As the maintainer,
I want a short evaluation of what this feature reached and a ranked list of what comes next.

**Why this priority**: Proofs that are not re-checked go stale. The evaluation picks the next
step among the remaining `Region` implementors (`Cell`'s float methods, the region adaptors)
and the follow-ups.

**Independent Test**: A change that breaks one proven property fails the automated check and
names it; the revert passes. A maintainer who did not do the work can answer from the
evaluation alone: "how much of `PointRegion` and of `CellUnion`'s floating-point half is
proven, which `Region` laws hold for each, and what next?"

**Acceptance Scenarios**:

1. **Given** a change to `PointRegion`, `CellUnion`'s floating-point methods or the verification
   artifacts, **When** CI runs, **Then** these proofs run and their result appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the default
   quality gate is not slowed down or made to depend on verification toolchains.
3. **Given** the completed feature, **When** the evaluation is read, **Then** it says which
   entries are proven, which are limited and why, and ranks the follow-ups.

---

### Edge Cases

- **Points that are not unit length.** `PointRegion::new` accepts any three numbers: the zero
  vector, (3, 4, 0), NaN and infinite coordinates. Its bounds are then those of the given
  vector (the probe's special vectors match upstream's Release build), and the laws are stated
  over unit points, upstream's precondition (K3). In debug builds, NaN and infinite
  coordinates can reach upstream's debug checks (K4).
- **A point equal to a cell vertex.** The region of a vertex intersects the cell, since
  `Cell::contains_point` accepts its own vertices (expanded by ε in u-v); the vertex law for
  `PointRegion` rests on it.
- **Signed zeros.** `PointRegion::contains_point` is `==` on points, so (0, 0, 1) and
  (−0, −0, 1) are the same point; a NaN point contains nothing, itself included.
- **The empty and the whole union.** The empty union's cap is empty, its rectangle empty, its
  covering the four cells of the empty cap (008 K8), its areas +0 after K2. The six faces' cap
  is centred on (1, 0, 0), since the centroid is the zero vector.
- **Unions of opposite faces.** Faces 0 and 3 give a centroid of exactly zero, hence the
  fallback centre; the cap must still contain both faces.
- **Ids without a `Cell`.** The bounds and areas panic, in every build, where building the
  id's `Cell` panics (002's documentation); the other invalid ids give an unspecified result.
  The proofs take valid unions.
- **Radius expansion.** A radius of 0, negative, below the leaf width, above the face width
  (expanded twice), infinite, or NaN (a debug-build panic after K4); `max_level_diff` up to
  255, clamped to level 30.
- **Unmodelled functions.** `atan2` (in `LatLng::from_point`, so in the rectangle bounds and
  `Cell::rect_bound`), `asin` (in the cap's angle, so in the covering's level) and `sqrt` (in
  the centroid's normalisation) run under the contracts of 008, 010 and the coverage review. A
  property that needs two calls on one input to agree uses the pure contracts of 010; one that
  needs the value stays with the C++ vectors.
- **Size bounds.** `CellUnion` proofs run over unions of at most a few cells, as 002's did; each
  bound is measured and stated.

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify `PointRegion` and its trait implementations, and MUST
  instantiate the generic `Region` laws and equality laws for it.
- **FR-002**: The verification artifacts MUST include an operation inventory for `PointRegion`:
  `new`, `point`, the six `Region` methods, the derived equality and the derived
  deserialisation (10 entries); text display is excluded, as for every other type.
- **FR-003**: The feature MUST bring into scope `CellUnion`'s nine floating-point inventory rows
  of spec 002: `contains_point`, `expand_by_radius`, `average_based_area`, `approx_area`,
  `exact_area`, and `Region`'s `cap_bound`, `rect_bound`, `cell_union_bound` and
  `contains_point`. Each MUST get a crash proof or a recorded tool limit with a measurement; an
  entry that only calls a measured tool limit (`exact_area` through `Cell::exact_area`, the
  coverings through `Cap::cell_union_bound`, `expand_by_radius` through `expand_at_level`'s
  `all_neighbors`) MAY inherit its measurement, with the inheritance written down.
- **FR-004**: Entries that go through `atan2`, `asin`, `sqrt`, `sin` or `cos` MUST be proven
  under the existing tested function contracts (status **proven under a stated contract**). A
  new contract or stub MUST be tested against the real function as the existing ones are and
  recorded in the trusted base.

**Guarantees**

- **FR-005**: For every in-scope operation, verification MUST establish that it completes
  without panic, arithmetic overflow or undefined behaviour, for every input in its
  documented domain:
  - a `PointRegion` ranges over every triple of `f64`, NaN and the infinities included,
    narrowed only by a documented precondition;
  - a cell over every valid cell;
  - a union over valid unions within stated size bounds;
  - a radius over every `Angle`, and `max_level_diff` over every `u8`.

  A precondition counts as documented if the API documentation, a debug-build check or
  upstream states it. An undocumented precondition that a proof needs MUST be recorded as a
  finding and documented (K3, K4).
- **FR-006**: The verification artifacts MUST include a property catalogue covering at least:
  - **`PointRegion`'s construction**: `point` returns what `new` was given, bit for bit.
  - **`PointRegion`'s bounds**: the cap bound is `Cap::from_point` of the point (radius zero);
    the rectangle bound is the single `LatLng` of the point; the covering is the cap's covering.
  - **`PointRegion`'s cell and point tests**: `contains_cell` is always false; `intersects_cell`
    is `Cell::contains_point`; `contains_point` is equality of points.
  - **`CellUnion`'s point test**: `contains_point(p)` is `contains_cell_id` of `p`'s leaf cell.
  - **`CellUnion`'s bounds**: the rectangle bound is the union of the cells' rectangle bounds;
    the cap bound is upstream's construction (the area-weighted centroid, or (1, 0, 0) when it
    is zero, grown by each cell's cap bound), empty for the empty union; the covering is the
    cap bound's covering.
  - **`CellUnion`'s areas**: the average-based area is the leaf count times the leaf area; the
    approximate and exact areas are the sums of the cells' areas in order, +0 for the empty
    union (after K2).
  - **`CellUnion::expand_by_radius`**: the level chosen is upstream's (`MIN_WIDTH`'s level for
    the radius, clamped by `max_level_diff` and level 30, with the face-width double
    expansion); the result contains the original union.
  - **Equality**: `PointRegion`'s equality is the point's; the equality laws on points without
    NaN.

  Each entry MUST record a plain-language statement, its kind, the upstream source, the domain,
  and a status: **proven**, **proven within stated bounds**, **proven by composition**, **proven
  under a stated contract**, or **not proven (reason)**.
- **FR-007**: Properties MUST be stated against a reference description written from
  upstream's semantics that does not restate the implementation's code: a point region is one
  point; a cell union is the set of points whose leaf cell lies in one of its cells; a bound
  contains the region. Where a property is "bit for bit upstream's formula", the restatement is
  an explicit part of the description, kept apart from the meaning and checked against the C++
  vectors.
- **FR-008**: Verification MUST be exhaustive and automated over every input in the domain
  (within the stated size bounds for unions), with exact bit-level IEEE-754 semantics, and MUST
  produce a concrete counterexample on failure.
- **FR-009**: Every proven property MUST be shown to be non-vacuous: at least one injected
  defect MUST make it fail, and the injected defects and outcomes MUST be recorded.
  Crash-freedom proofs MUST show their domains non-empty and their outcomes reachable.

**The `Region` laws**

- **FR-010**: The generic laws MUST gain the two `Region` invariants `CORE_TRAITS.md` states
  without a law: **the bounding rectangle contains every contained point**, and **the covering
  bound contains every contained point**. Each MUST be stated once, generically, with a
  quickcheck twin for every `Region` implementor the existing twins cover.
- **FR-011**: Every generic `Region` law MUST be instantiated for `PointRegion` over every unit
  point and every valid cell: containment implies intersection; the covering bound covers every
  contained cell; the vertex law; the bounding cap, the bounding rectangle and the covering each
  contain the point. A law instance that needs a function's value beyond its contract MUST be
  recorded as not proven with the reason, and SHOULD then be proven by composition from proven
  entries where the argument is short.
- **FR-012**: Every generic `Region` law but the vertex law MUST be instantiated for `CellUnion`
  over valid unions within stated size bounds and every valid cell. The vertex law MUST be
  recorded as not a law of `CellUnion` (as for `Cell`), with the probe's counterexample as a
  fixed test and the reason in `laws.rs` and `CORE_TRAITS.md`.
- **FR-013**: The quickcheck twins MUST draw `PointRegion`s at cell vertices, on face
  boundaries and at the poles, and unions with polar faces, opposite faces and clustered cells,
  so that the probe's special cases are reachable.

**Non-intrusiveness and operability**

- **FR-014**: The feature MUST NOT change the public API of `PointRegion`, `CellUnion` or
  `Cell`, and MUST NOT change observable behaviour except through FR-017 to FR-019. It MUST NOT
  add runtime dependencies or slow the library down. Verification-only code MUST be excluded
  from normal builds.
- **FR-015**: A defect found by verification MUST be fixed as a separate, recorded change,
  with parity checked against upstream and a regression test that fails without the fix (red
  first, then green).
- **FR-016**: The proofs MUST run from the project's documented verification command, with a
  documented way to run only this feature's proofs. They MUST run automatically in CI for
  changes to `PointRegion`, `CellUnion`'s floating-point methods, `laws.rs` or the verification
  artifacts, and stay out of the default quality gate. Proofs over the per-change budget run in
  the weekly tier, with a CI leg.

**Known gaps (survey findings)**

- **FR-017** (K1): `Cell::rect_bound` MUST use upstream's latitude of a polar face's corners,
  the `f64` nearest `asin(sqrt(1/3))`, so that the bounds of face cells 2 and 5, and every
  bound that unions them, are upstream's bit for bit. Its own change, with the two face cells
  and the probe's three unions as regression tests. Tests that pin the old bounds are updated
  and listed with the change; the Python and wasm bindings inherit it.
- **FR-018** (K2): `CellUnion::approx_area` and `exact_area` MUST return +0 for the empty union,
  as upstream, and otherwise the same sums in the same order. Its own change, with a regression
  test.
- **FR-019** (K3, K4):
  - `PointRegion::new` MUST document upstream's precondition (a unit point) and what the region
    does with any other point;
  - `PointRegion`'s deserialiser MUST reject a point that is not unit length, as upstream's
    decoder and `Cap`'s deserialiser do (a documented behaviour change, as 008's K7);
  - every debug-build panic of K4 MUST be documented where it happens and on the operations of
    this feature that reach it (`expand_by_radius`, `PointRegion::rect_bound`,
    `intersects_cell` and `cell_union_bound`, `CellUnion::contains_point`), stating the inputs
    exactly, and proven exactly there where the checker reaches it.

  Each with a test.
- **FR-020** (K5): Upstream's `S2PointRegion` codec is missing from the library. It is a
  follow-up, not ported here, as spec 010's missing operations were: the evaluation MUST rank
  it, and `VERIFIED_TYPES.md` MUST list it as a missing upstream operation.

**Reference vectors, upstream findings and records**

- **FR-021**: The feature MUST add committed C++ reference vectors, with differential tests:
  for `S2PointRegion`, its three bounds and two cell tests, over random and special points; for
  `S2CellUnion`, `GetCapBound`, `GetRectBound`, `GetCellUnionBound`, `Contains(point)`, the
  three areas and `Expand(radius, diff)`, over random, clustered and special unions (empty, the
  whole sphere, the polar faces, opposite faces); and `S2Cell::GetRectBound` of the six face
  cells. CI MUST NOT need a C++ toolchain to check them.
- **FR-022**: Undefined or wrong behaviour found in the upstream reference MUST be noted as a
  follow-up task before the affected proof is marked complete, and listed in the evaluation and
  in `UPSTREAM.md`. (The survey found none.) Reporting upstream needs the maintainer's approval.
- **FR-023**: As the constitution requires ("Verification records"), the change MUST update
  the records it affects in the same change: the value-type suite's inventory and catalogue
  for `PointRegion`; 002's inventory and catalogue for `CellUnion`'s nine rows; the index of
  properties by kind and its rendered copies; the trusted base; `VERIFIED_TYPES.md`, where
  `PointRegion` gets a section and §3's `Region` row gains both; `CORE_TRAITS.md`, where the
  `Region` invariants name their instances; and `verify/COVERAGE.md`. The evaluation MUST
  record the effort, the run time, what was reachable, the limitations and the defects found,
  and rank the follow-ups.

### Key Entities

- **`PointRegion`**: the first target. One point as a `Region`; any point is accepted, the laws
  hold over unit points.
- **`CellUnion`'s floating-point half**: the second target. Its bounds, point test, areas and
  radius expansion, over valid unions within stated size bounds.
- **Generic `Region` laws**: the four of 004 and 008, and the two FR-010 adds, stated once and
  instantiated per implementor.
- **Reference description**: a point region is a point; a union is the points whose leaf cell
  it contains; restatements of upstream's constructions kept apart (FR-007).
- **Function contracts, stubs and the abstract u-v bound**: those of 008, 010 and the coverage
  review, and any added under FR-004.
- **Operation inventory, property catalogue, trusted base and evaluation**: as in 001–010.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope `PointRegion` operation is proven free of panics, overflow and
  undefined behaviour over its whole documented domain, with no size bound.
- **SC-002**: All 10 `PointRegion` entries and all nine `CellUnion` floating-point entries have
  a crash proof, or are recorded as tool limits with a measurement (their own or an inherited
  one). Every exclusion has a written reason. Every contract passes its tests against the real
  function.
- **SC-003**: The catalogue covers every area in FR-006. At least 90% of the entries whose
  operations are all in scope and expressible are proven, over the whole domain, within stated
  bounds, by composition, or under a stated contract. Entries that need the value of an
  unmodelled function, or an operation measured as a tool limit, are not counted.
- **SC-004**: `PointRegion` has a proven instance of each of the six `Region` laws, or a written
  reason with a proof by composition where FR-011 allows. `CellUnion` has a proven instance
  within bounds, or a written reason, of each law but the vertex law, and the vertex law's
  exception is recorded with its counterexample.
- **SC-005**: 100% of proven properties have recorded non-vacuity evidence, and 100% of
  crash-freedom proofs have satisfied reachability checks.
- **SC-006**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine, a hosted Linux runner with 4 virtual CPUs and 16 GB.
- **SC-007**: Each regression test of FR-017 to FR-019 fails before its fix and passes after
  it. After the fixes, the probe's library-versus-upstream differences are 0 in every section.
- **SC-008**: The differential tests against the new C++ reference vectors pass on every row.
- **SC-009**: Apart from FR-017 to FR-019, the public API and behaviour of `PointRegion`,
  `CellUnion` and `Cell` are unchanged, and the full quality gate passes before and after.
- **SC-010**: Every operation the proofs show can panic documents when it panics.
- **SC-011**: Every catalogue entry appears exactly once in the index of properties by kind,
  and the consistency check passes.
- **SC-012**: Every undefined or wrong upstream behaviour met has a follow-up task and an
  entry in the evaluation and in `UPSTREAM.md`.
- **SC-013**: A maintainer who did not take part can answer from the evaluation alone: "how
  much of `PointRegion` and of `CellUnion`'s floating-point half is proven, which `Region` laws
  hold for each, and what comes next?"

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Users benefit through bounds that match
  upstream and a `Region` contract they can rely on for two more implementors.
- **The scope**: `PointRegion` and `CellUnion`'s floating-point methods, chosen by the
  maintainer on 2026-10-28 from 010's ranked list; `Cell`'s remaining floating-point methods
  stay a follow-up, except K1's constant, which this feature's bounds expose.
- **Builds on 001–010**: the verification setup, the memory-guarded runner, the value-type and
  union suites, the mutant campaign and its per-spec CI leg, the weekly tier, the generic laws,
  the function contracts, pure contracts and lemma stubs of 008 and 010, the abstract u-v bound
  and abstract cell of the coverage review, 002's union domains and size bounds, and 009's
  bounded types.
- **Lessons carried over**: recomputed float products time out, so laws use lemma stubs and
  drift guards on the source text (008, 010); two calls of `sqrt` or `atan2` on one input may
  differ in the checker unless a pure contract stubs them; CBMC's built-in `sqrt` of a negative
  argument fails its NaN check (use the `sqrt` contract); union proofs need concrete lengths and
  the smallest unwinding that passes (002).
- **Assurance level**: exhaustive model checking of the library's own code, as in 001–010,
  within stated size bounds for unions, with the unmodelled functions under their tested
  contracts.
- **Reference semantics**: upstream C++ at v0.14.0-40, Release build on x86-64, with Go and Java
  secondary. Where the library deviates and documents it, the documented behaviour is verified.
- **Parity first** (Principle I): K1 and K2 are library-only, so they are fixed to upstream's
  result. K3's deserialiser change follows 008's precedent for `Cap` (K7 there), and matches
  upstream's decoder.
- **No new public API**: the missing codec (K5) is a follow-up, as spec 010's missing upstream
  operations were at the maintainer's choice.
- **Out of scope**: `CellUnion`'s integer operations (proven in 002); `Cell`'s other
  floating-point methods; the region adaptors (`RegionUnion`, `RegionIntersection`); porting the
  codec (K5); text display.
- **Survey findings**: K1–K5 were confirmed by the probe on 2026-10-28 against a running
  upstream build; the C++ vectors of FR-021 re-check them.
- **Existing tests stay**: the proofs complement the unit, property, differential and generic
  trait tests. No test is removed. Tests that pin old behaviour changed by FR-017 to FR-019 are
  updated, and the update is listed with the change.
