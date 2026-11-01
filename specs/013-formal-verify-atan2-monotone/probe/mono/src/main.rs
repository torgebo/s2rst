//! Is f64::atan2 monotone in the exact angle of its (finite) arguments, for x > 0?
//! Exact order of y1/x1 vs y2/x2 (x > 0): sign of y1*x2 - y2*x1, in big integers.
use num_bigint::BigInt;
fn exact(v: f64) -> (BigInt, i32) {
    let b = v.to_bits();
    let s = if b >> 63 == 1 { -1 } else { 1 };
    let e = ((b >> 52) & 0x7ff) as i32;
    let m = b & ((1u64 << 52) - 1);
    let (m, e) = if e == 0 { (m, -1074) } else { (m | (1 << 52), e - 1075) };
    (BigInt::from(s) * BigInt::from(m), e)
}
fn mul(a: f64, b: f64) -> (BigInt, i32) {
    let (ma, ea) = exact(a);
    let (mb, eb) = exact(b);
    (ma * mb, ea + eb)
}
fn cmp(p: (BigInt, i32), q: (BigInt, i32)) -> std::cmp::Ordering {
    let e = p.1.min(q.1);
    let a = p.0 << ((p.1 - e) as usize);
    let b = q.0 << ((q.1 - e) as usize);
    a.cmp(&b)
}
struct R(u64);
impl R { fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 } 
  fn f(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 } }
fn main() {
    let n: u64 = std::env::args().nth(1).map_or(10_000_000, |s| s.parse().unwrap());
    let mut r = R(0x9e3779b97f4a7c15);
    let (mut viol, mut worst) = (0u64, 0i64);
    let mut ex = None;
    for _ in 0..n {
        let x1 = r.f() * 2.0 + 1e-3;
        let y1 = (r.f() * 2.0 - 1.0) * 3.0;
        let d = |r: &mut R, v: f64| { let k = (r.next() % 9) as i64 - 4; f64::from_bits((v.to_bits() as i64 + k) as u64) };
        let (x2, y2) = (d(&mut r, x1), d(&mut r, y1));
        let o = cmp(mul(y1, x2), mul(y2, x1)); // y1/x1 vs y2/x2
        let (a1, a2) = (y1.atan2(x1), y2.atan2(x2));
        let bad = match o { std::cmp::Ordering::Less => a1 > a2, std::cmp::Ordering::Greater => a1 < a2, _ => a1 != a2 };
        if bad {
            viol += 1;
            let u = (a1.to_bits() as i64 - a2.to_bits() as i64).abs();
            if u > worst { worst = u; ex = Some((y1, x1, y2, x2, a1, a2, o)); }
        }
    }
    println!("pairs {n} violations {viol} worst {worst} ulp");
    if let Some(e) = ex { println!("{e:?}"); }
}
