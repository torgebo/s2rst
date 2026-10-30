# Verified types and traits

Which types and traits of the `s2rst` crate (`core/`) have proofs, what those proofs cover,
what has none, whether the existing proofs can be made simpler, and the remaining work as
implementation steps.

State on branch `003-next-specs/1` at commit `537e7fd` (2026-10-07), with the six value types
of specs 004–006 added on 2026-10-14 (sections 2.4–2.6) and spec 007's two on 2026-10-18
(section 2.7; the counts in section 1 and the tables of sections 3 and 4 include them), and
spec 008's `Cap` on 2026-10-25 (section 2.9), spec 009's bounded types on 2026-10-27 (section
2.10), spec 010's `LatLng`, with `Rect`'s `Region` laws, on 2026-10-28 (section 2.11), and spec 011's
`PointRegion`, with `CellUnion`'s floating-point methods, on 2026-10-28 (section 2.12), and spec
012's region adaptors and `Cell`'s bounding laws on 2026-10-30 (section 2.13). Sources: the inventories (`verify/INVENTORY.md`,
`verify/cell_union/INVENTORY.md`, `verify/decoding/INVENTORY.md`, `verify/values/INVENTORY.md`),
their property catalogues and evaluations, and `cargo kani list` on the current tree.

**What counts as a proof here.** A Kani harness checks the library's own code for every input
in a stated domain, sometimes within a size bound (bounded model checking). A Verus proof covers
every input and every length, but of a restated copy of the library function, linked back to
it by bounded agreement, tests and a drift guard. Tests, property tests, fuzzing and the C++
differential vectors are evidence, not proofs; section 4 lists them for the uncovered types.

## 1. At a glance

