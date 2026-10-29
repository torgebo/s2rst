//! Spec 011 research R5: the panic of `CellId::from_point` for two infinite coordinates.
use s2rst::r3::Vector;
use s2rst::s2::{CellId, Point};
fn main() {
    let p = Point(Vector { x: 0.0, y: f64::INFINITY, z: f64::INFINITY });
    println!("{:016x}", CellId::from_point(&p).id());
}
