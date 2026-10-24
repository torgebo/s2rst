# Verified types and traits

Which types and traits of the `s2rst` crate (`core/`) have proofs, what those proofs cover,
what has none, whether the existing proofs can be made simpler, and the remaining work as
implementation steps.

State on branch `003-next-specs/1` at commit `537e7fd` (2026-10-07), with the six value types
of specs 004–006 added on 2026-10-14 (sections 2.4–2.6) and spec 007's two on 2026-10-18
(section 2.7; the counts in section 1 and the tables of sections 3 and 4 include them). Sources: the inventories (`verify/INVENTORY.md`,
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
| Code the proofs exercise | about 6,500 lines (5.7 %): `cell_id.rs`, `cell_union.rs`, the four vector codec files, and the codec impls in `encoding.rs`; and the value types' files, about 9,400 lines with their tests' files left out (`s2/cell.rs`, `s1/interval.rs`, `r1/interval.rs`, `s2/rect.rs`, `r2/rect.rs`, `s2/point.rs`, `s1/chord_angle.rs`, `s1/angle.rs`). Not every line of those files is proven (section 2) |
| Types with their own proof suite | `CellId` (spec 001), `CellUnion` (spec 002), the decoding layer (spec 003), `Cell` and `s1::Interval` (spec 004, section 2.4), `r1::Interval` and `s2::Rect` (spec 005, section 2.5), `r2::Rect` and `s2::Point` (spec 006, section 2.6), `s1::ChordAngle` and `s1::Angle` (spec 007, section 2.7) |
| Types reached only through other proofs | `Level`, `Face`, `Cap` (section 2.8) |
| Traits with proofs | `S2Encode`/`S2Decode` for 6 of 12 implementors; `Region` for 2 methods of 1 implementor (`CellUnion`) and the generic laws for `Cell`; the equality laws for the eight value types, and the order laws for `Point`, `ChordAngle` and `Angle` |
| Kani harnesses | 557 per change (`CellId` 196, `CellUnion` 120, decoding 241) and 221 for the value types (spec 007 adds 36); 173 more weekly (91, 45, 37) and 75 for the value types (spec 007 adds 10); 16 xlarge (decoding, never run). Spec 007 also proves 4 statements by enumerating every `i32` natively (`verify/tests/angle_enumeration.rs`) |
| Verus | 5 restated codecs, 132 obligations, 17 s |
| Mutants | 38 `M-0NN`, 25 `M-1NN`, 35 `M-2NN`, 10 `V-NNN`; all killed except `M-222`, whose target is xlarge. For the value types 45 `M-3NN`, 43 `M-4NN`, 40 `M-5NN`: all killed except `M-344`, `M-345` and `M-539`, whose targets time out (no verdict); 38 `M-6NN` for spec 007 (campaign pending) |

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
  limits); `expand_at_level` (it calls `CellId::all_neighbors`); the floating-point operations
  `contains_point`, `expand_by_radius`, `average_based_area`, `approx_area`, `exact_area`;
  `Display`.
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
  three statements that time out at 60 minutes, also on the points whose largest coordinate is
  at least ½ (unit length, `point_cross`'s sign laws, the chord angle's symmetry). The C++ vectors and property tests check each.
- **Records:** `verify/values/INVENTORY.md` §5–6, `PROPERTIES.md` §6–7, the spec-006 part of
  `EVALUATION.md`, `MUTANTS.md` (`M-501` … `M-540`).

### 2.7 `s1::ChordAngle` and `s1::Angle` (spec 007)

- **Scope:** `ChordAngle` 35 of 36 entries (28 with crash proofs; the 7 through `asin`,
  `to_angle` and what reads it, are tool limits; `Display` excluded); `Angle` 30 of 31 (29
  with crash proofs; `tan` a tool limit; `Display` excluded).
- **Changed first (option B, FR-017 to FR-023):** every chord angle is canonical (−1, a length
  in [0, 4], or +∞), so the type keeps its own invariant, a first among the value types; NaN gives
  `STRAIGHT` as upstream; the arithmetic checks special operands in debug builds; deserialisation
  rejects non-canonical values; `normalized` is the exact remainder; the E-forms round ties to
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
- **Not proven:** `normalized`'s value (Kani's `%` on `f64` is wrong); `ChordAngle`'s `sin2`
  and `sin` and the angle's quotients (timeouts); everything through `asin` and `Angle::tan`.
  The C++ vectors (bit for bit) and an exact big-integer property test check them.
