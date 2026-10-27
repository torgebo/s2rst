# Bounded types: where they make the API safer, and what they cost

This is the decision record of spec 009 (`specs/009-bounded-types/`). A **bounded type** is a
type whose values are checked once, when an instance is created. A function that takes one
needs no check of its own, documents no panic for that argument, and cannot be handed a value
out of range.

The record does three things:

- it classifies every public precondition of the `s2rst` core on an argument's range or a
  value's validity (section 2);
- it compares the design options for each candidate on safety, ergonomics, downstream cost
  (Python and WebAssembly), verification, performance and parity with upstream (section 3);
- it records what was adopted and how, with red/green evidence (section 4), the deviations from
  upstream (section 5), the migration notes (section 6) and the follow-ups (section 7).

## 1. Scope and method

**Candidates prototyped and adopted** (spec FR-005, FR-012):

- the cell level, 0..=30 (`Level`);
- the cube face, 0..=5 (`Face`);
- the level modulus, 1..=3 (`LevelMod`);
- `Cell`'s vertex index, 0..=3 (`CellVertex`, gated by spec research R5).

**Options compared** (spec FR-003):

| Option | Shape |
|--------|-------|
| O1 | the status quo: a raw integer and a documented panic |
| O2 | a raw integer with a fallible result |
| O3 | a checked newtype, with or without an implicit conversion from raw integers that panics |
| O4 | an enum |
| O5 | clamping or saturating |

Compile-time rejection of out-of-range constants is assessed as a property of each option.

**Dimensions** (spec FR-004):

- safety: what is removed, where an error is caught, and release-build behaviour;
- ergonomics: call sites, literals, arithmetic and iteration, and error messages;
- downstream: Python signatures, stub and exceptions; JavaScript and TypeScript signatures; an
  exception or a trap;
- serialisation;
- verification;
- performance;
- parity with upstream.

**Method**:

- The four candidates were changed through the core and both bindings, one commit each.
- Removing the implicit conversions made the compiler list every call site that passed a raw
  integer. Those lists are the counts in section 3.
- Every crash, silent wrong result or missing check got a test that was written first and seen
  failing on the unfixed tree (section 4).

### Starting state (2026-10-27)

**The gate on the unfixed tree.** `make test` at `bde133d` passed: every Rust suite, Python
(919 passed) and wasm (283 passed). This is the baseline that every commit of this feature
keeps.

**The Python bindings on out-of-range arguments.** Probes P1 and P2 were re-run against the
extension that `make test` had just rebuilt, on the leaf `CellId.from_face(0).child_begin_at_level(30)`:

| Call | Result |
|------|--------|
| `CellId.parent_at_level(31)` | `pyo3_runtime.PanicException`: level must be 0..=30 |
| `CellId.parent_at_level(256)` | `builtins.OverflowError`: number too large to fit in target type |
| `CellId.parent_at_level(-1)` | `builtins.OverflowError`: number too small to fit in target type |
| `CellId.parent_at_level(2**70)` | `builtins.OverflowError`: Python int too large to convert to C long |
| `CellId.parent_at_level(True)` | returns level 1 |
| `CellId.from_face(7)` | returns `CellId(Invalid: f000000000000000)` |
| `RegionCoverer(level_mod=0).level_mod` | returns `1` (clamped) |
| `RegionCoverer(level_mod=9).level_mod` | returns `3` (clamped) |
| `RegionCoverer(min_level=31)` | `pyo3_runtime.PanicException`: level must be 0..=30 |
| `RegionCoverer(min_level=10, max_level=5)` | constructs |
| `CellUnion.denormalize(0, 4)` | `builtins.ValueError`: level_mod must be 1, 2 or 3, got 4 |
| `Cell.vertex(4)` | `builtins.IndexError`: vertex index must be 0, 1, 2, or 3 |
| `Cell.vertex(-1)` | `builtins.OverflowError`: can't convert negative int to unsigned |

So one mistake, a level out of range, gives three different exceptions depending on the value.
One of them, `PanicException`, derives from `BaseException`, so `except Exception` does not
catch it. A face out of range gives no exception at all.

## 2. Classification

Every public precondition of the core on an argument's range or a value's validity, by class:

- **(a)** an absolute range of one argument;
- **(b)** a small finite set;
- **(c)** a relation between arguments, or with the receiver;
- **(d)** a floating-point invariant;
- **(e)** the validity of an existing value type.

"Debug" and "release" give the behaviour outside the domain in each build. Upstream is C++
`s2geometry` at v0.14.0-40, with file and line numbers there. The options are those of section 1.

**What a bounded type can remove.**

- A type over one argument removes classes (a) and (b) completely: the range becomes the type's
  domain.
- It cannot remove class (c), which relates two values. "A level no finer than this cell's
  level" depends on the receiver. Expressing it would need a value tied to one particular cell,
  which costs more than the panic it removes. Class (c) rows therefore keep O1 as a documented
  precondition. Where a binding can check the relation at its boundary, it does (the coverer's
  `min_level ≤ max_level`, section 3.3).
- Classes (d) and (e) are value-type validity. A type can carry them, as `ChordAngle` has done
  since spec 007, but each touches the whole geometry API. They are ranked in section 7.

### 2.1 The 71 operations of gap 1 (`verify/COVERAGE.md`)

#### `CellId` (21 operations; `verify/INVENTORY.md` §3)

| Operations | Argument or receiver | Class | Debug | Release | Upstream | Gap-1 harness | Option | Status |
|------------|----------------------|-------|-------|---------|----------|---------------|--------|--------|
| `impl From<u8> for Level`, and the 12 `CellId` operations taking `impl Into<Level>`: `from_face_pos_level`, `child_position`, `parent_at_level`, `child_begin_at_level`, `child_end_at_level`, `begin`, `end`, `lsb_for_level`, `size_ij`, `size_st`, `vertex_neighbors`, `all_neighbors` | the level, above 30 | (a) | panics (the conversion) | panics | `ABSL_DCHECK` of 0..=30 (`s2cell_id.h:614`, `:637`, `:664`, `:697`, `:711`); an unchecked shift in release builds | `crash::n_raw_level` | O3 without the implicit conversion: take `Level`, convert with `TryFrom` | adopted (section 3.1) |
| `level` | an id with no level marker (`0`, `1 << 62`, `1 << 63`, `3 << 62`) | (e) | panics | panics | no check: computed from the trailing zeros (`s2cell_id.h:596` explains why it cannot `DCHECK(is_valid())`) | `crash::n_level` | O1 (a validated cell-id type is a follow-up) | follow-up F5 |
| `child_position(level)` | a level outside `1..=self.level()`, or an invalid id | (c), (e) | panics | returns, unspecified | `ABSL_DCHECK`s (`s2cell_id.h:636–638`) | `crash::n_child_position`, `r_child_position` | O1: the range part is (a) and goes to `Level`; the relation stays | the (a) part adopted; the (c) part not applicable |
| `parent()` | a face cell, or an invalid id | (c), (e) | panics | returns | `ABSL_DCHECK(!is_face())` (`s2cell_id.h:671–672`) | `crash::n_parent`, `r_parent` | O1 | not applicable (a relation) |
| `children`, `child_begin`, `child_end` | a leaf, or an invalid id | (c), (e) | panics | returns (`child_end` wraps near 2⁶⁴) | `ABSL_DCHECK(!is_leaf())` (`s2cell_id.h:678–704`) | `crash::n_children`, `r_children` | O1 | not applicable |
| `next_wrap`, `prev_wrap` | an invalid id | (e) | panics | returns | `ABSL_DCHECK(is_valid())` (`s2cell_id.h:725`, `:732`) | `crash::n_wrap`, `r_wrap` | O1 | follow-up F5 |
| `vertex_neighbors(level)` | level 30, or a level not below the cell's | (c) | panics | panics | `ABSL_DCHECK_LT(level, this->level())` (`s2cell_id.cc:518`) | `crash::n_vertex_neighbors` | O1: the range part goes to `Level` | the (a) part adopted |
| `common_ancestor_level` | (`none`, `none`) | (e) | panics | panics | `ABSL_ASSUME(bits != 0)` (`s2cell_id.cc:198`): undefined for that input | `crash::n_common_ancestor_level` | O1 | follow-up F5 |
| `from_face_ij(face, i, j)` (no gap-1 harness: not every such input panics) | `i` or `j` outside `[0, 2³⁰)` | (a) | some panic, some return | unspecified id | no check (`s2cell_id.cc:267`) | none | O1: an i-j newtype would be possible, but `from_face_ij_wrap` already exists for out-of-range input | follow-up F7 |

