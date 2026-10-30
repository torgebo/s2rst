// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
//
// Reference-vector generator for s2rst's Cell bounds against the points a cell contains, and
// for RegionUnion and RegionIntersection (specs/012-formal-verify-cell-bounds-adaptors, FR-018,
// research R6), using the C++ reference implementation (google/s2geometry) as the oracle.
// Emits CSV that core/tests/cpp_cell_bounds_diff.rs diffs the library against. Deterministic
// (fixed seed). See tools/refgen-cpp/README.md.
//
// Every double is the 16-hex-digit bit pattern of its value; a bool is 0 or 1; a cell id is its
// 16-hex-digit value.
//
// # CELL_BOUNDS
//   cell,cx,cy,cz,length2,latlo,lathi,lnglo,lnghi
//   S2Cell(cell).GetCapBound() and GetRectBound().
//
// # CELL_BOUND_POINTS
//   cell,x,y,z,contains,cap_contains,rect_contains
//   For the point p = (x, y, z): Contains(p), GetCapBound().Contains(p), GetRectBound().Contains(p).
//   The points are the cell's u-v corners and edge midpoints moved outward by 0, 1 and
//   (5/3 − 0.001) DBL_EPSILON, normalised, and two interior points (U-017).
//
// # REGION_UNION, REGION_INTERSECTION
//   members,cell,x,y,z,cx,cy,cz,length2,latlo,lathi,lnglo,lnghi,cover,contains,may_intersect,contains_point
//   members is ';'-separated, each one of `c:x:y:z:radians` (an S2Cap from an S1ChordAngle of the
//   radians), `r:latlo:lathi:lnglo:lnghi` (an S2LatLngRect), `e:cell` (an S2Cell) or `p:x:y:z`
//   (an S2PointRegion); cover is GetCellUnionBound()'s size; the tests are on S2Cell(cell) and
//   (x, y, z).

#include <algorithm>
#include <cfloat>
#include <cinttypes>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <memory>
#include <random>
#include <string>
#include <vector>

#include "s2/r2rect.h"
#include "s2/s1chord_angle.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2coords.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2point_region.h"
#include "s2/s2region_intersection.h"
#include "s2/s2region_union.h"

