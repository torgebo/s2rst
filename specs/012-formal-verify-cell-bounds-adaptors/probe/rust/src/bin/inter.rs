//! Spec 012 R0c: the replay of `p_adbnd_01__n1`'s counterexample: the full rectangle intersected
//! with a member rectangle is not contained in the member.
use s2rst::s2::Rect;
use s2rst::{r1, s1};
fn main() {
    let lo = f64::from_bits(0x0009_21fb_5444_2d19);
    let hi = f64::from_bits(0x8009_21fb_5444_2d19);
    let m = Rect { lat: r1::Interval { lo: 0.0, hi: -0.0 }, lng: s1::Interval { lo, hi } };
    let i = Rect::full().intersection(m);
    println!("m lng [{:e}, {:e}] valid {}", m.lng.lo, m.lng.hi, m.is_valid());
    println!("i lat [{:e}, {:e}] lng [{:e}, {:e}]", i.lat.lo, i.lat.hi, i.lng.lo, i.lng.hi);
    println!("m.contains(i) {} lat {} lng {}", m.contains(i), m.lat.contains_interval(i.lat), m.lng.contains_interval(i.lng));
}
