# Feature Specification: Formal Verification of the Decoding Layer

**Feature Branch**: `003-next-specs/1`

**Created**: 2026-10-04

**Status**: Draft

**Input**: User description: "Take the formal method changes introduced recently on branch
claude/formal-methods-type-invariance-ept6u0 and use them as inspiration for continued work on
this branch by analyzing the methods used in accordance with previous formal method work and
the constitution in the repo"

## Context

### Where the earlier features left off

Feature 001 proved `CellId` with exhaustive bounded model checking. Feature 002 proved
`CellUnion` within small size bounds. Both evaluations recommend the same next step: the code
that decodes untrusted bytes. The 002 evaluation ranks the integer coding layer first (the
fixed-width and variable-length integers and the integer, cell-identifier and string vectors)
and the other fixed-size decoders second. Its reasons are untrusted input, fixed-width integer
arithmetic and byte-level formats. Those are where the checker did best, and Constitution II
places its strongest demand there: decoders must never panic, abort or hang.

The 002 evaluation also found the limit of the method. A bounded checker covers collections
only up to a few elements, and removing that bound needs a deductive verifier. It recommended
deductive proof for later. Decoding is where that limit hurts first, because every vector
format has an unbounded length.

### What the other branch contributes

The branch `claude/formal-methods-type-invariance-ept6u0` holds a survey of formal methods for
this library (`docs/formal-methods-and-type-invariance.md`) and about twenty pilots. It was
written against release v0.3.2, an ancestor of this branch. Its methods were compared with
001, 002 and the constitution, with this outcome:

| Method on that branch | What it showed | Fit with 001/002 and the constitution | Decision for 003 |
|---|---|---|---|
| Bounded model checking (Kani) of the real code | Fixed-width integer round trip in 5 s. Encode/decode round trips for five fixed-size types in 2.5–40 s. Comparison-only floating-point types in seconds to minutes | The same tool as 001/002. That branch puts its harnesses inside the published crate under a verification-only flag and adds an opt-in feature to its manifest. 001/002 keep proofs in a separate workspace that uses only the public API | **Adopted**, with proofs kept outside the library. Internal helpers are reached through the public operations that use them |
| One law stated once and checked against many implementors | The `Shape`, `Region` and encode/decode laws, each checked against several types. Found that `EdgeVectorShape::set_dimension` breaks the `Shape` contract | New to this repository's method. It suits a layer where many codecs share one contract | **Adopted**: one decoder contract (FR-004), checked for every codec |
| Measured memory boundary | Vector framing with two symbolic elements took 7.5 GB and still failed its unwinding check. Rejecting invalid rectangles hit an 8 GB cap. Symbolic allocation sizes are the memory wall | Agrees with 002's lessons: concrete lengths, small collection bounds | **Adopted** as planning input. These numbers seed the bounds in SC-001 |
| Deductive verification (Verus) of restated code | The fixed-width integer encoding proved with loop invariants and induction in 1.4 s, and the `CellId` core in 2.5 s. Floating-point arithmetic cannot be stated. Checking the real code in place would need verifier crates in the published library and a pinned compiler | 002 names deductive proof as the way to remove size bounds. The constitution keeps the library's runtime dependencies minimal, and a restated copy can drift from the real code | **Adopted for unbounded results** (Clarifications). It runs on restated code, with a checked link to the library (FR-011), in its own CI job, and is never a library dependency (FR-012) |
| Tool-generated mutation testing (cargo-mutants), with proofs to sort out equivalent mutants | On one source file, 130 mutants were caught, 28 missed and 28 timed out. One proof killed every non-equivalent missed mutant in its function and showed three others to be equivalent | 001/002 use hand-written mutants only, to show that proofs are not vacuous. Nothing measures what the proofs add over the tests | **Adopted** as a measurement over the in-scope source files (FR-018) |
| Type-level invariants (private fields, validating constructors, total orders) | `CellId`, `Point`, `ChordAngle` and `s1::Interval` carry invariants that the type system does not enforce | A public API change. It conflicts with the non-intrusiveness rule of 001/002 and touches both bindings | **Declined** here, and recorded as a candidate for later |
| Certified floating-point error bounds, structured kernel fuzzing, a new property-testing framework, API-stability checks | Proposed, not piloted | Outside the decoding layer | **Out of scope** |

Three findings of that branch still hold on this branch (checked 2026-10-04):

- `s1::Interval::length` returns the empty-interval sentinel for a one-ulp arc across the
  ±π seam. This is inherited from upstream.
- `ChordAngle::is_valid` accepts every negative value, not only the negative sentinel. This is
  inherited from upstream.
- `EdgeVectorShape::set_dimension` does not check the edges against the dimension it sets.

A fourth no longer applies: the `Cap` decoder accepted any bytes at v0.3.2 and now rejects
invalid caps. The three remaining findings are outside this feature. FR-017 carries them into
its evaluation so that they are not lost.

### Survey of the decoding layer

A survey before this specification (2026-10-04) confirmed four findings with concrete probes:

