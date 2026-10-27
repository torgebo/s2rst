# Research: Bounded Types for Safer APIs (spec 009)

**Date**: 2026-10-27. **Inputs**: [spec.md](spec.md); the maintainer's planning request: "Add
red/green tests to missing functionality/crashes (for example in python). Check if related
function signatures containing type T should be rewritten to `impl Into<T>`."

Each item gives the decision, the rationale and the alternatives. Probes run at planning are
marked **probed**, with what they showed. Items read from source code are marked **read**.
Anything still unconfirmed is marked "to be confirmed", with the test that will confirm it.

## R0. Probes run at planning

| # | Probe | Result |
|---|-------|--------|
| P1 | The installed Python bindings, on out-of-range levels | `parent_at_level(31)` and `RegionCoverer(min_level=31)` raise `pyo3_runtime.PanicException`, which is **not** an `Exception` subclass. `256` and `−1` give `OverflowError`, `2**70` gives `OverflowError` ("too large to convert to C long"), and `3.5` and `"5"` give `TypeError`. `True` is accepted as level 1 |
| P2 | Python, on faces and moduli | `CellId.from_face(7)` returns `CellId(Invalid: f000000000000000)`. `RegionCoverer(level_mod=0)` and `level_mod=9` become 1 and 3, silently. `CellUnion.denormalize(0, 4)` raises `ValueError` (validated at the call site, A12). `Cell.vertex(4)` raises `IndexError` (A1) |
| P3 | `rustc` 1.99.1: a literal passed through `impl Into<Level>` ([probe/into_inference.rs](probe/into_inference.rs), compiled with `--cfg case_a`, `case_b`, `case_c` and `case_const`) | With `From<u8>` as the only integer source, `parent_at_level(5)` compiles. With a second one (`From<u16>`), every literal call fails: the literal falls back to `i32`, and there is no `Level: From<i32>` (E0277). Without `From<u8>`, a literal does not compile. `const BAD: Level = Level::new(31)` is rejected when the code is compiled (E0080, "evaluation panicked"), and `const { Level::new(5) }` works inline |
| P4 | wasm-bindgen 0.2.129's ABI for `u8` parameters (read) | `type_abi_as_u32!(i8 u8 i16 u16)`: `from_abi(js: u32) -> u8 { js as u8 }`. A JavaScript number reaches a wasm `i32` parameter through ToInt32. So 286 becomes 30, −1 becomes 255, `NaN` and `Infinity` become 0, 3.5 becomes 3, and a `BigInt` throws a JavaScript `TypeError`. `parentAtLevel(286)` therefore answers for level 30 without any error. To be confirmed by WA-01 |
| P5 | Upstream `s2region_coverer.cc` and `r2rect.h` at v0.14.0-40 (read) | The setters `ABSL_DCHECK` levels and `level_mod`, then clamp them (`max(0, min(30, ·))`, `max(1, min(3, ·))`). `R2Rect::GetVertex(int k)` uses `k`'s low two bits, so `S2Cell::GetVertex(4)` is vertex 0. `FromFace(int)` does not check |
| P6 | Signature audit of `core/src` (grep) | 23 public functions take `impl Into<Level>`, 2 take `Level`, 10 return a `Level`. 4 take a face as `impl Into<u8>` (`CellId::from_face`, `Cell::from_face` and both `from_face_pos_level`). 12 coordinate functions take `Face`. 4 `Cell` methods take `CellEdge` and one `Rect` method takes `RectVertex`. `Cell::vertex` and `vertex_raw` take `usize`, and `r2::Rect::vertex` takes `i32`. `denormalize`, `RegionCoverer::level_mod` and `RegionTermIndexer`'s `Options::level_mod` (a `usize`) take raw moduli |
| P7 | Call sites (grep, an estimate; exact counts come from the compiler in R2) | The level-taking functions are called about 820 times: `core/src` 442 (171 with a literal), `core/tests` 130, `core/benches` 45, `core/examples` 7, `python/src` 19, `wasm/src` 21, `wasm/tests` 24, `verify/` 129. `from_face(` is called about 500 times (`core/src` 372, `core/tests` 94, `benches` 12, bindings 6, `verify` 16). `Cell::vertex`/`vertex_raw` about 79 times in `core` |

