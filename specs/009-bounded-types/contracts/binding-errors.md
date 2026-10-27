# Contract: Errors at the Binding Boundaries

What a Python or JavaScript caller sees for each bounded kind after the change (spec FR-006,
FR-007, FR-010; research R8, R9). There is one conversion per kind and binding:
`python/src/bounded.rs` and `wasm/src/bounded.rs`. Every method that takes the kind uses it.

## Messages

Every message comes from the core's `OutOfRange` `Display` (research R3). A binding adds nothing
except, in JavaScript, the description of a value that is not an integer.

| Kind | Range | Message for an integer out of range | Extra JavaScript message for a non-integer |
|------|-------|-------------------------------------|--------------------------------------------|
| level | 0..=30 | `level must be in 0..=30, got 31` | `level must be an integer in 0..=30, got NaN` (or `3.5`, `Infinity`) |
| face | 0..=5 | `face must be in 0..=5, got 7` | `face must be an integer in 0..=5, got 3.5` |
| level modulus | 1..=3 | `level_mod must be in 1..=3, got 0` | `level_mod must be an integer in 1..=3, got NaN` |
| cell vertex index | 0..=3 | `cell vertex index must be in 0..=3, got 4` | `cell vertex index must be an integer in 0..=3, got 3.5` |
| cell edge index | 0..=3 | `cell edge index must be in 0..=3, got 4` | `cell edge index must be an integer in 0..=3, got NaN` |
| coverer levels (relation) | `min_level ≤ max_level` | `min_level (10) must not exceed max_level (5)` | n/a |

The exact wording is fixed by the first test that pins it. The structure is fixed here: the
kind, the range, the value.

## Python

| Input | Level, face, modulus | Vertex or edge index | Relation |
|-------|----------------------|--------------|----------|
| an `int` (or `bool`, or an object with `__index__`) in range | accepted | accepted | n/a |
| an `int` out of range, including beyond 64 bits | `ValueError` | `IndexError` | n/a |
| `float`, `str`, `None` or another non-integer | `TypeError` (PyO3's, naming the argument) | `TypeError` | n/a |
| `min_level > max_level` in `RegionCoverer(…)` or `RegionTermIndexer(…)` | n/a | n/a | `ValueError` at construction |

Invariants:

- No input in this table raises `pyo3_runtime.PanicException` or `OverflowError`.
- No input is clamped or wrapped.
- The stub (`_core.pyi`) keeps `int` for every such parameter.
- The docstring of each method states the range and the exception.

## JavaScript (wasm)

| Input | Level, face, modulus, vertex index | Relation |
|-------|-------------------------------------|----------|
| a `number` that is an integer in range (5 and 5.0 alike) | accepted | n/a |
| a `number` that is an integer out of range | throws the message | n/a |
| `NaN`, ±`Infinity`, a non-integer `number` | throws the message | n/a |
| a `BigInt` | throws an `Error` from the glue, before Rust runs ("expected a number argument, found bigint" in a debug build; observed at red time, 2026-10-27) | n/a |
| a `string` | coerced by JavaScript's ToNumber (`"5"` is 5, `"x"` is `NaN` and throws) | n/a |
| `min_level > max_level` when a covering method is called | n/a | throws the message |

"Throws the message": the crate's one error funnel (`wasm/src/error.rs`, `js_err`) throws the
message as a JavaScript string, as it does for every expected failure of the wasm API, so `catch`
sees `"level must be in 0..=30, got 31"`. Only the glue's own `BigInt` rejection is an `Error`
object. Throwing `Error` objects from the funnel would change every error of the wasm API, so
it is a follow-up (`BOUNDED_TYPES.md` §7), not part of this feature. Observed at green time,
2026-10-27.

Invariants:

- No input in this table traps (`RuntimeError: unreachable`).
- The instance stays usable after every throw.
- No input is silently truncated, wrapped or clamped.
- The `.d.ts` type stays `number`, and the JSDoc states the range and `@throws`.

## Rust (core)

| Construction | Out-of-range result |
|--------------|---------------------|
| `T::try_from(n)` for `u8`, `i64` and the other listed integer types | `Err(OutOfRange)` |
| `Level::new(n)`, `LevelMod::new(n)`, `Face::from_u8(n)` | a panic, in every build, documented; in a constant context, rejected when the code is compiled |
| deserialising `Level`, `LevelMod`, `Face`, `CellVertex`, or a struct holding them | a deserialisation error carrying the `OutOfRange` text |
| a raw integer where a bounded type is expected | does not compile |
