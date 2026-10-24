// Exhaustive check over every i32 of the E-form round trips and upstream's identities (research
// R0): the domain is finite, so enumerating it is a proof on this machine's IEEE-754 arithmetic.
// Run: cargo run --release -q --bin exhaust
use s2rst::s1::Angle;
use std::time::Instant;

fn e(a: Angle, k: f64) -> i32 { (a.degrees() * k).round_ties_even() as i32 }

fn main() {
    let t = Instant::now();
    let (mut bad5, mut bad6, mut bad7, mut lib7, mut id67) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut first7 = None;
    for n in i32::MIN..=i32::MAX {
        if e(Angle::from_e5(n), 1e5) != n { bad5 += 1; }
        if e(Angle::from_e6(n), 1e6) != n { bad6 += 1; }
        if e(Angle::from_e7(n), 1e7) != n { bad7 += 1; if first7.is_none() { first7 = Some(n); } }
        if Angle::from_e7(n).e7() != n { lib7 += 1; }
        if n > i32::MIN / 10 && n < i32::MAX / 10 && Angle::from_e6(n) != Angle::from_e7(10 * n) { id67 += 1; }
    }
    let (b6, b7) = identities();
    println!("Degrees(n) != E6(1e6 n): {b6}; Degrees(n) != E7(1e7 n): {b7}");
    println!("ties-even round trips failing: e5 {bad5}, e6 {bad6}, e7 {bad7} (first {first7:?}); library e7 {lib7}; E6(n) != E7(10n): {id67}; {:.1} s", t.elapsed().as_secs_f64());
}

// Upstream's other integer identities (s1angle.h:50–51): Degrees(n) == E6(10⁶ n) and
// Degrees(n) == E7(10⁷ n), for every n whose product fits an i32.
fn identities() -> (u32, u32) {
    let (mut b6, mut b7) = (0, 0);
    for n in -2147..=2147i32 {
        if Angle::from_degrees(f64::from(n)) != Angle::from_e6(n * 1_000_000) { b6 += 1; }
        if (-214..=214).contains(&n) && Angle::from_degrees(f64::from(n)) != Angle::from_e7(n * 10_000_000) { b7 += 1; }
    }
    (b6, b7)
}
