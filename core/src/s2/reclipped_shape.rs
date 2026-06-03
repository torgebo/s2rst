// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - Java: google/s2-geometry-library-java

//! A shape re-clipped to a new (smaller) cell.
//!
//! Ported from Java `S2ReclippedShape`. Stores the edges and crossings of a
//! shape after re-clipping from a parent cell to a descendant cell using
//! [`RobustCellClipper`].

use crate::s2::Point;
use crate::s2::cell::Cell;
use crate::s2::cell_id::CellId;
use crate::s2::robust_cell_clipper::{Crossing, RobustCellClipper};
use crate::s2::shape::Dimension;

/// An edge that was re-clipped into a cell.
#[derive(Clone, Debug)]
pub struct ReclippedEdge {
    /// First vertex.
    pub v0: Point,
    /// Second vertex.
    pub v1: Point,
    /// Whether v0 was inside the cell.
    pub v0_contained: bool,
    /// Whether v1 was inside the cell.
    pub v1_contained: bool,
}

/// A shape that has been re-clipped to a (potentially smaller) cell.
///
/// Re-clipping is the process of taking a clipped shape from a parent cell
/// and further clipping its edges to a descendant cell. This is used in
/// join operations where one index has larger cells than another.
///
/// The shape ID is tracked to avoid re-processing the same shape when called
/// repeatedly in a loop. Call [`reset`](Self::reset) to force reprocessing.
#[derive(Debug)]
pub struct ReclippedShape {
    cell_id: CellId,
    shape_id: i32,
    dimension: Option<Dimension>,
    contains_center: bool,
    edges: Vec<ReclippedEdge>,
    crossings: Vec<Crossing>,
}

impl Default for ReclippedShape {
    fn default() -> Self {
        Self::new()
    }
}

impl ReclippedShape {
    /// Creates a new empty reclipped shape.
    pub fn new() -> Self {
        Self {
            cell_id: CellId::sentinel(),
            shape_id: -1,
            dimension: None,
            contains_center: false,
            edges: Vec::new(),
            crossings: Vec::new(),
        }
    }

    /// Re-clips the given edges to the cell configured in `clipper`.
    ///
    /// If `shape_id` matches the previously processed shape, processing is
    /// skipped (returns `false`). Call [`reset`](Self::reset) to force.
    ///
    /// # Arguments
    ///
    /// * `clipper` — Must have been initialized with `start_cell`.
    /// * `shape_id` — The shape being clipped.
    /// * `dimension` — The geometric dimension of the shape.
    /// * `contains_center` — Whether the shape contains the center of the
    ///   *parent* cell. For polygons, this may be recomputed for the
    ///   smaller cell.
    /// * `edges` — Iterator of `(v0, v1)` edge endpoints.
    /// * `save_crossings` — Whether to save boundary crossings.
    ///
    /// Returns `true` if edges were processed, `false` if skipped.
    pub fn init(
        &mut self,
        clipper: &mut RobustCellClipper,
        shape_id: i32,
        dimension: Dimension,
        contains_center: bool,
        edges: impl Iterator<Item = (Point, Point)>,
        save_crossings: bool,
    ) -> bool {
        if self.shape_id == shape_id && shape_id >= 0 {
            return false;
        }

        self.shape_id = shape_id;
        self.dimension = Some(dimension);
        self.cell_id = clipper.cell().map_or(CellId::sentinel(), Cell::id);
        self.contains_center = contains_center;
        self.edges.clear();
        self.crossings.clear();

        clipper.reset();
        for (v0, v1) in edges {
            let result = clipper.clip_edge(v0, v1, false);
            if result.is_hit() {
                self.edges.push(ReclippedEdge {
                    v0,
                    v1,
                    v0_contained: result.v0_inside(),
                    v1_contained: result.v1_inside(),
                });
            }
        }

        if save_crossings && clipper.options().enable_crossings {
            self.crossings.extend_from_slice(clipper.get_crossings());
        }

        true
    }

    /// Resets the shape ID, forcing the next [`init`](Self::init) to process.
    pub fn reset(&mut self) {
        self.shape_id = -1;
    }

    /// Returns the cell ID this shape was clipped to.
    pub fn cell_id(&self) -> CellId {
        self.cell_id
    }

    /// Returns the shape ID.
    pub fn shape_id(&self) -> i32 {
        self.shape_id
    }

    /// Returns the dimension of the shape, or `None` if not initialized.
    pub fn dimension(&self) -> Option<Dimension> {
        if self.shape_id >= 0 {
            self.dimension
        } else {
            None
        }
    }

    /// Whether the reclipped shape contains the cell center.
    pub fn contains_center(&self) -> bool {
        self.contains_center
    }

    /// Returns the edges that intersected the cell.
    pub fn edges(&self) -> &[ReclippedEdge] {
        &self.edges
    }

    /// Returns the crossings (if enabled).
    pub fn crossings(&self) -> &[Crossing] {
        &self.crossings
    }

