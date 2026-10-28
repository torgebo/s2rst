// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
//
// Spec 010 research R0: the worst misses of ../rust/src/bin/capbound.rs, on upstream
// S2LatLngRect::GetCapBound: a pole cap (a rectangle at the north pole and a point an ulp past
// it) and a cap around the centre (a point a few ulps inside a vertex).
#include <cstdio>

#include "s2/r1interval.h"
#include "s2/s1interval.h"
#include "s2/s2cap.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"

static void check(const char* name, double lat_lo, double lat_hi, double lng_lo, double lng_hi,
                  double plat, double plng) {
  S2LatLngRect r(R1Interval(lat_lo, lat_hi), S1Interval(lng_lo, lng_hi));
  S2Point p = S2LatLng::FromRadians(plat, plng).ToPoint();
  S2Cap cap = r.GetCapBound();
  std::printf("%s: rect contains %d, cap contains %d, cap centre (%.17g, %.17g, %.17g) height %.17g\n",
              name, r.Contains(p) ? 1 : 0, cap.Contains(p) ? 1 : 0, cap.center().x(),
              cap.center().y(), cap.center().z(), cap.height());
}

int main() {
  check("pole", 1.5707963267948966, 1.5707963267948966, -3.141592653589793, 3.141592653589793,
        1.5707963267948968, 4.6539129845781746e-1);
  check("mid", 1.0712215270180785, 1.0712376688984673, -8.905155217671079e-1,
        -8.905155217089906e-1, 1.0712215270180785, -8.905155217089911e-1);
  return 0;
}
