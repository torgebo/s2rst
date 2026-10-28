# refgen-cpp — C++ reference-vector generators for differential tests

Generates [`core/tests/data/cpp_cellid_vectors.csv`](../../core/tests/data/cpp_cellid_vectors.csv),
which [`core/tests/cpp_cellid_diff.rs`](../../core/tests/cpp_cellid_diff.rs) diffs
the Rust `CellId` implementation against. It uses the **C++ reference**
[google/s2geometry](https://github.com/google/s2geometry) as the oracle, so the
tests catch *wrong answers* (a mis-stepped Hilbert curve, a bad neighbour) by
comparison against the canonical implementation.

The committed CSV is the durable artifact — CI runs the differential test with no
C++ toolchain involved. You only need this generator to extend or regenerate it.

## Build the C++ reference (once)

s2geometry needs abseil; the easiest path fetches and builds it:

```sh
cd /path/to/s2geometry
cmake -B build -DCMAKE_BUILD_TYPE=Release -DFETCH_ABSEIL=ON \
      -DBUILD_TESTS=OFF -DBUILD_EXAMPLES=OFF -DWITH_PYTHON=OFF \
      -DS2_ENABLE_INSTALL=ON -DCMAKE_INSTALL_PREFIX=/tmp/s2-install
cmake --build build -j"$(nproc)"
cmake --install build
```

## Build and run the generator

```sh
cd tools/refgen-cpp
cmake -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install
cmake --build build
./build/generate_cellid > ../../core/tests/data/cpp_cellid_vectors.csv
```

Output is deterministic (fixed RNG seed). After changing the generator, regenerate
and run `cargo test -p s2rst --test cpp_cellid_diff`.

## Sections emitted

| Section                  | Rust method(s) checked              | Oracle (S2CellId)       |
|--------------------------|-------------------------------------|-------------------------|
| `CELLID_EDGE_NEIGHBORS`  | `edge_neighbors`                    | `GetEdgeNeighbors`      |
| `CELLID_RANGE`           | `range_min`, `range_max`            | `range_min`, `range_max`|
| `CELLID_CONTAINS`        | `contains`, `intersects`            | `contains`, `intersects`|
| `CELLID_CHILDREN`        | `children`                          | `child`                 |
| `CELLID_FACE_IJ`         | `to_face_ij_orientation`            | `ToFaceIJOrientation`   |
| `CELLID_FROM_TOKEN`      | `from_token` (hex-encoded input)    | `FromToken`             |
| `CELLID_FROM_DEBUG_STRING` | `from_debug_string` (hex-encoded input) | `FromDebugString`   |
| `CELLID_TO_STRING`       | `to_debug_string`, `Display`        | `ToString`              |

All values are exact `u64` cell ids, compared exactly. Text inputs are written as the hex
of their bytes, so NUL, commas, spaces and multi-byte UTF-8 survive the CSV.

New sections are appended after the existing ones: the generator shares one seeded RNG
across sections, so appending leaves every earlier line byte-identical. Check that with
`git diff` after regenerating (only `+` lines).

## CellUnion oracle (`generate_cellunion`)

`generate_cellunion` writes
[`core/tests/data/cpp_cellunion_vectors.csv`](../../core/tests/data/cpp_cellunion_vectors.csv)
for spec 002 (`specs/002-formal-verify-cell-union`, data-model §10). Two tests read it:
[`core/tests/cpp_cellunion_diff.rs`](../../core/tests/cpp_cellunion_diff.rs) diffs the
Rust `CellUnion` against it, and `verify/tests/union_spec_vs_cpp.rs` checks the
verification model against it.

```sh
cmake --build build --target generate_cellunion
./build/generate_cellunion > ../../core/tests/data/cpp_cellunion_vectors.csv
```

It has its own seeded RNG, so it can be extended without touching the `CellId` vectors.
Ids are 16 lowercase hex digits, a list of ids is `;`-separated (an empty field is the
empty list), bytes are lowercase hex, and booleans are `0`/`1`.

| Section                     | Row                                                              | Rust method(s) checked | Oracle (S2CellUnion) |
|-----------------------------|------------------------------------------------------------------|------------------------|----------------------|
| `CELLUNION_NORMALIZE`       | `input,output`                                                   | `from_cell_ids` | `S2CellUnion(ids)` |
| `CELLUNION_DENORMALIZE`     | `input,min_level,level_mod,output`                               | `denormalize` (FR-015) | `Denormalize` |
| `CELLUNION_SETOPS`          | `a,b,union,intersection,difference,contains,intersects`          | `union`, `intersection`, `difference`, `contains_union`, `intersects_union` | `Union`, `Intersection`, `Difference`, `Contains`, `Intersects` |
| `CELLUNION_FROM_BEGIN_END`  | `begin,end,output`                                               | `from_begin_end`, `from_min_max`, `from_range` | `FromBeginEnd` |
| `CELLUNION_LEAF_CELLS`      | `ids,count`                                                      | `leaf_cells_covered` | `LeafCellsCovered` |
| `CELLUNION_ENCODE`          | `ids,bytes`                                                      | `S2Encode::encode` | `Encode` |
| `CELLUNION_DECODE`          | `bytes,ok,valid,ids`                                             | `S2Decode::decode` (FR-017) | `Decode`, then `IsValid` |

In `CELLUNION_DECODE`, `ok` is what C++ `Decode` returned and `valid` is `IsValid()` of the
decoded union. C++ validates nothing, so rows with `ok = 1, valid = 0` exist (unsorted,
duplicate, overlapping or invalid ids): that is upstream finding U-003. The Rust decoder
rejects them, as its rustdoc documents.

## Decoding-layer oracle (`generate_coding`)

`generate_coding` writes
[`core/tests/data/cpp_coding_vectors.csv`](../../core/tests/data/cpp_coding_vectors.csv)
for spec 003 (`specs/003-formal-verify-decoders`, FR-024, data-model §10). Two tests read
it: `core/tests/cpp_coding_diff.rs` diffs the Rust codecs against it, and
`verify/tests/wire_spec_vs_cpp.rs` checks the reference description against it.

```sh
cmake --build build --target generate_coding
LD_LIBRARY_PATH=/tmp/s2-install/lib ./build/generate_coding > ../../core/tests/data/cpp_coding_vectors.csv
```

Every row is `kind,input,expected,note`. Bytes are lowercase hex. Lists are `;`-separated. A
point is `x:y:z`, each coordinate the 16-hex-digit bit pattern of the double. A cap adds
`:length2`, and a rectangle is `lat_lo:lat_hi:lng_lo:lng_hi`. A decode's `expected` is
`reject`, or `ok|<bytes consumed>|<item count>|<items>`. `note` is `deviation:reject` where
the library documents that it rejects an input C++ accepts (spec 003 FR-022), and
`deviation:negative-canonical` on a `CAP_DECODE` row whose negative `length2` (other than −1)
C++ keeps and the library decodes as −1 (spec 007 FR-017).

| Section | Input | Expected | Oracle |
|---------|-------|----------|--------|
| `VARINT_DECODE` | canonical, over-long, ten-byte and truncated varints | value, or reject | `Decoder::get_varint64` |
| `UINT32_ENCODE`, `UINT64_ENCODE` | value lists of every width, empty, all-zero, maximal | bytes | `s2coding::EncodeUintVector<T>` |
| `UINT32_DECODE`, `UINT64_DECODE` | every encoding and every prefix of it; the K2 header; overflowing counts | values, or reject | `EncodedUintVector<T>::Init` + `Decode` |
| `CELLID_VECTOR_ENCODE` / `_DECODE` | id lists at every level, mixed, raw bit patterns; malformed headers | bytes; ids or reject | `EncodeS2CellIdVector`; `EncodedS2CellIdVector` |
| `STRING_VECTOR_ENCODE` / `_DECODE` | string lists (the encode input is the count, a bar, then the items); corrupt offsets (`[5, 2, 6]`); the K2 input | bytes; strings or reject | `StringVectorEncoder::Encode`; `EncodedStringVector` (v0.14's lazy rule) |
| `POINT_VECTOR_ENCODE` / `_DECODE` | FAST point lists, NaN and non-unit included; tags 2–7; the K2 header | bytes; points or reject | `EncodeS2PointVector(…, FAST)`; `EncodedS2PointVector` |
| `CELLID_*`, `POINT_*`, `CAP_*`, `RECT_*`, `CELL_*` | fixed-size values and every prefix of their encodings; `Rect` versions 0 and 2 | bytes; value or reject | `S2CellId`, `S2Cap`, `S2LatLngRect`, `S2Cell` `Encode`/`Decode`; three `putdouble`s for a point |

Left out on purpose: `S2Cell::Decode` of an invalid id (undefined behaviour in C++,
candidate upstream finding U-004), and vectors above the library's count limits (rows would
be tens of megabytes). The generator's header comment says why.

## Boolean-op oracle (`generate_boolean_op`)

A second oracle, for the `graph_edge_clipper` boolean-op bug (see `BUG.md` §2).
The 13 near-degenerate regression inputs are lifted verbatim from the Rust tests
and run through the C++ reference:

```sh
# 1. Extract the inputs from the Rust tests (single source of truth).
python3 tools/refgen-cpp/extract_boolean_op_inputs.py
#    -> core/tests/data/boolean_op_inputs.txt

# 2. Build + run the C++ oracle (needs /tmp/s2-install as above).
cmake --build build --target generate_boolean_op
LD_LIBRARY_PATH=/tmp/s2-install/lib \
  ./build/generate_boolean_op ../../core/tests/data/boolean_op_inputs.txt \
  > ../../core/tests/data/cpp_boolean_op_vectors.txt
```

`core/tests/cpp_boolean_op_diff.rs` then replays the inputs through the Rust
pipeline and diffs against the committed vectors (no C++ toolchain in CI).
Upstream returns a non-empty union for every case; the Rust port currently
matches only some — the remaining gap is the bug tracked in `BUG.md` §2.

Pinned oracle version: `google/s2geometry` `v0.14.0-39-g8d5c2a8`.

## Value-type oracle (`generate_values`)

`generate_values` writes
[`core/tests/data/cpp_values_vectors.csv`](../../core/tests/data/cpp_values_vectors.csv)
for spec 004 (`specs/004-formal-verify-cell-interval`, FR-021, research R10) and spec 005
(`specs/005-formal-verify-r1-interval-rect`, FR-021, research R9). Two tests read it:
`core/tests/cpp_values_diff.rs` diffs the Rust `s1::Interval`, `Cell`, `r1::Interval` and
`s2::Rect` against it, and `verify/tests/values_spec_vs_cpp.rs` checks the reference
descriptions (`arc_spec`, `cell_spec`, `real_spec`, `rect_spec`) against it. Spec 005's
sections come after spec 004's, from the same random sequence, so regenerating leaves every
earlier line byte-identical.

```sh
cmake --build build --target generate_values
LD_LIBRARY_PATH=/tmp/s2-install/lib ./build/generate_values > ../../core/tests/data/cpp_values_vectors.csv
```

Every double is the 16-hex-digit bit pattern of its value, a boolean is `0`/`1`, and `-`
marks a result that is not computed because the input lies outside the operation's domain
(`Project` on an empty interval, or an invalid latitude–longitude where upstream only logs
it). Cell ids are decimal. The generator's header comment lists the results of every section
in order.

| Section | Row | Rust method(s) checked | Oracle |
|---------|-----|------------------------|--------|
| `S1INTERVAL_OPS` | `x_lo,x_hi,p,margin,max_error,y_lo,y_hi,results,note`, 5,000 random rows | every in-scope `s1::Interval` operation except `Display` | `S1Interval` |
| `S1INTERVAL_SPECIAL` | the same, for every pair of upstream's test fixtures, the K4 corner and its neighbours, and D-005's inputs, with special points, margins (0, ±π, ±2π, ±3.2, ±the smallest subnormal, ±∞) and error bounds; then four intervals shrunk to almost nothing, where the bounds cross (U-011) | as above | as above |
| `CELL_FIELDS` | `id,face,level,orientation,u_lo,u_hi,v_lo,v_hi,edge0..edge3,size_ij,is_leaf`, 5,000 random cells at every level | `Cell::from_cell_id`, the accessors, `bound_uv`, `uv_coord_of_edge`, `size_ij`, `is_leaf` | `S2Cell(id)`, `GetBoundUV`, `GetUVCoordOfEdge`, `GetSizeIJ` |
| `CELL_CHILDREN` | `parent_id,k,child_id,child_orientation,u_lo,u_hi,v_lo,v_hi`, 250 parents | `Cell::children` | `S2Cell::Subdivide` |
| `R1INTERVAL_OPS` | `x_lo,x_hi,p,margin,max_error,y_lo,y_hi,results,note`, 3,000 random rows (every binade, signed zeros, rare infinities, many empty intervals) | every in-scope `r1::Interval` operation except `Display` (23 results) | `R1Interval` |
| `R1INTERVAL_SPECIAL` | the same, for every pair of upstream's fixtures, signed zeros, bounds near `DBL_MAX`, every pair of bounds from {−∞, −1, −0, 0, 1, ∞}, and `[±5, ∞]` | as above | as above |
| `RECT_OPS` | `a_lat_lo,a_lat_hi,a_lng_lo,a_lng_hi,ll_lat,ll_lng,m_lat,m_lng,b_lat_lo,b_lat_hi,b_lng_lo,b_lng_hi,results,note`, 2,000 random valid rectangles, a latitude–longitude (invalid in 10% of rows) and a margin | every in-scope `s2::Rect` operation except `Display` (49 results) | `S2LatLngRect` |
| `RECT_SPECIAL` | the same, for every pair of: the empty and full rectangles, a band and a point at each pole, rectangles across the antimeridian, full latitudes at ±π, a latitude point with full longitudes, and the seam corner as a longitude range | as above | as above |
| `R2RECT_OPS` (spec 006) | `a_x_lo,a_x_hi,a_y_lo,a_y_hi,p_x,p_y,m_x,m_y,max_error,b_x_lo,b_x_hi,b_y_lo,b_y_hi,results,note`, 800 random valid rectangles (half through two points; signed zeros, small integers, every binade below 2¹⁰²²), a point, a margin per axis and an error bound | every in-scope `r2::Rect` operation except `Display` (53 results): `is_valid`, `is_empty`, `vertex`, `center`, `size`, `contains_point`, `interior_contains_point`, `add_point`, `project`, `expanded`, `expanded_by_margin`, the four relations, `add_rect`, `union`, `intersection`, `approx_eq_with`, `approx_eq`, `==`, `from_center_size` | `R2Rect` |
| `R2RECT_SPECIAL` (spec 006) | the same, for: the empty rectangle's representations, signed-zero sides, upstream's fixtures, bounds near `DBL_MAX`, every interval from two bounds of {−∞, −1, −0, 0, 1, ∞} as an x side and a y side, each against itself and two others; then every cell's u-v bound at levels 0–3 against itself, its parent's and the next cell's; margins 0, ±1, ±∞ | as above | as above |
| `S2POINT_OPS` (spec 006) | `a_x,a_y,a_z,b_x,b_y,b_z,angle,results,note`, 1,500 random unit points, each with a partner that is in half the rows nearly or exactly equal or antipodal, and an angle | `PartialOrd` and `cmp_point`, `==`, `is_unit`, `normalize`, `s2::ortho`, `edge_crossings::robust_cross_prod`, `distance`, `chord_angle`, `s2::rotate` (18 results) | `operator<`, `S2::IsUnitLength`, `Normalize`, `S2::Ortho`, `S2::RobustCrossProd`, `S1Angle(a, b)`, `S1ChordAngle(a, b)`, `S2::Rotate` |
| `S2POINT_SPECIAL` (spec 006) | the same, for: the signed zero vectors, the axes, ties in the largest component, subnormal, tiny (2⁻²⁴² and below) and huge coordinates, `S2::Origin()`, each against itself, its antipode, the next fixture and a random unit point; equal and antipodal unit points; NaN and ±∞ in each coordinate (results 0–6 only); and `RobustCrossProd(−v, v)` for `v = (2⁻ᵏ, 0, 1)`, k = 300, 1022, 1023, 1024, 1074 | as above | as above |
| `CHORD_ANGLE` (spec 007) | `kind,l,m,e,results,note`, 900 random lengths in [0, 4] (uniform, and squared chords of random unit points) with an error bound; then every length from {−1, −5, −0.001, −∞, −0, 0, the smallest subnormal, 2, 4, the next double above 4, 5, +∞, NaN} against {0, 1, 4, −1, +∞} and a random length, and the `Infinity()` sentinel (`kind` `I`) against the same | `ChordAngle::from_length2`, the five classifications, `successor`, `predecessor`, `plus_error`, the two error bounds, `sin2`, `cos`, `sin`, `tan`, `to_angle`, `e5`–`e7`, `Add`, `Sub`, `<`, `==`, `from_angle` (24 results) | `S1ChordAngle` |
| `ANGLE` (spec 007) | `r,s,n,results,note`, 900 random angles (magnitudes 10⁻³⁰⁰ to 10³⁰⁰, both signs, a third within a turn) with a second angle and an `int32`; then multiples of π, 7, 10⁶, 10¹⁶, 10²⁰, 10³⁰⁰, −10²⁰, −2.462748394469102·10¹⁶, ±0, ±∞, NaN, ±10¹⁰ and the survey's exact E6/E7 ties, each against {0, 1, −0, +∞, π} and the `int32` extremes | `Angle::from_degrees`, `degrees`, `e5`–`e7`, `normalized`, `abs`, `sin`, `cos`, `tan`, `Add`, `Sub`, `Mul<f64>`, `Div<f64>`, `Div<Angle>`, `from_e5`–`from_e7`, `from_unsigned_e6`, `from_unsigned_e7` (21 results) | `S1Angle` |

`note` is `deviation:seam-length` when either interval is the one-ulp interval across the
seam, `[π, nextafter(−π, 0)]`. There upstream's `GetLength` returns −1 (`UPSTREAM.md`
U-007), while the library returns the true length (spec 004 FR-020). The results computed
from that length (the length, `Expanded`, `Intersection` and `ApproxEquals`) are the
documented deviation on those rows, and every other result still matches C++. The same holds
for a rectangle whose longitudes are the corner (its size, `Expanded`, `Intersection` and
`ApproxEquals`).

