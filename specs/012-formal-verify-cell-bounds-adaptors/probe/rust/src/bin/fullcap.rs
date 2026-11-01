//! Spec 012: the full rectangle's cap bound (the empty intersection's).
use s2rst::s2::Rect;
fn main() {
    let c = Rect::full().cap_bound();
    println!("full: is_full {} length2 {:e} center {:?}", c.is_full(), c.chord_radius().length2(), c.center());
    let e = Rect::empty().cap_bound();
    println!("empty: is_empty {}", e.is_empty());
}
