// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
//
// Spec 010 survey probe, upstream side: the cases of ../rust/src/main.rs on S2LatLng and
// S2LatLngRect, with the same LCG inputs, printed in the same line format.
#include <cinttypes>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "s2/r1interval.h"
#include "s2/s1interval.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"

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

static void ll_line(const char* tag, size_t i, const S2LatLng& ll) {
  std::printf("%s %zu %s %s\n", tag, i, b(ll.lat().radians()).c_str(),
              b(ll.lng().radians()).c_str());
}

static void pt_line(const char* tag, size_t i, const S2Point& p) {
  std::printf("%s %zu %s %s %s\n", tag, i, b(p.x()).c_str(), b(p.y()).c_str(),
              b(p.z()).c_str());
}

static S2LatLngRect random_rect(Lcg& g) {
  double a = g.range(-M_PI_2, M_PI_2);
  double c = g.range(-M_PI_2, M_PI_2);
  double lo = a <= c ? a : c, hi = a <= c ? c : a;
  double l1 = g.range(-M_PI, M_PI);
  double l2 = g.range(-M_PI, M_PI);
  return S2LatLngRect(R1Interval(lo, hi), S1Interval(l1, l2));
}

static S2Cell random_cell(Lcg& g, uint64_t max_level) {
  int face = static_cast<int>(g.next() % 6);
  uint64_t pos = g.next() & ((uint64_t{1} << 61) - 1);
  int level = static_cast<int>(g.next() % (max_level + 1));
  return S2Cell(S2CellId::FromFacePosLevel(face, pos, level));
}