#### `CellUnion` (11 operations; `verify/cell_union/INVENTORY.md` §3)

| Operations | Argument or receiver | Class | Debug | Release | Upstream | Gap-1 harness | Option | Status |
|------------|----------------------|-------|-------|---------|----------|---------------|--------|--------|
| `from_cell_ids`, `normalize`, `FromIterator` | an invalid id outside every valid id's leaf range | (e) | panics | keeps the id | `ABSL_DCHECK(id.is_valid())` (`s2cell_union.cc:178`) | `cu_crash::n_from_cell_ids__n1`, `__n2`, `r_…` | O1 (a validated cell-id type) | follow-up F5 |
| `contains_cell_id`, `intersects_cell_id`, `intersection_with_cell_id` | an invalid id | (e) | panics | returns | `ABSL_DCHECK(id.is_valid())` (`s2cell_union.cc:296`, `:304`, `:356`) | `cu_crash::n_cell_queries__n2`, `r_…` | O1 | follow-up F5 |
| the same, and the other queries | an invalid union | (e) | unspecified, may panic | unspecified | `IsNormalized` is assumed | none (no check reads the union) | O1 (typestate for a normalised union) | follow-up F6 |
| `leaf_cells_covered` (and `average_based_area`) | an id with no level marker | (e) | panics | panics | no check (`s2cell_union.cc:486`) | `cu_crash::n_leaf_cells_covered__n2` | O1 | follow-up F5 |
| `denormalize(min_level, level_mod)` | `level_mod` outside 1..=3 | (a) | panics (an `assert!`) | panics | `ABSL_DCHECK` of 1..=3 (`s2cell_union.cc:209–210`) | `cu_crash::n_denormalize__n2` | O3: `LevelMod` | adopted (section 3.3) |
| `denormalize` | the first id with no level marker | (e) | panics | panics | no check | `cu_crash::n_denormalize__n2` | O1 | follow-up F5 |
| `from_begin_end`, `from_range` | a bound neither a leaf nor `end(30)`, or `begin > end` | (e), (c) | panics | panics | `ABSL_DCHECK`s (`s2cell_union.cc:98–103`) | `cu_crash::n_from_begin_end` | O1 | not applicable (a relation) |
| `from_min_max` | `max` not a valid leaf, `min` neither a valid leaf nor `max.next()`, or `min > max.next()` | (e), (c) | panics | panics | `ABSL_DCHECK(max_id.is_valid())` (`s2cell_union.cc:93`), then `FromBeginEnd`'s | `cu_crash::n_from_min_max` | O1 | not applicable |
| `approx_area`, `exact_area`, the `Region` bounds | an id that `Cell::from_cell_id` rejects | (e) | panics | panics | no check | none (floating point; a native check) | O1 | follow-up F5 |

#### The decoding layer (3 operations; `verify/decoding/INVENTORY.md` §2)

| Operations | Argument | Class | Debug | Release | Upstream | Gap-1 harness | Option | Status |
|------------|----------|-------|-------|---------|----------|---------------|--------|--------|
| `encode_uint_with_length`, `decode_uint_with_length` | a length above 8 | (a) | panics (a slice) | panics | `ABSL_DCHECK(length >= 0 && length <= sizeof(T))` (`encoded_uint_vector.h:145`, `:159`) | `cd_crash::n_uint_with_length` | O1: every caller computes the length from data it wrote, so a type would add a conversion at each internal site and remove no reachable panic | follow-up F8 (low) |
| `get_uint_with_length` | a length above 8, or an input shorter than the length | (a), (c) | panics | panics | `ABSL_DCHECK(len >= 1 && len <= 8)` (`encoded_uint_vector.h:217`) | `cd_crash::n_get_uint_with_length` | O1 | follow-up F8 |

#### The value types (36 operations; `verify/values/INVENTORY.md` §10)

| Operations | Argument or receiver | Class | Debug | Release | Upstream | Gap-1 harness | Option | Status |
|------------|----------------------|-------|-------|---------|----------|---------------|--------|--------|
| `Cell::from_cell_id`, `From<CellId>` | an id with no level marker, or on face 6 or 7 | (e) | panics | panics | no check | `cl_crash::n_from_cell_id` | O1 | follow-up F5 |
| `Cell::from_face`, `Cell::from_face_pos_level` | face 6 or 7; a level above 30 | (a), (b) | panics | panics | no check | `cl_crash::n_from_cell_id` | O4 `Face` and O3 `Level` | adopted (sections 3.1, 3.2) |
| `Cell::from_point`, `CellId::from_point` | the zero vector; some points with a NaN coordinate | (d) | panics | returns an unspecified leaf | `GetFace` of the zero vector is face 2, unchecked (`s2coords.h:409`) | `cl_crash::n_from_point`, `r_from_point` | O1 (a unit-point type) | follow-up F1 |
| `Cell::vertex`, `Cell::vertex_raw` | an index above 3 | (b) | panics | panics | wraps modulo 4 (`r2rect.h:214–218`) | `cl_crash::n_vertex` | O4 `CellVertex`, gated by R5 | section 3.4 |
| `s1::Interval::new`, `from_point_pair` | an endpoint outside [−π, π], or NaN | (d) | panics | returns an invalid interval | `ABSL_DCHECK(is_valid())` (`s1interval.cc:72`, `:79`) | `iv_crash::n_constructors`, `r_…` | O1 (a validated angle) | follow-up F2 |
| `s1::Interval::contains`, `interior_contains`, `add_point` | a point outside [−π, π], or NaN | (d) | panics | returns | `ABSL_DCHECK_LE(fabs(p), M_PI)` (`s1interval.cc:159–198`) | `iv_crash::n_point_operations`, `r_…` | O1 | follow-up F2 |
| `s1::Interval::project` | an empty interval, or a point out of range | (c), (d) | panics | returns | `ABSL_DCHECK`s (`s1interval.cc:279`) | `iv_crash::n_project`, `r_…` | O1 | follow-up F2 |
| `s1::Interval::expanded` | a NaN margin on a non-full interval | (d) | panics | returns | `ABSL_DCHECK` | `iv_crash::n_expanded`, `r_…`, `o_expanded_nan_full` | O1 | follow-up F2 |
| `s1::Interval::bound(i)` | an index above 1 | (b) | panics | panics | an array index, unchecked (`s1interval.h`) | `iv_crash::n_bound` | O4: the existing `r1::Endpoint` enum | follow-up F9 (cheap) |
| `r1::Interval::project` | an empty interval, or a NaN bound | (e) | panics (`f64::clamp`) | panics | `ABSL_DCHECK(!is_empty())` (`r1interval.h:174`) | `ri_crash::n_project` | O1 | follow-up F3 |
| `s2::Rect::from_lat_lng`, `from_point_pair` | a longitude outside [−π, π], or NaN | (d) | panics | returns | `ABSL_DCHECK`s (`s2latlng_rect.cc`) | `rl_crash::n_from_points`, `r_…` | O1 (a validated `LatLng`) | follow-up F2 |
| `s2::Rect::from_center_size` | a centre or size longitude out of range | (d) | panics | returns | the same | `rl_crash::n_from_center_size`, `r_…` | O1 | follow-up F2 |
| `s2::Rect::interior_contains_lat_lng` | a longitude out of range, with the latitude inside | (d) | panics | returns | the same | `rl_crash::n_interior_contains_lat_lng`, `r_…` | O1 | follow-up F2 |
| `s2::Rect::expanded` | a NaN longitude margin, longitudes not full | (d) | panics | returns | the same | `rl_crash::n_expanded`, `r_…`, `o_expanded_nan_full_lng` | O1 | follow-up F2 |
| `s2::Rect::get_distance_to_latlng` | an empty rectangle, or an invalid point | (e), (d) | panics | returns | logged only (`s2latlng_rect.cc:603`) | `rl_crash::n_get_distance_to_latlng` | O1 | follow-up F3 |
| `s2::Rect::get_distance` | an empty rectangle | (e) | panics | returns | `ABSL_DCHECK`s (`s2latlng_rect.cc:510–511`) | `rl_crash::n_get_distance` | O1 | follow-up F3 |
| `r2::Rect::new`, `from_points` | exactly one empty axis | (e) | panics | returns an invalid rectangle | `ABSL_DCHECK(is_valid())` (`r2rect.h:73–79`) | `ur_crash::n_constructors`, `r_…` | O1 | follow-up F3 |
| `r2::Rect::from_center_size` | a size with exactly one axis empty | (e) | panics | returns an invalid rectangle | the same | `ur_crash::n_from_center_size`, `r_…` | O1 | follow-up F3 |
| `r2::Rect::project` | an empty axis, or a NaN bound | (e) | panics | panics | `ABSL_DCHECK(!is_empty())` (`r2rect.h:178–189`) | `ur_crash::n_project` | O1 | follow-up F3 |
| `ChordAngle::sin`, `sin2`, `cos`, `tan` | `NEGATIVE` or `INFINITY` | (e) | panics | returns NaN, −1.25, 1.5; NaN, −∞, −∞ | `ABSL_DCHECK(!is_special())` (`s1chord_angle.cc:132`, `:147`) | `ca_crash::n_trigonometry`, `r_…` | O1: the specials are valid chord angles, so a type would split `ChordAngle` in two | not applicable |
| `ChordAngle` `Add`, `Sub` | an operand `NEGATIVE` or `INFINITY` | (e) | panics | returns | `ABSL_DCHECK(!is_special())` (`s1chord_angle.cc:92–93`, `:116–117`) | `ca_crash::n_add_sub`, `r_…` | O1 | not applicable |
| `Cap::from_center_angle` | a centre that is not unit length | (d) | panics | returns an invalid cap | `ABSL_DCHECK(is_valid())` (`s2cap.h:250–255`) | `cp_crash::n_from_center_angle`, `r_…` | O1 (a unit-point type) | follow-up F1 |
| `Cap::expanded` | a negative or NaN distance; +∞ with a non-empty cap | (d) | panics | returns | `ABSL_DCHECK_GE(distance.radians(), 0)` (`s2cap.cc`) | `cp_crash::n_expanded`, `r_…`, `o_expanded_empty_infinite` | O1 | not applicable |

