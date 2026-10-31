// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
//
// Spec 011 survey probe, upstream side: the cases of ../rust/src/main.rs on S2PointRegion and
// S2CellUnion's floating-point methods, with the same LCG inputs, printed in the same line
// format.
#include <cinttypes>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "s2/s1angle.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2cell_union.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2point_region.h"

struct Lcg {
  uint64_t s;
  uint64_t next() {
    s = s * 6364136223846793005ULL + 1442695040888963407ULL;
    return s;
  }
  double unit() { return static_cast<double>(next() >> 11) / 9007199254740992.0; }
  double range(double lo, double hi) { return lo + (hi - lo) * unit(); }
};

static std::string b(double x) {
  uint64_t u;
  std::memcpy(&u, &x, 8);
  char buf[17];
  std::snprintf(buf, sizeof buf, "%016" PRIx64, u);
  return buf;
}

static std::string cap_s(const S2Cap& c) {
  return b(c.center().x()) + " " + b(c.center().y()) + " " + b(c.center().z()) + " " +
         b(c.radius().length2());
}

static std::string rect_s(const S2LatLngRect& r) {
  return b(r.lat().lo()) + " " + b(r.lat().hi()) + " " + b(r.lng().lo()) + " " +
         b(r.lng().hi());
}

static std::string ids_s(const std::vector<S2CellId>& ids) {
  std::string s = std::to_string(ids.size());
  for (S2CellId id : ids) {
    char buf[17];
    std::snprintf(buf, sizeof buf, "%016" PRIx64, id.id());
    s += " ";
    s += buf;
  }
  return s;
}

// FNV-1a over the ids, for unions too long to print.
static std::string ids_hash(const std::vector<S2CellId>& ids) {
  uint64_t h = 14695981039346656037ULL;
  for (S2CellId id : ids) {
    uint64_t v = id.id();
    for (int k = 0; k < 8; ++k) {
      h ^= (v >> (8 * k)) & 0xff;
      h *= 1099511628211ULL;
    }
  }
  char buf[40];
  std::snprintf(buf, sizeof buf, "%zu %016" PRIx64, ids.size(), h);
  return buf;
}

static S2Point random_point(Lcg& g) {
  for (;;) {
    double x = g.range(-1, 1), y = g.range(-1, 1), z = g.range(-1, 1);
    double n2 = x * x + y * y + z * z;
    if (n2 > 1e-6 && n2 <= 1) return S2Point(x, y, z).Normalize();
  }
}

static S2CellId random_id(Lcg& g, int min_level, int max_level) {
  int face = static_cast<int>(g.next() % 6);
  uint64_t pos = g.next() & ((uint64_t{1} << 61) - 1);
  int level = min_level + static_cast<int>(g.next() % (max_level - min_level + 1));
  return S2CellId::FromFacePosLevel(face, pos, level);
}

// A random point of the cell: the centre of a random descendant at a random finer level.
static S2Point point_in(Lcg& g, S2CellId id) {
  int level = id.level() + static_cast<int>(g.next() % (31 - id.level()));
  S2CellId d = id;
  while (d.level() < level) d = d.child(static_cast<int>(g.next() % 4));
  return d.ToPoint();
}

// A random union: either cells anywhere (levels 0..20), or a cluster of descendants of one cell.
static std::vector<S2CellId> random_union_ids(Lcg& g) {
  std::vector<S2CellId> ids;
  int n = 1 + static_cast<int>(g.next() % 8);
  if (g.next() % 2 == 0) {
    for (int k = 0; k < n; ++k) ids.push_back(random_id(g, 0, 20));
  } else {
    S2CellId base = random_id(g, 0, 18);
    for (int k = 0; k < n; ++k) {
      int extra = 1 + static_cast<int>(g.next() % 8);
      S2CellId d = base;
      for (int e = 0; e < extra && d.level() < 30; ++e) d = d.child(static_cast<int>(g.next() % 4));
      ids.push_back(d);
    }
  }
  return ids;
}

