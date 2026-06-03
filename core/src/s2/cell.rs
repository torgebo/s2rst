// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A concrete cell from a [`CellId`], with precomputed vertices and edges.
//!
//! Corresponds to C++ `S2Cell`, Go `s2.Cell`, Java `S2Cell`.
//!
//! Unlike [`CellId`], `Cell` supports efficient containment and intersection
//! tests by precomputing the cell's bounding rectangle in (u,v) space.

#![expect(
    clippy::cast_possible_truncation,
    reason = "cell level arithmetic — bounded by MAX_CELL_LEVEL"
)]
use crate::r2;
use crate::r3::Vector;
use crate::s1::ChordAngle;
use crate::s2::coords::{
    Face, Level, POS_TO_IJ, POS_TO_ORIENTATION, face_uv_to_xyz, face_xyz_to_uv, face_xyz_to_uvw,
    get_u_axis, get_u_norm, get_v_axis, get_v_norm, uv_to_st,
};
use crate::s2::{Cap, CellId, LatLng, Point, Rect, ij_level_to_bound_uv, size_ij};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// Latitude at pole (arcsin(sqrt(1/3)) - 0.5*epsilon).
const POLE_MIN_LAT: f64 = 0.615479708670387; // math.Asin(math.Sqrt(1.0/3))

/// Identifies one of the four edges of a [`Cell`].
///
/// Edges are numbered in CCW order: bottom (0), right (1), top (2), left (3).
/// Edge k runs from vertex k to vertex k+1 (mod 4).
#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CellEdge {
    /// Edge 0: bottom (v = `v_lo`), from vertex 0 to vertex 1.
    #[default]
    Bottom = 0,
    /// Edge 1: right  (u = `u_hi`), from vertex 1 to vertex 2.
    Right = 1,
    /// Edge 2: top    (v = `v_hi`), from vertex 2 to vertex 3.
    Top = 2,
    /// Edge 3: left   (u = `u_lo`), from vertex 3 to vertex 0.
    Left = 3,
}

impl CellEdge {
    /// All four edges in order.
    pub const ALL: [CellEdge; 4] = [
        CellEdge::Bottom,
        CellEdge::Right,
        CellEdge::Top,
        CellEdge::Left,
    ];
}

/// A concrete S2 cell with precomputed bounds.
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::s2::{Cell, CellId, LatLng, Point};
///
/// // Create a cell from a lat/lng (Paris).
/// let ll = LatLng::from_degrees(48.8566, 2.3522);
/// let cell = Cell::from_lat_lng(ll);
/// assert_eq!(cell.level(), 30); // leaf cell
///
/// // Get the four vertices.
/// let v0 = cell.vertex(0);
/// let v1 = cell.vertex(1);
/// assert!(v0 != v1);
///
/// // Navigate to a parent cell and check area.
/// let parent_id = cell.id().parent_at_level(10);
/// let parent = Cell::from_cell_id(parent_id);
/// assert!(parent.approx_area() > cell.approx_area());
///
/// // Bounding rectangles.
/// let rect = parent.rect_bound();
/// assert!(!rect.is_empty());
/// assert!(rect.contains_point(ll.to_point()));
/// ```
#[must_use]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cell {
    face: Face,
    level: Level,
    orientation: u8,
    id: CellId,
    uv: r2::Rect,
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::from_cell_id(CellId::from_face(0))
    }
}

impl Cell {
    // --- Constructors ---

    /// Constructs a `Cell` corresponding to the given [`CellId`].
    #[inline]
    pub fn from_cell_id(id: CellId) -> Self {
        let (f, i, j, o) = id.to_face_ij_orientation();
        Cell {
            face: f,
            level: id.level(),
            orientation: o,
            id,
            uv: ij_level_to_bound_uv(i, j, id.level()),
        }
    }

    /// Constructs a leaf cell containing the given point.
    #[inline]
    pub fn from_point(p: Point) -> Self {
        Self::from_cell_id(CellId::from_point(&p))
    }

    /// Constructs a leaf cell containing the given lat/lng.
    #[inline]
    pub fn from_lat_lng(ll: LatLng) -> Self {
        Self::from_cell_id(CellId::from_lat_lng(&ll))
    }

    /// Constructs a cell for the given face (level-0 cell).
    #[inline]
    pub fn from_face(face: impl Into<u8>) -> Self {
        Self::from_cell_id(CellId::from_face(face))
    }

    /// Constructs a cell from face, Hilbert curve position, and level.
    #[inline]
    pub fn from_face_pos_level(face: impl Into<u8>, pos: u64, level: impl Into<Level>) -> Self {
        Self::from_cell_id(CellId::from_face_pos_level(face, pos, level))
    }

    // --- Accessors ---

    /// Returns the face number (0–5).
    #[inline]
    pub fn face(self) -> Face {
        self.face
    }

    /// Returns the subdivision level (0–30).
    #[inline]
    pub fn level(self) -> Level {
        self.level
    }

    /// Returns the [`CellId`] of this cell.
    #[inline]
    pub fn id(self) -> CellId {
        self.id
    }

    /// Returns the cell's orientation (0–3).
    #[inline]
    pub fn orientation(self) -> u8 {
        self.orientation
    }

    /// Returns whether this is a leaf cell (level 30).
    #[inline]
    pub fn is_leaf(self) -> bool {
        self.level == Level::MAX
    }

    /// Returns the edge length of this cell in (i,j)-space.
    #[inline]
    pub fn size_ij(self) -> i32 {
        size_ij(self.level)
    }

    /// Returns the edge length of this cell in (s,t)-space.
    #[inline]
    pub fn size_st(self) -> f64 {
        CellId::size_st(self.level)
    }

    /// Returns the bounding rectangle in (u,v) space.
    #[inline]
    pub fn bound_uv(self) -> r2::Rect {
        self.uv
    }

    // --- Vertices and edges ---

