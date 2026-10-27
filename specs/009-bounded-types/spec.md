# Feature Specification: Bounded Types for Safer APIs, from the Core to the Bindings

**Feature Branch**: `009-bounded-types/1`

**Created**: 2026-10-27

**Status**: Implemented 2026-10-27, as local commits that are not pushed: `df687f0`, `8b0d338`,
`0a5b4c9`, `e514181`, `9a7cbc0`, `2738c80`, `4f5eb70`, `fcf5569`, `b1c44d7`, and the final
documentation commit. Verified: `make verify` 892/892, `make verify-release` 41/41, every
spec-009 mutant killed, and the full gate passed. Clarified
2026-10-27: FR-012, adopt the recommendation for the four prototyped candidates; FR-013,
breaking changes in the next minor release, with migration notes.

- **All four candidates were adopted**: `Level`, `Face`, `LevelMod` and `CellVertex`. R5's gate
  passed with about 1% of call sites genuinely needing an integer, against a threshold of 25%.
- **SC-001 to SC-005 and SC-007 to SC-009 are met.**
- **SC-006 is met except for one micro-benchmark**: the gate passes, the stub is unchanged, and
  the hot paths are flat, but `Cell::distance_to_cell`'s early-return path costs 14 more
  instructions (+7.9%), which is recorded (`BOUNDED_TYPES.md` §3.4).
- **Two findings beyond the survey**:
  - B11: the cell-id snap functions stored a raw level.
  - B12: reversed coverer levels hung a release build. The check is now in every build.

**Input**: User description: "009 - Check if we can use bounded types (new types validated at
instance creation) to make the use of certain APIs safer. One example use could be to create an
integer in a certain interval and pass it downstream. Compare ergonomics and safety and remember
complications will pass downstream from core to python and wasm."

## Context

### What a bounded type is here

A **bounded type** is a type whose values are checked once, when an instance is created. A
function that takes one needs no check of its own, documents no panic for that argument, and
cannot be handed an out-of-range value. Two forms exist in the library today:

- a **checked newtype** over an integer: `Level`, an integer in 0..=30;
- an **enum** for a small set: `Face` (six faces), `CellEdge` (four edges) and `RectVertex`
  (four corners).

The floating-point analogue is a value type that keeps its own invariant. `ChordAngle` has done
so since spec 007. Spec 008 left `Cap`'s validity as a precondition, because making it a type
property meant an API change ("Validity stays a precondition"). This feature is where that kind
of change is weighed.

### Where the library stands (read from the code and probed on 2026-10-27)

- **B1, `Level` exists, but its conversion from a raw integer panics.**
  - 23 public functions take `impl Into<Level>`, so callers can write `parent_at_level(5)`.
    Two take a plain `Level`, and ten return one (corrected at planning: the first count was of
    every public function that names `Level`).
  - `From<u8> for Level` panics above 30. The panic has moved from the callee into the
    conversion. Nothing is caught earlier, and the panic is still there in every build.
  - Rust's convention is that `From` does not fail and `TryFrom` does. `Level` has both
    (`TryFrom` from `u16`, `i32` and `usize`).
- **B2, deserialising `Level` skips its check.** `Level` derives its deserialiser, so a level of
  200 deserialises without error. So does `RegionCoverer`, which holds two levels and a
  `level_mod`. This is the gap that 007 closed for `ChordAngle` and 008 for `Cap` (K7). It was
  read from the code and needs a test to confirm it.
- **B3, `Face` exists, but the face constructors take a raw integer.**
  - `CellId::from_face`, `Cell::from_face` and `from_face_pos_level` take `impl Into<u8>`.
  - `CellId::from_face(7)` returns the invalid id `f000000000000000` without a panic. Python
    behaves the same way (probed).
  - Upstream's `FromFace(int)` does not check either. The silent result is upstream's, but the
    `Face` type that would prevent it already exists in the library.
- **B4, vertex and edge indices are inconsistent.**
  - `Cell::edge` takes `CellEdge`, and `s2::Rect::vertex` takes `RectVertex`.
  - `Cell::vertex` and `vertex_raw` take a `usize` and panic above 3.
  - `r2::Rect::vertex` takes an `i32` and reduces it modulo 4, as upstream's `GetVertex(int)`
    does with bit operations. Upstream's `S2Cell::GetVertex(4)` is vertex 0, where the
    library's `Cell::vertex(4)` panics. This undocumented deviation was found at planning.
