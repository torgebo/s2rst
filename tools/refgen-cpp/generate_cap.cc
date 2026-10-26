// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
//
// Reference-vector generator for the s2rst cap S2Cap (specs/008-formal-verify-cap, FR-024,
// research R9), using the C++ reference implementation (google/s2geometry) as the oracle. Emits
// CSV that core/tests/cpp_cap_diff.rs diffs the library against, and that
// verify/tests/values_spec_vs_cpp.rs checks the reference description against. A file of its
// own: cpp_values_vectors.csv is already 9.6 MB. Deterministic (fixed seed). See
// tools/refgen-cpp/README.md.
//
// Every double is the 16-hex-digit bit pattern of its value; a bool is 0 or 1; a cell id is its
// 16-hex-digit value; a list of ids is space-separated.
//
// # CAP_OPS, # CAP_SPECIAL
//   cx,cy,cz,l,px,py,pz,bx,by,bz,bl,d,h,a,ang,results,note
//   The cap a = S2Cap(c, FromLength2(l)) (or Empty() for l = −1); p a unit point; the cap
//   b = S2Cap((bx, by, bz), FromLength2(bl)); d ≥ 0 an expansion in radians; h a height, a an
//   area and ang an angle (radians) for the three constructors around c.
//   results is ';'-separated, in this order:
//     0 is_valid  1 is_empty  2 is_full  3 height  4 GetArea  5..7 GetCentroid
//     8 Contains(p)  9 InteriorContains(p)  10 Contains(b)  11 Intersects(b)
//     12 InteriorIntersects(b)  13..15 Complement center  16 Complement length2
//     17 Expanded(d) length2  18..20 Union(b) center  21 Union(b) length2
//     22 AddPoint(p) length2  23..25 AddPoint(p) center  26 AddCap(b) length2
//     27..29 AddCap(b) center  30 ApproxEquals(b)  31 ApproxEquals(b, 1e-10)  32 a == b
//     33 FromCenterHeight(c, h) length2  34 FromCenterArea(c, a) length2
//     35 S2Cap(c, S1Angle(ang)) length2  36 GetRadius().radians()
//   note is empty, or the special case's name in CAP_SPECIAL.
//
// # CAP_BOUNDS
//   cx,cy,cz,l,results,note
//   results: 0..3 GetRectBound() lat.lo, lat.hi, lng.lo, lng.hi  4 GetCellUnionBound() ids
//   5 S2RegionCoverer (max_cells 8) covering ids
//
// # CAP_CELL
//   cx,cy,cz,l,cell,results,note
//   results: 0 Contains(S2Cell(cell))  1 MayIntersect(S2Cell(cell))

#include <cfloat>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <random>
#include <string>
#include <vector>

#include "s2/s1angle.h"
#include "s2/s1chord_angle.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2metrics.h"
#include "s2/s2point.h"
#include "s2/s2region_coverer.h"

