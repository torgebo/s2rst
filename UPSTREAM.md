# Upstream issues

Undefined or wrong behaviour in the upstream S2 libraries, found while porting and formally
verifying s2rst (specs 001–010 under `specs/`). None has been reported upstream yet; each
numbered entry has a follow-up task that records that decision. Bugs in the verification tools
(Kani and CBMC) are in `KANI_ISSUES.md`.

All line references were re-checked on 2026-10-07 against the pinned upstream sources:
- **C++:** [google/s2geometry](https://github.com/google/s2geometry) at `v0.14.0-40-g37a4501`
  (commit `37a4501d5dae`, 2026-06-10). Links below go to that commit.
- **Go:** [golang/geo](https://github.com/golang/geo) at `fe8a584c1b09` (2026-02-12).

"Observed" means the behaviour was seen in running code: the C++ reference-vector generators
in `tools/refgen-cpp/` (built in Release) or Go. "From the code" means it was read, not run,
because running it is undefined behaviour or needs a debug build.

| Id | Where | What | Severity | s2rst | Follow-up |
|----|-------|------|----------|-------|-----------|
| [U-001](#u-001-s2cellidlevel-on-id-0) | C++ `S2CellId::level()` | undefined behaviour for id `0`; level −1 for some invalid ids | undefined behaviour | panics, documented | 001 T054 |
| [U-002](#u-002-getcommonancestorlevel-on-none-none) | C++ `S2CellId::GetCommonAncestorLevel`, Go `CommonAncestorLevel` | C++: undefined behaviour for (`None`, `None`); Go: answers level 30 | undefined behaviour (C++), wrong result (Go) | panics, documented | 001 T055 |
| [U-003](#u-003-s2celluniondecode-accepts-invalid-unions) | C++ `S2CellUnion::Decode` | accepts unions that break its own `IsValid()` requirement | wrong results downstream | rejects | 002 T054 |
| [U-004](#u-004-s2celldecode-on-unvalidated-ids) | C++ `S2Cell::Decode` | builds a cell from any 8 bytes: undefined behaviour from 8 zero bytes | undefined behaviour from untrusted input | rejects | 003 T074 |
| [U-005](#u-005-getuintwithlength-beyond-sizeoft) | C++ `GetUintWithLength<T>` | a length above `sizeof(T)` returns a wrong value in release builds | minor (no caller passes one) | panics, documented | 003 T075 |
| [U-006](#u-006-s2capdecode-does-not-validate) | C++ `S2Cap::Decode` | accepts invalid caps in release builds, aborts in debug builds | wrong results, or a crash on untrusted input | rejects | 003 T077 |
| [U-007](#u-007-s1intervalgetlength-at-the-seam) | C++ `S1Interval::GetLength` | the one-ulp interval across the seam gets length −1, the empty interval's; `Expanded`, `ApproxEquals`, `Intersection` and `S2LatLngRect::Area` inherit it | wrong results | true length, a documented deviation | 004 T053 |
| [U-008](#u-008-r1intervalproject-on-an-empty-interval) | C++ `R1Interval::Project` | an empty interval reaches `std::clamp` with `hi < lo` in release builds | undefined behaviour | panics in every build, documented | 005 T050 |
| [U-009](#u-009-r1intervalgetdirectedhausdorffdistance-at-infinite-bounds) | C++ `R1Interval::GetDirectedHausdorffDistance` | at infinite bounds the NaN of `∞ − ∞` makes it return 0: `[0, ∞]` to `[5, ∞]` | wrong results (infinite bounds only) | the true distance, a documented deviation | 005 T051 |
| [U-010](#u-010-r1intervalgetcenter-overflows) | C++ `R1Interval::GetCenter` | `0.5 * (lo + hi)` overflows for finite bounds beyond `DBL_MAX / 2`: the center of `[DBL_MAX, DBL_MAX]` is ∞ | wrong results (huge bounds only) | the true center, a documented deviation | 005 T052 |
| [U-011](#u-011-s1intervalexpanded-crosses-its-bounds) | C++ `S1Interval::Expanded`, Go `Interval.Expanded` | shrinking an arc to almost nothing can give almost the full circle: the rounding allowance of 2·ε is too small, and the bounds cross | wrong results (margins within about 10⁻¹⁵ of emptying an inverted arc) | the empty interval, a documented deviation | 004 T057 |
| [U-012](#u-012-robustcrossprod-overflows-for-unit-points) | C++ `S2::RobustCrossProd` (`EnsureNormalizable`) | `RobustCrossProd(−b, b)` is `(NaN, ∞, NaN)` for the unit point `b = (2⁻¹⁰⁷⁴, 0, 1)`: the scaling factor overflows below `2⁻¹⁰²³` | wrong results (antipodal points with a coordinate below 2⁻¹⁰²³) | the scaled vector, a documented deviation | 006 T057 |
| [U-013](#u-013-pointcross-breaks-its-documented-sign-laws) | Go `Point.PointCross` | the documented laws (2)–(4) fail for distinct parallel points, not only for `p == ±op`: `(1, 0, 0)` and `(1 + 2ε, 0, 0)`, both unit | wrong documentation | the same behaviour, the true exception documented | 006 T058 |
| [U-014](#u-014-s1angle-e5e6e7-outside-the-int32-range) | C++ `S1Angle::e5`/`e6`/`e7`, `S1ChordAngle::e5`/`e6`/`e7` | outside the `int32` range, release builds return −2147483648 (+∞ and large positive angles come back negative) and debug builds fail a check, also for `S1ChordAngle::Infinity()`, a valid value; ties round to even on x86-64 and away from zero elsewhere | wrong results (angles beyond ±214.7° for E7, the `Infinity()` sentinel) | saturates, ties to even, documented | 007 T061 |
| [U-015](#u-015-s2capunion-does-not-always-enclose-both-caps) | C++ `S2Cap::Union` | the result does not contain both caps on 337 of 1,089 random and special reference rows, by rounding (one ulp of the radius in the example), though its comment says it "encloses" them | a union used as a bound can miss points of one cap by an ulp | adds the conversion errors to the radius (7 rows of 1,089 still miss), documented as a deviation; `add_cap` for guaranteed containment | 008 T060 |
| [U-016](#u-016-s2latlngrectgetcapbound-misses-contained-points) | C++ `S2LatLngRect::GetCapBound` | the bounding cap misses points the rectangle contains, by up to an ulp of π/2 in angle: the north pole as a rectangle misses a point one ulp past the pole, and a cap around the centre a point a few ulps inside a vertex; 10,507,883 of 74,647,924 adversarial contained points, 114 of the survey's 129,595 | a bound used by the coverer and the `Region` contract misses points near the rectangle's boundary | widened by 10⁻¹⁵ rad and 8ε of the length², a documented deviation | 010 T014 |

## U-001: `S2CellId::level()` on id 0

[`s2cell_id.h:595–603`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell_id.h#L595-L603):

```cpp
inline int S2CellId::level() const {
  // ... there is no good way to define S2CellId::None().level(), so we do prohibit that.
  ABSL_ASSUME(id_ != 0);
  return kMaxLevel - (absl::countr_zero(id_) >> 1);
}
```

- **Inputs:** `S2CellId(0)` (that is, `S2CellId::None()`); `1 << 62`; `1 << 63`.
- **Upstream (from the code):** for `0`, `ABSL_ASSUME` tells the compiler the case cannot
  happen, so the behaviour is undefined. For `1 << 62` and `1 << 63`, `countr_zero >> 1` is
  31 and the result is −1, outside 0–30. `level()` checks nothing else, and it is reachable
  from untrusted bytes through U-004.
- **s2rst:** `CellId::level` (`core/src/s2/cell_id.rs:296`) panics on these ids, as its
  `# Panics` section documents.
- **Follow-up:** `specs/001-formal-verify-base-type/tasks.md` T054.

## U-002: `GetCommonAncestorLevel` on (`None`, `None`)

C++ [`s2cell_id.cc:193–207`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell_id.cc#L193-L207):

```cpp
uint64_t bits = max(id() ^ other.id(), max(lsb(), other.lsb()));
ABSL_ASSUME(bits != 0);  // Because lsb() is non-zero.
```

Go [`s2/cellid.go:741–748`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/s2/cellid.go#L741-L748):

```go
diffBits := max(uint64(ci^other), ci.lsb(), other.lsb())
msbPos := bits.Len64(diffBits) - 1
if msbPos > 60 {
    return 0, false
}
return (60 - msbPos) >> 1, true
```

- **Inputs:** `None` with `None`, the only pair for which `bits` is 0 (`lsb()` of id 0 is 0,
  so the comment is false for it).
- **Upstream:** C++ has undefined behaviour (from the code). Go returns `(30, true)`: with
  `msbPos = -1` the `> 60` check passes, so it reports a common ancestor at the leaf level
  for two ids that are not cells.
- **s2rst:** `CellId::common_ancestor_level` (`core/src/s2/cell_id.rs:868`) panics for that
  pair, as its `# Panics` section documents.
- **Follow-up:** `specs/001-formal-verify-base-type/tasks.md` T055.

## U-003: `S2CellUnion::Decode` accepts invalid unions

[`s2cell_union.cc:544–562`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell_union.cc#L544-L562)
decodes the ids with `S2CellId::Decode`, which validates nothing
([`s2cell_id.cc:261–265`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell_id.cc#L261-L265)),
and stores them as they are. But a verbatim union is required to be valid:
`FromVerbatim` says "REQUIRES: `cell_ids` satisfies the requirements of `IsValid()`"
([`s2cell_union.h:142`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell_union.h#L142)),
and `Contains` relies on it.

- **Inputs:** version 1, count 2, ids `[0x3000000000000000, 0x1000000000000000]` (face 1,
  then face 0: unsorted); version 1, count 1, id `0`.
- **Upstream (observed):** `Decode` returns `true` and a union whose `IsValid()` is
  `false`. Ten rows of `core/tests/data/cpp_cellunion_vectors.csv` (`CELLUNION_DECODE`,
  `ok = 1, valid = 0`) record it, with invalid, unsorted, duplicate and overlapping ids.
  Queries on such a union can then answer wrongly without any error, and
  `LeafCellsCovered` can wrap (from the code).
- **s2rst:** `impl S2Decode for CellUnion` (`core/src/s2/encoding.rs:406`) rejects such
  input with `InvalidData`, a documented deviation.
- **Follow-up:** `specs/002-formal-verify-cell-union/tasks.md` T054.

## U-004: `S2Cell::Decode` on unvalidated ids

[`s2cell.cc:326–332`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell.cc#L326-L332):

```cpp
bool S2Cell::Decode(Decoder* const decoder) {
  S2CellId id;
  if (!id.Decode(decoder)) return false;   // accepts any 8 bytes
  this->~S2Cell();
  new (this) S2Cell(id);
  return true;
}
```

The constructor ([`s2cell.cc:64–71`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell.cc#L64-L71))
calls `id.level()`.

- **Inputs:** 8 zero bytes; an id whose face bits are 6 or 7 (for example
  `0xd000000000000000`).
- **Upstream (from the code):** 8 zero bytes give `S2CellId(0)`, so the constructor reaches
  U-001's undefined behaviour, from untrusted input. An id with face 6 or 7 decodes; the
  undefined behaviour comes later, when the cell is used: `GetUVWAxis` indexes
  `kFaceUVWAxes[face]` out of bounds
  ([`s2coords.h:481`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2coords.h#L481)).
  Not run: the C++ vector generator leaves these inputs out because they are undefined
  (`tools/refgen-cpp/generate_coding.cc:25`).
- **s2rst:** `impl S2Decode for Cell` (`core/src/s2/encoding.rs:1026`) rejects any id that
  is not a valid cell, a documented deviation.
- **The constructor too:** `S2Cell(S2CellId)` itself ([`s2cell.cc:64–71`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cell.cc#L64-L71))
  takes the same ids with no precondition stated. s2rst's `Cell::from_cell_id` panics on the
  null id and on faces 6 and 7, and gives a cell with unspecified fields for any other invalid
  id, as its `# Panics` section documents (spec 004 FR-017).
- **Follow-up:** `specs/003-formal-verify-decoders/tasks.md` T074.

## U-005: `GetUintWithLength` beyond `sizeof(T)`

[`encoded_uint_vector.h:155–192`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/encoded_uint_vector.h#L155-L192)
checks its precondition `length <= sizeof(T)` only with `ABSL_DCHECK` (line 159).

- **Inputs:** `GetUintWithLength<uint64_t>(ptr, 16)`, or any length above `sizeof(T)`.
- **Upstream (from the code):** release builds return a silently wrong value: for length
  16 every branch's bit test fails and the result is 0; for 9–15 it is the first 8 bytes.
  It never reads more than `sizeof(T)` bytes, so this is not undefined behaviour, and
  upstream's own decoders cannot pass such a length (their widths come from 3-bit fields).
  The risk is to other callers of a public header function.
- **s2rst:** `get_uint_with_length` (`core/src/s2/encoded_uint_vector.rs:48`) panics on a
  length above 8, as its `# Panics` section documents.
- **Follow-up:** `specs/003-formal-verify-decoders/tasks.md` T075.

## U-006: `S2Cap::Decode` does not validate

[`s2cap.cc:340–353`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cap.cc#L340-L353):

```cpp
bool S2Cap::Decode(Decoder* decoder) {
  if (decoder->avail() < 4 * sizeof(double)) return false;
  double x = decoder->getdouble();
  double y = decoder->getdouble();
  double z = decoder->getdouble();
  center_ = S2Point(x, y, z);
  radius_ = S1ChordAngle::FromLength2(decoder->getdouble());
  if (absl::GetFlag(FLAGS_s2debug)) {
    ABSL_CHECK(is_valid()) << "Invalid S2Cap: " << *this;
  }
  return true;
}
```

`FLAGS_s2debug` defaults to `S2_DEBUG_MODE`
([`s2debug.cc:23`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2debug.cc#L23)),
which is true in any build without `NDEBUG`
([`base/log_severity.h:20–24`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/base/log_severity.h#L20-L24)).
`FromLength2` is `std::min(4.0, length2)`
([`s1chord_angle.h:365–367`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1chord_angle.h#L365-L367)).

- **Inputs:** a center with a NaN coordinate; the center `(3, 4, 12)`, of length 13; a NaN
  `length2`.
- **Upstream:** in release builds all three decode (observed: the `CAP_DECODE` rows of
  `core/tests/data/cpp_coding_vectors.csv` marked `deviation:reject`). The NaN `length2`
  becomes 4.0, a full cap, because `std::min(4.0, NaN)` returns its first argument. In debug
  builds the same bytes fail the `ABSL_CHECK` and abort the process (from the code). The
  sibling decoder `S2LatLngRect::Decode` returns `false` on an invalid rectangle instead
  ([`s2latlng_rect.cc:386–390`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2latlng_rect.cc#L386-L390)).
- **s2rst:** `impl S2Decode for Cap` (`core/src/s2/encoding.rs:234`) rejects an invalid cap
  with `InvalidData`, including a NaN `length2` (D-004, test
  `cap_with_nan_length2_is_rejected`), a documented deviation.
- **Follow-up:** `specs/003-formal-verify-decoders/tasks.md` T077.

## U-007: `S1Interval::GetLength` at the seam

[`s1interval.cc:41–47`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1interval.cc#L41-L47):

```cpp
double S1Interval::GetLength() const {
  double length = hi() - lo();
  if (length >= 0) return length;
  length += 2 * M_PI;
  // Empty intervals have a negative length.
  return (length > 0) ? length : -1;
}
```

- **Inputs:** the valid, non-empty one-ulp interval across the seam,
  `S1Interval(M_PI, nextafter(-M_PI, 0))`, also reachable as
  `FromPointPair(M_PI, nextafter(-M_PI, 0))`.
- **Upstream (observed, `S1INTERVAL_SPECIAL` rows of `core/tests/data/cpp_values_vectors.csv`
  marked `deviation:seam-length`):** `hi − lo` rounds to exactly `−2π`, adding `2π` gives 0,
  and the function returns −1, the empty interval's length. The true length is one ulp of π,
  `2^−51`, which `(hi + π) − (lo − π)` computes exactly. The wrong value spreads to the callers:
  - `Expanded` ([`:213`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1interval.cc#L213), [`:218`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1interval.cc#L218)): expanding
    by 3.2 gives a short arc around 0, on the other side of the circle, not the full circle;
  - `ApproxEquals` ([`:286–295`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1interval.cc#L286-L295)): at zero error the interval
    counts as approximately empty;
  - `Intersection` ([`:268`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1interval.cc#L268)) compares lengths;
  - `S2LatLngRect::Area` ([`s2latlng_rect.cc:94–99`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2latlng_rect.cc#L94-L99)): a
    rectangle with this longitude range has a negative area (−0.0998 for latitudes [0, 0.1]).
- **Uniqueness:** it is the only valid, non-empty interval for which the arithmetic gives no
  positive length. Spec 004's Kani model check P-IMEA-03 proves it over every pair of bounds.
- **s2rst:** `s1::Interval::length` (`core/src/s1/interval.rs`) returns the true length there
  and upstream's value everywhere else, bit for bit (Kani proof P-IMEA-01). It is a documented
  deviation (spec 004 FR-020); the regression tests are in `core/src/s1/interval_tests.rs`.
- **Follow-up:** `specs/004-formal-verify-cell-interval/tasks.md` T053.

## U-008: `R1Interval::Project` on an empty interval

[`r1interval.h:173–176`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/r1interval.h#L173-L176):

```cpp
double Project(double p) const {
  ABSL_DCHECK(!is_empty());
  return std::clamp(p, lo(), hi());
}
```

- **Inputs:** any empty interval, for example `R1Interval::Empty().Project(0.5)` (`[1, 0]`).
- **Upstream (from the code):** debug builds stop at the `ABSL_DCHECK`. Release builds skip
  it and call `std::clamp(p, 1, 0)`, whose behaviour is undefined when `hi < lo`
  ([alg.clamp]). Nothing in the function's comment states the precondition.
- **s2rst:** `r1::Interval::project` (`core/src/r1/interval.rs`) panics on an empty interval in
  every build: a debug assertion, then `f64::clamp`, which asserts its bounds in release
  builds too. Its `# Panics` section documents it (spec 005 FR-019; probed on 2026-10-13 in
  debug and release builds).
- **Follow-up:** `specs/005-formal-verify-r1-interval-rect/tasks.md` T050.

## U-009: `R1Interval::GetDirectedHausdorffDistance` at infinite bounds

[`r1interval.h:145–149`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/r1interval.h#L145-L149):

```cpp
double GetDirectedHausdorffDistance(const R1Interval& y) const {
  if (is_empty()) return 0.0;
  if (y.is_empty()) return HUGE_VAL;
  return std::max(0.0, std::max(hi() - y.hi(), y.lo() - lo()));
}
```

- **Inputs:** `[0, ∞]` to `[5, ∞]`; `[−∞, ∞]` to `[−1, ∞]`.
- **Upstream (observed, the 27 `R1INTERVAL_SPECIAL` rows of
  `core/tests/data/cpp_values_vectors.csv` marked `deviation:infinite-corner`):** `hi − y.hi`
  is `∞ − ∞`, NaN. `std::max(a, b)` returns `a` unless `a < b`, so the NaN wins the inner
  `max` and loses the outer one: the result is 0. The true distances are 5 and ∞.
- **s2rst:** `r1::Interval::directed_hausdorff_distance` uses `f64::max`, which drops a NaN,
  and returns the true distance; for finite bounds the two agree bit for bit. A documented
  deviation (spec 005 FR-017).
- **Follow-up:** `specs/005-formal-verify-r1-interval-rect/tasks.md` T051.

## U-010: `R1Interval::GetCenter` overflows

[`r1interval.h:97`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/r1interval.h#L97):

```cpp
double GetCenter() const { return 0.5 * (lo() + hi()); }
```

- **Inputs:** finite bounds whose sum overflows: `[DBL_MAX, DBL_MAX]`, `[1e308, 1.5e308]`.
- **Upstream (observed, the 150 `R1INTERVAL_SPECIAL` rows marked
  `deviation:center-overflow`):** `lo + hi` overflows to ∞, so the center is ∞ instead of
  `DBL_MAX` and `1.25e308`.
- **s2rst:** `r1::Interval::center` uses `f64::midpoint`, which halves before adding when a
  bound is beyond `f64::MAX / 2`, and returns the true center; everywhere else it is
  upstream's, bit for bit. A documented deviation (spec 005).
- **Follow-up:** `specs/005-formal-verify-r1-interval-rect/tasks.md` T052.

## U-011: `S1Interval::Expanded` crosses its bounds

[`s1interval.cc:208–224`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1interval.cc#L208-L224) (Go: [`interval.go:323–351`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/s1/interval.go#L323-L351), the same
code):

```cpp
  } else {
    if (is_full()) return *this;
    // Check whether this interval will be empty after expansion, allowing
    // for a 1-bit rounding error when computing each endpoint.
    if (GetLength() + 2 * margin - 2 * DBL_EPSILON <= 0) return Empty();
  }
  S1Interval result(remainder(lo() - margin, 2*M_PI),
                    remainder(hi() + margin, 2*M_PI));
```

- **Inputs:** `S1Interval(-0.4669189453124994, -2.7499821821795862).Expanded(-2.0000610351562496)`,
  an inverted arc of length 4.0001220703125 shrunk by just under half its length. Kani found it
  for spec 004's P-IEXP-01 (2026-10-13). A search of the margins just past the emptiness test
  found 182 more in 15 million, all on inverted arcs. They are the last rows of
  `S1INTERVAL_SPECIAL` in `core/tests/data/cpp_values_vectors.csv`, marked
  `deviation:expanded-crossing`.
- **Upstream (observed, C++ and Go):** the true result is an arc of length 5.2e-16. The test
  lets the margin through, because the computed new length is 8.9e-16, above the 2·ε
  allowance. Rounding `hi + margin` (4.4e-16 down), and reducing it by the float `2*M_PI`
  (2.4e-16 short of 2π), move the high bound 2.2e-16 below the low one. The result is
  `[1.5331420898437502, 1.53314208984375]`: inverted, length 2π, almost the full circle, and not
  contained in the arc it was shrunk from. `S2LatLngRect::Expanded` inherits it through the
  longitudes, as does `ExpandedByDistance` with a negative distance. No growing case is known (a
  search of 15 million margins just short of the fullness test found none).
- **s2rst:** `s1::Interval::expanded` (`core/src/s1/interval.rs`) keeps upstream's arithmetic and
  then checks containment: when shrinking gives an interval this one does not contain, it
  returns the empty interval, as upstream does for new lengths up to 2·ε; when growing gives one
  that does not contain this one, the full interval. Every other result is upstream's, bit for
  bit: Kani property P-IEXP-03 states it but does not finish, so the C++ vectors check it on
  their rows, and Kani proves the containment (P-IEXP-01). A documented deviation (spec 004
  FR-023); the regression tests are in `core/src/s1/interval.rs` and `core/src/s2/rect_tests.rs`.
- **Follow-up:** `specs/004-formal-verify-cell-interval/tasks.md` T057.

## U-012: `RobustCrossProd` overflows for unit points

[`s2edge_crossings.cc:294–311`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2edge_crossings.cc#L294-L311), called at lines 344, 346,
357 and 359 on the symbolic perturbation's result:

```cpp
    double p_max = max(fabs(p[0]), max(fabs(p[1]), fabs(p[2])));

    // The expression below avoids signed overflow for any value of ilogb().
    return ldexp(2, -1 - ilogb(p_max)) * p;
```

- **Inputs:** `S2::RobustCrossProd(−b, b)` for `b = (2⁻ᵏ, 0, 1)`, a unit point
  (`S2::IsUnitLength` holds). `−b` and `b` are exactly antipodal, so the stable, `long double`
  and exact cross products are all zero and the symbolic perturbation answers with
  `(−0, 2⁻ᵏ, 0)` before its scaling.
- **Upstream (observed, C++ Release; the `S2POINT_SPECIAL` rows marked
  `deviation:robust-cross-overflow`):** for k ≤ 1023 the factor `2ᵏ` is finite and the result
  is `(−0, 1, 0)`. For k = 1024 and k = 1074 the factor overflows to ∞, and `∞ · 0` makes the
  result `(NaN, ∞, NaN)`: a NaN "cross product" of two valid unit points, which every caller
  then normalises or measures.
- **s2rst:** `edge_crossings::robust_cross_prod` scales with `Point::ensure_normalizable`, which
  multiplies by `2ᵏ` in two exact steps (spec 006 FR-020, FR-021), and returns `(−0, 1, 0)` for
  every k. Wherever upstream's factor is finite the two agree, bit for bit. A documented
  deviation; the regression tests are in `core/src/s2/edge_crossings_tests.rs`. Candidate, not
  reported.
- **Follow-up:** `specs/006-formal-verify-r2-rect-point/tasks.md` T057.

## U-013: `PointCross` breaks its documented sign laws

Go [`s2/point.go:71–95`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/s2/point.go#L71-L95):

```go
// It satisfies the following properties (f == PointCross):
//
//	(1) f(p, op) != 0 for all p, op
//	(2) f(op,p) == -f(p,op) unless p == op or p == -op
//	(3) f(-p,op) == -f(p,op) unless p == op or p == -op
//	(4) f(p,-op) == -f(p,op) unless p == op or p == -op
func (p Point) PointCross(op Point) Point {
	x := p.Add(op.Vector).Cross(op.Sub(p.Vector))
	// Compare exactly to the 0 vector.
	if x == (r3.Vector{}) {
		return Point{p.Ortho()}
	}
```

- **Inputs:** `p = (1, 0, 0)` and `op = (1 + 2ε, 0, 0)`, both `IsUnit`, distinct and parallel.
- **Upstream (observed, Go):** `(p + op) × (op − p)` is exactly the zero vector, so both
  `p.PointCross(op)` and `op.PointCross(p)` return `(0, −1, 0)`, and (2) fails: they are equal,
  not opposite. The same holds for every pair of distinct parallel points, and for points so
  small that the products underflow. C++'s `RobustCrossProd`, the function Go's comment names,
  perturbs parallel points symbolically and keeps the laws.
- **s2rst:** `Point::point_cross` behaves as Go's, and documents the true exception: the result
  is `ortho(p)` wherever `(p + q) × (q − p)` is exactly zero, and the three laws hold, as
  values, elsewhere (Kani property P-PTORT-03; spec 006 FR-022, K11). For equal points it returns
  C++'s `S2::Ortho`, not Go's axis `Ortho` (below). Candidate, not reported.
- **Follow-up:** `specs/006-formal-verify-r2-rect-point/tasks.md` T058; making the laws hold as
  C++ does is T059.

## U-014: `S1Angle::e5`/`e6`/`e7` outside the `int32` range

[`s1angle.h:248–264`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1angle.h#L248-L264), with the `S1ChordAngle` forms at
[`s1chord_angle.h:427–429`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1chord_angle.h#L427-L429):

```cpp
inline int32_t S1Angle::e5() const {
  // TODO(user,b/298298095): Tighten this to (-180, 180] (see IsNormalized).
  ABSL_DCHECK_LE(std::numeric_limits<int32_t>::min() / 1e5, degrees());
  ABSL_DCHECK_LE(degrees(), std::numeric_limits<int32_t>::max() / 1e5);
  return MathUtil::Round<int32_t>(1e5 * degrees());
}

inline int32_t S1ChordAngle::e5() const { return ToAngle().e5(); }
```

`MathUtil::Round<int32_t, double>` is `FastIntRound` on x86 with GCC, the `cvtsd2si` instruction
([`util/math/mathutil.h:172–192`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/util/math/mathutil.h#L172-L192)), and elsewhere
`static_cast<int32_t>(x < 0 ? x - 0.5 : x + 0.5)` ([`:71–79`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/util/math/mathutil.h#L71-L79)).

- **Inputs:** `S1ChordAngle::Infinity().e5()`; `S1Angle::Radians(1e10).e7()`; `S1Angle::Infinity().e5()`;
  `S1Angle::Radians(NaN).e5()`.
- **Upstream (observed, Release, x86-64):** −2147483648 for all four: `cvtsd2si` returns the
  "integer indefinite" value for anything out of range, so +∞ and large positive angles come
  back as the most negative integer. Debug builds fail the `ABSL_DCHECK`s instead, also for
  `S1ChordAngle::Infinity()`, which is a valid chord angle whose `ToAngle()` is
  `S1Angle::Infinity()`. Off x86 the generic `Round` casts an out-of-range double to `int32_t`,
  which is undefined behaviour (from the code).
- **Ties:** where degrees × 10ᵏ is exactly a half integer, `cvtsd2si` rounds to even (`e6` of the
  angle at 2.5 is 2, observed), and the generic `Round` away from zero (it would give 3, from
  the code): upstream's result depends on the processor.
- **s2rst:** the E-forms round ties to even, as the x86-64 reference build (spec 007 FR-022),
  and saturate outside the range: `i32::MAX` above and for +∞, `i32::MIN` below and for −∞,
  0 for NaN, a documented deviation (FR-023). `ChordAngle::INFINITY.e5()` is `i32::MAX`.
  The C++ vectors mark these rows `deviation:eform-saturate`. Candidate, not reported.
- **Follow-up:** `specs/007-formal-verify-chord-angle-angle/tasks.md` T061.

## U-015: `S2Cap::Union` does not always enclose both caps

[`s2cap.cc:121–141`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2cap.cc#L121-L141),
with its comment in `s2cap.h`: "Return the smallest cap which encloses this cap and 'other'."

```cpp
S1Angle result_radius = 0.5 * (distance + this_radius + other_radius);
S2Point result_center = S2::GetPointOnLine(center(), other.center(),
                                           0.5 * (distance - this_radius + other_radius));
return S2Cap(result_center, result_radius);
```

- **Inputs** (from `core/tests/data/cpp_cap_vectors.csv`, `CAP_OPS`): `a` centred at
  (0.9441294862385324, 0.0042567807093452726, 0.3295472546281599) with length² 2.334711958929947e-07,
  `b` centred at (−0.7407004247752106, 0.6718065679323608, −0.006230250457628594) with length²
  0.011072219812086694.
- **Upstream (observed, Release, x86-64):** the union's length² is 1.321982643415236; containing
  both caps needs 1.3219826434152362, one ulp more, so `Union(a, b).Contains(b)` is false. Over the
  reference vectors, 337 of 1,089 unions miss part of a cap this way: the conversion through
  `S1Angle` (`asin`) and back, and the interpolated centre, round, and nothing absorbs it.
- **s2rst:** `Cap::union` adds the chord angle's conversion and point errors to the radius, a
  documented deviation; 7 of the 1,089 rows still miss, all with a single-point or nearly
  single-point cap (spec 008 K9). `Cap::add_cap`, which keeps the centre and rounds up, contains
  the other cap by construction (proven, spec 008 P-CPGRO-02). Candidate, not reported.
- **Follow-up:** `specs/008-formal-verify-cap/tasks.md` T060; the spec-008 part of
  `verify/values/EVALUATION.md` ("Upstream findings").

## U-016: `S2LatLngRect::GetCapBound` misses contained points

[`s2latlng_rect.cc:298–332`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2latlng_rect.cc#L298-L332),
with the `S2Region` contract in `s2region.h`: "Returns a bounding spherical cap that contains
the region."

```cpp
S2Cap pole_cap(S2Point(0, 0, pole_z),
               S1Angle::Radians((1 + 2 * DBL_EPSILON) * pole_angle));
...
  S2Cap mid_cap(GetCenter().ToPoint(), S1Angle::Zero());
  for (int k = 0; k < 4; ++k) mid_cap.AddPoint(GetVertex(k).ToPoint());
```

- **Why.** `Contains(const S2Point&)` tests the point's `S2LatLng`, whose `atan2` puts the
  latitude within about an ulp of π/2 (2.2·10⁻¹⁶ rad) of the point's near the poles, so a
  contained point can lie that far outside the rectangle's true region. The pole cap allows
  only a relative `2ε` of its angle, and the cap around the centre is built from the four
  vertices alone.
- **Inputs** (`specs/010-formal-verify-latlng/probe/cpp/capbound_cases.cc`, observed, Release,
  x86-64):
  - the rectangle `[π/2, π/2] × [−π, π]` (the north pole) and the point of latitude
    1.5707963267948968, longitude 0.46539129845781746 (one ulp past the pole): contained, and
    outside the pole cap of radius 0 (its squared chord is 2.6·10⁻³²);
  - the rectangle `[1.0712215270180785, 1.0712376688984673] × [−0.8905155217671079,
    −0.8905155217089906]` and the point (1.0712215270180785, −0.8905155217089911), on the low
    latitude bound five ulps inside the high longitude bound: contained, and outside the cap
    around the centre by 1.8·10⁻²⁴ of squared chord.
- **How often.** Spec 010's survey (section G: 20,000 random rectangles, 7 points on and inside
  each boundary) found 114 of 129,595 contained points outside, all in pole caps, in C++ and the
  library alike; its planning probe R0 (a million rectangles near and at the poles, points on
  the bounds and a few ulps inside them) found 10,507,883 of 74,647,924, 489,465 of them in caps
  around the centre.
- **s2rst:** `Rect::cap_bound` computes and chooses upstream's cap, then widens it by 10⁻¹⁵ rad
  (`Cap::expanded`) and by 8ε of its squared chord length, a documented deviation; R0 then
  finds no miss. The centre stays upstream's bit for bit, and the C++ vectors mark the radii as
  deviation rows. Candidate, not reported.
- **Follow-up:** `specs/010-formal-verify-latlng/tasks.md` T014; the spec-010 part of
  `verify/values/EVALUATION.md` ("Upstream findings").

## Upstream behaviours s2rst does not follow

Deliberate deviations where the C++ reference has no defect of its own but the library
chose another reference's behaviour. Each is documented in the rustdoc of the item
(Constitution I).

| C++ | Go | s2rst |
|-----|----|-------|
| `S2LatLngRect::AddPoint(const S2LatLng&)` and `Contains(const S2LatLng&)` ([`s2latlng_rect.cc:156–162`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2latlng_rect.cc#L156-L162), [`214–220`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2latlng_rect.cc#L214-L220)) only log an invalid point (`ABSL_DLOG_IF`) and then use it | `Rect.AddPoint` returns the rectangle and `Rect.ContainsLatLng` returns `false` for an invalid point ([`rect.go:139–145`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/s2/rect.go#L139-L145), [`284–289`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/s2/rect.go#L284-L289)) | follows Go: `Rect::add_point` ignores and `Rect::contains_lat_lng` refuses an invalid `LatLng`, so an invalid point never silently widens a bound (spec 005 FR-018) |
| `S2::RobustCrossProd(a, a)` returns `S2::Ortho(a)` ([`s2edge_crossings.cc:165–167`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2edge_crossings.cc#L165-L167)) | `Point.PointCross(p, p)` returns the axis-aligned `Vector.Ortho` ([`point.go:88–92`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/s2/point.go#L88-L92)) | follows C++: `Point::point_cross(p, p)` is `ortho(p)`, upstream's perturbed `Ortho` (spec 006 FR-022, K10) |
| `Vector3::Normalize` multiplies by zero where the norm is zero, so each zero keeps its coordinate's sign ([`util/math/vector.h:191–198`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/util/math/vector.h#L191-L198)) | `Vector.Normalize` returns `(0, 0, 0)` where the squared norm is zero ([`r3/vector.go:44–50`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/r3/vector.go#L44-L50)) | follows Go: `Vector::normalize` and `Point::normalize` return `(+0, +0, +0)` there; equal as values (spec 006 FR-022, K12) |
| `S2::RobustCrossProd` answers nearly parallel points from a `long double` step with `(a − b) × (a + b)`, twice the cross product, and its exact step returns the cross product unnormalised ([`s2edge_crossings.cc:168–174`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2edge_crossings.cc#L168-L174), [`318–336`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2edge_crossings.cc#L318-L336)) | no `RobustCrossProd` (listed as not yet ported); `PreciseVector.Vector` normalises ([`r3/precisevector.go:96–102`](https://github.com/golang/geo/blob/fe8a584c1b096853683cd667ed9fe510050dd8d3/r3/precisevector.go#L96-L102)) | `edge_crossings::robust_cross_prod` has no `long double` step, and its exact step returns the cross product normalised: the same direction (within 2·10⁻¹⁶ rad on the C++ vectors), another length (spec 006 FR-021) |

## Other upstream behaviours that s2rst inherits

Not numbered: they came from the survey on the branch
`claude/formal-methods-type-invariance-ept6u0`
(`docs/formal-methods-and-type-invariance.md`) or are already tracked upstream. Each was
re-checked against the sources above; the s2rst side was re-run on 2026-10-06
(`verify/decoding/EVALUATION.md`, "The other branch's findings").

| Upstream | Behaviour | Status |
|----------|-----------|--------|
| `S1ChordAngle::is_valid` ([`s1chord_angle.h:361–363`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1chord_angle.h#L361-L363), [`435–438`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s1chord_angle.h#L435-L438)) | Accepts every negative `length2`, not only `Negative()`: `FromLength2(-5.0)` is valid, and `−∞` is both negative and infinite. Upstream's own `TODO(ericv)` on `is_negative` suggests the stricter check. | no longer inherited since spec 007 (FR-017, option B): `ChordAngle::from_length2` maps every negative value, −∞ included, to `NEGATIVE`, and the `Cap` decoder decodes a negative `length2` as −1 where C++ keeps it (a documented deviation); the C++ vectors mark those rows `deviation:negative-canonical` |
| `S2EdgeVectorShape::set_dimension` ([`s2edge_vector_shape.h:60`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2edge_vector_shape.h#L60)) | Does not check the edges: dimension 0 with a non-degenerate edge, or dimension 2 with an open chain, breaks the `S2Shape` contract that `S2ShapeIndex` relies on. | inherited by `EdgeVectorShape::set_dimension` (`core/src/s2/edge_vector_shape.rs:61`) |
| `S2::FaceUVtoXYZ` ([`s2coords.h:368–380`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/s2coords.h#L368-L380)) | Treats faces 6 and 7 as face 5. Upstream's `TODO(b/465912778)` says the error check is missing in `EncodedS2PointVector::DecodeCellIdsFormat`. | known upstream; s2rst clamps the face to 5 in the same place (`core/src/s2/encoded_s2point_vector.rs:584`) |

## Checked and not an issue

- **Decreasing string-vector offsets.** Upstream v0.14 accepts them and decodes an
  out-of-order or out-of-range entry as empty ("Corrupt data, return empty string",
  [`encoded_string_vector.h:155–158`](https://github.com/google/s2geometry/blob/37a4501d5daea8da8bb2aa159b23ea5c61083a72/src/s2/encoded_string_vector.h#L155-L158),
  since the sync commit `95b1e15`). s2rst does the same (spec 003 FR-020). An earlier survey
  had read v0.11.1, which rejected them.
- **Decoders reserving memory from a declared count** (spec 003, K2) was an s2rst defect,
  not an upstream one: upstream checks the bytes available before allocating. Fixed in
  s2rst by FR-019.