int main() {
  // PC, PR, PU: S2PointRegion's bounds over random unit points and special vectors.
  Lcg g{0x0110001100110011ULL};
  std::vector<S2Point> pts;
  for (int i = 0; i < 20000; ++i) pts.push_back(random_point(g));
  std::vector<S2Point> specials = {
      S2Point(1, 0, 0),         S2Point(-1, -0.0, 0),  S2Point(-1, 0, 0),
      S2Point(0, 0, 1),         S2Point(0, 0, -1),     S2Point(0, 0, 0),
      S2Point(3, 4, 0),         S2Point(1e-300, 0, 0), S2Point(NAN, 0, 1),
      S2Point(INFINITY, 0, 0),  S2Point(-0.0, -0.0, -1)};
  for (size_t i = 0; i < pts.size(); ++i) {
    S2PointRegion r(pts[i]);
    std::printf("PC %zu %s\n", i, cap_s(r.GetCapBound()).c_str());
    std::printf("PR %zu %s\n", i, rect_s(r.GetRectBound()).c_str());
    if (i < 2000) {
      std::vector<S2CellId> ids;
      r.GetCellUnionBound(&ids);
      std::printf("PU %zu %s\n", i, ids_s(ids).c_str());
      // The covering contains the point.
      std::printf("PUC %zu %d\n", i, S2CellUnion(ids).Contains(pts[i]) ? 1 : 0);
    }
  }
  for (size_t i = 0; i < specials.size(); ++i) {
    S2PointRegion r(specials[i]);
    std::printf("PSC %zu %s\n", i, cap_s(r.GetCapBound()).c_str());
    std::printf("PSR %zu %s\n", i, rect_s(r.GetRectBound()).c_str());
  }

  // PX: Contains(cell) and MayIntersect(cell): the point's own cell at each level, and a random
  // cell.
  g = Lcg{0x0120001200120012ULL};
  for (int i = 0; i < 5000; ++i) {
    S2Point p = random_point(g);
    S2PointRegion r(p);
    S2CellId leaf(p);
    int level = static_cast<int>(g.next() % 31);
    S2Cell own(leaf.parent(level));
    S2Cell other(random_id(g, 0, 30));
    std::printf("PX %d %d %d %d %d\n", i, r.Contains(own) ? 1 : 0, r.MayIntersect(own) ? 1 : 0,
                r.Contains(other) ? 1 : 0, r.MayIntersect(other) ? 1 : 0);
  }

  // PV: the vertex law for S2PointRegion: the region of a cell's vertex intersects the cell,
  // i.e. S2Cell::Contains(vertex), for each of the four vertices.
  g = Lcg{0x0130001300130013ULL};
  int pv_fail = 0;
  for (int i = 0; i < 20000; ++i) {
    S2Cell c(random_id(g, 0, 30));
    std::string s;
    for (int k = 0; k < 4; ++k) {
      bool in = S2PointRegion(c.GetVertex(k)).MayIntersect(c);
      if (!in) ++pv_fail;
      s += in ? " 1" : " 0";
    }
    std::printf("PV %d%s\n", i, s.c_str());
  }
  std::fprintf(stderr, "PV cells whose own vertex S2Cell::Contains rejects: %d of 80000\n",
               pv_fail);

  // UC, UR, UU, UA: S2CellUnion's bounds and areas over random unions.
  g = Lcg{0x0140001400140014ULL};
  int ug_contained = 0, ug_cap_miss = 0, ug_rect_miss = 0, uu_miss = 0;
  for (int i = 0; i < 10000; ++i) {
    S2CellUnion u(random_union_ids(g));
    S2Cap cap = u.GetCapBound();
    S2LatLngRect rect = u.GetRectBound();
    std::printf("UC %d %s\n", i, cap_s(cap).c_str());
    std::printf("UR %d %s\n", i, rect_s(rect).c_str());
    std::vector<S2CellId> bound;
    u.GetCellUnionBound(&bound);
    std::printf("UU %d %s\n", i, ids_s(bound).c_str());
    std::printf("UA %d %s %s %s\n", i, b(u.AverageBasedArea()).c_str(),
                b(u.ApproxArea()).c_str(), b(u.ExactArea()).c_str());
    // UG: the bounds contain every contained point: each cell's centre, its vertices and
    // random points of it.
    std::string s;
    int cap_miss = 0, rect_miss = 0, contained = 0;
    for (S2CellId id : u) {
      S2Cell c(id);
      std::vector<S2Point> ps = {c.GetCenter()};
      for (int k = 0; k < 4; ++k) ps.push_back(c.GetVertex(k));
      for (int k = 0; k < 4; ++k) ps.push_back(point_in(g, id));
      for (const S2Point& p : ps) {
        if (!u.Contains(p)) continue;
        ++contained;
        if (!cap.Contains(p)) ++cap_miss;
        if (!rect.Contains(p)) ++rect_miss;
      }
    }
    ug_contained += contained;
    ug_cap_miss += cap_miss;
    ug_rect_miss += rect_miss;
    std::printf("UG %d %d %d %d\n", i, contained, cap_miss, rect_miss);
    // UUC: the covering covers every cell of the union.
    S2CellUnion cover(bound);
    int miss = 0;
    for (S2CellId id : u) {
      if (!cover.Contains(id)) ++miss;
    }
    uu_miss += miss;
    std::printf("UUC %d %d\n", i, miss);
  }
  std::fprintf(stderr,
               "UG contained points %d, outside the cap bound %d, outside the rect bound %d; "
               "union cells outside the covering %d\n",
               ug_contained, ug_cap_miss, ug_rect_miss, uu_miss);

  // US: special unions: empty, whole sphere, one leaf, the six faces' opposite pairs.
  std::vector<std::vector<S2CellId>> sp = {
      {},
      {S2CellId::FromFace(0), S2CellId::FromFace(1), S2CellId::FromFace(2),
       S2CellId::FromFace(3), S2CellId::FromFace(4), S2CellId::FromFace(5)},
      {S2CellId(S2Point(1, 0, 0))},
      {S2CellId::FromFace(0), S2CellId::FromFace(3)},
      {S2CellId::FromFace(2), S2CellId::FromFace(5)},
      {S2CellId::FromFace(1).child(0), S2CellId::FromFace(4).child(3)}};
  for (size_t i = 0; i < sp.size(); ++i) {
    S2CellUnion u(sp[i]);
    std::vector<S2CellId> bound;
    u.GetCellUnionBound(&bound);
    std::printf("US %zu %s %s %s %s %s %s\n", i, cap_s(u.GetCapBound()).c_str(),
                rect_s(u.GetRectBound()).c_str(), ids_s(bound).c_str(),
                b(u.AverageBasedArea()).c_str(), b(u.ApproxArea()).c_str(),
                b(u.ExactArea()).c_str());
  }

  // CF: S2Cell::GetRectBound() of the six face cells (UR's differences).
  for (int f = 0; f < 6; ++f) {
    std::printf("CF %d %s\n", f, rect_s(S2Cell(S2CellId::FromFace(f)).GetRectBound()).c_str());
  }

  // UV: the vertex law for S2CellUnion: the edge neighbours of a union's cells that the union
  // does not intersect, and how many of their vertices the union contains.
  g = Lcg{0x0160001600160016ULL};
  int uv_cells = 0, uv_contained = 0;
  for (int i = 0; i < 5000; ++i) {
    S2CellUnion u(random_union_ids(g));
    int cells = 0, contained = 0;
    for (S2CellId id : u) {
      S2CellId nbr[4];
      id.GetEdgeNeighbors(nbr);
      for (S2CellId n : nbr) {
        S2Cell c(n);
        if (u.MayIntersect(c)) continue;
        ++cells;
        for (int k = 0; k < 4; ++k) contained += u.Contains(c.GetVertex(k)) ? 1 : 0;
      }
    }
    uv_cells += cells;
    uv_contained += contained;
    std::printf("UV %d %d %d\n", i, cells, contained);
  }
  std::fprintf(stderr,
               "UV non-intersected edge neighbours %d, their vertices the union contains %d\n",
               uv_cells, uv_contained);

  // UE: Expand(min_radius, max_level_diff) over small clustered unions.
  g = Lcg{0x0150001500150015ULL};
  const double radii[] = {0.0, -1.0, 4.0, 2.0, 1e-300, INFINITY, 0.9428090415820634,
                          0.9428090415820635, 0.4714045207910317};
  for (int i = 0; i < 3000; ++i) {
    std::vector<S2CellId> ids;
    S2CellId base = random_id(g, 4, 20);
    int n = 1 + static_cast<int>(g.next() % 3);
    for (int k = 0; k < n; ++k) {
      S2CellId d = base;
      int extra = static_cast<int>(g.next() % 4);
      for (int e = 0; e < extra && d.level() < 30; ++e) d = d.child(static_cast<int>(g.next() % 4));
      ids.push_back(d);
    }
    S2CellUnion u(ids);
    double radius = radii[i % 9];
    if (i >= 9 * 20) {
      // Two statements: the operands of `*` are unsequenced in C++.
      double e = g.range(-9, 0);
      radius = std::pow(10.0, e) * (g.next() % 2 ? 1.0 : 0.5);
    }
    int diff = static_cast<int>(g.next() % 6);
    if (i % 50 == 0) diff = 40;
    // Keep expansions small: a radius far above the cells' width expands by many cells.
    if (radius > 1e-3 && base.level() > 8 && i >= 9 * 20) radius = 1e-4;
    u.Expand(S1Angle::Radians(radius), diff);
    std::printf("UE %d %s\n", i, ids_hash(u.cell_ids()).c_str());
  }
  return 0;
}
