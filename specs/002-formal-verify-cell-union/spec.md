# Feature Specification: Formal Verification of `CellUnion`

**Feature Branch**: `002-next-verification/1`

**Created**: 2026-10-02

**Status**: Draft

**Input**: User description: "Formal verification of CellUnion"

## Context

Feature 001 used exhaustive automated model checking to prove `CellId`. It proved
crash-freedom for 48 operations and 39 functional properties against an independent model
of the cell hierarchy. Along the way it found one new defect, two parity defects and two
cases of undefined behaviour in the C++ reference. Its evaluation (`verify/EVALUATION.md`)
recommends continuing with the same approach for integer code and avoiding
floating-point geometry.

`CellUnion` is the natural next type. A cell union is a sorted collection of cells that
represents a region of the sphere. Region coverings, term indexing, sharding, the cell index
and the closest-cell query all build on it, and the library decodes it from untrusted bytes.
Its set operations (normalising, membership, union, intersection, difference, range
construction) are integer logic built from the `CellId` operations that 001 proved.

A survey before this specification (2026-10-02) used concrete tests to confirm three
behaviours that differ from upstream:

- `denormalize` panics for valid arguments where C++ clamps to the maximum level.
- The leaf-cell count can overflow on an overlapping union built verbatim.
- Decoding normalises the union, where C++ keeps the identifiers exactly as encoded.

This feature handles them (FR-015 to FR-017).

One difference from 001 shapes the whole feature. A `CellId` is a fixed-size value, so most
of 001's proofs covered every input. A cell union has no fixed length, so exhaustive
checking can cover unions only up to a stated number of cells. Most results here will
therefore be "proven within stated bounds". The planning probe found those bounds to be
small for any operation that builds a union (research R0).

## Clarifications

### Session 2026-10-02

- Q: How should decoding a cell union behave? Upstream C++ keeps the decoded identifiers
  exactly as encoded and validates nothing. The library rejects invalid identifiers and
  then normalises, so four encoded siblings decode as their parent. Neither deviation is
  documented. → A: Keep the identifiers exactly as encoded, but return an error unless they
  form a valid union (valid cells, sorted, non-overlapping). Every valid union round-trips
  exactly. The rejection is documented as a deliberate deviation from upstream (FR-017).
  C++ decodes input into a union that breaks its own documented invariant. When the
  decoding proof is done, that is assessed as an upstream finding under FR-019.
