//! Survey probe for spec 010 (`LatLng`, and `s2::Rect` as a `Region`), 2026-10-27. Library
//! side; `../cpp/survey.cc` runs the same cases on upstream C++. Both print one line per case
//! (`<section> <index> <values as hex bits or 0/1>`), so `compare.py` can diff them line by
//! line. The inputs come from the same 64-bit LCG in both programs.

use s2rst::r1;
use s2rst::s1::{self, Angle};
use s2rst::s2::{Cell, CellId, CellVertex, Face, LatLng, Level, Point, Rect, Region};
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
    /// A double in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0
    }
    /// A double in [lo, hi).
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

fn b(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn ll_line(tag: &str, i: usize, ll: LatLng) {
    println!("{tag} {i} {} {}", b(ll.lat.radians()), b(ll.lng.radians()));
}

fn pt_line(tag: &str, i: usize, p: Point) {
    println!("{tag} {i} {} {} {}", b(p.x()), b(p.y()), b(p.z()));
}

fn random_rect(g: &mut Lcg) -> Rect {
    let a = g.range(-FRAC_PI_2, FRAC_PI_2);
    let c = g.range(-FRAC_PI_2, FRAC_PI_2);
    let (lo, hi) = if a <= c { (a, c) } else { (c, a) };
    let l1 = g.range(-PI, PI);
    let l2 = g.range(-PI, PI);
    Rect::new(r1::Interval::new(lo, hi), s1::Interval::new(l1, l2))
}

fn random_cell(g: &mut Lcg, max_level: u64) -> Cell {
    let face = (g.next() % 6) as usize;
    let pos = g.next() & ((1u64 << 61) - 1);
    let level = (g.next() % (max_level + 1)) as u8;
    Cell::from(CellId::from_face_pos_level(
        Face::ALL[face],
        pos,
        Level::try_from(level).expect("level in range"),
    ))
}

fn main() {
    // N: Normalized(): special longitudes, then random ones at six scales, with lat 0.3.
    let mut specials = vec![
        PI,
        -PI,
        3.0 * PI,
        -3.0 * PI,
        5.0 * PI,
        2.0 * PI,
        -2.0 * PI,
        PI + 1e-15,
        1e10,
        1e16,
        -2.462_748_394_469_102e16,
        1e300,
        0.0,
        -0.0,
    ];
    let mut g = Lcg(0x0010_0010_0010_0010);
    for &scale in &[4.0, 100.0, 1e6, 1e12, 1e16, 1e300] {
        for _ in 0..2000 {
            specials.push(g.range(-scale, scale));
        }
    }
    for (i, &v) in specials.iter().enumerate() {
        ll_line("N", i, LatLng::from_radians(0.3, v).normalized());
    }
    // NL: latitude clamping and non-finite inputs.
    let lats = [2.0, -2.0, FRAC_PI_2, f64::NAN, f64::INFINITY, -f64::INFINITY];
    for (i, &lat) in lats.iter().enumerate() {
        ll_line("NL", i, LatLng::from_radians(lat, 1.0).normalized());
    }

    // P: ToPoint() over random valid LatLngs, then the poles and the antimeridian.
    let mut g = Lcg(0x0020_0020_0020_0020);
    for i in 0..20000 {
        let ll = LatLng::from_radians(g.range(-FRAC_PI_2, FRAC_PI_2), g.range(-PI, PI));
        pt_line("P", i, ll.to_point());
    }
    for (i, &(lat, lng)) in [(FRAC_PI_2, 0.0), (-FRAC_PI_2, 1.0), (0.0, PI), (0.0, -PI)]
        .iter()
        .enumerate()
    {
        pt_line("PS", i, LatLng::from_radians(lat, lng).to_point());
    }

    // L: S2LatLng(p) over random points (from random LatLngs at doubled range, so that
    // the points are not round trips of valid inputs), and special vectors.
    let mut g = Lcg(0x0030_0030_0030_0030);
    for i in 0..20000 {
        let p = LatLng::from_radians(g.range(-PI, PI), g.range(-4.0, 4.0)).to_point();
        ll_line("L", i, LatLng::from_point(p));
    }
    let vecs = [
        (0.0, 0.0, 1.0),
        (0.0, 0.0, -1.0),
        (-1.0, -0.0, 0.0),
        (-1.0, 0.0, 0.0),
        (-1.0, -1e-300, 0.0),
        (0.0, 0.0, 0.0),
        (f64::NAN, 0.0, 1.0),
        (f64::INFINITY, 1.0, 1.0),
        (1e-320, 1e-320, 1e-320),
    ];
    let mut valid_all = true;
    for (i, &(x, y, z)) in vecs.iter().enumerate() {
        let p = Point(s2rst::r3::Vector { x, y, z });
        let ll = LatLng::from_point(p);
        ll_line("LS", i, ll);
        if x.is_finite() && y.is_finite() && z.is_finite() {
            valid_all &= ll.is_valid();
        }
    }
    // LV: is S2LatLng(p) valid for every finite point? (Library only; C++ is the same formula.)
    let mut g = Lcg(0x0031_0031_0031_0031);
    let mut invalid = 0;
    for _ in 0..200_000 {
        let v = s2rst::r3::Vector {
            x: g.range(-1.0, 1.0) * 10f64.powi((g.next() % 600) as i32 - 300),
            y: g.range(-1.0, 1.0) * 10f64.powi((g.next() % 600) as i32 - 300),
            z: g.range(-1.0, 1.0) * 10f64.powi((g.next() % 600) as i32 - 300),
        };
        if !LatLng::from_point(Point(v)).is_valid() {
            invalid += 1;
        }
    }
    eprintln!("LV from_point invalid for {invalid} of 200000 random finite vectors; specials valid: {valid_all}");

    // D: GetDistance() over random valid pairs, then pairs with an out-of-range latitude.
    let mut g = Lcg(0x0040_0040_0040_0040);
    for i in 0..20000 {
        let a = LatLng::from_radians(g.range(-FRAC_PI_2, FRAC_PI_2), g.range(-PI, PI));
        let c = LatLng::from_radians(g.range(-FRAC_PI_2, FRAC_PI_2), g.range(-PI, PI));
        println!("D {i} {}", b(a.get_distance(c).radians()));
    }
    let bad = [
        ((2.0, 0.0), (0.0, 0.0)),
        ((2.0, 0.0), (0.0, 1.0)),
        ((-2.0, 0.5), (1.0, 2.0)),
        ((f64::NAN, 0.0), (0.0, 0.0)),
        ((0.0, 4.0), (0.0, -4.0)),
        // One point, written twice: rounding makes the haversine term negative.
        ((3.124_793_493_547_795, 0.0), (0.016_799_160_041_997_9, PI)),
    ];
    for (i, &((a1, a2), (c1, c2))) in bad.iter().enumerate() {
        let d = LatLng::from_radians(a1, a2).get_distance(LatLng::from_radians(c1, c2));
        println!("DS {i} {}", b(d.radians()));
    }

    // S: ToStringInDegrees().
    for (i, &(lat, lng)) in [(0.3, PI), (0.3, -PI), (95.0, 190.0), (-0.0, -0.0), (1e-7, 3e-7)]
        .iter()
        .enumerate()
    {
        println!("S {i} {}", LatLng::from_degrees(lat, lng).to_string_in_degrees());
    }

    // R: S2LatLngRect as an S2Region: Contains(cell), MayIntersect(cell), random rects ×
    // random cells (levels 0..=8).
    let mut g = Lcg(0x0050_0050_0050_0050);
    for i in 0..20000 {
        let r = random_rect(&mut g);
        let c = random_cell(&mut g, 8);
        println!(
            "R {i} {} {}",
            u8::from(r.contains_cell(&c)),
            u8::from(r.intersects_cell(&c))
        );
    }

    // G: the bounding cap contains every contained point: rects × points on and near their
    // boundaries (corners, edge points, centre) and random points.
    let mut g = Lcg(0x0060_0060_0060_0060);
    let mut viol = 0;
    let mut checked = 0;
    let mut miss = std::collections::BTreeMap::new();
    let mut max_ulps = 0.0_f64;
    for i in 0..20000 {
        let r = random_rect(&mut g);
        let cap = r.cap_bound();
        let lat = g.range(r.lat.lo, r.lat.hi);
        let t = g.unit();
        let lng = if r.lng.is_inverted() {
            let len = r.lng.length();
            let v = r.lng.lo + t * len;
            if v > PI { v - 2.0 * PI } else { v }
        } else {
            r.lng.lo + t * (r.lng.hi - r.lng.lo)
        };
        let cands = [
            LatLng::from_radians(r.lat.lo, r.lng.lo),
            LatLng::from_radians(r.lat.hi, r.lng.hi),
            LatLng::from_radians(r.lat.lo, lng),
            LatLng::from_radians(r.lat.hi, lng),
            LatLng::from_radians(lat, r.lng.lo),
            LatLng::from_radians(lat, r.lng.hi),
            LatLng::from_radians(lat, lng),
        ];
        let mut bits = String::new();
        let pole = cap.center().x() == 0.0 && cap.center().y() == 0.0;
        for (k, ll) in cands.into_iter().enumerate() {
            let p = ll.to_point();
            let inside = r.contains_point(p);
            let in_cap = cap.contains_point(p);
            if inside {
                checked += 1;
                if !in_cap {
                    viol += 1;
                    // Which cap, which candidate, and by how much (chord length² over the
                    // cap's, in units of the cap's length²'s ulp).
                    let d2 = (p.vector() - cap.center().vector()).norm2();
                    let r2 = cap.chord_radius().length2();
                    let ulps = (d2 - r2) / (f64::EPSILON * r2);
                    *miss.entry((pole, k)).or_insert(0) += 1;
                    max_ulps = f64::max(max_ulps, ulps);
                }
            }
            bits.push(if inside { '1' } else { '0' });
            bits.push(if in_cap { '1' } else { '0' });
        }
        println!("G {i} {bits}");
    }
    eprintln!("G cap_bound misses {viol} of {checked} contained points");
    eprintln!(
        "G misses by (pole cap, candidate 0=lo/lo 1=hi/hi 2=lat.lo 3=lat.hi 4=lng.lo 5=lng.hi \
         6=inside): {miss:?}; largest excess {max_ulps:.1} ε of the cap's length²"
    );

    // V: no intersection, no contained vertex: rects × cells (levels 0..=12).
    let mut g = Lcg(0x0070_0070_0070_0070);
    let mut viol = 0;
    for i in 0..20000 {
        let r = random_rect(&mut g);
        let c = random_cell(&mut g, 12);
        let may = r.intersects_cell(&c);
        let mut bits = String::new();
        for k in CellVertex::ALL {
            let inside = r.contains_point(c.vertex(k));
            if !may && inside {
                viol += 1;
            }
            bits.push(if inside { '1' } else { '0' });
        }
        println!("V {i} {} {bits}", u8::from(may));
    }
    eprintln!("V vertices contained in a cell MayIntersect rejects: {viol}");

    // Angle sanity for the reader: Angle::normalized maps -π to π, unlike S2LatLng.
    eprintln!(
        "Angle::normalized(-π) = {}, LatLng::normalized lng for -π = {}",
        Angle::from_radians(-PI).normalized().radians(),
        LatLng::from_radians(0.0, -PI).normalized().lng.radians()
    );
}