    /// Returns the normalized k-th vertex (k = 0,1,2,3) in CCW order.
    #[inline]
    pub fn vertex(self, k: usize) -> Point {
        Point(self.vertex_raw(k).vector().normalize())
    }

    /// Returns the unnormalized k-th vertex.
    #[inline]
    pub fn vertex_raw(self, k: usize) -> Point {
        let verts = self.uv.vertices();
        Point(face_uv_to_xyz(self.face, verts[k].x, verts[k].y))
    }

    /// Returns the normalized inward-facing normal of the given edge.
    #[inline]
    pub fn edge(self, k: CellEdge) -> Point {
        Point(self.edge_raw(k).vector().normalize())
    }

    /// Returns the unnormalized inward-facing normal of the given edge.
    ///
    /// Edge k runs from vertex k to vertex k+1 (mod 4).
    #[inline]
    pub fn edge_raw(self, k: CellEdge) -> Point {
        match k {
            CellEdge::Bottom => Point(get_v_norm(self.face, self.uv.y.lo)),
            CellEdge::Right => Point(get_u_norm(self.face, self.uv.x.hi)),
            CellEdge::Top => Point(get_v_norm(self.face, self.uv.y.hi) * -1.0),
            CellEdge::Left => Point(get_u_norm(self.face, self.uv.x.lo) * -1.0),
        }
    }

    /// Returns the center of the cell as a unit-length point.
    #[inline]
    pub fn center(self) -> Point {
        self.id.to_point()
    }

    /// Returns the unnormalized center of the cell.
    #[inline]
    pub fn center_raw(self) -> Point {
        self.id.to_point_raw()
    }

    /// Returns the U or V coordinate that is constant along the given edge.
    #[inline]
    pub fn uv_coord_of_edge(self, k: CellEdge) -> f64 {
        match k {
            CellEdge::Bottom => self.uv.y.lo,
            CellEdge::Right => self.uv.x.hi,
            CellEdge::Top => self.uv.y.hi,
            CellEdge::Left => self.uv.x.lo,
        }
    }

    /// Returns the I or J coordinate that is constant along the given edge
    /// in (i,j)-space.
    #[inline]
    pub fn ij_coord_of_edge(self, k: CellEdge) -> i32 {
        // MAX_SIZE = 2^30 = kLimitIJ in C++
        let limit_ij = f64::from(size_ij(0));
        (limit_ij * uv_to_st(self.uv_coord_of_edge(k))).round() as i32
    }

    // --- Children ---

    /// Returns the four direct children in traversal order, or `None`
    /// if this is a leaf cell.
    pub fn children(self) -> Option<[Cell; 4]> {
        if self.id.is_leaf() {
            return None;
        }

        let uv_mid = self.id.center_uv();
        let mut cid = self.id.child_begin();
        let mut children = [Cell::default(); 4];

        for pos in 0..4 {
            children[pos] = Cell {
                face: self.face,
                level: self.level + 1,
                orientation: self.orientation ^ POS_TO_ORIENTATION[pos],
                id: cid,
                uv: r2::Rect::default(),
            };

            let ij = POS_TO_IJ[self.orientation as usize][pos];
            let i = ij >> 1;
            let j = ij & 1;

            children[pos].uv.x = if i == 1 {
                crate::r1::Interval::new(uv_mid.x, self.uv.x.hi)
            } else {
                crate::r1::Interval::new(self.uv.x.lo, uv_mid.x)
            };
            children[pos].uv.y = if j == 1 {
                crate::r1::Interval::new(uv_mid.y, self.uv.y.hi)
            } else {
                crate::r1::Interval::new(self.uv.y.lo, uv_mid.y)
            };

            cid = cid.next();
        }

        Some(children)
    }

    // --- Area ---

    /// Returns the average area of cells at the given level.
    pub fn average_area_for_level(level: impl Into<Level>) -> f64 {
        4.0 * PI / (6.0 * (4.0_f64).powi(level.into().as_i32()))
    }

    /// Returns the average area of cells at the level of this cell.
    pub fn average_area(self) -> f64 {
        Cell::average_area_for_level(self.level)
    }

    /// Returns the approximate area of this cell.
    pub fn approx_area(self) -> f64 {
        if self.level < Level::new(2) {
            return self.average_area();
        }

        let flat_area = 0.5
            * (self.vertex(2).vector() - self.vertex(0).vector())
                .cross(self.vertex(3).vector() - self.vertex(1).vector())
                .norm();

        flat_area * 2.0 / (1.0 + (1.0 - (flat_area / PI).min(1.0)).sqrt())
    }

    /// Returns the exact area of this cell.
    ///
    /// Computed as the sum of two spherical triangles, which is accurate
    /// at all cell levels.
    pub fn exact_area(self) -> f64 {
        let v0 = self.vertex(0);
        let v1 = self.vertex(1);
        let v2 = self.vertex(2);
        let v3 = self.vertex(3);
        crate::s2::point_measures::point_area(v0, v1, v2)
            + crate::s2::point_measures::point_area(v0, v2, v3)
    }

    // --- Containment ---

    /// Reports whether this cell contains the given point.
    ///
    /// The cell is treated as a closed set (boundary points belong to
    /// both adjacent cells).
    #[inline]
    pub fn contains_point(self, p: Point) -> bool {
        let uv = face_xyz_to_uv(self.face, &p.vector());
        let Some((u, v)) = uv else {
            return false;
        };
        let margin = (5.0 / 3.0) * f64::EPSILON;
        self.uv
            .expanded_by_margin(margin)
            .contains_point(r2::Point::new(u, v))
    }

    /// Reports whether this cell contains the other cell.
    #[inline]
    pub fn contains_cell(self, other: Cell) -> bool {
        self.id.contains(other.id)
    }

    /// Reports whether this cell intersects the other cell.
    #[inline]
    pub fn intersects_cell(self, other: Cell) -> bool {
        self.id.intersects(other.id)
    }