| | |
|---|---|
| Crate size | 113,700 non-test lines in 127 files; `s2/` alone is 105,200, of which `builder/` 18,300 and `boolean_operation/` 6,300 |
| Public items | 196 structs, 35 enums, 13 trait definitions (9 names; `Target` and `DistanceTarget` are defined in several query modules) |
| Code the proofs exercise | about 6,500 lines (5.7 %): `cell_id.rs`, `cell_union.rs`, the four vector codec files, and the codec impls in `encoding.rs`; and the value types' files, about 9,400 lines with their tests' files left out (`s2/cell.rs`, `s1/interval.rs`, `r1/interval.rs`, `s2/rect.rs`, `r2/rect.rs`, `s2/point.rs`, `s1/chord_angle.rs`, `s1/angle.rs`, `s2/cap.rs`, `s2/latlng.rs`). Not every line of those files is proven (section 2) |
| Types with their own proof suite | `CellId` (spec 001), `CellUnion` (spec 002), the decoding layer (spec 003), `Cell` and `s1::Interval` (spec 004, section 2.4), `r1::Interval` and `s2::Rect` (spec 005, section 2.5), `r2::Rect` and `s2::Point` (spec 006, section 2.6), `s1::ChordAngle` and `s1::Angle` (spec 007, section 2.7), `s2::Cap` (spec 008, section 2.9), `s2::LatLng` (spec 010, section 2.11), `PointRegion` (spec 011, section 2.12), `RegionUnion` and `RegionIntersection` (spec 012, section 2.13) |
| Types reached only through other proofs | `Level`, `Face` (section 2.8) |
| Traits with proofs | `S2Encode`/`S2Decode` for 6 of 12 implementors; `Region` for `CellUnion` (the cell tests, 002; the point test, both bounds' containment of each cell's bound and containment implies intersection, 011) and the generic laws for `Cell`, `Cap` (three of four, with lemma stubs), `Rect` (two of four, under contract, spec 010) and `PointRegion` (five of six, spec 011, with the two new laws of the bounding rectangle and the covering); and, by preservation over any members, `RegionUnion` and `RegionIntersection` (three of six, spec 012); the codec round trip under `Cap`'s own equality; the equality laws for the ten value types, and the order laws for `Point`, `ChordAngle` and `Angle` |
| Kani harnesses | 557 per change (`CellId` 196, `CellUnion` 120, decoding 241) and 272 for the value types (spec 007 adds 36, spec 008 45), plus 27 from spec 010 (24 `ll_*`, 3 `ll_reg`) and 42 from spec 011 (15 `pr_*`, 27 `uf_*`) and 32 from spec 012 (`ad_*`); 173 more weekly (91, 45, 37) and 87 for the value types (spec 007 adds 10, spec 008 13 and removes 006's `p_ptdst_01__symmetric`), plus 2 from spec 010, 21 from spec 011 and 8 from spec 012; 16 xlarge (decoding, never run). Spec 007 also proves 4 statements by enumerating every `i32` natively (`verify/tests/angle_enumeration.rs`) |
| Verus | 5 restated codecs, 132 obligations, 17 s |
| Mutants | 38 `M-0NN`, 25 `M-1NN`, 35 `M-2NN`, 10 `V-NNN`; all killed except `M-222`, whose target is xlarge. For the value types 45 `M-3NN`, 43 `M-4NN`, 40 `M-5NN`: all killed except `M-344`, `M-345` and `M-539`, whose targets time out (no verdict); 39 `M-6NN` for spec 007, 41 `M-7NN` for spec 008, 37 `M-901` … `M-937` for spec 010 24 `M-950` … `M-973` for spec 011 and 14 `M-974` … `M-987` for spec 012, all killed |

## 2. Types with proofs

### 2.1 `CellId` (spec 001)

- **Scope:** 48 of 64 operations (54 methods, 3 free functions, 7 trait impls).
- **Proven:** crash freedom of all 48 for every input in its documented domain (any bits, or
  valid ids); a 49-entry property catalogue against the model `verify/src/spec.rs`: encoding
  round trips, hierarchy, ranges, traversal, common ancestors, text formats, neighbours.
- **Bounds:** the three text parsers within 17 or 33 bytes (longer input is rejected by a
  length check before any loop); vertex and edge neighbours only weekly.
- **Not proven:** 12 floating-point operations (`from_point`, `from_lat_lng`, `to_point`,
  `to_point_raw`, `to_lat_lng`, `center_st`, `center_uv`, `size_st`, `bound_st`, `bound_uv`,
  `From<&Point>`, `From<&LatLng>`); `all_neighbors` (out of memory above 9.5 GB); `to_token`,
  `to_debug_string` and `Display` (`core::fmt` is too slow for Kani).
- **Records:** `verify/INVENTORY.md`, `PROPERTIES.md`, `EVALUATION.md`, `MUTANTS.md`.

### 2.2 `CellUnion` (spec 002)

- **Scope:** 27 of 42 operations (29 inherent methods, 13 trait methods).
- **Proven:** crash freedom within size bounds (one to four cells, byte bounds for decoding)
  and a 27-entry property catalogue against `verify/src/union_spec.rs`, a model of the leaf
  cells a union covers: normalisation, membership, set operations, denormalisation, measures,
  the verbatim encoding.
- **Not proven:** `from_begin_end`, `from_range`, `from_min_max` and `difference` (tool
  limits); `expand_at_level` (it calls `CellId::all_neighbors`); `Display`. The floating-point
  operations are spec 011's (section 2.12): six now proven within bounds, `exact_area`,
  `cell_union_bound` and `expand_by_radius` inheriting tool limits.
- **Records:** `verify/cell_union/`.

### 2.3 The decoding layer (spec 003)

- **Scope:** 22 of 24 operations: the fixed-width helpers and the integer, cell-id, string and
  point vector codecs (`encoded_uint_vector`, `encoded_s2cell_id_vector`,
  `encoded_string_vector`, `encoded_s2point_vector`), and `S2Encode`/`S2Decode` for `CellId`,
  `Point`, `Cap`, `Rect` and `Cell`.
- **Proven (Kani):** crash freedom, and one contract per codec (round trip, exact consumption,
  sound acceptance, rejection) against a reference description of upstream's formats
  (`verify/src/wire_spec.rs`). Whole domains for the fixed-width helpers and the five
  fixed-size codecs (`Cap` at full length only weekly). Every input up to 8 bytes per change
  for the integer vector decoders and up to 4 for the cell-id vector decoder, both up to 11
  weekly; encoders up to two values.
- **Proven (Verus):** the varint, fixed-width, integer vector, cell-id vector and string vector
  codecs, for every input and length, on restated copies in `verify/deductive/`.
- **Not proven:** the compact point-vector encoder (excluded by design); the string vector
  decoder on the Kani side (a tool limit: only the Verus proof covers it); the point vector
  decoder beyond 1 byte (a tool limit); the cell-id encoder beyond zero ids on the Kani side
  (xlarge, not run).
- **Records:** `verify/decoding/`.

### 2.4 `Cell` and `s1::Interval` (spec 004)

- **Scope:** `Cell` 21 of 53 entries (its integer operations and the two stored-bound readers;
  31 floating-point geometry entries and `Display` excluded); `s1::Interval` 30 of 31
  (`Display` excluded).
- **Proven:** crash freedom of all 51 over their whole documented domain (19 crash harnesses).
  The catalogue has 41 entries, against `cell_spec` (a cell is its identifier's decomposition
  in 001's model) and `arc_spec` (an interval is the set of real arcs it denotes):
  - 35 are proven over their whole domain, among them the proof that the seam fix (FR-020)
    changes the length of exactly one valid interval;
  - 4 are proven within bounds: the four families of cell bounds, per level;
  - the generic equality and `Region` laws (`verify/src/laws.rs`) were introduced here.
- **Bounds:** the bound families at levels 0–3 per change, and weekly up to level 12
  (P-CBND-01, -02), 6 (P-CCHD-02) and 10 (P-CEQ-02), all proven; the higher levels are
  tested by `tests/uv_projection.rs` and the C++ vectors.
- **Weekly:** 41 members (28 of `Cell`, 13 of `s1::Interval`), `make verify-slow F=values
  P=cell` and `P=interval`.
- **Not proven:**
  - P-IEXP-02's symmetry and default form, and P-IEXP-03 (FR-023's fix is narrow): their
    members time out at 60 minutes (tool limits);
  - `Cell`'s floating-point geometry (vertices, edges, area, its `Region` point methods);
  - `Display`.
- **Found and fixed:** K2 (FR-018, `Cell` deserialises only consistent cells), K4 (FR-020,
  the one-ulp interval across the seam), D-005, D-006 (`Rect::cap_bound`, found by the
  generic trait tests) and D-007 (U-011, `expanded` crossing its bounds; FR-023). The
  upstream findings are U-007 and U-011.
- **Records:** `verify/values/INVENTORY.md` §1–2, `PROPERTIES.md` §1–3, `MUTANTS.md` (`M-301`
  … `M-345`), and the spec-004 part of `EVALUATION.md`. That part is still interim: spec
  004's evaluation and final run (its T046, T049) are open.

### 2.5 `r1::Interval` and `s2::Rect` (spec 005)

- **Scope:** `r1::Interval` 26 of 27 entries (`Display` excluded); `s2::Rect` 34 of 53 (18
  floating-point geometry entries and `Display` excluded; its binary codec is spec 003's).
- **Proven:** crash freedom of all 60 over their whole documented domain (21 crash
  harnesses), whose arithmetic operations leave out exactly the infinite corners FR-017
  documents. 30 of the 31 catalogue entries are proven over their whole domain, against
  `real_spec` (an interval is the set of reals between its bounds) and `rect_spec`, the
  product of `real_spec` and spec 004's `arc_spec`. 12 of the rectangle's 16 entries are
  stated through `arc_spec`, and 14 rely on spec 004's results.
- **Weekly:** 5 members (`make verify-slow F=values P=real` and `P=rect`): the real
  interval's approximate-equality symmetry, the rectangle's bounds, and its expansion
  containment.
- **Not proven:** P-LEXP-02's symmetry (the rectangle's approximate equality), which times
  out at 60 minutes; the rectangle's floating-point geometry (area, distances, `cap_bound`,
  the `Region` cell methods), which property tests and D-006's tests cover; `Display`.
- **Found and fixed:** K4 (FR-020, deserialisation rejects an invalid rectangle); the
  documentation of FR-017 to FR-019. The upstream findings are U-008 (undefined behaviour in
  `R1Interval::Project`), U-009 and U-010.
