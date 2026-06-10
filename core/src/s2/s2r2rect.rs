// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! An axis-aligned rectangle in (s,t)-space on face 0, usable as a [`Region`].
//!
//! Corresponds to C++ `s2r2rect.h/.cc`.

use crate::r2;
use crate::s1::Angle;
use crate::s2::cap::Cap;
use crate::s2::cell::Cell;
use crate::s2::cell_id::CellId;
use crate::s2::coords::{self, Face};
use crate::s2::point::Point;
use crate::s2::rect::Rect;
use crate::s2::region::Region;

/// An [`r2::Rect`] interpreted as a region of (s,t)-space on face 0, allowing
/// the spherical geometry machinery (notably `RegionCoverer`) to be applied
/// to planar geometry.
///
/// The rectangle `[0,1]×[0,1]` corresponds to the `CellId` covering all of
/// face 0; only subsets of `[0,1]×[0,1]` can be approximated by coverings.
/// This type is also a convenient way to find the (s,t)-region covered by a
/// given cell (see [`S2R2Rect::from_cell`] / [`S2R2Rect::from_cell_id`]).
///
/// Port of C++ `S2R2Rect` (the `S2` prefix is kept to avoid confusion with
/// the plain planar [`r2::Rect`] it wraps and exposes via [`Self::rect`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct S2R2Rect(r2::Rect);

impl S2R2Rect {
    /// Constructs a rectangle from an [`r2::Rect`].
    pub fn new(rect: r2::Rect) -> Self {
        S2R2Rect(rect)
    }

    /// Constructs a rectangle from lower-left and upper-right corners.
    pub fn from_points(lo: r2::Point, hi: r2::Point) -> Self {
        S2R2Rect(r2::Rect::from_points(lo, hi))
    }

    /// The canonical empty rectangle. Use [`Self::is_empty`] to test for
    /// empty rectangles, since they have more than one representation.
    pub fn empty() -> Self {
        S2R2Rect(r2::Rect::empty())
    }

    /// The (s,t)-space boundary of the given cell, always within
    /// `[0,1]×[0,1]`.
    pub fn from_cell(cell: &Cell) -> Self {
        let size = cell.size_st();
        Self::from_center_size(cell.id().center_st(), r2::Point::new(size, size))
    }

    /// The (s,t)-space boundary of the given cell id.
    pub fn from_cell_id(id: CellId) -> Self {
        let size = CellId::size_st(id.level());
        Self::from_center_size(id.center_st(), r2::Point::new(size, size))
    }

    /// Constructs a rectangle from a center point and a (non-negative) size
    /// in each dimension.
    pub fn from_center_size(center: r2::Point, size: r2::Point) -> Self {
        S2R2Rect(r2::Rect::from_center_size(center, size))
    }

    /// A rectangle containing a single point.
    pub fn from_point(p: r2::Point) -> Self {
        S2R2Rect(r2::Rect::from_point(p))
    }

    /// The minimal bounding rectangle containing the two points (which need
    /// not be ordered).
    pub fn from_point_pair(p1: r2::Point, p2: r2::Point) -> Self {
        S2R2Rect(r2::Rect::from_point_pair(p1, p2))
    }

    /// The underlying planar rectangle.
    pub fn rect(&self) -> r2::Rect {
        self.0
    }

    /// The x-interval of the rectangle.
    pub fn x(&self) -> crate::r1::Interval {
        self.0.x
    }

    /// The y-interval of the rectangle.
    pub fn y(&self) -> crate::r1::Interval {
        self.0.y
    }

    /// The lower-left corner.
    pub fn lo(&self) -> r2::Point {
        self.0.lo()
    }

    /// The upper-right corner.
    pub fn hi(&self) -> r2::Point {
        self.0.hi()
    }

    /// True if the bound for each axis is empty or non-empty consistently.
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// True if the rectangle contains no points at all.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The k-th vertex (k = 0,1,2,3) in CCW order, starting at the
    /// lower-left corner. `k` is reduced modulo 4.
    pub fn vertex(&self, k: i32) -> r2::Point {
        self.0.vertex(k)
    }