On the `S1INTERVAL_*` rows, `note` also lists `deviation:expanded-crossing`, joined with `+`,
when the interval is not the corner and upstream's `Expanded` breaks containment: shrinking
gives an interval the original does not contain, or growing one that does not contain the
original. Rounding has crossed the
bounds there (`UPSTREAM.md` U-011), and the library returns the empty or full interval
instead (spec 004 FR-023). Only the four fixed rows at the end of `S1INTERVAL_SPECIAL` have it.

On the `CHORD_ANGLE` rows, `note` lists `deviation:negative-canonical` when `l` is negative and
not −1: C++ keeps it, and the library's `from_length2` returns `NEGATIVE` (spec 007 FR-017,
option B), so the results that follow from the value are the library's for −1. On both spec 007
sections, `deviation:eform-saturate` marks a row whose E5, E6 or E7 is outside the `int32`
range: C++ (x86-64, Release) returns −2147483648, and the library saturates (spec 007 FR-023,
`UPSTREAM.md` U-014). Exact E-form ties are not deviations: the library rounds them to even, as
C++'s `cvtsd2si` does (FR-022).

On the `R1INTERVAL_*` rows, `note` lists the library's other documented deviations, joined
with `+` (spec 005 FR-017):

- `deviation:center-overflow`: upstream's `GetCenter` computes `0.5 * (lo + hi)`, which
  overflows for finite bounds beyond `DBL_MAX / 2` (`UPSTREAM.md` U-010). The library's
  `f64::midpoint` halves first and returns the true centre.