- **Records:** `verify/values/INVENTORY.md` §7–8, `PROPERTIES.md` §8–9, the spec-007 part of
  `EVALUATION.md`, `MUTANTS.md` (`M-601` … `M-648`, all 39 killed).

### 2.8 Types reached only through those proofs

| Type | What the proofs touch | What has no proof |
|------|-----------------------|-------------------|
| `Level`, `Face` (`coords.rs`) | built and read by every `CellId` harness; the Hilbert tables as `CellId`'s traversal uses them | the conversions as functions of their own (`st_to_ij`, `si_ti_to_st`), and the tables' own laws |
| `Cap` | its codec and validity (`Cap::is_valid` is trusted, research R18) | containment, union, bounds, its `Region` impl |

## 3. Traits

| Trait | Implementors | With proofs | Without |
|-------|--------------|-------------|---------|
| `S2Encode` / `S2Decode` | 12 | `CellId`, `Point`, `Cap`, `Rect`, `Cell` (003); `CellUnion` (002) | `Polyline`, `Loop`, `Polygon`, `LaxPolyline`, `LaxPolygon`, `PointVector` |
| `Region` | 12 | `CellUnion::contains_cell` and `intersects_cell`, within size bounds (002); for `Cell`, the generic laws and `contains_cell`, `intersects_cell` and `cell_union_bound` (004); for `Rect`, `rect_bound` (005) | `CellUnion`'s float methods (`cap_bound`, `rect_bound`, `cell_union_bound`, `contains_point`); `Cell`'s `cap_bound`, `rect_bound` and `contains_point`; `Rect`'s other five; and every method of `Cap`, `Loop`, `Polygon`, `Polyline`, `PointRegion`, `RegionUnion`, `RegionIntersection`, `ShapeIndexRegion`, `ShapeIndexBufferedRegion` |
| `Shape` | 13 | none | all (the other branch has pilots, section 6, step 4) |
| `Target` / `DistanceTarget` | 5 | none | `PointTarget`, `EdgeTarget`, `CellTarget`, `CellUnionTarget`, `ShapeIndexTarget` |
| `Projection` | 2 | none | `PlateCarreeProjection`, `MercatorProjection` |
| `SnapFunction` | 3 | none | `IdentitySnapFunction`, `IntLatLngSnapFunction`, `S2CellIdSnapFunction` |
| `Layer` | 15 | none | every builder layer |
| Standard traits on `CellId` | | `Default`, `From<u64>`, `From<CellId> for u64`, `FromStr` | `Display`, `From<&Point>`, `From<&LatLng>` |
| Standard traits on `CellUnion` | | `Deref`, `FromIterator`, both `IntoIterator` | `Display` |
| Standard traits on the value types (004–006) | | `PartialEq` (the equivalence laws, and upstream's meaning where it differs from field equality), `Default`, `From` conversions; `Point`'s `PartialOrd` (a total order without NaN) | `Display` |

The decoding layer states its contract once, generically, over a `Codec` trait in
`verify/src/codec.rs`, and instantiates it per codec. That is the pattern to reuse for the
other trait laws (section 6, step 4).

## 4. Uncovered code

Line counts are non-test lines. "Evidence" lists what checks the code today.

| Group | Main types | Lines | Evidence today | Fit for proof |
|-------|------------|-------|----------------|---------------|
| Value types in `r1`–`r3`, `s1` | `r2::Point`, `Vector`, `PreciseVector`, `ExactFloat`, `Matrix3x3` (`s1::Interval`, `r1::Interval`, `r2::Rect`, `ChordAngle`, `Angle`: sections 2.4–2.7) | 6,600 (7,400 before spec 007 moved its 800) | property tests (`angle`, `chord`, `s1interval`, `r2`, `r3`); Kani pilots on the other branch | good: comparisons, additions and laws prove in seconds; asserting on products of floats does not finish (section 6, step 3) |
| Value types in `s2` | `LatLng`, `Cap`, `Metric`, `PaddedCell`, `coords`, earth units, projections, and the floating-point geometry of `Cell` and `Rect` (their other operations, and `Point`: sections 2.4–2.6) | 11,500 | C++ cross-validation (`CELL_AREA`, `CELL_VERTICES`, `LATLNG_*`, `POINT_DISTANCE`, `RECT_OPERATIONS`); property tests | mixed: integer and ordering parts are cheap, geometry needs float bounds |
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
evaluation.

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