- **B5, `RegionCoverer`'s fields are public.**
  - The `level_mod` setter clamps to 1..=3, as upstream's does.
  - Assigning the field directly, or deserialising it, skips the clamp. `covering` then calls
    `denormalize`, which panics in every build for a `level_mod` outside 1..=3.
  - `min_level ≤ max_level` is checked only in debug builds.
  - So one quantity has two policies: the setter clamps silently, and `denormalize` panics.
- **B6, the library is stricter than upstream in some places and looser in others.**
  - Upstream's coverer setters check levels and `level_mod` in debug builds and clamp them in
    release builds.
  - The library's level setters panic in every build above 30, through B1.
  - The library's `level_mod` setter clamps in every build, with no debug check.
  - Upstream takes a plain `int` everywhere.
- **B7, the bindings pass raw integers straight into panicking conversions.**
  - At least 14 Python methods and 16 WebAssembly methods take a level as a raw `u8`.
  - In Python, `CellId.parent_at_level(31)` and `RegionCoverer(min_level=31)` raise
    `pyo3_runtime.PanicException`. That class is not an `Exception` subclass, so
    `except Exception` does not catch it. `parent_at_level(256)` raises `OverflowError`.
    `RegionCoverer(level_mod=0)` silently becomes 1, and `level_mod=9` becomes 3. All of this
    was probed.
  - In WebAssembly, the same calls panic. The crate's own error model says that a panic aborts
    and poisons the instance, and keeps panics for genuine defects. `RegionTermIndexer`'s level
    setters call `Level::new` directly.
  - Each binding validates some arguments at the call site (`Cell.vertex`, `denormalize`), but
    not levels or faces.
- **B8, the conversion of a JavaScript number to a small integer parameter is unmeasured.**
  What a level of 286, −1, 3.5, `NaN` or a `BigInt` becomes on its way to a `u8` parameter
  (wrapped, truncated, rejected) has not been measured.
- **B9, most documented panics compare an argument with something else.** The coverage review of
  2026-10-26 (`verify/COVERAGE.md`) proved that 71 operations panic exactly where their
  documentation says. Some of these panics come from one argument's own range: a level above
  30, a face above 5, an index above 3. Many others relate an argument to the receiver or to
  another argument:
  - a level finer than the cell's own;
  - the parent of a face cell;
  - the children of a leaf;
  - `min > max`.

  A bounded type on its own removes only the first kind.

### Why now

- The coverage review gave every documented panic a precise, proven domain. That makes the
  candidates countable.
- The constitution says that the bindings SHOULD turn invalid input into language-native errors
  instead of relying on core panics (Principle IV). B7 shows that they do not do this for levels.
- Each decision passes downstream: a core signature change changes two bindings, the Python
  stub, the TypeScript declarations, the serialised forms and the verification harnesses.
  Weighing these costs once, with measurements, is cheaper than discovering them type by type.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - An evidence-based decision for every candidate (Priority: P1)

As the s2rst maintainer, I want every public operation whose arguments carry a range or validity
precondition classified. For each candidate, I want the design options compared on safety,
ergonomics, downstream cost and parity, and a recommendation. Then I can decide where bounded
types pay off without trying each one.

**Why this priority**: The question the user asked is whether bounded types can make APIs safer,
and at what cost. Without the classification, any change is anecdotal. The study also
separates the panics that a type can remove from those it cannot (B9), which bounds the benefit.

**Independent Test**: Hand the evaluation to a maintainer who did not take part. For any of the
71 operations with a documented panic, and for any public function that takes a level, face,
index or modulus as a raw integer, they can answer from the evaluation alone:

- whether a bounded type removes the panic or the silent wrong result;
- which option is recommended;
- what it costs at the call sites;
- what changes for Python and JavaScript users.

**Acceptance Scenarios**:

1. **Given** the evaluation, **When** a reviewer looks up `CellId::parent_at_level`, **Then** it
   shows:
   - the precondition's class: an absolute range for "level ≤ 30", and a relation for "level ≤
     the cell's level";
   - the current behaviour outside the domain, for each build;
   - the options compared;
   - the recommendation.
2. **Given** a candidate whose current behaviour differs from upstream's (B6), **When** the
   evaluation recommends an option, **Then** it states whether the option keeps, narrows or
   widens that difference, and how the difference is documented.
3. **Given** a precondition that relates two values (B9), **When** it is classified, **Then** the
   evaluation states whether any type-level construction removes it, and at what cost. If none
   does, it states that the precondition stays documented.