    // --- Bounding ---

    /// Returns the bounding cap of this cell.
    pub fn cap_bound(self) -> Cap {
        let uv_center = self.uv.center();
        let cap_center = Point(face_uv_to_xyz(self.face, uv_center.x, uv_center.y).normalize());
        let mut cap = Cap::from_point(cap_center);
        for k in 0..4 {
            cap = cap.add_point(self.vertex(k));
        }
        cap
    }

    /// Returns the bounding lat-lng rectangle of this cell.
    pub fn rect_bound(self) -> Rect {
        if self.level > Level::MIN {
            let u = self.uv.x.lo + self.uv.x.hi;
            let v = self.uv.y.lo + self.uv.y.hi;

            let i = if get_u_axis(self.face).z == 0.0 {
                if u < 0.0 { 1 } else { 0 }
            } else if u > 0.0 {
                1
            } else {
                0
            };
            let j = if get_v_axis(self.face).z == 0.0 {
                if v < 0.0 { 1 } else { 0 }
            } else if v > 0.0 {
                1
            } else {
                0
            };

            let lat = crate::r1::Interval::from_point(self.latitude(i, j))
                .add_point(self.latitude(1 - i, 1 - j));
            let lng = crate::s1::Interval::empty()
                .add_point(self.longitude(i, 1 - j))
                .add_point(self.longitude(1 - i, j));

            let eps2 = 2.0 * f64::EPSILON;
            return Rect::new(lat, lng)
                .expanded(LatLng::from_radians(eps2, eps2))
                .polar_closure();
        }

        // Level 0 face cells
        let bound = match self.face {
            Face::F0 => Rect::new(
                crate::r1::Interval::new(-FRAC_PI_4, FRAC_PI_4),
                crate::s1::Interval::new(-FRAC_PI_4, FRAC_PI_4),
            ),
            Face::F1 => Rect::new(
                crate::r1::Interval::new(-FRAC_PI_4, FRAC_PI_4),
                crate::s1::Interval::new(FRAC_PI_4, 3.0 * FRAC_PI_4),
            ),
            Face::F2 => Rect::new(
                crate::r1::Interval::new(POLE_MIN_LAT - 0.5 * f64::EPSILON, FRAC_PI_2),
                crate::s1::Interval::full(),
            ),
            Face::F3 => Rect::new(
                crate::r1::Interval::new(-FRAC_PI_4, FRAC_PI_4),
                crate::s1::Interval::new(3.0 * FRAC_PI_4, -3.0 * FRAC_PI_4),
            ),
            Face::F4 => Rect::new(
                crate::r1::Interval::new(-FRAC_PI_4, FRAC_PI_4),
                crate::s1::Interval::new(-3.0 * FRAC_PI_4, -FRAC_PI_4),
            ),
            Face::F5 => Rect::new(
                crate::r1::Interval::new(-FRAC_PI_2, -(POLE_MIN_LAT - 0.5 * f64::EPSILON)),
                crate::s1::Interval::full(),
            ),
        };
        bound.expanded(LatLng::from_radians(f64::EPSILON, 0.0))
    }

    /// Returns the `CellId` of this cell as a singleton covering.
    pub fn cell_union_bound(self) -> Vec<CellId> {
        vec![self.id]
    }

    /// Returns the minimum distance from the cell to the given point.
    /// Returns zero if the cell contains the point.
    #[inline]
    pub fn distance_to_point(self, p: Point) -> ChordAngle {
        self.distance_internal(p, true)
    }

    /// Returns the minimum distance from the cell to the given edge.
    /// Returns zero if the edge intersects the cell.
    pub fn distance_to_edge(self, a: Point, b: Point) -> ChordAngle {
        use crate::s1::ChordAngle;
        use crate::s2::edge_distances;
        if self.contains_point(a) || self.contains_point(b) {
            return ChordAngle::ZERO;
        }
        let mut min_dist = ChordAngle::INFINITY;
        for k in 0..4 {
            let v0 = self.vertex(k);
            let v1 = self.vertex((k + 1) % 4);
            let (cp_a, cp_b) = edge_distances::edge_pair_closest_points(a, b, v0, v1);
            let d = cp_a.chord_angle(cp_b);
            if d < min_dist {
                min_dist = d;
            }
        }
        min_dist
    }

    /// Returns the minimum distance from this cell to the other cell.
    /// Returns zero if the cells intersect.
    pub fn distance_to_cell(self, other: Cell) -> ChordAngle {
        use crate::s1::ChordAngle;
        use crate::s2::edge_distances;
        // Check if any vertex of either cell is contained in the other.
        for k in 0..4 {
            if self.contains_point(other.vertex(k)) || other.contains_point(self.vertex(k)) {
                return ChordAngle::ZERO;
            }
        }
        let mut min_dist = ChordAngle::INFINITY;
        for i in 0..4 {
            let a0 = self.vertex(i);
            let a1 = self.vertex((i + 1) % 4);
            for j in 0..4 {
                let b0 = other.vertex(j);
                let b1 = other.vertex((j + 1) % 4);
                let (cp_a, cp_b) = edge_distances::edge_pair_closest_points(a0, a1, b0, b1);
                let d = cp_a.chord_angle(cp_b);
                if d < min_dist {
                    min_dist = d;
                }
            }
        }
        min_dist
    }

    /// Returns the maximum distance from the cell to the given point.
    pub fn max_distance_to_point(self, p: Point) -> ChordAngle {
        use crate::s1::ChordAngle;
        // First check the 4 cell vertices. If all are within the hemisphere
        // centered around the target, the max distance is to one of these vertices.
        let target_uvw = face_xyz_to_uvw(self.face, &p.vector());
        let max_dist = chord_max(
            chord_max(
                self.vertex_chord_dist(&target_uvw, 0, 0),
                self.vertex_chord_dist(&target_uvw, 1, 0),
            ),
            chord_max(
                self.vertex_chord_dist(&target_uvw, 0, 1),
                self.vertex_chord_dist(&target_uvw, 1, 1),
            ),
        );
        if max_dist <= ChordAngle::RIGHT {
            return max_dist;
        }
        // Otherwise, find the minimum distance to the antipodal point and the
        // maximum distance will be Pi - d_min.
        ChordAngle::STRAIGHT - self.distance_to_point(-p)
    }