- **Records:** `verify/values/INVENTORY.md` §3–4, `PROPERTIES.md` §4–5, the spec-005 part of
  `EVALUATION.md`, `MUTANTS.md` (`M-401` … `M-443`).

### 2.6 `r2::Rect` and `s2::Point` (spec 006)

- **Scope:** `r2::Rect` 32 of 33 operations (30 inherent, `Default`, `PartialEq`; `Display`
  excluded); `Point` 25 of 31 (24 in scope and `rotate`; the 5 operations through `atan2` are
  tool limits; `Display` excluded).
- **Proven:** crash freedom of every in-scope operation over its documented domain (`Point`'s
  `from_coords` and `normalize` for coordinates up to `2⁵¹¹`). All 16 catalogue entries of
  `r2::Rect`, stated as products of spec 005's real-interval description; 10 of `Point`'s 16:
  the order for every triple (NaN included) and its total-order laws without NaN, the
  classifications, the exact power-of-two scaling (FR-020) and its agreement with upstream
  where upstream is finite, the arithmetic bit for bit, and the orthogonal directions never
  zero over the box `[−1, 1]³`.
- **Weekly:** 5 members of `r2::Rect` and 14 of `Point` (`make verify-slow F=values P=uvrect`
  and `P=point`).
- **Not proven:** comparisons of two normalisations (the checker's `sqrt` does not fix its
  result: P-PTVAL-02's normalising clause, P-PTORT-01), anything after `atan2` (P-PTDST-02), and
  two statements that time out at 60 minutes, also on the points whose largest coordinate is
  at least ½ (unit length, `point_cross`'s sign laws). The C++ vectors and property tests check
  each. The chord angle's symmetry, a third such timeout, is proven by composition since spec 008
  (`cp_lem::l_cp_03`, `l_cp_04`: the differences negate exactly and their squares agree).
- **Records:** `verify/values/INVENTORY.md` §5–6, `PROPERTIES.md` §6–7, the spec-006 part of
  `EVALUATION.md`, `MUTANTS.md` (`M-501` … `M-540`).

### 2.7 `s1::ChordAngle` and `s1::Angle` (spec 007)

- **Scope:** `ChordAngle` 35 of 36 entries (28 with crash proofs; the 7 through `asin`,
  `to_angle` and what reads it, are tool limits; `Display` excluded); `Angle` 30 of 31 (29
  with crash proofs; `tan` a tool limit; `Display` excluded).
- **Changed first (option B, FR-017 to FR-023):** every chord angle is canonical (−1, a length
  in [0, 4], or +∞), so the type keeps its own invariant, a first among the value types; NaN gives
  `STRAIGHT` as upstream; the arithmetic checks special operands in debug builds; deserialisation
  returns only canonical values (since the v0.4.0..v0.5.0 review, any negative reads as `NEGATIVE`
  and JSON `null` as `INFINITY`; a length above 4 or NaN is an error); `normalized` is the exact remainder; the E-forms round ties to
  even and saturate. Kani found D-008 (`max_angle_error`, fixed).
- **Proven:** crash freedom of every in-scope operation; 13 of `ChordAngle`'s 14 catalogue
  entries per change (construction, the canonical-value invariant, classifications, the total
  order, the neighbours, `plus_error`, the error bounds, the arithmetic); of `Angle`'s 8, the
  representation, the order, upstream's degree–radian identities, and the E-form round trips by
  enumeration over every `i32` (`proven-by-enumeration`).
- **Weekly:** 2 members of `ChordAngle` (`sin2`, `sin`) and 8 of `Angle` (the conversions'
  products, the E-forms over every `f64`, the products and quotients): two float products
  compared bit for bit time out at 12 minutes. In the weekly runs (2026-10-18), 6 of `Angle`'s
  are proven (6 to 16 minutes each), so P-AGCNV-01 and P-AGEF-01 are `proven`; `sin2`, `sin`
  and the two quotients time out at 60 minutes (`verify/known-timeouts.txt`).
- **Not proven:** `normalized`'s value (Kani's `%` on `f64` is wrong, KI-002 in
  `KANI_ISSUES.md`); `ChordAngle`'s `sin2` and `sin` and the angle's quotients (timeouts);
  everything through `asin` and `Angle::tan`.
  The C++ vectors (bit for bit) and an exact big-integer property test check them.
- **Records:** `verify/values/INVENTORY.md` §7–8, `PROPERTIES.md` §8–9, the spec-007 part of
  `EVALUATION.md`, `MUTANTS.md` (`M-601` … `M-648`, all 39 killed).

### 2.8 Types reached only through those proofs

| Type | What the proofs touch | What has no proof |
|------|-----------------------|-------------------|
| `Level`, `Face` (`coords.rs`) | built and read by every `CellId` harness; the Hilbert tables as `CellId`'s traversal uses them | the conversions as functions of their own (`st_to_ij`, `si_ti_to_st`), and the tables' own laws |

### 2.9 `s2::Cap` (spec 008)

- **Scope:** 42 of 43 entries (the 32 inherent operations, `PartialEq`, the six `Region`
  methods, the codec and `Deserialize`; `Display` excluded).
- **Changed first (K1–K7, FR-017 to FR-023):** the covering level, the cell tests (upstream's
  vertex-and-edge algorithm in place of the bounding-cap test), the complement's radius, the
  longitude reduction of `rect_bound`, `approx_eq_with`'s centre tolerance, and a validating
  `Deserialize`; each with a test that failed before. Then K8: the empty cap's covering is
  upstream's four cells (it was none). Differential tests against new C++ vectors
  (`cpp_cap_diff`) pin the remaining differences: K9 and U-015 (neither union always contains
  both caps) and K10 (the region coverer, a follow-up).
- **Two new methods.** *Function contracts* replace `asin`, `atan2` and `cos` (unsupported or
  loosely modelled by the checker) by their ranges, tested against libm: 8 entries are crash-proven
  under contract. *Lemma stubs* replace a function a law computes twice on one input by a pure
  function whose facts (L-CP-01 to -08) are proven on the real code: every law that timed out in
  the first probe round proves per change in seconds to minutes.
- **Proven:** crash freedom of 31 entries on the real code or with lemma stubs and 5 under
  contract, over their whole documented domains; of the 25 counted catalogue entries, 22 (88 %):
  construction (but `from_center_area`), classifications, membership, containment and
  intersection laws, growth, the complement, the bounds' delegation, level, validity and longitude
  reduction, equality and approximate equality, and three of the four generic `Region` laws and
  the codec round trip under `Cap`'s equality. Spec 006's chord-angle symmetry is proven by
  composition, and `Cell::cap_bound` contains the cell's vertices (P-CCAP-01, weekly).
- **Weekly:** 13 members (`make verify-slow F=values P=...`, the `values-cap` legs). In the run of
  2026-10-25, 6 prove (5 to 17 minutes; `expanded` on the real code with no stub), 5 time out at
  60 minutes (`verify/known-timeouts.txt`), `union`'s crash proof runs out of memory at 8 GiB (and
  times out at 16 GiB), and the area's bit-for-bit statement failed on D-009. Since D-009's fix it
  proves (20 minutes).