---

### User Story 2 - A prototype that measures the downstream cost (Priority: P1)

As the maintainer, I want the leading candidates prototyped through the core, the Python binding
and the WebAssembly binding, with their tests. Then the ergonomic and downstream costs are
measured on real call sites instead of guessed.

**Why this priority**: The user stressed that complications pass downstream. A change that is
pleasant in Rust can be awkward in Python or JavaScript, where ranges are not part of the type
system. B7 shows that the bindings are where today's panics surface as crashes.

**Independent Test**: Build the prototype. Run the three test suites and a fixed list of
out-of-range inputs against each binding. Count the call sites the change touched in the core,
its tests, its examples and documentation, and both bindings.

**Acceptance Scenarios**:

1. **Given** the prototype, **When** a Python caller passes a level of 31, a face of 7, a vertex
   index of 4 or a `level_mod` of 0, **Then** a Python-native exception is raised, of the kind the
   bindings use for that kind of argument. No `PanicException` is raised, and the value is not
   silently clamped or turned into an invalid result, unless the evaluation records why.
2. **Given** the prototype, **When** a JavaScript caller passes the same values, and also `NaN`,
   3.5, −1, 286 and a `BigInt` where a level is expected, **Then** a catchable JavaScript
   exception is thrown. The module instance stays usable, and no value is silently wrapped.
3. **Given** Rust code that passes a level as a literal, **When** it is written in each prototyped
   style, **Then** the evaluation shows the call as written. For a constant context, it shows
   whether an out-of-range literal is rejected when the code is compiled.

---

### User Story 3 - Adopting what the study recommends (Priority: P2)

As a user of the library in Rust, Python or JavaScript, I want out-of-range integers rejected
when the value is created or when it crosses into the library. A mistake should not surface later
as a panic, an aborted WebAssembly instance or a silently invalid cell.

**Why this priority**: This is the user value of the study. It depends on User Stories 1 and 2.
It covers the four prototyped candidates (FR-012), and the core's API may break for them in the
next minor release (FR-013).

**Independent Test**: For each adopted candidate:

- the operations that took the raw value no longer document a panic for its range;
- the constructor is the only place that rejects out-of-range input;
- the bindings' tests show the native error;
- the serialised forms of in-range values are unchanged.

**Acceptance Scenarios**:

1. **Given** an adopted candidate, **When** a reviewer reads the `# Panics` sections of the
   operations that take it, **Then** the range of that argument is no longer mentioned. The
   constructor's documentation states the range and its failure.
2. **Given** a serialised value of an adopted bounded type that is out of range, **When** it is
   deserialised, **Then** deserialisation fails. **Given** an in-range value, **Then** its
   serialised form is the same as before the change.
3. **Given** a `RegionCoverer`, **When** its settings are changed by any public means (setter,
   field, deserialisation), **Then** they cannot reach a state in which `covering` panics.
   Alternatively, the evaluation records why that cannot be guaranteed.

---

### User Story 4 - Proofs and records kept in step (Priority: P3)

As a contributor, I want the verification suite and the records to follow the change:

- an absolute-range precondition that a type removes is proven at the type's constructor
  instead of at every operation;
- the harnesses' domains say so;
- the inventories, `VERIFIED_TYPES.md` and the coverage record are updated.

**Why this priority**: The constitution makes out-of-date verification records a defect. It
comes after the decision and the adoption it records.

**Independent Test**: Run the per-change and release verification passes. Each adopted type's
constructor has a proof that it accepts exactly its range. Each operation that lost a panic has
no complement harness for that panic, and its crash proof ranges over the type. The records name
the change.

**Acceptance Scenarios**:

1. **Given** an adopted bounded type, **When** verification runs, **Then** a proof shows that its
   checked constructor succeeds exactly on the range and fails or panics exactly outside it.
2. **Given** an operation whose range panic was removed, **When** its records are read, **Then**
   its gap-1 entry points to the type's constructor proof instead of its own complement harness.

---

### Edge Cases

- **Arithmetic on bounded values.**
  - The next level after 30, and the one before 0.
  - The difference of two levels.
  - Loops over every level.
  - Indexing per-level tables.

  The study states, for each option, which of these stay checked, which become fallible and
  which need a conversion back to the integer.
- **Literals.** `parent_at_level(5)` in user code, tests and documentation examples. A constant
  `Level::new(31)` already fails when the code is compiled. The same call in ordinary code
  panics when it runs.
