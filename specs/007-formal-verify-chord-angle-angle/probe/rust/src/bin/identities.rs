// Upstream's documented degree–radian identities (s1angle.h:58–72), scaled by powers of two:
// the range of exponents where each holds (spec 007 P-AGCNV-02). Run:
// cargo run --release -q --bin identities
use s2rst::s1::Angle;
use std::f64::consts::PI;

fn main() {
    for k in 0..=8i32 {
        let (mut lo, mut hi) = (None, None);
        for j in -1100..=1100i32 {
            let s = 2f64.powi(j);
            let ok = Angle::from_degrees(45.0 * f64::from(k) * s).radians()
                == f64::from(k) * PI / 4.0 * s;
            if ok && lo.is_none() { lo = Some(j); }
            if ok { hi = Some(j); }
        }
        let back = Angle::from_degrees(45.0 * f64::from(k)).degrees() == 45.0 * f64::from(k);
        println!("k={k}: Degrees(45k·2^j) == Radians(kπ/4·2^j) for j in {lo:?}..={hi:?}; Degrees(45k).degrees() == 45k: {back}");
    }
    println!("Degrees(180) == Radians(PI): {}", Angle::from_degrees(180.0).radians() == PI);
}
