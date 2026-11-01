//! Spec 013 survey: Cell::rect_bound against the cell's points as directions and normalised;
//! the quadrant premise; atan2's dominance order per quadrant.
use s2rst::s2::{Cell, CellId, Face, Level, Point};
use s2rst::s2::coords::face_uv_to_xyz;
struct R(u64);
impl R {
    fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn f(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
}
fn main() {
    let mut r = R(0x5eed_0013);
    let (mut cells, mut straddle, mut dir_miss, mut norm_miss, mut pts) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for k in 0..400_000u64 {
        let face = r.next() % 6;
        let level = if k < 6 { 0 } else { (r.next() % 31) as u64 };
        let pos = r.next() & ((1u64 << 61) - 1);
        let id = CellId::from_face_pos_level(Face::from_u8(face as u8), pos, Level::new(level as u8));
        let c = Cell::from(id);
        cells += 1;
        let uv = c.bound_uv();
        if level >= 1 && ((uv.x.lo < 0.0 && uv.x.hi > 0.0) || (uv.y.lo < 0.0 && uv.y.hi > 0.0)) { straddle += 1; }
        let rb = c.rect_bound();
        let us = [uv.x.lo, uv.x.hi, 0.5 * (uv.x.lo + uv.x.hi)];
        let vs = [uv.y.lo, uv.y.hi, 0.5 * (uv.y.lo + uv.y.hi)];
        let mut check = |u: f64, v: f64| {
            let p = face_uv_to_xyz(c.face(), u, v);
            pts += 1;
            if !rb.contains_point(Point(p)) { dir_miss += 1; }
            if !rb.contains_point(Point(p.normalize())) { norm_miss += 1; }
        };
        for &u in &us { for &v in &vs { check(u, v); } }
        for _ in 0..4 {
            let u = uv.x.lo + (uv.x.hi - uv.x.lo) * r.f();
            let v = uv.y.lo + (uv.y.hi - uv.y.lo) * r.f();
            if u >= uv.x.lo && u <= uv.x.hi && v >= uv.y.lo && v <= uv.y.hi { check(u, v); }
        }
    }
    println!("cells {cells} straddling (level>=1) {straddle} points {pts} outside as directions {dir_miss} outside normalised {norm_miss}");
    // atan2 per-quadrant dominance: same quadrant (sign bits), y1<=y2 and x1>=x2 in Q1 etc.
    let mut viol = 0u64; let mut n = 0u64;
    for _ in 0..20_000_000u64 {
        let q = r.next() % 4;
        let mag = |r: &mut R| { let e = (r.next() % 60) as i32 - 50; r.f() * 2f64.powi(e) };
        let (mut x1, mut y1) = (mag(&mut r), mag(&mut r));
        let d = |r: &mut R, v: f64| { let k = (r.next() % 9) as i64 - 4; let b = v.to_bits() as i64 + k; if b < 0 { 0.0 } else { f64::from_bits(b as u64) } };
        let (mut x2, mut y2) = (d(&mut r, x1), d(&mut r, y1));
        if r.next() % 4 == 0 { x2 = x1; } else if r.next() % 4 == 0 { y2 = y1; }
        let (sx, sy) = (if q & 1 == 1 { -1.0 } else { 1.0 }, if q & 2 == 2 { -1.0 } else { 1.0 });
        x1 *= sx; x2 *= sx; y1 *= sy; y2 *= sy;
        // order: angle1 <= angle2 implied by dominance in the quadrant
        let ok_pre = match q {
            0 => y1 <= y2 && x1 >= x2,          // Q1: up in y, down in x
            1 => y1 >= y2 && x1 >= x2,          // Q2 (x<=0,y>=0): down in y, down in x
            2 => y1 <= y2 && x1 <= x2,          // Q4 (x>=0,y<=0): up in y, up in x
            _ => y1 >= y2 && x1 <= x2,          // Q3 (x<=0,y<=0): down in y, up in x
        };
        if !ok_pre { continue; }
        n += 1;
        if y1.atan2(x1) > y2.atan2(x2) { viol += 1; if viol < 4 { println!("viol q{q} ({y1:e},{x1:e}) ({y2:e},{x2:e})"); } }
    }
    println!("dominance pairs {n} violations {viol}");
}
