//! Survey probe for spec 012 (`Cell`'s bounding laws, and the region adaptors), 2026-10-29.
//! Library side; `../cpp/survey.cc` runs the same cases on upstream C++, with the same LCG
//! inputs and line format, so `compare.py` can diff them line by line.

use s2rst::r1;
use s2rst::r3::Vector;
use s2rst::s1::{self, Angle, ChordAngle};
use s2rst::s2::coords::face_uv_to_xyz;
use s2rst::s2::point_region::PointRegion;
use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;
use s2rst::s2::{Cap, Cell, CellId, Face, Level, Point, Rect, Region};
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

fn b(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn cap_s(c: Cap) -> String {
    let p = c.center();
    format!(
        "{} {} {} {}",
        b(p.x()),
        b(p.y()),
        b(p.z()),
        b(c.chord_radius().length2())
    )
}

fn rect_s(r: Rect) -> String {
    format!(
        "{} {} {} {}",
        b(r.lat.lo),
        b(r.lat.hi),
        b(r.lng.lo),
        b(r.lng.hi)
    )
}

fn random_point(g: &mut Lcg) -> Point {
    loop {
        let (x, y, z) = (g.range(-1.0, 1.0), g.range(-1.0, 1.0), g.range(-1.0, 1.0));
        let n2 = x * x + y * y + z * z;
        if n2 > 1e-6 && n2 <= 1.0 {
            return Point(Vector { x, y, z }.normalize());
        }
    }
}

fn random_id(g: &mut Lcg, min_level: u64, max_level: u64) -> CellId {
    let face = (g.next() % 6) as usize;
    let pos = g.next() & ((1u64 << 61) - 1);
    let level = (min_level + g.next() % (max_level - min_level + 1)) as u8;
    CellId::from_face_pos_level(Face::ALL[face], pos, Level::new(level))
}

fn random_region(g: &mut Lcg) -> Box<dyn Region> {
    match g.next() % 4 {
        0 => {
            let c = random_point(g);
            let r = g.range(0.0, 1.5);
            Box::new(Cap::from_center_chord_angle(
                c,
                ChordAngle::from_angle(Angle::from_radians(r)),
            ))
        }
        1 => {
            let a = g.range(-FRAC_PI_2, FRAC_PI_2);
            let c = g.range(-FRAC_PI_2, FRAC_PI_2);
            let l1 = g.range(-PI, PI);
            let l2 = g.range(-PI, PI);
            Box::new(Rect::new(
                r1::Interval::new(a.min(c), a.max(c)),
                s1::Interval::new(l1, l2),
            ))
        }
        2 => Box::new(Cell::from(random_id(g, 0, 12))),
        _ => Box::new(PointRegion::new(random_point(g))),
    }
}

fn main() {
    // CB: Cell's bounds against adversarial points around its corners and edges.
    let mut g = Lcg(0x0210_0021_0021_0021);
    let offs = [0.0, 0.5, 1.0, 1.5, 5.0 / 3.0 - 1e-3, 5.0 / 3.0 + 1e-3, 2.0];
    let (mut contained, mut cap_miss, mut rect_miss) = (0i64, 0i64, 0i64);
    for i in 0..20000 {
        let c = Cell::from(random_id(&mut g, 0, 30));
        let uv = c.bound_uv();
        let cap = c.cap_bound();
        let rect = c.rect_bound();
        let mut s = String::new();
        let (mut cm, mut rm, mut n) = (0, 0, 0);
        let mut test = |u: f64, v: f64| {
            let p = Point(face_uv_to_xyz(c.face(), u, v).normalize());
            let inside = c.contains_point(p);
            let ic = cap.contains_point(p);
            let ir = rect.contains_point(p);
            s += if inside {
                match (ic, ir) {
                    (true, true) => "3",
                    (true, false) => "2",
                    (false, true) => "1",
                    (false, false) => "0",
                }
            } else {
                "x"
            };
            if inside {
                n += 1;
                if !ic {
                    cm += 1;
                }
                if !ir {
                    rm += 1;
                }
            }
        };
        for &o in &offs {
            let d = o * f64::EPSILON;
            for k in 0..4 {
                let u = if k == 0 || k == 3 { uv.x.lo - d } else { uv.x.hi + d };
                let v = if k < 2 { uv.y.lo - d } else { uv.y.hi + d };
                test(u, v);
            }
            let (um, vm) = (uv.x.center(), uv.y.center());
            test(uv.x.lo - d, vm);
            test(uv.x.hi + d, vm);
            test(um, uv.y.lo - d);
            test(um, uv.y.hi + d);
        }
        for _ in 0..4 {
            let u = g.range(uv.x.lo, uv.x.hi);
            let v = g.range(uv.y.lo, uv.y.hi);
            test(u, v);
        }
        contained += n;
        cap_miss += cm;
        rect_miss += rm;
        println!("CB {i} {} {} {s}", cap_s(cap), rect_s(rect));
    }
    eprintln!(
        "CB contained points {contained}, outside the cap bound {cap_miss}, outside the rect \
         bound {rect_miss}"
    );

    // RU, RI: the region adaptors over 1 to 4 mixed members.
    for kind in 0..2 {
        g = Lcg(if kind == 0 {
            0x0220_0022_0022_0022
        } else {
            0x0230_0023_0023_0023
        });
        for i in 0..5000 {
            let n = 1 + g.next() % 4;
            let regions: Vec<Box<dyn Region>> = (0..n).map(|_| random_region(&mut g)).collect();
            let cell = Cell::from(random_id(&mut g, 0, 12));
            let p = random_point(&mut g);
            let r: Box<dyn Region> = if kind == 0 {
                Box::new(RegionUnion::from_regions(regions))
            } else {
                Box::new(RegionIntersection::from_regions(regions))
            };
            println!(
                "{} {i} {} {} {} {} {} {}",
                if kind == 0 { "RU" } else { "RI" },
                cap_s(r.cap_bound()),
                rect_s(r.rect_bound()),
                r.cell_union_bound().len(),
                u8::from(r.contains_cell(&cell)),
                u8::from(r.intersects_cell(&cell)),
                u8::from(r.contains_point(&p))
            );
        }
    }
}
