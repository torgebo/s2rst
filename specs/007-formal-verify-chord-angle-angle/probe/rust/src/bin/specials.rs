// The release-build results of the chord angle's formulas on its sentinels (spec 007 FR-019),
// as the rustdoc states them. Run: cargo run --release -q --bin specials
use s2rst::s1::ChordAngle as C;

fn main() {
    for (name, c) in [("NEGATIVE", C::NEGATIVE), ("INFINITY", C::INFINITY)] {
        println!("{name}: sin2 {} sin {} cos {} tan {}", c.sin2(), c.sin(), c.cos(), c.tan());
    }
    println!("NEGATIVE + RIGHT = {}", (C::NEGATIVE + C::RIGHT).length2());
    println!("INFINITY - RIGHT = {}", (C::INFINITY - C::RIGHT).length2());
}
