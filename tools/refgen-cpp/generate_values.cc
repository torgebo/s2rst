// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
//
// Reference-vector generator for the s2rst value types S1Interval and S2Cell
// (specs/004-formal-verify-cell-interval, FR-021, research R10), R1Interval and
// S2LatLngRect (specs/005-formal-verify-r1-interval-rect, FR-021, research R9), and R2Rect and
// S2Point (specs/006-formal-verify-r2-rect-point, FR-023, research R9), and S1ChordAngle and
// S1Angle (specs/007-formal-verify-chord-angle-angle, FR-024, research R9), using the C++
// reference implementation (google/s2geometry) as the oracle. Emits CSV that
// core/tests/cpp_values_diff.rs diffs the Rust types against, and that
// verify/tests/values_spec_vs_cpp.rs checks the reference descriptions against.
// Deterministic (fixed seed). Each spec's sections come after the earlier ones, so the earlier
// lines stay byte-identical. See tools/refgen-cpp/README.md.
//
// Every double is the 16-hex-digit bit pattern of its value; a bool is 0 or 1; "-" marks a
// result that is not computed because the input is outside the operation's domain.
//
// # S1INTERVAL_OPS, # S1INTERVAL_SPECIAL
//   x_lo,x_hi,p,margin,max_error,y_lo,y_hi,results,note
//   results is ';'-separated, in this order:
//     0 is_valid  1 is_empty  2 is_full  3 is_inverted  4 GetCenter  5 GetLength
//     6 Complement.lo  7 Complement.hi  8 GetComplementCenter  9 Contains(p)
//     10 InteriorContains(p)  11 Project(p) ("-" for an empty x)  12 AddPoint(p).lo
//     13 AddPoint(p).hi  14 Expanded(margin).lo  15 Expanded(margin).hi  16 Contains(y)
//     17 InteriorContains(y)  18 Intersects(y)  19 InteriorIntersects(y)  20 Union(y).lo
//     21 Union(y).hi  22 Intersection(y).lo  23 Intersection(y).hi
//     24 GetDirectedHausdorffDistance(y)  25 ApproxEquals(y, max_error)
//   note is empty, or deviation:seam-length when x or y is the one-ulp interval across the
//   seam, [π, nextafter(−π, 0)]: there upstream's GetLength returns −1 (U-007), and the
//   library returns the true length (spec FR-020). Results 5, 14, 15, 22, 23 and 25 use that
//   length, so the library's documented values differ from C++'s. The note also lists, joined
//   with '+', deviation:expanded-crossing when x is not the corner and Expanded(margin) breaks
//   containment: shrinking gives an interval x does not contain, or growing one that does not
//   contain x. There the bounds crossed (U-011), and the library returns the empty or full
//   interval (spec FR-023). (The corner's own expansion is the seam-length deviation.)
//
// # CELL_FIELDS
//   id,face,level,orientation,u_lo,u_hi,v_lo,v_hi,edge0,edge1,edge2,edge3,size_ij,is_leaf
//   (edgeK is GetUVCoordOfEdge(k); id is decimal)
//
// # CELL_CHILDREN
//   parent_id,k,child_id,child_orientation,u_lo,u_hi,v_lo,v_hi   (from Subdivide)
//
// Inputs deliberately left out: points outside [−π, π], a NaN margin, and Project on an
// empty interval, which upstream checks only with ABSL_DCHECK (spec K3, K5); and invalid cell
// ids, which reach U-001's undefined behaviour (spec K1, U-004).
//
// # R1INTERVAL_OPS, # R1INTERVAL_SPECIAL (spec 005)
//   x_lo,x_hi,p,margin,max_error,y_lo,y_hi,results,note
//   results is ';'-separated, in this order:
//     0 is_empty  1 GetCenter  2 GetLength  3 Contains(p)  4 InteriorContains(p)
//     5 Project(p) ("-" for an empty x)  6 AddPoint(p).lo  7 AddPoint(p).hi
//     8 Expanded(margin).lo  9 Expanded(margin).hi  10 Contains(y)  11 InteriorContains(y)
//     12 Intersects(y)  13 InteriorIntersects(y)  14 AddInterval(y).lo  15 AddInterval(y).hi
//     16 Union(y).lo  17 Union(y).hi  18 Intersection(y).lo  19 Intersection(y).hi
//     20 GetDirectedHausdorffDistance(y)  21 ApproxEquals(y, max_error)  22 x == y
//   note is empty, or a '+'-joined list of the library's documented deviations on the row:
//     deviation:center-overflow  GetCenter computes 0.5 * (lo + hi), which overflows for
//       finite bounds beyond DBL_MAX / 2 (and gives NaN for [−∞, ∞] in both); the library's
//       f64::midpoint halves first (result 1 differs);
//     deviation:infinite-corner  at infinite bounds, std::max keeps the NaN of ∞ − ∞ where
//       the library's f64::max drops it (result 20 differs; spec 005 U-009).
//
// # RECT_OPS, # RECT_SPECIAL (spec 005)
//   a_lat_lo,a_lat_hi,a_lng_lo,a_lng_hi,ll_lat,ll_lng,m_lat,m_lng,b_lat_lo,b_lat_hi,
//   b_lng_lo,b_lng_hi,results,note
//   results is ';'-separated, in this order:
//     0 is_valid  1 is_empty  2 is_full  3 is_point  4 is_inverted
//     5..12 GetVertex(k).lat, GetVertex(k).lng for k = 0..3
//     13 GetCenter.lat  14 GetCenter.lng  15 GetSize.lat  16 GetSize.lng
//     17 Contains(ll)  18 InteriorContains(ll)  19..22 AddPoint(ll) lat lo, hi, lng lo, hi
//       (17..22 are "-" for an invalid ll: upstream only logs it; the library ignores it,
//       spec 005 K2)
//     23..26 Expanded(m)  27..30 PolarClosure()  (bounds as for AddPoint)
//     31 Contains(b)  32 InteriorContains(b)  33 Intersects(b)  34 InteriorIntersects(b)
//     35..38 Union(b)  39..42 Intersection(b)  43 ApproxEquals(b)  44 a == b
//     45..48 FromCenterSize(ll, |m|) ("-" for an invalid ll)
//   note is empty, or deviation:seam-length when a's or b's longitudes are the seam corner
//   (as for S1INTERVAL_*): results 16, 23..26, 39..42 and 43 use its length.
//
// Inputs deliberately left out (spec 005): NaN anywhere; Project on an empty interval
// (U-008); a NaN margin; and an invalid latitude–longitude passed to the operations that
// upstream only logs (17..22, 45..48 above).
//
// # R2RECT_OPS, # R2RECT_SPECIAL (spec 006)
//   a_x_lo,a_x_hi,a_y_lo,a_y_hi,p_x,p_y,m_x,m_y,max_error,b_x_lo,b_x_hi,b_y_lo,b_y_hi,results,
//   note
//   results is ';'-separated, in this order:
//     0 is_valid  1 is_empty  2..9 GetVertex(k).x, GetVertex(k).y for k = 0..3
//     10 GetCenter.x  11 GetCenter.y  12 GetSize.x  13 GetSize.y ("-" for an empty a)
//     14 Contains(p)  15 InteriorContains(p)  16..19 AddPoint(p) x lo, x hi, y lo, y hi
//     20 Project(p).x  21 Project(p).y ("-" for an empty a: U-008)
//     22..25 Expanded(m)  26..29 Expanded(m_x)  (bounds as for AddPoint)
//     30 Contains(b)  31 InteriorContains(b)  32 Intersects(b)  33 InteriorIntersects(b)
//     34..37 AddRect(b)  38..41 Union(b)  42..45 Intersection(b)
//     46 ApproxEquals(b, max_error)  47 ApproxEquals(b)  48 a == b
//     49..52 FromCenterSize(p, (|m_x|, |m_y|))
//   note is empty, or a '+'-joined list:
//     deviation:center-overflow  as for R1INTERVAL_*: a bound beyond DBL_MAX / 2, where the
//       library's f64::midpoint differs from 0.5 * (lo + hi) (results 10, 11);
//     deviation:infinite-corner  an operand bound, the point or the margin is infinite and a
//       result among 10..13, 22..29, 46, 47 or 49..52 is NaN (∞ − ∞); the library computes the
//       same arithmetic and documents these corners (spec 006 FR-018).
//   Every rectangle is valid (both axes empty or both not) and no input is NaN, so no
//   constructor leaves its ABSL_DCHECK domain.
//
// # S2POINT_OPS, # S2POINT_SPECIAL (spec 006)
//   a_x,a_y,a_z,b_x,b_y,b_z,angle,results,note
//   results is ';'-separated, in this order:
//     0 a < b  1 b < a  2 a == b  3 S2::IsUnitLength(a)  4..6 a.Normalize()
//     7..9 S2::Ortho(a)  10..12 S2::RobustCrossProd(a, b)  13 S1Angle(a, b).radians()
//     14 S1ChordAngle(a, b).length2()  15..17 S2::Rotate(a, b, angle)
//   7..17 are "-" when a coordinate of a or b is not finite; 10..12, 14 and 15..17 are "-"
//   unless a and b are both unit length (their ABSL_DCHECKs).
//   note is empty, or a '+'-joined list:
//     deviation:robust-cross-overflow  RobustCrossProd's result is not finite: its
//       EnsureNormalizable overflows for a largest component below 2^-1023 (spec 006 K6,
//       U-012); the library's result is finite, non-zero and normalizable (FR-021);
//     deviation:robust-cross-long-double  RobustCrossProd took its long double step, which
//       the library does not have: C++ returns (a − b) × (a + b) rounded in 80 bits, twice the
//       cross product, where the library's exact step returns a × b itself;
//     deviation:normalize-zero-sign  the squared length of a is zero, so Normalize multiplies
//       by zero and keeps each coordinate's sign, where the library (as Go) returns
//       (+0, +0, +0) (spec 006 K12; results 4..6, and 7..9 when Ortho's cross product is zero).
//
// # CHORD_ANGLE (spec 007)
//   kind,l,m,e,results,note
//   kind L: a = S1ChordAngle::FromLength2(l); kind I: a = S1ChordAngle::Infinity() (which
//   FromLength2 cannot build: it clamps +inf to 4), with l = +inf. b = FromLength2(m); e is an
//   error bound.
//   results is ';'-separated, in this order:
//     0 a.length2()  1 is_zero  2 is_negative  3 is_infinity  4 is_special  5 is_valid
//     6 a.Successor().length2()  7 a.Predecessor().length2()  8 a.PlusError(e).length2()
//     9 GetS2PointConstructorMaxError()  10 GetS1AngleConstructorMaxError()
//     11 sin2(a)  12 cos(a)  13 sin(a)  14 tan(a)  ("-" for a special a: their ABSL_DCHECKs)
//     15 a.ToAngle().radians()  16 a.e5()  17 a.e6()  18 a.e7()
//     19 (a + b).length2()  20 (a - b).length2()  ("-" if a or b is special)
//     21 a < b  22 a == b  23 S1ChordAngle(S1Angle::Radians(l)).length2()
//   note is empty, or a '+'-joined list:
//     deviation:negative-canonical  l is negative and not -1 (or -inf): C++ keeps it, the
//       library's FromLength2 returns Negative() (spec 007 FR-017, option B), so results 0..10,
//       15..18 and 21..22 are the library's for -1 instead;
//     deviation:eform-saturate  one of 16..18 is out of the int32 range: C++ (x86-64, Release)
//       returns -2147483648, the library saturates (spec 007 FR-023, U-014).
//
// # ANGLE (spec 007)
//   r,s,n,results,note
//   r and s are doubles, n an int32 (as a decimal integer).
//   results is ';'-separated, in this order:
//     0 S1Angle::Degrees(r).radians()  1 S1Angle::Radians(r).degrees()
//     2..4 e5(), e6(), e7() of Radians(r)  5 Normalized().radians()  6 IsNormalized()
//     7 abs().radians()  8 sin  9 cos  10 tan  (of Radians(r))
//     11 (Radians(r) + Radians(s)).radians()  12 (Radians(r) - Radians(s)).radians()
//     13 (Radians(r) * s).radians()  14 (Radians(r) / s).radians()  15 Radians(r) / Radians(s)
//     16..18 E5(n), E6(n), E7(n) radians  19 UnsignedE6(n's bits)  20 UnsignedE7(n's bits)
//   note is empty, or deviation:eform-saturate (one of 2..4 out of the int32 range, as above).

