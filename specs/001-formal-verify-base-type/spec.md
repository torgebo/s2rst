# Feature Specification: Formal Verification of a Core Base Type (`CellId`)

**Feature Branch**: `tb/001-formal-verify-base-type`

**Created**: 2026-09-29

**Status**: Draft

**Input**: User description: "Spec 001 Verify one of the base types in core using formal methods
from the research folder"

## Context

s2rst checks correctness today with example tests, property-based tests (about 112K random
inputs across 224 properties), differential tests against the C++ and Go reference
implementations, and fuzzing. All of these sample the input space. None of them can show that
a property holds for **every** input.

`verify/research/rust_formalmethods.md` surveys the formal-verification approaches that are practical
for Rust as of September 2026. They fall into three groups: automated model checking and
SMT-backed verifiers, extraction to a proof assistant (Lean or Coq) for kernel-checked proofs,
and type-level encodings.

This feature is a **pilot**. It applies exhaustive automated model checking to `CellId`, the
64-bit identifier of a cell in the S2 cell hierarchy. `CellId` underlies spatial indexing,
region coverings and the shape index. Almost all of its hierarchy and navigation logic is pure
integer bit manipulation, which makes it the best fit in the core library for complete,
automated proofs. The pilot proves `CellId`'s key properties for all inputs, and it produces
evidence on whether formal verification is worth extending to more of the library.

## Clarifications

### Session 2026-09-30

- Q: Which base type is the verification target? → A: `s2::CellId`. Its integer-only
  hierarchy and navigation operations suit complete automated proof, and it is the most
  depended-upon foundational type. Operations that depend on floating-point geometry are
  excluded.
- Q: How strong must the guarantee be? → A: Exhaustive automated model checking of the compiled
  implementation (for example Kani, from the SMT-backed/model-checking group in the research
  document). It is push-button, reasons about machine arithmetic exactly at the bit level,
  produces concrete counterexamples, and needs no annotations in library code. Deductive proofs
  and kernel-checked proofs are out of scope for this pilot.

### Session 2026-10-01

- Q: A parity defect in token parsing was found while the plan was reviewed. The library
  accepts a token with a leading plus sign: `+1` decodes to the valid cell `0/00`, while
  upstream C++ and Go reject it. Is it fixed separately, or in this feature? → A: In this
  feature, as part of the implementation (FR-015). It is an exception to FR-009 and FR-010.
- Q: Should `edge_neighbors`, `vertex_neighbors` and `all_neighbors` stay in the verification
  scope, even though they use floating-point geometry when a neighbour lies across a
  cube-face edge? → A: Yes. Keep them in scope and prove the face-crossing path too. Exclude
  them as floating-point only if even crash-freedom proves intractable.
- Q: What should count as proof that a crash-freedom check isn't passing vacuously: its own
  injected defect, or evidence that its assumed inputs can actually occur? → A: Evidence that
  its inputs can occur. Crash proofs need satisfied cover checks plus one demonstration that
  an injected overflow is caught. They are kept apart from the property catalogue, so SC-003
  and SC-004 count functional properties only.
- Q: If a proof can cover only part of an operation's inputs for a reason other than loop
  length (for example, only ASCII text), should the operation count as "proven within stated
  bounds" or be excluded from scope? → A: Excluded. SC-001 stays strict: only an
  input-dependent loop bound may leave part of a domain uncovered. Any other restriction
  makes the operation a tool-limit exclusion, justified in the pilot evaluation.
- Q: The token and debug-string parsers take untrusted text. Constitution II requires such
  APIs to return a `Result` and to have bounded robustness tests and a fuzz target. How should
  this feature handle that? → A: Add the bounded robustness tests and a fuzz target for the
  three text parsers in this feature (FR-017). Record a justified exception only for their
  return types, which follow upstream (Principle I) and cannot change without breaking the
  public API (FR-009).
- Q: For an invalid id, C++ `ToString` prints the hex value zero-padded to 16 digits, while
  the Rust debug string and `Display` print it unpadded. Should this feature fix it too, as it
  does the token defect? → A: Yes. Fix it in this feature as a second known defect (FR-016),
  under the same rules as FR-015.