namespace {

std::mt19937_64 rng(0x43415053ULL);  // "CAPS"
std::uniform_real_distribution<double> unit(0.0, 1.0);

constexpr int kOpsRows = 1000;
constexpr int kBoundsRows = 600;
constexpr int kCellRows = 1000;

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

std::string D(double d) { return Hex64(Bits(d)); }
std::string B(bool b) { return b ? "1" : "0"; }
std::string P(const S2Point& p) { return D(p.x()) + ',' + D(p.y()) + ',' + D(p.z()); }
std::string PS(const S2Point& p) { return D(p.x()) + ';' + D(p.y()) + ';' + D(p.z()); }

std::string Ids(const std::vector<S2CellId>& ids) {
  std::string s;
  for (const S2CellId& id : ids) {
    if (!s.empty()) s += ' ';
    s += Hex64(id.id());
  }
  return s;
}

double In(double lo, double hi) { return lo + (hi - lo) * unit(rng); }

S2Point LL(double lat_deg, double lng_deg) {
  return S2LatLng::FromDegrees(lat_deg, lng_deg).ToPoint();
}

// A unit point uniform on the sphere.
S2Point RandomPoint() {
  return S2LatLng::FromRadians(std::asin(In(-1, 1)), In(-M_PI, M_PI)).ToPoint();
}

// A point at angle t (radians) from c, in a random direction.
S2Point PointAt(const S2Point& c, double t) {
  S2Point u = S2::Ortho(c);
  S2Point v = c.CrossProd(u).Normalize();
  double phi = In(0, 2 * M_PI);
  S2Point dir = (u * std::cos(phi) + v * std::sin(phi)).Normalize();
  return (c * std::cos(t) + dir * std::sin(t)).Normalize();
}

// A valid cap's radius as a length²: uniform in [0, 4], from an angle across magnitudes, or one
// of empty, zero, full.
double RandomLength2() {
  switch (rng() % 8) {
    case 0: return -1.0;
    case 1: return 0.0;
    case 2: return 4.0;
    case 3: return In(0, 4);
    default: {
      double t = std::pow(10.0, In(-9, std::log10(M_PI)));
      return S1ChordAngle(S1Angle::Radians(t)).length2();
    }
  }
}

S2Cap MakeCap(const S2Point& c, double l) {
  if (l < 0) return S2Cap(c, S1ChordAngle::Negative());
  return S2Cap(c, S1ChordAngle::FromLength2(l));
}

void OpsRow(const S2Point& c, double l, const S2Point& p, const S2Point& bc, double bl,
            double d, double h, double area, double ang, const char* note) {
  const S2Cap a = MakeCap(c, l), b = MakeCap(bc, bl);
  std::string res = B(a.is_valid());
  auto add = [&res](const std::string& x) { res += ';' + x; };
  add(B(a.is_empty()));
  add(B(a.is_full()));
  add(D(a.height()));
  add(D(a.GetArea()));
  add(PS(a.GetCentroid()));
  add(B(a.Contains(p)));
  add(B(a.InteriorContains(p)));
  add(B(a.Contains(b)));
  add(B(a.Intersects(b)));
  add(B(a.InteriorIntersects(b)));
  const S2Cap k = a.Complement();
  add(PS(k.center()));
  add(D(k.radius().length2()));
  add(D(a.Expanded(S1Angle::Radians(d)).radius().length2()));
  const S2Cap u = a.Union(b);
  add(PS(u.center()));
  add(D(u.radius().length2()));
  S2Cap ap = a;
  ap.AddPoint(p);
  add(D(ap.radius().length2()));
  add(PS(ap.center()));
  S2Cap ac = a;
  ac.AddCap(b);
  add(D(ac.radius().length2()));
  add(PS(ac.center()));
  add(B(a.ApproxEquals(b)));
  add(B(a.ApproxEquals(b, S1Angle::Radians(1e-10))));
  add(B(a == b));
  add(D(S2Cap::FromCenterHeight(c, h).radius().length2()));
  add(D(S2Cap::FromCenterArea(c, area).radius().length2()));
  add(D(S2Cap(c, S1Angle::Radians(ang)).radius().length2()));
  add(D(a.GetRadius().radians()));
  std::printf("%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n", P(c).c_str(), D(l).c_str(), P(p).c_str(),
              P(bc).c_str(), D(bl).c_str(), D(d).c_str(), D(h).c_str(), D(area).c_str(),
              D(ang).c_str(), res.c_str(), note);
}

void RandomOpsRow(const S2Point& c, double l, const char* note) {
  const S2Cap a = MakeCap(c, l);
  // p: half near the boundary, half anywhere.
  S2Point p = rng() % 2 ? PointAt(c, a.GetRadius().radians() * In(0.5, 1.5)) : RandomPoint();
  if (a.is_empty()) p = RandomPoint();
  S2Point bc = rng() % 3 ? PointAt(c, In(0, M_PI)) : RandomPoint();
  OpsRow(c, l, p, bc, RandomLength2(), std::pow(10.0, In(-12, 0.5)), In(-0.5, 2.5),
         In(-1, 4 * M_PI + 1), In(-0.5, M_PI + 0.5), note);
}

void CapOps() {
  std::printf("# CAP_OPS\n");
  for (int n = 0; n < kOpsRows; ++n) RandomOpsRow(RandomPoint(), RandomLength2(), "");
}

void CapSpecial() {
  std::printf("# CAP_SPECIAL\n");
  const S2Point x(1, 0, 0);
  // Empty, zero, full, the hemisphere, against identical and antipodal partners.
  for (double l : {-1.0, 0.0, 2.0, 4.0, 1.0, 3.9}) {
    for (double bl : {-1.0, 0.0, 2.0, 4.0, 1.0}) {
      OpsRow(x, l, x, x, bl, 0.1, 1.0, 1.0, 1.0, "identical-centres");
      OpsRow(x, l, -x, -x, bl, 0.1, 1.0, 1.0, 1.0, "antipodal-centres");
    }
  }
  // K4's radii, and the constructors' clamps.
  for (double l : {0.1, 0.5, 1.0, 1.7, 2.9, 3.3}) {
    OpsRow(LL(10, 20), l, LL(10, 21), LL(11, 20), 0.2, 0.0, -1.0, -1.0, -1.0, "k4");
    OpsRow(LL(10, 20), l, LL(10, 21), LL(11, 20), 0.2, 4.0, 2.0, 4 * M_PI, M_PI, "clamps");
  }
  // K6's pair: centres 5.02e-15 rad apart.
  const S2Point k6a = LL(10, 20);
  const S2Point k6b = S2LatLng::FromRadians(S2LatLng(k6a).lat().radians() + 5e-15,
                                            S2LatLng(k6a).lng().radians())
                          .ToPoint();
  OpsRow(k6a, 1.0, k6b, k6b, 1.0, 0.0, 1.0, 1.0, 1.0, "k6");
  // Poles and the antimeridian.
  for (const S2Point& c : {S2Point(0, 0, 1), S2Point(0, 0, -1), LL(0, 180), LL(0, -180)}) {
    for (int i = 0; i < 4; ++i) RandomOpsRow(c, RandomLength2(), "poles-antimeridian");
  }
}

void BoundsRow(const S2Point& c, double l, const char* note) {
  const S2Cap a = MakeCap(c, l);
  const S2LatLngRect r = a.GetRectBound();
  std::string res = D(r.lat().lo()) + ';' + D(r.lat().hi()) + ';' + D(r.lng().lo()) + ';' +
                    D(r.lng().hi());
  std::vector<S2CellId> ids;
  a.GetCellUnionBound(&ids);
  res += ';' + Ids(ids);
  S2RegionCoverer::Options opt;
  opt.set_max_cells(8);
  S2RegionCoverer coverer(opt);
  std::vector<S2CellId> cov;
  coverer.GetCovering(a, &cov);
  res += ';' + Ids(cov);
  std::printf("%s,%s,%s,%s\n", P(c).c_str(), D(l).c_str(), res.c_str(), note);
}

void CapBounds() {
  std::printf("# CAP_BOUNDS\n");
  for (int n = 0; n < kBoundsRows; ++n) BoundsRow(RandomPoint(), RandomLength2(), "");
  // K1: radii next to each covering level's power of two.
  const double deriv = S2::kMinWidth.deriv();
  const S2Point c = LL(10, 20);
  for (int k = 1; k <= 29; ++k) {
    double r = deriv / std::ldexp(1.0, k);
    for (int i = 0; i < 3; ++i) r = std::nextafter(r, 0.0);
    for (int i = 0; i < 7; ++i) {
      BoundsRow(c, S1ChordAngle(S1Angle::Radians(r)).length2(), "k1");
      r = std::nextafter(r, 4.0);
    }
  }
  // K5's cap, the poles, the antimeridian, empty and full.
  BoundsRow(LL(0, 1e-14), S1ChordAngle(S1Angle::Radians(1e-18)).length2(), "k5");
  for (const S2Point& p : {S2Point(0, 0, 1), S2Point(0, 0, -1), LL(0, 180), LL(0, -180),
                           LL(89.9, 0), LL(-89.9, 45)}) {
    for (double l : {1e-20, 1e-6, 0.01, 0.5, 2.0}) BoundsRow(p, l, "poles-antimeridian");
  }
  BoundsRow(LL(10, 20), -1.0, "empty");
  BoundsRow(LL(10, 20), 4.0, "full");
}

void CellRow(const S2Point& c, double l, S2CellId id, const char* note) {
  const S2Cap a = MakeCap(c, l);
  const S2Cell cell(id);
  std::printf("%s,%s,%s,%s;%s,%s\n", P(c).c_str(), D(l).c_str(), Hex64(id.id()).c_str(),
              B(a.Contains(cell)).c_str(), B(a.MayIntersect(cell)).c_str(), note);
}

void CapCell() {
  std::printf("# CAP_CELL\n");
  for (int n = 0; n < kCellRows; ++n) {
    const S2Point c = RandomPoint();
    const double l = RandomLength2();
    const double r = l < 0 ? 0.1 : MakeCap(c, l).GetRadius().radians();
    const S2Point q = PointAt(c, std::min(M_PI, 2 * r * unit(rng)));
    CellRow(c, l, S2CellId(q).parent(static_cast<int>(rng() % 11)), "");
  }
  // K2's grid and K3's cap, against face cell 0.
  const S2CellId face0 = S2CellId::FromFace(0);
  for (int dlat = 0; dlat <= 12; ++dlat) {
    for (double rad : {56.0, 58.0, 60.0, 62.0, 64.0}) {
      CellRow(LL(dlat, 0), S1ChordAngle(S1Angle::Degrees(rad)).length2(), face0, "k2");
    }
  }
  CellRow(LL(0, 50), S1ChordAngle(S1Angle::Degrees(2)).length2(), face0, "k3");
}

}  // namespace

int main() {
  std::printf(
      "# Generated by tools/refgen-cpp/generate_cap (oracle: google/s2geometry "
      "v0.14.0-40-g37a4501). Do not edit.\n");
  CapOps();
  CapSpecial();
  CapBounds();
  CapCell();
  return 0;
}