- **K1, undocumented panics.** Three public fixed-width integer helpers panic in every build
  when their length argument exceeds 8 or the input is shorter than that length. None of them
  documents a panic. Upstream guards the same preconditions with debug-only checks.
- **K2, abort on a five-byte input.** The decoders reserve memory for the element count an
  input declares, before reading any element. A 5-byte input makes the 64-bit integer vector
  decoder request 400 MB. It makes the string vector decoder request 500 MB and the point
  vector decoder 1.2 GB. By the same code path, the 32-bit integer vector decoder can request
  200 MB, and the cell-identifier vector decoder inherits the 400 MB case. Where that much
  memory cannot be reserved (an address-space limit, strict overcommit, a 32-bit or
  WebAssembly target), the process aborts. Under an address-space limit (300 MB, or 1 GB for
  the point vector), each probe ended with "memory allocation of N bytes failed" and signal 6.
  The library's geometry decoders already cap this reservation, and upstream checks the
  available bytes before decoding. The Python binding exposes the cell-identifier vector and
  point vector decoders directly.
- **K3, decreasing string offsets (withdrawn during planning).** The string vector decoder
  accepts offsets that decrease, and decodes each entry with an invalid range as empty:
  offsets `[5, 2, 6]` over `abcdef` give `["abcde", "", "cdef"]`. The survey read upstream
  `v0.11.1`, which rejects such offsets. Planning re-read the pinned oracle `v0.14.0-39`.
  Since commit `95b1e15` (2025-12-29, "EncodedStringVector: Make safety check lazy"),
  upstream gives exactly the library's answer (research R1). K3 is therefore not a defect.
  The behaviour is documented and proven instead (FR-020).
- **K4, undocumented deviations.** The `CellId` decoder rejects invalid identifiers, where
  upstream accepts any 64 bits. The `Cap` decoder rejects invalid caps, where upstream checks
  validity only in a debug mode and aborts there. Both choices protect the types' invariants,
  but only code comments record them. The API documentation does not, which Constitution I
  requires.

The survey also found a gap against Constitution II: the binary decoders of `CellId`, `Point`
and `Rect` have robustness tests but no fuzz target. It also noted one upstream
behaviour to assess under FR-025: the C++ cell decoder builds a cell from an identifier it
has not validated.

This feature handles the findings in FR-019 to FR-023.

## Clarifications

### Session 2026-10-04

- Q: Which target should this feature verify: the decoding layer, the comparison-only
  floating-point value types, or trait laws across implementors? → A: The decoding layer.
  That means the integer coding primitives, the integer, cell-identifier, string and point
  vectors, and the fixed-size binary codecs of `CellId`, `Point`, `Cap`, `Rect` and `Cell`.
  The floating-point value types and the `Shape` and `Region` laws are left for later
  features.
- Q: Should the feature include a deductive verification track? → A: Yes. Use deductive
  proofs where the bounded checker's size bounds cut off the domain (vector framing and
  multi-element decoding), as a tracked deliverable with its own CI job. It is not a side
  pilot.

### Session 2026-10-05 (planning)

- Q (raised by the planning probe, research R0): at 4 GiB, the string-vector decoder runs out
  of memory even on 4-byte inputs. The point-vector decoder runs out at 25 bytes, and its
  compact branch at 11. SC-001 assumed 11 and 25 bytes. The integer and cell-id vector
  decoders fit at 11 bytes, and every fixed-size codec over its whole domain. How should the
  spec handle the two? → A: Measured bounds, as 002 did. Both get the largest bound that
  fits. Implementation measures smaller lengths and the 8 GiB weekly tier first. What still
  does not fit becomes a tool-limit exclusion with its measurements. The string vector's
  every-length results come from the deductive track, with a test-only link where the
  bounded checker cannot reach (FR-011's stated fallback). The point vector beyond its bound
  relies on tests and fuzzing.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Proven crash-freedom on untrusted bytes (Priority: P1)

As the s2rst maintainer, I want a machine-checked proof that every in-scope decoder returns a
value or an error, and never panics, overflows, hits undefined behaviour, aborts or fails to
terminate. The proof should cover every byte sequence within stated bounds. I also want proof
that every in-scope encoder and integer helper completes safely on its documented domain. The
library decodes these formats from files and network messages, and two of the decoders are
reachable from Python with a few bytes.

**Why this priority**: Constitution II already requires this property, and K2 shows it is
broken today. It is also the cheapest result to reach. It is useful even if no functional
property is proven.

**Independent Test**: Run the documented verification command on the unmodified repository.
It reports each in-scope operation as proven crash-free within its stated bounds, with no
unproven obligations remaining.

**Acceptance Scenarios**:

1. **Given** the unmodified repository, **When** the maintainer runs the verification
   command, **Then** it names each in-scope operation as proven crash-free, states the
   byte-length or element-count bound used, and exits successfully.
2. **Given** any byte sequence up to the stated length, **When** an in-scope decoder reads it,
   **Then** the proof shows that the decoder returns a value or an error and never panics,
   aborts or loops.
3. **Given** a decoder that is given an input declaring a huge element count, **When** it
   runs, **Then** it reserves no more than a fixed amount of memory before the data runs out,
   and returns an error.
4. **Given** an in-scope operation deliberately changed so that it can overflow or index out
   of bounds, **When** verification runs, **Then** it fails and names the operation and a
   concrete input that triggers the failure.

---

### User Story 2 - One proven contract for every codec (Priority: P2)

As the maintainer, I want one statement of what a correct codec does, checked against every
in-scope codec. Decoding an encoded value returns exactly that value. The encoder writes, and
the decoder consumes, exactly the documented bytes. Every accepted input yields a well-formed
value, and every malformed input is rejected. Each format's own rules (header layout, element
width, offsets, version bytes) are proven against an independent description of the wire
format. Format parity with upstream is checked against committed C++ reference vectors.