    /// Returns the maximum distance from the cell to the given edge.
    pub fn max_distance_to_edge(self, a: Point, b: Point) -> ChordAngle {
        use crate::s1::ChordAngle;
        // If the maximum distance from both endpoints to the cell is less than
        // Pi/2 then the maximum distance from the edge to the cell is the
        // maximum of the two endpoint distances.
        let max_dist = chord_max(self.max_distance_to_point(a), self.max_distance_to_point(b));
        if max_dist <= ChordAngle::RIGHT {
            return max_dist;
        }
        ChordAngle::STRAIGHT - self.distance_to_edge(-a, -b)
    }

    /// Returns the maximum distance from this cell to the other cell.
    pub fn max_distance_to_cell(self, other: Cell) -> ChordAngle {
        use crate::s1::ChordAngle;
        use crate::s2::edge_distances;
        // Check if the antipodal cell intersects this cell.
        let opposite_face = other.face.opposite();
        if self.face == opposite_face {
            // Antipodal UV is the transpose of the original UV.
            let antipodal_uv = r2::Rect {
                x: other.uv.y,
                y: other.uv.x,
            };
            if self.uv.intersects(antipodal_uv) {
                return ChordAngle::STRAIGHT;
            }
        }
        // Otherwise, the maximum distance always occurs between a vertex of one
        // cell and an edge of the other cell (including the edge endpoints).
        // This represents a total of 32 possible (vertex, edge) pairs.
        let va: [Point; 4] = std::array::from_fn(|i| self.vertex(i));
        let vb: [Point; 4] = std::array::from_fn(|i| other.vertex(i));
        let mut max_dist = ChordAngle::NEGATIVE;
        for i in 0..4 {
            for j in 0..4 {
                let (d, updated) =
                    edge_distances::update_max_distance(va[i], vb[j], vb[(j + 1) & 3], max_dist);
                if updated {
                    max_dist = d;
                }
                let (d, updated) =
                    edge_distances::update_max_distance(vb[i], va[j], va[(j + 1) & 3], max_dist);
                if updated {
                    max_dist = d;
                }
            }
        }
        max_dist
    }

    /// Returns the distance from the cell boundary to the given point.
    /// Unlike `distance_to_point`, this returns a positive distance even
    /// when the point is inside the cell.
    #[inline]
    pub fn boundary_distance(self, target: Point) -> ChordAngle {
        self.distance_internal(target, false)
    }

    /// Returns true if the minimum distance from this cell to `target` is
    /// less than `limit`.
    #[inline]
    pub fn is_distance_less(self, target: Cell, limit: ChordAngle) -> bool {
        self.distance_to_cell(target) < limit
    }

    /// Returns true if the minimum distance from this cell to `target` is
    /// at most `limit`.
    #[inline]
    pub fn is_distance_less_or_equal(self, target: Cell, limit: ChordAngle) -> bool {
        self.distance_to_cell(target) <= limit
    }

    /// Returns true if the maximum distance from this cell to `target` is
    /// less than `limit`.
    #[inline]
    pub fn is_max_distance_less(self, target: Cell, limit: ChordAngle) -> bool {
        self.max_distance_to_cell(target) < limit
    }

    /// Returns true if the maximum distance from this cell to `target` is
    /// at most `limit`.
    #[inline]
    pub fn is_max_distance_less_or_equal(self, target: Cell, limit: ChordAngle) -> bool {
        self.max_distance_to_cell(target) <= limit
    }

    // --- Private helpers ---

    /// Internal distance computation. When `to_interior` is true, returns zero
    /// for points inside the cell (same as `distance_to_point`). When false,
    /// returns the distance to the boundary even for interior points.
    fn distance_internal(self, target_xyz: Point, to_interior: bool) -> ChordAngle {
        // Work in the (u,v,w) coordinate frame of this cell's face.
        let target = face_xyz_to_uvw(self.face, &target_xyz.vector());

        // Compute dot products with all four edge normals.
        // dirIJ = dot product for axis I, endpoint J.
        let dir00 = target.x - target.z * self.uv.x.lo;
        let dir01 = target.x - target.z * self.uv.x.hi;
        let dir10 = target.y - target.z * self.uv.y.lo;
        let dir11 = target.y - target.z * self.uv.y.hi;

        let mut inside = true;
        if dir00 < 0.0 {
            inside = false;
            if self.v_edge_is_closest(&target, 0) {
                return Self::edge_distance(-dir00, self.uv.x.lo);
            }
        }
        if dir01 > 0.0 {
            inside = false;
            if self.v_edge_is_closest(&target, 1) {
                return Self::edge_distance(dir01, self.uv.x.hi);
            }
        }
        if dir10 < 0.0 {
            inside = false;
            if self.u_edge_is_closest(&target, 0) {
                return Self::edge_distance(-dir10, self.uv.y.lo);
            }
        }
        if dir11 > 0.0 {
            inside = false;
            if self.u_edge_is_closest(&target, 1) {
                return Self::edge_distance(dir11, self.uv.y.hi);
            }
        }
        if inside {
            if to_interior {
                return ChordAngle::ZERO;
            }
            // Point is inside: return minimum distance to any edge.
            let d0 = Self::edge_distance(-dir00, self.uv.x.lo);
            let d1 = Self::edge_distance(dir01, self.uv.x.hi);
            let d2 = Self::edge_distance(-dir10, self.uv.y.lo);
            let d3 = Self::edge_distance(dir11, self.uv.y.hi);
            return chord_min(chord_min(d0, d1), chord_min(d2, d3));
        }
        // Outside: closest point is one of the four cell vertices.
        let d0 = self.vertex_chord_dist(&target, 0, 0);
        let d1 = self.vertex_chord_dist(&target, 1, 0);
        let d2 = self.vertex_chord_dist(&target, 0, 1);
        let d3 = self.vertex_chord_dist(&target, 1, 1);
        chord_min(chord_min(d0, d1), chord_min(d2, d3))
    }