- `deviation:infinite-corner`: at infinite bounds, upstream's `GetDirectedHausdorffDistance`
  keeps the NaN of `∞ − ∞` in its inner `std::max` and answers 0, where the library's
  `f64::max` drops it and answers the true distance (`UPSTREAM.md` U-009).

On the `R2RECT_*` rows (spec 006), `note` lists, joined with `+`:

- `deviation:center-overflow`: as on the `R1INTERVAL_*` rows, for either axis' centre.
- `deviation:infinite-corner`: an input is infinite and a centre, size, expansion, approximate
  equality or `FromCenterSize` result is NaN (`∞ − ∞`). The library computes the same
  arithmetic and documents these corners (spec 006 FR-018); the rows mark where a test
  compares NaN with NaN.

On the `S2POINT_*` rows (spec 006), `note` lists, joined with `+`:

- `deviation:robust-cross-overflow`: C++'s `RobustCrossProd` is not finite, because
  `EnsureNormalizable`'s factor overflows for a largest component below 2⁻¹⁰²³
  (`UPSTREAM.md` U-012). The library's result is finite, non-zero and normalizable (spec 006
  FR-021).
- `deviation:robust-cross-long-double`: C++ answered from its `long double` step, which
  returns `(a − b) × (a + b)` rounded in 80 bits, twice the cross product. The library has no
  `long double` step: its exact step returns `a × b` itself, the same direction at half the
  length.