## R1. Should signatures with a bounded type `T` become `impl Into<T>`? (the maintainer's question)

**Decision: no. The direction is the opposite.** Every adopted bounded type is taken as plain
`T`:

- The 23 `impl Into<Level>` parameters become `Level`.
- The four `impl Into<u8>` face parameters become `Face`.
- The parameters that already take `Level`, `Face`, `CellEdge` or `RectVertex` stay as they are.
- No signature is rewritten to `impl Into<T>`. The full audit, with a verdict per signature, is
  in [contracts/signature-audit.md](contracts/signature-audit.md).

**Rationale**:

1. **`impl Into<T>` helps only when `T` has infallible conversions from types that callers
   hold.** A bounded type's natural sources are raw integers, and converting a raw integer to a
   bounded type can fail by definition. The standard library's contract for `From` is that it
   must not fail. A failing conversion belongs in `TryFrom`, and `impl TryInto<T>` would make
   every callee fallible. Today's `impl Into<Level>` works only because `From<u8> for Level`
   breaks that contract by panicking. That panic is the crash that Python users see as
   `PanicException` (P1).
2. **Inference of literals is fragile (P3).** `parent_at_level(5)` compiles only while `u8` is
   the single integer type with `Into<Level>`. Adding any second source, even an infallible one
   such as `From<u16>`, breaks every literal call site in the workspace and in downstream code.
3. **Without `From<u8>`, `impl Into<Level>` accepts only `Level`.** That is `Level` with
   generics added: one instantiation per caller type, a vaguer rustdoc, and worse error
   messages.
4. **Plain `T` makes the compiler find every unchecked call site.** Once `From<u8>` is gone,
   every place that passes a raw integer stops compiling, in the core, its tests, examples,
   benches, both bindings, the fuzz targets and the verification crate. That error list is the
   exact count FR-005 asks for. It is also how FR-007's single conversion per binding is
   enforced: a binding cannot keep passing a raw `u8` by accident.

**Where `impl Into<T>` would still be right**: a parameter whose type has genuine infallible
sources that callers commonly hold, as `impl Into<Angle>` could for a `ChordAngle`. No adopted
type has one. The audit lists the non-bounded parameters that already take `impl Into<…>`
(`builder::Graph::vertex(impl Into<VertexId>)`, `edge(impl Into<EdgeId>)`). Those are out of
scope, and the evaluation ranks them as follow-ups.

**Alternatives considered**:

- **(a) Keep `impl Into<Level>` with the panicking `From<u8>`** (the status quo, O3 with an
  implicit conversion). There is no safety gain, the panic stays in every build, and the
  inference stays fragile.
- **(b) `impl TryInto<Level>` and fallible results everywhere.** Every hierarchy operation would
  return a `Result` for a check that its caller can do once. Rejected.
- **(c) Keep `impl Into<Level>` but drop `From<u8>`.** It accepts the same values as `Level`, with
  generic noise. Rejected.

## R2. `Level`: drop the panicking `From<u8>`, keep the explicit constructors

**Decision**:

- Remove `From<u8> for Level`.
- Keep `Level::new(u8)`: a `const fn` that panics, documented, and named at the call site.
  - In a constant context (`const { Level::new(5) }`, or a `const` item) an out-of-range literal
    is rejected when the code is compiled (P3).
  - In ordinary code, `Level::new(5)` is the idiom for a literal. Its range is visible where it
    is written.
- Add `TryFrom<u8>`. This is now possible: before, it clashed with the blanket `TryFrom` that
  `From<u8>` implied.
- Add `TryFrom<i64>`, the bindings' common source type.
- Keep `TryFrom<u16>`, `TryFrom<i32>` and `TryFrom<usize>`.
- All `TryFrom` impls use the error type of R3.
- Add `checked_add(u8)` and `checked_sub(u8)`, returning `Option<Level>`. The `+ u8` and `- u8`
  operators keep panicking at the ends of the range, as documented, like slice indexing.
- Keep `PartialEq<u8>` and `PartialOrd<u8>`, so that `level == 30` and `level < 5` still read
  naturally.

**Rationale**:

- After R1, the call site makes the range explicit: `Level::new(5)` for literals, and
  `Level::try_from(n)?` for values computed at run time.
- About 280 literal call sites (P7, mostly tests and benches) change mechanically.
- The sites that already pass a `Level` do not change: `self.level()`, `level + 1`, the coverer's
  fields.

**Alternatives considered**:

- **A `level!(n)` macro expanding to `const { Level::new(n) }`.** It checks the range when the
  code is compiled at every literal site, at the cost of a new exported macro. Deferred. The
  evaluation counts literal sites, and the macro is a follow-up if `Level::new(n)` proves noisy.
- **Thirty-one associated constants (`Level::L5`).** Rejected as noise.
- **`Level::new` returning `Option`, as `NonZeroU8::new` does.** That breaks every constant use
  and every literal site twice over. Rejected.

## R3. One error type for every bounded conversion

**Decision**: add one public error type to `s2`, `OutOfRange`. It carries:

- what was being converted: a level, a face, a cell vertex index or a level modulus;
- the offending value, as an `i64`;
- the accepted range.

It implements `Display` (for example "level must be in 0..=30, got 31") and
`std::error::Error`. Every bounded type's `TryFrom` impls return it.

Two existing error types change, which is a breaking change under FR-013:

- `Level`'s `&'static str` becomes `OutOfRange`.
- `Face`'s `u8` becomes `OutOfRange`.

**Rationale**:

- Each binding then needs one generic conversion: Python's `ValueError` or `IndexError` and
  JavaScript's `Error`, both carrying the `Display` text (FR-007).
- The two existing error types are inconsistent with each other and lose the value.

**Alternatives considered**:

- **Per-type error types.** That means four types with one shape.
- **`std::num::TryFromIntError`.** It has no value and no range in its message.
- **Reusing `builder::S2Error`.** It belongs to the builder layer.

## R4. The face constructors take `Face`

**Decision**: `CellId::from_face`, `Cell::from_face`, `CellId::from_face_pos_level` and
`Cell::from_face_pos_level` take `Face`.

- Loops `for f in 0..6` become `for face in Face::ALL`, which already exists.
- Literals `from_face(0)` become `from_face(Face::F0)`.
- `Face::from_u8`, which panics, stays as the explicit counterpart of `Level::new`. Its
  documentation already says that it panics.

**Rationale**:

- `from_face(7)` silently builds an invalid id (P2, B3). The type that prevents this exists, and
  the face constructors are the only public entry points that bypass it.
- The change is mechanical. About 500 call sites change (P7), almost all of them literal faces
  or loops in tests, where `Face::Fk` also reads better.
- The deviation from upstream is only one of signature: upstream's `FromFace(int)` gives an
  undefined id for 6 or 7, and the library makes that unrepresentable. It is documented
  (FR-011).

**Alternatives considered**:

- **Keep `impl Into<u8>` and add a check that panics.** That adds a new panic where a type
  already exists.
- **`impl Into<Face>` with a `From<u8>`.** This is R1's anti-pattern.

## R5. `Cell`'s vertex index: a `CellVertex` enum, behind a decision gate

**Decision**: add `CellVertex`, which mirrors `RectVertex` and `CellEdge`:

- the variants `LowerLeft = 0`, `LowerRight`, `UpperRight` and `UpperLeft`, in (u, v) and in
  counter-clockwise order;
- `ALL`, `next()` and `prev()`, which wrap as upstream's `k + 1` does;
- `index() -> usize`;
- `TryFrom<usize>` and `TryFrom<i64>`, which return `OutOfRange`;
- serde derived as for `CellEdge`.

`Cell::vertex` and `Cell::vertex_raw` then take a `CellVertex`, and their every-build panic is
removed.

**Gate**: the prototype counts how each call site is rewritten. The enum is adopted unless more
than a quarter of the call sites need an integer conversion (`CellVertex::ALL[k]`, `.index()`
inside arithmetic), which would mean that callers think in integers. In that case the evaluation
recommends O1 and records the measurement:

- keep `usize` and its panic, as slice indexing does;
- document the deviation from upstream's wrapping.

**Rationale**:

- `cell.edge(CellEdge::Right)` and `cell.vertex(1)` sit side by side, and the vertex is the
  odd one out.
- The enum carries upstream's wrapping (`GetVertex(k + 1)`) as `next()`. It is a type-checked
  replacement for both the panic and the wrap.
- The bindings already reject index 4 (Python `IndexError`, A1 and A2; wasm `Result`).
  Downstream behaviour therefore does not change, and they move to the shared conversion.

**Alternatives considered**:

- **Reusing `RectVertex`.** Its documentation describes latitudes and longitudes, and cells
  already have their own `CellEdge`.
- **Wrapping modulo 4, as upstream does (O5).** This turns a caller's mistake into a silent
  vertex. It stays the library's choice only for `r2::Rect::vertex(i32)`, which already
  matches upstream, and the audit lists it as a follow-up.

## R6. `LevelMod`, and `RegionCoverer`'s settings

**Decision**:

- Add `LevelMod`, a checked newtype over 1..=3, with:
  - `new` (a `const fn` that panics);
  - `get() -> u8`;
  - `branching_factor() -> u32` (4, 16 or 64);
  - `DEFAULT` (1);
  - `TryFrom<u8>`, `TryFrom<usize>` and `TryFrom<i64>`, which return `OutOfRange`;
  - a validating `Deserialize`.

  A newtype and not an enum, because callers use the modulus in arithmetic
  (`(level − min_level) % level_mod`).
- `CellUnion::denormalize(Level, LevelMod)` drops its `assert!`.
- `RegionCoverer::level_mod` (field and setter) and `RegionTermIndexer`'s `Options::level_mod`
  take `LevelMod`. The setter stops clamping.
- **FR-010**:
  - The fields stay public. Their types now guarantee the ranges, through assignment and
    deserialisation alike.
  - `min_level ≤ max_level` is a relation (class c). It stays a documented precondition of
    `covering` and `interior_covering` (a debug check, as upstream's `DCHECK`), stated in their
    `# Panics` sections.
  - Both bindings check it at their boundary and raise a native error:
    - Python's `RegionCoverer` and `RegionTermIndexer` in their constructors, which receive
      both levels;
    - wasm in the covering methods, because its setters are chained one at a time.