    /// Returns the chord angle distance from point p (in UVW coords) to the
    /// vertex at (uv[0][i], uv[1][j]).
    fn vertex_chord_dist(self, p: &Vector, i: usize, j: usize) -> ChordAngle {
        let u = if i == 0 { self.uv.x.lo } else { self.uv.x.hi };
        let v = if j == 0 { self.uv.y.lo } else { self.uv.y.hi };
        let vertex = Vector::new(u, v, 1.0).normalize();
        Point(vertex).chord_angle(Point(*p))
    }

    /// Returns true if the closest point to p on the horizontal edge through
    /// uv[1][v_end] lies in the interior of the edge.
    fn u_edge_is_closest(self, p: &Vector, v_end: usize) -> bool {
        let u0 = self.uv.x.lo;
        let u1 = self.uv.x.hi;
        let v = if v_end == 0 {
            self.uv.y.lo
        } else {
            self.uv.y.hi
        };
        // Normals to the planes perpendicular to the edge at each endpoint.
        let dir0 = Vector::new(v * v + 1.0, -u0 * v, -u0);
        let dir1 = Vector::new(v * v + 1.0, -u1 * v, -u1);
        p.dot(dir0) > 0.0 && p.dot(dir1) < 0.0
    }

    /// Returns true if the closest point to p on the vertical edge through
    /// uv[0][u_end] lies in the interior of the edge.
    fn v_edge_is_closest(self, p: &Vector, u_end: usize) -> bool {
        let v0 = self.uv.y.lo;
        let v1 = self.uv.y.hi;
        let u = if u_end == 0 {
            self.uv.x.lo
        } else {
            self.uv.x.hi
        };
        let dir0 = Vector::new(-u * v0, u * u + 1.0, -v0);
        let dir1 = Vector::new(-u * v1, u * u + 1.0, -v1);
        p.dot(dir0) > 0.0 && p.dot(dir1) < 0.0
    }

    /// Computes the chord angle distance from a point to a cell edge, given
    /// the signed dot product `dir_ij` with the edge normal and the UV
    /// coordinate of the edge.
    fn edge_distance(dir_ij: f64, uv: f64) -> ChordAngle {
        let pq2 = (dir_ij * dir_ij) / (1.0 + uv * uv);
        let qr = 1.0 - (1.0 - pq2).sqrt();
        ChordAngle::from_length2(pq2 + qr * qr)
    }

    /// Returns the latitude of the cell corner (i,j) in radians.
    fn latitude(self, i: usize, j: usize) -> f64 {
        let u = if i == 0 { self.uv.x.lo } else { self.uv.x.hi };
        let v = if j == 0 { self.uv.y.lo } else { self.uv.y.hi };
        LatLng::latitude(Point(face_uv_to_xyz(self.face, u, v))).radians()
    }

    /// Returns the longitude of the cell corner (i,j) in radians.
    fn longitude(self, i: usize, j: usize) -> f64 {
        let u = if i == 0 { self.uv.x.lo } else { self.uv.x.hi };
        let v = if j == 0 { self.uv.y.lo } else { self.uv.y.hi };
        LatLng::longitude(Point(face_uv_to_xyz(self.face, u, v))).radians()
    }
}

/// Returns the minimum of two `ChordAngles`.
#[inline]
fn chord_min(a: ChordAngle, b: ChordAngle) -> ChordAngle {
    if a < b { a } else { b }
}

fn chord_max(a: ChordAngle, b: ChordAngle) -> ChordAngle {
    if a > b { a } else { b }
}

impl From<CellId> for Cell {
    fn from(id: CellId) -> Self {
        Cell::from_cell_id(id)
    }
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn cell_is_send_sync() {
        is_send_sync::<Cell>();
    }

    #[test]
    fn test_from_cell_id() {
        for face in Face::iter() {
            let id = CellId::from_face(face);
            let c = Cell::from_cell_id(id);
            assert_eq!(c.face(), face);
            assert_eq!(c.level(), 0);
            assert_eq!(c.id(), id);
        }
    }