namespace {

std::mt19937_64 rng(0x43424e44ULL);  // "CBND"
std::uniform_real_distribution<double> unit(0.0, 1.0);

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

std::string P(const S2Point& p) { return D(p.x()) + "," + D(p.y()) + "," + D(p.z()); }

std::string Cap(const S2Cap& c) { return P(c.center()) + "," + D(c.radius().length2()); }

std::string Rect(const S2LatLngRect& r) {
  return D(r.lat().lo()) + "," + D(r.lat().hi()) + "," + D(r.lng().lo()) + "," +
         D(r.lng().hi());
}

S2Point RandomPoint() {
  for (;;) {
    double x = In(-1, 1), y = In(-1, 1), z = In(-1, 1);
    double n2 = x * x + y * y + z * z;
    if (n2 > 1e-6 && n2 <= 1) return S2Point(x, y, z).Normalize();
  }
}

S2CellId RandomId(int min_level, int max_level) {
  int face = static_cast<int>(rng() % 6);
  uint64_t pos = rng() & ((uint64_t{1} << 61) - 1);
  int level = min_level + static_cast<int>(rng() % (max_level - min_level + 1));
  return S2CellId::FromFacePosLevel(face, pos, level);
}

void CellBounds() {
  std::vector<S2CellId> ids;
  for (int f = 0; f < 6; ++f) ids.push_back(S2CellId::FromFace(f));
  ids.push_back(S2CellId(0x0400000000000000ULL));
  ids.push_back(S2CellId(0x4340000000000000ULL));
  for (int i = 0; i < 1000; ++i) ids.push_back(RandomId(0, 30));
  std::printf("# CELL_BOUNDS\n");
  for (S2CellId id : ids) {
    S2Cell c(id);
    std::printf("%s,%s,%s\n", Hex64(id.id()).c_str(), Cap(c.GetCapBound()).c_str(),
                Rect(c.GetRectBound()).c_str());
  }
  std::printf("# CELL_BOUND_POINTS\n");
  const double offs[] = {0.0, 1.0, 5.0 / 3.0 - 1e-3};
  for (S2CellId id : ids) {
    S2Cell c(id);
    R2Rect uv = c.GetBoundUV();
    S2Cap cap = c.GetCapBound();
    S2LatLngRect rect = c.GetRectBound();
    auto row = [&](double u, double v) {
      S2Point p = S2::FaceUVtoXYZ(c.face(), u, v).Normalize();
      std::printf("%s,%s,%s,%s,%s\n", Hex64(id.id()).c_str(), P(p).c_str(),
                  B(c.Contains(p)).c_str(), B(cap.Contains(p)).c_str(),
                  B(rect.Contains(p)).c_str());
    };
    for (double o : offs) {
      double d = o * DBL_EPSILON;
      for (int k = 0; k < 4; ++k) {
        row((k == 0 || k == 3) ? uv[0][0] - d : uv[0][1] + d, (k < 2) ? uv[1][0] - d : uv[1][1] + d);
      }
      double um = uv[0].GetCenter(), vm = uv[1].GetCenter();
      row(uv[0][0] - d, vm);
      row(uv[0][1] + d, vm);
      row(um, uv[1][0] - d);
      row(um, uv[1][1] + d);
    }
    for (int k = 0; k < 2; ++k) {
      double u = In(uv[0][0], uv[0][1]);
      double v = In(uv[1][0], uv[1][1]);
      row(u, v);
    }
  }
}

// A random member and its encoding.
std::unique_ptr<S2Region> Member(std::string* enc) {
  switch (rng() % 4) {
    case 0: {
      S2Point c = RandomPoint();
      double r = In(0, 1.5);
      *enc = "c:" + D(c.x()) + ":" + D(c.y()) + ":" + D(c.z()) + ":" + D(r);
      return std::make_unique<S2Cap>(c, S1ChordAngle::Radians(r));
    }
    case 1: {
      double a = In(-M_PI_2, M_PI_2), b = In(-M_PI_2, M_PI_2);
      double l1 = In(-M_PI, M_PI), l2 = In(-M_PI, M_PI);
      double lo = std::min(a, b), hi = std::max(a, b);
      *enc = "r:" + D(lo) + ":" + D(hi) + ":" + D(l1) + ":" + D(l2);
      return std::make_unique<S2LatLngRect>(R1Interval(lo, hi), S1Interval(l1, l2));
    }
    case 2: {
      S2CellId id = RandomId(0, 12);
      *enc = "e:" + Hex64(id.id());
      return std::make_unique<S2Cell>(id);
    }
    default: {
      S2Point p = RandomPoint();
      *enc = "p:" + D(p.x()) + ":" + D(p.y()) + ":" + D(p.z());
      return std::make_unique<S2PointRegion>(p);
    }
  }
}

void Adaptors(bool is_union) {
  std::printf(is_union ? "# REGION_UNION\n" : "# REGION_INTERSECTION\n");
  for (int i = 0; i < 2000; ++i) {
    int n = static_cast<int>(rng() % 5);  // 0 to 4 members
    std::vector<std::unique_ptr<S2Region>> regions;
    std::string members;
    for (int k = 0; k < n; ++k) {
      std::string e;
      regions.push_back(Member(&e));
      if (k > 0) members += ";";
      members += e;
    }
    S2CellId cell_id = RandomId(0, 12);
    S2Point p = RandomPoint();
    std::unique_ptr<S2Region> r;
    if (is_union) {
      r = std::make_unique<S2RegionUnion>(std::move(regions));
    } else {
      r = std::make_unique<S2RegionIntersection>(std::move(regions));
    }
    S2Cell cell(cell_id);
    std::vector<S2CellId> cover;
    r->GetCellUnionBound(&cover);
    std::printf("%s,%s,%s,%s,%s,%zu,%s,%s,%s\n", members.c_str(), Hex64(cell_id.id()).c_str(),
                P(p).c_str(), Cap(r->GetCapBound()).c_str(), Rect(r->GetRectBound()).c_str(),
                cover.size(), B(r->Contains(cell)).c_str(), B(r->MayIntersect(cell)).c_str(),
                B(r->Contains(p)).c_str());
  }
}

}  // namespace

int main() {
  CellBounds();
  Adaptors(true);
  Adaptors(false);
  return 0;
}