- `deviation:normalize-zero-sign`: `a`'s squared length is zero, so C++'s `Normalize`
  multiplies by zero and keeps each coordinate's sign, where the library returns
  `(+0, +0, +0)`, as Go does (spec 006 K12). `Normalize`'s result and, where its cross product
  is zero, `Ortho`'s differ in the zeros' signs.

## Cap oracle (`generate_cap`)

`generate_cap` writes
[`core/tests/data/cpp_cap_vectors.csv`](../../core/tests/data/cpp_cap_vectors.csv), which
[`core/tests/cpp_cap_diff.rs`](../../core/tests/cpp_cap_diff.rs) diffs `s2::Cap` against and
[`verify/tests/values_spec_vs_cpp.rs`](../../verify/tests/values_spec_vs_cpp.rs) checks
`verify/src/cap_spec.rs` against (spec 008, FR-024, research R9). It is a file of its own
because `cpp_values_vectors.csv` is already 9.6 MB.

```sh
cmake --build build --target generate_cap
LD_LIBRARY_PATH=/tmp/s2-install/lib ./build/generate_cap > ../../core/tests/data/cpp_cap_vectors.csv
```

The reference build needs no rebuilding from source if `/home/t/sources/s2geometry/build`
exists: `cmake --install /home/t/sources/s2geometry/build --prefix /tmp/s2-install` installs it.

