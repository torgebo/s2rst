//! Spec 012 U-017: a small counterexample: a cell, and a point of its u-v margin that the cell
//! contains and its cap (and, separately, its rectangle) bound does not.
use s2rst::s2::coords::face_uv_to_xyz;
use s2rst::s2::{Cell, CellId, Face, Level, Point};
fn main() {
    let (mut cap_done, mut rect_done) = (false, false);
    for level in [1u8, 3, 5, 10, 20, 30] {
        let mut id = CellId::from_face(Face::F0).child_begin_at_level(Level::new(level));
        for _ in 0..2000 {
            if id == CellId::end(Level::new(level)) {
                break;
            }
            let c = Cell::from(id);
            let uv = c.bound_uv();
            for k in 0..8 {
                let d = f64::EPSILON;
                let (um, vm) = (uv.x.center(), uv.y.center());
                let (u, v) = match k {
                    4 => (uv.x.lo - d, vm),
                    5 => (uv.x.hi + d, vm),
                    6 => (um, uv.y.lo - d),
                    7 => (um, uv.y.hi + d),
                    _ => (if k == 0 || k == 3 { uv.x.lo - d } else { uv.x.hi + d }, if k < 2 { uv.y.lo - d } else { uv.y.hi + d }),
                };
                let p = Point(face_uv_to_xyz(c.face(), u, v).normalize());
                if c.contains_point(p) {
                    if !cap_done && !c.cap_bound().contains_point(p) {
                        println!("cap: cell {:016x} p ({:e}, {:e}, {:e}) bits {:016x} {:016x} {:016x}", id.id(), p.x(), p.y(), p.z(), p.x().to_bits(), p.y().to_bits(), p.z().to_bits());
                        cap_done = true;
                    }
                    if !rect_done && !c.rect_bound().contains_point(p) {
                        println!("rect: cell {:016x} p bits {:016x} {:016x} {:016x}", id.id(), p.x().to_bits(), p.y().to_bits(), p.z().to_bits());
                        rect_done = true;
                    }
                }
            }
            if cap_done && rect_done {
                return;
            }
            id = id.next();
        }
    }
}