#include <cfloat>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <random>
#include <string>
#include <vector>

#include "s2/r1interval.h"
#include "s2/r2rect.h"
#include "s2/s1angle.h"
#include "s2/s1chord_angle.h"
#include "s2/s1interval.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2edge_crossings.h"
#include "s2/s2edge_crossings_internal.h"
#include "s2/s2point.h"
#include "s2/s2pointutil.h"

namespace {

std::mt19937_64 rng(0x56414C55ULL);  // "VALU"
std::uniform_real_distribution<double> unit(0.0, 1.0);

// Spec 006's random rows, sized to keep the CSV under 9 MB (it is not published).
constexpr int kR2OpsRows = 800;
constexpr int kPointOpsRows = 1500;

std::string Hex64(uint64_t x) {
  char buf[17];
  std::snprintf(buf, sizeof buf, "%016llx", static_cast<unsigned long long>(x));
  return buf;
}

uint64_t Bits(double d) {
  uint64_t b;
  std::memcpy(&b, &d, sizeof b);
  return b;
}

double FromBits(uint64_t b) {
  double d;
  std::memcpy(&d, &b, sizeof d);
  return d;
}

std::string D(double d) { return Hex64(Bits(d)); }
std::string B(bool b) { return b ? "1" : "0"; }

// ---- s1::Interval -------------------------------------------------------------------------

const double kCornerHi = FromBits(Bits(-M_PI) - 1);  // nextafter(−π, 0)

bool IsCorner(const S1Interval& x) {
  return x.lo() == M_PI && Bits(x.hi()) == Bits(kCornerHi);
}

double AnyPointIn(double lo, double hi) { return lo + (hi - lo) * unit(rng); }

// A point in [−π, π], with the special values and tiny ones mixed in.
double RandomPoint() {
  switch (rng() % 10) {
    case 0: return M_PI;
    case 1: return -M_PI;
    case 2: return 0.0;
    case 3: return std::ldexp(AnyPointIn(-1, 1), -static_cast<int>(rng() % 1000));
    default: return AnyPointIn(-M_PI, M_PI);
  }
}

// A valid interval: empty, full, a single point, the arc through two points, or two
// independent bounds (often inverted). The two-argument constructor leaves a valid pair
// unchanged, so these are the struct literals the Rust harnesses build.
S1Interval RandomInterval() {
  switch (rng() % 12) {
    case 0: return S1Interval::Empty();
    case 1: return S1Interval::Full();
    case 2: {
      double p = RandomPoint();
      return S1Interval::FromPoint(p);
    }
    case 3:
    case 4: return S1Interval::FromPointPair(RandomPoint(), RandomPoint());
    default: {
      double lo = RandomPoint(), hi = RandomPoint();
      if (lo == -M_PI && hi != M_PI) lo = M_PI;
      if (hi == -M_PI && lo != M_PI) hi = M_PI;
      return S1Interval(lo, hi);
    }
  }
}

double RandomMargin() {
  switch (rng() % 8) {
    case 0: return 0.0;
    case 1: return (rng() % 2 ? 1 : -1) * M_PI;
    case 2: return (rng() % 2 ? 1 : -1) * std::ldexp(unit(rng), -static_cast<int>(rng() % 60));
    case 3: return (rng() % 2 ? 1 : -1) * 2 * M_PI * unit(rng) * 2;
    default: return AnyPointIn(-4, 4);
  }
}

double RandomMaxError() {
  switch (rng() % 4) {
    case 0: return 0.0;
    case 1: return 1e-15;
    default: return 0.1 * unit(rng);
  }
}

void IntervalRow(const S1Interval& x, double p, double margin, double max_error,
                 const S1Interval& y) {
  std::string r;
  auto add = [&r](const std::string& s) {
    if (!r.empty()) r += ';';
    r += s;
  };
  add(B(x.is_valid()));
  add(B(x.is_empty()));
  add(B(x.is_full()));
  add(B(x.is_inverted()));
  add(D(x.GetCenter()));
  add(D(x.GetLength()));
  S1Interval c = x.Complement();
  add(D(c.lo()));
  add(D(c.hi()));
  add(D(x.GetComplementCenter()));
  add(B(x.Contains(p)));
  add(B(x.InteriorContains(p)));
  add(x.is_empty() ? std::string("-") : D(x.Project(p)));
  S1Interval a = x;
  a.AddPoint(p);
  add(D(a.lo()));
  add(D(a.hi()));
  S1Interval e = x.Expanded(margin);
  add(D(e.lo()));
  add(D(e.hi()));
  add(B(x.Contains(y)));
  add(B(x.InteriorContains(y)));
  add(B(x.Intersects(y)));
  add(B(x.InteriorIntersects(y)));
  S1Interval u = x.Union(y);
  add(D(u.lo()));
  add(D(u.hi()));
  S1Interval i = x.Intersection(y);
  add(D(i.lo()));
  add(D(i.hi()));
  add(D(x.GetDirectedHausdorffDistance(y)));
  add(B(x.ApproxEquals(y, max_error)));
  std::string note = (IsCorner(x) || IsCorner(y)) ? "deviation:seam-length" : "";
  if (!IsCorner(x) && (margin >= 0 ? !e.Contains(x) : !x.Contains(e))) {
    note += std::string(note.empty() ? "" : "+") + "deviation:expanded-crossing";
  }
  std::printf("%s,%s,%s,%s,%s,%s,%s,%s,%s\n", D(x.lo()).c_str(), D(x.hi()).c_str(),
              D(p).c_str(), D(margin).c_str(), D(max_error).c_str(), D(y.lo()).c_str(),
              D(y.hi()).c_str(), r.c_str(), note.c_str());
}

void IntervalOps() {
  std::printf("# S1INTERVAL_OPS\n");
  for (int n = 0; n < 5000; ++n) {
    S1Interval x = RandomInterval(), y = RandomInterval();
    double p = RandomPoint();
    IntervalRow(x, p, RandomMargin(), RandomMaxError(), y);
  }
}

void IntervalSpecial() {
  std::printf("# S1INTERVAL_SPECIAL\n");
  // Upstream's s1interval_test.cc fixtures (after the constructor's normalisation).
  const double h = M_PI_2;
  const S1Interval fixtures[] = {
      S1Interval::Empty(),       S1Interval::Full(),        S1Interval(0, 0),
      S1Interval(M_PI, M_PI),    S1Interval(-M_PI, -M_PI),  S1Interval(0, h),
      S1Interval(h, -M_PI),      S1Interval(M_PI, -h),      S1Interval(-h, 0),
      S1Interval(0, M_PI),       S1Interval(h, -h),         S1Interval(-M_PI, 0),
      S1Interval(-h, h),         S1Interval(0, -h),         S1Interval(h, 0),
      S1Interval(M_PI, h),       S1Interval(-h, -M_PI),     S1Interval(h - 0.01, h + 0.02),
      S1Interval(M_PI - 0.01, -M_PI + 0.02),
      // The K4 corner, its mirror and its neighbours.
      S1Interval(M_PI, kCornerHi), S1Interval(kCornerHi, M_PI),
      S1Interval(std::nextafter(M_PI, 0), kCornerHi),
      S1Interval(M_PI, std::nextafter(kCornerHi, 0)),
      // D-005: a short interval near 0, whose complement's centre rounds to −π.
      S1Interval(0, 1e-17), S1Interval(1, 2),
  };
  const double points[] = {0.0, M_PI, -M_PI, h, -h, kCornerHi, 1e-300};
  const double margins[] = {0.0,  M_PI, -M_PI, 2 * M_PI, -2 * M_PI, 3.2,
                            -3.2, FromBits(1), -FromBits(1), INFINITY, -INFINITY};
  const double errors[] = {0.0, 1e-15, 0.01};
  int k = 0;
  for (const S1Interval& x : fixtures) {
    for (const S1Interval& y : fixtures) {
      const double p = points[k % 7];
      const double m = margins[k % 11];
      const double e = errors[k % 3];
      IntervalRow(x, p, m, e, y);
      ++k;
    }
  }
  // U-011: shrinking to almost nothing, where the bounds cross (spec FR-023). Kani's
  // counterexample, then three from a search of the margins just past the emptiness test.
  const uint64_t crossing[][3] = {
      {0xBFDDE1FFFFFFFFF5, 0xC005FFF6A8885A30, 0xC000001FFFFFFFFF},
      {0xBFD43FFFFFFFFFF5, 0xC00336FE9A4B5060, 0xC000CA7C071E84E7},
      {0xBFCB909E00000009, 0xC001FFC80000000E, 0xC000FE9C44442D11},
      {0x4003707C24BBE916, 0x3FCB5C0000000009, 0xC000449D41E6388D},
  };
  for (const auto& c : crossing) {
    const S1Interval x(FromBits(c[0]), FromBits(c[1]));
    IntervalRow(x, 0.0, FromBits(c[2]), 0.0, x);
  }
}

// ---- S2Cell -------------------------------------------------------------------------------

S2CellId RandomCellId(int level) {
  int face = static_cast<int>(rng() % 6);
  uint64_t pos = rng() & ((uint64_t{1} << S2CellId::kPosBits) - 1);
  return S2CellId::FromFacePosLevel(face, pos, level);
}

void CellFields() {
  std::printf("# CELL_FIELDS\n");
  for (int n = 0; n < 5000; ++n) {
    S2CellId id = RandomCellId(static_cast<int>(rng() % 31));
    S2Cell c(id);
    const R2Rect& uv = c.GetBoundUV();
    std::printf("%llu,%d,%d,%d,%s,%s,%s,%s,%s,%s,%s,%s,%d,%s\n",
                static_cast<unsigned long long>(id.id()), c.face(), c.level(),
                c.orientation(), D(uv.x().lo()).c_str(), D(uv.x().hi()).c_str(),
                D(uv.y().lo()).c_str(), D(uv.y().hi()).c_str(),
                D(c.GetUVCoordOfEdge(0)).c_str(), D(c.GetUVCoordOfEdge(1)).c_str(),
                D(c.GetUVCoordOfEdge(2)).c_str(), D(c.GetUVCoordOfEdge(3)).c_str(),
                c.GetSizeIJ(), B(c.is_leaf()).c_str());
  }
}

void CellChildren() {
  std::printf("# CELL_CHILDREN\n");
  for (int n = 0; n < 250; ++n) {
    S2CellId id = RandomCellId(static_cast<int>(rng() % 30));
    S2Cell c(id), children[4];
    if (!c.Subdivide(children)) continue;
    for (int k = 0; k < 4; ++k) {
      const S2Cell& ch = children[k];
      const R2Rect& uv = ch.GetBoundUV();
      std::printf("%llu,%d,%llu,%d,%s,%s,%s,%s\n", static_cast<unsigned long long>(id.id()), k,
                  static_cast<unsigned long long>(ch.id().id()), ch.orientation(),
                  D(uv.x().lo()).c_str(), D(uv.x().hi()).c_str(), D(uv.y().lo()).c_str(),
                  D(uv.y().hi()).c_str());
    }
  }
}

// ---- r1::Interval (spec 005) ---------------------------------------------------------------

// A real: zeros of both signs, small integers (ties), every binade (ldexp), rarely an
// infinity, and ordinary values.
double RandomReal() {
  switch (rng() % 12) {
    case 0: return 0.0;
    case 1: return -0.0;
    case 2: return static_cast<double>(static_cast<int>(rng() % 7) - 3);
    case 3: return std::ldexp(AnyPointIn(-1, 1), static_cast<int>(rng() % 2099) - 1074);
    case 4: return rng() % 10 == 0 ? (rng() % 2 ? INFINITY : -INFINITY) : AnyPointIn(-1e6, 1e6);
    default: return AnyPointIn(-10, 10);
  }
}

// An interval: empty, a single point, the interval through two reals, or two independent
// bounds (often empty). The two-argument constructor keeps the bounds as given, so these are
// the struct literals the Rust harnesses build.
R1Interval RandomR1() {
  switch (rng() % 10) {
    case 0: return R1Interval::Empty();
    case 1: return R1Interval::FromPoint(RandomReal());
    case 2: case 3: case 4: case 5: case 6:
      return R1Interval::FromPointPair(RandomReal(), RandomReal());
    default: return R1Interval(RandomReal(), RandomReal());
  }
}

// The library's centre (Rust's f64::midpoint): halve before adding when a bound is beyond
// DBL_MAX / 2, so finite bounds never overflow.
double LibCenter(const R1Interval& x) {
  const double kLo = DBL_MIN * 2, kHi = DBL_MAX / 2;
  const double a = x.lo(), b = x.hi();
  if (std::fabs(a) <= kHi && std::fabs(b) <= kHi) return (a + b) / 2;
  if (std::fabs(a) < kLo) return a + b / 2;
  if (std::fabs(b) < kLo) return a / 2 + b;
  return a / 2 + b / 2;
}

// The library's directed Hausdorff distance: f64::max drops a NaN, as std::fmax does.
double LibHausdorff(const R1Interval& x, const R1Interval& y) {
  if (x.is_empty()) return 0.0;
  if (y.is_empty()) return HUGE_VAL;
  return std::fmax(0.0, std::fmax(x.hi() - y.hi(), y.lo() - x.lo()));
}

bool SameBits(double a, double b) { return Bits(a) == Bits(b) || (a != a && b != b); }

void R1Row(const R1Interval& x, double p, double margin, double max_error, const R1Interval& y) {
  std::string r;
  auto add = [&r](const std::string& s) {
    if (!r.empty()) r += ';';
    r += s;
  };
  add(B(x.is_empty()));
  add(D(x.GetCenter()));
  add(D(x.GetLength()));
  add(B(x.Contains(p)));
  add(B(x.InteriorContains(p)));
  add(x.is_empty() ? std::string("-") : D(x.Project(p)));
  R1Interval a = x;
  a.AddPoint(p);
  add(D(a.lo()));
  add(D(a.hi()));
  R1Interval e = x.Expanded(margin);
  add(D(e.lo()));
  add(D(e.hi()));
  add(B(x.Contains(y)));
  add(B(x.InteriorContains(y)));
  add(B(x.Intersects(y)));
  add(B(x.InteriorIntersects(y)));
  R1Interval ai = x;
  ai.AddInterval(y);
  add(D(ai.lo()));
  add(D(ai.hi()));
  R1Interval u = x.Union(y);
  add(D(u.lo()));
  add(D(u.hi()));
  R1Interval i = x.Intersection(y);
  add(D(i.lo()));
  add(D(i.hi()));
  add(D(x.GetDirectedHausdorffDistance(y)));
  add(B(x.ApproxEquals(y, max_error)));
  add(B(x == y));
  std::string note;
  if (!SameBits(x.GetCenter(), LibCenter(x))) note = "deviation:center-overflow";
  if (!SameBits(x.GetDirectedHausdorffDistance(y), LibHausdorff(x, y))) {
    note += std::string(note.empty() ? "" : "+") + "deviation:infinite-corner";
  }
  std::printf("%s,%s,%s,%s,%s,%s,%s,%s,%s\n", D(x.lo()).c_str(), D(x.hi()).c_str(),
              D(p).c_str(), D(margin).c_str(), D(max_error).c_str(), D(y.lo()).c_str(),
              D(y.hi()).c_str(), r.c_str(), note.c_str());
}

void R1Ops() {
  std::printf("# R1INTERVAL_OPS\n");
  for (int n = 0; n < 3000; ++n) {
    R1Interval x = RandomR1(), y = RandomR1();
    const double p = RandomReal(), margin = RandomReal();
    R1Row(x, p, margin, RandomMaxError(), y);
  }
}

void R1Special() {
  std::printf("# R1INTERVAL_SPECIAL\n");
  const double inf = INFINITY;
  std::vector<R1Interval> fixtures = {
      // Upstream's r1interval_test.cc fixtures.
      R1Interval::Empty(), R1Interval(0, 1), R1Interval(-1, 0), R1Interval(0.5, 0.5),
      // Signed zeros.
      R1Interval(0, 0), R1Interval(-0.0, 0), R1Interval(0, -0.0), R1Interval(-0.0, -0.0),
      // Bounds near DBL_MAX, where 0.5 * (lo + hi) overflows.
      R1Interval(DBL_MAX, DBL_MAX), R1Interval(1e308, 1.5e308), R1Interval(-DBL_MAX, DBL_MAX),
      R1Interval(-DBL_MAX, -1e308),
  };
  // Every pair of bounds from {−∞, −1, −0, 0, 1, ∞}, empty ones included.
  const double bounds[] = {-inf, -1, -0.0, 0, 1, inf};
  for (double lo : bounds) {
    for (double hi : bounds) fixtures.push_back(R1Interval(lo, hi));
  }
  // The Hausdorff pairs of research R1.
  fixtures.push_back(R1Interval(5, inf));
  fixtures.push_back(R1Interval(-5, inf));
  const double points[] = {0.0, -0.0, 1, -1, 0.5, inf, -inf, 1e308};
  const double margins[] = {0.0, 1, -1, inf, -inf, 0.25, -0.25};
  const double errors[] = {0.0, 1e-15, 0.5};
  int k = 0;
  for (const R1Interval& x : fixtures) {
    for (const R1Interval& y : fixtures) {
      R1Row(x, points[k % 8], margins[k % 7], errors[k % 3], y);
      ++k;
    }
  }
}

// ---- s2::Rect (spec 005) -------------------------------------------------------------------

// A valid latitude–longitude, with the poles and the antimeridian mixed in.
S2LatLng RandomValidLatLng() {
  switch (rng() % 10) {
    case 0: return S2LatLng::FromRadians(M_PI_2, RandomPoint());
    case 1: return S2LatLng::FromRadians(-M_PI_2, RandomPoint());
    case 2: return S2LatLng::FromRadians(AnyPointIn(-M_PI_2, M_PI_2), rng() % 2 ? M_PI : -M_PI);
    default: return S2LatLng::FromRadians(AnyPointIn(-M_PI_2, M_PI_2), RandomPoint());
  }
}

// A latitude–longitude, valid in 90% of rows; otherwise out of range in one coordinate.
S2LatLng RandomAnyLatLng() {
  switch (rng() % 20) {
    case 0: return S2LatLng::FromRadians((rng() % 2 ? 1 : -1) * AnyPointIn(M_PI_2, 4), 0.5);
    case 1: return S2LatLng::FromRadians(0.5, (rng() % 2 ? 1 : -1) * AnyPointIn(M_PI, 7));
    default: return RandomValidLatLng();
  }
}

// A valid rectangle: empty, full, a point, a full-longitude band, or the rectangle through
// two valid points.
S2LatLngRect RandomRect() {
  switch (rng() % 10) {
    case 0: return S2LatLngRect::Empty();
    case 1: return S2LatLngRect::Full();
    case 2: return S2LatLngRect::FromPoint(RandomValidLatLng());
    case 3:
      return S2LatLngRect(R1Interval::FromPointPair(AnyPointIn(-M_PI_2, M_PI_2),
                                                    AnyPointIn(-M_PI_2, M_PI_2)),
                          S1Interval::Full());
    default: return S2LatLngRect::FromPointPair(RandomValidLatLng(), RandomValidLatLng());
  }
}

void AddRect(std::string& r, const S2LatLngRect& x) {
  r += ';' + D(x.lat().lo()) + ';' + D(x.lat().hi()) + ';' + D(x.lng().lo()) + ';' +
       D(x.lng().hi());
}

void RectRow(const S2LatLngRect& a, const S2LatLng& ll, const S2LatLng& m,
             const S2LatLngRect& b) {
  std::string r = B(a.is_valid());
  auto add = [&r](const std::string& s) { r += ';' + s; };
  const std::string dash = "-";
  add(B(a.is_empty()));
  add(B(a.is_full()));
  add(B(a.is_point()));
  add(B(a.is_inverted()));
  for (int k = 0; k < 4; ++k) {
    add(D(a.GetVertex(k).lat().radians()));
    add(D(a.GetVertex(k).lng().radians()));
  }
  add(D(a.GetCenter().lat().radians()));
  add(D(a.GetCenter().lng().radians()));
  add(D(a.GetSize().lat().radians()));
  add(D(a.GetSize().lng().radians()));
  const bool ll_ok = ll.is_valid();
  add(ll_ok ? B(a.Contains(ll)) : dash);
  add(ll_ok ? B(a.InteriorContains(ll)) : dash);
  if (ll_ok) {
    S2LatLngRect ap = a;
    ap.AddPoint(ll);
    AddRect(r, ap);
  } else {
    r += ";-;-;-;-";
  }
  AddRect(r, a.Expanded(m));
  AddRect(r, a.PolarClosure());
  add(B(a.Contains(b)));
  add(B(a.InteriorContains(b)));
  add(B(a.Intersects(b)));
  add(B(a.InteriorIntersects(b)));
  AddRect(r, a.Union(b));
  AddRect(r, a.Intersection(b));
  add(B(a.ApproxEquals(b)));
  add(B(a == b));
  if (ll_ok) {
    AddRect(r, S2LatLngRect::FromCenterSize(
                   ll, S2LatLng::FromRadians(std::fabs(m.lat().radians()),
                                             std::fabs(m.lng().radians()))));
  } else {
    r += ";-;-;-;-";
  }
  const char* note = (IsCorner(a.lng()) || IsCorner(b.lng())) ? "deviation:seam-length" : "";
  std::printf("%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n", D(a.lat().lo()).c_str(),
              D(a.lat().hi()).c_str(), D(a.lng().lo()).c_str(), D(a.lng().hi()).c_str(),
              D(ll.lat().radians()).c_str(), D(ll.lng().radians()).c_str(),
              D(m.lat().radians()).c_str(), D(m.lng().radians()).c_str(),
              D(b.lat().lo()).c_str(), D(b.lat().hi()).c_str(), D(b.lng().lo()).c_str(),
              D(b.lng().hi()).c_str(), r.c_str(), note);
}

void RectOps() {
  std::printf("# RECT_OPS\n");
  for (int n = 0; n < 2000; ++n) {
    S2LatLngRect a = RandomRect(), b = RandomRect();
    S2LatLng ll = RandomAnyLatLng();
    S2LatLng m = S2LatLng::FromRadians(RandomMargin(), RandomMargin());
    RectRow(a, ll, m, b);
  }
}

void RectSpecial() {
  std::printf("# RECT_SPECIAL\n");
  const double h = M_PI_2;
  const S2LatLngRect fixtures[] = {
      S2LatLngRect::Empty(),
      S2LatLngRect::Full(),
      // At each pole: a band touching it, and a point on it.
      S2LatLngRect(R1Interval(1, h), S1Interval(0, 1)),
      S2LatLngRect(R1Interval(h, h), S1Interval(0.5, 0.5)),
      S2LatLngRect(R1Interval(-h, -1), S1Interval(-1, 0)),
      S2LatLngRect(R1Interval(-h, -h), S1Interval(-0.5, -0.5)),
      // Across the antimeridian.
      S2LatLngRect(R1Interval(-0.5, 0.5), S1Interval(3, -3)),
      S2LatLngRect(R1Interval(-0.1, 0.1), S1Interval(3.1, -3.1)),
      // Full latitudes on the antimeridian; −π normalises to π.
      S2LatLngRect(R1Interval(-h, h), S1Interval(M_PI, M_PI)),
      S2LatLngRect::FromPointPair(S2LatLng::FromRadians(-h, -M_PI),
                                  S2LatLng::FromRadians(h, -M_PI)),
      // A latitude point with full longitudes, and full latitudes at one longitude.
      S2LatLngRect(R1Interval(0, 0), S1Interval::Full()),
      S2LatLngRect(R1Interval(-h, h), S1Interval(0, 0)),
      // The seam corner as a longitude range (spec 004 FR-020).
      S2LatLngRect(R1Interval(0, 0.5), S1Interval(M_PI, kCornerHi)),
  };
  const S2LatLng points[] = {
      S2LatLng::FromRadians(h, 0),         S2LatLng::FromRadians(-h, M_PI),
      S2LatLng::FromRadians(0, M_PI),      S2LatLng::FromRadians(0, -M_PI),
      S2LatLng::FromRadians(0.05, 3.12),   S2LatLng::FromRadians(0.25, 0.5),
      S2LatLng::FromRadians(h + 0.1, 0),   S2LatLng::FromRadians(0, M_PI + 0.1),
  };
  const double margins[] = {0.0, 0.1, -0.1, h, -h, M_PI, -M_PI, 2 * M_PI, INFINITY, -INFINITY};
  int k = 0;
  for (const S2LatLngRect& a : fixtures) {
    for (const S2LatLngRect& b : fixtures) {
      const S2LatLng m = S2LatLng::FromRadians(margins[k % 10], margins[(k / 10) % 10]);
      RectRow(a, points[k % 8], m, b);
      ++k;
    }
  }
}

// ---- r2::Rect (spec 006) -------------------------------------------------------------------

// A u-v coordinate: zeros of both signs, small integers (ties), every binade below 2^1022
// (ldexp), and ordinary values in [−2, 2].
double RandomUv() {
  switch (rng() % 10) {
    case 0: return 0.0;
    case 1: return -0.0;
    case 2: return static_cast<double>(static_cast<int>(rng() % 5) - 2);
    case 3: return std::ldexp(AnyPointIn(-1, 1), static_cast<int>(rng() % 2097) - 1074);
    default: return AnyPointIn(-2, 2);
  }
}

R2Point RandomUvPoint() { return R2Point(RandomUv(), RandomUv()); }

// An empty interval: the canonical one, or two reversed bounds.
R1Interval RandomEmptyUv() {
  if (rng() % 3 == 0) return R1Interval::Empty();
  double a = RandomUv(), b = RandomUv();
  if (a == b) return R1Interval::Empty();
  return a > b ? R1Interval(a, b) : R1Interval(b, a);
}

// A valid rectangle: half through two points, the rest from two intervals that are both empty
// or both not (struct literals in the Rust harnesses).
R2Rect RandomR2() {
  if (rng() % 2 == 0) return R2Rect::FromPointPair(RandomUvPoint(), RandomUvPoint());
  switch (rng() % 5) {
    case 0: return R2Rect::Empty();
    case 1: return R2Rect(RandomEmptyUv(), RandomEmptyUv());
    case 2: return R2Rect::FromPoint(RandomUvPoint());
    default:
      return R2Rect(R1Interval::FromPointPair(RandomUv(), RandomUv()),
                    R1Interval::FromPointPair(RandomUv(), RandomUv()));
  }
}

double RandomUvMargin() {
  switch (rng() % 6) {
    case 0: return 0.0;
    case 1: return -0.0;
    case 2: return (rng() % 2 ? 1 : -1) * std::ldexp(unit(rng), -static_cast<int>(rng() % 60));
    default: return AnyPointIn(-2, 2);
  }
}

void AddR2(std::string& r, const R2Rect& x) {
  r += ';' + D(x.x().lo()) + ';' + D(x.x().hi()) + ';' + D(x.y().lo()) + ';' + D(x.y().hi());
}

bool Infinite(double d) { return std::isinf(d); }

bool AnyInfinite(const R2Rect& x) {
  return Infinite(x.x().lo()) || Infinite(x.x().hi()) || Infinite(x.y().lo()) ||
         Infinite(x.y().hi());
}

void R2Row(const R2Rect& a, const R2Point& p, const R2Point& m, double max_error,
           const R2Rect& b) {
  std::string r = B(a.is_valid());
  auto add = [&r](const std::string& s) { r += ';' + s; };
  bool nan = false;  // a result among 10..13, 22..29, 46, 47, 49..52 is NaN
  auto addn = [&](double d) {
    nan = nan || d != d;
    add(D(d));
  };
  add(B(a.is_empty()));
  for (int k = 0; k < 4; ++k) {
    add(D(a.GetVertex(k).x()));
    add(D(a.GetVertex(k).y()));
  }
  addn(a.GetCenter().x());
  addn(a.GetCenter().y());
  if (a.is_empty()) {
    r += ";-;-";
  } else {
    addn(a.GetSize().x());
    addn(a.GetSize().y());
  }
  add(B(a.Contains(p)));
  add(B(a.InteriorContains(p)));
  R2Rect ap = a;
  ap.AddPoint(p);
  AddR2(r, ap);
  if (a.is_empty()) {
    r += ";-;-";
  } else {
    add(D(a.Project(p).x()));
    add(D(a.Project(p).y()));
  }
  const R2Rect e = a.Expanded(m), e1 = a.Expanded(m.x());
  for (const R2Rect& x : {e, e1}) {
    addn(x.x().lo());
    addn(x.x().hi());
    addn(x.y().lo());
    addn(x.y().hi());
  }
  add(B(a.Contains(b)));
  add(B(a.InteriorContains(b)));
  add(B(a.Intersects(b)));
  add(B(a.InteriorIntersects(b)));
  R2Rect ar = a;
  ar.AddRect(b);
  AddR2(r, ar);
  AddR2(r, a.Union(b));
  AddR2(r, a.Intersection(b));
  const bool eq_with = a.ApproxEquals(b, max_error), eq = a.ApproxEquals(b);
  add(B(eq_with));
  add(B(eq));
  add(B(a == b));
  const R2Rect cs = R2Rect::FromCenterSize(p, R2Point(std::fabs(m.x()), std::fabs(m.y())));
  addn(cs.x().lo());
  addn(cs.x().hi());
  addn(cs.y().lo());
  addn(cs.y().hi());
  // ApproxEquals is a bool; at an infinite corner it compares through ∞ − ∞ (FR-018).
  auto nan_diff = [](double u, double v) { return (u - v) != (u - v); };
  const bool approx_nan = nan_diff(b.x().lo(), a.x().lo()) || nan_diff(b.x().hi(), a.x().hi()) ||
                          nan_diff(b.y().lo(), a.y().lo()) || nan_diff(b.y().hi(), a.y().hi());
  std::string note;
  if (!SameBits(a.x().GetCenter(), LibCenter(a.x())) ||
      !SameBits(a.y().GetCenter(), LibCenter(a.y()))) {
    note = "deviation:center-overflow";
  }
  const bool inf = AnyInfinite(a) || AnyInfinite(b) || Infinite(p.x()) || Infinite(p.y()) ||
                   Infinite(m.x()) || Infinite(m.y());
  if (inf && (nan || approx_nan)) {
    note += std::string(note.empty() ? "" : "+") + "deviation:infinite-corner";
  }
  std::printf("%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n", D(a.x().lo()).c_str(),
              D(a.x().hi()).c_str(), D(a.y().lo()).c_str(), D(a.y().hi()).c_str(),
              D(p.x()).c_str(), D(p.y()).c_str(), D(m.x()).c_str(), D(m.y()).c_str(),
              D(max_error).c_str(), D(b.x().lo()).c_str(), D(b.x().hi()).c_str(),
              D(b.y().lo()).c_str(), D(b.y().hi()).c_str(), r.c_str(), note.c_str());
}

void R2Ops() {
  std::printf("# R2RECT_OPS\n");
  for (int n = 0; n < kR2OpsRows; ++n) {
    const R2Rect a = RandomR2(), b = RandomR2();
    const R2Point p = RandomUvPoint();
    const R2Point m(RandomUvMargin(), RandomUvMargin());
    R2Row(a, p, m, RandomMaxError(), b);
  }
}

void R2Special() {
  std::printf("# R2RECT_SPECIAL\n");
  const double inf = INFINITY;
  std::vector<R2Rect> fixtures = {
      // The empty rectangle and its other representations.
      R2Rect::Empty(),
      R2Rect(R1Interval(5, -5), R1Interval(0, -1)),
      R2Rect(R1Interval(inf, -inf), R1Interval(inf, -inf)),
      R2Rect(R1Interval(1, 0), R1Interval(inf, -inf)),
      // Sides of one signed zero.
      R2Rect(R1Interval(0, 0), R1Interval(-0.0, 0)),
      R2Rect(R1Interval(0, -0.0), R1Interval(-0.0, -0.0)),
      // Upstream's r2rect_test.cc fixtures.
      R2Rect(R2Point(0, 0.25), R2Point(0.5, 0.75)),
      R2Rect(R2Point(0, 0), R2Point(0.25, 0.25)),
      R2Rect(R2Point(0.25, 0.25), R2Point(0.5, 0.5)),
      // Bounds near DBL_MAX, where 0.5 * (lo + hi) overflows.
      R2Rect(R1Interval(1e308, 1.5e308), R1Interval(-DBL_MAX, -1e308)),
  };
  // Every interval from two bounds of {−∞, −1, −0, 0, 1, ∞}; each one is an x side once and a
  // y side once, partnered with an interval of the same emptiness.
  const double bounds[] = {-inf, -1, -0.0, 0, 1, inf};
  std::vector<R1Interval> empty_sides, sides;
  for (double lo : bounds) {
    for (double hi : bounds) {
      const R1Interval side(lo, hi);
      (side.is_empty() ? empty_sides : sides).push_back(side);
    }
  }
  for (const auto* set : {&sides, &empty_sides}) {
    const size_t n = set->size();
    for (size_t i = 0; i < n; ++i) {
      fixtures.push_back(R2Rect((*set)[i], (*set)[(i * 7 + 3) % n]));
      fixtures.push_back(R2Rect((*set)[(i * 5 + 1) % n], (*set)[i]));
    }
  }
  const R2Point points[] = {R2Point(0, 0),     R2Point(-0.0, 1),  R2Point(0.5, 0.5),
                            R2Point(inf, 0),   R2Point(-1, -inf), R2Point(2, -2),
                            R2Point(1e308, 1), R2Point(0.25, 0.25)};
  const double margins[] = {0.0, 1, -1, inf, -inf};
  const double errors[] = {0.0, 1e-15, 0.5};
  int k = 0;
  const size_t n = fixtures.size();
  for (size_t i = 0; i < n; ++i) {
    for (size_t j : {i, (i + 1) % n, (i * 3 + 2) % n}) {
      R2Row(fixtures[i], points[k % 8], R2Point(margins[k % 5], margins[(k / 5) % 5]),
            errors[k % 3], fixtures[j]);
      ++k;
    }
  }
  // Every cell's u-v bound at levels 0–3 (the four faces' bounds repeat, so face 0's), each
  // against itself, its parent's bound and the next cell's along the curve.
  // child_begin(level) enumerates the cells in Hilbert order, and a cell's four children are
  // consecutive, so cell c's parent is cell c / 4 of the level above.
  std::vector<R2Rect> uv;
  for (int level = 0; level <= 3; ++level) {
    const S2CellId end = S2CellId::FromFace(0).child_end(level);
    for (S2CellId id = S2CellId::FromFace(0).child_begin(level); id != end; id = id.next()) {
      uv.push_back(S2Cell(id).GetBoundUV());
    }
  }
  int first = 0;  // index of the first bound at the current level
  for (int level = 0, count = 1; level <= 3; first += count, ++level, count *= 4) {
    const int parent_first = level == 0 ? 0 : first - count / 4;
    for (int c = 0; c < count; ++c) {
      const R2Rect& a = uv[first + c];
      const R2Rect& up = uv[parent_first + c / 4];
      const R2Rect& next = uv[first + (c + 1) % count];
      for (const R2Rect* b : {&a, &up, &next}) {
        R2Row(a, points[k % 8], R2Point(margins[k % 5], margins[(k / 5) % 5]), errors[k % 3], *b);
        ++k;
      }
    }
  }
}

// ---- s2::Point (spec 006) ------------------------------------------------------------------

std::normal_distribution<double> gauss(0.0, 1.0);

S2Point RandomUnitPoint() {
  return S2Point(gauss(rng), gauss(rng), gauss(rng)).Normalize();
}

// A partner for a: half the time nearly equal or nearly antipodal, sometimes exactly.
S2Point RandomPartner(const S2Point& a) {
  switch (rng() % 8) {
    case 0: return a;
    case 1: return -a;
    case 2: case 3: {
      const double small = std::ldexp(1.0, -static_cast<int>(10 + rng() % 51));
      const S2Point d = RandomUnitPoint() * small;
      return ((rng() % 2 ? a : -a) + d).Normalize();
    }
    default: return RandomUnitPoint();
  }
}

double RandomAngle() {
  switch (rng() % 6) {
    case 0: return 0.0;
    case 1: return M_PI;
    default: return AnyPointIn(-2 * M_PI, 2 * M_PI);
  }
}

bool Finite(const S2Point& p) {
  return std::isfinite(p.x()) && std::isfinite(p.y()) && std::isfinite(p.z());
}

void AddPoint3(std::string& r, const S2Point& p) {
  r += ';' + D(p.x()) + ';' + D(p.y()) + ';' + D(p.z());
}

// Which step of RobustCrossProd answers (s2edge_crossings.cc:147–174).
bool TakesLongDoubleStep(const S2Point& a, const S2Point& b) {
  Vector3_d r;
  if (S2::internal::GetStableCrossProd(a, b, &r) || a == b) return false;
  Vector3<long double> r_ld;
  return S2::internal::GetStableCrossProd(
      Vector3<long double>(a.x(), a.y(), a.z()), Vector3<long double>(b.x(), b.y(), b.z()),
      &r_ld);
}

void PointRow(const S2Point& a, const S2Point& b, double angle) {
  std::string r = B(a < b);
  auto add = [&r](const std::string& s) { r += ';' + s; };
  const std::string dash = "-";
  add(B(b < a));
  add(B(a == b));
  add(B(S2::IsUnitLength(a)));
  AddPoint3(r, a.Normalize());
  std::string note;
  auto flag = [&note](const char* s) { note += std::string(note.empty() ? "" : "+") + s; };
  if (a.Norm2() == 0) flag("deviation:normalize-zero-sign");
  const bool finite = Finite(a) && Finite(b);
  const bool unit = finite && S2::IsUnitLength(a) && S2::IsUnitLength(b);
  if (finite) {
    AddPoint3(r, S2::Ortho(a));
  } else {
    r += ";-;-;-";
  }
  if (unit) {
    const S2Point x = S2::RobustCrossProd(a, b);
    AddPoint3(r, x);
    if (!Finite(x)) flag("deviation:robust-cross-overflow");
    if (TakesLongDoubleStep(a, b)) flag("deviation:robust-cross-long-double");
  } else {
    r += ";-;-;-";
  }
  add(finite ? D(S1Angle(a, b).radians()) : dash);
  add(unit ? D(S1ChordAngle(a, b).length2()) : dash);
  if (unit) {
    AddPoint3(r, S2::Rotate(a, b, S1Angle::Radians(angle)));
  } else {
    r += ";-;-;-";
  }
  std::printf("%s,%s,%s,%s,%s,%s,%s,%s,%s\n", D(a.x()).c_str(), D(a.y()).c_str(),
              D(a.z()).c_str(), D(b.x()).c_str(), D(b.y()).c_str(), D(b.z()).c_str(),
              D(angle).c_str(), r.c_str(), note.c_str());
}

void PointOps() {
  std::printf("# S2POINT_OPS\n");
  for (int n = 0; n < kPointOpsRows; ++n) {
    const S2Point a = RandomUnitPoint();
    const S2Point b = RandomPartner(a);
    PointRow(a, b, RandomAngle());
  }
}

void PointSpecial() {
  std::printf("# S2POINT_SPECIAL\n");
  const double inf = INFINITY, nan = NAN;
  const double tiny = std::ldexp(1.0, -242);
  std::vector<S2Point> fixtures = {
      // The zero vector and its signed variants.
      S2Point(0, 0, 0), S2Point(-0.0, 0, 0), S2Point(0, -0.0, -0.0), S2Point(-0.0, -0.0, -0.0),
      // The axes, both signs, and a negative zero beside one.
      S2Point(1, 0, 0), S2Point(-1, 0, 0), S2Point(-0.0, 1, 0), S2Point(0, -1, 0),
      S2Point(0, 0, 1), S2Point(0, -0.0, -1),
      // Ties in the largest component, raw and normalised.
      S2Point(1, 1, 0), S2Point(1, -1, 1), S2Point(1, 1, 0).Normalize(),
      S2Point(1, -1, 1).Normalize(), S2Point(0.5, 0.5, 0.5),
      // Subnormal, tiny and huge coordinates.
      S2Point(std::ldexp(1.0, -1074), 0, 0), S2Point(0, -std::ldexp(1.0, -1074), 0),
      S2Point(tiny, 0, 0), S2Point(0, 0, std::nextafter(tiny, 0)), S2Point(-tiny, tiny, tiny),
      S2Point(1e-170, 1e-170, 0), S2Point(-1e-170, 0, -1e-170), S2Point(1e-160, 0, 0),
      S2Point(1e154, 0, 0), S2Point(0, 0, 1.4e154), S2Point(1e200, 1, 1),
      S2Point(0.012, 0.0053, 0.00457), S2::Origin(),
  };
  // Each fixture against itself, its antipode, the next fixture and a random unit point.
  int k = 0;
  const double angles[] = {0.0, M_PI_2, -M_PI, 1e-300, 3.0};
  for (size_t i = 0; i < fixtures.size(); ++i) {
    const S2Point& a = fixtures[i];
    for (const S2Point& b : {a, S2Point(-a), fixtures[(i + 1) % fixtures.size()],
                             RandomUnitPoint()}) {
      PointRow(a, b, angles[k % 5]);
      ++k;
    }
  }
  // Equal and antipodal unit points along the axes and off them.
  for (int n = 0; n < 20; ++n) {
    const S2Point a = RandomUnitPoint();
    PointRow(a, a, angles[n % 5]);
    PointRow(a, -a, angles[(n + 1) % 5]);
  }
  // NaN and ±∞ in each position of a and of b (results 0..6 only).
  for (double v : {nan, inf, -inf}) {
    for (int pos = 0; pos < 3; ++pos) {
      S2Point a(0.6, 0.0, 0.8), b(0.0, 1.0, -0.0);
      a[pos] = v;
      PointRow(a, b, 0.0);
      PointRow(b, a, 0.0);
    }
  }
  // U-012: RobustCrossProd(−v, v) for v = (2^-k, 0, 1), a unit point.
  for (int e : {300, 1022, 1023, 1024, 1074}) {
    const S2Point v(std::ldexp(1.0, -e), 0, 1);
    PointRow(-v, v, 0.0);
    PointRow(v, -v, 0.0);
  }
}


// ---- s1::ChordAngle and s1::Angle (spec 007) -----------------------------------------------

// Spec 007's random rows, kept small so the CSV stays under 10 MB (it is not published).
constexpr int kChordOpsRows = 900;
constexpr int kAngleOpsRows = 900;

std::string I(int32_t n) { return std::to_string(n); }

// Whether MathUtil::Round<int32_t> of v is out of range: cvtsd2si rounds to nearest even, so
// [-2147483648.5, 2147483647.5) is in range.
bool EOutOfRange(double v) {
  return !(v >= -2147483648.5 && v < 2147483647.5);
}

void ChordRow(bool infinity, double l, double m, double e) {
  const S1ChordAngle a = infinity ? S1ChordAngle::Infinity() : S1ChordAngle::FromLength2(l);
  const S1ChordAngle b = S1ChordAngle::FromLength2(m);
  std::string r = D(a.length2());
  auto add = [&r](const std::string& x) { r += ';' + x; };
  const std::string dash = "-";
  add(B(a.is_zero()));
  add(B(a.is_negative()));
  add(B(a.is_infinity()));
  add(B(a.is_special()));
  add(B(a.is_valid()));
  add(D(a.Successor().length2()));
  add(D(a.Predecessor().length2()));
  add(D(a.PlusError(e).length2()));
  add(D(a.GetS2PointConstructorMaxError()));
  add(D(a.GetS1AngleConstructorMaxError()));
  const bool special = a.is_special();
  add(special ? dash : D(sin2(a)));
  add(special ? dash : D(cos(a)));
  add(special ? dash : D(sin(a)));
  add(special ? dash : D(tan(a)));
  const S1Angle angle = a.ToAngle();
  add(D(angle.radians()));
  std::string note;
  auto flag = [&note](const char* x) { note += std::string(note.empty() ? "" : "+") + x; };
  bool saturate = false;
  for (double k : {1e5, 1e6, 1e7}) saturate = saturate || EOutOfRange(k * angle.degrees());
  add(I(a.e5()));
  add(I(a.e6()));
  add(I(a.e7()));
  const bool sum = !special && !b.is_special();
  add(sum ? D((a + b).length2()) : dash);
  add(sum ? D((a - b).length2()) : dash);
  add(B(a < b));
  add(B(a == b));
  add(D(S1ChordAngle(S1Angle::Radians(l)).length2()));
  if (l < 0 && l != -1.0) flag("deviation:negative-canonical");
  if (saturate) flag("deviation:eform-saturate");
  std::printf("%s,%s,%s,%s,%s,%s\n", infinity ? "I" : "L", D(l).c_str(), D(m).c_str(),
              D(e).c_str(), r.c_str(), note.c_str());
}

// A length in [0, 4]: uniform, or the squared chord between two random unit points.
double RandomLength() {
  if (rng() % 2) return 4.0 * unit(rng);
  return S1ChordAngle(RandomUnitPoint(), RandomUnitPoint()).length2();
}

double RandomError() {
  switch (rng() % 6) {
    case 0: return 0.0;
    case 1: return DBL_EPSILON;
    case 2: return -DBL_EPSILON;
    case 3: return 1e-15;
    default: return std::ldexp(AnyPointIn(-1, 1), -static_cast<int>(rng() % 60));
  }
}

void ChordOps() {
  std::printf("# CHORD_ANGLE\n");
  for (int n = 0; n < kChordOpsRows; ++n) {
    ChordRow(false, RandomLength(), RandomLength(), RandomError());
  }
  const double inf = INFINITY, nan = NAN;
  const double tiny = std::ldexp(1.0, -1074);
  const double specials[] = {-1.0, -5.0, -0.001, -inf, -0.0, 0.0, tiny, 2.0, 4.0,
                             std::nextafter(4.0, 5.0), 5.0, inf, nan};
  const double partners[] = {0.0, 1.0, 4.0, -1.0, inf};
  const double errors[] = {0.0, DBL_EPSILON, -1e-15, 1.0, -1.0, inf, -inf, nan};
  int k = 0;
  for (double l : specials) {
    for (double m : partners) {
      ChordRow(false, l, m, errors[k % 8]);
      ++k;
    }
    ChordRow(false, l, RandomLength(), errors[k % 8]);
    ++k;
  }
  // The Infinity() sentinel (U-014: its E-forms).
  for (double m : partners) {
    ChordRow(true, inf, m, errors[k % 8]);
    ++k;
  }
}

void AngleRow(double r, double s, int32_t n) {
  const S1Angle a = S1Angle::Radians(r);
  std::string res = D(S1Angle::Degrees(r).radians());
  auto add = [&res](const std::string& x) { res += ';' + x; };
  add(D(a.degrees()));
  bool saturate = false;
  for (double k : {1e5, 1e6, 1e7}) saturate = saturate || EOutOfRange(k * a.degrees());
  add(I(a.e5()));
  add(I(a.e6()));
  add(I(a.e7()));
  add(D(a.Normalized().radians()));
  add(B(a.IsNormalized()));
  add(D(a.abs().radians()));
  add(D(sin(a)));
  add(D(cos(a)));
  add(D(tan(a)));
  const S1Angle b = S1Angle::Radians(s);
  add(D((a + b).radians()));
  add(D((a - b).radians()));
  add(D((a * s).radians()));
  add(D((a / s).radians()));
  add(D(a / b));
  add(D(S1Angle::E5(n).radians()));
  add(D(S1Angle::E6(n).radians()));
  add(D(S1Angle::E7(n).radians()));
  uint32_t u;
  std::memcpy(&u, &n, sizeof u);
  add(D(S1Angle::UnsignedE6(u).radians()));
  add(D(S1Angle::UnsignedE7(u).radians()));
  std::printf("%s,%s,%s,%s,%s\n", D(r).c_str(), D(s).c_str(), I(n).c_str(), res.c_str(),
              saturate ? "deviation:eform-saturate" : "");
}

// An angle across magnitudes 1e-300 to 1e300, both signs, or one within a turn.
double RandomRadians() {
  if (rng() % 3 == 0) return AnyPointIn(-2 * M_PI, 2 * M_PI);
  const double mag = std::pow(10.0, AnyPointIn(-300, 300));
  return rng() % 2 ? mag : -mag;
}

int32_t RandomInt32() {
  switch (rng() % 5) {
    case 0: return static_cast<int32_t>(rng() % 2000001) - 1000000;
    default: return static_cast<int32_t>(static_cast<uint32_t>(rng()));
  }
}

void AngleOps() {
  std::printf("# ANGLE\n");
  for (int n = 0; n < kAngleOpsRows; ++n) {
    AngleRow(RandomRadians(), RandomRadians(), RandomInt32());
  }
  const double inf = INFINITY, nan = NAN;
  // The radians whose degrees * 1e6 or * 1e7 is exactly a half integer (the survey probe,
  // specs/007-formal-verify-chord-angle-angle/probe/cpp/survey_cpp.txt).
  const uint64_t ties[] = {0x3e42bd8646c1beedULL, 0x3e5c1c496a229e64ULL, 0x3e676ce7d8722ea9ULL,
                           0x3e7515370f99f6cbULL, 0x3e79c498a14a6686ULL, 0x3e7e73fa32fad641ULL,
                           0xbe0dfc0a0acf97e2ULL, 0xbe267d07881bb1e9ULL, 0xbe32bd8646c1beedULL};
  std::vector<double> specials = {M_PI, -M_PI, 3 * M_PI, -3 * M_PI, 5 * M_PI, 2 * M_PI, 7.0,
                                  1e6, 1e16, 1e20, 1e300, -1e20, -2.462748394469102e16,
                                  0.0, -0.0, inf, -inf, nan, 1e10, -1e10};
  for (uint64_t t : ties) specials.push_back(FromBits(t));
  const int32_t ints[] = {INT32_MIN, INT32_MAX, 0, 1, -1, 1800000000, -1800000000};
  const double partners[] = {0.0, 1.0, -0.0, inf, M_PI};
  int k = 0;
  for (double r : specials) {
    for (double s : partners) {
      AngleRow(r, s, ints[k % 7]);
      ++k;
    }
  }
}

}  // namespace

int main() {
  std::printf(
      "# Generated by tools/refgen-cpp/generate_values (oracle: google/s2geometry "
      "v0.14.0-40-g37a4501). Do not edit.\n");
  IntervalOps();
  IntervalSpecial();
  CellFields();
  CellChildren();
  // Spec 005: appended, so every earlier line stays byte-identical.
  R1Ops();
  R1Special();
  RectOps();
  RectSpecial();
  // Spec 006: appended, so every earlier line stays byte-identical.
  R2Ops();
  R2Special();
  PointOps();
  PointSpecial();
  // Spec 007: appended, so every earlier line stays byte-identical.
  ChordOps();
  AngleOps();
  return 0;
}