- **Relational preconditions** (B9): a level finer or coarser than the cell's own, the parent of
  a face cell, the children of a leaf, `min ≤ max`, a non-empty range. The study records whether
  a construction tied to the receiver is worth its cost, for example a level checked against a
  given cell, or whether they stay documented panics or become optional results.
- **Debug-only checks become every-build checks.** Moving a debug-only check into a constructor
  that always checks changes release behaviour. Where the result was unspecified, it becomes an
  error. Where upstream clamps in release builds (B6), this is a behavioural deviation that the
  constitution requires to be documented.
- **Clamping versus rejecting.** `RegionCoverer::level_mod` and `max_cells` clamp, as upstream
  does. A bounded type that rejects would disagree with upstream's setters, and one that clamps
  would hide caller mistakes. The study picks one policy per quantity and states it.
- **Python integers.**
  - Negative values, values above 255 (today an `OverflowError`), and very large values.
  - `bool`, which is an `int` subclass.
  - NumPy integer scalars.
  - An `IntEnum` for faces: is it accepted where an `int` is, and is an `int` still accepted
    where an `IntEnum` is offered?
  - The stub's types, and whether static checking can express any range at all.
- **JavaScript numbers** (B8): non-integers, negatives, values above 255, `NaN`, `Infinity`,
  `BigInt`; and whether the TypeScript declarations can name a small set (`0 | 1 | 2 | 3 | 4 |
  5`) usefully.
- **Serialised forms.** A bounded type serialised as a bare number must stay a bare number, so
  stored data reads back. The S2 binary decoders that read levels and faces already return
  errors and must not gain panics (Principle II).
- **Equality, ordering, hashing and display** of a newtype must match those of the integer it
  replaces wherever users rely on them: sort order, map keys, printed output.
- **Hot paths.** The coverer, the per-level metric tables and the cell hierarchy loops must not
  pay for validation on every step.

## Requirements *(mandatory)*

### Functional Requirements

**Survey and classification**

- **FR-001**: The feature MUST inventory the core's public operations whose arguments or
  receivers carry a range or validity precondition. The inventory includes at least:
  - the 71 operations of the coverage review's gap 1;
  - the 23 functions that take `impl Into<Level>`, and the two that take a plain `Level`;
  - every function that takes a face, a vertex or edge index, a child position, a level modulus
    or a byte length as a raw integer;
  - the value-type invariants: a unit-length point, latitude and longitude ranges, interval and
    rectangle validity, cap validity, and a normalised cell union.
- **FR-002**: Each inventoried precondition MUST be classified as one of:
  - (a) an absolute range of one argument;
  - (b) a small finite set;
  - (c) a relation between arguments, or with the receiver;
  - (d) a floating-point invariant;
  - (e) the validity of an existing value type.

  Each MUST record its current behaviour outside the domain, for debug and release builds: a
  panic, a silent wrong or invalid result, a clamp, or an error. Each MUST also record
  upstream's behaviour.
- **FR-003**: For each candidate, the evaluation MUST compare these design options, and say why
  any option does not apply:
  - **O1**, the status quo: a raw integer and a documented panic.
  - **O2**, a raw integer with a fallible result.
  - **O3**, a checked newtype, with and without an implicit conversion from raw integers that
    panics.
  - **O4**, an enum.
  - **O5**, clamping or saturating.
  - Compile-time rejection of out-of-range constants, as a property of each option.
- **FR-004**: The comparison MUST cover these dimensions:
  - **Safety**:
    - which panics and silent wrong results each option removes;
    - where an error is caught: when the code is compiled, when the value is created, or when
      it is used;
    - release-build behaviour.
  - **Ergonomics**:
    - call-site changes, counted in the core, its tests, its examples and documentation, and
      both bindings;
    - literals;
    - arithmetic and iteration;
    - the quality of error messages.
  - **Downstream**:
    - Python signatures, the stub, and exception types;
    - JavaScript and TypeScript signatures;
    - whether an error is an exception or a trap.
  - **Serialisation compatibility.**
  - **Verification**: harness domains and complement harnesses.
  - **Performance.**
  - **Parity of the API's shape and behaviour with upstream.**

**Prototype**

