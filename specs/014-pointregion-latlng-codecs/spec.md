# Feature Specification: The `PointRegion` and `LatLng` Codecs

**Feature Branch**: `tb/014-frm/1`

**Created**: 2026-11-01

**Status**: Implemented 2026-11-01. No clarification beyond the maintainer's scope choice. Both
codecs are upstream's byte for byte and proven (P-FX-11 … 14); every C++ row matches. The
decoding suite's `codecs` CI leg was split in two to stay under 10 minutes. SC-004 on a hosted
runner needs a push (tasks T014). Evaluation: the spec-014 part of
`verify/decoding/EVALUATION.md`.

**Input**: User description: "continue with next" — scope chosen by the maintainer on 2026-11-01:
"Missing codecs: add upstream's S2PointRegion and S2LatLng encodings to the library and prove
their round trips and decoders like spec 003's."

## Context

### Where the earlier features left off

- **003** verified the decoding layer: the variable-length codecs (with Verus) and the
  fixed-format ones (`CellId`, `Point`, `Cap`, `Rect`, `Cell`), each with a round trip, an
  agreement proof against a reference description of the wire format (`wire_spec`) on every
  input up to its length plus one, and C++ reference vectors.
- **010** and **011** verified `LatLng` and `PointRegion` and listed their codecs as missing
  (spec 011's FR-020, spec 012's and 013's recommendations): upstream has
  `S2PointRegion::Encode`/`Decode` and `S2LatLng::Encode`/`Init(Decoder*)`; the library has
  neither, so data written by upstream cannot be read, nor written for it.
- **013** recommended accuracy contracts next; the maintainer chose the codecs instead, as the
  concrete parity gap.

### Upstream, read (v0.14.0-40)

- **`S2PointRegion`** (`s2point_region.cc:55–78`): `Encode` writes one version byte (1) and the
  point's three coordinates as little-endian doubles, 25 bytes. `Decode` rejects fewer than 25
  bytes, reads the version and rejects one above 1 (**so version 0 is accepted**), reads three
  doubles, and rejects a point that is not unit length (`S2::IsUnitLength`:
  `|x² + y² + z² − 1| ≤ 5ε`, the library's `Point::is_unit`; NaN fails it).
- **`S2LatLng`** (`s2latlng.cc:38–54`): `Encode` writes the latitude and the longitude in radians
  as two little-endian doubles, 16 bytes. `Init` rejects fewer than 16 bytes and otherwise
  accepts **every** pair, invalid and NaN included: an `S2LatLng` may be invalid by design.
- Neither format is used inside another upstream encoding; both are standalone.

### Decisions this spec takes by default

- **Parity**: both codecs follow upstream byte for byte, including version 0's acceptance and
  `S2LatLng`'s acceptance of any 16 bytes. No deviation is planned; one found would be recorded
  as a U-NNN finding first.
- **Encoding a non-unit point**: `PointRegion::new` accepts any point (spec 011's K3), and
  upstream encodes whatever it holds; the library does the same, so such an encoding does not
  decode. The round trip is therefore stated for unit-length points.
- **Bindings**: the Python and WASM bindings expose none of the fixed-format codecs (`Point`,
  `Cap`, `Rect`), so they gain none here (Principle IV: thin, synchronized bindings).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read and write upstream's encodings (Priority: P1)

As a user of the library who exchanges data with upstream S2, I want to encode and decode
`PointRegion` and `LatLng` in upstream's binary format, so that values written by either side
are read by the other.

**Why this priority**: It is the parity gap itself.

**Independent Test**: Encode values with the library and compare with bytes C++ writes for the
same values; decode bytes C++ wrote, and adversarial bytes, and compare the outcomes with C++'s.

**Acceptance Scenarios**:

1. **Given** a `PointRegion` or a `LatLng`, **When** it is encoded, **Then** the bytes are
   upstream's for the same value.
2. **Given** bytes upstream accepts, **When** the library decodes them, **Then** it accepts them
   with the same value, consuming the same number of bytes; **Given** bytes upstream rejects,
   **Then** the library rejects them with an error, never a panic.
3. **Given** a unit-length point region or any latitude–longitude, **When** it is encoded and
   decoded, **Then** the result is the original, bit for bit.

---

### User Story 2 - Proven like the other codecs (Priority: P1)

As the maintainer, I want both codecs proven as spec 003 proved the others: crash freedom,
round trip, and agreement with the reference description on every input up to the format's
length plus one, so that the new code meets the same bar.

**Why this priority**: New decoders read untrusted bytes; Principle II requires crash freedom,
and the suite's standard is proof, not sampling.

**Independent Test**: The verification command reports each property proven; an injected defect
in either codec (a wrong version check, a missing unit test, a swapped coordinate) fails one.

**Acceptance Scenarios**:

1. **Given** every byte string up to 26 bytes (`PointRegion`) and up to 17 bytes (`LatLng`),
   **When** verification runs, **Then** the library's decoder agrees with the reference
   description: same acceptance, same value, same bytes consumed.
2. **Given** every unit-length point and every pair of doubles, **When** verification runs,
   **Then** the round trips hold bit for bit.

---

### User Story 3 - Records, CI and evaluation (Priority: P3)

As a contributor, I want the new proofs in CI and the records updated; as the maintainer, a short
evaluation.

**Why this priority**: Proofs that are not re-checked go stale.

**Independent Test**: A change to either codec runs these proofs in CI; the evaluation answers
"are the new codecs upstream's, and proven?"

**Acceptance Scenarios**:

1. **Given** a change to the codecs, **When** CI runs, **Then** these proofs and the C++ diff
   tests run.
2. **Given** the completed feature, **When** the evaluation is read, **Then** it says what is
   proven and what comes next.

---

### Edge Cases

- **Short input.** 0 to 24 bytes for `PointRegion`, 0 to 15 for `LatLng`: rejected.
- **Version byte.** 0 and 1 accepted, 2 to 255 rejected (`PointRegion`).
- **Not unit length.** A point off unit length beyond `5ε` in squared norm, the zero vector, a
  NaN or infinite coordinate: rejected. A point within `5ε` but not normalised exactly: accepted
  as written.
- **Trailing bytes.** Bytes after the format are left unread, as upstream's decoder leaves them.
- **Signed zeros and NaN payloads.** Kept bit for bit by both codecs.
- **Invalid latitude–longitude.** Any 16 bytes decode, as upstream (out-of-range angles, NaN).

## Requirements *(mandatory)*

### Functional Requirements

**The codecs**

- **FR-001**: The library MUST provide `S2Encode` and `S2Decode` for `PointRegion` in upstream's
  format: version byte 1 and three little-endian doubles on encoding; on decoding, reject input
  shorter than 25 bytes, a version above 1, and a point that is not unit length (`Point::is_unit`),
  and otherwise return the point region, consuming 25 bytes.
- **FR-002**: The library MUST provide `S2Encode` and `S2Decode` for `LatLng` in upstream's
  format: latitude then longitude in radians as little-endian doubles; on decoding, reject input
  shorter than 16 bytes and otherwise return the pair as read, consuming 16 bytes.
- **FR-003**: Decoding errors MUST be the library's existing kinds (`UnexpectedEof` for short
  input, `InvalidData` for a rejected value), and no input may panic.
- **FR-004**: Each new public item MUST be documented with its format, what it rejects, and the
  upstream function it mirrors.

**Verification**

- **FR-005**: The reference description (`wire_spec`) MUST gain both formats, written from
  upstream's source, and be checked against C++ vectors.
- **FR-006**: Kani MUST prove, for each codec: the round trip (every unit-length point; every
  pair of doubles), the encoder's canonical bytes, and agreement with the reference description
  on every input up to the format's length plus one byte; and crash freedom of both decoders on
  every input of those lengths.
- **FR-007**: Every proven property MUST be shown non-vacuous by an injected defect in the library
  code, recorded with its outcome.
- **FR-008**: The C++ generator MUST emit encode and decode rows for both codecs (random values,
  unit points at `5ε`'s edge, adversarial bytes: short inputs, every version class, NaN and
  infinite coordinates), and a differential test MUST check the library on every row.
- **FR-009**: Quickcheck twins MUST check both round trips and the decoders on random bytes.

**Non-intrusiveness and operability**

- **FR-010**: No existing behaviour or encoding may change; the only API additions are the four
  trait implementations. No new dependency.
- **FR-011**: A defect found MUST be fixed as a separate, recorded change with a regression test,
  after the maintainer's go-ahead if not in this spec; an upstream finding gets a U-NNN entry.
- **FR-012**: The proofs MUST run from the decoding suite's command, with a way to run only this
  feature's, in CI for changes to the files they cover; over-budget ones weekly.
- **FR-013**: The records the change affects MUST be updated in the same change: the decoding
  suite's inventory, catalogue and evaluation, the index by kind and its render, `MUTANTS.md`,
  `VERIFIED_TYPES.md`, `CORE_TRAITS.md` (the codec trait's implementors), and the user-facing
  documentation of the encodings.

### Key Entities

- **`PointRegion` encoding**: 25 bytes, a version and a unit point.
- **`LatLng` encoding**: 16 bytes, two angles in radians.
- **Reference description**: `wire_spec`'s decoders and encoders, from upstream's source.
- **Inventory, catalogue, evaluation**: the decoding suite's (spec 003).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Both codecs encode bit for bit as upstream and decode every C++ vector row with
  upstream's outcome (zero differences).
- **SC-002**: Round trip, encoder and agreement are proven for both codecs over their whole
  domains (every input up to 26 and 17 bytes), with crash freedom of both decoders.
- **SC-003**: 100% of proven properties have recorded non-vacuity evidence.
- **SC-004**: This feature's per-change proofs complete in under 10 minutes on the standard CI
  machine (a hosted Linux runner, 4 vCPUs, 16 GB).
- **SC-005**: Existing behaviour and encodings are unchanged, and the full quality gate passes.
- **SC-006**: Every catalogue entry appears once in the index of properties by kind, and the
  consistency check passes.
- **SC-007**: From the evaluation alone, a maintainer can answer "are the new codecs upstream's,
  are they proven, and what comes next?"

## Assumptions

- **Scope**: the maintainer's choice on 2026-11-01 (the missing codecs); the exact
  `S2LatLngRect::Intersects(const S2Cell&)` on the same list is out of scope (edge geometry).
- **Reference semantics**: upstream C++ at v0.14.0-40, Release, x86-64.
- **Builds on 003**: the codec harness framework (`round_trip`, `check_encoder`, the agreement
  harnesses), `wire_spec`, the C++ coding generator and its diff tests, and 011's
  `PointRegion` and 010's `LatLng`.
- **Lessons carried over**: a float check computed twice (library and model) times out, so the
  model may state acceptance through the library's own `Point::is_unit` with a drift guard on
  its text (spec 003's R18 fallback for `Cap`); a checker-unproven fact gets a native test.
- **Out of scope**: binding changes (no fixed-format codec is bound); the Verus track (fixed
  formats have no lengths to induct over).
- **Existing tests stay**; tests are added, none removed.