- Q: The planning probe showed that the three formatters (token, debug string and standard
  display) cannot be checked within the tool's limits, so they become tool-limit exclusions.
  Their catalogue properties then cannot be proven, and the text parsers can only be proven
  within a length bound. That leaves SC-003's 90% out of reach as written. How should SC-003
  count? → A: Only properties whose operations are all in scope count towards the 90%.
  Properties that exercise an excluded operation stay in the catalogue with their "not
  proven" status and reason, and the evaluation reports them.
- Q: Should a proof that covers every input up to a stated size (for example every text of at
  most 17 or 33 bytes) count as "proven within stated bounds", even when the operation has no
  loop over the larger inputs? → A: Yes. A bound on input size (the length of a text or
  collection) counts the same as a loop bound, provided the bound is stated, the uncovered
  inputs are documented, and ordinary tests or fuzzing exercise them. Any other domain
  restriction is still a tool-limit exclusion.
- Q: How should the two parity fixes (FR-015, FR-016) be checked against the C++ reference
  implementation? → A: With differential tests. The C++ reference-vector generator gains
  inputs for both behaviours: sign-prefixed, non-hexadecimal, empty and over-long tokens, and
  the debug string of invalid identifiers. The committed vectors are regenerated, and the
  ordinary test suite compares the library against them. That is in addition to the
  regression tests.
- Q: When the proofs show that a public operation can panic but its documentation has no
  "Panics" section, should this feature add the missing documentation or only record it? →
  A: Add it in this feature, as one separate documentation-only change listed in the
  evaluation. A panic that also differs from upstream behaviour is documented the same way,
  and the behaviour difference is recorded as a defect for a separate fix (FR-010).
- Q (raised by the maintainer): What happens when the verification work shows that the
  upstream reference itself has undefined or wrong behaviour? → A: Note it as a follow-up
  task before the affected proof is completed (FR-018).
- Q: Which machine should SC-005's "standard CI machine" be? → A: A GitHub-hosted
  `ubuntu-latest` runner with 4 vCPUs and 16 GB, the public-repository size. On a smaller
  runner, the job derives its parallelism from the cores and memory it actually has, and
  the evaluation reports the time on both.