### 2.2 Signatures outside gap 1 (`specs/009-bounded-types/contracts/signature-audit.md`)

| Item | Argument | Class | Behaviour outside the domain (both builds) | Upstream | Option | Status |
|------|----------|-------|--------------------------------------------|----------|--------|--------|
| `CellId::from_face`, `CellId::from_face_pos_level` (the face) | 6 or 7 | (b) | a silent invalid id (`from_face(7)` is `f000000000000000`) | `FromFace(int)` unchecked | O4: take `Face` | adopted (section 3.2) |
| `RegionCoverer::level_mod` (setter) | outside 1..=3 | (a) | clamped silently | `ABSL_DCHECK` of 1..=3, then clamp (`s2region_coverer.cc:79–83`) | O3: `LevelMod`, rejected | adopted (section 3.3) |
| `RegionCoverer` field `level_mod`, and its `Deserialize` | outside 1..=3 | (a) | skips the setter; `covering` then panics in `denormalize` | (no public fields) | O3: `LevelMod` | adopted |
| `region_term_indexer::Options::level_mod` (`usize`) | outside 1..=3 | (a) | passed to the coverer | `set_level_mod` clamps | O3: `LevelMod` | adopted |
| `RegionCoverer` `min_level`, `max_level` | `min > max` | (c) | a debug-build panic in `covering` | "checked by `S2RegionCoverer`" (`ABSL_DCHECK`) | O1 in the core; each binding checks at its boundary | adopted (bindings) |
| `RegionCoverer::max_cells` | 0 | (a) | clamped to 1 | not clamped (`s2region_coverer.cc:56–58`) | O1 (documented) | follow-up F10 |
| `get_uvw_axis(face, axis)`, `get_uvw_face(face, axis, direction)` | `axis` above 2, `direction` above 1 | (b) | an index out of bounds | unchecked | O4: `r3::Axis` exists | follow-up F9 |
| `face_si_ti_to_xyz(face, si, ti)` | above 2³¹ | (a) | an unspecified point | unchecked | O1 | follow-up F7 |
| `fractal::set_max_level`, `num_vertices_at_level` (`i32`) | negative | (a) | a test-data generator's level | — | O1 (a testing utility) | not applicable |
| `DensityTree::new(…, max_level: u8)` | above 30 | (a) | unspecified | — | O3: `Level` | follow-up F11 |
| `r2::Rect::vertex(k: i32)` | any `i32` | (b) | wraps modulo 4, as upstream | wraps (`r2rect.h:214`) | O1 (consistent with upstream) | follow-up F9 |
| the typed identifiers (`impl Into<ShapeId>`, `VertexId`, `EdgeId`, `InputEdgeId`) | an index into a container | (c) | `Option`, or a panic, where used | unchecked indices | `impl Into` is right (infallible wrappers); checking validity is relative to a container | not applicable |

### 2.3 Value-type invariants (classes d and e, ranked in section 7)

| Invariant | Where it is a precondition today | Behaviour outside it | Status |
|-----------|----------------------------------|----------------------|--------|
| a unit-length point | `Cap::from_center_angle`, `contains_point`, `s2::rotate` (documented, unchecked), the predicates' inputs, the edge functions | debug checks in some; unspecified results elsewhere | follow-up F1 |
| latitude in [−π/2, π/2], longitude in [−π, π] | `s1::Interval`'s point operations, `s2::Rect`'s constructors and tests | debug checks; release results unspecified | follow-up F2 |
| the validity of `s1::Interval`, `r1::Interval`, `s2::Rect` and `r2::Rect` | constructors (debug checks), `project`, the distances | debug checks; some every-build panics (`f64::clamp`) | follow-up F3 |
| `Cap` validity | every operation but `is_valid` (spec 008, "Validity stays a precondition") | unspecified for an invalid cap | follow-up F4 |
| a valid `CellId` | `level`, the hierarchy operations, `Cell::from_cell_id`, the union constructors and queries | panics, or unspecified results | follow-up F5 |
| a normalised `CellUnion` | the queries and set operations | unspecified results | follow-up F6 |

**Coverage**: section 2.1 lists the 71 gap-1 operations (21 + 11 + 3 + 36), the same split as
`verify/COVERAGE.md`. Section 2.2 lists the signature audit's remaining rows, and section 2.3
the class (d) and (e) invariants.

## 3. Comparison per candidate

### 3.0 `T` or `impl Into<T>`?

Should a function that takes a bounded type `T` take `impl Into<T>`, so that callers can pass a
raw integer? **No. It takes `T`, and a raw integer is converted where the caller has it.**

- `impl Into<T>` helps only when `T` has infallible conversions from types that callers hold.
  A bounded type's sources are raw integers, and converting one can fail by definition. The
  standard library's contract is that `From` does not fail. A failing conversion is `TryFrom`,
  and `impl TryInto<T>` would make every callee return a `Result`.
- Before this feature, `impl Into<Level>` worked only because `From<u8> for Level` panicked
  above 30, breaking that contract. That panic is what Python users saw as `PanicException`
  (section 1).
- Literal inference through `impl Into<T>` is fragile (`specs/009-bounded-types/probe/`, rustc
  1.99.1):

  | `From` impls for `Level` | `parent_at_level(5)` through `impl Into<Level>` |
  |--------------------------|-------------------------------------------------|
  | `From<u8>` | compiles: `5` is inferred as `u8`, the only candidate |
  | `From<u8>` and `From<u16>` | error E0277, `Level: From<i32>`: with two candidates the literal falls back to `i32` |
  | none | error E0277 |

  One more infallible source would have broken every literal call site, in this workspace and
  downstream.
- Without `From<u8>`, `impl Into<Level>` accepts only `Level`. That is `Level` with generic
  noise: one instantiation per caller type, and vaguer documentation and errors.
