# Feature Specification: A Monotone `atan2`, and the Bounding Laws It Opens

**Feature Branch**: `tb/013-frm/1`

**Created**: 2026-11-01

**Status**: Implemented 2026-11-01. No clarification was needed. P-CREG-02 is proven for the
cell's points as directions, under a tested order contract for `atan2`; `CellUnion`'s P-UREG-05
follows by composition. P-LREG-03, P-ADREG-04, P-PTDST-02 and the normalised point are recorded
with sharpened reasons, as SC-003 and SC-004 allow. Every success criterion is met on the
workstation; SC-006 on a hosted runner needs a push (tasks T015). Evaluation: the spec-013 part
of `verify/values/EVALUATION.md`.

**Input**: User description: "next features to verify: spec 012's recommendation 1, a tested
monotonicity contract for atan2 (and what else it needs, e.g. sin_cos) to open Rect's
bounding-cap law P-LREG-03, the adaptors' P-ADREG-04 by composition, and Cell's
bounding-rectangle law P-CREG-02"

## Context

### Where the earlier features left off

- **001–011** proved `CellId`, `CellUnion`, the decoding layer, the value types, `PointRegion`
  and most of the generic `Region` laws for `Cell`, `Cap`, `Rect` and `CellUnion`.
- **012** proved the region adaptors (`RegionUnion`, `RegionIntersection`) over abstract members,
  and recorded U-017 (`Cell`'s bounds miss `contains_point`'s margin, upstream too). Three laws
  were left with reasons that name the same missing fact:
  - **P-CREG-02**, `Cell`'s bounding rectangle holds the cell's own points: "needs `atan2`'s
    monotonicity: under its range contract a counterexample appears, which the real `atan2` never
    gives" (012 R0).
  - **P-LREG-03**, `Rect`'s bounding cap holds every point the rectangle holds: "needs `atan2`'s
    and `sin_cos`'s values" (spec 010).
  - **P-ADREG-04**, the adaptors' bounding cap: by composition from P-ADREG-03 and P-LREG-03.
