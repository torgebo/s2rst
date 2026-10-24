# Specification Quality Checklist: Formal Verification of a Core Base Type (`CellId`)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-29
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- Iteration 1 (2026-09-29): two [NEEDS CLARIFICATION] markers were open.
  - FR-001: the target type.
  - FR-007: the assurance level.
- Iteration 2 (2026-09-30): both markers were resolved by the user and recorded under
  "Clarifications / Session 2026-09-30".
  - Q1: A, `s2::CellId`.
  - Q2: A, exhaustive automated model checking.
  The edge cases, FR-002 to FR-007, SC-001 to SC-003 and the assumptions were made specific to
  `CellId`, based on the actual operation set in the core library:
  - The inventory has about 64 entries, and about 12 of them depend on floating-point geometry.
  - Past-the-end and sentinel values are legitimate inputs.
  - Listing all neighbours has an input-dependent loop length.
  All items pass.
- "Non-technical stakeholders": the stakeholders of a geometry library are its maintainers and
  contributors. The spec is written for that audience. Domain terms (cell, level, face, panic,
  overflow) are kept in plain language.
- "No implementation details": the requirements name no verification tool, file layout or
  command. The research document's tools are mentioned only in the Context section and in the
  recorded clarification answer ("for example Kani"). The concrete tool is chosen and version
  pinned at `/speckit-plan` time.
- SC-002 counts ("about 52 of about 64") are approximate at time of writing. The exact
  inventory is a deliverable (FR-002). The 75% threshold leaves room for a few justified
  exclusions caused by tool limits, for example on string formatting.
- Iteration 3 (2026-10-01): consistency review against the plan artifacts, the constitution
  and `core/src/s2/cell_id.rs`. The inventory counts were confirmed exactly: 54 methods, 3 free
  functions and 7 hand-written trait impls (64), of which 12 are floating-point (52 in scope).
  Spec changes:
  - Edge Cases and Assumptions: level arguments *can* be out of range as raw integers. The
    conversion to the level type panics above 30, inside the operation.
  - FR-003: domains for non-cell arguments. A "documented precondition" can come from the API
    docs, a debug-build check, or upstream. An undocumented panic is recorded as a finding.
  - FR-005 and SC-003: "proven" vs "proven within stated bounds" defined without overlap.
  - FR-008: crash-freedom proofs must show that their domain is non-empty and that their
    outcomes are reachable.
  - Key Entities: the inventory is defined by FR-002, not by "every public operation".
  - Header: the branch name now matches the git branch.
  All items still pass.
- Iteration 4 (2026-10-01): the user decided to fix the `from_token` parity defect (a leading
  `+` is accepted) in this feature. It is recorded under "Clarifications / Session
  2026-10-01". Spec changes:
  - New FR-015 and SC-008.
  - FR-009, FR-010 and SC-006 name FR-015 as their single exception.
  - The text-encoding edge case mentions the sign character.
  The plan, research (new R11), quickstart and data model were updated to match. The plan
  records the focused-PR exception in Complexity Tracking. All items still pass.
- Iteration 5 (2026-10-01, during `/speckit-plan`): a memory-capped Kani probe showed that the
  three formatters are intractable and that the text parsers can only be proven within a
  length bound. SC-003's 90% was then out of reach as written. The user decided that only
  properties whose operations are all in scope count. That decision is recorded under
  "Clarifications / Session 2026-10-01", and SC-003 was amended. The plan records the other
  decision made with the probe, one audited `unsafe` helper in `verify/`, in research R6 and
  Complexity Tracking. All items still pass.