- Q (raised during planning): the feasibility probe (research R0) found that SC-001's
  minimum bounds cannot be met within the per-harness budget for operations that build a
  union. Normalising needs about 14 GB at 3 cells, so a 4-cell sibling merge is out of
  reach. `difference` and the range constructors exceed the budget even at 1 cell.
  Decoding stalls in the text formatting of its error messages. How should the feature
  proceed? → A: Keep the model checker and amend the bounds to what fits. The read-only
  operations are covered at 3 or more cells, the normalising operations at 2 cells, and
  the two-union operations at 1 cell each. `difference` and the three range constructors
  become tool-limit exclusions; property tests and the C++ vectors cover them. The
  decoder's error messages are formatted only when they are displayed, with the same
  text, so that decoding can be checked. The three known-defect fixes, the documentation
  and the C++ vectors are unchanged. The evaluation reports whether bounded proofs over
  collections are worth it.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for `CellUnion`, including decoding (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope `CellUnion`
operation completes without panicking, overflowing or hitting undefined behaviour. The
proof should cover every input within stated bounds, including every byte sequence passed
to the decoder. Cell unions arrive from files and network messages, and Constitution II
requires that decoding untrusted bytes never crashes.

**Why this priority**: Crash-freedom is the property the constitution already requires
(Principle II), and decoding is where it matters most. It is also the cheapest result to
reach, and it is useful even if no functional property is proven.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope operation as proven free of panics, overflow and undefined
behaviour within its stated bounds, with no unproven obligations remaining.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification
   command, **Then** it reports each in-scope operation by name as proven crash-free, states
   the union-size or input-size bound used, and exits successfully.
2. **Given** any byte sequence up to the stated length, **When** it is decoded as a cell
   union, **Then** the proof shows that decoding returns either a cell union or an error, and
   never panics or hangs.
3. **Given** an operation with a documented precondition (for example "the union must be
   valid" or "the identifier must be a valid cell"), **When** verification runs, **Then** the
   proof assumes exactly that precondition and no stronger one, and the report states it.
4. **Given** an in-scope operation deliberately changed so that it can overflow, **When**
   verification runs, **Then** it fails and names the operation and a concrete union (and any
   other argument) that triggers the failure.

---

### User Story 2 - Proven set semantics (Priority: P2)

As the maintainer, I want a catalogue of `CellUnion`'s functional properties, each proven
within stated bounds against an independent description of what a cell union means. A cell
union stands for the set of leaf cells it covers. Normalising must keep that set unchanged.
Membership queries must agree with it. Union, intersection and difference must produce the
set union, intersection and difference. Range constructors must cover exactly the requested
range of leaf cells. Encoding then decoding must give back the same union. Each property is
traced to upstream S2 semantics, which extends the parity guarantee (Principle I) from
sampled inputs to every input within the bounds.

**Why this priority**: This is where formal methods add the most over the existing tests. It
depends on the scaffolding from Story 1.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven, proven within stated bounds, or not proven with a written reason. Each proven
property fails when a matching defect is injected into the implementation.

**Acceptance Scenarios**:

1. **Given** the property catalogue, **When** verification runs, **Then** every property
   marked proven, or proven within stated bounds, is discharged with no remaining
   obligations.
2. **Given** any proven property, **When** a defect that violates it is introduced into the
   implementation, **Then** verification fails for that property.
3. **Given** any property, **When** a reviewer reads the catalogue, **Then** they find a
   plain-language statement, the upstream source, the bound used and the proof status.

---

### User Story 3 - Regression protection for future changes (Priority: P3)

As a contributor changing `CellUnion`, its encoding, or the `CellId` operations it builds on,
I want the proofs to run automatically in CI. If my change breaks a proven property, I am
told which property broke and given a concrete counterexample before the change is merged.

**Why this priority**: Proofs that are not re-checked go stale. This protects the investment
from Stories 1 and 2, but it only matters once they exist.

**Independent Test**: Open a change that breaks one proven `CellUnion` property. The
automated verification check fails and names the property. Revert the change and the check
passes.

**Acceptance Scenarios**:

1. **Given** a proposed change to `CellUnion`, its encoding, `CellId`, or the verification
   artifacts, **When** CI runs, **Then** the `CellUnion` proofs run and their result appears
   on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the existing
   quality gate is not slowed down or made to depend on verification toolchains.

---

### User Story 4 - Evaluation of collection proofs (Priority: P3)

As the maintainer, I want a short written evaluation of this second verification feature.
It should cover the effort, the run time, the bounds that were reachable, what is trusted but
not proven, and the defects found. It should say whether bounded proofs over collections give
enough assurance to justify the cost, and which type or component should come next.

**Why this priority**: 001 showed that integer code is tractable. This feature tests a new
question: whether the approach still pays off for collections, whose size must be bounded.
The answer decides the shape of any further work.

**Independent Test**: A maintainer who did not do the work reads the evaluation and can answer
"should we continue?", "with which approach?" and "what next?" without asking anyone.

**Acceptance Scenarios**:

1. **Given** the completed feature, **When** the evaluation is read, **Then** it contains a
   go or no-go recommendation, the rationale, a comparison with 001, and a ranked list of
   candidates for follow-up work.

---

### Edge Cases

- **Empty and whole-sphere unions**: the empty union, and the six face cells that cover the
  whole sphere, must behave as documented in every operation.
- **Non-normalised and invalid unions**: a union built verbatim, or deserialised, may be
  unsorted, overlapping or duplicated, or may contain invalid identifiers. For each operation,
  the proof must state whether its domain is any sequence of identifiers, sequences of valid
  cells, valid unions (sorted and non-overlapping), or normalised unions, and why. Operations
  documented to accept any sequence, such as the validity and normalisation checks, are
  proven over that whole domain.
- **Sibling merging**: normalising replaces four sibling cells by their parent. The merge
  can cascade: the new parent may complete a group of four at the next level up. A merge
  needs four cells, which is beyond what the checker can reach (research R0). The proofs
  cover the bounds that fit. Merges and cascades are named as unreached behaviour, and
  the unit tests, property tests and C++ vectors exercise them (FR-007).
- **Mixed levels**: unions that mix face cells, leaf cells and every level in between,
  including a cell next to one of its own descendants or ancestors.
- **Cube-face boundaries**: ranges and unions whose cells lie on different faces, including
  the last cell of face 5 and the past-the-end value used to mark the end of a range.
- **Range constructors**: an empty range (begin equals end), a range of one leaf cell, the
  whole range of all leaf cells, and ranges that cross a face boundary. The constructors are
  tool-limit exclusions (FR-002), so property tests and the C++ vectors must cover these
  cases.
- **Denormalising at the finest levels**: every combination of minimum level 0–30 and level
  step 1–3. That includes cells at levels 29 and 30, where rounding up would pass the
  maximum level (FR-015). Level steps outside 1–3 are a documented precondition.
- **Leaf-cell count limits**: the count of a valid union is at most 6·4^30 leaf cells, which
  fits the result type. An overlapping union can exceed that (FR-016).
- **Decoding**: these must all be rejected with an error:
  - empty or truncated input;
  - an unsupported version byte;
  - a cell count above the maximum, or larger than the bytes that follow;
  - invalid identifiers;
  - identifiers that are unsorted, duplicated or overlapping.

  A valid but non-normalised union, such as four sibling cells, must decode unchanged
  (FR-017).
- **Debug-only checks**: several operations guard a precondition with a check that runs
  only in debug builds. Each must be proven safe for the domain its check guards, without
  relying on the check firing in release builds.
- **Recursion**: computing a difference recurses into child cells, up to 30 levels deep.
  Difference is a tool-limit exclusion (FR-002), so property tests and the C++ vectors
  must cover deep level gaps.

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify the `CellUnion` type of the core library.
- **FR-002**: The verification artifacts MUST include an operation inventory. It lists every
  inherent public operation of `CellUnion` and its hand-written trait implementations: slice
  access, iteration, collection from an iterator, text display, the region interface, and
  binary encoding and decoding. Each entry is marked **in scope** or **excluded**, with a
  written reason for each exclusion. The following are excluded by default:
  - operations whose results depend on floating-point geometry: area, cap and rectangle
    bounds, the covering bound, containment of a point, and expansion by a radius;
  - expansion at a level, which depends on the all-neighbours operation that 001 excluded as
    intractable;
  - text display, which goes through the token formatter that 001 excluded as a tool limit.

  An operation that only reads a cell's identifier stays in scope, even though the cell
  argument carries floating-point bounds. The region interface's cell containment and cell
  intersection are such operations. Any other exclusion MUST be justified in the
  evaluation, with measurements. Planning found four such tool-limit exclusions:
  - difference and the begin–end range constructor, measured over the budget even for
    one-cell inputs (research R0);
  - the minimum–maximum range constructor, which calls the begin–end one;
  - the library's own range constructor, which builds every output cell with a similar
    loop. It was not measured during planning. It is measured during implementation, and
    it stays excluded only if it does not fit.

  Their properties stay in the catalogue as "not proven". New property tests for larger
  unions and the C++ reference vectors cover them.

**Guarantees**

- **FR-003**: For every in-scope operation, verification MUST establish that the operation
  completes without panic, arithmetic overflow or undefined behaviour:
  - It MUST hold for every input in the operation's documented domain, within the stated
    bounds (FR-007).
  - The domain of a union argument is one of four: any sequence of identifiers, sequences
    of valid cells, valid unions, or normalised unions. Each proof states which one.
  - The domain of a cell argument follows 001: every bit pattern, or the valid cells
    extended with past-the-end values where upstream usage requires them.
  - Every other argument (level, level step, byte sequence) ranges over all values of its
    type, narrowed only by a documented precondition.

  A precondition counts as documented if any of these states it for the same operation:
  - the library's API documentation;
  - a debug-build check;
  - the upstream reference.

  If a proof must assume a precondition that the library's own documentation does not state:
  - the undocumented panic MUST be recorded as a finding in the inventory and the evaluation;
  - the missing "Panics" documentation MUST be added in one separate documentation-only
    change (Constitution II);
  - if the panic also differs from upstream behaviour, the difference is a defect under
    FR-010.
- **FR-004**: `CellUnion`'s two invariants MUST be stated formally:
  - **Validity**: every member is a valid cell, members are in strictly increasing order,
    and no two members overlap.
  - **Normalisation**: the union is valid, and no four consecutive members are the four
    children of one parent.

  Verification MUST establish, within the bounds, that every in-scope operation returns a
  union satisfying the invariant upstream documents for it:
  - **Normalised results**: constructing from identifiers, normalising, union, the whole
    sphere, and the minimum–maximum and begin–end range constructors.
  - **Normalised when the inputs are**: intersection, intersection with a single cell, and
    difference return a normalised union whenever their union inputs are normalised. For
    difference, only the first input needs to be normalised.
  - **Valid results**: denormalising a normalised union.
- **FR-005**: The verification artifacts MUST include a property catalogue covering at least
  these areas:
  - **Invariants**: the validity and normalisation checks agree with their formal
    statements for any sequence of identifiers.
  - **Normalisation**: the result is normalised and covers exactly the same leaf cells as
    the input. Normalising twice gives the same result. The order of the input does not
    matter.
  - **Membership**: containment and intersection of a cell agree with the set of covered
    leaf cells. So do containment and intersection of another union, under upstream's
    caveat for unions that are not normalised.
  - **Set algebra**: union, intersection, intersection with a single cell, and difference
    cover exactly the set union, intersection and difference of the covered leaf cells.
  - **Range construction**: the minimum–maximum and begin–end constructors, and the
    library's own range constructor, cover exactly the requested leaf cells.
  - **Denormalisation**: the result covers the same leaf cells, and every member satisfies
    the minimum-level and level-step constraints, except where the maximum level prevents it.
  - **Measures**: the leaf-cell count equals the number of leaf cells covered by a valid
    union.
  - **Encoding**: decoding an encoded valid union gives back exactly that union, every
    successful decode returns a valid union (FR-017), and the encoded length is as upstream
    specifies.

  Each entry MUST record a plain-language statement, the upstream source it is derived from,
  the bound used, and a status:
  - **Proven**: the whole domain is covered.
  - **Proven within stated bounds**: a union-size, input-size or loop bound leaves part of
    the domain uncovered, and that part is documented.
  - **Not proven (reason)**.
- **FR-006**: Properties MUST be expressed against a reference specification of what a cell
  union denotes, the set of leaf cells it covers. That specification is built on 001's
  reference model of the cell hierarchy and written from upstream S2 semantics. It MUST NOT
  restate the implementation's own code. The proof then establishes agreement between two
  independent descriptions (Constitution Principle I).
- **FR-007**: Verification MUST be exhaustive and automated within its bounds. For every
  in-scope operation and property, the checker MUST:
  - explore every input up to the stated bounds, not a sample;
  - use exact bit-level machine-arithmetic semantics;
  - produce a concrete counterexample on failure.

  Every proof that takes a union MUST state its union-size bound, the maximum number of
  cells per union argument. A proof that decodes MUST state its input-size bound, the
  maximum number of bytes. For the bounds:
  - The bounds SHOULD be large enough to reach every behaviour the operation documents.
    For example, a group of four siblings needs unions of at least four cells. Where the
    checker cannot reach a documented behaviour within the resource budget, the property
    is proven within the largest bound that fits. The unreached behaviour MUST be named
    in the inventory and the evaluation, and ordinary tests and the C++ reference vectors
    MUST exercise it. Normalising's sibling merge is such a case (research R0).
  - The uncovered sizes MUST be documented, and ordinary tests, property tests or fuzzing
    MUST exercise them.
  - Loop bounds follow 001's rule: the proof shows that the bound covers the whole domain,
    or it documents the uncovered part.

  Size bounds and loop bounds are the only accepted reasons to leave part of a domain
  uncovered. An operation that can be checked only on a narrower domain for any other
  reason MUST be excluded as a tool limit (FR-002).
- **FR-008**: Every proven catalogue property MUST be shown to be non-vacuous: at least one
  injected defect MUST make its verification fail, and the injected defects and outcomes MUST
  be recorded. Crash-freedom proofs MUST show that their assumed domain is non-empty and that
  the outcomes they cover are reachable, for example a contained cell discarded and one
  kept, or a decode that
  succeeds and one that fails. At least one injected overflow MUST be shown to make a
  crash-freedom proof fail.

**Non-intrusiveness**

- **FR-009**: The feature MUST NOT change `CellUnion`'s public API. It MUST NOT change its
  observable behaviour either, except for the known-defect fixes in FR-015 and FR-017.
  The deferred formatting of decoder error messages in FR-017 keeps every displayed
  message, so it does not count as a behaviour change. It
  MUST NOT add runtime dependencies to the published library, and it MUST NOT slow down the
  published library. Verification-only code MUST be excluded from normal builds.
- **FR-010**: A defect found by verification MUST be fixed as a separate, recorded change
  under the constitution, with parity checked against upstream (Principle I) and a regression
  test that fails without the fix (Principle III). It MUST NOT be absorbed silently into the
  verification work. The known defects in FR-015 to FR-017 are the exception: the survey
  found them before this feature, and they are fixed inside it. Adding missing "Panics"
  documentation changes no behaviour, so it is done inside this feature.

**Operability**

- **FR-011**: The `CellUnion` proofs MUST run from the project's documented verification
  command, and that command MUST report pass or fail per operation and per property. A
  documented way to run only the `CellUnion` proofs MUST also exist.
- **FR-012**: The `CellUnion` proofs MUST run automatically in CI, through the same command,
  for changes to `CellUnion`, its encoding, `CellId`, or the verification artifacts. They MUST
  stay out of the default quality gate. Proofs over the per-change budget MAY run in 001's
  weekly tier, and their results count like any other proof.
- **FR-013**: The trusted base statement MUST be extended to cover this feature, including:
  - every `CellId` result from 001 that a `CellUnion` proof relies on instead of
    re-proving;
  - any standard-library behaviour (for example sorting) that is modelled or taken as
    given;
  - every bound used.

**Evaluation**

- **FR-014**: The feature MUST produce a written evaluation. It records the effort, the
  verification run time, the bounds reached, the limitations hit and the defects found. It
  MUST also:
  - compare the cost and value with 001;
  - give a go or no-go recommendation for collection types;
  - give a ranked list of candidates for follow-up work.

**Known defects**

- **FR-015**: Denormalising MUST match upstream when rounding up a cell's level would pass
  the maximum level. Today the library panics in both debug and release builds; upstream C++
  clamps to the maximum level. For example, a level-29 cell with minimum level 1 and level
  step 3 panics with "level must be 0..=30", where C++ returns its four level-30 children.
  After the fix, the result MUST match upstream for every cell, every minimum level 0–30 and
  every level step 1–3. The fix MUST:
  - be its own identifiable change;
  - come with a regression test that fails without the fix;
  - be checked by a differential test against committed C++ reference vectors;
  - be listed in the evaluation.

  The Python binding exposes this operation, so it inherits the fix.
- **FR-016**: The leaf-cell count MUST state upstream's precondition. Upstream requires a
  union built verbatim to be valid, and it bounds the count by 6·4^30 leaf cells. The
  library's verbatim constructor documents neither. Its count is a signed 64-bit integer,
  which overflows for an overlapping union of eight or more face cells: debug builds
  panic, and release builds wrap around to a wrong, often negative, count. Upstream
  returns an unsigned count. The feature MUST add both preconditions to the documentation:
  a valid union for the verbatim constructor, and for the leaf-cell count. It MUST also add
  the count's "Panics" documentation and record the signed result type as a documented
  deliberate deviation (Constitution I). A proof MUST show that the count cannot overflow for any valid union
  within the bounds. A written argument MUST extend this to valid unions of any size.
  This changes no behaviour.
- **FR-017**: Decoding a cell union MUST keep the identifiers exactly as encoded, without
  normalising them. It MUST return an error unless the identifiers form a valid union:
  every identifier a valid cell, in strictly increasing order, and no two overlapping.

  Today the library rejects invalid identifiers and then normalises, so four encoded
  sibling cells come back as their parent, and neither behaviour is documented. Upstream
  C++ keeps the identifiers as encoded and validates nothing, not even single identifiers.

  After the fix:
  - every valid union, normalised or not, decodes to exactly the identifiers that were
    encoded;
  - the rejection of input that is not a valid union is documented in the decoder's API
    documentation as a deliberate deviation from upstream (Constitution I). The reason is
    that a decoded union then always satisfies the validity invariant that the other
    operations rely on, even when the bytes are untrusted.

  The fix follows FR-015's four rules. Its differential test checks that every valid union
  in the C++ reference vectors decodes to the same identifiers as in C++. The existing fuzz
  target's round-trip check MUST still hold.

  The decoder's error messages MUST keep their text, but they MUST be formatted only when
  the error is displayed, not while decoding. Formatting a message from decoded values
  puts text formatting into every decoding proof, and that is beyond the checker's reach
  (research R0, R11). This changes no displayed message.

**Untrusted-input coverage (Constitution II)**

- **FR-018**: Decoding a cell union already has bounded robustness tests and a nightly fuzz
  target. The feature MUST keep both, update them for FR-017's behaviour, and add the
  formal crash-freedom proof for decoding (User Story 1, scenario 2). The proof complements
  the tests and fuzzing and does not replace them.

**Upstream findings**

- **FR-019**: If the work shows undefined or wrong behaviour in the upstream reference, the
  finding MUST be noted as a follow-up task in the feature's task list before the affected
  proof is marked complete. The upstream reference means the C++ implementation, the Go and
  Java secondaries, or the reference vectors generated from them. The task records the
  upstream location, the inputs and the observed behaviour. The proof then states the
  behaviour the library documents. Each finding is also listed in the evaluation. The
  feature does not change upstream code.

### Key Entities

- **`CellUnion`**: the verification target. A sequence of cell identifiers that represents
  the region made of those cells.
- **Covered set**: the set of leaf cells that a union covers. It is the reference meaning
  against which every functional property is stated.
- **Validity and normalisation invariants**: the formal statements of when a union is valid
  (sorted, non-overlapping, valid cells) and when it is normalised (valid, with no complete
  group of four siblings).
- **Operation inventory**: each operation listed in FR-002, with its status, the reason for
  any exclusion, its union domain, its bounds and its crash-freedom result.
- **Property catalogue**: the functional properties, each with a statement, an upstream
  source, a bound, a status and its non-vacuity evidence.
- **Bounds**: the union-size, input-size and loop bounds under which each proof holds, with
  the uncovered part and the tests that exercise it.
- **Trusted base statement**: what the proofs rely on without proving it, including the
  `CellId` results reused from 001.
- **Evaluation**: the findings, the comparison with 001 and the recommendation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope operation is proven free of panics, overflow and undefined
  behaviour within its stated bounds. Each bound below is a size limit N: the operation is
  covered for every input of **up to** N cells, and N must be at least the stated value.
  The planning probe measured most of these values within the budget (research R0). R4
  marks three as estimates (intersection with a single cell, union containment and
  intersection, and decoding), and these are measured first during implementation:
  - An operation that only reads one union is covered for every union of up to N cells,
    with N at least 3. These are the membership queries, the validity and normalisation
    checks, the leaf count, the accessors and encoding.
  - Normalising, construction from identifiers and collection from an iterator are
    covered for every input of up to N cells, with N at least 2.
  - Intersection with a single cell is covered for every union of up to N cells, with N
    at least 2.
  - Denormalising is covered for every union of up to N cells, with N at least 1.
  - Union, intersection, and union containment and intersection are covered for every
    pair of unions of up to N cells each, with N at least 1.
  - Decoding is covered for every byte sequence up to the length of an encoded one-cell
    union (17 bytes). If the check still exceeds the budget after the FR-017 message
    change, decoding becomes a tool-limit exclusion, recorded with its measurements.
  - An operation with no union argument covers its whole domain, unless a loop bound is
    documented under FR-007.

  Larger bounds are recorded where they fit. Every behaviour the bounds cannot reach is
  named under FR-007.
- **SC-002**: At least 60% of the operation inventory is in scope. Of the 42 entries, 31
  do not depend on floating-point geometry or on an operation that 001 excluded. Planning
  found 4 of those to be tool limits (FR-002; one of them is measured during
  implementation), which leaves 27 (64%). Every excluded entry
  has a written reason, and every tool-limit exclusion has its measurements.
- **SC-003**: The property catalogue has at least one functional property for each in-scope
  operation, and it covers every area in FR-005. At least 90% of the entries whose
  operations are all in scope are proven, or proven within bounds that meet SC-001. The
  remaining entries are listed with a reason.
- **SC-004**: 100% of proven catalogue properties have recorded evidence of non-vacuity, and
  100% of crash-freedom proofs have satisfied reachability checks.
- **SC-005**: The per-change `CellUnion` proofs complete in under 10 minutes on the standard
  CI machine, a hosted Linux runner with 4 virtual CPUs and 16 GB of memory. The existing
  `CellId` per-change run stays within its 15-minute budget. Proofs over the budget run in
  the weekly tier and finish within its time limit.
- **SC-006**: Apart from the documentation in FR-016 and the behaviour fixes in FR-015 and
  FR-017, the library's public API and behaviour are unchanged. No runtime dependencies are
  added, and the project quality gate passes before and after the feature.
- **SC-007**: A maintainer who did not take part can answer from the evaluation alone:
  "continue or stop?", "with which approach?", "are bounded proofs over collections worth
  it?" and "what next?"
- **SC-008**: After the feature, denormalising matches upstream for every minimum level
  0–30, level step 1–3 and cell level 0–30, with no panic. The regression test fails before
  the fix and passes after it, and the differential test against the C++ vectors passes.
- **SC-009**: The verbatim constructor and the leaf-cell count document upstream's
  validity precondition, and the count documents its panic and its signed result type. A
  proof shows that the count cannot overflow for any valid union within the bounds.
- **SC-010**: After the feature, decoding gives back exactly the encoded identifiers for
  every valid union, including a group of four sibling cells, and returns an error for every
  input that is not a valid union. Its documentation states the deviation from upstream. The
  regression test fails before the fix and passes after it, the differential test against
  the C++ vectors passes, and the robustness tests and fuzz target pass.
- **SC-011**: Every public operation in the inventory that the proofs show can panic
  documents when it panics, and the evaluation lists each documentation addition.
- **SC-012**: Every undefined or wrong upstream behaviour met during the work has a
  follow-up task, created before the affected proof was marked complete, and an entry in the
  evaluation.

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Downstream users benefit through
  stronger guarantees about regions they cover, index and decode.
- **Builds on 001**: the feature reuses 001's verification setup. That covers the reference
  model of the cell hierarchy, input generators, the memory-guarded command runner, the
  mutant campaign, the CI job and the weekly tier. A `CellUnion` proof may rely on a
  `CellId` property that 001 proved instead of re-proving it, and the trusted base records
  each such reliance (FR-013).
- **Assurance level**: exhaustive bounded model checking, as in 001. Its evaluation
  recommended staying at this level. Deductive or kernel-checked proofs that would remove
  the union-size bound are out of scope; the evaluation of this feature may recommend them.
- **Bounds**: the bounds in SC-001 are the ones the planning probe measured, or for three
  of them estimated, to fit the per-harness budget of 4 GB and 12 minutes (research R0,
  R4). Implementation may raise them
  where they fit. Lowering them needs a written justification in the evaluation.
- **Reference semantics**: upstream C++ defines intended behaviour, and Go and Java are
  secondary references (Constitution I). Where the library has an operation with no upstream
  counterpart, such as its own range constructor, the property is derived from the
  mathematical meaning of the covered set. Where the library deliberately deviates and
  documents it, the documented behaviour is what gets verified.
- **Floating-point exclusion**: about 9 inventory entries compute areas or bounds, or
  convert from points on the sphere, and they are excluded. Expanding a union at a level is
  excluded because 001 found the all-neighbours operation intractable. Text display is
  excluded because 001 found the token formatter beyond the checker's limits.
- **Out of scope**: the general-purpose integer encodings of identifier and integer
  vectors are a separate follow-up, even though the 001 evaluation ranked them highly. So
  are derived trait implementations, including the optional serialisation-framework
  support. Validating input in the language bindings (Constitution IV) is also out of
  scope; the bindings inherit the core fixes.
- **Survey findings**: the three findings in FR-015 to FR-017 were confirmed with concrete
  tests on 2026-10-02, before this specification was written.
- **Existing tests stay**: the formal proofs complement the existing unit, property,
  differential, robustness and fuzz tests. No existing test is removed.