- **The list** (spec 012's evaluation, "Recommendations"): "1. **`Rect`'s bounding-cap law**
  (P-LREG-03): with it, the adaptors' P-ADREG-04 follows by composition. It needs `atan2`'s and
  `sin_cos`'s values; a monotonicity contract on `atan2`, tested like the others, is the likely
  route, and would also open `Cell`'s rectangle law (P-CREG-02)." Nothing is pushed.

### Why a contract, and what kind

The checker does not model `atan2` (memory: unsupported), so every statement through it uses a
contract. Today's contract gives only the result's range and sign. A bound computed from corner
values (`Cell::rect_bound` takes the latitudes and longitudes of two corners each) holds a point
only if `atan2` *orders* its results as it orders its arguments' angles. That relation spans two
calls: a contract that remembers the calls it has answered and keeps their results in order. Such
a contract is an assumption about the platform's `atan2`, which no standard guarantees exactly
(correctly rounded implementations are monotone; glibc's is accurate to within an ulp but not
proven monotone), so it must be stated in the weakest form the proofs need and tested hard
against the real function.

### Survey findings (probed 2026-11-01)

The probe is in `probe/` (`survey/` on the library, `mono/` on `f64::atan2` with exact
arithmetic):

- **K1, `atan2` keeps the order of its arguments' angles, on every pair tried.** 20,000,000
  near-tie pairs in the right half-plane (each argument within four ulps of the other's), their
  exact angle order decided in big integers: no result out of order. 8,335,328 pairs in all four
  quadrants that are ordered by *dominance* (in the first quadrant, `y₁ ≤ y₂` and `x₁ ≥ x₂`; and
  the mirror images): none out of order.
- **K2, no cell below level 0 straddles a u-v axis.** Of 400,000 cells at every level and face,
  none at level 1 or deeper has a u-v bound that crosses `u = 0` or `v = 0`: each lies in one
  quadrant of its face, where latitude and longitude are monotone in each coordinate. Level-0 cells
  have fixed bounds.
- **K3, `Cell::rect_bound` holds the cell's own points.** 5,200,000 points of those cells (their
  corners, edge midpoints, centres and random interior points): none outside the rectangle,
  whether taken as the direction `face_uv_to_xyz(face, u, v)` or normalised.
- **K4, the same contract could open one more entry.** P-PTDST-02 (spec 006, the distance and
  the stable angle are symmetric bit for bit) is a tool limit only because the checker does not
  model `atan2`; a contract that answers equal arguments equally may prove it.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A tested order contract for `atan2` (Priority: P1)

As the s2rst maintainer, I want a contract for `atan2` that states how it orders its results, in
the weakest form the proofs need, tested against the real function on adversarial inputs and
recorded in the trusted base, so that bounds computed from `atan2` can be proven.

**Why this priority**: Every other story rests on it, and it is an assumption about the platform,
so it must be justified before anything is built on it.

**Independent Test**: Run the contract's tests: they check the real `atan2` on at least ten
million adversarial pairs per rule, and the existing range contract still holds. The trusted base
names the contract, its rules and its evidence.

**Acceptance Scenarios**:

1. **Given** two argument pairs in the same closed quadrant, one dominating the other, **When**
   the real `atan2` is evaluated, **Then** the results are in the dominance order (K1).
2. **Given** equal arguments, **When** the contract answers twice, **Then** it gives one value.
3. **Given** the contract, **When** a reviewer reads the trusted base, **Then** they find its
   rules, why the real function satisfies them, and the tests that check it.

---

### User Story 2 - `Cell`'s bounding-rectangle law (Priority: P1)

As the maintainer, I want it proven that a cell's bounding rectangle holds every point of the cell
(its u-v bound mapped to the sphere), so that the `Region` contract's rectangle half holds for
`Cell` and, by composition, for `CellUnion`.

**Why this priority**: It is the law the contract most directly opens (K2, K3), and `CellUnion`'s
P-UREG-05 waits on it.

**Independent Test**: Run the verification command: P-CREG-02 is reported proven for every valid
cell and every point of its u-v bound, under the stated contracts; injecting a defect into
`Cell::rect_bound` (a wrong corner, a missing expansion) fails it.

**Acceptance Scenarios**:

1. **Given** a cell at level 1 or deeper and any `(u, v)` in its u-v bound, **When** verification
   runs, **Then** the cell's rectangle bound contains `LatLng::from_point` of the point
   `face_uv_to_xyz(face, u, v)`.
2. **Given** a face cell (level 0), **When** verification runs, **Then** the same holds with its
   fixed bound.
3. **Given** the point normalised first, **When** verification runs, **Then** the law is proven or
   recorded with the reason (the rectangle's `2ε` expansion must absorb normalisation's rounding),
   with K3's points as evidence.
4. **Given** `CellUnion`'s P-UREG-05, **When** P-CREG-02 is proven, **Then** it is re-stated by
   composition for the points of its cells, or keeps its reason.

---

### User Story 3 - `Rect`'s bounding-cap law, and the adaptors' (Priority: P2)

As the maintainer, I want `Rect`'s bounding-cap law (P-LREG-03) proven or its reason sharpened,
and the adaptors' (P-ADREG-04) following by composition if it is.

**Why this priority**: The list's first item names it, but it needs more than an order: the cap
compares chord distances from points built with `sin` and `cos` against a point whose angles came
from `atan2`, so it may need contracts relating the three functions, which may not be provable in
the checker.

**Independent Test**: The verification command reports P-LREG-03 proven, or the catalogue records
the exact facts it would need and why they are out of reach, with the measurement; P-ADREG-04
follows the same status.

**Acceptance Scenarios**:

1. **Given** a valid rectangle and a point it contains, **When** verification runs, **Then** its
   (widened) cap bound contains the point, or the instance is recorded with the reason.
2. **Given** P-LREG-03 proven, **When** the adaptors' records are updated, **Then** P-ADREG-04 is
   proven by composition with P-ADREG-03; otherwise it keeps its reason.

---

### User Story 4 - What else the contract opens (Priority: P3)

As the maintainer, I want every other entry blocked only on `atan2` re-tried under the new
contract (P-PTDST-02 first), so that the catalogue reflects what the contract gives.

**Why this priority**: Cheap to try once the contract exists; not required for the list's item.

**Independent Test**: Each such entry is reported proven under the contract, or keeps its reason
with the new measurement.

**Acceptance Scenarios**:

1. **Given** P-PTDST-02, **When** verification runs with the contract, **Then** it is proven or
   keeps its reason.

---

### User Story 5 - Records, CI and evaluation (Priority: P3)

As a contributor, I want the new proofs in CI, and as the maintainer a short evaluation of what
the contract opened and what comes next.

**Why this priority**: Proofs that are not re-checked go stale.

**Independent Test**: A change that breaks a proven property fails the automated check; the
evaluation answers "what does the order contract assume, what did it prove, and what next?"

**Acceptance Scenarios**:

1. **Given** a change to `Cell`'s or `Rect`'s bounds, `LatLng`'s conversions or the contract,
   **When** CI runs, **Then** these proofs run.
2. **Given** the completed feature, **When** the evaluation is read, **Then** it answers which laws
   hold, under which assumptions, and ranks the follow-ups.

---

### Edge Cases

- **Signed zeros and axes.** `atan2(+0, −1) = π` but `atan2(−0, −1) = −π`; the library adds `+0`
  to longitude arguments. Quadrants are closed and defined by sign bits, so a pair on an axis
  belongs to the quadrant its signs say, and the contract never orders across the `±π` cut.
- **Ties.** Equal arguments give equal results; arguments of equal exact angle but different
  values (`(1, 2)` and `(2, 4)`) are not related by dominance, and the contract says nothing about
  them unless the proofs need it.
- **The poles.** A cell touching a pole has the pole at a corner (K2); its latitude there is
  `atan2(±1, 0) = ±π/2`, and `polar_closure` makes the longitude full.
- **Face cells.** Their bounds are fixed (±π/4, the polar caps' `POLE_MIN_LAT`); the proof needs
  `atan2`'s values at a few exact arguments (`atan2(y, y) = π/4` for `y > 0`), stated and tested
  as anchors of the contract.
- **Normalisation.** A normalised point's coordinates are each rounded separately, so its angles
  may move by a few ulps; only the bound's `2ε` expansion covers that (User Story 2, scenario 3).
- **The margin (U-017).** Points within `contains_point`'s margin but outside the cell stay the
  documented exception; this feature is about the cell's own points.
- **Not monotone after all.** If a test finds the real `atan2` out of order, the contract is
  weakened to what holds (e.g. order within one ulp), or the dependent laws are recorded as not
  proven; nothing is proven on a contract its tests contradict.

## Requirements *(mandatory)*

### Functional Requirements

**The contract**

- **FR-001**: The feature MUST define an order contract for `atan2`: the existing range and sign
  rules, one value per argument pair across a proof's calls, results ordered as their arguments
  are by dominance within a closed quadrant (K1), and the exact anchor values the proofs need; in
  the weakest form that serves the proofs.
- **FR-002**: The contract MUST be tested against the real `atan2`: at least ten million
  adversarial pairs per rule (near ties, signed zeros, subnormals, axes), every anchor exactly; and
  recorded in the trusted base with why the real function satisfies it.
- **FR-003**: Any other new contract (`sqrt`'s order, `sin`/`cos` relations for P-LREG-03) MUST meet
  FR-002's standard; a contract the real function cannot be shown to meet MUST NOT be used.
- **FR-004**: Statements that rely on these contracts MUST carry the status **proven under a
  stated contract**, naming them.

**The laws**

- **FR-005**: P-CREG-02 MUST be attempted for every valid cell and every point
  `face_uv_to_xyz(face, u, v)` with `(u, v)` in its u-v bound, and for the normalised point; each
  recorded as proven or not proven with the reason and measurement.
- **FR-006**: P-LREG-03 MUST be attempted for every valid rectangle and every point it contains,
  on its widened cap (spec 010 FR-019), and recorded likewise; if not proven, the record MUST name
  the facts the proof would need.
- **FR-007**: P-ADREG-04 MUST be re-stated by composition if P-LREG-03 is proven; `CellUnion`'s
  P-UREG-05 likewise from P-CREG-02 (and P-UREG-04 if P-CREG-01 ever is).
- **FR-008**: Every other catalogue entry whose reason is only that `atan2` is unmodelled MUST be
  re-tried under the contract (P-PTDST-02 at least).
- **FR-009**: Verification MUST be exhaustive and automated over its domain, with exact IEEE-754
  semantics apart from the stated contracts, and MUST produce a counterexample on failure.
- **FR-010**: Every proven property MUST be shown non-vacuous by an injected defect in the library
  code it covers, recorded with its outcome.
- **FR-011**: The quickcheck twins MUST cover each newly proven law over random cells (every level
  and face) and points at corners, edges and interiors, and rectangles across the antimeridian
  and the poles.

**Non-intrusiveness and operability**

- **FR-012**: The feature MUST NOT change the public API or the behaviour of the library. No
  runtime dependency; verification-only code excluded from normal builds.
- **FR-013**: A defect found by verification MUST be fixed as a separate, recorded change with a
  regression test that fails without the fix, after the maintainer's go-ahead if it is not in this
  spec. An upstream finding gets a U-NNN entry and a follow-up task; reporting it upstream needs
  the maintainer's approval.
- **FR-014**: The proofs MUST run from the documented verification command with a way to run only
  this feature's, in CI for changes to the files they cover, outside the default quality gate;
  proofs over the per-change budget run weekly.

**Records**

- **FR-015**: As the constitution requires, the records the change affects MUST be updated in the
  same change: catalogues, the index by kind and its render, the trusted base, `MUTANTS.md`,
  `VERIFIED_TYPES.md`, `CORE_TRAITS.md`, `COVERAGE.md`, and the evaluation, which ranks the
  follow-ups.

### Key Entities

- **The order contract**: `atan2`'s results, related across the calls of one proof by the
  arguments' dominance order within a quadrant; a stated, tested assumption about the platform.
- **The anchors**: `atan2`'s exact values at a few arguments the bounds use.
- **`Cell`'s rectangle bound**: two corners' latitudes and longitudes, expanded by `2ε` and closed
  at the poles; judged against the cell's points.
- **`Rect`'s cap bound**: spec 010's widened cap; judged against the points the rectangle holds.
- **Inventory, catalogue, trusted base, evaluation**: as in 001–012.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The order contract passes its tests on at least ten million adversarial pairs per
  rule with no counterexample, and every anchor exactly.
- **SC-002**: P-CREG-02 is proven for every valid cell and every point of its u-v bound, under the
  stated contracts; its normalised form is proven or recorded with a reason.
- **SC-003**: P-LREG-03 and P-ADREG-04 are proven, or recorded with the exact missing facts and a
  measurement; P-UREG-05 is proven by composition for its cells' points or keeps its reason.
- **SC-004**: Every entry re-tried under the contract (FR-008) is proven or keeps its reason with
  the new measurement.
- **SC-005**: 100% of proven properties have recorded non-vacuity evidence.
- **SC-006**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine (a hosted Linux runner, 4 vCPUs, 16 GB).
- **SC-007**: The public API and behaviour are unchanged, and the full quality gate passes.
- **SC-008**: Every catalogue entry appears once in the index of properties by kind, and the
  consistency check passes.
- **SC-009**: From the evaluation alone, a maintainer can answer "what does the order contract
  assume, which laws does it prove, and what comes next?"

## Assumptions

- **Scope**: spec 012's ranked item 1 as the user framed it: the contract, P-CREG-02, P-LREG-03,
  P-ADREG-04, and what else the contract opens. `Cell`'s bounding-cap law (P-CREG-01, `sqrt`) is
  out of scope unless the same work opens it.
- **The cell's points**: P-CREG-02 is stated for the directions `face_uv_to_xyz(face, u, v)`, which
  are the cell's points as the library defines them (a direction need not be unit length for
  `LatLng::from_point`); the normalised form is an additional statement (FR-005).
- **The platform**: the reference is glibc's `atan2` on x86-64, as the tests run; the trusted base
  says that another platform's `atan2` would need the tests re-run.
- **Builds on 001–012**: the suite, the runner, the contracts and stubs, the abstract u-v bound and
  cell, the laws and their twins, spec 010's widened cap and its C++ rows.
- **Lessons carried over**: products recomputed in a statement time out (lemma stubs); `sqrt` and
  `powi` are not functions to the checker; CBMC's `cos` and `log2` are loose; replay every
  counterexample natively before believing it.
- **Assurance level**: exhaustive model checking of the library's own code, with `atan2` (and any
  other unmodelled function) under tested contracts.
- **Out of scope**: widening or changing any bound; U-017's margin; the shape-backed regions; the
  codecs and inexact remainders of spec 012's list.
- **Existing tests stay**; tests are added, none removed.
