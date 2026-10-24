# Specification Quality Checklist: Formal Verification of `CellUnion`

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-02
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

- Iteration 1 (2026-10-02): one [NEEDS CLARIFICATION] marker is open.
  - FR-017: how decoding a cell union should behave. Upstream keeps the identifiers as
    encoded; the library normalises them, without documenting it.

  The other two survey findings did not need a question:
  - FR-015 (`denormalize` panics) has one parity answer: clamp as C++ does.
  - FR-016 (leaf-cell count overflow) is outside upstream's contract. C++ requires a valid
    union for `FromVerbatim` and bounds the count by 6·2^60. Documenting that precondition
    needs no API change.
- Inventory counts in SC-002 are approximate. The source has 29 inherent public methods
  and 13 hand-written trait methods: slice access, display, collect, two iterator impls,
  six region methods, encode and decode. Of those 42:
  - 9 are floating-point: point containment (twice), expansion by a radius, three areas,
    and the cap, rectangle and covering bounds;
  - expansion at a level depends on `all_neighbors`, which 001 excluded;
  - display goes through the token formatter, which 001 excluded.

  That leaves 31 in scope (74%). The 70% threshold left room for one or two tool-limit
  exclusions found during planning. Superseded by iteration 3.
- "No implementation details": the requirements name no verification tool, file layout or
  command. Debug builds, signed 64-bit counts and the Python binding are named because they
  are the observable behaviour that the known-defect requirements describe. 001 accepted
  the same choice.
- "Non-technical stakeholders": as in 001, the stakeholders of a geometry library are its
  maintainers and contributors. Domain terms (cell, union, level, leaf cell, normalise) are
  kept in plain language.
- SC-001's minimum bounds are informed defaults: the smallest sizes that reach a sibling
  merge and an interleaving of two unions. The Assumptions section says that planning may
  raise them, and that lowering them needs a written justification.
- Upstream contracts were checked against `s2geometry/src/s2/s2cell_union.{h,cc}`:
  - which results are normalised, and when (FR-004);
  - the `FromVerbatim` validity requirement and the `LeafCellsCovered` bound (FR-016);
  - `Denormalize`'s clamp to `kMaxLevel` (FR-015);
  - `Decode` keeps identifiers verbatim (FR-017).

  `S2CellId::Decode` (`s2cell_id.cc:261`) does not validate the id either.
- Iteration 2 (2026-10-02): the user answered the FR-017 question with "Verbatim, reject
  invalid". Decoding keeps the identifiers exactly as encoded and returns an error unless
  they form a valid union, as a documented deliberate deviation. This is recorded under
  "Clarifications / Session 2026-10-02". Spec changes:
  - FR-017 and SC-010 are rewritten.
  - The FR-005 Encoding area now reads "a successful decode returns a valid union".
  - The decoding edge case lists what must be rejected.
  - FR-007 and SC-001: the four-cell minimum applies to every operation whose documented
    behaviour involves a group of four siblings, not only to normalising. That includes
    the normalisation check.

  All items pass.
- Iteration 3 (2026-10-02, during `/speckit-plan`): a memory-capped Kani probe (research
  R0) showed that the SC-001 minimum bounds cannot be met within the per-harness budget
  for operations that build a union. The user chose "Kani at measured bounds"; the
  decision is recorded under "Clarifications / Session 2026-10-02". Spec changes:
  - **FR-002**: four measured tool-limit exclusions: `difference`, `from_begin_end`,
    `from_min_max` and `from_range`.
  - **FR-007**: "bounds MUST reach every documented behaviour" became SHOULD. Unreached
    behaviour must be named, and tests and vectors must exercise it.
  - **FR-009 and FR-017**: decoder error messages are formatted only when displayed, with
    the same text.
  - **SC-001**: measured bounds. Read-only operations at ≥ 3 cells, normalising at ≥ 2,
    two-union operations at ≥ 1 × 1, and decoding up to 17 bytes, with a stated
    fallback.
  - **SC-002**: 60% (27/42).
  - **Edge cases, Context and Assumptions**: aligned.

  "Requirements are testable" still holds. The decoding fallback is a stated rule, not an
  open question. All items pass.
- Iteration 4 (2026-10-03, after `/speckit-analyze`): wording fixes, with no change of
  intent.
  - **FR-002 and SC-002:** the planning probe measured only `difference` and the
    begin–end constructor. The minimum–maximum constructor inherits that measurement, and
    the library's range constructor is measured during implementation.
  - **FR-008:** the reachable-outcome example no longer names a sibling merge, which no
    proof can reach.
  - **SC-001:** each bound is now stated as "every input of up to N cells, with N at least
    …", and three of its values are marked as estimates (R4).
  - **Assumptions, "Bounds":** aligned with SC-001.

  All items still pass.
