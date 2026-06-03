// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - Java: google/s2-geometry-library-java

//! Tracks shape visitation completeness across S2 cells.
//!
//! Ported from Java `S2ShapeTracker`. Determines when all pieces of a shape
//! have been visited during a traversal of an `ShapeIndex`.
//!
//! The tracker works by:
//! 1. Tracking which chains have been seen via a bitset.
//! 2. Tracking IJ-coordinate boundary segments in two hash maps (one per
//!    axis). Segments are toggled on/off so that matching pairs on opposite
//!    sides of cell boundaries cancel.
//!
//! A shape is "finished" when all chains are seen and both maps are empty.

use std::collections::HashMap;

use crate::s2::cell::{Cell, CellEdge};
use crate::s2::coords;
use crate::s2::robust_cell_clipper::{Crossing, CrossingType};
use crate::s2::shape::Dimension;

/// `LIMIT_IJ` as an `i64` for IJ coordinate math.
const LIMIT_IJ: i64 = coords::LIMIT_IJ as i64;

/// Tracks whether all parts of a shape have been visited.
#[derive(Debug)]
pub struct ShapeTracker {
    dimension: Dimension,
    num_chains: usize,
    chains_seen: Vec<bool>,
    chains_seen_count: usize,
    /// IJ-coordinate point maps, one per axis. Map from packed (face,i,j) key
    /// to a signed count.
    points: [HashMap<u64, i32>; 2],
}

impl ShapeTracker {
    /// Creates a new tracker for a shape with the given dimension and chain count.
    pub fn new(dimension: Dimension, num_chains: usize) -> Self {
        Self {
            dimension,
            num_chains,
            chains_seen: vec![false; num_chains],
            chains_seen_count: 0,
            points: [HashMap::new(), HashMap::new()],
        }
    }

    /// Resets the tracker for a new shape.
    pub fn reset(&mut self, dimension: Dimension, num_chains: usize) {
        self.dimension = dimension;
        self.num_chains = num_chains;
        self.chains_seen = vec![false; num_chains];
        self.chains_seen_count = 0;
        self.points[0].clear();
        self.points[1].clear();
    }

    /// Marks a chain as seen. Idempotent.
    pub fn mark_chain(&mut self, chain: usize) {
        if !self.chains_seen[chain] {
            self.chains_seen[chain] = true;
            self.chains_seen_count += 1;
        }
    }

    /// Returns true if the shape is finished: all chains seen and no
    /// outstanding boundary segments.
    pub fn finished(&self) -> bool {
        self.chains_seen_count >= self.num_chains
            && self.points[0].is_empty()
            && self.points[1].is_empty()
    }