- **Not proven:** the covering law (P-CPREG-04: the level needs `asin`'s value); the values of
  everything through `asin` and `atan2` (crash proofs only); `from_center_area`'s and the
  centroid's products and `cell_union_bound`'s crash proof (timeouts); `union` (out of memory, and
  no property holds); `Deserialize` (tested). The C++ vectors check each.
- **Found and fixed:** D-009, the area of a cap with radius length² −0 was −0 in a debug build
  (upstream +0); fixed with the maintainer's go-ahead (`1b26156`).
- **Records:** `verify/values/INVENTORY.md` §9, `PROPERTIES.md` §10, the spec-008 part of
  `EVALUATION.md`, `TRUSTED_BASE.md` (the contracts and stubs), `MUTANTS.md` (`M-701` …
  `M-741`).

### 2.10 Bounded types (spec 009)

Spec 009 made four range-restricted parameters into types that are checked when an instance is
created (`BOUNDED_TYPES.md`). Each has a proof that its constructors accept exactly its range,
and the operations that took the raw values take the type, so they cannot be handed a value
outside it:

| Type | Range | Constructor proofs | Operations that take it |
|------|-------|--------------------|-------------------------|
| `Level` | 0..=30 | `crash::c_level_try_from` (`try_from` exact, `OutOfRange` with kind and value), `crash::n_level_new` (`new` panics exactly above 30) | the 25 functions that took `impl Into<Level>`; `crash::n_raw_level` retired |
| `Face` | 0..=5 | `crash::c_face_try_from`, `crash::n_face_from_u8` | `CellId::from_face`, `Cell::from_face`, both `from_face_pos_level`; `cl_crash::n_from_cell_id` lost its face cases |
| `LevelMod` | 1..=3 | `cu_crash::c_level_mod_try_from`, `cu_crash::n_level_mod_new` | `CellUnion::denormalize`, `RegionCoverer::level_mod`, the term indexer's options; `n_denormalize__n2` keeps only its id half |
| `CellVertex` | 0..=3 | `cl_crash::c_cell_vertex_try_from` | `Cell::vertex`, `Cell::vertex_raw`; `cl_crash::n_vertex` retired |

Non-vacuity: M-803 and M-806 (re-pointed to `Level::new` and `LevelMod::new`), and M-809, M-810
and M-811 (each `try_from` accepting one value too many). The proofs take under a second each.

### 2.11 `s2::LatLng`, and `s2::Rect` as a `Region` (spec 010)

- **Scope:** 31 of 33 `LatLng` entries (the 19 inherent operations, the four operators,
  `Default`, `PartialEq`, both `From<Point>` conversions, `Deserialize`, the four `geo-types`
  conversions; `Display` and `to_string_in_degrees`' formatting excluded), and the four generic
  `Region` laws for `Rect`.
- **Changed first (K1, K3, K4; FR-017 to FR-019), each with a test that failed before:**
  `normalized` reduces the longitude with upstream's exact `remainder(lng, 2π)`, shared with
  `Angle::normalized` (`s1::angle::remainder_2pi`; it returned −4.0 for 10¹⁶); `get_distance`
  clamps before the root (NaN where upstream gives NaN); `Rect::cap_bound` widens upstream's cap
  by 10⁻¹⁵ rad and 8ε of its length², since upstream's misses contained points next to the
  boundary (U-016). New C++ vectors (`cpp_latlng_vectors.csv`, seven sections) match every row
  but the widened radii.
- **Proven:** crash freedom of the 26 Kani entries over every pair of `f64` (the distance over
  its documented domain, and outside it every finite pair), 9 harnesses in 35.5 s; all 19 counted
  catalogue entries: validity, the clamp, the exact reduction's step, `from_point`'s validity for
  every point without NaN, the distance's range and zero, the arithmetic, equality, and
  approximate equality (weekly), plus five by composition (the restatements through drift guards,
  the constructors with `Angle`'s uninterpreted, `normalized`'s remainder through IEEE `fmod`).
  For `Rect`: containment implies intersection (P-LREG-01) and the vertex law (P-LREG-02), and
  its point test receives a valid `LatLng` (P-LMEM-03).
- **New methods.** *Pure contracts*: one contract value per input, so a restatement that calls
  `sin`, `cos`, `atan2` or `sqrt` again gets the library's value. *A contract on a library
  function*: `Cell::rect_bound` contains its vertices' `LatLng`s, tested on every cell to level 5
  and 10⁶ random cells. *Drift guards for formulas*: a restatement compared with the library's
  text, where a second copy of its float products times out.
- **Not proven:** the distance's symmetry within a tolerance (needs the functions' values);
  `Rect`'s bounding-cap and covering laws (P-LREG-03, -04: the values of `atan2`, `sin_cos` and
  `asin`); the C++ vectors, the native boundary sweep and the quickcheck twins check them.