- Plain `T` makes the compiler find every call site that passes a raw integer. That list is the
  measured count below, and it forces each binding through one conversion per kind.

`impl Into<T>` stays right for the typed identifiers (`ShapeId`, `VertexId`, `EdgeId`,
`InputEdgeId`), whose conversions are infallible wrappers.

### 3.1 The cell level (0..=30)

| Dimension | O1 raw `u8` + panic | O2 raw `u8` + `Result` | O3 `Level`, with a panicking `From<u8>` (before) | **O3 `Level`, `TryFrom` only (chosen)** | O5 clamp |
|-----------|---------------------|------------------------|---------------------------------------------------|------------------------------------------|----------|
| Safety: what is removed | nothing | the panic, in exchange for a `Result` at every hierarchy call | nothing: the panic moves into the conversion, in every build | the panic in all 23 callees; the range check happens once, where the integer is converted | the panic, by silently changing the input |
| Where an error is caught | at the call | at the call | at the call (implicit) | at the conversion. With `Level::new(n)` in a constant context, when the code is compiled | never |
| Ergonomics: literals | `f(5)` | `f(5)?` | `f(5)` (fragile, P3) | `f(Level::new(5))` | `f(5)` |
| Ergonomics: values the caller already holds as `Level` (`id.level()`, `level + 1`) | convert back to an integer | the same | pass through | pass through | — |
| Downstream: Python | `PanicException` (31), `OverflowError` (256, −1) | `ValueError` per method | the same as O1 | one `LevelArg` conversion: `ValueError` for every out-of-range value | silent |
| Downstream: JavaScript | a trap at 31; at 286, wasm-bindgen's `as u8` silently gives 30 | per method | the same as O1 | one `level_arg(f64)` conversion: a thrown `Error`, with no wrapping | silent |
| Serialisation | n/a | n/a | derived; accepted 31 (B2) | validated (CO-01) | — |
| Verification | the `n_raw_level` complement over 13 operations | — | the same | one constructor proof (`n_level_new`); the operations' proofs range over the type | — |
| Performance | — | a branch at each call | the same as O1 | unchanged: 9 of 37 benchmarks moved by at most 10 instructions | — |
| Parity with upstream | upstream `DCHECK`s, then misbehaves in release builds | — | stricter in every build | stricter in every build, and visible at the call | upstream clamps in the coverer's setters only |
| Call sites to change | — | — | — | 571 in the core (317 literals, 213 conversions, 40 constants, 1 loop), 17 Python and 17 wasm parameters, measured below | — |

**Recommendation: O3 with `TryFrom` only.** This is adopted.

**Measured (2026-10-27).** These are the compiler-driven counts of the `Level` change. Removing
`From<u8>` and changing the parameters made the compiler list every call site, crate by crate.

- **Signatures**: 25, not the 23 the audit found by grep. `point_compression::encode_points_compressed`
  and `decode_points_compressed` take `impl Into<crate::s2::coords::Level>` over several lines,
  which the one-line grep missed. All 25 now take `Level`.