    /// Processes a list of cell boundary crossings from
    /// [`RobustCellClipper`](super::robust_cell_clipper::RobustCellClipper).
    pub fn process_crossings(&mut self, cell: Cell, crossings: &[Crossing]) {
        let ncrossing = crossings.len();
        if ncrossing == 0 || self.dimension == Dimension::Point {
            return;
        }

        let face = i32::from(cell.face().as_u8());

        // Precompute UV and IJ coords for each boundary.
        let mut cell_uv_coords = [0.0f64; 4];
        let mut cell_ij_coords = [0i64; 4];
        let bound = cell.bound_uv();
        // uvcoords: [v_lo, u_hi, v_hi, u_lo]
        cell_uv_coords[0] = bound.y.lo;
        cell_uv_coords[1] = bound.x.hi;
        cell_uv_coords[2] = bound.y.hi;
        cell_uv_coords[3] = bound.x.lo;

        for k in 0..4 {
            cell_ij_coords[k] = uv_to_ij_round(cell_uv_coords[k]);
        }

        if self.dimension == Dimension::Polyline {
            // Polyline: each crossing is a point toggle.
            for cr in crossings {
                let ij = uv_to_ij_round(cr.intercept);
                let axis = constant_boundary_axis(face, cr.boundary);
                let coord = cell_ij_coords[cr.boundary as usize];

                if (cr.boundary as usize) < 2 {
                    self.add_point(face, axis, coord, ij);
                } else {
                    self.del_point(face, axis, coord, ij);
                }
            }
            return;
        }

        // Polygon: scan around the boundary and make intervals where the
        // shape contains the boundary.
        let mut interior = crossings[0].crossing_type == CrossingType::Incoming;
        let mut i = 0;

        for b in 0..4u8 {
            let boundary = CellEdge::from_index(b as usize);
            let axis = constant_boundary_axis(face, boundary);
            let coord = cell_ij_coords[b as usize];

            let b_next = ((b + 1) % 4) as usize;
            let b_prev = ((b + 3) % 4) as usize;

            let uv_beg = cell_uv_coords[b_prev];
            let uv_end = cell_uv_coords[b_next];
            let ij_beg = cell_ij_coords[b_prev];
            let ij_end = cell_ij_coords[b_next];

            let ordered = |x: f64, y: f64| -> bool { if b < 2 { x < y } else { x > y } };

            // First crossing on this boundary.
            if i < ncrossing && crossings[i].boundary == boundary {
                let cr = &crossings[i];
                let mut uv_prev = cr.intercept;
                i += 1;

                if interior && ordered(uv_beg, cr.intercept) {
                    let ij0 = ij_beg;
                    let ij1 = if b < 2 {
                        uv_to_ij_ceil(cr.intercept)
                    } else {
                        uv_to_ij_floor(cr.intercept)
                    };
                    if ij0 != ij1 {
                        self.add_interval(face, axis, coord, ij0, ij1);
                    }
                }

                interior = cr.crossing_type == CrossingType::Outgoing;

                // Remaining crossings on this boundary.
                while i < ncrossing && crossings[i].boundary == boundary {
                    let cr = &crossings[i];

                    if interior {
                        let uv0 = uv_prev.min(cr.intercept);
                        let uv1 = uv_prev.max(cr.intercept);

                        let mut ij0 = uv_to_ij_floor(uv0);
                        let mut ij1 = uv_to_ij_ceil(uv1);
                        if b >= 2 {
                            std::mem::swap(&mut ij0, &mut ij1);
                        }
                        if ij0 != ij1 {
                            self.add_interval(face, axis, coord, ij0, ij1);
                        }
                    }

                    interior = cr.crossing_type == CrossingType::Outgoing;
                    uv_prev = cr.intercept;
                    i += 1;
                }

                // Final segment to the end of this boundary.
                if interior && ordered(uv_prev, uv_end) {
                    let ij0 = if b < 2 {
                        uv_to_ij_floor(uv_prev)
                    } else {
                        uv_to_ij_ceil(uv_prev)
                    };
                    let ij1 = ij_end;
                    if ij0 != ij1 {
                        self.add_interval(face, axis, coord, ij0, ij1);
                    }
                }
            } else {
                // No crossings on this boundary — add entire boundary if
                // interior.
                if interior {
                    self.add_interval(face, axis, coord, ij_beg, ij_end);
                }
            }
        }
        debug_assert_eq!(i, ncrossing);
    }

    /// Adds all 4 boundary segments of a cell to the tracker.
    ///
    /// Used when a cell has no edges but is contained by the shape (2D only).
    pub fn add_cell_boundary(&mut self, cell: Cell) {
        let face = i32::from(cell.face().as_u8());
        let bound = cell.bound_uv();
        let uv = [bound.y.lo, bound.x.hi, bound.y.hi, bound.x.lo];
        let mut ij = [0i64; 4];
        for k in 0..4 {
            ij[k] = uv_to_ij_round(uv[k]);
        }

        let bottom_axis = constant_boundary_axis(face, CellEdge::Bottom);
        let right_axis = constant_boundary_axis(face, CellEdge::Right);
        let top_axis = constant_boundary_axis(face, CellEdge::Top);
        let left_axis = constant_boundary_axis(face, CellEdge::Left);

        // Bottom: left→right, Right: bottom→top, Top: right→left, Left: top→bottom.
        self.add_interval(face, bottom_axis, ij[0], ij[3], ij[1]);
        self.add_interval(face, right_axis, ij[1], ij[0], ij[2]);
        self.add_interval(face, top_axis, ij[2], ij[1], ij[3]);
        self.add_interval(face, left_axis, ij[3], ij[2], ij[0]);
    }

    /// Adds an interval `(ij0, ij1)` to the tracker. Always increments the
    /// count at `ij0` and decrements at `ij1`.
    ///
    /// REQUIRES: `ij0 != ij1`, dimension == 2.
    pub fn add_interval(&mut self, face: i32, axis: i32, ijcoord: i64, ij0: i64, ij1: i64) {
        debug_assert!(ij0 != ij1);
        debug_assert!(self.dimension == Dimension::Polygon);

        let mut face = face;
        let mut axis = axis;
        let mut ijcoord = ijcoord;
        let mut ij0 = ij0;
        let mut ij1 = ij1;

        // Wrap at maximum coordinate to adjacent face.
        if ijcoord == LIMIT_IJ {
            ijcoord = 0;
            face = adjacent_face(face, axis);
            if axis == 1 {
                ij0 = LIMIT_IJ - ij0;
                ij1 = LIMIT_IJ - ij1;
            }
            axis = 1 - axis;
        }

        self.increment_point(axis, ij_key(face, ijcoord, ij0));
        self.decrement_point(axis, ij_key(face, ijcoord, ij1));
    }