**Why this priority**: This is where proofs add most over fuzzing, which finds crashes but not
wrong values. Stating the contract once also gives future codecs a ready-made check. It
depends on the scaffolding from Story 1.

**Independent Test**: Run the verification command. Every catalogue property is reported as
proven, proven within stated bounds, proven for all sizes on restated code, or not proven
with a written reason. Each proven property fails when a matching defect is injected.

**Acceptance Scenarios**:

1. **Given** the decoder contract and an in-scope codec, **When** verification runs, **Then**
   each part of the contract is discharged for that codec, or recorded as not applicable with
   a reason.
2. **Given** any proven property, **When** a defect that violates it is introduced, **Then**
   verification fails for that property.
3. **Given** the committed C++ reference vectors, **When** the differential tests run, **Then**
   the library's encodings are byte-identical to C++. Its decoders agree with C++ on
   acceptance and on the decoded value, except for deviations its documentation states.

---

### User Story 3 - Unbounded proofs for the variable-length formats (Priority: P3)

As the maintainer, I want the round trip, exact consumption and crash-freedom of the
variable-length formats proven for **every** length, not only up to the few elements the
bounded checker reaches. I also want those proofs tied to the library's actual code, so that
they cannot silently drift from it.

**Why this priority**: It removes the main limitation that 002 recorded. It depends on Stories
1 and 2 for the contract, the catalogue and the bounded agreement check.

**Independent Test**: Run the deductive verification job. It reports each unbounded property
as proven. An injected defect in the restated code makes the deductive proof fail. An injected
divergence between the restated code and the library makes the agreement check fail.

**Acceptance Scenarios**:

1. **Given** the restated variable-length codecs, **When** the deductive job runs, **Then**
   every property assigned to it (FR-010) is proven for all lengths, with no assumed lemmas
   left unproven.
2. **Given** a change to an in-scope library function that its restated copy does not mirror,
   **When** CI runs, **Then** the agreement check or the drift guard fails and names the
   function.
3. **Given** a change that touches none of the in-scope code or the deductive artifacts,
   **When** CI runs, **Then** the deductive job does not run, and the default gate does not
   depend on its toolchain.

---

### User Story 4 - Regression protection for future changes (Priority: P3)

As a contributor changing the coding layer, a fixed-size codec, or the `CellId` operations
the codecs rely on, I want both proof tracks to run in CI. A change that breaks a proven
property should fail with the property's name and a concrete counterexample before it is
merged.

**Why this priority**: Proofs that are not re-checked go stale. This protects Stories 1 to 3,
but it only matters once they exist.

**Independent Test**: Open a change that breaks one proven codec property. The verification
check fails and names the property. Revert it and the check passes.

**Acceptance Scenarios**:

1. **Given** a proposed change to the in-scope code or the verification artifacts, **When** CI
   runs, **Then** the decoding proofs run and their result appears on the change.
2. **Given** a change that touches none of these, **When** CI runs, **Then** the default
   quality gate is not slowed down or made to depend on verification toolchains.

---

### User Story 5 - Evaluation of the methods (Priority: P3)

As the maintainer, I want a written evaluation of this third feature. It should cover cost,
reach and findings, as the earlier evaluations did. It should also answer the questions this
feature adds. What does the deductive track cost and buy compared with the bounded checker,
on the same properties? What do the proofs add over the test suite, measured by
tool-generated mutants? Which of the other branch's methods and findings should come next?

**Why this priority**: Its answer decides whether deductive proof extends to `CellUnion`
normalisation and beyond, which 002 left open.

**Independent Test**: A maintainer who did not do the work reads the evaluation. From it alone
they can answer "continue?", "with which method or methods?", "is the deductive track worth
its cost?" and "what next?".

**Acceptance Scenarios**:

1. **Given** the completed feature, **When** the evaluation is read, **Then** it contains a
   go or no-go recommendation for each proof track, a comparison with 001 and 002, the
   mutation measurement, and a ranked list of candidates for follow-up work.

---

### Edge Cases

- **Empty encodings**: vectors of zero elements, a string vector of zero strings, and strings
  of zero length must round-trip and decode as documented.
- **Element width**: integer vectors whose largest element needs 1 to 8 bytes (1 to 4 for
  32-bit elements), including all-zero vectors and the maximum value.
