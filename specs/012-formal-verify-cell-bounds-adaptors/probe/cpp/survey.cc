// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
//
// Spec 012 survey probe, upstream side: S2Cell's bounds against the points the cell contains
// (adversarial points around its corners and edges, within and beyond Contains(p)'s u-v
// margin), and S2RegionUnion / S2RegionIntersection over mixed members. The same LCG inputs and
// line format as ../rust/src/main.rs.
#include <cinttypes>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <memory>
#include <string>
#include <vector>

#include "s2/r2rect.h"
#include "s2/s1angle.h"
#include "s2/s1chord_angle.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2coords.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2point_region.h"
#include "s2/s2region_intersection.h"
#include "s2/s2region_union.h"

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

// A random member region: a cap, a rectangle, a cell or a point region.
static std::unique_ptr<S2Region> random_region(Lcg& g) {
  switch (g.next() % 4) {
    case 0: {
      S2Point c = random_point(g);
      double r = g.range(0, 1.5);
      return std::make_unique<S2Cap>(c, S1ChordAngle::Radians(r));
    }
    case 1: {
      double a = g.range(-M_PI_2, M_PI_2), c = g.range(-M_PI_2, M_PI_2);
      double l1 = g.range(-M_PI, M_PI), l2 = g.range(-M_PI, M_PI);
      return std::make_unique<S2LatLngRect>(R1Interval(std::min(a, c), std::max(a, c)),
                                            S1Interval(l1, l2));
    }
    case 2:
      return std::make_unique<S2Cell>(random_id(g, 0, 12));
    default:
      return std::make_unique<S2PointRegion>(random_point(g));
  }
}

int main() {
  // CB: S2Cell's bounds against adversarial points. Per cell: the 4 corners and 4 edge
  // midpoints of the u-v bound, each moved outward by 0, 0.5, 1, 1.5, 5/3 (just under and
  // over) and 2 times DBL_EPSILON in each coordinate it bounds, and 4 random interior points.
  Lcg g{0x0210002100210021ULL};
  const double offs[] = {0.0, 0.5, 1.0, 1.5, 5.0 / 3.0 - 1e-3, 5.0 / 3.0 + 1e-3, 2.0};
  long contained = 0, cap_miss = 0, rect_miss = 0;
  for (int i = 0; i < 20000; ++i) {
    S2Cell c(random_id(g, 0, 30));
    R2Rect uv = c.GetBoundUV();
    S2Cap cap = c.GetCapBound();
    S2LatLngRect rect = c.GetRectBound();
    std::string s;
    int cm = 0, rm = 0, n = 0;
    auto test = [&](double u, double v) {
      S2Point p = S2::FaceUVtoXYZ(c.face(), u, v).Normalize();
      bool in = c.Contains(p);
      bool ic = cap.Contains(p);
      bool ir = rect.Contains(p);
      s += in ? (ic ? (ir ? "3" : "2") : (ir ? "1" : "0")) : "x";
      if (in) {
        ++n;
        if (!ic) ++cm;
        if (!ir) ++rm;
      }
    };
    for (double o : offs) {
      double d = o * DBL_EPSILON;
      for (int k = 0; k < 4; ++k) {
        double u = (k == 0 || k == 3) ? uv[0][0] - d : uv[0][1] + d;
        double v = (k < 2) ? uv[1][0] - d : uv[1][1] + d;
        test(u, v);
      }
      double um = uv[0].GetCenter(), vm = uv[1].GetCenter();
      test(uv[0][0] - d, vm);
      test(uv[0][1] + d, vm);
      test(um, uv[1][0] - d);
      test(um, uv[1][1] + d);
    }
    for (int k = 0; k < 4; ++k) {
      test(g.range(uv[0][0], uv[0][1]), g.range(uv[1][0], uv[1][1]));
    }
    contained += n;
    cap_miss += cm;
    rect_miss += rm;
    std::printf("CB %d %s %s %s\n", i, cap_s(cap).c_str(), rect_s(rect).c_str(), s.c_str());
  }
  std::fprintf(stderr,
               "CB contained points %ld, outside the cap bound %ld, outside the rect bound %ld\n",
               contained, cap_miss, rect_miss);

  // RU, RI: S2RegionUnion and S2RegionIntersection of 1 to 4 mixed members: their bounds, and
  // their tests on a random cell and a random point.
  for (int kind = 0; kind < 2; ++kind) {
    g = Lcg{kind == 0 ? 0x0220002200220022ULL : 0x0230002300230023ULL};
    for (int i = 0; i < 5000; ++i) {
      int n = 1 + static_cast<int>(g.next() % 4);
      std::vector<std::unique_ptr<S2Region>> regions;
      for (int k = 0; k < n; ++k) regions.push_back(random_region(g));
      S2Cell cell(random_id(g, 0, 12));
      S2Point p = random_point(g);
      std::unique_ptr<S2Region> r;
      if (kind == 0) {
        r = std::make_unique<S2RegionUnion>(std::move(regions));
      } else {
        r = std::make_unique<S2RegionIntersection>(std::move(regions));
      }
      std::vector<S2CellId> cover;
      r->GetCellUnionBound(&cover);
      std::printf("%s %d %s %s %zu %d %d %d\n", kind == 0 ? "RU" : "RI", i,
                  cap_s(r->GetCapBound()).c_str(), rect_s(r->GetRectBound()).c_str(),
                  cover.size(), r->Contains(cell) ? 1 : 0, r->MayIntersect(cell) ? 1 : 0,
                  r->Contains(p) ? 1 : 0);
    }
  }
  return 0;
}