    /// Adds a point crossing to the tracker (for polylines).
    pub fn add_point(&mut self, face: i32, axis: i32, ijcoord: i64, ij: i64) {
        let mut face = face;
        let mut axis = axis;
        let mut ijcoord = ijcoord;
        let mut ij = ij;
        let mut flip = false;

        if ijcoord == LIMIT_IJ {
            ijcoord = 0;
            face = adjacent_face(face, axis);
            if axis == 1 {
                flip = true;
                ij = LIMIT_IJ - ij;
            }
            axis = 1 - axis;
        }

        if flip {
            self.del_point(face, axis, ijcoord, ij);
            return;
        }

        self.increment_point(axis, ij_key(face, ijcoord, ij));
    }

    /// Removes a point crossing from the tracker (for polylines).
    pub fn del_point(&mut self, face: i32, axis: i32, ijcoord: i64, ij: i64) {
        let mut face = face;
        let mut axis = axis;
        let mut ijcoord = ijcoord;
        let mut ij = ij;
        let mut flip = false;

        if ijcoord == LIMIT_IJ {
            ijcoord = 0;
            face = adjacent_face(face, axis);
            if axis == 1 {
                flip = true;
                ij = LIMIT_IJ - ij;
            }
            axis = 1 - axis;
        }

        if flip {
            self.add_point(face, axis, ijcoord, ij);
            return;
        }

        self.decrement_point(axis, ij_key(face, ijcoord, ij));
    }

    fn increment_point(&mut self, axis: i32, key: u64) {
        let map = &mut self.points[axis.unsigned_abs() as usize];
        let entry = map.entry(key).or_insert(0);
        *entry += 1;
        if *entry == 0 {
            map.remove(&key);
        }
    }

    fn decrement_point(&mut self, axis: i32, key: u64) {
        let map = &mut self.points[axis.unsigned_abs() as usize];
        let entry = map.entry(key).or_insert(0);
        *entry -= 1;
        if *entry == 0 {
            map.remove(&key);
        }
    }
}

/// Creates a 64-bit key from face, i, j coordinates.
fn ij_key(face: i32, i: i64, j: i64) -> u64 {
    debug_assert!(face < 6);
    debug_assert!(i <= LIMIT_IJ);
    debug_assert!(j <= LIMIT_IJ);

    let mut ans = 0u64;
    ans |= u64::from(face.unsigned_abs()) << 60;
    ans |= i.cast_unsigned() << 30;
    ans |= j.cast_unsigned();
    ans
}

/// Converts UV to IJ, rounding down.
#[expect(clippy::cast_possible_truncation, reason = "IJ values fit in i64")]
fn uv_to_ij_floor(uv: f64) -> i64 {
    (f64::from(coords::LIMIT_IJ) * coords::uv_to_st(uv)).floor() as i64
}

/// Converts UV to IJ, rounding up.
#[expect(clippy::cast_possible_truncation, reason = "IJ values fit in i64")]
fn uv_to_ij_ceil(uv: f64) -> i64 {
    (f64::from(coords::LIMIT_IJ) * coords::uv_to_st(uv)).ceil() as i64
}

/// Converts UV to IJ, rounding ties away from zero.
#[expect(clippy::cast_possible_truncation, reason = "IJ values fit in i64")]
fn uv_to_ij_round(uv: f64) -> i64 {
    (f64::from(coords::LIMIT_IJ) * coords::uv_to_st(uv)).round() as i64
}

/// Returns the axis along which a given cell boundary is constant.
fn constant_boundary_axis(face: i32, boundary: CellEdge) -> i32 {
    let axis = match boundary {
        CellEdge::Bottom | CellEdge::Top => 1,
        CellEdge::Right | CellEdge::Left => 0,
    };
    // Odd faces have axes flipped.
    if face % 2 == 0 { axis } else { 1 - axis }
}

