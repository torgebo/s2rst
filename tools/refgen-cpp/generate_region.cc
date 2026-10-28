// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
//
// Reference-vector generator for s2rst's PointRegion and for CellUnion's floating-point methods
// (specs/011-formal-verify-point-region-cell-union, FR-021, research R9), using the C++
// reference implementation (google/s2geometry) as the oracle. Emits CSV that
// core/tests/cpp_region_diff.rs diffs the library against. Deterministic (fixed seed). See
// tools/refgen-cpp/README.md.
//
// Every double is the 16-hex-digit bit pattern of its value; a bool is 0 or 1; an integer is
// decimal; a cell id is its 16-hex-digit value; a list of ids is ';'-separated (empty for none).
//
// # POINTREGION
//   x,y,z,cx,cy,cz,length2,latlo,lathi,lnglo,lnghi,bound
//   S2PointRegion(p): GetCapBound() (centre, squared chord radius), GetRectBound(),
//   GetCellUnionBound().
//
// # POINTREGION_CELL
//   x,y,z,cell,contains,may_intersect
//   S2PointRegion(p).Contains(S2Cell(cell)) and MayIntersect(S2Cell(cell)).
//
// # CELLUNION_BOUND
//   ids,cx,cy,cz,length2,latlo,lathi,lnglo,lnghi,bound
//   S2CellUnion(ids) (ids already normalised): GetCapBound(), GetRectBound(),
//   GetCellUnionBound().
//
// # CELLUNION_AREA
//   ids,average,approx,exact
//   AverageBasedArea(), ApproxArea(), ExactArea().
//
// # CELLUNION_POINT
//   ids,x,y,z,contains
//   Contains(S2Point(x, y, z)).
//
// # CELLUNION_EXPAND
//   ids,radius,diff,result
//   Expand(S1Angle::Radians(radius), diff), the resulting ids.
//
// # CELL_FACE_RECT
//   face,latlo,lathi,lnglo,lnghi
//   S2Cell(S2CellId::FromFace(face)).GetRectBound().

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <random>
#include <string>
#include <vector>

#include "s2/s1angle.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2cell_union.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2metrics.h"
#include "s2/s2point.h"
#include "s2/s2point_region.h"