- **Missing upstream operations (FR-020, follow-ups):** `S2LatLng::Encode`/`Init` (a 16-byte
  codec) and `S2LatLngRect::Intersects(const S2Cell&)` (the exact cell test; the library's
  `intersects_cell` is upstream's `MayIntersect`).
- **Records:** `verify/values/INVENTORY.md` §12, `PROPERTIES.md` §11 and `Rect`'s rows in §5,
  the spec-010 part of `EVALUATION.md`, `TRUSTED_BASE.md`, `MUTANTS.md` (`M-901` … `M-937`),
  `UPSTREAM.md` (U-016).

### 2.12 `PointRegion`, and `CellUnion`'s floating-point methods (spec 011)

- **Scope:** `PointRegion`'s 12 entries (`new`, `point`, the six `Region` methods,
  `PartialEq`, `Deserialize`; `Debug` excluded) and its six `Region` laws; `CellUnion`'s nine
  floating-point inventory rows of spec 002 (the point tests, the three areas,
  `expand_by_radius`, the three bounds) and its `Region` laws.
- **Changed first (K1–K4; FR-017 to FR-019), each with a test that failed before:**
  `Cell::rect_bound`'s polar faces use upstream's `asin(sqrt(1/3))`, not Go's 15-digit literal
  2 ulps below it; the empty union's approximate and exact areas are +0, not −0; `PointRegion`
  documents upstream's unit-point precondition and its deserialiser rejects other points; and
  four debug-build panics that upstream's `DCHECK`s share are documented (`expand_by_radius` on
  NaN, `Cell::contains_point` off its face, `CellId::from_point` with two infinite coordinates,
  `s1::Interval::from_point` on NaN). New C++ vectors (`cpp_region_vectors.csv`, seven
  sections, 23,271 rows) match every row; the survey probe matches C++ in every section.
- **Proven:** `PointRegion`'s crash freedom over every triple (its documented debug panics
  exact), 13 of its 14 counted catalogue entries, and five of its six `Region` laws: containment
  implies intersection, the covering (vacuous), the bounding cap, the bounding rectangle and the
  covering containing the point. The vertex law (a cell contains its own vertices) times out at
  60 minutes per face: a tool limit, tested on 80,000 vertices.
  `CellUnion`: crash freedom of six rows within 002's size bounds; the point test, both bounds'
  containment of each cell's bound (the cap's for every length, by composition), the empty
  union's bounds, the areas' sums, and containment implies intersection.
- **New methods.** Contracts on library functions (`CellId::from_point` returns a valid leaf,
  one per point; `Cell::cap_bound`'s centre in the box), uninterpreted stubs for values the
  statements do not need (`Cell::contains_point`, the cells' areas, `average_area_for_level`,
  since the checker's `powi` is not a function, KI-013), and drift guards for upstream's
  constructions.
- **Not proven:** `CellUnion`'s bounding-cap and rectangle laws (they wait on `Cell`'s own
  bounding laws), its covering laws (`asin`'s value, as `Cap`'s), `expand_by_radius`'s
  containment (`all_neighbors`); `exact_area` and `cell_union_bound` inherit tool limits. The
  vertex law is not a law of `CellUnion` (by design, upstream too). The probe and the C++ rows
  check all of them.
- **Missing upstream operation (FR-020, a follow-up):** `S2PointRegion::Encode`/`Decode` (a
  25-byte codec whose decoder rejects a point that is not unit length).
- **Records:** `verify/values/INVENTORY.md` §13, `PROPERTIES.md` §12, the nine rows of
  `verify/cell_union/INVENTORY.md` and the spec-011 entries of `cell_union/PROPERTIES.md`, the
  spec-011 part of `verify/values/EVALUATION.md`, `TRUSTED_BASE.md`, `MUTANTS.md` (`M-950` …
  `M-973`), `KANI_ISSUES.md` (KI-013).

### 2.13 `Cell`'s bounding laws, and the region adaptors (spec 012)

- **Scope:** `Cell`'s bounding-cap and bounding-rectangle laws for its own points, and its
  covering law; `RegionUnion` and `RegionIntersection` (18 entries) and their `Region` laws.
- **Found (U-017):** `Cell::contains_point` accepts points up to `(5/3)·ε` outside the cell in u-v
  (upstream's, so a point's leaf cell contains it), and the cap and rectangle bounds cover the cell
  alone, upstream too: 118,093 and 36,315 of 1,008,724 contained adversarial points lie outside
  them, all in that margin. At the maintainer's choice the bounds stay upstream's and are
  documented; the laws are stated for the cell's own points. `RegionIntersection::region(i)`'s
  panic past the end is documented (K2). New C++ vectors (`cpp_cell_bounds_vectors.csv`, 31,216
  rows) match every row but spec 010's documented cap radii.
- **Proven:** the adaptors' crash freedom (16 entries, over abstract members: any answers, so any
  members), their meaning, and the preservation of three `Region` laws (containment implies
  intersection, the vertex law, the bounding rectangle), for up to 3 members (2 for the vertex
  law) and any number by a written argument; `Cell`'s covering law by composition.
- **Not proven:** `Cell`'s bounding laws for its own points (the cap's harnesses time out; the
  rectangle's need `atan2`'s monotonicity), tested on 240,000 own points; the adaptors'
  bounding-cap and covering laws, which need `Rect`'s and `Cap`'s.
- **New methods.** The abstract region (`verify/src/any_region.rs`): a member that stands for every
  implementor, with laws assumed of its answers.
- **Records:** `verify/values/INVENTORY.md` §14, `PROPERTIES.md` §13 and §1's `P-CREG` rows, the
  spec-012 part of `EVALUATION.md`, `TRUSTED_BASE.md`, `MUTANTS.md` (`M-974` … `M-987`),
  `UPSTREAM.md` (U-017).

## 3. Traits

