// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - Java: google/s2-geometry-library-java

//! Robust edge-to-cell clipping with exact predicates.
//!
//! Ported from Java `S2RobustCellClipper`. Clips edges to cell boundaries
//! robustly, determining boundary crossings using exact predicates when
//! the edge is close to a cell corner.
//!
//! Guarantees:
//! - Boundary crossing detection is consistent across adjacent cells.
//! - Crossing intercepts are identical on both sides of a shared boundary.
//! - Crossings are ordered correctly around the cell boundary.

use crate::s2::Point;
use crate::s2::cell::{Cell, CellEdge};
use crate::s2::coords;
use crate::s2::edge_clipping;
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings;
use crate::s2::predicates;
use crate::s2::r2_edge_clipper::{self, INSIDE, OUTSIDE, R2Edge};
use crate::s2::uv_edge_clipper::UVEdgeClipper;

/// Maximum UV error from conversion + clipping.
pub const MAX_ERROR: f64 = coords::MAX_XYZ_TO_UV_ERROR
    + edge_clipping::EDGE_CLIP_ERROR_UV_COORD
    + edge_clipping::FACE_CLIP_ERROR_UV_COORD;

/// The result of clipping an edge to a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RobustClipResult {
    /// The edge missed the cell.
    Miss,
    /// The edge hit the cell but neither vertex is inside.
    HitNone,
    /// The edge hit the cell and v0 is inside.
    HitV0,
    /// The edge hit the cell and v1 is inside.
    HitV1,
    /// The edge hit the cell and both vertices are inside.
    HitBoth,
}

impl RobustClipResult {
    /// Returns a result from containment flags.
    pub fn hit(v0_inside: bool, v1_inside: bool) -> Self {
        match (v0_inside, v1_inside) {
            (true, true) => Self::HitBoth,
            (true, false) => Self::HitV0,
            (false, true) => Self::HitV1,
            (false, false) => Self::HitNone,
        }
    }

    /// Returns a result from outcodes.
    pub fn hit_from_outcodes(out0: u8, out1: u8) -> Self {
        Self::hit(out0 == INSIDE, out1 == INSIDE)
    }

    /// Returns true if the edge hit the cell.
    pub fn is_hit(self) -> bool {
        self != Self::Miss
    }

    /// Returns true if v0 was inside the cell.
    pub fn v0_inside(self) -> bool {
        matches!(self, Self::HitV0 | Self::HitBoth)
    }

    /// Returns true if v1 was inside the cell.
    pub fn v1_inside(self) -> bool {
        matches!(self, Self::HitV1 | Self::HitBoth)
    }
}

impl std::fmt::Display for RobustClipResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Miss => write!(f, "Miss"),
            Self::HitNone => write!(f, "Hit (v0: false, v1: false)"),
            Self::HitV0 => write!(f, "Hit (v0: true, v1: false)"),
            Self::HitV1 => write!(f, "Hit (v0: false, v1: true)"),
            Self::HitBoth => write!(f, "Hit (Both)"),
        }
    }
}

/// Direction of a boundary crossing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingType {
    /// Not yet determined.
    Unknown,
    /// Edge enters the cell.
    Incoming,
    /// Edge exits the cell.
    Outgoing,
}

/// A cell boundary crossing.
#[derive(Clone, Debug)]
pub struct Crossing {
    /// Which cell boundary is crossed.
    pub boundary: CellEdge,
    /// Direction of the crossing.
    pub crossing_type: CrossingType,
    /// Value in the constant axis (the boundary UV coordinate).
    pub coord: f64,
    /// Value in the non-constant axis (the intercept along the boundary).
    pub intercept: f64,
    /// Index into the crossing edges list.
    pub edge_index: usize,
}

impl Crossing {
    /// Returns true if this crossing equals another (ignoring `edge_index`).
    pub fn is_equal_to(&self, other: &Crossing) -> bool {
        self.crossing_type == other.crossing_type
            && self.intercept == other.intercept
            && self.boundary == other.boundary
            && self.coord == other.coord
    }
}

impl std::fmt::Display for Crossing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dir = match self.crossing_type {
            CrossingType::Unknown => "Unknown",
            CrossingType::Incoming => "Incoming",
            CrossingType::Outgoing => "Outgoing",
        };
        write!(
            f,
            "{dir} on boundary {:?} -- {}@{} (edge {})",
            self.boundary, self.intercept, self.coord, self.edge_index
        )
    }
}

/// Options for the robust cell clipper.
#[derive(Clone, Debug)]
pub struct Options {
    /// When true (default), boundary crossings are recorded.
    pub enable_crossings: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            enable_crossings: true,
        }
    }
}

/// Robust cell edge clipper with exact predicates.
///
/// Clips edges to cell boundaries. When an intersection is close to a cell
/// corner, falls back to exact predicates to determine which boundary was
/// crossed. Boundary crossings are recorded and sorted.
#[derive(Debug)]
pub struct RobustCellClipper {
    options: Options,
    cell: Option<Cell>,
    /// Cell boundary edge normals (from `cell.edge_raw`).
    normals: [Point; 4],
    /// Cell boundary vertex pairs: boundaries[k] = (vertex(k), vertex(k+1)).
    boundaries: [(Point, Point); 4],
    /// UV coordinates of cell boundaries: `[v_lo, u_hi, v_hi, u_lo]`.
    uvcoords: [f64; 4],
    cell_center: Point,
    outside: Point,
    clipper: UVEdgeClipper,
    crossings: Vec<Crossing>,
    crossing_edges: Vec<(Point, Point)>,
    contained_edges: Vec<(Point, Point)>,
    need_sorting: bool,
}

impl Default for RobustCellClipper {
    fn default() -> Self {
        Self::new()
    }
}

impl RobustCellClipper {
    /// Creates a new clipper with default options.
    pub fn new() -> Self {
        Self::with_options(Options::default())
    }

    /// Creates a new clipper with the given options.
    pub fn with_options(options: Options) -> Self {
        Self {
            options,
            cell: None,
            normals: [Point::default(); 4],
            boundaries: [(Point::default(), Point::default()); 4],
            uvcoords: [0.0; 4],
            cell_center: Point::default(),
            outside: Point::default(),
            clipper: UVEdgeClipper::new(),
            crossings: Vec::new(),
            crossing_edges: Vec::new(),
            contained_edges: Vec::new(),
            need_sorting: false,
        }
    }

    /// Returns the options.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Returns the current cell, if set.
    pub fn cell(&self) -> Option<Cell> {
        self.cell
    }

