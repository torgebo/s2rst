# Data Model: Bounded Types (spec 009)

The entities this feature adds or changes, their validation rules and their relations. The
signatures are in [contracts/signature-audit.md](contracts/signature-audit.md), the errors in
[contracts/binding-errors.md](contracts/binding-errors.md), and the reasons in
[research.md](research.md).

## 1. `BoundedKind` and `OutOfRange` (new, `s2rst::s2::bounded`, re-exported from `s2`)

The one error of every bounded conversion (R3).

`BoundedKind` is a public enum with one variant per bounded kind. Each variant knows its
`name()` and `range()`:

| Variant | `name()` | `range()` |
|---------|----------|-----------|
| `Level` | `level` | `0..=30` |
| `Face` | `face` | `0..=5` |
| `LevelMod` | `level_mod` | `1..=3` |
| `CellVertex` | `cell vertex index` | `0..=3` |
| `CellEdge` | `cell edge index` | `0..=3` |

The bindings build their messages for values that never reach a `TryFrom` from these: a Python
integer beyond 64 bits, or a JavaScript `NaN`.

`OutOfRange` has private fields with accessors:

| Field | Meaning |
|-------|---------|
| `kind: BoundedKind` | what was converted |
| `value: i64` | the input |

- `range()` is `kind.range()`.
- `Display`: "`<name>` must be in `<lo>..=<hi>`, got `<value>`".
- Implements `std::error::Error`, `Clone`, `Copy`, `Debug`, `PartialEq` and `Eq`.
- Its constructor is `pub(crate)`: only the bounded types create one.

`CellEdge` is not a prototyped candidate: it is already an enum. It gains `TryFrom<i64>`, which
returns `OutOfRange`, only so that both bindings convert edge indices through the same shared
path as vertex indices. The bindings' existing edge checks (`cell_edge_from_index`, and wasm's
`edge(k: u8)`) are the ad-hoc conversions that FR-007 replaces.

## 2. `Level` (changed, `s2rst::s2::coords`)

A newtype over `u8`. **Invariant: 0 ≤ value ≤ 30.**

| Construction | Behaviour |
|--------------|-----------|
| `Level::new(u8)` (`const fn`) | panics above 30. In a constant context this is a compile-time error |
| `TryFrom<u8>`, `TryFrom<u16>`, `TryFrom<i32>`, `TryFrom<i64>`, `TryFrom<usize>` | `Err(OutOfRange)` outside 0..=30. `TryFrom<u8>` and `TryFrom<i64>` are new |
| `Deserialize` | hand-written over the same wire shape (R7). An out-of-range value is an error |
| ~~`From<u8>`~~ | **removed** (R1, R2) |

| Operation | Behaviour |
|-----------|-----------|
| `MIN`, `MAX`, `as_u8`, `as_usize`, `as_u32`, `as_i32`; `From<Level>` for `u8`, `u32`, `i32`, `usize` and `f64` | unchanged |
| `+ u8` and `- u8` | unchanged: they panic past the ends, as documented |
| `checked_add(u8)`, `checked_sub(u8)` | new: `None` past the ends |
| `PartialEq<u8>`, `PartialOrd<u8>`, `Ord`, `Hash`, `Display` | unchanged: they match the integer's |

## 3. `Face` (changed, `s2rst::s2::coords`)

An enum, `F0`–`F5`. The invariant holds by construction.

- **`TryFrom<u8>`** now returns `OutOfRange` instead of the raw `u8`.
- **`TryFrom<i64>`** is new.
- **`from_u8`** (it panics), `ALL`, `iter`, `axis`, `opposite`, `is_positive`, `From<Face> for u8`
  and serde are unchanged.
- **Signatures that now take it** (R4): `CellId::from_face`, `Cell::from_face`, and both
  `from_face_pos_level`.

## 4. `LevelMod` (new, `s2rst::s2`)

A newtype over `u8`. **Invariant: 1 ≤ value ≤ 3.**

| Item | Behaviour |
|------|-----------|
| `LevelMod::new(u8)` (`const fn`) | panics outside 1..=3; a compile-time error in a constant context |
| `DEFAULT` | 1 |
| `TryFrom<u8>`, `TryFrom<usize>`, `TryFrom<i64>` | `Err(OutOfRange)` outside 1..=3 |
| `get() -> u8` | the value |
| `branching_factor() -> u32` | 4, 16 or 64 (4 to the power of the value) |
| `Serialize` derived; `Deserialize` hand-written as for `Level` | a bare number in JSON; out of range is an error |
| `Default`, `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Display` | as an integer |

**Used by**:

- `CellUnion::denormalize(min_level: Level, level_mod: LevelMod)`, whose `assert!` is removed;
- `RegionCoverer::level_mod`, both the field and the setter, which no longer clamps;
- `region_term_indexer::Options::level_mod`, which was a `usize`.

## 5. `CellVertex` (new, `s2rst::s2::cell`; adopted only if R5's gate passes)

