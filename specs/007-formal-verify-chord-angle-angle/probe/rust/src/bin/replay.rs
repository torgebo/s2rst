// Replays the R0 counterexamples of spec 007 natively (research R0), to tell the library's
// behaviour from the checker's model. Run: cargo run --release -q --bin replay
use s2rst::s1::Angle;
use std::f64::consts::PI;

fn normalized_draft(x: f64) -> f64 {
    let y = 2.0 * PI;
    let r = x % y;
    if r > PI { r - y } else if r <= -PI { r + y } else { r }
}

fn f(b: [u8; 8]) -> f64 { f64::from_le_bytes(b) }

fn main() {
    let x = f([233, 59, 154, 14, 21, 56, 153, 74]);
    let n = normalized_draft(x);
    println!("draft_range x={x:e} ({:016x}) -> {n:e} in(-pi,pi]={}", x.to_bits(), -PI < n && n <= PI);
    let x = f([242, 197, 204, 239, 175, 134, 59, 0]);
    let n = normalized_draft(x);
    println!("draft_idempotent x={x:e} -> {n:e}, again {:e}, same_bits={}", normalized_draft(n), normalized_draft(n).to_bits() == n.to_bits());
    let x = f([83, 9, 15, 60, 164, 223, 85, 195]);
    let n = Angle::from_radians(x).normalized().radians();
    println!("library_range x={x:e} -> {n:e} in(-pi,pi]={} draft={:e}", -PI < n && n <= PI, normalized_draft(x));
    let v = f([0, 0, 0, 0, 0, 0, 248, 63]);
    println!("round_ties_even({v}) = {}, floor = {}, floor % 2 = {}", v.round_ties_even(), v.floor(), v.floor() % 2.0);
}