| Trait | Implementors | With proofs | Without |
|-------|--------------|-------------|---------|
| `S2Encode` / `S2Decode` | 12 | `CellId`, `Point`, `Cap`, `Rect`, `Cell` (003), with `Cap`'s round trip also under its own equality (008); `CellUnion` (002) | `Polyline`, `Loop`, `Polygon`, `LaxPolyline`, `LaxPolygon`, `PointVector` |
| `Region` | 12 | `CellUnion::contains_cell` and `intersects_cell`, within size bounds (002); for `Cell`, the generic laws and `contains_cell`, `intersects_cell` and `cell_union_bound` (004); for `Rect`, `rect_bound` (005), and containment implies intersection and the vertex law under contract, and the point test's `LatLng` (010); for `Cap`, three of the four generic laws (containment implies intersection, no vertex of a non-intersected cell, the bounding cap) with lemma stubs, the crash freedom of five of the six methods (`cell_union_bound`'s, under contract, times out weekly) and `Cell::cap_bound` containing the cell's vertices (008); for `PointRegion`, five of the six generic laws (the vertex law times out) and every method (011); for `RegionUnion` and `RegionIntersection`, every method's crash freedom and the preservation of three laws over any members (012); for `Cell`, its covering law in the closed form (012); for `CellUnion`'s float methods, the point test, both bounds' containment of each cell's bound, the areas, and containment implies intersection (011) | `Cap`'s, `Rect`'s and `CellUnion`'s covering laws (need `asin`'s value); `Rect`'s bounding-cap law (needs `atan2`'s and `sin_cos`'s values; widened against U-016 and tested); `CellUnion`'s bounding-cap and rectangle laws (wait on `Cell`'s); `Cell`'s bounding-cap and rectangle laws for its own points (time out, need `atan2`'s values; its `contains_point` margin is outside them, U-017); the adaptors' bounding-cap and covering laws; and every method of `Loop`, `Polygon`, `Polyline`, `ShapeIndexRegion`, `ShapeIndexBufferedRegion` |
| `Shape` | 13 | none | all (the other branch has pilots, section 6, step 4) |
| `Target` / `DistanceTarget` | 5 | none | `PointTarget`, `EdgeTarget`, `CellTarget`, `CellUnionTarget`, `ShapeIndexTarget` |
| `Projection` | 2 | none | `PlateCarreeProjection`, `MercatorProjection` |
| `SnapFunction` | 3 | none | `IdentitySnapFunction`, `IntLatLngSnapFunction`, `S2CellIdSnapFunction` |
| `Layer` | 15 | none | every builder layer |
| Standard traits on `CellId` | | `Default`, `From<u64>`, `From<CellId> for u64`, `FromStr` | `Display`, `From<&Point>`, `From<&LatLng>` |
| Standard traits on `CellUnion` | | `Deref`, `FromIterator`, both `IntoIterator` | `Display` |
| Standard traits on the value types (004–010) | | `PartialEq` (the equivalence laws, and upstream's meaning where it differs from field equality, as `Cap`'s empty and full caps whatever their centres), `Default`, `From` conversions; `Point`'s `PartialOrd` (a total order without NaN) | `Display` |

The decoding layer states its contract once, generically, over a `Codec` trait in
`verify/src/codec.rs`, and instantiates it per codec. That is the pattern to reuse for the
other trait laws (section 6, step 4).

## 4. Uncovered code

Line counts are non-test lines. "Evidence" lists what checks the code today.

| Group | Main types | Lines | Evidence today | Fit for proof |
|-------|------------|-------|----------------|---------------|
| Value types in `r1`–`r3`, `s1` | `r2::Point`, `Vector`, `PreciseVector`, `ExactFloat`, `Matrix3x3` (`s1::Interval`, `r1::Interval`, `r2::Rect`, `ChordAngle`, `Angle`: sections 2.4–2.7) | 6,600 (7,400 before spec 007 moved its 800) | property tests (`angle`, `chord`, `s1interval`, `r2`, `r3`); Kani pilots on the other branch | good: comparisons, additions and laws prove in seconds; asserting on products of floats does not finish (section 6, step 3) |
| Value types in `s2` | `Metric`, `PaddedCell`, `coords`, earth units, projections, and the floating-point geometry of `Cell` and `Rect` (their other operations, and `Point`: sections 2.4–2.6) | 11,500 | C++ cross-validation (`CELL_AREA`, `CELL_VERTICES`, `LATLNG_*`, `POINT_DISTANCE`, `RECT_OPERATIONS`); property tests | mixed: integer and ordering parts are cheap, geometry needs float bounds |
| Shapes and measures | `Loop`, `Polygon`, `Polyline`, the `Lax*` shapes, `PointVector`, `EdgeVectorShape`, `WrappedShape`, measures, centroids | 16,400 | unit and property tests; C++ boolean-operation vectors; fuzzed decoders | `Shape` laws at small sizes (piloted); `Loop` construction runs out of memory at 3 symbolic vertices |
| Predicates and edges | `predicates`, `edge_crossings`, `EdgeCrosser`, `edge_distances`, the clippers, `EdgeTessellator`, `RobustCellClipper` | 9,200 | C++ cross-validation (crossings, distances); unit tests | needs certified floating-point error bounds, a different method |
| Indexes, queries, regions | `ShapeIndex`, `CellIndex`, `S2PointIndex`, the closest/furthest/crossing/contains queries, `ValidationQuery`, `RegionCoverer`, the region adaptors | 23,700 | integration tests (`shape_index_queries`, `region_covering`) | poor: large state; rely on tests and differential checks |
| Construction | `S2Builder` and its layers and snap functions, `S2BooleanOperation`, buffer, winding, simplifier, alignment | 29,600 | `builder_integration`, `polygon_operations`, C++ boolean-operation differential tests | poor, as above |
| Other encodings and text | shape-index encoding, `EncodedS2ShapeIndex`, compressed points, `S2DensityTree`, lexicons, `text_format` | 6,300 | fuzz targets for the shape-index, compressed-point and density-tree decoders; robustness tests; none for the lexicons | good for crash freedom within byte bounds, built on the proven vectors (step 5) |
| The rest of `encoding.rs` | the `Polyline`, `Loop`, `Polygon`, `Lax*` and `PointVector` codecs | 1,300 | fuzz targets (`decode_polyline`, `decode_loop`, `decode_polygon`, `decode_lax_*`); round-trip tests | good, as above |

## 5. Can the existing proofs be simplified?

Measured on the final per-change run (`verify/target/us2-final`, 3 jobs). Kani spends about
5.3 s setting up each harness beyond its reported verification time, so harness count matters
as much as solver time.

### S1. Run the bounded agreement with the restated codecs weekly, not per change (recommended)

