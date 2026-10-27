# Contract: Red/Green Tests

The maintainer asked, at planning, for red/green tests for the crashes and missing checks this
feature addresses (research R10).

- Every test below is written before its fix and run against the unfixed tree.
- **Red** is the behaviour observed or expected today. It must be seen, and the output (an
  exception type, a panic message, a returned value) is recorded in `BOUNDED_TYPES.md`'s
  red/green table.
- **Green** is the behaviour after the fix.
- A test and its fix are committed together.
- "Probed" means seen at planning (research R0). "Read" means read from the code, to be seen at
  red time.

Tests go through surfaces whose signatures do not change (Python and JavaScript calls, serialised
text, doctests), so one test is red before and green after. The error contract the green column
refers to is [binding-errors.md](binding-errors.md).

## Python: `python/tests/test_bounded_args.py` (new)

Each test is parametrised over every Python method that takes the kind. The method list is
finalised from the compiler's error list in R2 and R4 (FR-007), and starts with the methods in
the table. The out-of-range inputs are those of spec SC-003.

| ID | Finding | Methods (initial list) | Inputs | Red (today) | Green |
|----|---------|------------------------|--------|-------------|-------|
| PY-01 | B1, B7 | `CellId.parent_at_level`, `child_position`, `child_begin_at_level`, `child_end_at_level`, `vertex_neighbors`, `all_neighbors`, `CellId.from_face_pos_level` (level); `CellUnion.expand_at_level`, `denormalize` (min_level); `RegionCoverer(min_level=, max_level=)`; `RegionTermIndexer(min_level=, max_level=)`; `Metric.value`; the cell-id snap function's level | 31, 255, 256, −1, 2⁴⁰, 2⁷⁰ | 31 and 255: `PanicException` (probed for 31). 256 and −1: `OverflowError` (probed). 2⁷⁰: `OverflowError` (probed). `denormalize`: `ValueError` already (A12) | `ValueError` for every input, its message naming the range and the value |
| PY-02 | guard | the same methods | 0 and 30 | work (probed for 0..=30) | work, with the same results |
| PY-03 | guard | the same methods | 3.5, `"5"`, `None` | `TypeError` (probed for 3.5 and `"5"`) | `TypeError` |
| PY-04 | guard | the same methods | `True` | accepted as 1 (probed) | accepted as 1 (documented) |
| PY-05 | B3 | `CellId.from_face`, `CellId.from_face_pos_level` (face) | 6, 7, 255, −1, 256 | 6, 7 and 255 return an invalid id (probed for 7). −1 and 256: `OverflowError` | `ValueError` |
| PY-06 | B5, B6 | `RegionCoverer(level_mod=)`, `RegionTermIndexer(level_mod=)` | 0, 4, 255, −1 | `RegionCoverer`: clamped to 1 or 3 (probed for 0 and 9). Term indexer: read | `ValueError` |
| PY-07 | B6 | `CellUnion.denormalize` (level_mod) | 0, 4, −1, 2⁴⁰ | `ValueError` (A12, probed) | `ValueError`, now through the shared conversion |
| PY-08 | FR-010 | `RegionCoverer(min_level=10, max_level=5)`; the term indexer likewise | n/a | constructs. `covering` then panics in a debug build (read) | `ValueError` at construction |
| PY-09 | B4 | `Cell.vertex`, `Cell.vertex_raw` | 4, −1, 2⁷⁰ | 4: `IndexError` (probed). −1 and 2⁷⁰: `OverflowError` (read: a `usize` parameter) | `IndexError` for every input |
| PY-10 | B7 | every input of PY-01 and PY-05 to PY-09 | n/a | `PanicException` escapes `except Exception` (probed) | every raised exception is an `Exception` subclass |

## WebAssembly: `wasm/tests/bounded_args.rs` (new, `wasm-pack test --node`)

Each test converts the Rust object to a `JsValue` and calls the JavaScript method by name with
`js_sys::Reflect` and `Function::call1`/`call2`. The argument therefore passes through the real
glue and ToInt32 (research P4, R10). After each rejected call, a valid call on a fresh object
must succeed, which shows that the instance was not poisoned. At red time each test runs alone,
because a trap can poison the instance for later tests.