- Q (raised during implementation): `vertex_neighbors`' crash proof is tractable (about
  11 minutes) but exceeds the per-change budget in every split tried, and the neighbour
  properties do too. How should the pilot proceed? → A: A weekly slow tier. Harnesses over
  the per-change budget run only in the weekly scheduled CI job, through the same command
  runner, with a larger time limit (30 minutes per harness). The per-change run (SC-005)
  covers the rest. Results from either tier count the same for SC-001 to SC-004.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven absence of crashes for `CellId` (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope `CellId` operation
completes without panicking, overflowing, or hitting undefined behaviour for **every** input in
its valid domain. Then the most foundational guarantee for the cell hierarchy no longer depends
on which random inputs the tests happened to try.

**Why this priority**: Panic-freedom is the property the constitution already requires
(Principle II). It is also the cheapest to prove and the easiest for a newcomer to understand.
It delivers value alone, even if no functional properties are proven.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports every in-scope operation as proven free of panics, overflow, and undefined
behaviour, with no unproven obligations remaining inside each operation's stated bounds
(FR-007).

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification command,
   **Then** it reports each in-scope operation by name as proven crash-free and exits
   successfully.
2. **Given** an operation with a documented precondition (for example "must be a valid cell" or
   "must not be a leaf cell"), **When** verification runs, **Then** the proof assumes exactly
   that precondition and no stronger one, and the report states which precondition was assumed.
3. **Given** an in-scope operation deliberately changed so that it can overflow for some input,
   **When** verification runs, **Then** it fails and names the operation and a concrete cell
   identifier (and any other argument) that triggers the failure.

---

### User Story 2 - Proven functional correctness properties (Priority: P2)

As the maintainer, I want a catalogue of `CellId`'s key functional properties, each proven for
all inputs in its domain. The catalogue covers validity preservation, the hierarchy laws
(parent, children, levels), range and containment laws, traversal laws, and encoding round
trips. Each property is stated independently of the code and traced to upstream S2 semantics.
A proof then shows that the implementation agrees with the reference behaviour. That extends
the parity guarantee (Principle I) from sampled inputs to all inputs.

**Why this priority**: This is where formal methods add the most over the existing tests. It
depends on the scaffolding from Story 1, and it takes more effort.

**Independent Test**: Run the verification command. Every property in the catalogue is
reported as proven, or it is explicitly marked as not proven with a written reason. Each proven
property also fails when a matching defect is injected into the implementation.

**Acceptance Scenarios**:

1. **Given** the property catalogue, **When** verification runs, **Then** every property marked
   "proven" is discharged with no remaining obligations.
2. **Given** any proven property, **When** a defect that violates it is introduced into the
   implementation, **Then** verification fails for that property. This shows the proof is not
   vacuous.
3. **Given** any property, **When** a reviewer reads the catalogue, **Then** they find a
   plain-language statement, the upstream source it is derived from, and its proof status.

---

### User Story 3 - Regression protection for future changes (Priority: P3)

As a contributor changing `CellId`, I want the proofs to run automatically in CI. If my change
breaks a proven property, I am told which property broke and given a concrete counterexample,
before the change is merged.

**Why this priority**: Proofs that are not re-checked go stale. This story protects the
investment made in Stories 1 and 2, but it only matters once those exist.

**Independent Test**: Open a change that breaks one proven property. The automated verification
check fails and names the property. Revert the change and the check passes.

**Acceptance Scenarios**:

1. **Given** a proposed change to `CellId` or to the verification artifacts, **When** CI runs,
   **Then** the verification check runs and its pass or fail result appears on the change.
2. **Given** a change that does not touch `CellId` or the verification artifacts, **When** CI
   runs, **Then** the existing quality gate is not slowed down or made to depend on
   verification toolchains.

---

### User Story 4 - Pilot evaluation and recommendation (Priority: P3)

As the maintainer, I want a short written evaluation of the pilot. It should cover the effort
it took, what worked, what did not, what is trusted but not proven, and any defects found. With
that I can decide whether to extend formal verification to more types, and with which approach.
For example, the evaluation should say whether a follow-up should add deductive or
kernel-checked proofs.

**Why this priority**: This is the decision-making output of a pilot. It is written last, but
without it the pilot's lessons are lost.

**Independent Test**: A maintainer who did not do the work reads the evaluation and can answer
three questions without asking anyone: "should we continue?", "with which approach?", and "which
types next?"

**Acceptance Scenarios**:

1. **Given** the completed pilot, **When** the evaluation is read, **Then** it contains a clear
   go or no-go recommendation, the rationale, and a ranked list of candidate types for any
   follow-up.

---

### Edge Cases

- **Hierarchy boundaries**: face cells (level 0) have no parent, and leaf cells (level 30) have
  no children. The proofs must show that operations documented for those cases behave as
  documented, whether as a precondition or a defined result.
- **Invalid identifiers**: the identifier is a public 64-bit value, so any bit pattern can be
  built. That includes zero (`none`), all ones (`sentinel`), face numbers 6 and 7, and a
  lowest set bit in an odd position. For each operation, the proof must state whether its
  domain is every bit pattern or only valid cells, and why. Operations documented to accept any
  value, such as the validity check itself, are proven over every bit pattern.
- **Past-the-end and sentinel values**: the value one past the last cell at a level (`end`) and
  the `sentinel` value are not valid cells. Upstream still passes them to comparison, range and
  traversal operations during iteration. The domain of each such operation must include them
  wherever upstream usage does.
- **Wrap-around**: stepping past the last cell of face 5 or before the first cell of face 0. The
  wrapping and clamping variants must be proven to wrap or clamp exactly as documented.
- **Large step counts**: stepping along the curve by counts near the minimum and maximum 64-bit
  signed values, where intermediate arithmetic could overflow even though the result is
  clamped or wrapped into range.
- **Level arguments**: a value of the level type is always in `0..=30`. Operations also accept
  raw integers that convert to a level. That conversion panics for values above 30, as its own
  documentation states, and it runs inside the operation before any cell logic. The proofs take
  level arguments as level values, and the inventory records the conversion panic for every
  operation that takes a level. Level arguments that are valid but inconsistent with the cell,
  such as asking for a "parent" at a finer level than the cell itself, are governed by each
  operation's documented precondition.
- **Text encodings**: empty, over-long, non-hexadecimal, and non-ASCII tokens and debug strings
  must be rejected as documented, without crashing. That includes a token with a leading sign
  character, which upstream rejects (FR-015).
- **Debug-only checks**: several operations guard their precondition with a check that is
  active only in debug builds. Each must be proven safe for the domain that its check guards,
  and it must not rely on the check firing in release builds.
- **Input-dependent loop lengths and input sizes**: listing all neighbours at a finer level
  loops about 2^(level gap) times, and a text argument can be arbitrarily long. Where
  exhaustive checking can cover such an operation only up to a loop bound or an input-size
  bound, the bound must be stated and the part left uncovered must be documented (FR-007).

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The pilot MUST verify the `CellId` type of the core library.
- **FR-002**: The verification artifacts MUST include an operation inventory. It lists every
  inherent public operation of `CellId`, the public module-level functions that construct a
  `CellId` or compute its level-derived constants, and its hand-written trait implementations.
  Each entry is marked **in scope** or **excluded**, with a written reason for each exclusion.
  Operations whose results depend on floating-point geometry (conversions to and from points,
  latitude/longitude, and (s,t)/(u,v) coordinates and bounds) are excluded by default. An
  operation that returns cells but uses floating-point geometry internally stays in scope:
  the neighbour operations use it to cross a cube-face edge. The checker reasons about
  floating-point arithmetic bit for bit, so its proofs stay exact. Such an operation is
  excluded as floating-point only if even its crash-freedom proves intractable, and that
  exclusion is justified in the pilot evaluation. Any other exclusion MUST also be justified
  in the pilot evaluation.

**Guarantees**

- **FR-003**: For every in-scope operation, verification MUST establish, for **all** inputs in
  its documented domain, that the operation completes without panic, arithmetic overflow, or
  undefined behaviour. The domain of a cell argument is either every bit pattern or the valid
  cells, extended with past-the-end and sentinel values where upstream usage requires them.
  Every other argument (level, face, curve position, step count, (i,j) coordinate, or text)
  ranges over all values of its type, narrowed only by a documented precondition. A
  precondition counts as documented if the library's API documentation states it, a
  debug-build check guards it, or the upstream reference states it for the same operation. If
  a proof must assume a precondition that the library's own documentation does not state, the
  undocumented panic MUST be recorded as a finding in the inventory and in the pilot
  evaluation. The feature MUST also add the missing "Panics" documentation to that operation,
  in one separate documentation-only change (Constitution II). If the panic also differs
  from upstream behaviour, the behaviour difference is a defect under FR-010.
- **FR-004**: `CellId`'s validity invariant MUST be stated formally: a face number 0–5 followed
  by a path of 2-bit child positions, terminated by a single marker bit in an even position.
  Verification MUST establish that every in-scope constructor and every in-scope operation that
  returns a `CellId` maps inputs in its domain to valid cells, except where upstream S2
  documents a different result (for example a past-the-end value).
- **FR-005**: The verification artifacts MUST include a property catalogue of functional
  properties covering at least the following areas:
  - **Validity**: characterisation of valid identifiers, and preservation of validity.
  - **Hierarchy**: parent and child levels, ancestors at a given level, and the four children
    exactly partitioning their parent in curve order.
  - **Ranges and containment**: range bounds, containment agreeing with range inclusion, and
    any two valid cells being either nested or disjoint.
  - **Traversal**: next and previous stepping being inverse, stepping by n agreeing with n
    single steps, and clamping and wrapping behaviour.
  - **Encoding round trips**: face/(i,j) coordinates, hex tokens, and debug strings.
  - **Common ancestor**: the common ancestor level is the finest level at which both cells
    share an ancestor.

  Each entry MUST record a plain-language statement, the upstream S2 source (behaviour or
  documentation) it is derived from, and a status of **proven**, **proven within stated
  bounds**, or **not proven (reason)**. **Proven** means the whole domain is covered. That
  includes a proof whose loop bound is shown to cover the whole domain (FR-007). **Proven
  within stated bounds** means an input-dependent loop bound or an input-size bound (FR-007)
  leaves part of the domain uncovered, and the uncovered part is documented.
- **FR-006**: Properties MUST be expressed against a reference specification written from
  upstream S2 semantics or the underlying mathematical definition of the cell hierarchy. They
  MUST NOT restate the implementation's own code. A proof then establishes agreement between two
  independent descriptions (see Constitution Principle I).
- **FR-007**: Verification MUST be exhaustive and automated. For every in-scope operation and
  property, the checker MUST explore all inputs in the stated domain, not a sample, with exact
  bit-level machine-arithmetic semantics. It MUST produce a concrete counterexample on failure.
  Where an operation contains a loop whose iteration count depends on its input, the proof MUST
  state the bound used. It MUST either show that the bound covers the whole domain (for example,
  at most 30 iterations, one per level) or document the uncovered part. Where an argument has
  no fixed size (a text or a collection), the proof MAY cover every input up to a stated size,
  as an input-size bound. The bound MUST be stated, the uncovered inputs MUST be documented,
  and ordinary tests or fuzzing MUST exercise them. A loop bound or an input-size bound is the
  only accepted reason to leave part of a domain uncovered. If the checker can only handle a
  narrower domain for any other reason (for example, only ASCII text), the operation MUST be
  excluded as a tool limit (FR-002) rather than reported as proven within bounds.
- **FR-008**: Every proven catalogue property (FR-005) MUST be shown to be non-vacuous: at
  least one deliberately injected defect in the implementation MUST cause that property's
  verification to fail. The injected defects and their outcomes MUST be recorded.
  Crash-freedom proofs (FR-003) are not catalogue properties and need no defect of their own.
  Instead, each MUST show that its assumed domain is non-empty and that the outcomes it covers
  (for example the wrapping and non-wrapping cases) are reachable, so that over-constrained
  assumptions cannot make it pass vacuously. At least one injected overflow MUST be shown to
  make a crash-freedom proof fail (User Story 1, scenario 3).

**Non-intrusiveness**

- **FR-009**: The pilot MUST NOT change `CellId`'s public API. It MUST NOT change its
  observable behaviour either, except for the known-defect fixes in FR-015 and FR-016. It
  MUST NOT add runtime dependencies to the published library, and it MUST NOT slow down or
  otherwise degrade the published library. Any verification-only code MUST be excluded from
  normal builds of the library.
- **FR-010**: If verification finds a defect, the fix MUST be handled as a separate, recorded
  change under the constitution: parity with upstream checked (Principle I), and a regression
  test added to the ordinary test suite (Principle III). The fix MUST NOT be absorbed silently
  into the verification work. The exceptions are the two known defects in FR-015 and FR-016,
  which were found before implementation and are fixed inside this feature. Adding missing
  "Panics" documentation (FR-003) changes no behaviour, so it is not a defect fix under this
  rule, and it is done inside this feature.

**Operability**

- **FR-011**: Verification MUST run from a single documented command at the repository root. It
  MUST report pass or fail per operation and per property.
- **FR-012**: Verification MUST run automatically in CI, through the same command, at least for
  changes that touch `CellId` or the verification artifacts. It MUST be kept out of the existing
  default quality gate if it needs toolchains that the gate does not already require. Proofs
  that exceed the per-change budget MAY run in a weekly scheduled tier instead, with a larger
  time limit per proof; they still run automatically, and their results count like any other
  proof.
- **FR-013**: The verification artifacts MUST document the trusted base: the checker and its
  solver backend, any models or stand-ins used for standard-library behaviour, every assumption
  and bound, and every behaviour that is left unproven.

**Evaluation**

- **FR-014**: The pilot MUST produce a written evaluation. It records the effort spent,
  verification run time, the limitations hit, and the defects found. It MUST also give a go or
  no-go recommendation, state whether a follow-up should add a stronger assurance level from
  `verify/research/rust_formalmethods.md` (deductive or kernel-checked proofs), and give a ranked list
  of candidate types for follow-up work.

**Known defects**

- **FR-015**: The feature MUST fix the known parity defect in token parsing. Today a token with
  a leading plus sign is accepted: `+1` decodes to the valid cell `0/00`. Upstream C++ and Go
  reject such a token and return the invalid `none` identifier. After the fix, a token with any
  character that is not a hexadecimal digit MUST decode to `none`, exactly as upstream does. No
  other behaviour may change. The fix MUST:
  - be its own identifiable change within the feature, not mixed into verification work;
  - come with a regression test in the ordinary test suite that fails without the fix
    (Principle III);
  - be checked against the upstream C++ implementation by a differential test. That test
    uses committed reference vectors that the project's C++ reference generator produces for
    sign-prefixed, non-hexadecimal, empty and over-long tokens (Principle I). Generating the
    vectors may need a C++ toolchain; running the test MUST NOT;
  - be listed in the pilot evaluation as a defect found, with its upstream reference.
- **FR-016**: The feature MUST fix the known parity defect in the debug string of an invalid
  identifier. Upstream C++ prints `Invalid: ` followed by the value as 16 zero-padded
  hexadecimal digits. The library prints it without padding, for example `Invalid: 2` instead
  of `Invalid: 0000000000000002`. The standard display of a cell uses the debug string, so it
  changes the same way. After the fix both MUST match upstream for every invalid identifier.
  Valid identifiers are unaffected, and no other behaviour may change. The fix follows the
  same four rules as FR-015: its own change, a regression test that fails without it, a
  differential test against C++ vectors for invalid identifiers, and a listing in the
  evaluation.

**Untrusted-input coverage (Constitution II)**

- **FR-017**: The three text parsers (from a token, from a debug string, and the standard
  string-parsing conversion) MUST gain the robustness coverage that Constitution II requires
  of APIs that take untrusted input. Unlike the formal proofs, this coverage must not depend
  on how tractable text is for the checker:
  - bounded robustness tests alongside the existing decoder robustness tests. They cover
    empty, over-long, non-hexadecimal, sign-prefixed and non-ASCII input, and check that the
    parsers return the documented "not a cell" result without panicking;
  - a target in the nightly fuzzing campaign that feeds arbitrary bytes, valid as text, to
    all three parsers, added to the campaign's list of targets.

  The parsers keep their current return types: an identifier, an optional value, and a
  standard error result. Those types follow upstream (Principle I), and changing them would
  break the public API (FR-009). The departure from Constitution II's "MUST return a `Result`"
  rule MUST be recorded as a justified exception in the plan.

**Upstream findings**

- **FR-018**: If the verification work shows undefined or wrong behaviour in the upstream
  reference, the finding MUST be noted as a follow-up task in the feature's task list
  before the affected proof is marked complete. The upstream reference means the C++
  implementation, the Go and Java secondaries, or the reference vectors generated from them.
  An example is behaviour that C++ only assumes away, or a result that contradicts upstream's
  own documentation. The task records the upstream location, the inputs, and the observed
  behaviour. The proof then states the behaviour the library documents, under FR-003 and
  Principle I. Each such finding is also listed in the pilot evaluation. The pilot does not
  change upstream code.

### Key Entities

- **`CellId`**: The verification target. It is a 64-bit identifier made of a face number, a
  path of child positions in Hilbert-curve order, and a trailing marker bit that encodes the
  level.
- **Operation Inventory**: The operations that FR-002 lists, each with an in-scope or
  excluded status, a reason for any exclusion, and the result of its crash-freedom proof.
- **Validity Invariant**: The formal statement of which 64-bit values are valid cells.
- **Reference Specification**: A description of the cell hierarchy's intended behaviour that
  is independent of the implementation and derived from upstream S2 and the underlying
  mathematics.
- **Property Catalogue**: The set of functional properties. Crash-freedom proofs are not part
  of it; their results are recorded in the operation inventory. Each property has a
  statement, an upstream source, a proof status, and the non-vacuity evidence for it.
- **Trusted Base Statement**: What the verification relies on without proving it.
- **Pilot Evaluation**: The written findings and recommendation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope operation is proven free of panics, overflow, and undefined
  behaviour. For at least 90% of in-scope operations, the proof covers the entire documented
  domain. The remainder (only operations with input-dependent loop lengths or unbounded input
  sizes) are proven within a stated bound, and the uncovered part is documented. An operation
  whose proof needs any other domain restriction is excluded as a tool limit (FR-007), not
  counted here.
- **SC-002**: At least 75% of the operation inventory is in scope. At the time of writing, about
  52 of about 64 entries do not depend on floating-point geometry. Every excluded entry has a
  written reason.
- **SC-003**: The property catalogue has at least one functional property for each in-scope
  operation, and it covers every area listed in FR-005. At least 90% of the catalogue entries
  whose operations are all in scope have the status "proven", which means the whole domain
  is covered (FR-005). Entries that exercise an excluded operation stay in the catalogue with
  status "not proven" and the reason, and do not count towards the 90%.
- **SC-004**: 100% of proven catalogue properties have recorded evidence of non-vacuity: at
  least one injected defect that the verification catches. 100% of crash-freedom proofs have
  satisfied reachability checks.
- **SC-005**: A full per-change verification run (every proof except the weekly slow tier,
  FR-012) completes in under 15 minutes on the project's standard CI machine, so it can run on
  every relevant change. The weekly slow tier completes within its CI job's time limit. The standard CI machine is a hosted Linux
  runner with 4 virtual CPUs and 16 GB of memory. If the project's runners are smaller, the
  run adapts its parallelism to them, and the evaluation reports both times.
- **SC-006**: The pilot changes zero lines of the public API and adds zero runtime dependencies
  to the published library. Apart from the known-defect fixes (FR-015, FR-016), the library's
  observable behaviour is unchanged. The existing project quality gate passes unchanged, both
  before and after the pilot is merged.
- **SC-007**: The pilot evaluation lets a maintainer who did not take part answer "continue or
  stop?", "with which approach?" and "which type next?" without asking anyone.
- **SC-008**: After the pilot, the token `+1`, and every other token that contains a
  character that is not a hexadecimal digit, decodes to the invalid identifier, matching
  upstream. The regression test for this fails on the code before the fix and passes after
  it, and the differential test against the C++ vectors passes.
- **SC-009**: After the pilot, the debug string and standard display of every invalid
  identifier match upstream: `Invalid: ` plus 16 zero-padded hexadecimal digits. The
  regression test for this fails on the code before the fix and passes after it, and the
  differential test against the C++ vectors passes.
- **SC-010**: All three text parsers have bounded robustness tests that pass in the ordinary
  test suite, and a fuzz target that runs in the nightly fuzzing campaign. The plan records
  the return-type exception to Constitution II.
- **SC-011**: After the pilot, every public operation in the inventory that the proofs show
  can panic documents when it panics, and the evaluation lists each one added.
- **SC-012**: Every undefined or wrong upstream behaviour met during the pilot has a
  follow-up task, created before the affected proof was marked complete, and an entry in the
  evaluation.

## Assumptions

- **Audience**: The primary users are the s2rst maintainer and contributors. Downstream users
  benefit indirectly, through stronger guarantees about the cell hierarchy they depend on.
- **Pilot scope**: Only `CellId` is verified in this feature. Extending verification to other
  types, or adding deductive or kernel-checked proofs, is follow-up work. The pilot evaluation
  (FR-014) informs it.
- **Reference semantics**: Upstream C++ S2 defines intended behaviour, and Go and Java are
  secondary references (Constitution Principle I). Where the Rust port deliberately deviates
  and documents the deviation, the documented behaviour is what gets verified.
- **Precondition domains**: Some operations document a precondition, for example "must be a
  valid cell" or "must not be a leaf", enforced only by checks in debug builds. For those,
  proofs assume the precondition. Level arguments are taken as values of the level type, and
  that type's own invariant (`0..=30`) is taken as given. Converting a raw integer above 30
  panics in the conversion, as documented there (see Edge Cases).
- **Floating-point exclusion**: About 12 inventory entries convert to or from points on the
  sphere, latitude/longitude, or planar (s,t)/(u,v) coordinates and bounds. They depend on
  floating-point geometry computed outside `CellId`, and they are excluded from this pilot.
  Their integer-only building blocks, such as a cell's centre expressed in integer coordinates,
  stay in scope. So do the neighbour operations (edge, vertex and all neighbours), which
  return cells and use floating-point geometry only to cross a cube-face edge, unless even
  their crash-freedom proves intractable (FR-002). Planning found `all_neighbors`
  intractable, so it is excluded; the other two stay (research R12).
- **Toolchains**: The checker may need toolchains that the ordinary build does not use, such as
  a specific compiler version or a solver backend. Like the existing fuzzing setup,
  verification may live in its own isolated area of the repository. It keeps its own CI job,
  invoked through the project's usual command runner (Constitution: CI calls the same targets as
  local runs).
- **Existing tests stay**: Formal proofs complement the existing unit, property, differential,
  and fuzz tests. They do not replace them, and no existing test is removed.
- **Research source**: The approach is exhaustive model checking, as described in
  `verify/research/rust_formalmethods.md`. Version numbers quoted there are to be confirmed at planning
  time, as the document itself advises.