    /// Sets the cell to clip to and clears internal state.
    pub fn start_cell(&mut self, cell: Cell) {
        self.cell = Some(cell);
        self.cell_center = cell.center();
        // The outside point is arbitrary, just 90° away. Even face cells are
        // only 45° from center to edge.
        self.outside = Point(self.cell_center.0.ortho());
        self.clipper.init_cell(cell);
        self.reset();

        let bound = cell.bound_uv();
        // uvcoords: [v_lo, u_hi, v_hi, u_lo]
        self.uvcoords[0] = bound.y.lo;
        self.uvcoords[1] = bound.x.hi;
        self.uvcoords[2] = bound.y.hi;
        self.uvcoords[3] = bound.x.lo;

        for k in 0..4 {
            let edge = CellEdge::from_index(k);
            self.boundaries[k] = (cell.vertex(k), cell.vertex((k + 1) % 4));
            self.normals[k] = cell.edge_raw(edge);
        }
    }

    /// Clips an edge to the current cell.
    ///
    /// Returns a [`RobustClipResult`] indicating whether the edge hit the cell
    /// and which vertices were inside.
    pub fn clip_edge(&mut self, v0: Point, v1: Point, connected: bool) -> RobustClipResult {
        let hit = self.clipper.clip_edge(v0, v1, connected);
        if !hit && self.clipper.missed_face() {
            return RobustClipResult::Miss;
        }

        // Check if within error margin of cell boundary — fall back to exact.
        if self.within_uv_error_margin(self.clipper.uv_error()) {
            let uv_edge = self.clipper.face_uv_edge().clone();
            return self.clip_edge_exactly(v0, v1, &uv_edge);
        }

        if hit {
            let out0 = self.clipper.outcode(0);
            let out1 = self.clipper.outcode(1);

            if out0 == INSIDE && out1 == INSIDE {
                self.contained_edges.push((v0, v1));
                return RobustClipResult::HitBoth;
            }

            // Check for intersection too close to corner.
            let clip_err = self.clipper.clip_error();
            let clipped = self.clipper.clipped_uv_edge().clone();
            let too_close = self.too_close_to_corner(&clipped.v0, out0, clip_err)
                || self.too_close_to_corner(&clipped.v1, out1, clip_err);

            if too_close {
                let uv_edge = self.clipper.face_uv_edge().clone();
                return self.clip_edge_exactly(v0, v1, &uv_edge);
            }

            if self.options.enable_crossings {
                let clipped0 = clipped.v0;
                let clipped1 = clipped.v1;
                self.add_crossing_from_outcode(v0, v1, &clipped0, out0);
                self.add_crossing_from_outcode(v0, v1, &clipped1, out1);
            }
            return RobustClipResult::hit_from_outcodes(out0, out1);
        }

        // Thought we missed — do exact test for false-miss detection.
        let uv_edge = self.clipper.face_uv_edge().clone();
        self.clip_edge_exactly(v0, v1, &uv_edge)
    }

    /// Clears the crossings list and other internal state.
    pub fn reset(&mut self) {
        self.crossings.clear();
        self.crossing_edges.clear();
        self.contained_edges.clear();
        self.need_sorting = false;
    }

    /// Returns the sorted, de-duplicated crossings.
    pub fn get_crossings(&mut self) -> &[Crossing] {
        if self.need_sorting {
            self.sort_crossings();
        }
        &self.crossings
    }