- **Variable-length integer limits**: encodings of 1 to 10 bytes, a tenth byte with more than
  one significant bit, overlong encodings (a value written with more bytes than needed), and
  input that ends inside an integer. For each of these, upstream's behaviour decides what the
  library must document, and the proof covers what the library documents.
- **Header limits**: a declared element count above the decoder's maximum, a count larger than
  the bytes that follow (K2), and a count whose byte size would overflow.
- **Cell-identifier vectors**: every base length 0–7 and every shift code, including the
  extended shift code with its second byte and shifts above the maximum. Deltas whose sum with
  the base wraps around. The format can carry identifiers that are not valid cells. Like
  upstream, the library returns them unchanged; its documentation must say so, and the proofs
  state it.
- **String vectors**: decreasing offsets (K3), offsets beyond the data, the total-size limit,
  and empty strings between non-empty ones.
- **Point vectors**: the format bits in the first byte, unknown formats, uncompressed points
  with every bit pattern (NaN, infinities, non-unit vectors), and compact-format headers with
  levels above the maximum.
- **Fixed-size codecs**:
  - `Point` accepts any 24 bytes. It must round-trip every bit pattern exactly, NaN included,
    as upstream does.
  - `CellId` and `Cell` reject invalid identifiers.
  - `Cap` rejects invalid caps; empty and full caps must still round-trip.
  - `Rect` rejects unsupported versions and invalid rectangles, and accepts versions 0 and 1.
  - For every input longer than the encoded length, the decoder reads exactly the encoded
    length and leaves the rest.
- **Length arguments**: the fixed-width helpers' length argument across its whole range,
  including 0 and every value above 8 (K1).
- **Streams that end early**: every decoder, given every proper prefix of a valid encoding,
  returns an error and never a partial value.
- **Debug-only checks**: an operation guarded by a debug-only check must be proven safe for the
  domain the check guards, without relying on the check firing in release builds.
- **Restated code drift**: an in-scope library function changes, but its restated copy for the
  deductive track does not (FR-011).

## Requirements *(mandatory)*

### Functional Requirements

**Scope**

- **FR-001**: The feature MUST verify the decoding layer of the core library:
  - **(a)** the fixed-width integer helpers (write, read from a slice, read from a stream);
  - **(b)** the 32-bit and 64-bit integer vectors;
  - **(c)** the cell-identifier vector;
  - **(d)** the string vector;
  - **(e)** the point vector;
  - **(f)** the binary codecs of `CellId`, `Point`, `Cap`, `Rect` and `Cell`.

  The variable-length integer codec is internal to the library. It is verified through the
  public operations that use it, and by the deductive track (FR-009).
- **FR-002**: The verification artifacts MUST include an operation inventory. It lists every
  public encode and decode operation in (a)–(f), 24 entries: 3 helpers, 4 integer-vector
  operations, 2 each for the cell-identifier and string vectors, 3 for the point vector, and
  10 for the fixed-size codecs. The point vector has three entries because its encoder has two
  formats. Each entry is marked **in scope** or **excluded**, with a written reason for every
  exclusion.
  - The compact point-vector encoder is excluded by design. It turns points into cell centres
    with floating-point projections.
  - The compact branch of the point-vector decoder is in scope for crash-freedom only, and it
    is measured first. Its decoded values come from the same projections.
  - Any other exclusion is a tool limit. It MUST be justified in the evaluation with
    measurements.
- **FR-003**: Outside the inventory and outside this feature: the decoders of geometry shapes
  and indexes (polylines, loops, polygons, lax shapes, the point-vector shape, shape indexes),
  compressed points, the density tree and the text parsers. They keep their robustness tests
  and fuzz targets. They benefit from the FR-019 fix wherever they share its code.

**Guarantees**

- **FR-004**: The feature MUST state one **decoder contract**, once, and check it for every
  in-scope codec. The contract has five parts:
  1. **Round trip**: decoding the encoding of any encodable value returns exactly that value,
     bit for bit.
  2. **Exact consumption**: the encoder writes exactly the documented number of bytes. The
     decoder consumes exactly those bytes and leaves any following bytes unread, so that
     encodings can be concatenated.
  3. **Sound acceptance**: every successful decode returns a value that satisfies the
     validity condition the library documents for that decoder. Examples: a valid cell, a
     valid cap, a valid rectangle, and any bit pattern for a point. For a string vector,
     every entry is a range of the data or empty.
  4. **Rejection**: every input outside the documented accepted set returns an error. That
     includes truncated input, unsupported versions or formats, counts above the limit, and
     invalid values.
  5. **Bounded reservation**: before reading the data it describes, a decoder reserves at
     most a fixed amount of memory plus an amount proportional to the bytes it has read.

  A part that does not apply to a codec is recorded as such, with the reason. For example, a
  decoder that accepts every byte pattern has no rejection case beyond truncation.
