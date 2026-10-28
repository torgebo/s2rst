// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
//
// Reference-vector generator for s2rst's LatLng and for s2::Rect as a region
// (specs/010-formal-verify-latlng, FR-021, research R9), using the C++ reference implementation
// (google/s2geometry) as the oracle. Emits CSV that core/tests/cpp_latlng_diff.rs diffs the
// library against, and that verify/tests/values_spec_vs_cpp.rs checks the reference
// description's restatements against. Deterministic (fixed seed). See tools/refgen-cpp/README.md.
//
// Every double is the 16-hex-digit bit pattern of its value; a bool is 0 or 1; an integer is
// decimal; a cell id is its 16-hex-digit value.
//
// # LATLNG_CTOR
//   kind,a,b,lat,lng
//   kind is rad, deg (a, b doubles), e5, e6, e7, ue6, ue7 (a, b integers), invalid or default
//   (a, b empty); lat, lng the result's coordinates in radians.
//
// # LATLNG_POINT
//   x,y,z,lat,lng
//   S2LatLng::Latitude(p) and Longitude(p) of p = (x, y, z), which S2LatLng(p) pairs.
//
// # LATLNG_NORM
//   lat,lng,nlat,nlng
//   Normalized().
//
// # LATLNG_OPS
//   alat,alng,blat,blng,k,m,results
//   results is ';'-separated: 0..2 a.ToPoint()  3 a.GetDistance(b)  4 a.ApproxEquals(b)
//   5 a.ApproxEquals(b, S1Angle::Radians(m))  6,7 a + b  8,9 a - b  10,11 k * a  12,13 a * k
//   14 a.is_valid()  15 a == b
//
// # LATLNG_STR
//   lat,lng,s
//   ToStringInDegrees() of (lat, lng) radians; s itself contains one comma.
//
// # RECT_REGION
//   latlo,lathi,lnglo,lnghi,cell,contains,may_intersect
//   S2LatLngRect(R1Interval(latlo, lathi), S1Interval(lnglo, lnghi)) against S2Cell(cell).
//
// # RECT_CAPBOUND
//   latlo,lathi,lnglo,lnghi,cx,cy,cz,length2,plat,plng,contains,cap_contains
//   GetCapBound() of the rectangle (centre, squared chord radius), and for the point
//   p = S2LatLng::FromRadians(plat, plng).ToPoint(): Contains(p), GetCapBound().Contains(p).
//   The points lie on and next to the rectangle's boundary (spec 010 research R0).

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <random>
#include <string>
#include <vector>

#include "s2/r1interval.h"
#include "s2/s1angle.h"
#include "s2/s1interval.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2point.h"