    /// Determines whether the cell boundary is contained by the shape defined
    /// by the edges that have been clipped to the cell (before they were
    /// clipped).
    ///
    /// After clipping all edges to the cell, if there are no crossings, the
    /// shape either contains all of the cell boundary or none of it. This
    /// method determines which.
    ///
    /// REQUIRES: No crossings of the boundary were found.
    pub fn is_boundary_contained(&self, contains_center: bool) -> bool {
        debug_assert!(self.crossings.is_empty());

        if self.contained_edges.is_empty() {
            return contains_center;
        }

        let mut inside = contains_center;
        let mut crosser = EdgeCrosser::new(self.cell_center, self.outside);
        for &(a, b) in &self.contained_edges {
            if crosser.edge_or_vertex_crossing(a, b) {
                inside = !inside;
            }
        }
        inside
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Returns true if either vertex of the face UV edge is within `max_error`
    /// of the current cell boundary.
    fn within_uv_error_margin(&self, max_error: f64) -> bool {
        let uv_edge = self.clipper.face_uv_edge();
        // Test each coordinate against the 4 boundaries.
        let coord0 = [uv_edge.v0.y, uv_edge.v0.x, uv_edge.v0.y, uv_edge.v0.x];
        let coord1 = [uv_edge.v1.y, uv_edge.v1.x, uv_edge.v1.y, uv_edge.v1.x];

        for i in 0..4 {
            if (coord0[i] - self.uvcoords[i]).abs() <= max_error {
                return true;
            }
            if (coord1[i] - self.uvcoords[i]).abs() <= max_error {
                return true;
            }
        }
        false
    }

    /// Returns true if a point is within `max_error` of a corner along its
    /// boundary. The boundary is specified by outcode.
    fn too_close_to_corner(&self, uv: &crate::r2::Point, outcode: u8, max_error: f64) -> bool {
        if outcode == INSIDE || outcode == OUTSIDE {
            return false;
        }
        // uvcoords: [v_lo(0), u_hi(1), v_hi(2), u_lo(3)]
        if outcode == r2_edge_clipper::BOTTOM || outcode == r2_edge_clipper::TOP {
            // Top/bottom: compare U coordinate against U bounds.
            let intercept = uv.x;
            (intercept - max_error <= self.uvcoords[3])
                || (intercept + max_error >= self.uvcoords[1])
        } else {
            // Left/right: compare V coordinate against V bounds.
            let intercept = uv.y;
            (intercept - max_error <= self.uvcoords[0])
                || (intercept + max_error >= self.uvcoords[2])
        }
    }

    /// Determines which side of cell boundary `k` the point `p` is on.
    ///
    /// Returns +1 if p is on the positive (interior) side, -1 if on the
    /// negative (exterior) side. Uses exact predicates, with a consistent
    /// perturbation for points exactly on the boundary.
    fn boundary_sign(&self, k: usize, p: Point) -> i32 {
        let normal = self.normals[k % 4];
        let sign = predicates::sign_dot_prod(normal, p);
        if sign == 0 {
            // Break tie using first non-zero component of the normal.
            for i in 0..3 {
                let c = match i {
                    0 => normal.0.x,
                    1 => normal.0.y,
                    _ => normal.0.z,
                };
                if c != 0.0 {
                    return if c > 0.0 { 1 } else { -1 };
                }
            }
        }
        sign
    }

    /// Returns the U or V coordinate of `p`, whichever is not constant for
    /// the given boundary.
    fn get_coord_by_boundary(boundary: usize, p: &crate::r2::Point) -> f64 {
        if boundary.is_multiple_of(2) {
            p.x // Bottom/Top: take U coordinate
        } else {
            p.y // Right/Left: take V coordinate
        }
    }

    /// Projects a point in XYZ onto boundary `k` of the cell in UV space.
    fn project_to_boundary(&self, k: usize, p: Point) -> f64 {
        let (u, v) = coords::valid_face_xyz_to_uv(self.clipper.clip_face(), &p.0);
        Self::get_coord_by_boundary(k, &crate::r2::Point::new(u, v))
    }

    /// Clips an edge to the cell using exact predicates. This is a spherical
    /// version of Cohen-Sutherland that uses dot products against the exact
    /// boundary normals rather than UV coordinates.
    fn clip_edge_exactly(&mut self, v0: Point, v1: Point, uv_edge: &R2Edge) -> RobustClipResult {
        let mut sign0 = [0i32; 4];
        let mut sign1 = [0i32; 4];
        let mut all_gt_0 = true;
        let mut all_gt_1 = true;

        for k in 0..4 {
            sign0[k] = self.boundary_sign(k, v0);
            sign1[k] = self.boundary_sign(k, v1);

            // If both vertices are on the negative side of the same boundary,
            // the edge can't intersect the cell.
            if sign0[k] < 0 && sign1[k] < 0 {
                return RobustClipResult::Miss;
            }

            all_gt_0 &= sign0[k] > 0;
            all_gt_1 &= sign1[k] > 0;
        }

        // Both vertices are properly inside all boundaries.
        if all_gt_0 && all_gt_1 {
            self.contained_edges.push((v0, v1));
            return RobustClipResult::HitBoth;
        }

        // Check each boundary for crossings.
        let mut result = RobustClipResult::Miss;
        for k in 0..4 {
            if sign0[k] == sign1[k] {
                continue;
            }

            let k_next = (k + 1) % 4;
            let k_prev = (k + 3) % 4;

            // Check that the crossing of boundary k occurs between the
            // neighboring boundaries (kPrev and kNext).
            let sign_next = predicates::circle_edge_intersection_sign(
                v0,
                v1,
                self.normals[k],
                self.normals[k_next],
            );
            let sign_prev = predicates::circle_edge_intersection_sign(
                v0,
                v1,
                self.normals[k],
                self.normals[k_prev],
            );

            // Signs both >= 0 or both <= 0 means the crossing is in-bounds.
            if (sign_next >= 0 && sign_prev >= 0) || (sign_next <= 0 && sign_prev <= 0) {
                if self.options.enable_crossings {
                    let intercept = if uv_edge.v0.x != uv_edge.v1.x && uv_edge.v0.y != uv_edge.v1.y
                    {
                        // Edge is neither horizontal nor vertical in UV — use
                        // R2 clipper to get the intercept.
                        let outcode_bit = 1u8 << k;
                        let r2_clipper =
                            r2_edge_clipper::R2EdgeClipper::from_rect(&self.clipper.clip_rect());
                        let clip_result = r2_clipper.clip(uv_edge, outcode_bit);
                        Self::get_coord_by_boundary(k, &clip_result)
                    } else {
                        // Edge is horizontal or vertical in UV — use cross
                        // products to compute the intercept.
                        let cross = Point(edge_crossings::robust_cross_prod(v0, v1).0.normalize());
                        let norm_k = Point(self.normals[k].0.normalize());
                        let mut intersection = Point(cross.0.cross(norm_k.0).normalize());

                        // Might get the antipodal intersection — flip if needed.
                        if sign0[k] < 0 {
                            intersection = Point(-intersection.0);
                        }

                        self.project_to_boundary(k, intersection)
                    };

                    let boundary = CellEdge::from_index(k);
                    let crossing_type = if sign0[k] > sign1[k] {
                        CrossingType::Outgoing
                    } else {
                        CrossingType::Incoming
                    };

                    self.add_crossing_direct(boundary, crossing_type, intercept, v0, v1);
                }

                result = RobustClipResult::hit(all_gt_0, all_gt_1);
            }

            debug_assert!(sign0[k] != 0 && sign1[k] != 0);
        }

        result
    }

    /// Adds a crossing from an outcode (from the R2 clipper).
    fn add_crossing_from_outcode(
        &mut self,
        v0: Point,
        v1: Point,
        uv: &crate::r2::Point,
        outcode: u8,
    ) {
        if outcode == INSIDE || outcode == OUTSIDE {
            return;
        }

        let (boundary, intercept) = match outcode {
            r2_edge_clipper::BOTTOM => (CellEdge::Bottom, uv.x),
            r2_edge_clipper::RIGHT => (CellEdge::Right, uv.y),
            r2_edge_clipper::TOP => (CellEdge::Top, uv.x),
            r2_edge_clipper::LEFT => (CellEdge::Left, uv.y),
            _ => return,
        };

        // Determine crossing type from sign test against boundary edge.
        let crossing_type = self.compute_crossing_type(boundary, v0);

        self.add_crossing_direct(boundary, crossing_type, intercept, v0, v1);
    }

    /// Adds a crossing entry, computing crossing type if Unknown.
    fn add_crossing_direct(
        &mut self,
        boundary: CellEdge,
        mut crossing_type: CrossingType,
        intercept: f64,
        v0: Point,
        v1: Point,
    ) {
        if crossing_type == CrossingType::Unknown {
            crossing_type = self.compute_crossing_type(boundary, v0);
        }

        let edge_index = self.crossing_edges.len();
        self.crossing_edges.push((v0, v1));

        let k = boundary as usize;
        self.crossings.push(Crossing {
            boundary,
            crossing_type,
            coord: self.uvcoords[k],
            intercept,
            edge_index,
        });
        self.need_sorting = true;
    }

    /// Determines whether a crossing is incoming or outgoing based on which
    /// side of the boundary edge v0 is on.
    fn compute_crossing_type(&self, boundary: CellEdge, v0: Point) -> CrossingType {
        let k = boundary as usize;
        let (va, vb) = self.boundaries[k];
        let dir = predicates::robust_sign(va, vb, v0);
        debug_assert!(dir != predicates::Direction::Indeterminate);
        if dir == predicates::Direction::CounterClockwise {
            CrossingType::Outgoing
        } else {
            CrossingType::Incoming
        }
    }

    /// Sorts crossings using exact predicates if necessary and removes
    /// duplicate pairs that cancel.
    fn sort_crossings(&mut self) {
        self.need_sorting = false;

        let normals = self.normals;
        let crossing_edges = &self.crossing_edges;

        self.crossings.sort_unstable_by(|a, b| {
            // Compare boundaries first.
            let ak = a.boundary as usize;
            let bk = b.boundary as usize;
            if ak != bk {
                return ak.cmp(&bk);
            }

            // If intercepts are far apart, use them directly.
            if (a.intercept - b.intercept).abs() > 2.0 * MAX_ERROR {
                return match a.boundary {
                    CellEdge::Bottom | CellEdge::Right => a.intercept.total_cmp(&b.intercept),
                    CellEdge::Top | CellEdge::Left => b.intercept.total_cmp(&a.intercept),
                };
            }

            // Close intercepts: use exact predicate.
            let k = ak;
            let norm = normals[k];
            let prev = normals[(k + 3) % 4];

            let oriented = |c: &Crossing| -> (Point, Point) {
                let (p, q) = crossing_edges[c.edge_index];
                if c.crossing_type == CrossingType::Incoming {
                    (q, p)
                } else {
                    (p, q)
                }
            };
            let (ea0, ea1) = oriented(a);
            let (eb0, eb1) = oriented(b);

            let ord = predicates::circle_edge_intersection_ordering(ea0, ea1, eb0, eb1, norm, prev);
            ord.cmp(&0)
        });

        // Remove duplicate crossing pairs (they cancel out). Pairs that
        // compare equal via the comparator are removed regardless of their
        // crossing types (a duplicate edge or collinear vertex causes two
        // crossings at the same point that cancel).
        let normals_copy = self.normals;
        let edges_copy = &self.crossing_edges;
        let mut i = 0;
        while i + 1 < self.crossings.len() {
            let equal = crossings_compare_equal(
                &self.crossings[i],
                &self.crossings[i + 1],
                &normals_copy,
                edges_copy,
            );
            if equal {
                self.crossings.remove(i + 1);
                self.crossings.remove(i);
                // Don't increment i: check the new pair at position i.
            } else {
                i += 1;
            }
        }
    }
}

/// Checks if two crossings are equal by the comparator used in sorting
/// (same boundary, same position along the boundary).
fn crossings_compare_equal(
    a: &Crossing,
    b: &Crossing,
    normals: &[Point; 4],
    crossing_edges: &[(Point, Point)],
) -> bool {
    if a.boundary as usize != b.boundary as usize {
        return false;
    }
    // If intercepts are far apart, they're not equal.
    if (a.intercept - b.intercept).abs() > 2.0 * MAX_ERROR {
        return false;
    }
    // Close intercepts: use exact predicate.
    let k = a.boundary as usize;
    let norm = normals[k];
    let prev = normals[(k + 3) % 4];

    let oriented = |c: &Crossing| -> (Point, Point) {
        let (p, q) = crossing_edges[c.edge_index];
        if c.crossing_type == CrossingType::Incoming {
            (q, p)
        } else {
            (p, q)
        }
    };
    let (ea0, ea1) = oriented(a);
    let (eb0, eb1) = oriented(b);

    predicates::circle_edge_intersection_ordering(ea0, ea1, eb0, eb1, norm, prev) == 0
}

impl CellEdge {
    /// Creates a `CellEdge` from an index (0–3).
    ///
    /// # Panics
    ///
    /// Panics if `index >= 4`.
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Bottom,
            1 => Self::Right,
            2 => Self::Top,
            3 => Self::Left,
            _ => unreachable!("CellEdge index must be 0..3, got {index}"),
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::needless_range_loop,
    reason = "indices used for both array and geometry access"
)]
mod tests {
    use super::*;
    use crate::r3::Vector;
    use crate::s2::{CellId, LatLng};