- **FR-005**: For every in-scope operation, verification MUST establish that it completes
  without panic, arithmetic overflow, undefined behaviour, abort or non-termination.
  - It MUST hold for every input in the operation's documented domain, within the stated
    bounds (FR-008).
  - A decoder's domain is every byte sequence. An encoder's domain is every value of its
    input type, or every sequence of such values. Any other argument ranges over its whole
    type, narrowed only by a documented precondition.

  A precondition counts as documented if the library's API documentation, a debug-build
  check or the upstream reference states it for the same operation. If a proof must assume a
  precondition that the library's own documentation does not state:
  - the undocumented panic MUST be recorded as a finding in the inventory and the evaluation;
  - the missing "Panics" documentation MUST be added (Constitution II);
  - if the panic also differs from upstream behaviour, the difference is a defect under
    FR-014.
- **FR-006**: The verification artifacts MUST include a property catalogue covering at least
  these areas:
  - **Fixed-width integers**: the length chosen is the minimum that holds the value. Writing
    then reading returns the value for every length 0–8 and every value that fits. The slice
    and stream readers agree.
  - **Variable-length integers**, through the operations that use them: the encoding has the
    canonical length of 1 to 10 bytes and round-trips every 64-bit value. Overlong and
    over-wide encodings are handled as documented.
  - **Integer vectors**: the header records the element count and width as upstream
    specifies. The width is the minimum that holds the largest element. Vectors round-trip.
    Counts above the limit and truncated data are rejected.
  - **Cell-identifier vectors**: every sequence of 64-bit identifiers round-trips, whether or
    not they are valid cells. The base and shift are chosen as upstream specifies. Shifts
    above the maximum are rejected.
  - **String vectors**: every sequence of byte strings round-trips. An entry whose offsets
    are out of order or out of range decodes as empty, and decoding continues from its end,
    as upstream (FR-020). The total-size limit holds.
  - **Point vectors**: every sequence of points in the uncompressed format round-trips, for
    every bit pattern. Formats are dispatched on the format bits, unknown formats are
    rejected, and the compact branch is crash-free.
  - **Fixed-size codecs**: the FR-004 contract for each, with its documented validity
    condition.
  - **Format parity**: the wire-format description agrees with every committed C++
    reference vector (FR-024), and the differential tests check the library against the same
    vectors. This entry is a test, not a proof. SC-003's percentage does not count it.

  Each entry MUST record a plain-language statement, the upstream source, the bound used and
  a status:
  - **Proven**: the whole domain is covered by the bounded checker.
  - **Proven within stated bounds**: a byte-length, element-count or loop bound leaves part of
    the domain uncovered, and that part is documented.
  - **Proven for all sizes on restated code**: proven by the deductive track for every length,
    and linked to the library's code by FR-011.
  - **Not proven (reason)**.

  An entry may carry two statuses, one per track.
- **FR-007**: Properties MUST be stated against an independent reference description of each
  wire format. The description is written from upstream's format documentation, not from the
  library's code. It reuses 001's reference model for cell-identifier validity. The proofs
  then establish agreement between two independent descriptions (Constitution I).
- **FR-008**: The bounded checker MUST be exhaustive within its bounds. It MUST explore every
  input up to the stated bounds, use exact machine-arithmetic semantics, and produce a
  concrete counterexample on failure.
  - Every decoder proof MUST state its byte-length bound. Every encoder proof over sequences
    MUST state its element-count bound.
  - A decoder that reads a fixed number of bytes is proven over every byte sequence of up to
    that length. Together with exact consumption, that covers its whole domain.
  - Bounds SHOULD reach every behaviour the format documents. Where the checker cannot reach
    one within the resource budget, that behaviour MUST be named in the inventory and the
    evaluation. Tests, fuzzing or the deductive track MUST then cover it.
  - Loop bounds follow 001's rule: the proof shows that the bound covers the whole domain, or
    it documents the uncovered part.

  Size bounds and loop bounds are the only accepted reasons to leave part of a domain
  uncovered, apart from the by-design exclusion of the compact point format (FR-002).
- **FR-009**: Every proven property MUST be shown to be non-vacuous.
  - At least one injected defect per proven property MUST make its verification fail, and the
    defects and outcomes MUST be recorded.
  - Crash-freedom proofs MUST show that their domain is non-empty and that their outcomes are
    reachable, including a successful decode and each rejection branch.
  - At least one injected overflow or out-of-bounds access MUST make a crash-freedom proof
    fail.
  - For the deductive track, an injected defect in the restated code MUST make a deductive
    proof fail. An injected divergence in the library's code MUST make the agreement check of
    FR-011 fail.

**Unbounded results (deductive track)**

- **FR-010**: For the variable-length integer codec, the two integer vectors, the
  cell-identifier vector and the string vector, a deductive verifier MUST prove these
  properties for every length:
  - round trip;
  - exact consumption;
  - sound acceptance;
  - the decoder terminates and never fails an arithmetic or indexing check, on byte
    sequences of every length.

  The properties MUST be stated against the same reference description as FR-007, restated
  in the verifier's specification language. The deductive verifier cannot check the
  library's source in place without adding dependencies to the published crate (FR-012). Its
  results are therefore about a restated copy of each algorithm, linked to the library by
  FR-011.
