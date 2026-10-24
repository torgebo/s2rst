// Survey probe for spec 007: the library's Angle and ChordAngle, the same cases as
// ../cpp/survey.cc. Run: cargo run --release (from this directory).
use s2rst::s1::{Angle, ChordAngle};

fn main() {
    let nan = f64::NAN;
    let inf = f64::INFINITY;
    let mut shown = 0;
    for k in 0..2_000_000 {
        let d = (f64::from(k) + 0.5) / 1e6;
        let a = Angle::from_degrees(d);
        if a.degrees() * 1e6 == f64::from(k) + 0.5 {
            println!("e6_tie radians={:016x} deg*1e6={} e6={}", a.radians().to_bits(), a.degrees() * 1e6, a.e6());
            shown += 1;
            if shown == 6 { break; }
        }
    }
    shown = 0;
    for k in 0..2_000_000 {
        let d = -(f64::from(k) + 0.5) / 1e7;
        let a = Angle::from_degrees(d);
        if a.degrees() * 1e7 == -(f64::from(k) + 0.5) {
            println!("e7_tie radians={:016x} deg*1e7={} e7={}", a.radians().to_bits(), a.degrees() * 1e7, a.e7());
            shown += 1;
            if shown == 3 { break; }
        }
    }
    println!("e7_1e10rad {}", Angle::from_radians(1e10).e7());
    println!("e7_neg1e10rad {}", Angle::from_radians(-1e10).e7());
    println!("e5_inf {}", Angle::INFINITY.e5());
    println!("e5_nan {}", Angle::from_radians(nan).e5());
    println!("e6_400deg {}", Angle::from_degrees(400.0).e6());
    let pi = std::f64::consts::PI;
    for x in [3.0 * pi, -3.0 * pi, pi, -pi, 5.0 * pi, 7.0, 1e6, 1e16, 1e20, 1e300, -1e20, inf, nan] {
        let n = Angle::from_radians(x).normalized().radians();
        println!("normalized x={x:e} -> {n:e} ({:016x}) in(-pi,pi]={}", n.to_bits(), -pi < n && n <= pi);
    }
    println!("chord_from_length2_nan {}", ChordAngle::from_length2(nan).length2());
    println!("chord_from_length2_m5 {}", ChordAngle::from_length2(-5.0).length2());
    println!("chord_from_angle_m1rad {}", ChordAngle::from_angle(Angle::from_radians(-1.0)).length2());
    println!("chord_from_angle_nan {}", ChordAngle::from_angle(Angle::from_radians(nan)).length2());
    println!("chord_m5_is_valid {}", u8::from(ChordAngle::from_length2(-5.0).is_valid()));
    println!("chord_m5_predecessor {}", ChordAngle::from_length2(-5.0).predecessor().length2());
    println!("chord_m5_to_angle {}", ChordAngle::from_length2(-5.0).to_angle().radians());
    println!("chord_inf_e5 {}", ChordAngle::INFINITY.e5());
    println!("chord_inf_e7 {}", ChordAngle::INFINITY.e7());
    println!("chord_neg_e7 {}", ChordAngle::NEGATIVE.e7());
    println!("chord_plus_error_nan {}", ChordAngle::from_length2(nan).plus_error(0.0).length2());
    println!("chord_neg_inf is_negative={} is_infinity={}", ChordAngle::from_length2(-inf).is_negative(), ChordAngle::from_length2(-inf).is_infinity());
}