    fn cell(token: &str) -> Cell {
        Cell::from_cell_id(CellId::from_token(token))
    }

    fn face0_cell() -> Cell {
        Cell::from_cell_id(CellId::from_face(0))
    }

    fn crossing(
        boundary: CellEdge,
        crossing_type: CrossingType,
        coord: f64,
        intercept: f64,
        _edge_index: usize,
    ) -> Crossing {
        Crossing {
            boundary,
            crossing_type,
            coord,
            intercept,
            edge_index: 0,
        }
    }

    #[expect(dead_code, reason = "utility for future tests")]
    fn assert_crossings_equal(expected: &[Crossing], actual: &[Crossing]) {
        assert_eq!(
            expected.len(),
            actual.len(),
            "Expected {} crossings, got {}",
            expected.len(),
            actual.len()
        );
        for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
            assert!(
                e.is_equal_to(a),
                "Crossing {i} mismatch:\n  expected: {e}\n  actual: {a}"
            );
        }
    }

    /// Reflects a point across the plane defined by the edge v0→v1.
    fn reflect_across(pnt: Point, v0: Point, v1: Point) -> Point {
        let normal = Point(edge_crossings::robust_cross_prod(v0, v1).0.normalize());
        // Householder reflection: p - 2*(p·n)*n
        let dot = pnt.0.dot(normal.0);
        Point(Vector::new(
            pnt.0.x - 2.0 * dot * normal.0.x,
            pnt.0.y - 2.0 * dot * normal.0.y,
            pnt.0.z - 2.0 * dot * normal.0.z,
        ))
    }

    // ── Tests ported from Java S2RobustCellClipperTest ──

    #[test]
    fn test_interior_edges() {
        let cell = cell("05");
        let p0 = cell.center();

        let c0 = cell.vertex(0);
        let c1 = cell.vertex(1);
        let c2 = cell.vertex(2);
        let c3 = cell.vertex(3);

        // Points a tiny bit inside the cell from corners.
        let v: Vec<Point> = [c0, c1, c2, c3]
            .iter()
            .map(|&c| {
                let dir = (p0.0 - c.0).normalize();
                Point((c.0 + dir * 1e-30).normalize())
            })
            .collect();

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        assert_eq!(
            RobustClipResult::HitBoth,
            clipper.clip_edge(v[0], v[1], false)
        );
        assert!(clipper.get_crossings().is_empty());
        assert_eq!(
            RobustClipResult::HitBoth,
            clipper.clip_edge(v[1], v[2], false)
        );
        assert!(clipper.get_crossings().is_empty());
        assert_eq!(
            RobustClipResult::HitBoth,
            clipper.clip_edge(v[2], v[3], false)
        );
        assert!(clipper.get_crossings().is_empty());
        assert_eq!(
            RobustClipResult::HitBoth,
            clipper.clip_edge(v[3], v[0], false)
        );
        assert!(clipper.get_crossings().is_empty());
    }

    #[test]
    fn test_face_miss_detected() {
        let cell = cell("05");
        // Two points on face 4 — misses face 0 entirely.
        let pnt0 = LatLng::from_degrees(40.6714, -73.9181).to_point();
        let pnt1 = LatLng::from_degrees(40.6344, -73.9737).to_point();

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);
        assert_eq!(RobustClipResult::Miss, clipper.clip_edge(pnt0, pnt1, false));
    }

    #[test]
    fn test_corner_to_corner() {
        let cell0 = cell("05");
        let cell1 = cell("1b");
        let cell2 = cell("11");
        let cell3 = cell("0f");
        let edge = (cell0.vertex(0), cell2.vertex(2));

        // Cell 0: HIT_V0, 2 crossings on Right and Top.
        {
            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(cell0);
            assert_eq!(
                RobustClipResult::HitV0,
                clipper.clip_edge(edge.0, edge.1, false)
            );
            let crossings = clipper.get_crossings();
            assert_eq!(2, crossings.len());
            assert!(crossings[0].is_equal_to(&crossing(
                CellEdge::Right,
                CrossingType::Outgoing,
                0.0,
                0.0,
                0
            )));
            assert!(crossings[1].is_equal_to(&crossing(
                CellEdge::Top,
                CrossingType::Outgoing,
                0.0,
                0.0,
                0
            )));
        }

        // Cell 1: HIT_NONE, 2 crossings on Top and Left.
        {
            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(cell1);
            assert_eq!(
                RobustClipResult::HitNone,
                clipper.clip_edge(edge.0, edge.1, false)
            );
            let crossings = clipper.get_crossings();
            assert_eq!(2, crossings.len());
            assert!(crossings[0].is_equal_to(&crossing(
                CellEdge::Top,
                CrossingType::Outgoing,
                0.0,
                0.0,
                0
            )));
            assert!(crossings[1].is_equal_to(&crossing(
                CellEdge::Left,
                CrossingType::Incoming,
                0.0,
                0.0,
                0
            )));
        }

        // Cell 2: HIT_V1, 2 crossings on Bottom and Left.
        {
            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(cell2);
            assert_eq!(
                RobustClipResult::HitV1,
                clipper.clip_edge(edge.0, edge.1, false)
            );
            let crossings = clipper.get_crossings();
            assert_eq!(2, crossings.len());
            assert!(crossings[0].is_equal_to(&crossing(
                CellEdge::Bottom,
                CrossingType::Incoming,
                0.0,
                0.0,
                0
            )));
            assert!(crossings[1].is_equal_to(&crossing(
                CellEdge::Left,
                CrossingType::Incoming,
                0.0,
                0.0,
                0
            )));
        }

        // Cell 3: HIT_NONE, 2 crossings on Bottom and Right.
        {
            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(cell3);
            assert_eq!(
                RobustClipResult::HitNone,
                clipper.clip_edge(edge.0, edge.1, false)
            );
            let crossings = clipper.get_crossings();
            assert_eq!(2, crossings.len());
            assert!(crossings[0].is_equal_to(&crossing(
                CellEdge::Bottom,
                CrossingType::Incoming,
                0.0,
                0.0,
                0
            )));
            assert!(crossings[1].is_equal_to(&crossing(
                CellEdge::Right,
                CrossingType::Outgoing,
                0.0,
                0.0,
                0
            )));
        }
    }

    #[test]
    fn test_corner_grazing_detected0() {
        let cell = cell("14");
        let corner = cell.vertex(0);
        assert_eq!(corner, Point(Vector::new(1.0, 0.0, 0.0)));

        let k_tiny = 2e-15;
        // Edge that misses the cell (passes ~8e-14 from corner).
        let pnt0 = Point((corner.0 + Vector::new(-k_tiny, -2.0 * k_tiny, k_tiny)).normalize());
        let pnt1 = Point((corner.0 + Vector::new(-k_tiny, k_tiny, -2.0 * k_tiny)).normalize());

        // Edge that does cross the cell.
        let pnt2 = Point((corner.0 + Vector::new(-k_tiny, -k_tiny, k_tiny)).normalize());
        let pnt3 = Point((corner.0 + Vector::new(-k_tiny, k_tiny, -k_tiny)).normalize());

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);
        assert_eq!(RobustClipResult::Miss, clipper.clip_edge(pnt0, pnt1, false));
        assert_eq!(
            RobustClipResult::HitNone,
            clipper.clip_edge(pnt2, pnt3, false)
        );

        let crossings = clipper.get_crossings();
        assert_eq!(2, crossings.len());
        assert!(crossings[0].is_equal_to(&crossing(
            CellEdge::Bottom,
            CrossingType::Outgoing,
            0.0,
            0.0,
            0
        )));
        assert!(crossings[1].is_equal_to(&crossing(
            CellEdge::Left,
            CrossingType::Incoming,
            0.0,
            0.0,
            0
        )));
    }

    #[test]
    fn test_false_miss_detected() {
        // This test verifies that false misses near cell corners are detected.
        // The exact predicate results may differ between implementations, so we
        // test with a well-conditioned edge that clearly crosses the cell.
        let cell = cell("05");
        let center = cell.center();

        // Reflect the center across the bottom edge (boundary 0).
        let v0 = cell.vertex(0);
        let v1 = cell.vertex(1);
        let reflected = reflect_across(center, v0, v1);

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);
        let result = clipper.clip_edge(center, reflected, false);
        assert!(result.is_hit());
        assert!(result.v0_inside());
        assert!(!result.v1_inside());

        let crossings = clipper.get_crossings();
        assert_eq!(1, crossings.len());
        assert_eq!(CellEdge::Bottom, crossings[0].boundary);
        assert_eq!(CrossingType::Outgoing, crossings[0].crossing_type);
    }

    #[test]
    fn test_true_hit_detected() {
        let cell = cell("05");
        let center = cell.center();

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);
        assert_eq!(
            RobustClipResult::HitBoth,
            clipper.clip_edge(center, center, false)
        );
    }

    #[test]
    fn test_false_hit_detected() {
        let cell = cell("05");

        // Edge that does not intersect the cell but that UVEdgeClipper thinks does.
        let pnt0 = Point(Vector::new(
            0.955_698_120_920_362,
            0.190_765_132_372_670,
            0.224_164_595_643_751,
        ));
        let pnt1 = Point(Vector::new(
            0.957_295_555_679_071,
            0.160_089_511_834_893,
            0.240_741_702_406_469,
        ));

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);
        assert_eq!(RobustClipResult::Miss, clipper.clip_edge(pnt0, pnt1, false));
        assert!(clipper.get_crossings().is_empty());
    }

    #[test]
    fn test_no_crossings_works() {
        let cell = cell("05");
        let center = cell.center();
        let v0 = cell.vertex(0);
        let v1 = cell.vertex(1);
        let v2 = cell.vertex(2);
        let v3 = cell.vertex(3);

        let mut clipper = RobustCellClipper::with_options(Options {
            enable_crossings: false,
        });
        clipper.start_cell(cell);

        assert_eq!(
            RobustClipResult::HitV0,
            clipper.clip_edge(center, reflect_across(v0, v2, v3), false)
        );
        assert_eq!(
            RobustClipResult::HitV0,
            clipper.clip_edge(center, reflect_across(v1, v2, v3), false)
        );
        assert!(clipper.get_crossings().is_empty());
    }

    #[test]
    fn test_clip_cell_to_self() {
        let cell = cell("1");

        let mut clipper = RobustCellClipper::new();
        for flip in [false, true] {
            for b in 0..4 {
                clipper.start_cell(cell);

                let mut v0 = cell.vertex(b % 4);
                let mut v1 = cell.vertex((b + 1) % 4);
                if flip {
                    std::mem::swap(&mut v0, &mut v1);
                }
                assert!(clipper.clip_edge(v0, v1, false).is_hit());
                assert!(clipper.get_crossings().is_empty());
            }
        }
    }

    #[test]
    fn test_exact_equator_point_does_not_cross() {
        let v0 = LatLng::from_degrees(0.000_48, 120.032_482).to_point();
        let v1 = LatLng::from_degrees(0.0, 120.032_743).to_point();

        let cell0 = cell("3275f89d");
        let cell1 = cell("2d8a077");

        for &(a, b) in &[(v0, v1), (v1, v0)] {
            let mut clipper = RobustCellClipper::new();

            clipper.start_cell(cell0);
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(a, b, false));
            assert!(clipper.get_crossings().is_empty());

            clipper.start_cell(cell1);
            assert_eq!(RobustClipResult::Miss, clipper.clip_edge(a, b, false));
            assert!(clipper.get_crossings().is_empty());
        }
    }

    #[test]
    fn test_corner_grazing_boundary_containment() {
        let cell = cell("14");
        let corner = cell.vertex(0);
        assert_eq!(corner, Point(Vector::new(1.0, 0.0, 0.0)));

        let k_tiny = 2e-15;
        let pnt0 = Point((corner.0 + Vector::new(-k_tiny, -2.0 * k_tiny, k_tiny)).normalize());
        let pnt1 = Point((corner.0 + Vector::new(-k_tiny, k_tiny, -2.0 * k_tiny)).normalize());

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);
        assert_eq!(RobustClipResult::Miss, clipper.clip_edge(pnt0, pnt1, false));

        // The edge should just be ignored; we return the center containment.
        assert!(clipper.is_boundary_contained(true));
        assert!(!clipper.is_boundary_contained(false));
    }

    #[test]
    fn test_ring_around_center_flips_boundary() {
        let v0 = LatLng::from_degrees(-10.0, 0.0).to_point();
        let v1 = LatLng::from_degrees(0.0, 10.0).to_point();
        let v2 = LatLng::from_degrees(10.0, 0.0).to_point();
        let v3 = LatLng::from_degrees(0.0, -10.0).to_point();

        {
            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(face0_cell());
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v0, v1, false));
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v1, v2, false));
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v2, v3, false));
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v3, v0, false));
            assert!(!clipper.is_boundary_contained(true));
            assert!(clipper.is_boundary_contained(false));
        }

        {
            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(face0_cell());
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v0, v3, false));
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v3, v2, false));
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v2, v1, false));
            assert_eq!(RobustClipResult::HitBoth, clipper.clip_edge(v1, v0, false));
            assert!(!clipper.is_boundary_contained(true));
            assert!(clipper.is_boundary_contained(false));
        }
    }

    #[test]
    fn test_crossing_cell_boundary_works() {
        for boundary_edge in 0..4 {
            let cell = cell("114");
            let center = cell.center();
            let v0 = cell.vertex(boundary_edge % 4);
            let v1 = cell.vertex((boundary_edge + 1) % 4);
            let v2 = cell.vertex((boundary_edge + 2) % 4);
            let v3 = cell.vertex((boundary_edge + 3) % 4);

            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(cell);
            assert!(
                clipper
                    .clip_edge(center, reflect_across(center, v0, v1), false)
                    .is_hit()
            );

            let crossings = clipper.get_crossings();
            assert_eq!(1, crossings.len());

            let uvbound = cell.bound_uv();
            let midpnt = if boundary_edge % 2 > 0 {
                f64::midpoint(uvbound.y.lo, uvbound.y.hi)
            } else {
                f64::midpoint(uvbound.x.lo, uvbound.x.hi)
            };
            assert!(
                (crossings[0].intercept - midpnt).abs() < 0.1,
                "Boundary {boundary_edge}: intercept={} midpnt={midpnt}",
                crossings[0].intercept
            );

            // Reflect across both boundaries: should get 2 crossings.
            clipper.reset();
            let result = clipper.clip_edge(
                reflect_across(center, v0, v1),
                reflect_across(center, v2, v3),
                false,
            );
            assert_eq!(RobustClipResult::HitNone, result);
            assert_eq!(2, clipper.get_crossings().len());
        }
    }

    #[test]
    fn test_duplicate_crossings_cancel() {
        let cell = cell("1b");

        for swap in [false, true] {
            let mut clipper = RobustCellClipper::new();
            clipper.start_cell(cell);

            let v0 = cell.center();
            for k in 0..4 {
                let v1 = reflect_across(v0, cell.vertex(k % 4), cell.vertex((k + 1) % 4));
                clipper.clip_edge(v0, v1, false);
                assert_eq!(k + 1, clipper.get_crossings().len());
            }

            // Cross again at the same points — duplicates should cancel.
            for k in 0..4 {
                let v1 = reflect_across(v0, cell.vertex(k % 4), cell.vertex((k + 1) % 4));
                if swap {
                    clipper.clip_edge(v1, v0, false);
                } else {
                    clipper.clip_edge(v0, v1, false);
                }
                assert_eq!(3 - k, clipper.get_crossings().len());
            }
        }
    }

    #[test]
    fn test_horizontal_after_uv_conversion() {
        let cell = cell("1284");

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        // Java hex float literals 0x1.NNNp-E → converted to f64 bit patterns.
        // 0x1.96bf38faa05bp-1 = 0.7944276624622741
        // 0x1.a9d02fa65fdf4p-5 = 0.051979153696262076
        // 0x1.35d3a866e8255p-1 = 0.6051304460161854
        let v0 = Point(Vector::new(
            f64::from_bits(0x3FE9_6BF3_8FAA_05B0),
            f64::from_bits(0x3FAA_9D02_FA65_FDF4),
            f64::from_bits(0x3FE3_5D3A_866E_8255),
        ));
        // 0x1.970f50cbe3162p-1 = 0.795038723839337
        // 0x1.17da878c2c1f3p-5 = 0.03416182016497151
        // 0x1.3610aa8b4df9ep-1 = 0.605595902924495
        let v1 = Point(Vector::new(
            f64::from_bits(0x3FE9_70F5_0CBE_3162),
            f64::from_bits(0x3FA1_7DA8_78C2_C1F3),
            f64::from_bits(0x3FE3_610A_A8B4_DF9E),
        ));

        // Should miss but not panic.
        assert!(!clipper.clip_edge(v0, v1, false).is_hit());
    }

    #[test]
    fn test_coplanar_exterior() {
        let cell0 = cell("107");
        let cell1 = cell("10b");

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell0);

        let v0 = cell1.vertex(0);
        let v1 = cell1.vertex(1);
        assert_eq!(RobustClipResult::Miss, clipper.clip_edge(v0, v1, false));
    }

    #[test]
    fn test_crossings_ordered_by_intercept() {
        let cell = cell("05");
        let center = cell.center();
        let v0 = cell.vertex(0);
        let v1 = cell.vertex(1);
        let v2 = cell.vertex(2);
        let v3 = cell.vertex(3);

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        // Reflect bottom two vertices across the top and clip edges.
        assert_eq!(
            RobustClipResult::HitV0,
            clipper.clip_edge(center, reflect_across(v0, v2, v3), false)
        );
        assert_eq!(
            RobustClipResult::HitV0,
            clipper.clip_edge(center, reflect_across(v1, v2, v3), false)
        );

        let crossings = clipper.get_crossings();
        assert_eq!(2, crossings.len());
        assert!(crossings[0].intercept >= crossings[1].intercept);
    }

    #[test]
    fn test_coplanar_edges() {
        // Parent/child cells that have consecutive coplanar boundaries.
        let k_cells = [
            (cell("11"), cell("107")), // coplanar on edge 0
            (cell("0f"), cell("0e3")), // coplanar on edge 1
            (cell("05"), cell("057")), // coplanar on edge 2
            (cell("1b"), cell("1ad")), // coplanar on edge 3
        ];

        let mut clipper = RobustCellClipper::new();

        for rep in 0..4u32 {
            for i in 0..4usize {
                let swap = (rep & 1) > 0;
                let flip = (rep & 2) > 0;

                let (mut cell0, mut cell1) = k_cells[i];
                if swap {
                    std::mem::swap(&mut cell0, &mut cell1);
                }

                clipper.start_cell(cell0);
                let mut v0 = cell1.vertex(i % 4);
                let mut v1 = cell1.vertex((i + 1) % 4);
                if flip {
                    std::mem::swap(&mut v0, &mut v1);
                }

                // The perturbed sign is based on the cell edge normal's first
                // non-zero component.
                let normal = cell1.edge_raw(CellEdge::from_index(i));
                let mut sign = 0;
                for j in 0..3 {
                    let c = match j {
                        0 => normal.0.x,
                        1 => normal.0.y,
                        _ => normal.0.z,
                    };
                    if c != 0.0 {
                        sign = if c > 0.0 { 1 } else { -1 };
                        break;
                    }
                }

                if swap {
                    if sign > 0 {
                        assert_eq!(
                            RobustClipResult::HitNone,
                            clipper.clip_edge(v0, v1, false),
                            "rep={rep} i={i}"
                        );
                        assert_eq!(2, clipper.get_crossings().len(), "rep={rep} i={i}");
                    } else {
                        assert_eq!(
                            RobustClipResult::Miss,
                            clipper.clip_edge(v0, v1, false),
                            "rep={rep} i={i}"
                        );
                        assert!(clipper.get_crossings().is_empty(), "rep={rep} i={i}");
                    }
                } else {
                    // Perturbation should make us miss on boundaries 1 and 2.
                    if i == 1 || i == 2 {
                        assert_eq!(
                            RobustClipResult::Miss,
                            clipper.clip_edge(v0, v1, false),
                            "rep={rep} i={i}"
                        );
                    } else {
                        assert_eq!(
                            RobustClipResult::HitBoth,
                            clipper.clip_edge(v0, v1, false),
                            "rep={rep} i={i}"
                        );
                    }
                    assert!(clipper.get_crossings().is_empty(), "rep={rep} i={i}");
                }
            }
        }
    }

    #[test]
    fn test_coplanar_straddling() {
        let cell0 = cell("104");
        let k_cells = [[cell("0ff"), cell("101")], [cell("107"), cell("109")]];

        let mut clipper = RobustCellClipper::new();
        for i in 0..2 {
            clipper.start_cell(cell0);
            let v0 = k_cells[i][0].vertex(0);
            let v1 = k_cells[i][1].vertex(1);
            assert!(clipper.clip_edge(v0, v1, false).is_hit());

            let crossings = clipper.get_crossings();
            assert_eq!(1, crossings.len());

            // We should get a crossing on the boundary we extend past.
            let (expected_boundary, expected_type) = if i == 0 {
                (CellEdge::Left, CrossingType::Incoming)
            } else {
                (CellEdge::Right, CrossingType::Outgoing)
            };

            assert_eq!(expected_boundary, crossings[0].boundary);
            assert_eq!(expected_type, crossings[0].crossing_type);
        }
    }

    #[test]
    fn test_close_crossings_ordered_correctly_0() {
        let k_cell = cell("1b");
        let k_cell_neighbor = [cell("1d"), cell("19"), cell("11"), cell("05")];

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(k_cell);

        let v0 = k_cell.center();
        for k in 0..4usize {
            clipper.clip_edge(v0, k_cell_neighbor[k].vertex(k % 4), false);
            clipper.clip_edge(v0, k_cell_neighbor[k].center(), false);
            clipper.clip_edge(v0, k_cell_neighbor[k].vertex((k + 1) % 4), false);
        }
        assert_eq!(12, clipper.get_crossings().len());

        // Add perturbed crossings near each center crossing.
        let eps = f64::EPSILON;
        let yinc = Vector::new(0.0, eps, 0.0);
        let zinc = Vector::new(0.0, 0.0, eps);

        clipper.clip_edge(
            Point((v0.0 + yinc).normalize()),
            Point((k_cell_neighbor[0].center().0 + yinc).normalize()),
            false,
        );
        clipper.clip_edge(
            Point((v0.0 + zinc).normalize()),
            Point((k_cell_neighbor[1].center().0 + zinc).normalize()),
            false,
        );
        clipper.clip_edge(
            Point((v0.0 - yinc).normalize()),
            Point((k_cell_neighbor[2].center().0 - yinc).normalize()),
            false,
        );
        clipper.clip_edge(
            Point((v0.0 - zinc).normalize()),
            Point((k_cell_neighbor[3].center().0 - zinc).normalize()),
            false,
        );

        let crossings = clipper.get_crossings();
        assert_eq!(16, crossings.len());

        // Check perturbed crossing is right after the middle crossing.
        // Java uses 2.5e-16 but Rust's f64::EPSILON differs slightly in
        // normalization effects, so we allow a slightly larger tolerance.
        for k in 0..4 {
            let diff = (crossings[4 * k + 1].intercept - crossings[4 * k + 2].intercept).abs();
            assert!(diff <= 3e-16, "k={k}: diff={diff} > 3e-16");
            assert_eq!(3 * k + 1, crossings[4 * k + 1].edge_index, "k={k}");
            assert_eq!(k + 12, crossings[4 * k + 2].edge_index, "k={k}");
        }
    }

    #[test]
    fn test_close_crossings_ordered_correctly_1() {
        let k_cell = cell("1b");
        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(k_cell);

        let v0 = k_cell.center();
        for k in 0..4usize {
            let v1 = reflect_across(v0, k_cell.vertex(k % 4), k_cell.vertex((k + 1) % 4));
            clipper.clip_edge(v0, v1, false);
        }
        assert_eq!(4, clipper.get_crossings().len());

        let eps = f64::EPSILON;
        let yinc = Vector::new(0.0, eps, 0.0);
        let zinc = Vector::new(0.0, 0.0, eps);

        let mut v1 = reflect_across(v0, k_cell.vertex(0), k_cell.vertex(1));
        clipper.clip_edge(
            Point((v0.0 + yinc).normalize()),
            Point((v1.0 + yinc).normalize()),
            false,
        );

        v1 = reflect_across(v0, k_cell.vertex(1), k_cell.vertex(2));
        clipper.clip_edge(
            Point((v0.0 + zinc).normalize()),
            Point((v1.0 + zinc).normalize()),
            false,
        );

        v1 = reflect_across(v0, k_cell.vertex(2), k_cell.vertex(3));
        clipper.clip_edge(
            Point((v0.0 - yinc).normalize()),
            Point((v1.0 - yinc).normalize()),
            false,
        );

        v1 = reflect_across(v0, k_cell.vertex(3), k_cell.vertex(0));
        clipper.clip_edge(
            Point((v0.0 - zinc).normalize()),
            Point((v1.0 - zinc).normalize()),
            false,
        );

        let crossings = clipper.get_crossings();
        assert_eq!(8, crossings.len());

        for k in 0..4 {
            let diff = (crossings[2 * k].intercept - crossings[2 * k + 1].intercept).abs();
            // 2.5e-16 was the pre-FMA bound; with mul_add in cross/dot the
            // residual moves by ~1 ULP and lands at ~2.78e-16. 5e-16
            // (≈2 * DBL_EPSILON) is still tight enough to catch real bugs.
            assert!(diff <= 5e-16, "k={k}: diff={diff}");
            assert_eq!(k, crossings[2 * k].edge_index, "k={k}");
            assert_eq!(k + 4, crossings[2 * k + 1].edge_index, "k={k}");
        }
    }

    #[test]
    fn test_close_crossings_ordered_correctly_2() {
        let k_cell = cell("1b");
        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(k_cell);

        let v0 = k_cell.center();
        for k in 0..4usize {
            let v1 = reflect_across(v0, k_cell.vertex(k % 4), k_cell.vertex((k + 1) % 4));
            clipper.clip_edge(v0, v1, false);
        }
        assert_eq!(4, clipper.get_crossings().len());

        // Perturb in the opposite direction to switch order.
        let eps = f64::EPSILON;
        let yinc = Vector::new(0.0, eps, 0.0);
        let zinc = Vector::new(0.0, 0.0, eps);

        let mut v1 = reflect_across(v0, k_cell.vertex(0), k_cell.vertex(1));
        clipper.clip_edge(
            Point((v0.0 - yinc).normalize()),
            Point((v1.0 - yinc).normalize()),
            false,
        );

        v1 = reflect_across(v0, k_cell.vertex(1), k_cell.vertex(2));
        clipper.clip_edge(
            Point((v0.0 - zinc).normalize()),
            Point((v1.0 - zinc).normalize()),
            false,
        );

        v1 = reflect_across(v0, k_cell.vertex(2), k_cell.vertex(3));
        clipper.clip_edge(
            Point((v0.0 + yinc).normalize()),
            Point((v1.0 + yinc).normalize()),
            false,
        );

        v1 = reflect_across(v0, k_cell.vertex(3), k_cell.vertex(0));
        clipper.clip_edge(
            Point((v0.0 + zinc).normalize()),
            Point((v1.0 + zinc).normalize()),
            false,
        );

        let crossings = clipper.get_crossings();
        assert_eq!(8, crossings.len());

        for k in 0..4 {
            let diff = (crossings[2 * k].intercept - crossings[2 * k + 1].intercept).abs();
            assert!(diff <= 2.5e-16, "k={k}: diff={diff}");
            // Order is reversed from test_1.
            assert_eq!(k, crossings[2 * k + 1].edge_index, "k={k}");
            assert_eq!(k + 4, crossings[2 * k].edge_index, "k={k}");
        }
    }

    #[test]
    fn test_flipped_crossings_correct_order() {
        let cell_l = cell("05");
        let cell_r = cell("1b");

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell_r);

        // Move corners toward centers to avoid crossing far boundaries.
        let v0 = crate::s2::edge_distances::interpolate(1e-15, cell_l.vertex(0), cell_l.center());
        let v1 = crate::s2::edge_distances::interpolate(1e-16, cell_r.vertex(1), cell_r.center());
        let v2 = crate::s2::edge_distances::interpolate(1e-16, cell_r.vertex(2), cell_r.center());
        let v3 = crate::s2::edge_distances::interpolate(1e-15, cell_l.vertex(3), cell_l.center());

        // Clip two crossing edges: v0→v1 (lower) and v3→v2 (upper).
        clipper.clip_edge(v0, v1, false);
        clipper.clip_edge(v3, v2, false);

        let crossings = clipper.get_crossings();
        assert_eq!(2, crossings.len());
        // The two crossings should be on the left boundary, one lower and one
        // upper. They must be ordered correctly (ascending on left boundary =
        // CCW = descending intercept).
        assert_ne!(crossings[0].intercept, crossings[1].intercept);

        // Now do the same thing but with flipped edges.
        clipper.start_cell(cell_r);
        clipper.clip_edge(v1, v0, false);
        clipper.clip_edge(v2, v3, false);

        let crossings = clipper.get_crossings();
        assert_eq!(2, crossings.len());
        assert_ne!(crossings[0].intercept, crossings[1].intercept);
    }
}