- **FR-011**: Each restated copy MUST be linked to the library code it mirrors:
  - **Bounded agreement**: the bounded checker MUST prove that the restated code and the
    library's code give identical results on every input within the FR-008 bounds. If
    planning shows this cannot be done for a codec, the plan records why, and the trusted
    base names the test link below as that codec's only link.
  - **Tested agreement**: tests MUST show identical results on generated inputs well beyond
    those bounds and on the C++ reference vectors.
  - **Drift guard**: a change to a library function that a restated copy mirrors MUST fail CI
    until the restated copy is reviewed and the guard updated.

  The restated copies and the agreement checks are part of the trusted base (FR-016).
- **FR-012**: The deductive track MUST NOT add a dependency to any published crate. It MUST
  NOT change library code to suit the verifier. Its toolchain MUST stay out of the default
  quality gate. It MUST run in its own CI job, for changes to the in-scope code or to the
  deductive artifacts.

**Non-intrusiveness**

- **FR-013**: The feature MUST NOT change the public API. It MUST NOT change observable
  behaviour except for the fix in FR-019. Decoder error messages MAY be
  formatted only when displayed, with unchanged text, where that is needed to bring a decoder
  within the checker's reach (as in 002). That does not count as a behaviour change. The
  feature MUST NOT add runtime dependencies or slow down the library. Verification-only code
  MUST be excluded from normal builds.
- **FR-014**: A defect found by verification MUST be fixed as a separate, recorded change. It
  needs a parity check against upstream (Principle I) and a regression test that fails
  without the fix (Principle III). It MUST NOT be absorbed silently into the verification
  work. The survey findings in FR-019 to FR-023 are the exception: they are fixed inside this
  feature. Documentation that changes no behaviour is also added inside the feature.

**Operability**

- **FR-015**: The decoding proofs MUST run from the project's documented verification command.
  It MUST report pass or fail per operation and per property, and offer a documented way to
  run only the decoding proofs. CI MUST run them through the same command, for changes to the
  in-scope code, to `CellId`, or to the verification artifacts. They MUST stay out of the
  default quality gate. Proofs over the per-change budget MAY run in the weekly tier, and
  their results count like any other proof.
- **FR-016**: The trusted base statement MUST be extended to cover this feature. It records:
  - every `CellId` result from 001 that a proof relies on;
  - the standard-library behaviour taken as given (byte-slice readers and writers, memory
    reservation);
  - every bound used;
  - the deductive verifier, its solver and its pinned compiler;
  - each restated copy, together with the agreement checks that link it to the library.

**Evaluation**

- **FR-017**: The feature MUST produce a written evaluation. It records the effort, the run
  times, the bounds reached, the limits hit and the defects found. It MUST also:
  - compare cost and value with 001 and 002;
  - compare the two proof tracks on the properties both cover: run time, memory, size of the
    specification and proof text, what each reached, and the drift risk of restated code;
  - give a go or no-go recommendation for each track. That includes whether to apply the
    deductive track to `CellUnion` normalisation;
  - give a ranked list of candidates for follow-up work, weighing the other branch's methods
    that this feature declined or left out;
  - list the other branch's findings that still apply on this branch, each with its status:
    the `s1::Interval::length` seam, `ChordAngle` accepting every negative value, and
    `EdgeVectorShape::set_dimension`.
- **FR-018**: The feature MUST measure what the proofs add over the tests. A mutation tool
  generates mutants for the in-scope source files. Each mutant is classified as:
  - caught by the test suite;
  - killed by a proof;
  - shown equivalent by a proof;
  - timed out;
  - or undetected, with a reason.

  The counts before and after the feature MUST be reported in the evaluation. This
  measurement is in addition to the hand-written mutants of FR-009, not a replacement.

**Known defects and gaps**

- **FR-019** (K2, refined during planning): No in-scope decoder may reserve memory from a
  size it has decoded. Each grows its result as data arrives, so a short input cannot
  request more than a fixed amount before its data runs out. Valid inputs of any size still
  decode. Today, 5-byte inputs request 200 MB to 1.2 GB, and the process aborts where that
  much memory cannot be reserved. Planning found that a capped reservation, as the geometry
  decoders use, fixes the abort but leaves the decoders beyond the checker's reach; growing
  on demand fixes both (research R0, R7).
  - Upstream rejects such input because it checks the available bytes first. After the fix,
    the library returns an error when the data runs out.
  - The fix MUST be its own identifiable change. It MUST come with a regression test that
    fails without the fix, and a differential test against the C++ reference vectors. It
    MUST be listed in the evaluation.
  - The Python binding exposes two of these decoders, so it inherits the fix.
- **FR-020** (K3, amended during planning): Decoding a string vector keeps its current
  behaviour, which matches the pinned upstream: an entry whose offsets are out of order or
  outside the data decodes as an empty string, and decoding continues from that entry's end
  offset. This MUST be documented in the decoder's API documentation and proven as a
  catalogue property (FR-006). The C++ reference vectors MUST include such inputs. No
  behaviour changes.
- **FR-021** (K1): The three fixed-width helpers MUST document their panics: a length above 8,
  and an input shorter than the length. Upstream states the same preconditions as debug
  checks. This is documentation only.