- **Call sites in the core**:

  | Where | Literal → `Level::new(n)` | Conversion → `Level::new(v)` | Constant → `Level::MAX`/`MIN` | Loop | Total |
  |-------|---------------------------|-------------------------------|-------------------------------|------|-------|
  | the library's own code | 3 | 6 | 10 | 1 | 20 |
  | the test modules in `core/src` | 196 | 111 | 30 | — | 337 |
  | `core/tests` | 50 | 93 | — | — | 143 |
  | `core/benches` | 57 | 1 | — | — | 58 |
  | `core/examples` | 11 | 2 | — | — | 13 |
  | **total** | **317 (56%)** | **213 (37%)** | **40 (7%)** | **1** | **571** |

  The literal rewrites were made by a script, and the rest from the compiler's error list. 23
  modules gained a `use … Level`. Seven doc examples (and `core/README.md`'s) needed the new
  call and an import.
- **The library's own code** (20 sites) shows where the integer view is natural:
  - 10 passed the `u8` constant `MAX_CELL_LEVEL`, now `Level::MAX`;
  - 6 computed a level in integer arithmetic. `Cap::cell_union_bound`'s
    `i32::from(max_level) − 1` became `max_level.checked_sub(1)`, removing a cast and a sign
    check. `DensityTree`'s decoder and `PaddedCell::shrink_to_fit` index arrays by level, so
    they keep counting in `usize` and convert with `Level::new(l as u8)`, the old `From<u8>`
    made explicit.
- **The bindings**:
  - Python: 17 parameters now take `LevelArg`. Two ad-hoc validators were deleted (`to_level`,
    and `denormalize`'s level check). Two deferred panics found by the red run were fixed:
    `S2CellIdSnapFunction(31)` and `S2Builder(snap_level=31)` stored the raw `u8` and panicked
    only when used (B11).
  - wasm: 17 methods take an `f64` and return a `Result`. `RegionTermIndexer`'s setters no
    longer call `Level::new` on a raw value. `SnapFunction.cellId` stores a `Level` (B11).
    `wasm/tests/web.rs` needed 28 call sites updated.
  - Python's stub is unchanged, and so is wasm's TypeScript type (`number`).
  - `RegionTermIndexer`'s Python constructor needed an explicit `text_signature`: PyO3 shows a
    non-literal default (`LevelArg(Level::new(4))`) as `...`, which stubtest would reject
    against the stub's `= 4`.
- **The fuzz target** `decode_points_compressed`: 1 conversion.
- **Verification**:
  - one native test (`spec_vs_cpp`: `Level::MIN`);
  - 13 operations of `crash::n_raw_level` that can no longer receive a raw level. The harness
    is replaced by `n_level_new` and `c_level_try_from`, both proven;
  - one case of `cl_crash::n_from_cell_id` that is no longer expressible, and was removed;
  - one conversion in `cp_bnd::p_cpbnd_02`;
  - all four harnesses re-proven.
- **`expect` is forbidden** in the library's code (`clippy::expect_used`), so a conversion that
  cannot fail by construction is written `Level::new(x as u8)`, with a comment. Two
  `#[expect(clippy::cast_…)]` attributes in `cap.rs` became unfulfilled when `checked_sub`
  removed the cast, and were deleted.

**Documented panics (SC-004).** Run against the tree at `df687f0`, the audit of `# Panics`
sections (task T054) found 19 that named an adopted argument's range:

- the 13 `CellId` level functions ("the conversion to `Level` panics");
- `Cell::from_face`, both `from_face_pos_level`, `Cell::vertex` and `vertex_raw`;
- `denormalize`'s `level_mod`;
- `From<u8> for Level`.

At `2738c80` none remain. The ranges are documented once, on the constructors (`Level::new`,
`LevelMod::new`, `Face::from_u8`). The five sections the audit still matches are id validity
(class e), which stays: `Cell::from_cell_id`, `CellUnion::approx_area`, `exact_area` and its
`Region` bounds.

**Ergonomics, measured.** 56% of the call sites are literals, which read slightly longer
(`parent_at_level(Level::new(10))`). 37% convert a `u8`, almost all of them test loops over
`0..=30`. Restructuring those loops to iterate levels would remove most of the conversions; a
`Level::all()` iterator is a follow-up (section 7). No call site needed a `Result` to be
handled: every conversion in the library is in range by construction, and every binding
conversion raises its native error.

### 3.2 The cube face (0..=5)

| Dimension | O1 raw `u8` (before: `impl Into<u8>`, unchecked) | O3 checked `u8` with a panic | **O4 `Face` (chosen)** |
|-----------|--------------------------------------------------|------------------------------|-------------------------|
| Safety | `from_face(7)` is a silent invalid id | a new panic | unrepresentable |
| Ergonomics | `from_face(2)`, `for f in 0..6` | the same | `from_face(Face::F2)`, `for f in Face::ALL` (both exist) |
| Python and JavaScript | an invalid id returned (probed) | a crash | one conversion: `ValueError`, or a thrown `Error` |
| Serialisation | `Face` serialises as `"F0"` (CO-04): unchanged | — | unchanged |
| Parity | upstream `FromFace(int)` is unchecked | — | stricter, documented |
| Call sites | — | — | 561 in the core (434 literals, 119 conversions, 7 loops, 1 sentinel), 5 Python and 3 wasm parameters, measured below |

**Recommendation: O4.** This is adopted.

**Measured (2026-10-27).**

- **Signatures**: 4 (`CellId::from_face`, `Cell::from_face`, both `from_face_pos_level`).
  `Face`'s `TryFrom<u8>` now returns `OutOfRange`, not the raw `u8`, and `TryFrom<i64>` is new.
- **Call sites in the core**:

  | Where | Literal → `Face::Fn` | Conversion → `Face::from_u8(v)`, `Face::ALL[i]`, `try_from` | Loop → `Face::ALL` | Other | Total |
  |-------|----------------------|----------------------------------------------------------------|--------------------|-------|-------|
  | the library's own code | 6 | 3 | 2 | 1 sentinel | 12 |
  | the test modules in `core/src` | 343 | 36 | 5 | — | 384 |
  | `core/tests` | 47 | 78 | — | — | 125 |
  | `core/benches` | 37 | 2 | — | — | 39 |
  | `core/examples` | 1 | — | — | — | 1 |
  | **total** | **434 (77%)** | **119 (21%)** | **7** | **1** | **561** |

- **The sentinel** is the one place where the library itself relied on an out-of-range face.
  `boolean_operation`'s index walk sought `CellId::from_face(face + 1).range_min()`, which for
  face 5 builds the invalid face-6 id as a past-the-end marker. It is now
  `CellId::from_face(face).range_max().next()`, the first leaf after the face: the same value
  for every face (`0xC000000000000001` after face 5, which is also `CellId::end(Level::MAX)`),
  without an out-of-range face.
- **One conversion removed a check**: `CellId::from_debug_string`'s `if face > 5` became
  `Face::try_from(…).ok()?`.
- **The bindings**:
  - Python: 5 parameters take `FaceArg` (`CellId.from_face`, `from_face_pos_level`'s face,
    `face_uv_to_xyz`, `get_u_axis`, `get_v_axis`). The ad-hoc `face_from_int` was deleted, and
    the three toolkit functions no longer need to return a `PyResult`.
  - wasm: 3 methods (`CellId.fromFace`, `Cell.fromFace`, `fromFacePosLevel`). `web.rs` needed
    30 call sites updated.
- **Verification**:
  - 16 harness sites: 8 conversions (`Face::from_u8` of the model's `any_face()`), 7 literals
    and 1 loop;
  - `cl_crash::n_from_cell_id` lost its two face cases, which are no longer expressible;
  - 19 touched harnesses re-proven.
  - Ten mutant patches had gone stale from the `Level` and `Face` edits (M-017, M-018, M-019,
    M-021, M-024, M-025, M-058, M-128, M-313, M-720). Each was regenerated with the same
    defect, restated on the new code; for example M-720's "no `− 1`" is now `checked_sub(0)`.

**Ergonomics, measured.** 77% of the call sites are literal faces, which read better as
`Face::F2` than as `2`. The conversions are almost all tests that loop over `0..6` and use the
face as an integer elsewhere.

### 3.3 The level modulus (1..=3), and the coverer's settings

| Dimension | O1 (`denormalize`) | O5 (the coverer's setter, before) | **O3 `LevelMod`, rejecting (chosen)** | O4 an enum |
|-----------|--------------------|-----------------------------------|----------------------------------------|------------|
| Safety | an `assert!` in every build | a silent clamp, which assigning the public field or deserialising skips; `covering` then panics | one check, at construction; the field cannot hold 7 | the same |
| Ergonomics | `denormalize(l, 2)` | `.level_mod(9)` silently becomes 3 | `LevelMod::new(2)`; `.get()` in arithmetic | `LevelMod::Two`; a conversion back to an integer in arithmetic |
| Python and JavaScript | `ValueError` (ad hoc, A12) | silent | one conversion: `ValueError`, or a thrown `Error` | the same |
| Parity | upstream `DCHECK` | upstream `DCHECK`s, then clamps in release builds | rejects: a documented deviation from the release-build clamp | the same |

`min_level ≤ max_level` relates two settings (class c). It stays a documented debug check of
`covering` in the core. Both bindings check it at their boundary: Python in the constructors,
and wasm when a covering method is called, since its setters are chained one at a time.

**Recommendation: O3, rejecting.** This is adopted.

**Measured (2026-10-27).**

- **Types**: `LevelMod` (1..=3) with `new`, `get`, `branching_factor`, `DEFAULT`,
  `TryFrom<u8|usize|i64>` and a validating `Deserialize`.
- **Signatures and fields**: `CellUnion::denormalize`, the field `RegionCoverer::level_mod`
  and its setter, and `region_term_indexer::Options::level_mod` (a `usize` before).
- **Removed checks**: `denormalize`'s every-build `assert!`, the setter's clamp, and the
  internal coverer's second clamp. The range is now the type's.
- **The library's own code**:
  - 18 integer uses became `.get()`;
  - 3 defaults became `LevelMod::DEFAULT`;
  - 1 internal copy, kept as a `u8` for arithmetic, is re-wrapped when it is handed back to
    `denormalize`.
- **Tests**: 22 call sites (17 literals, 5 conversions), and one older test moved. A
  `denormalize` test of a modulus of 200 now shows the check where it moved, in `LevelMod`.
- **The relation (B12)**: a level-order check that panics in every build now opens each
  public coverer method that reads both levels (`covering`, `interior_covering`, `cell_union`,
  `fast_covering`, `is_canonical`, `canonicalize_covering`) and each of the term indexer's six
  `get_*` methods. Before, it was a `debug_assert!` in two of them: a debug build panicked
  with an anonymous message, and a release build's `covering` and `get_index_terms` did not
  finish (probed in the release-built Python extension).
- **The bindings**:
  - Python: 3 parameters take `LevelModArg`, and `denormalize`'s ad-hoc check was deleted.
    `check_level_order` runs in the `RegionCoverer` and `RegionTermIndexer` constructors.
  - wasm: `setLevelMod` takes an `f64`. The 12 coverings, `isCanonical` and the 10 term
    methods check the order first and return a `Result`, so a reversed pair throws instead of
    trapping. `web.rs` needed 17 call sites updated.
- **Verification**:
  - `cu_crash::n_denormalize__n2` keeps its id-validity half;
  - `n_level_mod_new` and `c_level_mod_try_from` are new, and proven;
  - 4 harness sites wrap the model's `u8` with `LevelMod::new`;
  - M-806 is re-pointed to `LevelMod::new`, and M-810 is new;
  - M-140 and M-150 went stale from a comment edit and were regenerated.

### 3.4 `Cell`'s vertex index (0..=3)

| Dimension | O1 `usize` + panic (before) | O5 wrap modulo 4 (upstream) | **O4 `CellVertex` (planned; gated)** |
|-----------|-----------------------------|-----------------------------|---------------------------------------|
| Safety | an every-build panic at 4 | a mistake becomes a silent vertex | unrepresentable |
| Ergonomics | `vertex(k)`, `(k + 1) % 4` | the same, without the `% 4` | `vertex(CellVertex::UpperLeft)`, `v.next()`; integer uses need `.index()` |
| Consistency | `Cell::edge` already takes `CellEdge`, and `s2::Rect::vertex` takes `RectVertex` | — | matches both |
| Python and JavaScript | `IndexError` (A1), but `OverflowError` at −1; wasm takes `NaN` as 0 | — | one conversion each |

**The gate (spec research R5)**: the enum is adopted unless more than a quarter of the call
sites need an integer conversion.

**Measured (2026-10-27). The gate passes, and O4 is adopted.**

- **The gate.** The compiler listed 111 call sites in `core/src` once `vertex` and `vertex_raw`
  took a `CellVertex`:
  - 60 literals;
  - 19 loops over `0..4`;
  - 24 with wrap arithmetic (`(k + 1) % 4`, `& 3`);
  - 8 that a first pass took to need an integer.

  Reading those 8 showed that 7 are `(0..4).map(|i| …)` loops. Only one genuinely needs an
  integer: a quickcheck property that draws the index as a `u8`. That is about 1% against the
  gate's 25%.
- **Call sites in the core** (with tests, benches and examples): 136 in all, 71 literals and
  65 index conversions (`CellVertex::ALL[i]`). In the library's own code, 21 of the 22
  non-literal sites became enum loops (`for &v in &CellVertex::ALL`, `v.next()`,
  `CellVertex::ALL.map(…)`). One stays an index conversion: `RobustCellClipper` also indexes a
  parallel array by the vertex number. The tests keep the mechanical `CellVertex::ALL[i]`.
- **New API**: `CellVertex` (`ALL`, `next`, `prev`, `index`, `TryFrom<usize|i64>`, serde
  derived as for `CellEdge`), and `TryFrom<i64>` for `CellEdge`, so both bindings convert edge
  indices through the same path.
- **The bindings**:
  - Python: `vertex`, `vertex_raw`, `edge` and `edge_raw` take `CellVertexArg` and
    `CellEdgeArg`, which raise `IndexError` with the core's text, for −1 and 2⁷⁰ too (they gave
    `OverflowError`). Three ad-hoc checks were deleted.
  - wasm: `vertex` and `edge` take an `f64`. Before, `vertex(−1)` reported index 4294967295,
    and `NaN` and 3.5 silently gave vertices 0 and 3.
- **Verification**:
  - `cl_crash::n_vertex` (72 s) is retired for `c_cell_vertex_try_from`, proven;
  - the two vertex stubs (`fixed_vertex`, `pair_vertex`) and `any_uv_bound` now match the
    stubbed signatures exactly;
  - 4 harness sites loop over `CellVertex::ALL`;
  - M-811 is new, and M-726 and M-740 were regenerated (M-740's "three vertices" is now
    `.take(3)`).
- **Performance** (gungraun, against `bde133d`):
  - the coverer and cell-hierarchy benchmarks are flat (−0.4% to +0.7%);
  - `Cell::cap_bound` is unchanged, once its loop iterates `&CellVertex::ALL` by reference
    (by value it cost 6 instructions);
  - `Cell::distance_to_edge` costs 12 more instructions (+0.3%);
  - `Cell::distance_to_cell`'s early-return path costs 14 more instructions (178 → 192,
    +7.9%), and so does the same-face benchmark that goes through it (+7.7%).

  At `9a7cbc0`, before this change, they were 178 and 182, so the cost is the vertex type's.
  The loop shape is not the cause: a range loop measured the same. Neither is `next()`: a
  `match` made `distance_to_edge` worse. The remaining cost is inside `vertex()`, likely an
  inlining decision. Neither function is on R13's list of hot paths, so the cost is recorded
  and not chased.

### 3.4a Adoption checks (US3, 2026-10-27)

- **No bounded type is taken through `impl Into`, and no face as a raw integer.** The search
  `impl Into<…(Level|Face|LevelMod|CellVertex)>` or `face: impl Into<u8>` over `core/src`
  finds nothing.
- **No binding method takes an adopted kind as a raw integer.** The search over
  `python/src` and `wasm/src` finds only parameters that are not adopted kinds:
  - `DensityTree`'s `max_level` (follow-up F11);
  - two container indices (`Polygon.loop_`, `loopAt`);
  - `s2::Rect`'s Python `vertex(k)`, whose core parameter is the existing `RectVertex` enum,
    converted by its own code (follow-up F9).
- **The Python stub** is byte for byte the one at `df687f0`, and stubtest passes.
- **The TypeScript declarations** (`wasm-pack build`, checked 2026-10-27) still take `number`.
  Each changed method's JSDoc states the range and that it throws, for example
  `parentAtLevel(level: number): CellId` with "Throws unless `level` is an integer in
  0..=30".
- **The serialised forms** are unchanged (CO-04).
- **The C++ and Go reference data** (`core/tests/data`, `tools/`) is unchanged since
  `df687f0`, and the differential tests pass.

### 3.5 The other candidates

Sections 2.1 to 2.3 give each other precondition a class and an option. Each relation (class c)
keeps O1, because no single-argument type removes it. Each value-type invariant (classes d and
e) is ranked as a follow-up in section 7.

## 4. Red/green table

Each row is one test of `specs/009-bounded-types/contracts/red-green-tests.md`, run first on the
unfixed tree (red), then after its fix (green).

| ID | Finding | Red output (verbatim excerpt) | Red run (date, commit) | Green commit |
|----|---------|-------------------------------|------------------------|--------------|
| CO-01 | B2 | `serde_json::from_str::<Level>("31")`: `31: Level(31)` (`Ok`, where an error was expected) | 2026-10-27, `bde133d` | `df687f0` |
| CO-02 (`max_level`) | B2 | `{"min_level":0,"max_level":31,…}`: `RegionCoverer { min_level: Level(0), max_level: Level(31), level_mod: 1, max_cells: 8 }` (`Ok`) | 2026-10-27, `bde133d` | `df687f0` |
| CO-04 | R7 | written green: the forms pinned from the unfixed tree. `Level` is a bare number, the default coverer `{"min_level":0,"max_level":30,"level_mod":1,"max_cells":8}`, and face cell 0 `{"face":"F0","level":0,…}`, so `Face` serialises as its variant name | 2026-10-27, `bde133d` | `df687f0` |
| PY-01 | B1, B7 | 93 of 102 cases fail. For 31, 255, 256, −1, 2⁴⁰ and 2⁷⁰ respectively: 13 methods give `PanicException`, `PanicException`, then `OverflowError` four times (`CellId.parent_at_level` … `Polygon.simplified`). `RegionTermIndexer`'s two give `ValueError`, `ValueError`, then `OverflowError` four times; `CellUnion.denormalize` gives `ValueError` five times, then `OverflowError`. **B11**: `S2CellIdSnapFunction(31)` and `(255)` return (the raw `u8` is stored, and converted, with a panic, only when the function is used) | 2026-10-27, `8b0d338` | `0a5b4c9` |
| PY-02, PY-03, PY-04 | guards | already green: legal levels work (17), `3.5` and `"5"` give `TypeError` (34), `True` is level 1 | 2026-10-27, `8b0d338` | `0a5b4c9` |
| PY-10 (levels) | B7 | 13 of 17 methods let a `PanicException` (a `BaseException`) escape | 2026-10-27, `8b0d338` | `0a5b4c9` |
| WA-01 | B1, B7, B8 | all 17 methods fail. `parentAtLevel` gives, for 31, 286, −1, `NaN`, 3.5, `Infinity` and `5n`: `RuntimeError: unreachable` (a trap: `level must be 0..=30` at `core/src/s2/cell_id.rs:357`); **returns level 30**; trap; **returns level 0**; **returns level 3**; **returns level 0**; `Error: expected a number argument, found bigint`. `vertexNeighbors` traps for 31, 286 and −1 (wrapped to 30 and 255) and returns for the others. **B11**: `SnapFunction.cellId` returns for every value, the level converted (with a panic) only when the function is used. The traps come from the core's `From<u8>` (`cell_id.rs:339`, `:357`, `:427`, `:437`, `:769`, `:808`; `cell.rs:356`; `region_coverer.rs:99`, `:105`; `builder/snap.rs:105`) and from the binding (`wasm/src/cell_union.rs:204`, `term_indexer.rs:45`, `:52`) | 2026-10-27, `8b0d338` | `0a5b4c9` |
| WA-02 | guard | already green: 0, 30 and 5.0 are levels | 2026-10-27, `8b0d338` | `0a5b4c9` |
| CD-01 | B1 | `let level: Level = 31u8.into();` and `parent_at_level(5u8)`: "Test compiled successfully, but it's marked `compile_fail`" (both) | 2026-10-27, `8b0d338` | `0a5b4c9` |
| CD-05 | guard | already green: `const TOO_FINE: Level = Level::new(31)` does not compile; the twin (`const FINEST: Level = Level::new(30)`) does | 2026-10-27, `8b0d338` | `0a5b4c9` |
| PY-05 | B3 | all 30 cases fail. For 6, 7, 255, −1, 256 and 2⁷⁰ respectively, `CellId.from_face` and `from_face_pos_level` **return** `CellId(Invalid: d000000000000000)`, `f000000000000000` (7), `f000000000000000` (255 wraps to 7's bits), then `OverflowError` three times. `face_uv_to_xyz`, `get_u_axis` and `get_v_axis` give `ValueError: invalid face: 6`, `: 7`, `: 255`, then `OverflowError` three times | 2026-10-27, `0a5b4c9` | `e514181` |
| PY-10 (faces) | B7 | already green: no face input panics (the wrong results are silent instead) | 2026-10-27, `0a5b4c9` | `e514181` |
| WA-03 | B3, B8 | all 3 methods fail. `CellId.fromFace(6)`, `(7)` and `(−1)` (wrapped to 255) **return invalid ids**; `Cell.fromFace` traps on them (`RuntimeError: unreachable`, `Cell::from_cell_id`'s face check); `NaN` and 3.5 **return faces 0 and 3** in all three | 2026-10-27, `0a5b4c9` | `e514181` |
| CD-02 | B3 | `CellId::from_face(7u8)`: "Test compiled successfully, but it's marked `compile_fail`" | 2026-10-27, `0a5b4c9` | `e514181` |
| PY-06, PY-07 | B5, B6 | all 15 cases fail. `RegionCoverer(level_mod=0)`, `4` and `255` are **silently clamped** to 1, 3 and 3; −1 and 2⁴⁰ give `OverflowError`. `RegionTermIndexer(level_mod=…)` **stores 0, 4, 255 and 2⁴⁰ unchecked** (a `usize`), and with 0 `get_index_terms` then raises `PanicException: attempt to calculate the remainder with a divisor of zero`, in every build; 4 and 255 give terms. `CellUnion.denormalize(0, 0)` raises `ValueError`, but with its own text ("level_mod must be 1, 2 or 3, got 0") | 2026-10-27, `e514181` | `9a7cbc0` |
| PY-08 | FR-010, **B12** | both construct: `RegionCoverer(min_level=10, max_level=5)` and the term indexer. **B12**: in the release-built extension, the coverer's `covering(Cap.full())` and the term indexer's `get_index_terms(Cap.full())` **do not finish within 20 s**: the core's `min_level ≤ max_level` check is debug-only, and its covering loop does not end without it (`interior_covering` returns) | 2026-10-27, `e514181` | `9a7cbc0` |
| PY-10 (moduli) | B7 | already green at construction (no `PanicException` until the term indexer is used) | 2026-10-27, `e514181` | `9a7cbc0` |
| WA-04 | B5, B6, B8 | `RegionCoverer.setLevelMod(0)`, `(4)`, `(NaN)` and `(2.5)` all **return** (clamped, or truncated and clamped) | 2026-10-27, `e514181` | `9a7cbc0` |
| WA-05 | FR-010, B12 | with `setMinLevel(10).setMaxLevel(5)`: the 9 full and interior coverings trap (`RuntimeError: unreachable`, the debug-build check), and the 3 fast coverings **return** (no check). A release build would hang where the debug build traps (B12, PY-08) | 2026-10-27, `e514181` | `9a7cbc0` |
| CO-02 (`level_mod`) | B2, B5 | a coverer JSON with `"level_mod"` 0, 4, 7 or 255 deserialises (`Ok`; with 7, `covering` would then panic in `denormalize`) | 2026-10-27, `e514181` | `9a7cbc0` |
| CD-03 | B5 | `RegionCoverer { level_mod: 7, ..Default::default() }`: "Test compiled successfully, but it's marked `compile_fail`" | 2026-10-27, `e514181` | `9a7cbc0` |
| CO-03 | — | not applicable: `region_term_indexer::Options` does not derive serde (checked 2026-10-27) | — | — |
| LO-01 (core) | B12 | `level_order_tests` and `region_term_indexer_tests` (7 tests, debug build): `covering` and `interior_covering` panic with the anonymous `assertion failed: self.min_level <= self.max_level`; `fast_covering`, `is_canonical` and the term indexer's point and query terms **do not panic at all** | 2026-10-27, `e514181` | `9a7cbc0` |
| PY-09 | B4 | all 12 cases fail. For 4, −1 and 2⁷⁰: `Cell.vertex` and `vertex_raw` give `IndexError: vertex index must be 0, 1, 2, or 3` (the binding's own text), then `OverflowError` twice; `Cell.edge` and `edge_raw` give `IndexError: edge index must be 0, 1, 2, or 3`, then `OverflowError` twice | 2026-10-27, `9a7cbc0` (the installed extension, built from it) | `2738c80` |
| WA-06 | B4, B8 | both fail. `Cell.vertex(4)` throws `cell vertex index 4 out of range (0..4)`; **`vertex(−1)` reports index 4294967295** (ToInt32, then `u32`); `edge(−1)` reports 255; `NaN` and 3.5 **return vertices (and edges) 0 and 3** | 2026-10-27, `9a7cbc0` (a scratch worktree) | `2738c80` |
| CD-04 | B4 | `cell.vertex(4usize)`: "Test compiled successfully, but it's marked `compile_fail`" | 2026-10-27, `9a7cbc0` (a scratch worktree) | `2738c80` |

## 5. Deviations from upstream

Behaviours in which the library differs from upstream C++ (v0.14.0-40) for inputs outside the
domain. For valid inputs, every result is upstream's. Each is documented in the rustdoc:

- `Cell::vertex` and `CellVertex`'s type documentation;
- `LevelMod`'s type documentation, and `RegionCoverer::level_mod` (setter);
- `RegionCoverer::covering` and the other coverer and term-indexer methods (`# Panics`);
- `CellId::from_face`;
- `RegionCoverer::max_cells`.

| Item | Upstream | The library | Reason |
|------|----------|-------------|--------|
| `Cell::vertex(k)`, `vertex_raw(k)` with k ≥ 4 | wraps: `R2Rect::GetVertex` uses `k`'s low two bits (`r2rect.h:214–218`), so `GetVertex(4)` is vertex 0 | panics before this feature; afterwards depends on the R5 gate (section 3.4) | an index of 4 is a caller's mistake. `r2::Rect::vertex(i32)` keeps upstream's wrap |
| `RegionCoverer::level_mod` outside 1..=3 | `ABSL_DCHECK`, then clamps in release builds (`s2region_coverer.cc:79–83`) | clamped before this feature (with no debug check); afterwards unrepresentable, and a raw integer is rejected by `LevelMod::try_from` | one policy for one quantity: `denormalize` already rejected it in every build |
| `RegionCoverer` levels outside 0..=30 | `ABSL_DCHECK`, then clamps (`s2region_coverer.cc:60–71`) | rejected (`Level`), in every build: unchanged by this feature, but now visible at the call | as above |
| `RegionCoverer` coverings and the term indexer's terms with `min_level > max_level` | `ABSL_DCHECK` only ("checked by `S2RegionCoverer`") | panics in every build, with "min_level (10) must not exceed max_level (5)"; the bindings raise their native error first | a release build looped (B12): a panic is a defined failure, a hang is not |
| `RegionCoverer::max_cells(0)` | stored as 0 (`s2region_coverer.cc:56–58`) | clamped to 1 | pre-existing; documented, not changed (follow-up F10) |
| `CellId::from_face`, `from_face_pos_level` with face 6 or 7 | an undefined id (`FromFace(int)` is unchecked) | unrepresentable (`Face`) | the `Face` type already existed |

<a id="migration-notes"></a>

## 6. Migration notes

The breaking changes of spec 009, for the next minor release. Each breaking commit's
`BREAKING CHANGE` footer names its entries. `9a7cbc0`'s footer describes M11 in words
("reversed coverer levels panic in every build"), because the entry was numbered after the
commit.

### Rust (`s2rst`)

| # | Commit | Change | Before | After |
|---|--------|--------|--------|-------|
| M1 | `0a5b4c9` | `From<u8> for Level` is removed | `let l: Level = 5.into();` | `Level::new(5)` for a literal; `Level::try_from(n)?` for a value |
| M2 | `0a5b4c9` | The 25 functions that took `impl Into<Level>` take `Level` (the 23 of the signature audit, and `point_compression::encode_points_compressed` and `decode_points_compressed`) | `id.parent_at_level(5)`, `CellId::begin(level_u8)` | `id.parent_at_level(Level::new(5))`, `CellId::begin(Level::try_from(level_u8)?)`; a value already of type `Level` (`id.level()`, `level + 1`, `Level::MAX`) passes unchanged |
| M3 | `0a5b4c9` | `Level`'s `TryFrom` error type | `Err(&'static str)` | `Err(OutOfRange)`, which implements `Display` and `Error`; `TryFrom<u8>` and `TryFrom<i64>` are new |
| M4 | `e514181` | `Face`'s `TryFrom<u8>` error type | `Err(u8)` | `Err(OutOfRange)`, whose `value()` gives the input; `TryFrom<i64>` is new |
| M5 | `e514181` | `CellId::from_face`, `Cell::from_face` and both `from_face_pos_level` take `Face` | `CellId::from_face(2)`, `for f in 0..6 { CellId::from_face(f) }` | `CellId::from_face(Face::F2)`, `for f in Face::ALL { … }`; from a value, `Face::try_from(n)?` |
| M6 | `9a7cbc0` | `CellUnion::denormalize` takes `LevelMod` | `cu.denormalize(min, 2)` | `cu.denormalize(min, LevelMod::new(2))` |
| M7 | `9a7cbc0` | `RegionCoverer::level_mod` (field and setter) is a `LevelMod`, and the setter no longer clamps | `.level_mod(2)`, `c.level_mod = 2`; `.level_mod(9)` was clamped to 3 | `.level_mod(LevelMod::new(2))`; a raw 9 is the caller's to handle (`LevelMod::try_from(9)` is an `Err`) |
| M8 | `9a7cbc0` | `region_term_indexer::Options::level_mod` is a `LevelMod` | `opts.level_mod = 1` (a `usize`) | `opts.level_mod = LevelMod::DEFAULT` |
| M9 | `2738c80` | `Cell::vertex` and `vertex_raw` take a `CellVertex` | `cell.vertex(0)`, `for k in 0..4 { cell.vertex(k) }`, `cell.vertex((k + 1) % 4)` | `cell.vertex(CellVertex::LowerLeft)`, `for v in CellVertex::ALL { … }`, `cell.vertex(v.next())`; from a value, `CellVertex::try_from(k)?` |
| M10 | `df687f0`, `9a7cbc0` | Deserialising an out-of-range `Level`, `LevelMod` or `RegionCoverer` fails | `{"level_mod": 7}` deserialised, and `covering` then panicked | a deserialisation error; in-range data reads back byte for byte as before |
| M11 | `9a7cbc0` | The coverer's coverings, `is_canonical` and `canonicalize_covering`, and the term indexer's six `get_*` methods panic in every build when `min_level > max_level` | a debug-build panic in two of them; in a release build `covering` and `get_index_terms` did not finish | order the levels; the message names both ("min_level (10) must not exceed max_level (5)") |

### Python (`s2rst`)

No signature changes; the stub is byte for byte as before.

| # | Change | Before | After |
|---|--------|--------|-------|
| P1 | out-of-range levels, faces and moduli raise `ValueError`, with the core's text | `PanicException` (31), `OverflowError` (256, −1, 2⁷⁰), an invalid id (`from_face(7)`), or a silent clamp (`level_mod`) | `ValueError`; code that caught `OverflowError` catches `ValueError` |
| P2 | vertex and edge indices out of range always raise `IndexError`, with the core's text | `IndexError` for 4 (other wording), `OverflowError` for −1 | `IndexError` for both |
| P3 | `RegionCoverer(level_mod=0)` and `level_mod=9` raise `ValueError`; so does `RegionTermIndexer(level_mod=0)` | silently 1 and 3; the term indexer stored 0, then divided by zero | pass 1..=3 |
| P4 | `RegionCoverer(min_level > max_level)` and `RegionTermIndexer` likewise raise `ValueError` at construction | constructed, and `covering` or `get_index_terms` did not finish | order the levels |
| P5 | `S2CellIdSnapFunction(31)` and `S2Builder(snap_level=31)` raise `ValueError` when created | accepted, then a `PanicException` when used | pass 0..=30 |

### JavaScript (the `s2rst_wasm` package)

No signature changes; the TypeScript types stay `number`. The thrown value is the message
string, as for every expected failure of the wasm API.

| # | Change | Before | After |
|---|--------|--------|-------|
| J1 | Methods taking a level, face, modulus, or vertex or edge index throw on bad input | a trap (`RuntimeError: unreachable`), or a silent wrap: 286 became level 30, −1 vertex index 4294967295, `NaN` 0, 3.5 3 | a catchable throw; the instance stays usable |
| J2 | `setLevelMod(0)` and `setLevelMod(9)` throw | silently clamped | pass 1..=3 |
| J3 | the coverings, `isCanonical` and the term indexer's term methods throw when the minimum level exceeds the maximum | a trap in a debug build; a hang in a release build | order the levels |
| J4 | these methods return a `Result` in Rust (`wasm-bindgen` throws on `Err`) | — | Rust callers of the binding crate `unwrap()` or `?` |

## 7. Follow-ups

Ranked by the safety gained per unit of cost. Costs are estimated from this feature's
measurements: about 560 call sites for a type used throughout the core (`Level`, `Face`), and
about 140 for one used in one module (`CellVertex`).

| Rank | Item | Class | What it would remove | Estimated cost |
|------|------|-------|----------------------|----------------|
| 1 | **F9: the remaining small enums**: `s1::Interval::bound(i)` → `r1::Endpoint`; `get_uvw_axis` and `get_uvw_face` → `r3::Axis` and a direction enum; Python's `Rect.vertex(k)` through the shared conversion | (b) | every-build index panics, and one ad-hoc binding check | small: the enums exist, and there are few call sites |
| 2 | **F11: `DensityTree::new(…, max_level: u8)` → `Level`** | (a) | a silent clamp | small: one signature, and one Python parameter |
| 3 | **The wasm error funnel throws `Error` objects** (`js_err` → `js_sys::Error::new`) | — | JavaScript callers get `e.message` and a stack, not a bare string | one line, but it changes every expected failure of the wasm API, so it is a breaking change of its own |
| 4 | **A `Level` iterator** (`Level::all()`, or a `level!` macro for checked literals) | ergonomics | most of `Level`'s 213 conversions: test loops over `0..=30` | small, additive |
| 5 | **F10: `RegionCoverer::max_cells(0)` clamps to 1**, where upstream stores 0 | (a) | a deviation | decide: match upstream, or keep and document (it is documented now) |
| 6 | **F5: a validated `CellId`** (`level`, the hierarchy and union operations assume validity) | (e) | the id-validity rows of section 2.1 (about 20 operations) | large: `CellId` is the most used type, and invalid ids are legitimate values (`none`, `sentinel`, the decoders) |
| 7 | **F1: a unit-length point type** | (d) | the unit-point preconditions (`Cap::from_center_angle`, `contains_point`, the predicates) | very large: it reaches every geometry API, and normalisation is not exact, so the type needs a tolerance policy |
| 8 | **F2, F3, F4: validated `s1::Interval`, `r1::Interval`, `s2::Rect`, `r2::Rect`, `Cap`, latitude and longitude** | (d), (e) | the debug-only constructor checks and their release-build invalid values | large, per type; spec 007 did it for `ChordAngle` |
| 9 | **F6: typestate for a normalised `CellUnion`** | (e) | the "unspecified on an invalid union" sections | large: every set operation's signature |
| 10 | **F7, F8: i-j and si-ti coordinates; the decoders' byte lengths** | (a) | panics no public caller can reach with ordinary input | low value |
| — | **Python `IntEnum` for faces; TypeScript literal unions** (`0 \| 1 \| … \| 5`, through wasm-bindgen's `unchecked_param_type`) | static checking | catching a bad face before the call, in mypy or TypeScript | small, but changes the stub and the declarations, and needs typed return values to avoid casts |
| — | **`Cell::distance_to_cell`'s 14 extra instructions** (section 3.4) | performance | 7.9% of one micro-benchmark | inspect the generated code of `Cell::vertex` |

Relational preconditions (class c) stay preconditions: a level finer than the cell's own, the
parent of a face cell, the children of a leaf, `min ≤ max`. No type of one argument can carry
them. The bindings check the one that users meet directly, the coverer's levels, at their
boundary.