/// Returns the face adjacent to the given face across the given axis.
fn adjacent_face(face: i32, axis: i32) -> i32 {
    (face + if axis == 1 { 2 } else { 1 }) % 6
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::CellId;

    fn crossing(
        boundary: CellEdge,
        crossing_type: CrossingType,
        coord: f64,
        intercept: f64,
        edge_index: usize,
    ) -> Crossing {
        Crossing {
            boundary,
            crossing_type,
            coord,
            intercept,
            edge_index,
        }
    }

    #[test]
    fn test_point_works() {
        let mut tracker = ShapeTracker::new(Dimension::Point, 3);
        assert!(!tracker.finished());

        tracker.mark_chain(0);
        tracker.mark_chain(1);
        tracker.mark_chain(2);
        assert!(tracker.finished());

        // Idempotent.
        tracker.mark_chain(0);
        tracker.mark_chain(1);
        tracker.mark_chain(2);
        assert!(tracker.finished());
    }

    #[test]
    fn test_polyline_works() {
        let mut tracker = ShapeTracker::new(Dimension::Polyline, 3);
        assert!(!tracker.finished());

        for i in 0..3 {
            tracker.mark_chain(i);
        }

        // Turn on a couple crossings.
        tracker.add_point(0, 1, 1, 11);
        tracker.del_point(3, 0, 1337, 13);
        tracker.add_point(4, 1, 3141, 17);
        assert!(!tracker.finished());

        // Turn them off one by one.
        tracker.del_point(0, 1, 1, 11);
        assert!(!tracker.finished());

        tracker.add_point(3, 0, 1337, 13);
        assert!(!tracker.finished());

        tracker.del_point(4, 1, 3141, 17);
        assert!(tracker.finished());
    }

    #[test]
    fn test_polygon_works() {
        let mut tracker = ShapeTracker::new(Dimension::Polygon, 5);
        assert!(!tracker.finished());

        for i in 0..5 {
            tracker.mark_chain(i);
        }

        // Add a couple of intervals.
        tracker.add_interval(0, 1, 1000, 11, 22);
        tracker.add_interval(3, 0, 1337, 23, 36);
        tracker.add_interval(4, 1, 3141, 55, 72);
        assert!(!tracker.finished());

        // Subtract them off one by one.
        tracker.add_interval(0, 1, 1000, 22, 18);
        tracker.add_interval(0, 1, 1000, 18, 11);
        assert!(!tracker.finished());

        tracker.add_interval(3, 0, 1337, 25, 24);
        tracker.add_interval(3, 0, 1337, 24, 23);
        tracker.add_interval(3, 0, 1337, 30, 25);
        tracker.add_interval(3, 0, 1337, 36, 30);
        assert!(!tracker.finished());

        tracker.add_interval(4, 1, 3141, 72, 55);
        assert!(tracker.finished());
    }

    #[test]
    fn test_face_axes_cancel() {
        let mut tracker = ShapeTracker::new(Dimension::Polygon, 1);
        tracker.mark_chain(0);
        assert!(tracker.finished());

        let cell0 = Cell::from_cell_id(CellId::from_face(0));
        let cell1 = Cell::from_cell_id(CellId::from_face(1));

        // Get IJ coords for boundaries.
        let bound0 = cell0.bound_uv();
        let ij0_bottom = uv_to_ij_round(bound0.y.lo);
        let ij0_top = uv_to_ij_round(bound0.y.hi);
        let ij0_right = uv_to_ij_round(bound0.x.hi);
        let _ij0_left = uv_to_ij_round(bound0.x.lo); // not used, but for clarity

        let bound1 = cell1.bound_uv();
        let ij1_bottom = uv_to_ij_round(bound1.y.lo);
        let ij1_top = uv_to_ij_round(bound1.y.hi);

        tracker.add_interval(0, 0, LIMIT_IJ, ij0_bottom, ij0_top);
        assert!(!tracker.finished());

        tracker.add_interval(0, 1, LIMIT_IJ, ij0_right, ij0_bottom);
        assert!(!tracker.finished());

        tracker.add_interval(1, 1, 0, ij1_top, ij1_bottom);
        assert!(!tracker.finished());

        tracker.add_interval(2, 0, 0, ij1_top, ij1_bottom);
        assert!(tracker.finished());
    }

    #[test]
    fn test_face_cells_close() {
        // Adding all 6 face cells should sum to zero.
        let faces = [0, 1, 2, 3, 4, 5];
        // Test a couple of orderings.
        for order in [[0, 1, 2, 3, 4, 5], [5, 4, 3, 2, 1, 0], [2, 0, 4, 1, 5, 3]] {
            let mut tracker = ShapeTracker::new(Dimension::Polygon, 1);
            assert!(!tracker.finished());
            tracker.mark_chain(0);
            assert!(tracker.finished());

            let mut cnt = 0;
            for &face in &order {
                let cell = Cell::from_cell_id(CellId::from_face(faces[face]));
                tracker.add_cell_boundary(cell);
                cnt += 1;
                assert_eq!(cnt == 6, tracker.finished());
            }
        }
    }

    #[test]
    fn test_face_children_close() {
        let mut tracker = ShapeTracker::new(Dimension::Polygon, 1);
        assert!(!tracker.finished());
        tracker.mark_chain(0);
        assert!(tracker.finished());

        let mut total_cells = 0;
        let total_expected = 6 * (4 + 16); // level 1 + level 2 children
        for face in 0..6 {
            let face_id = CellId::from_face(face);
            for level in 1..=2u8 {
                let beg = face_id.child_begin_at_level(level);
                let end = face_id.child_end_at_level(level);
                let mut id = beg;
                while id != end {
                    let cell = Cell::from_cell_id(id);
                    tracker.add_cell_boundary(cell);
                    total_cells += 1;
                    assert_eq!(
                        total_cells == total_expected,
                        tracker.finished(),
                        "total_cells={total_cells}, expected={total_expected}"
                    );
                    id = id.next();
                }
            }
        }
    }

    #[test]
    fn test_face_corners_close() {
        let mut tracker = ShapeTracker::new(Dimension::Polygon, 1);
        assert!(!tracker.finished());
        tracker.mark_chain(0);
        assert!(tracker.finished());

        let crossings = [
            crossing(CellEdge::Bottom, CrossingType::Incoming, -1.0, -0.75, 0),
            crossing(CellEdge::Bottom, CrossingType::Outgoing, -1.0, 0.75, 0),
            crossing(CellEdge::Right, CrossingType::Incoming, 1.0, -0.75, 0),
            crossing(CellEdge::Right, CrossingType::Outgoing, 1.0, 0.75, 0),
            crossing(CellEdge::Top, CrossingType::Incoming, 1.0, 0.75, 0),
            crossing(CellEdge::Top, CrossingType::Outgoing, 1.0, -0.75, 0),
            crossing(CellEdge::Left, CrossingType::Incoming, -1.0, 0.75, 0),
            crossing(CellEdge::Left, CrossingType::Outgoing, -1.0, -0.75, 0),
        ];

        for face in 0..6 {
            let cell = Cell::from_cell_id(CellId::from_face(face));
            tracker.process_crossings(cell, &crossings);
            assert_eq!(face == 5, tracker.finished());
        }
    }

    #[test]
    fn test_small_intervals_work() {
        for dim in [Dimension::Polyline, Dimension::Polygon] {
            let mut tracker = ShapeTracker::new(dim, 1);
            assert!(!tracker.finished());
            tracker.mark_chain(0);
            assert!(tracker.finished());

            let crossings = [
                crossing(
                    CellEdge::Bottom,
                    CrossingType::Incoming,
                    -1.0,
                    -(1.0 - 1e-15),
                    0,
                ),
                crossing(
                    CellEdge::Bottom,
                    CrossingType::Outgoing,
                    -1.0,
                    1.0 - 1e-15,
                    0,
                ),
                crossing(
                    CellEdge::Right,
                    CrossingType::Incoming,
                    1.0,
                    -(1.0 - 1e-15),
                    0,
                ),
                crossing(CellEdge::Right, CrossingType::Outgoing, 1.0, 1.0 - 1e-15, 0),
                crossing(CellEdge::Top, CrossingType::Incoming, 1.0, 1.0 - 1e-15, 0),
                crossing(
                    CellEdge::Top,
                    CrossingType::Outgoing,
                    1.0,
                    -(1.0 - 1e-15),
                    0,
                ),
                crossing(CellEdge::Left, CrossingType::Incoming, -1.0, 1.0 - 1e-15, 0),
                crossing(
                    CellEdge::Left,
                    CrossingType::Outgoing,
                    -1.0,
                    -(1.0 - 1e-15),
                    0,
                ),
            ];

            for face in 0..6 {
                let cell = Cell::from_cell_id(CellId::from_face(face));
                tracker.process_crossings(cell, &crossings);
                assert_eq!(face == 5, tracker.finished());
            }
        }
    }
}
