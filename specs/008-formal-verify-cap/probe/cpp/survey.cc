// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
//
// Spec 008 survey probe, upstream side: the cases of ../rust/src/main.rs on S2Cap.
#include <cmath>
#include <cstdio>
#include <vector>

#include "s2/s1angle.h"
#include "s2/s1chord_angle.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2region_coverer.h"

static S2Point LL(double lat, double lng) {
  return S2LatLng::FromDegrees(lat, lng).ToPoint();
}

int main() {
  // K1: the covering's level for the probe's first radius.
  S2Point center = LL(10, 20);
  S2Cap k1(center, S1Angle::Radians(0.11785113019775795));
  std::vector<S2CellId> ids;
  k1.GetCellUnionBound(&ids);
  std::printf("K1 angle_radius=%.17g level=%d cells=%zu\n", k1.GetRadius().radians(),
              ids.empty() ? -2 : ids[0].level(), ids.size());

  // K2: Contains(face cell 0) for the probe's grid; print the cases.
  S2Cell face0(S2CellId::FromFace(0));
  int k2_true = 0;
  for (int dlat = 0; dlat <= 12; ++dlat) {
    for (double rad : {56.0, 58.0, 60.0, 62.0, 64.0}) {
      S2Cap c(LL(dlat, 0), S1Angle::Degrees(rad));
      if (c.Contains(face0)) ++k2_true;
      if (dlat == 4 && rad == 58.0)
        std::printf("K2 cap (4°, 0°) radius 58°: Contains(face 0)=%d\n", c.Contains(face0));
    }
  }
  std::printf("K2 Contains(face 0) true in %d of 65\n", k2_true);

  // K3: MayIntersect and the coverer.
  S2Cap k3(LL(0, 50), S1Angle::Degrees(2));
  S2RegionCoverer::Options opt;
  opt.set_max_cells(8);
  S2RegionCoverer coverer(opt);
  std::vector<S2CellId> cov;
  coverer.GetCovering(k3, &cov);
  int face0n = 0;
  for (auto id : cov) face0n += id.face() == 0;
  std::printf("K3 MayIntersect(face 0)=%d; covering %zu cells, %d on face 0\n",
              k3.MayIntersect(face0), cov.size(), face0n);

  // K4: Complement's length² for fixed radii.
  for (double l2 : {0.1, 0.5, 1.0, 1.7, 2.9, 3.3}) {
    S2Cap c(LL(10, 20), S1ChordAngle::FromLength2(l2));
    std::printf("K4 l2=%.17g complement l2=%.17g\n", l2, c.Complement().radius().length2());
  }

  // K5: the rect bound of the tiny cap at longitude 1e-14°.
  S2Cap k5(LL(0, 1e-14), S1Angle::Radians(1e-18));
  S2LatLngRect rb = k5.GetRectBound();
  std::printf("K5 rect lng=[%.17g, %.17g] contains centre=%d\n", rb.lng().lo(), rb.lng().hi(),
              rb.Contains(k5.center()));

  // K6: ApproxEquals with centres 5e-15 rad apart and equal radii.
  S2Point a = LL(10, 20);
  S2Point b = S2LatLng::FromRadians(S2LatLng(a).lat().radians() + 5e-15,
                                    S2LatLng(a).lng().radians()).ToPoint();
  S2Cap ca(a, S1ChordAngle::FromLength2(1)), cb(b, S1ChordAngle::FromLength2(1));
  std::printf("K6 centres %.3g rad apart: ApproxEquals=%d\n", S1Angle(a, b).radians(),
              ca.ApproxEquals(cb));
  return 0;
}
