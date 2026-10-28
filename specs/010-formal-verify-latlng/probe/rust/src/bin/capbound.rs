//! Planning probe for spec 010 FR-019 (research R0, 2026-10-28): how far outside
//! `Rect::cap_bound` the points a rectangle contains can lie, for the pole cap and for the cap
//! around the centre, with adversarial points (the latitude bounds, the vertices and points a
//! few ulps from them, the poles), and rectangles near the poles.

use s2rst::r1;
use s2rst::s1::{self, Angle, ChordAngle};
use s2rst::s2::{Cap, LatLng, Point, Rect};
use std::f64::consts::{FRAC_PI_2, PI};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

fn up(x: f64, k: i64) -> f64 {
    let mut y = x;
    for _ in 0..k.unsigned_abs() {
        y = if k > 0 { y.next_up() } else { y.next_down() };
    }
    y
}

/// A rectangle from one of several families.
fn rect(g: &mut Lcg) -> Rect {
    let fam = g.next() % 6;
    let (lo, hi) = match fam {
        0 => {
            let a = g.range(-FRAC_PI_2, FRAC_PI_2);
            let c = g.range(-FRAC_PI_2, FRAC_PI_2);
            (a.min(c), a.max(c))
        }
        1 => {
            // near the north pole: lat.lo within 10^-k of π/2
            let k = (g.next() % 17) as i32;
            let lo = FRAC_PI_2 - g.unit() * 10f64.powi(-k);
            (lo, if g.next() % 2 == 0 { FRAC_PI_2 } else { g.range(lo, FRAC_PI_2) })
        }
        2 => {
            let k = (g.next() % 17) as i32;
            let hi = -FRAC_PI_2 + g.unit() * 10f64.powi(-k);
            (if g.next() % 2 == 0 { -FRAC_PI_2 } else { g.range(-FRAC_PI_2, hi) }, hi)
        }
        3 => (FRAC_PI_2, FRAC_PI_2),
        4 => {
            let a = g.range(-FRAC_PI_2, FRAC_PI_2);
            (a, a)
        }
        _ => {
            let a = g.range(0.0, FRAC_PI_2);
            (-a, a)
        }
    };
    let lfam = g.next() % 4;
    let lng = match lfam {
        0 => s1::Interval::full(),
        1 => {
            let c = g.range(-PI, PI);
            let w = g.unit() * 10f64.powi(-((g.next() % 12) as i32));
            s1::Interval::from_point_pair(c, (c + w).min(PI))
        }
        _ => s1::Interval::new(g.range(-PI, PI), g.range(-PI, PI)),
    };
    Rect::new(r1::Interval::new(lo, hi), lng)
}

fn lng_in(r: &Rect, t: f64) -> f64 {
    if r.lng.is_inverted() {
        let v = r.lng.lo + t * r.lng.length();
        if v > PI { v - 2.0 * PI } else { v }
    } else {
        r.lng.lo + t * (r.lng.hi - r.lng.lo)
    }
}

/// The library's cap_bound, with the pole cap's radius from `pole(angle)`.
fn cap_bound_with(r: Rect, pole: &dyn Fn(f64) -> ChordAngle, mid_err: f64) -> Cap {
    if r.is_empty() {
        return Cap::empty();
    }
    let (pole_z, pole_angle) = if r.lat.hi + r.lat.lo < 0.0 {
        (-1.0, FRAC_PI_2 + r.lat.hi)
    } else {
        (1.0, FRAC_PI_2 - r.lat.lo)
    };
    let pole_cap = Cap::from_center_chord_angle(
        Point(s2rst::r3::Vector { x: 0.0, y: 0.0, z: pole_z }),
        pole(pole_angle),
    );
    if r.lng.length() <= PI {
        let mut mid = Cap::from_point(r.center().to_point());
        for v in s2rst::s2::RectVertex::iter() {
            mid = mid.add_point(r.vertex(v).to_point());
        }
        let mid = if mid_err > 0.0 {
            Cap::from_center_chord_angle(mid.center(), mid.chord_radius().plus_error(mid_err * mid.chord_radius().length2()))
        } else {
            mid
        };
        if mid.height() < pole_cap.height() {
            return mid;
        }
    }
    pole_cap
}

/// The library's cap_bound, then both caps expanded by `delta` radians and their length² by
/// `rel` ε of itself.
fn widened(r: Rect, delta: f64, rel: f64) -> Cap {
    if r.is_empty() {
        return Cap::empty();
    }
    let w = |c: Cap| -> Cap {
        if delta == 0.0 && rel == 0.0 { return c; }
        let c = c.expanded(Angle::from_radians(delta));
        let r2 = c.chord_radius().length2();
        Cap::from_center_chord_angle(c.center(), c.chord_radius().plus_error(rel * f64::EPSILON * r2))
    };
    let (pole_z, pole_angle) = if r.lat.hi + r.lat.lo < 0.0 {
        (-1.0, FRAC_PI_2 + r.lat.hi)
    } else {
        (1.0, FRAC_PI_2 - r.lat.lo)
    };
    let pole_cap = Cap::from_center_angle(
        Point(s2rst::r3::Vector { x: 0.0, y: 0.0, z: pole_z }),
        Angle::from_radians((1.0 + 2.0 * f64::EPSILON) * pole_angle),
    );
    if r.lng.length() <= PI {
        let mut mid = Cap::from_point(r.center().to_point());
        for v in s2rst::s2::RectVertex::iter() {
            mid = mid.add_point(r.vertex(v).to_point());
        }
        if mid.height() < pole_cap.height() {
            return w(mid);
        }
    }
    w(pole_cap)
}