    /// Returns the number of edges.
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Tests whether this reclipped shape contains the given point.
    ///
    /// `center` must be the center of the cell this shape was clipped to
    /// (i.e., `cell_id().to_point()`). Only points within the clipped cell
    /// may be tested — points outside may return incorrect results.
    ///
    /// For dimension < 2, only exact vertex matches count as contained.
    pub fn contains(&self, center: Point, point: Point) -> bool {
        debug_assert!(self.shape_id >= 0);

        // Points and polylines don't contain anything except at vertices.
        if self.dimension != Some(Dimension::Polygon) {
            return self.edges.iter().any(|e| e.v0 == point || e.v1 == point);
        }

        // Test containment by drawing a line segment from the cell center to
        // the given point and counting edge crossings.
        let mut crosser = crate::s2::edge_crosser::EdgeCrosser::new(center, point);

        let mut inside = self.contains_center;
        for edge in &self.edges {
            if crosser.edge_or_vertex_crossing(edge.v0, edge.v1) {
                inside = !inside;
            }
        }
        inside
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::{Cell, CellId, LatLng};

    #[test]
    fn test_empty_reclipped_shape() {
        let shape = ReclippedShape::new();
        assert_eq!(shape.shape_id(), -1);
        assert_eq!(shape.dimension(), None);
        assert!(shape.edges().is_empty());
        assert!(shape.crossings().is_empty());
    }

    #[test]
    fn test_reclip_edges() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        let edges = vec![
            (
                LatLng::from_degrees(10.0, 10.0).to_point(),
                LatLng::from_degrees(20.0, 20.0).to_point(),
            ),
            (
                LatLng::from_degrees(10.0, -170.0).to_point(),
                LatLng::from_degrees(20.0, -170.0).to_point(),
            ),
        ];

        let mut shape = ReclippedShape::new();
        let processed = shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            false,
            edges.into_iter(),
            true,
        );
        assert!(processed);
        assert_eq!(shape.shape_id(), 0);
        assert_eq!(shape.dimension(), Some(Dimension::Polygon));
        // First edge is on face 0, second is not.
        assert_eq!(shape.num_edges(), 1);
    }

    #[test]
    fn test_skip_same_shape_id() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        let edges = vec![(
            LatLng::from_degrees(10.0, 10.0).to_point(),
            LatLng::from_degrees(20.0, 20.0).to_point(),
        )];

        let mut shape = ReclippedShape::new();
        shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            false,
            edges.clone().into_iter(),
            false,
        );

        clipper.start_cell(cell);
        let processed = shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            false,
            edges.into_iter(),
            false,
        );
        assert!(!processed); // Skipped because shape_id matches.
    }

    #[test]
    fn test_reset_forces_reprocessing() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        let edges = vec![(
            LatLng::from_degrees(10.0, 10.0).to_point(),
            LatLng::from_degrees(20.0, 20.0).to_point(),
        )];

        let mut shape = ReclippedShape::new();
        shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            false,
            edges.clone().into_iter(),
            false,
        );
        shape.reset();

        clipper.start_cell(cell);
        let processed = shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            false,
            edges.into_iter(),
            false,
        );
        assert!(processed);
    }

    #[test]
    fn test_reclip_with_crossings() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        // One vertex inside face 0, one outside (should cross boundary).
        let edges = vec![(
            LatLng::from_degrees(10.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 80.0).to_point(),
        )];

        let mut shape = ReclippedShape::new();
        shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            false,
            edges.into_iter(),
            true,
        );
        assert_eq!(shape.num_edges(), 1);
        assert!(shape.edges()[0].v0_contained);
        assert!(!shape.edges()[0].v1_contained);
        // Should have at least one crossing recorded.
        assert!(!shape.crossings().is_empty());
    }

    #[test]
    fn test_contains_interior_point() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let center = cell.center();

        // Small polygon ring entirely inside face 0.
        let edges = vec![
            (
                LatLng::from_degrees(-5.0, -5.0).to_point(),
                LatLng::from_degrees(-5.0, 5.0).to_point(),
            ),
            (
                LatLng::from_degrees(-5.0, 5.0).to_point(),
                LatLng::from_degrees(5.0, 5.0).to_point(),
            ),
            (
                LatLng::from_degrees(5.0, 5.0).to_point(),
                LatLng::from_degrees(5.0, -5.0).to_point(),
            ),
            (
                LatLng::from_degrees(5.0, -5.0).to_point(),
                LatLng::from_degrees(-5.0, -5.0).to_point(),
            ),
        ];

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        let mut shape = ReclippedShape::new();
        // The ring contains the cell center.
        shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            true,
            edges.into_iter(),
            false,
        );

        // Center of face 0 at (0°,0°) is inside the ring.
        assert!(shape.contains(center, center));

        // A point slightly off-center but still inside the ring.
        let near_center = LatLng::from_degrees(1.0, 1.0).to_point();
        assert!(shape.contains(center, near_center));

        // A point far outside the ring should not be contained.
        let outside = LatLng::from_degrees(40.0, 40.0).to_point();
        assert!(!shape.contains(center, outside));
    }

    #[test]
    fn test_contains_vertex_for_polyline() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let center = cell.center();

        let a = LatLng::from_degrees(10.0, 10.0).to_point();
        let b = LatLng::from_degrees(20.0, 20.0).to_point();
        let edges = vec![(a, b)];

        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        let mut shape = ReclippedShape::new();
        shape.init(
            &mut clipper,
            0,
            Dimension::Polyline,
            false,
            edges.into_iter(),
            false,
        );

        // Polyline contains its vertices.
        assert!(shape.contains(center, a));
        assert!(shape.contains(center, b));
        // But not arbitrary points.
        assert!(!shape.contains(center, center));
    }

    #[test]
    fn test_reclip_no_crossings_when_not_saved() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let mut clipper = RobustCellClipper::new();
        clipper.start_cell(cell);

        let edges = vec![(
            LatLng::from_degrees(10.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 80.0).to_point(),
        )];

        let mut shape = ReclippedShape::new();
        shape.init(
            &mut clipper,
            0,
            Dimension::Polygon,
            false,
            edges.into_iter(),
            false,
        );
        assert!(shape.crossings().is_empty());
    }
}