- **FR-022** (K4): The `CellId` and `Cap` decoders MUST document their rejection of invalid
  values as deliberate deviations from upstream, with the reason (Constitution I): a decoded
  value always satisfies its type's invariants.
  - Every in-scope decoder's error documentation MUST name each condition under which it
    rejects input, and state what it does not validate. For example, the cell-identifier
    vector does not validate identifiers, as upstream.
  - The library's count and size limits MUST be documented as deliberate deviations:
    integer vectors of more than 50,000,000 elements, point vectors of more than 50,000,000
    points, and string vectors with more than 500,000,000 data bytes are rejected. Upstream
    has no such limits and rejects a count only when its bytes are missing (research R1).
  - This is documentation only.
- **FR-023**: Every in-scope decoder of a complete wire format MUST have bounded robustness
  coverage and a fuzz target (Constitution II). There are ten: the two integer vectors, the
  cell-identifier, string and point vectors, and the five fixed-size codecs. The fixed-width
  helpers are primitives with documented preconditions (FR-021), covered by proofs over their
  whole domain. The missing targets for the binary `CellId`, `Point` and `Rect` decoders
  MUST be added to the nightly fuzz matrix. Existing robustness tests and fuzz targets MUST be
  kept, and updated for FR-019. The formal proofs complement them and do not
  replace them.

**Reference vectors and upstream findings**

- **FR-024**: The feature MUST add committed C++ reference vectors for the in-scope codecs:
  - encodings of representative values, including every element width, every shift code,
    and empty and maximal cases;
  - decoder outcomes (accepted with value, or rejected) for representative valid and
    malformed byte sequences, including the inputs behind K2, K3 and K4, and inputs above
    the count limits.

  Differential tests MUST check the library against them. CI MUST NOT need a C++ toolchain
  (Constitution I).
- **FR-025**: If the work shows undefined or wrong behaviour in the upstream reference, the
  finding MUST be noted as a follow-up task in the feature's task list before the affected
  proof is marked complete. The upstream reference means C++, the Go and Java secondaries, or
  the vectors generated from them. The task records the upstream location, the inputs and the
  observed behaviour. The proof then states the behaviour the library documents. Each finding
  is also listed in the evaluation. The feature does not change upstream code.

### Key Entities

- **Decoding layer**: the verification target. It is made of the codecs in FR-001, each an
  encoder and a decoder for one wire format.
- **Decoder contract**: the five-part statement of FR-004. It is stated once and checked for
  each codec.
- **Wire-format description**: the independent reference meaning of each format, written from
  upstream documentation. Every functional property is stated against it.
- **Restated copy**: the deductive track's version of a variable-length codec. It carries the
  unbounded proofs and is linked to the library by the agreement checks and the drift guard.
- **Operation inventory**: each operation in FR-002, with its status, its exclusion reason if
  any, its bounds and its crash-freedom result.
- **Property catalogue**: the functional properties, each with a statement, an upstream
  source, a bound, a status per track and its non-vacuity evidence.
- **Mutation measurement**: the classification of tool-generated mutants in FR-018, before and
  after the feature.
- **Trusted base statement**: what the proofs rely on without proving it.
- **Evaluation**: the findings, the comparisons and the recommendations.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every in-scope operation is proven crash-free within its stated bounds. Each bound
  below is a minimum. Values marked *estimate* are measured first during planning, as 002 did.
  - The fixed-width helpers and the fixed-size encoders are covered for every value of their
    inputs.
  - Each fixed-size decoder is covered for every byte sequence up to its encoded length: 8
    bytes for `CellId` and `Cell`, 24 for `Point`, 32 for `Cap` and 33 for `Rect`. That covers
    its whole domain. The planning probe measured each of these in 2–26 s.
  - The integer and cell-identifier vector decoders are covered for every byte sequence of
    up to N bytes, N at least 11. That is one byte beyond the longest variable-length
    integer (measured: 21–82 s once FR-019 grows on demand).
  - The string vector and point vector decoders are covered at the largest byte length that
    fits the per-change budget or the weekly tier. Implementation measures it, from the
    smallest length up. A decoder, or a branch of one, for which no useful length fits
    becomes a tool-limit exclusion with its measurements (clarification 2026-10-05). The
    probe found both out of reach at 4 GiB: the string vector from 4 bytes, the
    uncompressed point branch at 25 bytes, the compact branch at 11.
  - The vector encoders are covered for every sequence of up to N elements, N at least 2
    (measured at 2: under 10 s). Their round trip is proven for up to N elements, N at least
    1, by composition (research R4). A harness running encoder and decoder together does
    not fit.

  Larger bounds are recorded where they fit, and every unreached behaviour is named under
  FR-008.
- **SC-002**: At least 20 of the 24 inventory entries (83%) are in scope. Every excluded entry
  has a written reason, and every tool-limit exclusion has its measurements.
- **SC-003**: The property catalogue has at least one functional property for each in-scope
  operation, and it covers every area in FR-006. At least 90% of the entries whose operations
  are all in scope are proven by at least one track. The rest are listed with a reason.
