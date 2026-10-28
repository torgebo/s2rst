//! Spec 011 T017: the failing quickcheck case (0, 0, 1, 1, 0) of the cell twin.
use s2rst::s2::{Cell, CellId, CellVertex, LatLng, Level, Region};
fn main() {
    let id = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(Level::new(1));
    let c = Cell::from_cell_id(id);
    let p = c.vertex(CellVertex::ALL[1]);
    let leaf = CellId::from_point(&p);
    println!("cell {} contains {} rect {} leaf-in-cell {} cover {:?}",
        id.to_token(), c.contains_point(p), c.rect_bound().contains_point(p), id.contains(leaf),
        c.cell_union_bound().iter().map(|i| i.to_token()).collect::<Vec<_>>());
}