    #[test]
    fn test_from_point() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let c = Cell::from_point(p);
        assert!(c.is_leaf());
        assert!(c.contains_point(p));
    }

    #[test]
    fn test_vertex_count() {
        let c = Cell::from_cell_id(CellId::from_face(0));
        // Vertices should be unit-length.
        for k in 0..4 {
            let v = c.vertex(k);
            assert!(
                (v.vector().norm() - 1.0).abs() < 1e-14,
                "vertex {k} not unit length: {}",
                v.vector().norm(),
            );
        }
    }

    #[test]
    fn test_edge_raw_inward() {
        // For face 0 (positive x), edge normals should point inward.
        let c = Cell::from_cell_id(CellId::from_face(0));
        let center = c.center();
        for k in CellEdge::ALL {
            let e = c.edge_raw(k);
            // Dot product with center should be positive (inward normal).
            assert!(
                center.vector().dot(e.vector()) > 0.0,
                "edge {k:?} not pointing inward",
            );
        }
    }

    #[test]
    fn test_children() {
        let parent = Cell::from_cell_id(CellId::from_face(0));
        let children = parent.children().expect("should have children");
        for child in &children {
            assert_eq!(child.level(), 1);
            assert_eq!(child.face(), Face::F0);
            assert!(parent.contains_cell(*child));
        }
    }

    #[test]
    fn test_children_leaf() {
        // Leaf cell should return None.
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let leaf = Cell::from_point(p);
        assert!(leaf.is_leaf());
        assert!(leaf.children().is_none());
    }

    #[test]
    fn test_contains_point_center() {
        for face in Face::iter() {
            let c = Cell::from_cell_id(CellId::from_face(face));
            assert!(c.contains_point(c.center()));
        }
    }

    #[test]
    fn test_contains_cell() {
        let parent = Cell::from_cell_id(CellId::from_face(0));
        let child = Cell::from_cell_id(CellId::from_face(0).children()[0]);
        assert!(parent.contains_cell(child));
        assert!(!child.contains_cell(parent));
    }

    #[test]
    fn test_intersects_cell() {
        let c0 = Cell::from_cell_id(CellId::from_face(0));
        let c1 = Cell::from_cell_id(CellId::from_face(1));
        // Different face cells don't intersect in the CellId sense.
        assert!(!c0.intersects_cell(c1));
        // A cell intersects itself.
        assert!(c0.intersects_cell(c0));
    }

    #[test]
    fn test_cap_bound() {
        for face in Face::iter() {
            let c = Cell::from_cell_id(CellId::from_face(face));
            let cap = c.cap_bound();
            for k in 0..4 {
                assert!(cap.contains_point(c.vertex(k)));
            }
        }
    }

    #[test]
    fn test_rect_bound() {
        for face in Face::iter() {
            let c = Cell::from_cell_id(CellId::from_face(face));
            let r = c.rect_bound();
            assert!(r.is_valid());
            for k in 0..4 {
                assert!(
                    r.contains_point(c.vertex(k)),
                    "face {face}: rect_bound does not contain vertex {k}",
                );
            }
        }
    }

    #[test]
    fn test_rect_bound_subcells() {
        // Test that rect_bound works for several levels.
        let id = CellId::from_face(0);
        for level in 1..=5u8 {
            let child_id = id.child_begin_at_level(level);
            let child = Cell::from_cell_id(child_id);
            let r = child.rect_bound();
            assert!(r.is_valid());
            assert!(r.contains_point(child.center()));
        }
    }

    #[test]
    fn test_average_area() {
        // Level 0 face cell has area = 4*PI/6.
        let c = Cell::from_cell_id(CellId::from_face(0));
        let expected = 4.0 * PI / 6.0;
        assert!((c.average_area() - expected).abs() < 1e-14);
    }

    #[test]
    fn test_approx_area() {
        let c = Cell::from_cell_id(CellId::from_face(0));
        // Level 0 area ≈ average area.
        let avg = c.average_area();
        let approx = c.approx_area();
        assert!((approx - avg).abs() / avg < 0.05);
    }

    #[test]
    fn test_cell_union_bound() {
        let c = Cell::from_cell_id(CellId::from_face(0));
        let bound = c.cell_union_bound();
        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0], c.id());
    }

    // ─── Distance method tests ──────────────────────────────────────

    #[test]
    fn test_distance_to_point_inside() {
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(5));
        let center = c.center();
        assert_eq!(c.distance_to_point(center), ChordAngle::ZERO);
    }

    #[test]
    fn test_distance_to_point_outside() {
        // Face 0 cell at level 10, center is near a corner of face 0.
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(10));
        // Point far away on opposite face.
        let far = Point::from_coords(-1.0, 0.0, 0.0);
        let dist = c.distance_to_point(far);
        // Face 0 corner is ~125 degrees from (-1,0,0), so distance should be > 100.
        assert!(
            dist.to_angle().degrees() > 100.0,
            "dist = {} deg",
            dist.to_angle().degrees()
        );
    }

    #[test]
    fn test_distance_to_point_vertex() {
        // Distance from cell to one of its own vertices should be 0.
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(5));
        let v = c.vertex(0);
        assert!(c.distance_to_point(v) <= ChordAngle::from_length2(1e-20));
    }

    #[test]
    fn test_distance_to_edge_crossing() {
        // An edge that passes through the cell should have distance 0.
        let c = Cell::from_cell_id(CellId::from_face(0));
        let a = Point::from_coords(1.0, -0.5, -0.5);
        let b = Point::from_coords(1.0, 0.5, 0.5);
        assert_eq!(c.distance_to_edge(a, b), ChordAngle::ZERO);
    }

    #[test]
    fn test_distance_to_edge_far() {
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(10));
        // Edge on opposite side of the sphere.
        let a = Point::from_coords(-1.0, 0.0, -0.1);
        let b = Point::from_coords(-1.0, 0.0, 0.1);
        let dist = c.distance_to_edge(a, b);
        assert!(dist.to_angle().degrees() > 100.0);
    }

    #[test]
    fn test_distance_to_cell_same() {
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(5));
        assert_eq!(c.distance_to_cell(c), ChordAngle::ZERO);
    }

    #[test]
    fn test_distance_to_cell_adjacent() {
        // Two adjacent cells at the same level should have distance ~0.
        let id = CellId::from_face(0).child_begin_at_level(5);
        let c1 = Cell::from_cell_id(id);
        let c2 = Cell::from_cell_id(id.next());
        let dist = c1.distance_to_cell(c2);
        // Adjacent cells share an edge, so distance should be 0.
        assert!(
            dist.to_angle().degrees() < 0.01,
            "adjacent cell dist = {} deg",
            dist.to_angle().degrees()
        );
    }

    #[test]
    fn test_max_distance_to_point() {
        let c = Cell::from_cell_id(CellId::from_face(0));
        let p = c.center();
        let max_dist = c.max_distance_to_point(p);
        // Max distance from center to any face-0 vertex.
        assert!(max_dist.to_angle().degrees() > 30.0);
    }

    #[test]
    fn test_max_distance_to_cell() {
        // Two cells on opposite faces.
        let c1 = Cell::from_cell_id(CellId::from_face(0));
        let c2 = Cell::from_cell_id(CellId::from_face(3));
        let max_dist = c1.max_distance_to_cell(c2);
        // Some vertex pair should be > 90 degrees apart.
        assert!(max_dist.to_angle().degrees() > 90.0);
    }

    // ─── New method tests ──────────────────────────────────────

    #[test]
    fn test_center_raw() {
        // center_raw should be proportional to center but unnormalized.
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(10));
        let raw = c.center_raw();
        let norm = c.center();
        let normalized = Point(raw.vector().normalize());
        let diff = (normalized.vector() - norm.vector()).norm();
        assert!(diff < 1e-14);
        // Raw should NOT be unit length (unless it happens to be).
        // For a non-leaf cell, the raw point magnitude depends on cell size.
    }

    #[test]
    fn test_boundary_distance_outside() {
        // Point outside the cell — boundary_distance == distance_to_point.
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(10));
        let far = Point::from_coords(-1.0, 0.0, 0.0);
        let bd = c.boundary_distance(far);
        let dp = c.distance_to_point(far);
        assert!(
            (bd.length2() - dp.length2()).abs() < 1e-14,
            "boundary dist {} != distance_to_point {}",
            bd.length2(),
            dp.length2()
        );
    }

    #[test]
    fn test_boundary_distance_inside() {
        // Point inside the cell — distance_to_point is 0 but boundary_distance > 0.
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(5));
        let center = c.center();
        assert_eq!(c.distance_to_point(center), ChordAngle::ZERO);
        let bd = c.boundary_distance(center);
        assert!(
            bd > ChordAngle::ZERO,
            "boundary_distance for interior point should be > 0, got {}",
            bd.length2()
        );
    }

    #[test]
    fn test_is_distance_less() {
        let c1 = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(5));
        let c2 = Cell::from_cell_id(CellId::from_face(3).child_begin_at_level(5));
        // Opposite-face cells: distance is large.
        assert!(!c1.is_distance_less(c2, ChordAngle::from_degrees(10.0)));
        assert!(c1.is_distance_less(c2, ChordAngle::STRAIGHT));
    }

    #[test]
    fn test_is_distance_less_or_equal() {
        let c = Cell::from_cell_id(CellId::from_face(0).child_begin_at_level(5));
        // A cell's distance to itself is 0.
        assert!(c.is_distance_less_or_equal(c, ChordAngle::ZERO));
        assert!(!c.is_distance_less(c, ChordAngle::ZERO));
    }

    #[test]
    fn test_is_max_distance_less() {
        let c1 = Cell::from_cell_id(CellId::from_face(0));
        let c2 = Cell::from_cell_id(CellId::from_face(3));
        // Max distance between opposite-face cells is large.
        assert!(!c1.is_max_distance_less(c2, ChordAngle::STRAIGHT));
    }

    #[test]
    fn test_is_max_distance_less_or_equal() {
        let c1 = Cell::from_cell_id(CellId::from_face(0));
        let c2 = Cell::from_cell_id(CellId::from_face(0));
        // Max distance within same face cell is bounded.
        let max = c1.max_distance_to_cell(c2);
        assert!(c1.is_max_distance_less_or_equal(c2, max));
    }

    #[test]
    fn test_uv_coord_of_edge() {
        let c = Cell::from_cell_id(CellId::from_face(0));
        // Edge 0 is bottom (v=lo), edge 1 is right (u=hi),
        // edge 2 is top (v=hi), edge 3 is left (u=lo).
        let uv = c.bound_uv();
        assert!((c.uv_coord_of_edge(CellEdge::Bottom) - uv.y.lo).abs() < 1e-14);
        assert!((c.uv_coord_of_edge(CellEdge::Right) - uv.x.hi).abs() < 1e-14);
        assert!((c.uv_coord_of_edge(CellEdge::Top) - uv.y.hi).abs() < 1e-14);
        assert!((c.uv_coord_of_edge(CellEdge::Left) - uv.x.lo).abs() < 1e-14);
    }

    #[test]
    fn test_ij_coord_of_edge() {
        // Face-0 cell at level 0 covers the full IJ range [0, MAX_SIZE].
        let c = Cell::from_cell_id(CellId::from_face(0));
        // Bottom edge (j=0), Top edge (j=MAX_SIZE).
        let ij0 = c.ij_coord_of_edge(CellEdge::Bottom);
        let ij2 = c.ij_coord_of_edge(CellEdge::Top);
        assert_eq!(ij0, 0, "bottom edge IJ should be 0");
        assert_eq!(ij2, size_ij(0), "top edge IJ should be MAX_SIZE");
    }

    #[test]
    fn test_from_face_constructor() {
        for face in Face::iter() {
            let c = Cell::from_face(face);
            assert_eq!(c.face(), face);
            assert_eq!(c.level(), 0);
        }
    }

    #[test]
    fn test_from_face_pos_level() {
        let c1 = Cell::from_face_pos_level(0, 0, 1);
        assert_eq!(c1.face(), Face::F0);
        assert_eq!(c1.level(), 1);
    }

    #[test]
    fn test_orientation() {
        // Verify orientation is in range [0, 3] for various cells.
        for face in Face::iter() {
            let c = Cell::from_face(face);
            assert!(c.orientation() < 4);
            if let Some(children) = c.children() {
                for child in &children {
                    assert!(child.orientation() < 4);
                }
            }
        }
    }

    #[test]
    fn test_size_ij_and_st() {
        // Level 0: size_ij = 2^30, size_st = 1.0.
        let c0 = Cell::from_face(0);
        assert_eq!(c0.size_ij(), 1 << 30);
        assert!((c0.size_st() - 1.0).abs() < 1e-14);

        // Level 30 (leaf): size_ij = 1.
        let leaf = Cell::from_point(Point::from_coords(1.0, 0.0, 0.0));
        assert_eq!(leaf.size_ij(), 1);
        assert!(leaf.size_st() > 0.0);
        assert!(leaf.size_st() < 1e-8);
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn prop_cell_id_roundtrip(face: u8) -> bool {
        let face = Face::from_u8(face % 6);
        let id = CellId::from_face(face);
        let c = Cell::from_cell_id(id);
        c.id() == id && c.face() == face && c.level() == 0
    }

    // --- TestFaces: verify face geometry (ported from C++) ---

    #[test]
    fn test_faces() {
        use std::collections::HashMap;

        // Quantize a point to an integer triple (sign-based for cube vertices).
        let key = |p: Point| -> (i64, i64, i64) {
            (
                p.0.x.signum() as i64,
                p.0.y.signum() as i64,
                p.0.z.signum() as i64,
            )
        };

        let mut vertex_counts: HashMap<(i64, i64, i64), i32> = HashMap::new();

        for face in Face::iter() {
            let id = CellId::from_face(face);
            let cell = Cell::from_cell_id(id);
            assert_eq!(cell.id(), id);
            assert_eq!(cell.face(), face);
            assert_eq!(cell.level(), 0);
            assert!(!cell.is_leaf());

            for k in 0..4 {
                let vk = cell.vertex_raw(k);
                *vertex_counts.entry(key(vk)).or_insert(0) += 1;
            }
        }

        // Each cube vertex (±1,±1,±1) should appear exactly 3 times
        // (shared by 3 faces).
        assert_eq!(vertex_counts.len(), 8, "should be 8 distinct cube vertices");
        for count in vertex_counts.values() {
            assert_eq!(*count, 3, "each vertex should be shared by exactly 3 faces");
        }
    }

    // --- ConsistentWithS2CellIdFromPoint ---

    #[test]
    fn test_consistent_with_cell_id_from_point() {
        // Verify S2Cell from a point contains that point, for various positions.
        let test_points = [
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
            Point::from_coords(1.0, 1.0, 1.0).normalize(),
            Point::from_coords(-1.0, 0.5, 0.3).normalize(),
            Point::from_coords(0.1, -0.9, 0.4).normalize(),
        ];
        for p in &test_points {
            let id = CellId::from_point(p);
            let cell = Cell::from_cell_id(id);
            assert!(
                cell.contains_point(*p),
                "Cell({id:?}) does not contain its source point",
            );
        }
    }

    // --- Distance comparison tests ---

    #[test]
    fn test_distance_to_point_zero_for_center() {
        // Distance from cell to its own center should be zero.
        for face in Face::iter() {
            let cell = Cell::from_cell_id(CellId::from_face(face));
            let dist = cell.distance_to_point(cell.center());
            assert_eq!(dist, ChordAngle::ZERO);
        }
    }

    #[test]
    fn test_distance_to_cell_self_is_zero() {
        // Distance from a cell to itself should be zero.
        let cell = Cell::from_cell_id(CellId::from_face(0));
        assert_eq!(cell.distance_to_cell(cell), ChordAngle::ZERO);
    }

    #[test]
    fn test_max_distance_to_point_vertex() {
        use crate::s1::ChordAngle;
        // Max distance from face cell to its vertex should be positive.
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let max_dist = cell.max_distance_to_point(cell.vertex(0));
        assert!(
            max_dist > ChordAngle::ZERO,
            "max distance to vertex should be > 0"
        );
    }

    #[test]
    fn test_cell_union_bound_includes_only_self() {
        // A leaf cell's CellUnionBound should contain only itself.
        let id = CellId::from_token("123456789");
        let cell = Cell::from_cell_id(id);
        let bound = cell.cell_union_bound();
        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0], cell.id());
    }

    #[test]
    fn test_consistent_with_cell_id_from_point_example1() {
        // Specific edge case: cell created from a point contains that point.
        let p = Point::from_coords(
            0.38203141040035632,
            0.030196609707941954,
            0.9236558700239289,
        );
        let cell = Cell::from_cell_id(CellId::from_point(&p));
        assert!(
            cell.contains_point(p),
            "Cell from point should contain that point"
        );
    }

    #[test]
    fn test_cap_bound_contains_all_vertices() {
        // The cap bound of any cell should contain all 4 vertices.
        for face in Face::iter() {
            let cell = Cell::from_cell_id(CellId::from_face(face));
            let cap = cell.cap_bound();
            for k in 0..4 {
                assert!(
                    cap.contains_point(cell.vertex(k)),
                    "cap bound of face {face} should contain vertex {k}",
                );
            }
        }
    }

    #[test]
    fn test_rect_bound_contains_center() {
        // The rect bound of any face cell should contain its center.
        for face in Face::iter() {
            let cell = Cell::from_cell_id(CellId::from_face(face));
            let rect = cell.rect_bound();
            let center_ll = LatLng::from_point(cell.center());
            assert!(
                rect.contains_lat_lng(center_ll),
                "rect bound of face {face} should contain its center",
            );
        }
    }

    #[quickcheck]
    fn prop_contains_center(face: u8) -> bool {
        let face = face % 6;
        let c = Cell::from_cell_id(CellId::from_face(face));
        c.contains_point(c.center())
    }

    #[quickcheck]
    fn prop_vertex_unit_length(face: u8, k: u8) -> bool {
        let face = face % 6;
        let k = (k % 4) as usize;
        let c = Cell::from_cell_id(CellId::from_face(face));
        (c.vertex(k).vector().norm() - 1.0).abs() < 1e-14
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(face: u8) -> bool {
        let face = face % 6;
        let c = Cell::from_cell_id(CellId::from_face(face));
        let json1 = serde_json::to_string(&c).unwrap();
        let back: Cell = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        let back2: Cell = serde_json::from_str(&json2).unwrap();
        back == back2
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_cell_edge_roundtrip() {
        for e in [
            CellEdge::Bottom,
            CellEdge::Right,
            CellEdge::Top,
            CellEdge::Left,
        ] {
            let json = serde_json::to_string(&e).unwrap();
            let back: CellEdge = serde_json::from_str(&json).unwrap();
            assert_eq!(e, back);
        }
    }
}