For the five variable-length codecs the library is checked twice on the same inputs: against
`wire_spec` (`cd_uv`, `cd_cv`, `cd_sv`) and against the Verus-restated copies (`cd_agree`).
Per change, the second check adds nothing the first does not, except the link to the Verus
proofs. And any change to a mirrored function already fails the drift guard per change.

- **Change:** move `cd_agree::*` (38 per-change harnesses) to the weekly tier.
- **Saving:** 735 s of verification plus about 200 s of set-up per change. The `links` CI leg
  keeps only `cd_sv`, so the decoding legs go from three to two.
- **Kept:** the link to the Verus proofs, weekly, and the cross-check between the two
  references, which were written independently and both match the C++ vectors.
- **Risk:** low. The bounded link becomes weekly, as the longer agreement members already are.

### S2. One harness per fixed-size codec instead of one per input length (measure first)

The agreement families of the five fixed-size codecs (`cd_fx::p_fx_06` … `p_fx_10`) have one
harness per input length: 113 per change, with 170 s of verification time and about 600 s of
set-up. The per-length split is the rule for the vector codecs, whose loops depend on the length.
The fixed-size decoders have no loop over the input: they read 8 to 33 bytes with `read_exact`
and stop.

- **Change:** one harness per codec over `[u8; L_max + 1]` and a symbolic length `n <= L_max + 1`,
  decoding `&b[..n]`, with the covers (truncated, exact, trailing bytes) stated on `n`.
  The same may apply to `cd_fw`'s per-width families (19 harnesses, one per width of at most
  8 bytes) and to `cd_agree::a_fw` (11).
- **Saving:** for the fixed-size families alone, about 108 harnesses and 9.5 minutes of
  set-up per change (about 3 minutes of wall time at 3 jobs); with `cd_fw` and `a_fw`, about
  135 harnesses and 12 minutes.
- **Risk:** a symbolic slice length might cost more than the sum of the members. Measure one
  codec (`p_fx_09`, 35 members, 63 s) before converting the rest, and keep the per-length
  members if it does.

### S3. Generate the families with a macro, not with scripts (recommended)

The per-length member lines in `cd_uv.rs`, `cd_cv.rs`, `cd_agree.rs` and `cd_fx.rs` were
written by Python scripts in a scratch directory, which a reboot wiped. They are now
hand-maintained: 221 near-identical members in about 700 lines.

- **Change:** express each family with `seq-macro`'s `seq!` (a dependency of the unpublished
  `verify` crate only), with one range per cover variant (for example lengths below the full
  size, the full size, and one byte more), so that each member's covers stay reachable.
- **Saving:** maintenance, not run time.
- **Risk:** low. A cover that a member cannot reach counts as unsatisfied in Kani's summary,
  and `run-verify.sh` fails on it, so the ranges must stay split by cover, never selected
  with an `if` on the length.

### S4. Use the restated decoders as the Kani reference for the vector decoders (optional)

Going further than S1, `wire_spec`'s vector decoders (`lib_uint_vector`, `lib_cellid_vector`,
`lib_string_vector`) could be replaced by the restated copies. Then `cd_agree`'s decoder
families disappear altogether, and the Kani side has one reference fewer to trust.

- **Against:** the restated decoders cost more for Kani than `wire_spec`'s (`a_cv_dec__b3`
  takes 71 s, `p_cv_02__b3` 24 s), so the per-change bounds would shrink. The cross-check
  between two independent references is lost. The encoders cannot follow, because the restated
  encoders append byte by byte, as Verus needs, and run out of memory at two values.
- **Verdict:** only if S1 is not enough, and only after measuring the bounds it would cost.

### S5. Let property harnesses discharge `CellId`'s crash obligations (low priority)

Specs 002 and 003 let a property harness discharge an operation's crash obligation when its
domain contains the crash domain, which removed 50 harnesses in 002. Spec 001 predates that
rule: 45 operations keep a dedicated `crash::c_*` harness.

- **Saving:** small. Apart from `c_edge_neighbors` (weekly), these harnesses take seconds, so
  the gain is about 45 × 5 s of set-up and fewer harnesses to maintain.
- **Risk:** each operation needs its domain checked against the property harness's.
- **Verdict:** do it when 001's suite is next touched.

### Not worth simplifying

- **The weekly and xlarge tiers.** Each member exists because a smaller bound did not cover
  that case (the two-byte varint header, `Cap` at full length, the neighbour proofs).
- **The restated encoders' byte-by-byte appends.** Verus needs them; they are only expensive
  where Kani runs them (S4).
- **The drift guard.** It is cheap, and the only per-change link for the string vector decoder.

## 6. Implementation steps

Ordered by value and dependency. Steps 3–5 are each a feature of the size of specs 001–003 (3–4
days each there) and should start with their own spec (`/speckit-specify`). Steps 0–2 are small.

### Step 0: decisions that are the maintainer's (no code until decided)

1. **CI end to end:** approve a push and draft PR to run the verify workflow once
   (001 T046, 002 T046, 003 T064). It also tells whether the decoding legs stay under
   10 minutes on a hosted runner (003 SC-006).
2. **Upstream reports:** U-001 to U-006 (`UPSTREAM.md`; tasks 001 T054–T055, 002 T054,
   003 T074, T075, T077).
3. **D-001:** whether `s2::from_face_ij` should wrap like C++ for `i` or `j` outside
   `[0, 2^30)`, or keep its documented debug-build panic (001 T056).

### Step 1: finish spec 003

1. Run the xlarge tier on a machine with 32 GB of memory (`make verify-xlarge`, 16 harnesses,
   up to 4 hours each) and `make verify-mutants M=M-222 VERIFY_XLARGE=1`. Promote what passes
   to the weekly tier; record the rest as tool limits.
2. Done (2026-10-11): the test the mutation measurement found missing, that the point vector
   header `80 80` is rejected (`header_varint_cut_short_is_rejected`; `verify/decoding/MUTATION.md`).
   `reconstruct_uvarint` now calls the verified `read_uvarint` instead of copying it.

*Done when* every xlarge member has a recorded result and `M-222` is killed or recorded.

### Step 2: simplify the existing proofs

