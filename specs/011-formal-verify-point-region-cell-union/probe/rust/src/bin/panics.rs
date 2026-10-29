//! Spec 011 K4: the exact inputs of the debug-build panics, checked over the special triples
//! (debug build). `Cell::contains_point` for each face cell, `CellId::from_point`, and
//! `PointRegion::rect_bound`.
use s2rst::r3::Vector;
use s2rst::s2::point_region::PointRegion;
use s2rst::s2::{Cell, CellId, Face, Region};

fn panics(f: impl FnOnce() + std::panic::UnwindSafe) -> bool {
    std::panic::catch_unwind(f).is_err()
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let v = [0.0, -0.0, 1.0, -1.0, 1e-320, 1e300, -1e300, f64::NAN, f64::INFINITY, -f64::INFINITY];
    let (mut n, mut cp_bad, mut fp_bad, mut rb_bad) = (0, 0, 0, 0);
    for &x in &v {
        for &y in &v {
            for &z in &v {
                let p = s2rst::s2::Point(Vector { x, y, z });
                let c = [x, y, z];
                n += 1;
                for face in Face::ALL {
                    let cell = Cell::from(CellId::from_face(face));
                    let k = (face.as_u8() % 3) as usize;
                    let a = c[k];
                    let side = if face.as_u8() < 3 { a > 0.0 } else { a < 0.0 };
                    let other = (0..3).any(|j| j != k && !c[j].is_finite());
                    let expect = a.is_nan() || (side && other);
                    if panics(move || {
                        let _ = cell.contains_point(p);
                    }) != expect
                    {
                        cp_bad += 1;
                        println!("contains_point face {face}: ({x}, {y}, {z}) expected panic {expect}");
                    }
                }
                let infs = c.iter().filter(|t| t.is_infinite()).count();
                if infs >= 2 && !panics(move || {
                    let _ = CellId::from_point(&p);
                }) {
                    fp_bad += 1;
                    println!("from_point did not panic: ({x}, {y}, {z})");
                }
                let expect = x.is_nan() || y.is_nan();
                if panics(move || {
                    let _ = PointRegion::new(p).rect_bound();
                }) != expect
                {
                    rb_bad += 1;
                    println!("rect_bound: ({x}, {y}, {z}) expected panic {expect}");
                }
            }
        }
    }
    println!("{n} triples: contains_point mismatches {cp_bad}, from_point (two infinities) {fp_bad}, rect_bound {rb_bad}");
}