- **SC-004**: Every proven catalogue property, on either track, has recorded non-vacuity
  evidence. Every crash-freedom proof has passing reachability checks.
- **SC-005**: For each of the five variable-length codecs in FR-010, the four properties listed
  there are proven for all sizes. Each restated copy passes its agreement checks. An injected
  divergence on either side fails CI. The deductive job completes in under 10 minutes on the
  standard CI machine, with its toolchain restored from cache.
- **SC-006**: The per-change decoding proofs complete in under 10 minutes on the standard CI
  machine: a hosted Linux runner with 4 virtual CPUs and 16 GB of memory. The existing `CellId`
  and `CellUnion` per-change legs stay within their budgets. Proofs over the budget run in the
  weekly tier and finish within its time limit.
- **SC-007**: Apart from the documentation in FR-020 to FR-022 and the fix in FR-019, the
  library's public API and behaviour are unchanged. The dependency lists of the
  published crates are unchanged. The project quality gate passes before and after the
  feature.
- **SC-008**: After the FR-019 fix:
  - each in-scope decoder that reads an element count, given an input of at most 16 bytes
    that declares its largest accepted count, returns an error after reserving at most 2 MiB;
  - run under a 256 MiB address-space limit, each such input ends with an error, not an
    abort;
  - valid encodings of 1,000,000 elements still decode to the same values;
  - the regression tests fail before the fix and pass after it.
- **SC-009**: The string-vector rule for out-of-order and out-of-range offsets (FR-020) is
  documented and proven within the bounds. The decoder agrees with the C++ reference
  vectors on every such case, and the robustness tests and the fuzz target pass.
- **SC-010**: Every in-scope operation that can panic documents when it panics. Every
  in-scope decoder documents its rejection conditions, what it does not validate, and any
  deviation from upstream. The evaluation lists each documentation addition.
- **SC-011**: All ten decoders named in FR-023 have a fuzz target in the nightly matrix and
  bounded robustness coverage. The committed C++ reference vectors cover every codec in
  FR-001, and the differential tests pass.
- **SC-012**: Every mutant that the mutation tool generates for the in-scope source files is
  classified (FR-018). The evaluation reports the counts before and after the feature. Every
  mutant that survives both the tests and the proofs has a written reason.
- **SC-013**: A maintainer who did not take part can answer from the evaluation alone:
  "continue or stop?", "with which method or methods?", "is the deductive track worth its
  cost?" and "what next?".
- **SC-014**: Every undefined or wrong upstream behaviour met during the work has a follow-up
  task, created before the affected proof was marked complete, and an entry in the evaluation.

## Assumptions

- **Audience**: the s2rst maintainer and contributors. Downstream users, including Python and
  WebAssembly users, benefit through decoders that cannot be made to abort or to return
  malformed values.
- **Builds on 001 and 002**: the feature reuses their verification setup. That covers the
  separate verification workspace, 001's cell model, the input generators for byte strings,
  the memory-guarded runner, the hand-written mutant campaign, the per-change CI legs and the
  weekly tier. A proof may rely on a `CellId` property that 001 proved, and the trusted base
  records each such reliance.
- **Assurance levels**: exhaustive bounded model checking remains the CI gate, as in 001 and
  002. Deductive proof on restated code is added for the unbounded results (Clarifications).
  Kernel-checked proofs and deductive proof of the library's source in place are out of
  scope.
- **Deductive toolchain**: the deductive verifier needs its own pinned compiler and solver.
  The project's stable-toolchain and MSRV rules apply to the workspace crates, which this
  feature does not move to that toolchain. The verifier is a separate tool, as the nightly
  fuzzing toolchain already is.
- **Bounds and budget**: the bounds in SC-001 come from the planning probe (research R0).
  The per-harness budget is 4 GB and 12 minutes, as in 002, and the weekly tier allows
  8 GB and 30 minutes. Implementation may raise the bounds. Lowering one needs a written
  justification in the evaluation. The string vector and point vector bounds are set by
  measurement during implementation (clarification 2026-10-05).
- **Reference semantics**: upstream C++ defines intended behaviour, and Go and Java are
  secondary references (Constitution I). The reference vectors come from the pinned oracle
  version, `v0.14.0-39-g8d5c2a8`. The survey read C++ sources at `v0.11.1-25`. Planning
  re-confirms each cited upstream behaviour against the pinned version.
- **Out of scope**:
  - the decoders listed in FR-003;
  - the comparison-only floating-point value types;
  - `Shape` and `Region` laws;
  - type-level invariants;
  - certified floating-point error bounds;
  - the 002 evaluation's third and fourth candidates (more `ShapeIndex` and `RegionCoverer`
    differential coverage, loop contracts for normalisation);
  - input validation in the language bindings (Constitution IV). The bindings inherit the
    core fixes.
- **Survey findings**: K1 to K4 and the missing fuzz targets were confirmed with concrete
  probes on 2026-10-04, before this specification was written.
- **Existing tests stay**: the proofs complement the existing unit, property, differential,
  robustness and fuzz tests. No existing test is removed.