namespace {

std::mt19937_64 rng(0x4c41544cULL);  // "LATL"
std::uniform_real_distribution<double> unit(0.0, 1.0);

constexpr double kInf = std::numeric_limits<double>::infinity();
constexpr double kNaN = std::numeric_limits<double>::quiet_NaN();

std::string Hex64(uint64_t x) {
  char buf[17];
  std::snprintf(buf, sizeof buf, "%016llx", static_cast<unsigned long long>(x));
  return buf;
}

std::string D(double d) {
  uint64_t b;
  std::memcpy(&b, &d, sizeof b);
  return Hex64(b);
}

std::string B(bool b) { return b ? "1" : "0"; }

double In(double lo, double hi) { return lo + (hi - lo) * unit(rng); }

// A double across magnitudes: sign, a decade in [-lo, hi], a mantissa.
double Wide(int lo, int hi) {
  double x = In(1, 10) * std::pow(10.0, static_cast<int>(rng() % (lo + hi + 1)) - lo);
  return (rng() % 2) ? -x : x;
}

double Step(double x, int k) {
  for (int i = 0; i < std::abs(k); ++i) x = std::nextafter(x, k > 0 ? kInf : -kInf);
  return x;
}

void LL(const S2LatLng& ll) { std::printf("%s,%s\n", D(ll.lat().radians()).c_str(),
                                          D(ll.lng().radians()).c_str()); }

// ---- LATLNG_CTOR ----

void Ctor() {
  std::printf("# LATLNG_CTOR\n");
  const double specials[] = {0.0, -0.0, 1.0, -1.0, M_PI, -M_PI, M_PI_2, 2 * M_PI, 180.0, 90.0,
                             1e300, -1e-300, 5e-324, kInf, -kInf, kNaN};
  std::vector<double> ds(std::begin(specials), std::end(specials));
  for (int i = 0; i < 200; ++i) ds.push_back(Wide(10, 10));
  for (int i = 0; i < 200; ++i) ds.push_back(In(-400, 400));
  for (size_t i = 0; i < ds.size(); ++i) {
    double a = ds[i], b = ds[(i * 7 + 3) % ds.size()];
    std::printf("rad,%s,%s,", D(a).c_str(), D(b).c_str());
    LL(S2LatLng::FromRadians(a, b));
    std::printf("deg,%s,%s,", D(a).c_str(), D(b).c_str());
    LL(S2LatLng::FromDegrees(a, b));
  }
  const int32_t ints[] = {0, 1, -1, 2147483647, -2147483647 - 1, 18000000, -18000000,
                          9000000, 1800000000, -900000000};
  std::vector<int64_t> is(std::begin(ints), std::end(ints));
  for (int i = 0; i < 200; ++i) is.push_back(static_cast<int32_t>(rng()));
  for (size_t i = 0; i < is.size(); ++i) {
    int32_t a = static_cast<int32_t>(is[i]), b = static_cast<int32_t>(is[(i * 5 + 1) % is.size()]);
    std::printf("e5,%d,%d,", a, b);
    LL(S2LatLng::FromE5(a, b));
    std::printf("e6,%d,%d,", a, b);
    LL(S2LatLng::FromE6(a, b));
    std::printf("e7,%d,%d,", a, b);
    LL(S2LatLng::FromE7(a, b));
    uint32_t ua = static_cast<uint32_t>(a), ub = static_cast<uint32_t>(b);
    std::printf("ue6,%u,%u,", ua, ub);
    LL(S2LatLng::FromUnsignedE6(ua, ub));
    std::printf("ue7,%u,%u,", ua, ub);
    LL(S2LatLng::FromUnsignedE7(ua, ub));
  }
  std::printf("invalid,,,");
  LL(S2LatLng::Invalid());
  std::printf("default,,,");
  LL(S2LatLng());
}

// ---- LATLNG_POINT ----

void PointRow(double x, double y, double z) {
  S2Point p(x, y, z);
  std::printf("%s,%s,%s,%s,%s\n", D(x).c_str(), D(y).c_str(), D(z).c_str(),
              D(S2LatLng::Latitude(p).radians()).c_str(),
              D(S2LatLng::Longitude(p).radians()).c_str());
}

void Points() {
  std::printf("# LATLNG_POINT\n");
  const double v[] = {0.0, -0.0, 1.0, -1.0, 5e-324, -5e-324, 1e-320, 1e300, -1e300,
                      std::numeric_limits<double>::max(), kInf, -kInf, kNaN};
  for (double x : v)
    for (double y : v)
      for (double z : {0.0, -0.0, 1.0, -1.0, kInf, kNaN}) PointRow(x, y, z);
  for (int i = 0; i < 3000; ++i) {
    S2Point p = S2LatLng::FromRadians(std::asin(In(-1, 1)), In(-M_PI, M_PI)).ToPoint();
    PointRow(p.x(), p.y(), p.z());
  }
  for (int i = 0; i < 1000; ++i) PointRow(Wide(300, 300), Wide(300, 300), Wide(300, 300));
  // Near the poles and the antimeridian.
  for (int i = 0; i < 500; ++i) {
    double t = std::pow(10.0, In(-17, -1));
    PointRow(t * In(-1, 1), t * In(-1, 1), (rng() % 2) ? 1.0 : -1.0);
    PointRow(-1.0, (rng() % 2 ? -t : t) * In(0, 1), In(-1, 1));
  }
}

// ---- LATLNG_NORM ----

void NormRow(double lat, double lng) {
  std::printf("%s,%s,", D(lat).c_str(), D(lng).c_str());
  LL(S2LatLng::FromRadians(lat, lng).Normalized());
}

void Norm() {
  std::printf("# LATLNG_NORM\n");
  const double specials[] = {M_PI, -M_PI, 3 * M_PI, -3 * M_PI, 5 * M_PI, -5 * M_PI, 7 * M_PI,
                             -7 * M_PI, 2 * M_PI, -2 * M_PI, M_PI + 1e-15, 1e10, 1e16,
                             -2.462748394469102e16, 1e300, 0.0, -0.0, 5e-324,
                             std::numeric_limits<double>::max(), kInf, -kInf, kNaN};
  for (double lng : specials) NormRow(0.3, lng);
  for (double lat : {2.0, -2.0, M_PI_2, -M_PI_2, Step(M_PI_2, 1), -0.0, 1e300, kInf, -kInf,
                     kNaN})
    NormRow(lat, 1.0);
  for (double scale : {4.0, 100.0, 1e6, 1e12, 1e16, 1e300})
    for (int k = 0; k < 500; ++k) NormRow(In(-4, 4), In(-scale, scale));
}

// ---- LATLNG_OPS ----

S2LatLng RandomValid() {
  return S2LatLng::FromRadians(In(-M_PI_2, M_PI_2), In(-M_PI, M_PI));
}

void OpsRow(const S2LatLng& a, const S2LatLng& b, double k, double m) {
  std::string res;
  auto add = [&res](const std::string& x) {
    if (!res.empty()) res += ';';
    res += x;
  };
  S2Point p = a.ToPoint();
  add(D(p.x()));
  add(D(p.y()));
  add(D(p.z()));
  add(D(a.GetDistance(b).radians()));
  add(B(a.ApproxEquals(b)));
  add(B(a.ApproxEquals(b, S1Angle::Radians(m))));
  for (const S2LatLng& r : {a + b, a - b, k * a, a * k}) {
    add(D(r.lat().radians()));
    add(D(r.lng().radians()));
  }
  add(B(a.is_valid()));
  add(B(a == b));
  std::printf("%s,%s,%s,%s,%s,%s,%s\n", D(a.lat().radians()).c_str(),
              D(a.lng().radians()).c_str(), D(b.lat().radians()).c_str(),
              D(b.lng().radians()).c_str(), D(k).c_str(), D(m).c_str(), res.c_str());
}

void Ops() {
  std::printf("# LATLNG_OPS\n");
  for (int i = 0; i < 1500; ++i) {
    S2LatLng a = RandomValid(), b = RandomValid();
    OpsRow(a, b, Wide(5, 5), std::pow(10.0, In(-17, 0)));
  }
  // Close pairs, for ApproxEquals and the small distances.
  for (int i = 0; i < 500; ++i) {
    S2LatLng a = RandomValid();
    double e = std::pow(10.0, In(-17, -10));
    S2LatLng b = S2LatLng::FromRadians(a.lat().radians() + e * In(-2, 2),
                                       a.lng().radians() + e * In(-2, 2));
    OpsRow(a, b, In(-2, 2), (rng() % 2) ? 1e-15 : e);
    OpsRow(a, a, 1.0, 0.0);
  }
  // Out-of-range pairs (the survey's), the poles and the antimeridian.
  const double bad[][4] = {{2, 0, 0, 0},  {2, 0, 0, 1},       {-2, 0.5, 1, 2},
                           {0, 4, 0, -4}, {3.124793493547795, 0, 0.0167991600419979, M_PI},
                           {M_PI_2, 0, -M_PI_2, 0}, {0, M_PI, 0, -M_PI}, {-0.0, -0.0, 0.0, 0.0},
                           {M_PI_2, 1, M_PI_2, -2}};
  for (const auto& r : bad) {
    OpsRow(S2LatLng::FromRadians(r[0], r[1]), S2LatLng::FromRadians(r[2], r[3]), -1.0, 1e-15);
  }
  for (int i = 0; i < 250; ++i) {
    S2LatLng a = S2LatLng::FromRadians(In(-4, 4), In(-8, 8));
    S2LatLng b = S2LatLng::FromRadians(In(-4, 4), In(-8, 8));
    OpsRow(a, b, Wide(3, 3), std::pow(10.0, In(-17, 0)));
  }
}

// ---- LATLNG_STR ----

void Str() {
  std::printf("# LATLNG_STR\n");
  auto row = [](double lat, double lng) {
    std::printf("%s,%s,%s\n", D(lat).c_str(), D(lng).c_str(),
                S2LatLng::FromRadians(lat, lng).ToStringInDegrees().c_str());
  };
  const double specials[][2] = {{0.3, M_PI}, {0.3, -M_PI}, {-0.0, -0.0}, {0.0, -0.0},
                                {-0.0, 0.0}, {1e-9, -2 * M_PI}, {2.0, 4.0}, {1e-7, 3e-7},
                                {M_PI_2, 3 * M_PI}};
  for (const auto& s : specials) row(s[0], s[1]);
  for (int i = 0; i < 300; ++i) row(In(-2, 2), In(-10, 10));
}

// ---- RECT_REGION, RECT_CAPBOUND ----

S2LatLngRect RandomRect() {
  double a = In(-M_PI_2, M_PI_2), c = In(-M_PI_2, M_PI_2);
  return S2LatLngRect(R1Interval(std::min(a, c), std::max(a, c)),
                      S1Interval(In(-M_PI, M_PI), In(-M_PI, M_PI)));
}

// The families of spec 010's planning probe R0: near either pole, the north pole alone, a
// single latitude, symmetric about the equator; longitudes full, narrow or random.
S2LatLngRect FamilyRect() {
  int k = static_cast<int>(rng() % 17);
  double lo, hi;
  switch (rng() % 6) {
    case 0: {
      double a = In(-M_PI_2, M_PI_2), c = In(-M_PI_2, M_PI_2);
      lo = std::min(a, c);
      hi = std::max(a, c);
      break;
    }
    case 1:
      lo = M_PI_2 - unit(rng) * std::pow(10.0, -k);
      hi = M_PI_2;
      break;
    case 2:
      lo = -M_PI_2;
      hi = -M_PI_2 + unit(rng) * std::pow(10.0, -k);
      break;
    case 3:
      lo = hi = M_PI_2;
      break;
    case 4:
      lo = hi = In(-M_PI_2, M_PI_2);
      break;
    default:
      hi = In(0, M_PI_2);
      lo = -hi;
  }
  S1Interval lng;
  switch (rng() % 4) {
    case 0:
      lng = S1Interval::Full();
      break;
    case 1: {
      double c = In(-M_PI, M_PI);
      double w = unit(rng) * std::pow(10.0, -static_cast<int>(rng() % 12));
      lng = S1Interval::FromPointPair(c, std::min(c + w, M_PI));
      break;
    }
    default:
      lng = S1Interval(In(-M_PI, M_PI), In(-M_PI, M_PI));
  }
  return S2LatLngRect(R1Interval(lo, hi), lng);
}

S2Cell RandomCell(int max_level) {
  int face = static_cast<int>(rng() % 6);
  uint64_t pos = rng() & ((uint64_t{1} << 61) - 1);
  int level = static_cast<int>(rng() % (max_level + 1));
  return S2Cell(S2CellId::FromFacePosLevel(face, pos, level));
}

std::string Rect4(const S2LatLngRect& r) {
  return D(r.lat().lo()) + ',' + D(r.lat().hi()) + ',' + D(r.lng().lo()) + ',' +
         D(r.lng().hi());
}

void Region() {
  std::printf("# RECT_REGION\n");
  for (int i = 0; i < 10000; ++i) {
    S2LatLngRect r = (i % 2) ? RandomRect() : FamilyRect();
    S2Cell c = RandomCell(12);
    std::printf("%s,%s,%s,%s\n", Rect4(r).c_str(), Hex64(c.id().id()).c_str(),
                B(r.Contains(c)).c_str(), B(r.MayIntersect(c)).c_str());
  }
}

double LngAt(const S2LatLngRect& r, double t) {
  if (r.lng().is_inverted()) {
    double v = r.lng().lo() + t * r.lng().GetLength();
    return v > M_PI ? v - 2 * M_PI : v;
  }
  return r.lng().lo() + t * (r.lng().hi() - r.lng().lo());
}

void CapBoundRows(const S2LatLngRect& r, const std::vector<S2LatLng>& pts) {
  S2Cap cap = r.GetCapBound();
  std::string head = Rect4(r) + ',' + D(cap.center().x()) + ',' + D(cap.center().y()) + ',' +
                     D(cap.center().z()) + ',' + D(cap.radius().length2());
  for (const S2LatLng& ll : pts) {
    S2Point p = ll.ToPoint();
    std::printf("%s,%s,%s,%s,%s\n", head.c_str(), D(ll.lat().radians()).c_str(),
                D(ll.lng().radians()).c_str(), B(r.Contains(p)).c_str(),
                B(cap.Contains(p)).c_str());
  }
}

void CapBound() {
  std::printf("# RECT_CAPBOUND\n");
  // R0's two worst cases (probe/cpp/capbound_cases.cc).
  CapBoundRows(S2LatLngRect(R1Interval(M_PI_2, M_PI_2), S1Interval(-M_PI, M_PI)),
               {S2LatLng::FromRadians(1.5707963267948968, 4.6539129845781746e-1)});
  CapBoundRows(S2LatLngRect(R1Interval(1.0712215270180785, 1.0712376688984673),
                            S1Interval(-8.905155217671079e-1, -8.905155217089906e-1)),
               {S2LatLng::FromRadians(1.0712215270180785, -8.905155217089911e-1)});
  CapBoundRows(S2LatLngRect::Empty(), {S2LatLng::FromRadians(0, 0)});
  CapBoundRows(S2LatLngRect::Full(), {S2LatLng::FromRadians(M_PI_2, 0)});
  for (int i = 0; i < 1000; ++i) {
    S2LatLngRect r = (i % 3) ? FamilyRect() : RandomRect();
    if (r.is_empty()) continue;
    int k = 1 + static_cast<int>(rng() % 16);
    double lat = In(r.lat().lo(), r.lat().hi());
    std::vector<S2LatLng> pts = {
        S2LatLng::FromRadians(r.lat().lo(), LngAt(r, unit(rng))),
        S2LatLng::FromRadians(r.lat().hi(), LngAt(r, unit(rng))),
        S2LatLng::FromRadians(Step(r.lat().lo(), 1), r.lng().lo()),
        S2LatLng::FromRadians(r.lat().lo(), Step(r.lng().hi(), -k)),
        S2LatLng::FromRadians(r.lat().hi(), Step(r.lng().lo(), k)),
        S2LatLng::FromRadians(lat, LngAt(r, unit(rng))),
    };
    CapBoundRows(r, pts);
  }
}

}  // namespace

int main() {
  std::printf(
      "# Generated by tools/refgen-cpp/generate_latlng (oracle: google/s2geometry "
      "v0.14.0-40-g37a4501). Do not edit.\n");
  Ctor();
  Points();
  Norm();
  Ops();
  Str();
  Region();
  CapBound();
  return 0;
}