namespace {

std::mt19937_64 rng(0x52474e53ULL);  // "RGNS"
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

std::string Ids(const std::vector<S2CellId>& ids) {
  std::string s;
  for (size_t i = 0; i < ids.size(); ++i) {
    if (i > 0) s += ";";
    s += Hex64(ids[i].id());
  }
  return s;
}

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

S2CellId RandomDescendant(S2CellId id, int extra) {
  for (int e = 0; e < extra && id.level() < S2CellId::kMaxLevel; ++e) {
    id = id.child(static_cast<int>(rng() % 4));
  }
  return id;
}

// A random point of the cell: the centre of a random descendant at a random finer level.
S2Point PointIn(S2CellId id) {
  return RandomDescendant(id, static_cast<int>(rng() % (31 - id.level()))).ToPoint();
}

// The unions: special ones, then random ones (scattered or clustered).
std::vector<S2CellUnion> Unions() {
  std::vector<S2CellUnion> us;
  auto face = [](int f) { return S2CellId::FromFace(f); };
  us.emplace_back(std::vector<S2CellId>{});
  us.emplace_back(std::vector<S2CellId>{face(0), face(1), face(2), face(3), face(4), face(5)});
  for (int f = 0; f < 6; ++f) us.emplace_back(std::vector<S2CellId>{face(f)});
  us.emplace_back(std::vector<S2CellId>{face(0), face(3)});
  us.emplace_back(std::vector<S2CellId>{face(1), face(4)});
  us.emplace_back(std::vector<S2CellId>{face(2), face(5)});
  us.emplace_back(std::vector<S2CellId>{face(2), face(0).child(1)});
  us.emplace_back(std::vector<S2CellId>{S2CellId(S2Point(1, 0, 0))});
  us.emplace_back(std::vector<S2CellId>{S2CellId(S2Point(0, 0, -1)).parent(3)});
  for (int i = 0; i < 2000; ++i) {
    std::vector<S2CellId> ids;
    int n = 1 + static_cast<int>(rng() % 8);
    if (rng() % 2 == 0) {
      for (int k = 0; k < n; ++k) ids.push_back(RandomId(0, 20));
    } else {
      S2CellId base = RandomId(0, 18);
      for (int k = 0; k < n; ++k) {
        int extra = 1 + static_cast<int>(rng() % 8);
        ids.push_back(RandomDescendant(base, extra));
      }
    }
    us.emplace_back(ids);
  }
  return us;
}

void PointRegions() {
  std::printf("# POINTREGION\n");
  std::vector<S2Point> ps = {S2Point(1, 0, 0),  S2Point(-1, -0.0, 0), S2Point(-1, 0, 0),
                             S2Point(0, 0, 1),  S2Point(0, 0, -1),    S2Point(-0.0, -0.0, -1),
                             S2Point(0, 1, 0),  S2Point(0, -1, 0)};
  // Face boundaries and corners, and the cell vertices of a few cells.
  ps.push_back(S2Point(1, 1, 0).Normalize());
  ps.push_back(S2Point(1, -1, 1).Normalize());
  ps.push_back(S2Point(-1, -1, -1).Normalize());
  for (int i = 0; i < 50; ++i) {
    S2Cell c(RandomId(0, 30));
    for (int k = 0; k < 4; ++k) ps.push_back(c.GetVertex(k));
  }
  for (int i = 0; i < 2000; ++i) ps.push_back(RandomPoint());
  for (const S2Point& p : ps) {
    S2PointRegion r(p);
    std::vector<S2CellId> bound;
    r.GetCellUnionBound(&bound);
    std::printf("%s,%s,%s,%s\n", P(p).c_str(), Cap(r.GetCapBound()).c_str(),
                Rect(r.GetRectBound()).c_str(), Ids(bound).c_str());
  }
}

void PointRegionCells() {
  std::printf("# POINTREGION_CELL\n");
  for (int i = 0; i < 2000; ++i) {
    S2Point p = RandomPoint();
    S2PointRegion r(p);
    S2Cell own(S2CellId(p).parent(static_cast<int>(rng() % 31)));
    S2Cell other(RandomId(0, 30));
    for (const S2Cell* c : {&own, &other}) {
      std::printf("%s,%s,%s,%s\n", P(p).c_str(), Hex64(c->id().id()).c_str(),
                  B(r.Contains(*c)).c_str(), B(r.MayIntersect(*c)).c_str());
    }
  }
  // A cell's vertices, and the neighbours' cells at the same vertices.
  for (int i = 0; i < 500; ++i) {
    S2Cell c(RandomId(0, 30));
    for (int k = 0; k < 4; ++k) {
      S2Point v = c.GetVertex(k);
      S2PointRegion r(v);
      std::printf("%s,%s,%s,%s\n", P(v).c_str(), Hex64(c.id().id()).c_str(),
                  B(r.Contains(c)).c_str(), B(r.MayIntersect(c)).c_str());
    }
  }
}

void UnionBounds(const std::vector<S2CellUnion>& us) {
  std::printf("# CELLUNION_BOUND\n");
  for (const S2CellUnion& u : us) {
    std::vector<S2CellId> bound;
    u.GetCellUnionBound(&bound);
    std::printf("%s,%s,%s,%s\n", Ids(u.cell_ids()).c_str(), Cap(u.GetCapBound()).c_str(),
                Rect(u.GetRectBound()).c_str(), Ids(bound).c_str());
  }
  std::printf("# CELLUNION_AREA\n");
  for (const S2CellUnion& u : us) {
    std::printf("%s,%s,%s,%s\n", Ids(u.cell_ids()).c_str(), D(u.AverageBasedArea()).c_str(),
                D(u.ApproxArea()).c_str(), D(u.ExactArea()).c_str());
  }
}

void UnionPoints(const std::vector<S2CellUnion>& us) {
  std::printf("# CELLUNION_POINT\n");
  for (size_t i = 0; i < us.size() && i < 1000; ++i) {
    const S2CellUnion& u = us[i];
    std::vector<S2Point> ps = {RandomPoint(), RandomPoint()};
    for (S2CellId id : u) {
      S2Cell c(id);
      ps.push_back(PointIn(id));
      ps.push_back(c.GetVertex(static_cast<int>(rng() % 4)));
    }
    for (const S2Point& p : ps) {
      std::printf("%s,%s,%s\n", Ids(u.cell_ids()).c_str(), P(p).c_str(),
                  B(u.Contains(p)).c_str());
    }
  }
}

void UnionExpand() {
  std::printf("# CELLUNION_EXPAND\n");
  const double radii[] = {0.0, -1.0, 4.0, 2.0, 1e-300, std::numeric_limits<double>::infinity(),
                          0.9428090415820634, 0.9428090415820635, 0.4714045207910317};
  for (int i = 0; i < 1000; ++i) {
    std::vector<S2CellId> ids;
    S2CellId base = RandomId(4, 20);
    int n = 1 + static_cast<int>(rng() % 3);
    for (int k = 0; k < n; ++k) ids.push_back(RandomDescendant(base, static_cast<int>(rng() % 4)));
    S2CellUnion u(ids);
    double radius = radii[i % 9];
    if (i >= 9 * 10) {
      double e = In(-9, 0);
      radius = std::pow(10.0, e) * (rng() % 2 ? 1.0 : 0.5);
      // Keep expansions small: a radius far above the cells' width expands by many cells.
      if (radius > 1e-3 && base.level() > 8) radius = 1e-4;
    }
    int diff = static_cast<int>(rng() % 6);
    if (i % 50 == 0) diff = 40;
    // Keep the result small: at most three levels below the union's coarsest cell, since each
    // cell expands into all its descendants at the target level (a radius of 0 with diff 40
    // would give every leaf).
    int min_level = S2CellId::kMaxLevel;
    for (S2CellId id : u) min_level = std::min(min_level, id.level());
    int radius_level = S2::kMinWidth.GetLevelForMinValue(radius);
    if (std::min(min_level + diff, radius_level) > min_level + 3) diff = static_cast<int>(rng() % 4);
    std::string before = Ids(u.cell_ids());
    u.Expand(S1Angle::Radians(radius), diff);
    std::printf("%s,%s,%d,%s\n", before.c_str(), D(radius).c_str(), diff,
                Ids(u.cell_ids()).c_str());
  }
}

void FaceRects() {
  std::printf("# CELL_FACE_RECT\n");
  for (int f = 0; f < 6; ++f) {
    std::printf("%d,%s\n", f, Rect(S2Cell(S2CellId::FromFace(f)).GetRectBound()).c_str());
  }
}

}  // namespace

int main() {
  PointRegions();
  PointRegionCells();
  std::vector<S2CellUnion> us = Unions();
  UnionBounds(us);
  UnionPoints(us);
  UnionExpand();
  FaceRects();
  return 0;
}
