//! Spec 011 P-UREG-03: a small counterexample to the vertex law for `CellUnion`.
use s2rst::s2::{Cell, CellId, CellUnion, CellVertex, Face, Level, Region};
fn main() {
    for face in Face::ALL {
        let mut id = CellId::from_face(face).child_begin_at_level(Level::new(1));
        for _ in 0..4 {
            let u = CellUnion::from_cell_ids(vec![id]);
            for n in id.edge_neighbors() {
                let c = Cell::from(n);
                if u.intersects_cell(&c) {
                    continue;
                }
                for k in CellVertex::ALL {
                    if u.contains_point(c.vertex(k)) {
                        println!("union {} neighbour {} vertex {k:?}", id.to_token(), n.to_token());
                    }
                }
            }
            id = id.next();
        }
    }
}