    /// The center of the rectangle in (x,y)-space (in general this is not
    /// the center of the region on the sphere).
    pub fn center(&self) -> r2::Point {
        self.0.center()
    }

    /// The width and height in (x,y)-space; negative for empty rectangles.
    pub fn size(&self) -> r2::Point {
        self.0.size()
    }

    /// True if the (closed) rectangle contains the given planar point.
    pub fn contains_r2point(&self, p: r2::Point) -> bool {
        self.0.contains_point(p)
    }

    /// True if the interior of the rectangle contains the given point.
    pub fn interior_contains_r2point(&self, p: r2::Point) -> bool {
        self.0.interior_contains_point(p)
    }

    /// True if this rectangle contains the other rectangle.
    pub fn contains_rect(&self, other: &S2R2Rect) -> bool {
        self.0.contains(other.0)
    }

    /// True if the interior of this rectangle contains all points of the
    /// other rectangle (including its boundary).
    pub fn interior_contains_rect(&self, other: &S2R2Rect) -> bool {
        self.0.interior_contains(other.0)
    }

    /// True if this rectangle and the other have any points in common.
    pub fn intersects(&self, other: &S2R2Rect) -> bool {
        self.0.intersects(other.0)
    }

    /// True if the interior of this rectangle intersects any point of the
    /// other rectangle (including its boundary).
    pub fn interior_intersects(&self, other: &S2R2Rect) -> bool {
        self.0.interior_intersects(other.0)
    }

    /// Expands the rectangle to include the given point.
    pub fn add_point(&mut self, p: r2::Point) {
        self.0 = self.0.add_point(p);
    }

    /// The closest point in the rectangle to `p`. The rectangle must be
    /// non-empty.
    pub fn project(&self, p: r2::Point) -> r2::Point {
        self.0.project(p)
    }

    /// A rectangle expanded on each side by `margin.x()` / `margin.y()`
    /// (negative margins shrink; the result may be empty).
    pub fn expanded(&self, margin: r2::Point) -> S2R2Rect {
        S2R2Rect(self.0.expanded(margin))
    }

    /// A rectangle expanded by the same margin on all sides.
    pub fn expanded_by_margin(&self, margin: f64) -> S2R2Rect {
        S2R2Rect(self.0.expanded_by_margin(margin))
    }

    /// The smallest rectangle containing the union of the two rectangles.
    pub fn union(&self, other: &S2R2Rect) -> S2R2Rect {
        S2R2Rect(self.0.union(other.0))
    }

    /// The smallest rectangle containing the intersection of the two
    /// rectangles.
    pub fn intersection(&self, other: &S2R2Rect) -> S2R2Rect {
        S2R2Rect(self.0.intersection(other.0))
    }

    /// True if the x- and y-intervals of the two rectangles are the same up
    /// to the given tolerance.
    pub fn approx_eq_with(&self, other: &S2R2Rect, max_error: Angle) -> bool {
        self.0.approx_eq_with(other.0, max_error.radians())
    }

    /// [`Self::approx_eq_with`] with the default 1e-15 tolerance.
    pub fn approx_eq(&self, other: &S2R2Rect) -> bool {
        self.0.approx_eq(other.0)
    }

    /// The unit-length point on the sphere corresponding to the given point
    /// in the (s,t)-plane of face 0. `p` need not be inside `[0,1]×[0,1]`.
    pub fn to_point(p: r2::Point) -> Point {
        let face = Face::from_u8(0);
        Point(
            coords::face_uv_to_xyz(face, coords::st_to_uv(p.x), coords::st_to_uv(p.y)).normalize(),
        )
    }
}