fn main() {
    let upstream = |a: f64| ChordAngle::from_angle(Angle::from_radians((1.0 + 2.0 * f64::EPSILON) * a));
    let mut g = Lcg(0x0100_0100_0100_0100);
    let n: usize = std::env::args().nth(1).map_or(200_000, |s| s.parse().unwrap());
    let (mut checked, mut miss_pole, mut miss_mid) = (0u64, 0u64, 0u64);
    let (mut max_rel_pole, mut max_abs_pole, mut max_rel_mid, mut max_abs_mid) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut worst_pole = String::new();
    let mut worst_mid = String::new();
    // candidate bounds: (name, delta radians, relative chord error in eps)
    let cands: Vec<(&str, f64, f64)> = vec![
        ("upstream", 0.0, 0.0),
        ("2.5e-16 + 4eps", 2.5e-16, 4.0),
        ("1e-15 + 8eps", 1e-15, 8.0),
    ];
    let mut cand_miss = vec![0u64; cands.len()];
    let mut cand_miss_mid_err = vec![0u64; cands.len()];
    for _ in 0..n {
        let r = rect(&mut g);
        if r.is_empty() {
            continue;
        }
        let caps: Vec<Cap> = cands.iter().map(|&(_, d, e)| widened(r, d, e)).collect();
        let caps_m: Vec<Cap> = caps.clone();
        let lib = r.cap_bound();
        // Before FR-019 the library's bound was candidate 0 (upstream's), after it candidate 2.
        let lib_is = |i: usize| lib.chord_radius().length2().to_bits() == caps[i].chord_radius().length2().to_bits();
        assert!(lib_is(0) || lib_is(2));
        let _ = cap_bound_with(r, &upstream, 0.0);
        let pole = lib.center().x() == 0.0 && lib.center().y() == 0.0;
        let mut pts: Vec<LatLng> = Vec::new();
        let lats = [r.lat.lo, r.lat.hi, up(r.lat.lo, 1), up(r.lat.hi, -1), up(r.lat.lo, 3), up(r.lat.hi, -3), g.range(r.lat.lo, r.lat.hi), FRAC_PI_2, -FRAC_PI_2];
        let mut lngs = vec![r.lng.lo, r.lng.hi, lng_in(&r, g.unit()), lng_in(&r, g.unit()), lng_in(&r, 0.5)];
        for k in [1i64, 2, 4, 16] {
            lngs.push(up(r.lng.lo, k));
            lngs.push(up(r.lng.hi, -k));
        }
        for &la in &lats {
            for &ln in &lngs {
                pts.push(LatLng::from_radians(la, ln));
            }
        }
        for ll in pts {
            let p = ll.to_point();
            if !r.contains_point(p) {
                continue;
            }
            checked += 1;
            if !lib.contains_point(p) {
                let d2 = lib.center().chord_angle(p).length2();
                let r2 = lib.chord_radius().length2();
                let rel = (d2 - r2) / (f64::EPSILON * r2.max(f64::MIN_POSITIVE));
                let abs = d2 - r2;
                let s = format!("rect lat [{:e}, {:e}] lng [{:e}, {:e}] pt ({:e}, {:e}) r2 {:e} d2 {:e}", r.lat.lo, r.lat.hi, r.lng.lo, r.lng.hi, ll.lat.radians(), ll.lng.radians(), r2, d2);
                if pole {
                    miss_pole += 1;
                    if abs > max_abs_pole { max_abs_pole = abs; }
                    if rel > max_rel_pole { max_rel_pole = rel; worst_pole = s; }
                } else {
                    miss_mid += 1;
                    if abs > max_abs_mid { max_abs_mid = abs; }
                    if rel > max_rel_mid { max_rel_mid = rel; worst_mid = s; }
                }
            }
            for (i, c) in caps.iter().enumerate() {
                if !c.contains_point(p) { cand_miss[i] += 1; }
            }
            for (i, c) in caps_m.iter().enumerate() {
                let is_pole = c.center().x() == 0.0 && c.center().y() == 0.0;
                if !c.contains_point(p) && is_pole { cand_miss_mid_err[i] += 1; }
            }
        }
    }
    println!("checked {checked} contained points of {n} rectangles");
    println!("library misses: pole cap {miss_pole} (max {max_rel_pole:.1} eps of r2, abs {max_abs_pole:e}), mid cap {miss_mid} (max {max_rel_mid:.1} eps of r2, abs {max_abs_mid:e})");
    println!("worst pole: {worst_pole}");
    println!("worst mid: {worst_mid}");
    for (i, (name, _, _)) in cands.iter().enumerate() {
        println!("candidate {name}: misses {} (pole caps {})", cand_miss[i], cand_miss_mid_err[i]);
    }
}