- **FR-005**: The feature MUST prototype at least four candidates through the core, the Python
  binding and the WebAssembly binding, with tests:
  - `Level`, including the question of its panicking `From<u8>` (B1);
  - the face argument of the face constructors (B3);
  - `Cell`'s vertex index (B4, against the `CellEdge` precedent);
  - the level modulus of `denormalize` and `RegionCoverer` (B5).

  The counts and behaviours in the evaluation for these four MUST be measured on the prototype.
- **FR-006**: In the prototype, every out-of-range value that reaches a binding MUST produce that
  language's native error:
  - in Python, the exception the bindings already use for the kind of argument, `ValueError`
    for values and `IndexError` for indices as in `tests/test_bounds_checks.py`;
  - in JavaScript, a catchable exception.

  It MUST NOT produce a `PanicException`, a WebAssembly trap, or a silent clamp, wrap or invalid
  result, unless the evaluation records a reason, for example parity with an upstream clamp.
- **FR-007**: Each binding MUST convert each bounded kind in exactly one place: one conversion per
  kind and binding, used by every method that takes that kind. Ad-hoc checks at individual call
  sites are not allowed for prototyped kinds.
- **FR-008**: B8 MUST be measured: what each listed JavaScript value becomes on its way to an
  integer parameter today. The prototype's handling MUST be stated for each value.

**Findings that stand on their own**

- **FR-009** (B2): Deserialising `Level`, and any adopted bounded type, MUST reject out-of-range
  values. The serialised form of in-range values MUST NOT change. Confirm with a test first, then
  fix it as its own change with a regression test, as 007 and 008 did.
- **FR-010** (B5): The evaluation MUST state how `RegionCoverer`'s settings stay within their
  domain under every public means of change: setters, fields and deserialisation. It MUST state
  whether `min_level ≤ max_level` becomes a construction-time check.
- **FR-011** (B6, Principle I): Where an adopted option rejects a value that upstream clamps or
  accepts, the rustdoc of the affected item MUST document the deviation and its reason, and the
  evaluation MUST list it.

**Adoption**

- **FR-012**: The feature MUST adopt the study's recommended option for each of the four
  prototyped candidates of FR-005, in the core and both bindings together, with tests. If the
  recommendation for a candidate is O1 (keep the status quo), the evaluation records why, and
  nothing is adopted for that candidate. Every other candidate is ranked as a follow-up (FR-018).
  *Clarified 2026-10-27 by the maintainer: the four prototyped candidates.*
- **FR-013**: Changes to the core's public API MAY break compatibility. They go into the next
  minor release (the project is `0.x` beta), with no deprecation period. Every breaking change
  MUST carry a migration note (the old call and the new one) in two places: a migration section
  of the evaluation, and a `BREAKING CHANGE` footer of its conventional commit. The repository
  has no changelog file; the history uses conventional commits. The
  Python and WebAssembly bindings change in the same release. A binding's own API changes only
  where the native type of an argument changes (for example an `IntEnum` for faces). Otherwise
  its signatures stay the same and only its errors change (FR-006). *Clarified 2026-10-27 by the
  maintainer: breaking changes in the next minor release, not deprecations and not bindings-only
  validation.*
- **FR-014**: For each adopted candidate:
  - every operation that took the raw value MUST drop that range from its `# Panics` section;
  - the type's constructor MUST document the range and the failure;
  - the Python stub MUST match the runtime;
  - the TypeScript declarations MUST describe the accepted values.
- **FR-015**: Adoption MUST NOT change any geometric result, and MUST NOT change the serialised
  form of in-range values or the S2 binary encodings.

**Verification and records**

- **FR-016**: For each adopted type, verification MUST prove that its checked constructor accepts
  exactly its range. The crash proofs of the operations that take it MUST range over the type.
  The complement harnesses for the removed range panics MUST be retired or re-pointed, and the
  records MUST say which.
- **FR-017**: The change MUST update in the same change the records it affects, as the
  constitution requires (v1.2.0, "Verification records"):
  - the suites' inventories;
  - `verify/COVERAGE.md`;
  - `VERIFIED_TYPES.md`;
  - the index of properties by kind and its rendered copy;
  - `KANI_ISSUES.md`, if a tool limit is met.
- **FR-018**: The evaluation MUST rank the candidates that were not adopted as follow-ups, with
  the measured or estimated cost of each. It MUST include the floating-point invariants (class d)
  and the validity of value types (class e).

### Key Entities

- **Bounded type**: a type whose values are checked when an instance is created. It is a checked
  newtype over an integer (`Level`) or an enum (`Face`, `CellEdge`, `RectVertex`). Its range is
  the domain of the operations that take it.