| ID | Finding | Methods (initial list) | Inputs | Red (today, read; P4) | Green |
|----|---------|------------------------|--------|------------------------|-------|
| WA-01 | B1, B7, B8 | `CellId.parentAtLevel`, `childPosition`, `childBeginAtLevel`, `childEndAtLevel`, `vertexNeighbors`, `allNeighbors`, `begin`, `end`, `fromFacePosLevel` (level); `Cell.averageAreaForLevel`; `CellUnion.expandAtLevel`; `RegionCoverer.setMinLevel`, `setMaxLevel`; `RegionTermIndexer.setMinLevel`, `setMaxLevel`; `SnapFunction.cellId` | 31, 286, −1, `NaN`, 3.5, `Infinity`, `5n` | 31: a panic and a trap. 286: answers for level 30, with no error. −1: becomes 255, then a panic and a trap. `NaN` and `Infinity`: level 0, with no error. 3.5: level 3. `5n`: a `TypeError` | throws an `Error` naming the range (a `TypeError` for `5n`, from the glue); the instance stays usable |
| WA-02 | guard | the same methods | 0, 30, 5.0 | work | work, with the same results |
| WA-03 | B3 | `CellId.fromFace`, `Cell.fromFace`, `CellId.fromFacePosLevel` (face) | 6, 7, −1, `NaN`, 3.5 | 6 and 7: an invalid id. −1: 255, an invalid id. `NaN`: face 0. 3.5: face 3 | throws an `Error` |
| WA-04 | B5, B6 | `RegionCoverer.setLevelMod` | 0, 4, `NaN`, 2.5 | clamped, or truncated then clamped | throws an `Error` |
| WA-05 | FR-010 | `new RegionCoverer().setMinLevel(10).setMaxLevel(5).coveringCap(cap)`, and the other covering methods | n/a | a panic and a trap (debug check, read) | throws an `Error` naming both levels |
| WA-06 | B4 | `Cell.vertex`, `Cell.vertexRaw` (if exported), `Cell.edge` | 4, −1, `NaN`, 3.5 | 4 and −1: an `Error` already (a `Result`). `NaN`: vertex 0. 3.5: vertex 3 | throws an `Error` for every input |

## Core runtime: sibling `*_tests.rs` files

| ID | Finding | Test | Red (today) | Green |
|----|---------|------|-------------|-------|
| CO-01 | B2 | `serde_json::from_str::<Level>("31")`, `"255"` (`core/src/s2/coords_tests.rs`) | `Ok(Level(31))` (read: derived `Deserialize`) | `Err`, its message naming the range. `"30"` round-trips to the same text |
| CO-02 | B2, B5 | `serde_json::from_str::<RegionCoverer>` with `"level_mod": 7`, and separately with `"max_level": 31` (`core/src/s2/region_coverer_tests.rs`) | `level_mod`: `Ok`, and `covering` then panics (read). `max_level`: `Ok` | `Err` for both |
| CO-03 | B5 | the same for the term indexer's `Options`, if it is serialisable (read at red time) | as CO-02 | `Err` |
| CO-04 | R7 | a valid `Level`, `LevelMod`, `RegionCoverer` and `Cell` serialised before and after (fixed JSON strings in the test) | n/a (written green) | byte-for-byte identical output |

## Core type-level: `compile_fail` doctests

Each `compile_fail` block is paired with a passing twin that differs only in the fixed point.
The twin shows that the failure is the intended one, because rustdoc on stable does not check
error codes.

| ID | Finding | Must not compile | Twin that compiles | Red (today) | Green |
|----|---------|------------------|--------------------|-------------|-------|
| CD-01 | B1 | `let l: Level = 31u8.into();` and `id.parent_at_level(5u8)` | `let l = Level::new(30);`, `id.parent_at_level(Level::new(5))` | compiles (`From<u8>`), so the doctest fails | does not compile |
| CD-02 | B3 | `CellId::from_face(7u8)` | `CellId::from_face(Face::F5)` | compiles | does not compile |
| CD-03 | B5 | `RegionCoverer { level_mod: 7, ..Default::default() }` | `… level_mod: LevelMod::new(3) …` | compiles | does not compile |
| CD-04 | B4 (if R5's gate passes) | `cell.vertex(4usize)` | `cell.vertex(CellVertex::UpperLeft)` | compiles | does not compile |
| CD-05 | guard | `const L: Level = Level::new(31);` | `const L: Level = Level::new(30);` | does not compile (probed, P3) | does not compile |

## Verification: mutants as red

The constructor proofs (research R11) are shown non-vacuous by mutants. The mutant is the red
and the harness that kills it is the green:

- M-803 is regenerated against `Level::new`.
- M-806 is re-pointed from `denormalize`'s `assert!` to `LevelMod::new`.
- New mutants make `Face`'s, `LevelMod`'s and `CellVertex`'s `try_from` accept one value too
  many.

## Tests for new functionality (test-first, no red run)

These are written before the new API and fail to compile until it exists. They are not red/green
pairs, because no behaviour existed before:

- `OutOfRange`'s `Display` text;
- each `try_from` over its whole input domain (every `u8`, and the ends of `i64`);
- `LevelMod::branching_factor`;
- `CellVertex::next` and `prev`, which wrap;
- `Level::checked_add` and `checked_sub` at 0 and 30.