**Rationale**:

- One quantity gets one policy: reject.
- The every-build panic of `denormalize` and the field bypass (B5) both disappear.
- Rejecting instead of clamping deviates from upstream's release-build clamp. It agrees with
  upstream's debug-build `DCHECK`, and it is documented (FR-011).
- Making the fields private would be a much larger break for a relation that types cannot carry
  anyway.

**Alternatives considered**:

- **An enum `LevelMod { One, Two, Three }`.** That needs a conversion back to an integer at every
  use.
- **Keeping the clamp in the setter.** It keeps the silent repair that FR-006 forbids without a
  reason, and keeps the setter and `denormalize` inconsistent.
- **Private fields with a validating builder.** The validation would depend on the order the
  builder is called in, and every field use would break.

## R7. Serialised forms

**Decision**:

- `Level` and `LevelMod` keep their derived `Serialize`. Each gets a hand-written `Deserialize`:
  - it reads a private `#[derive(Deserialize)] #[serde(rename = "Level")] struct Raw(u8)`;
  - it converts through `TryFrom`;
  - it reports the `OutOfRange` text as a `custom` error.

  This is the pattern of `Cell`'s and `Cap`'s deserialisers. The wire shape is therefore that of
  the newtype struct in every format: a bare number in JSON, as today.
- `Face` and `CellEdge` keep their derived serde, which already rejects unknown variants.
  `CellVertex` derives it the same way.
- `RegionCoverer` and the term indexer's `Options` keep their derives. Their typed fields
  validate.

**Red/green**:

- `serde_json::from_str::<Level>("31")` is `Ok` today (B2, read) and must be an error.
- `"30"` must round-trip byte for byte.
- A `RegionCoverer` with `"level_mod": 7` must fail to deserialise.

**Alternatives considered**:

- **`#[serde(try_from = "u8", into = "u8")]`.** Formats that tag newtype structs, such as RON,
  would then lose the `Level(…)` wrapper, which changes the serialised form.

## R8. The Python boundary

**Decision**:

- One extractor type per bounded kind in a new `python/src/bounded.rs`: `LevelArg`, `FaceArg`,
  `LevelModArg` and `CellVertexArg`. Each implements PyO3 0.30's `FromPyObject`.
- Each reads any Python integer through `__index__` (an `i64`):
  - a Python integer outside `i64` maps to the same error as any out-of-range value;
  - a `float` or `str` keeps PyO3's `TypeError`;
  - `bool` and NumPy integers are accepted, as Python's own indexing accepts them.
- Each converts with `TryFrom<i64>` and raises a native exception:
  - `ValueError` for levels, faces and moduli;
  - `IndexError` for vertex indices, as A1 and A2 already do.
- The `#[pymethods]` take the extractor types in place of `u8`, `usize` and `i64`. The stub
  keeps `int`, so `stubtest` sees no change in signature.
- No Python `IntEnum` for faces in this feature. It would change the stub and the user API for
  no added safety, given the extractor. It is ranked as a follow-up.

**To be confirmed by PY-00**: PyO3 re-wraps only `TypeError`s raised during argument
extraction (with the argument's name), so the `ValueError` reaches the caller unchanged. **Confirmed
2026-10-27 by PY-01**: `parent_at_level(31)` raises `ValueError: level must be in 0..=30, got
31` straight from `LevelArg`'s extraction. If it
does not, each extractor is called in the method body instead. That is still one conversion per
kind.

**Alternatives considered**:

- **Validating in each method body.** This is today's ad-hoc pattern (A1, A12), which FR-007
  forbids for the prototyped kinds.
- **Keeping `u8` parameters.** `256` and `−1` would keep giving `OverflowError` while `31`
  gives `ValueError`: two exceptions for one mistake.

## R9. The WebAssembly boundary

**Decision**:

- Each method that takes a bounded kind takes an `f64` in place of `u8` or `usize`, converted by
  one helper per kind in a new `wasm/src/bounded.rs`. Each helper rejects `NaN`, ±∞,
  non-integers and out-of-range values, returning `Err(js_err(OutOfRange…))`. The method returns
  `Result<_, JsValue>`, so JavaScript sees a thrown `Error`.
- The TypeScript type stays `number`, and the rustdoc on each method (which becomes JSDoc in the
  `.d.ts`) states the range and the thrown error (FR-014).
- Literal-union types (`0 | 1 | 2 | 3 | 4 | 5`, through wasm-bindgen's `unchecked_param_type`)
  are a follow-up. Return values such as `face` would need the same typing, or TypeScript callers
  would need casts.

**Rationale**:

- `f64` is the only primitive parameter through which JavaScript's value arrives exactly (P4).
- `u8` and `i32` both pass through ToInt32, which wraps silently.
- A `BigInt` throws a `TypeError` in the glue before any Rust runs, which is catchable.
- A string such as `"5"` is coerced to 5 by JavaScript's own ToNumber, which is accepted and
  documented.

**Alternatives considered**:

- **`JsValue` parameters.** These give exact type checks, but the TypeScript type becomes `any`.
- **Implementing `FromWasmAbi` for a wrapper.** Its `from_abi` is an `unsafe fn`. Rejected.

## R10. Red/green tests (the maintainer's request)

**Decision**:

- Every crash, silent wrong result and missing check found by the survey gets a test that is
  written first and run against the unfixed tree. The test must fail ("red"), and the failure is
  recorded. The fix then turns it green.
- A finding's test and fix are committed together, so `make test` stays green at every commit.
- The red evidence goes in a table in `BOUNDED_TYPES.md`: the test, the finding, the red output
  (exception type or panic message) and the commit that turned it green. This is the convention
  of `python/tests/test_bounds_checks.py`.
- The tests run through surfaces that do not change, so the same test is red before and green
  after:
  - **Python** (`python/tests/test_bounded_args.py`): through the unchanged Python signatures.
  - **WebAssembly** (`wasm/tests/bounded_args.rs`, run by `wasm-pack test --node`): through the
    real JavaScript glue. The test wraps the Rust object as a `JsValue` (wasm-bindgen generates
    `From<T> for JsValue` for exported classes) and calls the method by name with
    `js_sys::Reflect` and `Function::call1`, passing raw JavaScript values: 286, −1, `NaN`, 3.5,
    `Infinity` and a `BigInt`. Rust-level calls would bypass the ToInt32 conversion that P4 is
    about.
  - **Core runtime findings** (B2 serde, B5 `level_mod` bypass by deserialisation): sibling
    `*_tests.rs` files.
  - **Core type-level guarantees** (`from_face(7)`, `Level` from a raw `u8`, assigning
    `level_mod: 7`): `compile_fail` doctests on the types. Each is paired with a passing twin
    that differs only in the fixed point, so a failure for any other reason is visible.
    - Red: the snippet compiles today, so the `compile_fail` doctest fails.
    - Green: after the change it does not compile.
- [contracts/red-green-tests.md](contracts/red-green-tests.md) lists every test with its input,
  today's (red) behaviour and the expected (green) one.

**Alternatives considered**:

- **Committing red tests as strict `xfail` first.** That gives CI-visible red evidence, but only
  for Python: there is no Rust equivalent, and it doubles the commits. The recorded red output
  serves all three surfaces.

## R11. The verification suite

**Decision**:

- Each adopted type gets a constructor proof:
  - a `c_` harness: `try_from` succeeds exactly on the range and round-trips the value;
  - an `n_` harness: `new` panics exactly outside it, with the trailing-cover convention of
    `verify/COVERAGE.md`.
- The operations that took the raw value draw a symbolic value of the type (`domain.rs` gains
  `any_level()`, `any_face()`, `any_level_mod()` and `any_cell_vertex()`).
- Gap 1's complements for the removed panics are retired or re-pointed:
  - `crash::n_raw_level`, over 13 operations, becomes the `Level::new` complement;
  - `cu_crash::n_denormalize__n2`'s `level_mod` half becomes `LevelMod::new`'s;
  - `cl_crash::n_vertex` becomes `CellVertex::try_from`'s.
- Mutants M-803 (`Level::new` unchecked) and M-806 (`denormalize`'s `level_mod` unchecked) are
  regenerated against the moved checks. One new mutant per new constructor makes it fail open.
- The 129 call sites in `verify/` are updated with the core. The per-change suite and the release
  pass are re-run in full, because the signatures change in most harnesses.

## R12. Where the evaluation and the migration notes live

**Decision**: a root-level `BOUNDED_TYPES.md`, beside `CORE_TRAITS.md` and `VERIFIED_TYPES.md`.
It holds:

- the classification (FR-001, FR-002);
- the comparison per candidate (FR-003, FR-004), with the prototype's measured counts;
- the red/green table (R10);
- the deviations from upstream (FR-011);
- the migration notes, old call and new (FR-013);
- the ranked follow-ups (FR-018).

Each breaking commit's `BREAKING CHANGE` footer points to its section. `CONTRIBUTING.md` gains
one line: new range-restricted parameters take a bounded type, and conversions from raw integers
are `TryFrom`, never `From`.

## R13. Performance

**Decision**: this is not a performance change, but it touches hot paths: the coverer, the
per-level tables and the cell hierarchy. The gungraun benches `core/benches/cell_id.rs`,
`region_coverer.rs` and `cell.rs` are run before and after the `Level` and `CellVertex` commits
(`make baseline` first), and the instruction counts are recorded in the evaluation.

**Expectation**:

- No change from `impl Into<Level>` becoming `Level`: the same code, minus one generic
  instantiation.
- `CellVertex` may remove a bounds check from `vertex_raw`.

## R14. Order of work

**Decision**:

1. The survey and classification (no code).
2. B2, the stand-alone deserialisation fix.
3. The additive `OutOfRange` and `TryFrom` impls.
4. `Level`.
5. `Face`.
6. `LevelMod` and `RegionCoverer`.
7. `CellVertex` (gated).
8. The verification suite and the records.
9. The evaluation's measurements and migration notes.

Each of steps 2–7 is one commit with its red/green tests, its binding changes and its
documentation. Every commit keeps `make test` green.

**Rationale**:

- `Level` first: it has the most call sites, and the compiler's error list gives the
  measurement.
- Steps 2 and 3 are additive, so the breaking commits stay small.