impl Region for S2R2Rect {
    fn cap_bound(&self) -> Cap {
        if self.is_empty() {
            return Cap::empty();
        }
        // The rectangle is a convex polygon on the sphere, since it is a
        // subset of one cube face. Bounding its center and vertices is not
        // minimal but is close.
        let mut cap = Cap::from_point(Self::to_point(self.center()));
        for k in 0..4 {
            cap = cap.add_point(Self::to_point(self.vertex(k)));
        }
        cap
    }

    fn rect_bound(&self) -> Rect {
        // Not very tight, matching C++.
        self.cap_bound().rect_bound()
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        if cell.face() != Face::from_u8(0) {
            return false;
        }
        self.contains_rect(&Self::from_cell(cell))
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        if cell.face() != Face::from_u8(0) {
            return false;
        }
        self.intersects(&Self::from_cell(cell))
    }

    fn contains_point(&self, p: &Point) -> bool {
        let face = Face::from_u8(0);
        if coords::get_face(&p.0) != face {
            return false;
        }
        let (u, v) = coords::valid_face_xyz_to_uv(face, &p.0);
        self.contains_r2point(r2::Point::new(coords::uv_to_st(u), coords::uv_to_st(v)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r3::Vector;

    fn p(x: f64, y: f64) -> r2::Point {
        r2::Point::new(x, y)
    }

    /// C++ `TestCellOps`: 0 = no intersection, 2 = intersects, 3 = intersects
    /// and one region contains a vertex of the other, 4 = contains.
    fn test_cell_ops(r: &S2R2Rect, cell: &Cell, level: i32) {
        let mut vertex_contained = false;
        for i in 0..4 {
            if let Some((u, v)) = coords::face_xyz_to_uv(Face::from_u8(0), &cell.vertex_raw(i).0)
                && r.contains_r2point(p(coords::uv_to_st(u), coords::uv_to_st(v)))
            {
                vertex_contained = true;
            }
            if !r.is_empty()
                && cell.contains_point(&S2R2Rect::to_point(r.vertex(i32::try_from(i).unwrap())))
            {
                vertex_contained = true;
            }
        }
        assert_eq!(level >= 2, r.intersects_cell(cell), "may_intersect");
        assert_eq!(level >= 3, vertex_contained, "vertex_contained");
        assert_eq!(level >= 4, r.contains_cell(cell), "contains");
    }

    #[test]
    fn test_empty_rectangles() {
        let empty = S2R2Rect::empty();
        assert!(empty.is_valid());
        assert!(empty.is_empty());
    }

    #[test]
    fn test_constructors_and_accessors() {
        let r = S2R2Rect::from_points(p(0.1, 0.0), p(0.25, 1.0));
        assert_eq!(r.x().lo, 0.1);
        assert_eq!(r.x().hi, 0.25);
        assert_eq!(r.y().lo, 0.0);
        assert_eq!(r.y().hi, 1.0);
        assert_eq!(r.lo(), p(0.1, 0.0));
        assert_eq!(r.hi(), p(0.25, 1.0));
        assert!(r.is_valid());
        assert!(!r.is_empty());
    }

    #[test]
    fn test_from_cell() {
        // FromCell, FromCellId.
        assert!(
            S2R2Rect::from_cell(&Cell::from_face_pos_level(0, 0, 1))
                .approx_eq(&S2R2Rect::from_points(p(0.0, 0.0), p(0.5, 0.5)))
        );
        assert!(
            S2R2Rect::from_cell_id(CellId::from_face(0))
                .approx_eq(&S2R2Rect::from_points(p(0.0, 0.0), p(1.0, 1.0)))
        );
    }

    #[test]
    fn test_to_point_center_of_face0() {
        // The center of face 0 in (s,t) is the x-axis.
        let pt = S2R2Rect::to_point(p(0.5, 0.5));
        assert_eq!(pt, Point(Vector::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_bounds_degenerate_rect() {
        let empty = S2R2Rect::empty();
        assert!(empty.cap_bound().is_empty());
        assert!(empty.rect_bound().is_empty());

        let degenerate = S2R2Rect::from_points(p(0.5, 0.5), p(0.5, 0.5));
        let cap = degenerate.cap_bound();
        assert!(cap.contains_point(Point(Vector::new(1.0, 0.0, 0.0))));
        let rect = degenerate.rect_bound();
        assert!(rect.contains_point(Point(Vector::new(1.0, 0.0, 0.0))));
    }

    #[test]
    fn test_bounds_contain_vertices() {
        // Deterministic stand-in for the C++ random-cell Bounds test.
        for id in [
            CellId::from_face(0),
            CellId::from_face(0).children()[2],
            CellId::from_face(0).children()[1].children()[3],
            CellId::from_face(0).children()[0].children()[0].children()[0],
        ] {
            let rect = S2R2Rect::from_cell_id(id);
            let cap = rect.cap_bound();
            let llrect = rect.rect_bound();
            for k in 0..4 {
                let v = S2R2Rect::to_point(rect.vertex(k));
                let center = cap.center();
                let v2 = Point((center.0 + (v.0 - center.0) * 3.0).normalize());
                assert!(
                    cap.contains_point(v),
                    "cap must contain vertex {k} of {id:?}"
                );
                assert!(!cap.contains_point(v2), "cap must not contain far point");
                assert!(llrect.contains_point(v));
                assert!(!llrect.contains_point(v2));
            }
        }
    }

    #[test]
    fn test_cell_operations() {
        let empty = S2R2Rect::empty();
        test_cell_ops(&empty, &Cell::from_face(3), 0);

        // This rectangle includes the first quadrant of face 0.
        let r4 = S2R2Rect::from_points(p(0.0, 0.0), p(0.5, 0.5));
        test_cell_ops(&r4, &Cell::from_face_pos_level(0, 0, 0), 3);
        test_cell_ops(&r4, &Cell::from_face_pos_level(0, 0, 1), 4);
        test_cell_ops(&r4, &Cell::from_face_pos_level(1, 0, 1), 0);

        // This rectangle intersects the first quadrant of face 0.
        let r5 = S2R2Rect::from_points(p(0.0, 0.45), p(0.5, 0.55));
        test_cell_ops(&r5, &Cell::from_face_pos_level(0, 0, 0), 3);
        test_cell_ops(&r5, &Cell::from_face_pos_level(0, 0, 1), 3);
        test_cell_ops(&r5, &Cell::from_face_pos_level(1, 0, 1), 0);

        // Rectangle consisting of a single point.
        test_cell_ops(
            &S2R2Rect::from_points(p(0.51, 0.51), p(0.51, 0.51)),
            &Cell::from_face(0),
            3,
        );

        // Rectangle that intersects the bounding rectangle of face 0 but not
        // the face itself.
        test_cell_ops(
            &S2R2Rect::from_points(p(0.01, 1.001), p(0.02, 1.002)),
            &Cell::from_face(0),
            0,
        );

        // Rectangle that intersects one corner of face 0.
        test_cell_ops(
            &S2R2Rect::from_points(p(0.99, -0.01), p(1.01, 0.01)),
            &Cell::from_face_pos_level(0, !0u64 >> 3, 5),
            3,
        );
    }

    #[test]
    fn test_region_coverer_integration() {
        // The headline use case: covering a planar (s,t) rectangle with cells.
        use crate::s2::region_coverer::RegionCoverer;
        let r = S2R2Rect::from_points(p(0.1, 0.1), p(0.3, 0.2));
        let coverer = RegionCoverer::new().max_cells(8);
        let covering = coverer.covering(&r);
        assert!(!covering.cell_ids().is_empty());
        for &id in covering.cell_ids() {
            assert_eq!(id.face(), Face::from_u8(0), "covering must stay on face 0");
        }
        // Every covering cell must intersect the rectangle.
        for &id in covering.cell_ids() {
            assert!(r.intersects_cell(&Cell::from_cell_id(id)));
        }
    }
}