| Section | Rows | Library operations checked | Oracle (`S2Cap`) |
|---------|------|----------------------------|------------------|
| `CAP_OPS` | 1,000 random caps, a point and a second cap each | the classifications, `height`, `area`, `centroid`, `contains_point`, `interior_contains_point`, `contains`, `intersects`, `interior_intersects`, `complement`, `expanded`, `union`, `add_point`, `add_cap`, `approx_eq`, `approx_eq_with`, `==`, `from_center_height`, `from_center_area`, `from_center_angle`, `angle_radius` | the same, by name |
| `CAP_SPECIAL` | 89: empty, zero, full and hemisphere caps with identical and antipodal partners; K4's radii and the constructors' clamps; K6's pair; the poles and the antimeridian | as `CAP_OPS` | as `CAP_OPS` |
| `CAP_BOUNDS` | 836: 600 random caps, the radii next to each covering level's power of two (K1), K5's cap, the poles, the antimeridian, empty and full | `rect_bound`, `cell_union_bound`, `RegionCoverer` with 8 cells | `GetRectBound`, `GetCellUnionBound`, `S2RegionCoverer` |
| `CAP_CELL` | 1,066: random caps against cells at levels 0–10 within two radii of the centre; K2's grid and K3's cap against face 0 | `Region::contains_cell`, `Region::intersects_cell` | `Contains(S2Cell)`, `MayIntersect` |