int main() {
  std::vector<double> specials = {M_PI,  -M_PI, 3 * M_PI, -3 * M_PI, 5 * M_PI,
                                  2 * M_PI, -2 * M_PI, M_PI + 1e-15, 1e10, 1e16,
                                  -2.462748394469102e16, 1e300, 0.0, -0.0};
  Lcg g{0x0010001000100010ULL};
  for (double scale : {4.0, 100.0, 1e6, 1e12, 1e16, 1e300}) {
    for (int k = 0; k < 2000; ++k) specials.push_back(g.range(-scale, scale));
  }
  for (size_t i = 0; i < specials.size(); ++i) {
    ll_line("N", i, S2LatLng::FromRadians(0.3, specials[i]).Normalized());
  }
  const double lats[] = {2.0, -2.0, M_PI_2, NAN, INFINITY, -INFINITY};
  for (size_t i = 0; i < 6; ++i) {
    ll_line("NL", i, S2LatLng::FromRadians(lats[i], 1.0).Normalized());
  }

  g = Lcg{0x0020002000200020ULL};
  for (size_t i = 0; i < 20000; ++i) {
    double lat = g.range(-M_PI_2, M_PI_2);
    double lng = g.range(-M_PI, M_PI);
    pt_line("P", i, S2LatLng::FromRadians(lat, lng).ToPoint());
  }
  const double ps[4][2] = {{M_PI_2, 0}, {-M_PI_2, 1}, {0, M_PI}, {0, -M_PI}};
  for (size_t i = 0; i < 4; ++i) {
    pt_line("PS", i, S2LatLng::FromRadians(ps[i][0], ps[i][1]).ToPoint());
  }

  g = Lcg{0x0030003000300030ULL};
  for (size_t i = 0; i < 20000; ++i) {
    double lat = g.range(-M_PI, M_PI);
    double lng = g.range(-4.0, 4.0);
    S2Point p = S2LatLng::FromRadians(lat, lng).ToPoint();
    ll_line("L", i, S2LatLng(p));
  }
  const double vecs[9][3] = {{0, 0, 1},   {0, 0, -1},       {-1, -0.0, 0},
                             {-1, 0, 0},  {-1, -1e-300, 0}, {0, 0, 0},
                             {NAN, 0, 1}, {INFINITY, 1, 1}, {1e-320, 1e-320, 1e-320}};
  for (size_t i = 0; i < 9; ++i) {
    ll_line("LS", i, S2LatLng(S2Point(vecs[i][0], vecs[i][1], vecs[i][2])));
  }

  g = Lcg{0x0040004000400040ULL};
  for (size_t i = 0; i < 20000; ++i) {
    double a1 = g.range(-M_PI_2, M_PI_2), a2 = g.range(-M_PI, M_PI);
    double c1 = g.range(-M_PI_2, M_PI_2), c2 = g.range(-M_PI, M_PI);
    std::printf("D %zu %s\n", i,
                b(S2LatLng::FromRadians(a1, a2)
                      .GetDistance(S2LatLng::FromRadians(c1, c2))
                      .radians())
                    .c_str());
  }
  const double bad[6][4] = {{2, 0, 0, 0},     {2, 0, 0, 1},
                            {-2, 0.5, 1, 2},  {NAN, 0, 0, 0},
                            {0, 4, 0, -4},    {3.124793493547795, 0, 0.0167991600419979, M_PI}};
  for (size_t i = 0; i < 6; ++i) {
    std::printf("DS %zu %s\n", i,
                b(S2LatLng::FromRadians(bad[i][0], bad[i][1])
                      .GetDistance(S2LatLng::FromRadians(bad[i][2], bad[i][3]))
                      .radians())
                    .c_str());
  }

  const double ss[5][2] = {{0.3, M_PI}, {0.3, -M_PI}, {95, 190}, {-0.0, -0.0}, {1e-7, 3e-7}};
  for (size_t i = 0; i < 5; ++i) {
    std::printf("S %zu %s\n", i,
                S2LatLng::FromDegrees(ss[i][0], ss[i][1]).ToStringInDegrees().c_str());
  }

  g = Lcg{0x0050005000500050ULL};
  int exact_differs = 0, may_true = 0;
  for (size_t i = 0; i < 20000; ++i) {
    S2LatLngRect r = random_rect(g);
    S2Cell c = random_cell(g, 8);
    bool may = r.MayIntersect(c);
    std::printf("R %zu %d %d\n", i, r.Contains(c) ? 1 : 0, may ? 1 : 0);
    if (may) ++may_true;
    if (may != r.Intersects(c)) ++exact_differs;
  }
  std::fprintf(stderr,
               "R exact Intersects(cell) differs from MayIntersect on %d of 20000 (MayIntersect "
               "true on %d)\n",
               exact_differs, may_true);

  g = Lcg{0x0060006000600060ULL};
  int viol = 0, checked = 0;
  for (size_t i = 0; i < 20000; ++i) {
    S2LatLngRect r = random_rect(g);
    S2Cap cap = r.GetCapBound();
    double lat = g.range(r.lat().lo(), r.lat().hi());
    double t = g.unit();
    double lng;
    if (r.lng().is_inverted()) {
      double v = r.lng().lo() + t * r.lng().GetLength();
      lng = v > M_PI ? v - 2 * M_PI : v;
    } else {
      lng = r.lng().lo() + t * (r.lng().hi() - r.lng().lo());
    }
    const S2LatLng cands[7] = {
        S2LatLng::FromRadians(r.lat().lo(), r.lng().lo()),
        S2LatLng::FromRadians(r.lat().hi(), r.lng().hi()),
        S2LatLng::FromRadians(r.lat().lo(), lng),
        S2LatLng::FromRadians(r.lat().hi(), lng),
        S2LatLng::FromRadians(lat, r.lng().lo()),
        S2LatLng::FromRadians(lat, r.lng().hi()),
        S2LatLng::FromRadians(lat, lng),
    };
    std::string bits;
    for (const S2LatLng& ll : cands) {
      S2Point p = ll.ToPoint();
      bool inside = r.Contains(p);
      bool in_cap = cap.Contains(p);
      if (inside) {
        ++checked;
        if (!in_cap) {
          ++viol;
          if (viol <= 5) {
            std::fprintf(stderr,
                         "G miss: rect lat [%.17g, %.17g] lng [%.17g, %.17g] point "
                         "(%.17g, %.17g) cap height %.17g\n",
                         r.lat().lo(), r.lat().hi(), r.lng().lo(), r.lng().hi(),
                         ll.lat().radians(), ll.lng().radians(), cap.height());
          }
        }
      }
      bits += inside ? '1' : '0';
      bits += in_cap ? '1' : '0';
    }
    std::printf("G %zu %s\n", i, bits.c_str());
  }
  std::fprintf(stderr, "G cap_bound misses %d of %d contained points\n", viol, checked);

  g = Lcg{0x0070007000700070ULL};
  viol = 0;
  for (size_t i = 0; i < 20000; ++i) {
    S2LatLngRect r = random_rect(g);
    S2Cell c = random_cell(g, 12);
    bool may = r.MayIntersect(c);
    std::string bits;
    for (int k = 0; k < 4; ++k) {
      bool inside = r.Contains(c.GetVertex(k));
      if (!may && inside) ++viol;
      bits += inside ? '1' : '0';
    }
    std::printf("V %zu %d %s\n", i, may ? 1 : 0, bits.c_str());
  }
  std::fprintf(stderr, "V vertices contained in a cell MayIntersect rejects: %d\n", viol);
  return 0;
}
