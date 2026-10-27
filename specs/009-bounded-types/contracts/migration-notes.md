# Contract: Migration Notes (draft for `BOUNDED_TYPES.md`)

Spec FR-013 needs a migration note for every breaking change: the old call and the new one.
These notes go into `BOUNDED_TYPES.md`'s migration section, and each breaking commit's
`BREAKING CHANGE` footer points to its entry. The list is final once the compiler's error lists
(research R2) are empty. The entries below are the expected ones.

## Rust (`s2rst`)

| # | Change | Before | After |
|---|--------|--------|-------|
| M1 | `From<u8> for Level` removed | `let l: Level = 5.into();` | `Level::new(5)` for a literal, `Level::try_from(n)?` for a value |
| M2 | 23 `impl Into<Level>` parameters now take `Level` | `id.parent_at_level(5)`, `CellId::begin(level_u8)` | `id.parent_at_level(Level::new(5))`, `CellId::begin(Level::try_from(level_u8)?)`; a value already of type `Level` (`id.level()`, `level + 1`) passes unchanged |
| M3 | `Level`'s `TryFrom` error type | `Err(&'static str)` | `Err(OutOfRange)`, which implements `Display` and `Error` |
| M4 | `Face`'s `TryFrom<u8>` error type | `Err(u8)` | `Err(OutOfRange)`, whose `value()` gives the input |
| M5 | `CellId::from_face`, `Cell::from_face` and both `from_face_pos_level` take `Face` | `CellId::from_face(2)`, `for f in 0..6 { CellId::from_face(f) }` | `CellId::from_face(Face::F2)`, `for f in Face::ALL { CellId::from_face(f) }`; from a value, `Face::try_from(n)?` |
| M6 | `CellUnion::denormalize` takes `LevelMod` | `cu.denormalize(min, 2)` | `cu.denormalize(min, LevelMod::new(2))` |
| M7 | `RegionCoverer::level_mod` (setter and field) takes `LevelMod`, and the setter no longer clamps | `.level_mod(2)`, `c.level_mod = 2`, `.level_mod(9)` (was clamped to 3) | `.level_mod(LevelMod::new(2))`; `9` must now be handled by the caller (`LevelMod::try_from(9)` is an `Err`) |
| M8 | `region_term_indexer::Options::level_mod` is a `LevelMod` | `opts.level_mod = 1` (a `usize`) | `opts.level_mod = LevelMod::DEFAULT` |
| M9 | `Cell::vertex` and `vertex_raw` take `CellVertex` (if R5's gate passes) | `cell.vertex(0)`, `for k in 0..4 { cell.vertex(k) }`, `cell.vertex((k + 1) % 4)` | `cell.vertex(CellVertex::LowerLeft)`, `for v in CellVertex::ALL { cell.vertex(v) }`, `cell.vertex(v.next())` |
| M10 | Deserialising an out-of-range `Level`, `LevelMod` or `RegionCoverer` fails | `{"level_mod": 7}` deserialised, then `covering` panicked | a deserialisation error; in-range data reads back unchanged |

## Python (`s2rst`)

No signature changes, and the stub is unchanged.

| # | Change | Before | After |
|---|--------|--------|-------|
| P1 | out-of-range levels, faces and moduli raise `ValueError` | `PanicException`, `OverflowError`, an invalid id, or a silent clamp | `ValueError`; code that caught `OverflowError` catches `ValueError` |
| P2 | vertex indices out of range always raise `IndexError` | `IndexError` for 4, `OverflowError` for −1 | `IndexError` for both |
| P3 | `RegionCoverer(level_mod=0)` and `level_mod=9` raise `ValueError` | silently 1 and 3 | pass 1..=3 |
| P4 | `RegionCoverer(min_level > max_level)` raises at construction | constructed, then crashed or misbehaved when covering | `ValueError` |

## JavaScript (the `s2rst_wasm` package)

No signature changes; the TypeScript type stays `number`.

| # | Change | Before | After |
|---|--------|--------|-------|
| J1 | methods taking a level, face, modulus or vertex index throw on bad input | traps (an aborted, poisoned instance), or silent wrapping (286 became level 30) | a catchable `Error`; the instance stays usable |
| J2 | `setLevelMod(0)` and `setLevelMod(9)` throw | silently clamped | pass 1..=3 |
| J3 | the covering methods throw when the min level exceeds the max | a trap in debug builds | a catchable `Error` |
