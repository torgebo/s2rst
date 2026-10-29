//! Spec 011 research R5: does `CellId::from_point` return a valid leaf for every triple? The
//! special values, then 10⁶ random triples over 600 decades (debug and release builds).

use s2rst::r3::Vector;
use s2rst::s2::{CellId, Level, Point};

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let v = [0.0, -0.0, 1.0, -1.0, 1e-320, -1e-320, 1e300, -1e300, f64::NAN, f64::INFINITY, -f64::INFINITY];
    let mut bad = 0;
    let mut n = 0;
    for &x in &v {
        for &y in &v {
            for &z in &v {
                let r = std::panic::catch_unwind(|| CellId::from_point(&Point(Vector { x, y, z })));
                n += 1;
                let Ok(id) = r else {
                    bad += 1;
                    println!("panics (debug): ({x}, {y}, {z})");
                    continue;
                };
                if !(id.is_valid() && id.level() == Level::MAX) {
                    bad += 1;
                    eprintln!("not a valid leaf: ({x}, {y}, {z}) -> {:016x}", id.id());
                }
            }
        }
    }
    let mut s: u64 = 0x0170_0017_0017_0017;
    let mut next = || {
        s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        s
    };
    for _ in 0..1_000_000 {
        let mut c = [0.0; 3];
        for k in &mut c {
            let m = (next() >> 11) as f64 / 9_007_199_254_740_992.0 * 2.0 - 1.0;
            *k = m * 10f64.powi((next() % 600) as i32 - 300);
        }
        let id = CellId::from_point(&Point(Vector { x: c[0], y: c[1], z: c[2] }));
        n += 1;
        if !(id.is_valid() && id.level() == Level::MAX) {
            bad += 1;
        }
    }
    println!("from_point: {bad} of {n} triples give no valid leaf");
}