1. S1: move `cd_agree` to the weekly tier; merge the `links` CI leg into the others.
2. S2: measure one collapsed fixed-size family; convert the rest if it pays.
3. S3: replace the hand-maintained member lists with `seq!` families.
4. Re-run `make verify-decoding`, `make verify-slow F=decoding` and `make verify-mutants` to
   show that every property still has its harnesses and every mutant is still killed.

*Done when* the per-change decoding suite fits two CI legs and all 35 `M-2NN` mutants are
still killed (34 run, `M-222` xlarge).

### Step 3: the value types (a new suite, spec 004)

Port the other branch's pilots (`claude/formal-methods-type-invariance-ept6u0`,
`core/src/*_proofs.rs`, written against v0.3.2) into `verify/`, where this tree keeps its
proofs. Its results show which laws fit:
- `r1::Interval`, `r2::Rect`: constructors, containment, union and intersection laws,
  `expanded` and `project` for finite values (under a second to 41 s each; one 545 s);
- `s1::Interval` (its one-ulp `length` corner stated as an exception), `ChordAngle`, `Angle`
  (without the E5/E6/E7 round trips, which did not finish in 25 minutes);
- `Point` ordering (a total order without NaN, transitive); `coords`' tables and conversions
  (Verus for the Hilbert tables, under a second);
- `Cell` from a valid id (its id, face, level, leaf-ness) and `children()` agreeing with
  `CellId::children()`.

First decide on type-level invariants (private fields with validated constructors for
`ChordAngle`, `s1::Interval`, `Point`): the other branch's first recommendation. It changes the
public API, so it needs its own spec, but it turns several laws into type properties.

*Done when* each type has an inventory, a catalogue against a model, mutants, and a CI leg.

*Status (2026-10-14):* done for six types by specs 004–006 (sections 2.4–2.6), without the
type-level invariants: the evaluations of specs 005 and 006 rank them after the value types, a
breaking change for the maintainer. *(2026-10-18):* spec 007 added `ChordAngle` (after the
maintainer chose option B, which makes it the first value type to keep its own invariant) and
`Angle` (section 2.7). `coords` has no spec yet; `LatLng` and `Cap` are ranked in spec 007's
evaluation. *(2026-10-25):* spec 008 added `Cap` (section 2.9); its evaluation ranks `LatLng`
next as a value type, now with an `atan2` contract. *(2026-10-28):* spec 010 added `LatLng` and
`Rect`'s `Region` laws (section 2.11).

### Step 4: trait laws

State each law once, generically, as 003's `Codec` does, and instantiate it per implementor.
1. **`Shape`:** chains partition the edges; `chain_position` inverts `chain_edge`;
   dimension-specific adjacency. Piloted for `LaxPolyline`, `Polyline`, `LaxLoop`,
   `PointVector`, `WrappedShape` (up to 4 symbolic vertices, 56–82 s) and `LaxPolygon` at
   concrete loop sizes. `EdgeVectorShape` waits on the `set_dimension` decision (`UPSTREAM.md`,
   inherited behaviours). `Loop` needs concrete sizes: it ran out of memory at 3 symbolic
   vertices.
2. **`Region`:** `contains_cell ⇒ intersects_cell`; `cell_union_bound` covers every contained
   cell; the bounds contain the region's points, for `Cap`, `Cell`, `Rect`, `CellUnion`,
   `PointRegion` and the two region adaptors over them. The integer laws first; laws that
   need float geometry go to step 6.
   *Status (2026-10-25):* `laws::region` holds four laws (spec 008 added "no intersection, no
   contained vertex" and "the bounding cap contains the contained points"). Proven for `Cell`
   (004) and for `Cap` (008: three of four; the covering law needs `asin`'s value). For `Cap`
   the float laws became provable through *lemma stubs*: a function the law computes twice on one
   input (`chord_angle`, the chord-angle sum, `is_unit`) is replaced by a pure function of its
   input's bits whose facts are proven on the real code, so the two calls agree by construction
   (`verify/TRUSTED_BASE.md`). That is the method for laws that recompute float values; it does
   not help bit-for-bit comparisons with an identical restatement. `Rect` is next (spec 008's
   evaluation).
   *Status (2026-10-28):* six laws (spec 011 added the bounding rectangle and the covering
   containing the contained points). Proven for `Rect` (010: two of four) and for `PointRegion`
   (011: five of six; the vertex law times out); for `CellUnion` (011) containment implies intersection, the vertex law is not
   one of its laws, and the bounding laws wait on `Cell`'s own (step 6). Next: `Cell`'s bounding
   laws, then the region adaptors (spec 011's evaluation).
3. **`S2Encode`/`S2Decode`:** the round trip for the six remaining implementors comes with
   step 5.

### Step 5: the remaining decoders (untrusted input)

Crash freedom first, within byte bounds, as spec 003 did; then the contract.
1. The geometry codecs in `encoding.rs`: `Polyline`, `Loop`, `Polygon`, `LaxPolyline`,
   `LaxPolygon`, `PointVector`. They are built on the proven vectors, so their proofs can
   assume the vector contracts (composition) rather than re-prove them.
2. Compressed points (`point_compression.rs`); the other branch piloted the bit interleaving.
3. The shape-index encoding and `EncodedS2ShapeIndex`; `S2DensityTree`; the lexicons.
4. The compact point-vector branch, out of Kani's reach from 2 bytes: a Verus restated copy,
   as for the string vector.

*Done when* every `S2Decode` implementor has a crash-freedom result and the six round trips of
step 4.3 are proven within bounds. Each needs C++ vectors for the reference first.

### Step 6: floating-point geometry (research first)

The predicates (`predicates.rs`: `sign`, `robust_sign`), edge crossings and distances decide
the correctness of everything above them, and need a different method: certified
floating-point error bounds (the other branch, §3.3). Start with a feasibility study on
`robust_sign`'s fast path, before any spec.

### Not planned

`S2Builder` and its layers, `S2BooleanOperation`, the indexes and queries, and
`RegionCoverer`: about 53,000 lines with large state, out of reach of both tools at useful
sizes. Their evidence stays the C++ differential tests, the integration tests and fuzzing.
Mutation testing (as in `verify/decoding/MUTATION.md`) would measure how strong that evidence
is, and is the cheapest next check for these modules.