An enum, `LowerLeft = 0`, `LowerRight = 1`, `UpperRight = 2`, `UpperLeft = 3`: the corners of
the cell's (u, v) rectangle, in counter-clockwise order.

| Item | Behaviour |
|------|-----------|
| `ALL` | the four, in order |
| `next()`, `prev()` | wrap modulo 4: upstream's `GetVertex(k + 1)` |
| `index() -> usize` | 0..=3 |
| `TryFrom<usize>`, `TryFrom<i64>` | `Err(OutOfRange)` outside 0..=3 |
| serde derived (variant names, as for `CellEdge`) | unknown variants are rejected |

**Used by**: `Cell::vertex(CellVertex)` and `Cell::vertex_raw(CellVertex)`, which no longer
panic.

**Gate (R5)**: if more than a quarter of the call sites need an integer conversion, the type is
not adopted, and `Cell::vertex(usize)` keeps its documented panic plus a documented deviation
from upstream's wrap.

## 6. `RegionCoverer` settings (changed)

| Field | Type | Invariant | Kept by |
|-------|------|-----------|---------|
| `min_level` | `Level` | 0..=30 | the type |
| `max_level` | `Level` | 0..=30 | the type |
| `level_mod` | `LevelMod` (was `u8`) | 1..=3 | the type (was the setter's clamp, which assignment and serde could skip) |
| `max_cells` | `usize` | ≥ 1 after the setter (upstream does not clamp) | unchanged; recorded as a follow-up |
| (relation) | | `min_level ≤ max_level` | a documented precondition of `covering` and `interior_covering` (a debug check). Each binding checks it at its boundary (R6) |

The fields stay public. `serde` derives remain, and the typed fields validate.
`region_term_indexer::Options` follows the same pattern.

## 7. Binding conversions (new)

| Binding | Module | Items | Source value | Error |
|---------|--------|-------|--------------|-------|
| Python | `python/src/bounded.rs` | `LevelArg`, `FaceArg`, `LevelModArg`, `CellVertexArg` (each `FromPyObject`, wrapping the core type) | any Python integer, through `__index__` | `ValueError`; `IndexError` for `CellVertexArg` (binding-errors.md) |
| Python | the same | `check_level_order(min, max)` | two `Level`s | `ValueError` |
| WebAssembly | `wasm/src/bounded.rs` | `level_arg(f64)`, `face_arg(f64)`, `level_mod_arg(f64)`, `cell_vertex_arg(f64)` | a JavaScript `number`, exactly | `Err(js_err(…))`, which JavaScript sees as a thrown `Error` |
| WebAssembly | the same | `check_level_order(min, max)` | two `Level`s | `Err(js_err(…))` |

**Rule (FR-007)**: every `#[pymethods]` or `#[wasm_bindgen]` method that takes one of these
kinds goes through the item for that kind. No method converts with its own code.

## 8. Records

### `BOUNDED_TYPES.md` (new, repository root; R12)

Its sections:

1. **Scope and method.**
2. **The classification table.** One row per inventoried precondition (FR-001, FR-002), with
   these columns:
   - operation;
   - argument or receiver;
   - class (a)–(e);
   - out-of-domain behaviour in debug builds, and in release builds;
   - upstream's behaviour;
   - the gap-1 harness, if any;
   - the recommended option (O1–O5);
   - its status: adopted, follow-up, or not applicable.
3. **The comparison per candidate** (FR-003, FR-004). For the four prototyped candidates, the
   numbers are measured:
   - call sites changed, per crate and per kind of change: literal, conversion, already typed;
   - the panics removed;
   - the binding behaviours;
   - the benchmark instruction counts (R13).
4. **The red/green table.** One row per test of
   [contracts/red-green-tests.md](contracts/red-green-tests.md), with these columns: ID, finding,
   red output (verbatim excerpt), red run date and commit, and the green commit.
5. **Deviations from upstream** (FR-011). Each with the item, upstream's behaviour, the
   library's behaviour, the reason, and where the rustdoc says it.
6. **Migration notes** (FR-013): [contracts/migration-notes.md](contracts/migration-notes.md),
   final.
7. **Follow-ups, ranked** (FR-018). They include the floating-point invariants, value-type
   validity, `r2::Rect::vertex`, axis and direction, typed identifiers, `max_cells`, the
   `level!` macro, Python `IntEnum`, and TypeScript literal unions.

### Verification records (FR-016, FR-017)

These change in the same commits as the harnesses:

- `verify/INVENTORY.md`, section 3, and `cell_union/INVENTORY.md`, section 3;
- `verify/COVERAGE.md`: gap 1's rows are re-pointed to the constructor proofs;
- `verify/MUTANTS.md`;
- `VERIFIED_TYPES.md`: a "bounded types" subsection listing `Level`, `Face`, `LevelMod` and
  `CellVertex` with their constructor proofs;
- `verify/PROPERTIES_BY_KIND.md` and `.html`: "Panics exactly where documented" gains the
  constructors and loses the retired complements.
