# Specification Quality Checklist: Formal Verification of `r2::Rect` and `Point`

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-13
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

- Validation run 1 (2026-10-13), for `r2::Rect` and `ChordAngle`: every item passed except one
  open marker, the chord angle's validity.
- Validation run 2 (2026-10-13): the maintainer deferred `ChordAngle` ("skip this type, make a
  note to pick it up later"). The spec now covers `r2::Rect` and `Point`, the next type in 005
  FR-016's order. `ChordAngle`'s findings and its open question stay in the Context, and
  FR-025 keeps the deferral in the evaluation. Every item passes, with no marker.
- **Defaults taken without a marker**, following the precedents:
  - **K5** is a library-only defect, so it is fixed to the documented behaviour (FR-020).
  - **K6** is wrong in upstream for valid input, so the library returns the correct value as
    a documented deviation, as for U-007, U-009 and U-010. Where upstream is right, parity is
    restored (FR-021).
  - **K7 and K8** are limits that upstream shares, so they are documented, not changed, as
    005 did for the real interval's infinite corners (FR-022).
  - **K10** follows C++, the canonical reference, where Go differs, so it is documented as a
    deviation from Go, not changed (FR-022).
  - **K9** is a build configuration that no published package uses, so it is stated in the
    trusted base and left out of scope.
- **The deserialisation fix (FR-019)** follows the precedent of `CellUnion`, `Cell` and
  `s2::Rect`. A point has no validity rule, so its deserialisation stays as it is.
- **Implementation details**: the spec names the two types, their operations, IEEE-754 and
  the C++ vectors because they are the subject of the feature, as in 001–005. It names no
  verification tool, and describes the checker and the models by what they guarantee.
- **Technology-agnostic success criteria**: SC-005 names the CI machine's size, and SC-011 the
  C++ vectors, as 001–005 did.
- Validation run 3 (2026-10-13, at planning): FR-004's point domain was narrowed for the
  arithmetic that can produce NaN, as 005's planning probe found for the real interval, and
  K10 was added from re-reading Go. Every item still passes.
- Validation run 4 (2026-10-14, after the planning probe): FR-004 gained the tool limit for
  `from_coords` and `normalize` (the checker reports `1 / ∞` as NaN); K11 was added, with
  FR-022 and FR-024 (U-013); the orthogonal directions' laws are stated as values. K11 takes
  the documentation default, as K10 did. Every item still passes.
