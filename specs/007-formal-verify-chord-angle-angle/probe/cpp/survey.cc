// Survey probe for spec 007: S1Angle and S1ChordAngle on upstream C++ (Release).
// Build: see ../README.md. Prints one line per case: <case> <result>.
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>

#include "s2/s1angle.h"
#include "s2/s1chord_angle.h"

static uint64_t bits(double x) {
  uint64_t b;
  std::memcpy(&b, &x, 8);
  return b;
}

int main() {
  const double nan = std::numeric_limits<double>::quiet_NaN();
  const double inf = std::numeric_limits<double>::infinity();

  // E6/E7 rounding at exact ties: degrees() * 10^k is exactly n + 0.5.
  int shown = 0;
  for (int k = 0; k < 2000000 && shown < 6; ++k) {
    double d = (k + 0.5) / 1e6;
    S1Angle a = S1Angle::Degrees(d);
    if (1e6 * a.degrees() == k + 0.5) {
      std::printf("e6_tie radians=%016llx deg*1e6=%.17g e6=%d\n",
                  (unsigned long long)bits(a.radians()), 1e6 * a.degrees(), a.e6());
      ++shown;
    }
  }
  shown = 0;
  for (int k = 0; k < 2000000 && shown < 3; ++k) {
    double d = -(k + 0.5) / 1e7;
    S1Angle a = S1Angle::Degrees(d);
    if (1e7 * a.degrees() == -(k + 0.5)) {
      std::printf("e7_tie radians=%016llx deg*1e7=%.17g e7=%d\n",
                  (unsigned long long)bits(a.radians()), 1e7 * a.degrees(), a.e7());
      ++shown;
    }
  }

  // E5/E6/E7 outside int32 range, infinite and NaN.
  std::printf("e7_1e10rad %d\n", S1Angle::Radians(1e10).e7());
  std::printf("e7_neg1e10rad %d\n", S1Angle::Radians(-1e10).e7());
  std::printf("e5_inf %d\n", S1Angle::Infinity().e5());
  std::printf("e5_nan %d\n", S1Angle::Radians(nan).e5());
  std::printf("e6_400deg %d\n", S1Angle::Degrees(400).e6());

  // Normalized: remainder() versus x - round(x / 2pi) * 2pi.
  const double cases[] = {3 * M_PI, -3 * M_PI, M_PI, -M_PI, 5 * M_PI, 7.0,
                          1e6,      1e16,      1e20, 1e300, -1e20, inf, nan};
  for (double x : cases) {
    S1Angle n = S1Angle::Radians(x).Normalized();
    std::printf("normalized x=%.17g -> %.17g (%016llx) is_normalized=%d\n", x,
                n.radians(), (unsigned long long)bits(n.radians()),
                n.IsNormalized() ? 1 : 0);
  }

  // S1ChordAngle: the special values and NaN.
  std::printf("chord_from_length2_nan %.17g\n", S1ChordAngle::FromLength2(nan).length2());
  std::printf("chord_from_length2_m5 %.17g\n", S1ChordAngle::FromLength2(-5).length2());
  std::printf("chord_from_angle_m1rad %.17g\n", S1ChordAngle(S1Angle::Radians(-1)).length2());
  std::printf("chord_from_angle_nan %.17g\n", S1ChordAngle(S1Angle::Radians(nan)).length2());
  std::printf("chord_m5_is_valid %d\n", S1ChordAngle::FromLength2(-5).is_valid() ? 1 : 0);
  std::printf("chord_m5_predecessor %.17g\n", S1ChordAngle::FromLength2(-5).Predecessor().length2());
  std::printf("chord_m5_to_angle %.17g\n", S1ChordAngle::FromLength2(-5).ToAngle().radians());
  std::printf("chord_inf_e5 %d\n", S1ChordAngle::Infinity().e5());
  std::printf("chord_inf_e7 %d\n", S1ChordAngle::Infinity().e7());
  std::printf("chord_neg_e7 %d\n", S1ChordAngle::Negative().e7());
  std::printf("chord_plus_error_nan %.17g\n", S1ChordAngle::FromLength2(nan).PlusError(0).length2());
  return 0;
}
