# Specification Quality Checklist: Formal Verification of the Decoding Layer

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-04
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

- Iteration 1 (2026-10-04). Two scope questions were asked before the spec was written,
  because each changed the whole document. Both are recorded under "Clarifications / Session
  2026-10-04":
  - Target: the decoding layer (recommended), rather than the floating-point value types,
    trait laws, or both.
  - Deductive track: "Verus for unbounded results", as a tracked deliverable with its own CI
    job. This was chosen over the recommended "Kani only" and over a side pilot. It became
    FR-010 to FR-012, User Story 3 and SC-005.

  No marker was needed after that.
- "No implementation details": the Context section names Kani, Verus and cargo-mutants. The
  feature request is an analysis of the other branch's methods, and those methods are those
  tools. The requirements and success criteria name no tool, file layout or command. They say
  "bounded checker", "deductive verifier" and "mutation tool". Rust type names (`CellId`,
  `Cap`, …), debug builds, address-space limits and the Python binding appear because they
  are the observable behaviour that the known-defect requirements describe. 001 and 002
  accepted the same choice.
- "Non-technical stakeholders": as in 001 and 002, the stakeholders of a geometry library are
  its maintainers and contributors. Domain terms (codec, wire format, varint, offset) are
  explained in plain language where they first appear.
- Method analysis: every method on the other branch has a row in the Context table with an
  adopt, decline or out-of-scope decision and its reason against 001/002 and the
  constitution. The deciding constraints:
  - Principle I (parity): the reference vectors and documented deviations;
  - Principle II (decoders never panic or abort): K2 and the missing fuzz targets;
  - the minimal-dependency rule: no verifier crates in the published library;
  - 001/002's non-intrusiveness rule: proofs outside the library, no API change.
- Survey evidence (2026-10-04). A throwaway probe program, linked against the core crate,
  confirmed:
  - K1: `encode_uint_with_length(len 9)`, `get_uint_with_length` (short slice, and len 9)
    and `decode_uint_with_length(len 9)` all panic.
  - K2: 5-byte inputs make `decode_uint_vector_u64` request 400 MB, `decode_string_vector`
    500 MB and `decode_s2point_vector` 1.2 GB. Under `ulimit -v` (300 MB, or 1 GB for the
    point vector) each ends with SIGABRT. The 200 MB (u32) and inherited cell-id vector
    figures come from reading the code, which follows the same path.
  - K3: offsets `[5, 2, 6]` over `abcdef` decode to `["abcde", "", "cdef"]`.
  - K4 comes from reading the code and its rustdoc. The fuzz-target gap comes from listing
    `fuzz/fuzz_targets/`.
- Upstream behaviour was checked in C++ `v0.11.1-25` sources, which are older than the
  pinned oracle `v0.14.0-39`. Planning re-confirms each item against the pinned version:
  - `EncodedUintVector<T>::Init` checks `decoder->avail()` before accepting a count (K2).
  - `EncodedStringVector::Init` returns false on decreasing offsets (K3).
  - `S2CellId::Decode` validates nothing. `S2Cap::Decode` checks `is_valid()` only under
    `FLAGS_s2debug`, with `ABSL_CHECK`. `S2LatLngRect::Decode` always rejects an invalid
    rectangle (K4: only the `CellId` and `Cap` decoders deviate).
  - `EncodeUintWithLength` and `GetUintWithLength` `ABSL_DCHECK` that the length is at most
    `sizeof(T)` (K1).
  - `S2Cell::Decode` constructs `S2Cell(id)` from an unvalidated id. This is to be assessed
    under FR-025, not asserted.
- Counts:
  - The inventory in FR-002 has 24 entries: 3 helpers, 4 integer-vector operations, 2
    cell-id vector, 2 string vector, 3 point vector (encode fast, encode compact, decode) and
    10 fixed-size codec operations.
  - SC-002's 83% allows up to three tool-limit exclusions besides the by-design compact
    encoder. The `Rect` decoder (8 GB on the other branch) and the compact point branch are
    the likely ones.
  - FR-023 and SC-011 count ten wire-format decoders. Seven already have fuzz targets, and
    three are added.
- SC-001's bounds are informed estimates: 002's decoder reached 25 bytes, and the other
  branch's per-value and fixed-size round trips took seconds. The Assumptions section says
  planning measures them first, implementation may raise them, and lowering them needs a
  written justification. This is the pattern 002 settled on after its iteration 3.
- FR-011's fallback (a test-only link when bounded agreement is infeasible) is a stated
  rule, not an open question. Whether the restated code can run under the bounded checker is
  a planning question. The other branch reports that the Verus pilot compiles to ordinary
  Rust once ghost code is erased.

  All items pass.
- Iteration 2 (2026-10-05, during `/speckit-plan`). No user decision was involved. The
  changes correct facts, following the constitution's parity rule:
  - **K3 withdrawn.** The pinned oracle `v0.14.0-39` accepts decreasing string offsets and
    decodes bad entries as empty (upstream commit `95b1e15`, 2025-12-29). That is the
    library's behaviour. FR-020 now documents and proves it instead of changing it.
    SC-009, FR-006, FR-013, FR-023, FR-024 and SC-007 were aligned. The survey had read
    `v0.11.1`.
  - **FR-022** gains the count and size limits (50M elements, 50M points, 500 MB of string
    data) as documented deviations. Upstream has none of them.
  - **FR-019** now says "grow on demand". The planning probe showed that a capped
    reservation fixes the abort, but leaves every vector decoder out of the checker's
    memory budget (research R0, R7).
  - **FR-006**'s parity bullet is a test entry, not counted by SC-003.
  - A D-003 candidate (the varint tenth byte) is in research R1. It is handled under FR-014
    and needs no spec change.

  All items still pass.
- Iteration 3 (2026-10-05, during `/speckit-plan`). The probe found the string-vector and
  point-vector decoders out of the checker's reach at SC-001's estimated bounds. The user
  chose "measured bounds". It is recorded under "Clarifications / Session 2026-10-05". Spec
  changes:
  - SC-001: the measured bounds; the string and point vector bounds set by measurement,
    with tool-limit exclusions where nothing useful fits; the round trip by composition.
  - Assumptions, "Bounds and budget": aligned.

  "Requirements are testable" still holds, because the measure-then-exclude rule is stated.
  All items pass.