- **Candidate**: a public operation's argument or receiver that carries a precondition. It has a
  class (FR-002), current behaviour per build, upstream behaviour and a recommendation.
- **Design option**: O1–O5 (FR-003), each with its safety, ergonomics and downstream profile.
- **Boundary conversion**: each binding's single conversion from a language-native value to a
  bounded type (FR-007), with the native error it raises.
- **Evaluation**: the decision record. It holds the classification, the comparison, the
  prototype's measurements, the recommendation per candidate, the deviations from upstream and
  the ranked follow-ups.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the operations of FR-001 are classified, each with its out-of-domain
  behaviour in both builds and upstream's behaviour.
- **SC-002**: Every candidate has a recommendation backed by every dimension of FR-004. For the
  four prototyped candidates, every count and behaviour cited is measured, not estimated.
- **SC-003**: In the prototype, none of the listed out-of-range inputs produces a crash, an
  aborted instance, or a silently clamped, wrapped or invalid result in either binding, except
  where the evaluation records a reason. The list:
  - levels −1, 31, 255, 256 and 2⁴⁰;
  - faces −1, 6 and 7;
  - vertex indices −1 and 4;
  - level moduli 0 and 4;
  - in JavaScript also `NaN`, 3.5, `Infinity` and a `BigInt`.
- **SC-004**: For each adopted candidate, the number of operations whose panic documentation
  mentions that argument's range drops to zero. The constructor's range is proven exact.
- **SC-005**: Deserialising out-of-range values of each adopted bounded type, and of `Level`,
  fails in 100% of the tested cases. In-range values serialise exactly as before.
- **SC-006**: The full quality gate passes before and after. No existing test is removed. The
  Python stub matches the runtime. The deterministic benchmarks of any touched hot path show no
  slowdown.
- **SC-007**: A maintainer who did not take part can answer from the evaluation alone, for any
  candidate: "can a bounded type remove this panic, what does it cost in Rust, Python and
  JavaScript, and is it recommended?"
- **SC-008**: Every behavioural deviation from upstream that an adopted option introduces is
  documented in the rustdoc and listed in the evaluation.
- **SC-009**: Every breaking change to the core's public API has a migration note (FR-013).
  Following the notes alone, a caller can update each changed call site.

## Assumptions

- **Audience**: the maintainer decides; users of all three surfaces (Rust, Python, JavaScript)
  benefit.
- **The prototype's candidates** are the integer and index kinds (FR-005). They are cheap to
  change, `Level` and `Face` already exist, and they cover all three option shapes (newtype,
  enum, clamp). The floating-point invariants (a unit-length point, latitude and longitude
  ranges, interval, rectangle and cap validity) and the normalised cell union are surveyed and
  ranked (FR-018), but not prototyped. Making them type properties reaches through the whole
  geometry API and is a feature of its own.
- **Compatibility**: `0.x` beta, one shared version for all crates. Breaking the core's API for
  the adopted candidates is accepted in the next minor release, with migration notes (FR-013).
  Downstream users of the bindings mostly see only better errors.
- **Relational preconditions stay preconditions** unless the study finds a construction that pays
  for itself. Bounded types are not expected to remove them.
- **Upstream reference**: C++ at v0.14.0-40, which takes plain `int` levels, faces and moduli and
  checks them with debug-only checks, clamping in the coverer's setters. Go is secondary. The API
  shape (a type instead of an `int`) is a Rust-side choice. The constitution's parity principle
  governs results and errors, not parameter types.
- **Rust conventions**: `From` does not fail and `TryFrom` does, as the standard library
  documents. Out-of-range constants in a constant context are rejected when the code is
  compiled. The workspace MSRV (1.88) allows both.
- **Python**: plain `int` stays the parameter type unless the study shows a better choice. An
  `IntEnum` is considered for faces. `PanicException` derives from `BaseException` (probed).
- **WebAssembly**: a panic aborts the instance and poisons it, as the crate's error model states.
  Expected failures are JavaScript exceptions.
- **Builds on 001–008 and the coverage review**: the gap-1 harnesses and their domains, the
  release verification pass, the bindings' bounds-check tests, and the decoders' error model.
- **Out of scope**: changing any geometric algorithm; changing the S2 binary encodings; the
  query and builder layers' option structs beyond `RegionCoverer` (surveyed only); typestate for
  `CellUnion` normalisation (surveyed only).
