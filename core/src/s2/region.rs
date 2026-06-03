// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Abstract interface for geometric regions on the unit sphere.
//!
//! Corresponds to C++ `S2Region`, Go `s2.Region`, Java `S2Region`.

use crate::s2::{Cap, Cell, CellId, Point, Rect};

/// A geometric region on the unit sphere.
///
/// All types implementing `Region` must provide bounding methods
/// (`cap_bound`, `rect_bound`, `cell_union_bound`) and containment /
/// intersection tests against cells and points.
///
/// # Examples
///
/// ```
/// use s2rst::s1::Angle;
/// use s2rst::s2::{Cap, LatLng, Point, Region};
///
/// // Cap implements Region.
/// let center = LatLng::from_degrees(48.8566, 2.3522).to_point();
/// let cap = Cap::from_center_angle(center, Angle::from_degrees(1.0));
///
/// // Bounding rectangle and bounding cap.
/// let rect = cap.rect_bound();
/// assert!(!rect.is_empty());
/// let bound = cap.cap_bound();
/// assert!(bound.contains_point(center));
///
/// // Cell union bound for spatial indexing.
/// let cells = cap.cell_union_bound();
/// assert!(!cells.is_empty());
/// ```
pub trait Region: std::fmt::Debug {
    /// Returns a bounding spherical cap for this region.
    fn cap_bound(&self) -> Cap;

    /// Returns a bounding latitude-longitude rectangle for this region.
    fn rect_bound(&self) -> Rect;

    /// Returns a small set of cells that cover this region. The output is
    /// not sorted.
    fn cell_union_bound(&self) -> Vec<CellId>;

    /// Reports whether this region completely contains the given cell.
    fn contains_cell(&self, cell: &Cell) -> bool;

    /// Reports whether this region intersects the given cell.
    fn intersects_cell(&self, cell: &Cell) -> bool;

    /// Reports whether this region contains the given point.
    fn contains_point(&self, p: &Point) -> bool;
}

// --- impl Region for Cap ---

impl Region for Cap {
    fn cap_bound(&self) -> Cap {
        Cap::cap_bound(*self)
    }

    fn rect_bound(&self) -> Rect {
        Cap::rect_bound(*self)
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        Cap::cell_union_bound(*self)
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        // A cap contains a cell if it contains the cell's bounding cap.
        // This is conservative: it may return false even if the cell is contained.
        self.contains(cell.cap_bound())
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        // A cap intersects a cell if it intersects the cell's bounding cap.
        // This is conservative for the negative case.
        self.intersects(cell.cap_bound())
    }

    fn contains_point(&self, p: &Point) -> bool {
        Cap::contains_point(*self, *p)
    }
}

// --- impl Region for Rect ---

impl Region for Rect {
    fn cap_bound(&self) -> Cap {
        Rect::cap_bound(*self)
    }

    fn rect_bound(&self) -> Rect {
        *self
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        Rect::cell_union_bound(*self)
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        self.contains(cell.rect_bound())
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        // Conservative: check if rect intersects cell's rect bound.
        self.intersects(cell.rect_bound())
    }

    fn contains_point(&self, p: &Point) -> bool {
        Rect::contains_point(*self, *p)
    }
}

// --- impl Region for Cell ---

impl Region for Cell {
    fn cap_bound(&self) -> Cap {
        Cell::cap_bound(*self)
    }

    fn rect_bound(&self) -> Rect {
        Cell::rect_bound(*self)
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        Cell::cell_union_bound(*self)
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        Cell::contains_cell(*self, *cell)
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        Cell::intersects_cell(*self, *cell)
    }

    fn contains_point(&self, p: &Point) -> bool {
        Cell::contains_point(*self, *p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cap_region() {
        let cap = Cap::from_point(Point::from_coords(1.0, 0.0, 0.0));
        let r: &dyn Region = &cap;
        assert!(!r.cap_bound().is_empty());
        assert!(!r.rect_bound().is_empty());
        assert!(r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_rect_region() {
        let rect = Rect::full();
        let r: &dyn Region = &rect;
        assert!(r.cap_bound().is_full());
        assert!(r.rect_bound().is_full());
        assert!(r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_cell_region() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let r: &dyn Region = &cell;
        assert!(!r.cap_bound().is_empty());
        assert!(!r.rect_bound().is_empty());
        assert!(r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_dyn_region() {
        // Ensure dyn Region is object-safe.
        let regions: Vec<Box<dyn Region>> = vec![
            Box::new(Cap::full()),
            Box::new(Rect::full()),
            Box::new(Cell::from_cell_id(CellId::from_face(0))),
        ];
        for region in &regions {
            assert!(!region.cap_bound().is_empty());
        }
    }
}
