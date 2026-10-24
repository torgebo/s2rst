# Specification Quality Checklist: Formal Verification of `r1::Interval` and `s2::Rect`

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-12
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

- Validation run 1 (2026-10-12): all items pass.
- Amended during `/speckit-plan` (2026-10-12, research R0):
  - K1 and FR-017 now include approximate equality and the directed Hausdorff distance at
    infinite bounds;
  - FR-004 restricts the arithmetic operations to the pairs where `∞ − ∞` cannot arise;
  - the checker counts a NaN produced by arithmetic as a failed check.

  Re-checked: all items still pass.
- **The two types** were chosen from the evidence, without a clarification marker:
  - 004's FR-016 names the latitude interval and the latitude–longitude rectangle first;
  - the pilot passed for both;
  - the rectangle is the product of 004's circular interval and this feature's real interval;
  - D-006 shows the rectangle has real defects.

  The runners-up and the reasons are in the Context.
- **K4 (FR-020)** follows the precedent of the same type's binary decoder and of
  `CellUnion` and `Cell`, rather than asking. 004 left the circular interval's
  deserialisation as it is, and the Assumptions say why the cases differ.
- **Implementation details**: the spec names the two types, their operations, IEEE-754 and
  the C++ vectors because they are the subject of the feature, as in 001–004. It names no
  verification tool, and describes the checker and the models by what they guarantee.
- **Technology-agnostic success criteria**: SC-005 names the CI machine's size, and SC-011 the
  C++ vectors, as 001–004 did.
- Amended during `/speckit-tasks` (2026-10-13): FR-017 also lists the centre of `[−∞, ∞]`,
  which is NaN (read from the code; upstream computes the same). Re-checked: all items pass.