`Union` differs from the library by design: the library adds an error margin so that the union
contains both caps (`cap.rs`, a documented deviation); the diff test checks containment there.

## LatLng and rectangle-region oracle (`generate_latlng`)

`generate_latlng` writes
[`core/tests/data/cpp_latlng_vectors.csv`](../../core/tests/data/cpp_latlng_vectors.csv)
(3.7 MB), which [`core/tests/cpp_latlng_diff.rs`](../../core/tests/cpp_latlng_diff.rs) diffs
`s2::LatLng` and `s2::Rect`'s region methods against and
[`verify/tests/values_spec_vs_cpp.rs`](../../verify/tests/values_spec_vs_cpp.rs) checks
`verify/src/latlng_spec.rs`'s restatements against (spec 010, FR-021, research R9).

```sh
cmake --build build --target generate_latlng
LD_LIBRARY_PATH=/tmp/s2-install/lib ./build/generate_latlng > ../../core/tests/data/cpp_latlng_vectors.csv
```

| Section | Rows | Library operations checked | Oracle |
|---------|------|----------------------------|--------|
| `LATLNG_CTOR` | 1,884: random and special doubles, random and edge integers | `from_radians`, `from_degrees`, `from_e5`/`e6`/`e7`, `from_unsigned_e6`/`e7`, `invalid`, `Default` | `S2LatLng::FromRadians`, … |
| `LATLNG_POINT` | 6,014: the special coordinates (±0, subnormals, huge, ±∞, NaN) crossed, random unit points, random points over 600 decades, points near the poles and the antimeridian | `latitude`, `longitude`, `from_point`, `From<Point>` | `Latitude`, `Longitude` |
| `LATLNG_NORM` | 3,032: the survey's special longitudes, the ties ±π to ±7π, out-of-range and non-finite latitudes, random longitudes at six scales | `normalized` | `Normalized` |
| `LATLNG_OPS` | 2,759: random valid pairs, close pairs, the survey's out-of-range pairs, the poles | `to_point`, `get_distance`, `approx_eq`, `approx_eq_with`, `+`, `−`, both scalings, `is_valid`, `==` | the same, by name |
| `LATLNG_STR` | 309 | `to_string_in_degrees` | `ToStringInDegrees` |
| `RECT_REGION` | 10,000: random rectangles and those of spec 010's R0 families, against cells of levels 0–12 | `Region::contains_cell`, `Region::intersects_cell` | `Contains(S2Cell)`, `MayIntersect` |
| `RECT_CAPBOUND` | 6,004: R0's worst cases, the empty and full rectangles, 1,000 rectangles with six points on and next to their boundaries each | `cap_bound`, `contains_point` | `GetCapBound`, `Contains(S2Point)` |

`RECT_CAPBOUND`'s radii are the one deviation: the library widens upstream's cap (spec 010
FR-019, `UPSTREAM.md` U-016); the diff test checks the centre bit for bit and the radius
against upstream's widened by the documented rule, and pins upstream's misses (993 of the 5,552
contained points), where the library's bound has none.
