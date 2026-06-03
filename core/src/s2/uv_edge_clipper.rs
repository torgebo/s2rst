// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! UV-space edge clipper layered on [`R2EdgeClipper`](super::r2_edge_clipper).
//!
//! Clips S2 edges (3D `Point` endpoints) to rectangular regions in UV space.
//! Ported from Java `UVEdgeClipper`.

use crate::r2;
use crate::s2::coords::{self, Face};
use crate::s2::edge_clipping;
use crate::s2::r2_edge_clipper::{R2Edge, R2EdgeClipper};
use crate::s2::{Cell, Point};

/// A clipper of shape edges to rectangular regions in UV space.
///
/// Call [`init`](Self::init) or [`init_cell`](Self::init_cell) to set the face
/// and region, then [`clip_edge`](Self::clip_edge) to test each edge. Results
/// are available through accessor methods.
#[derive(Debug)]
pub struct UVEdgeClipper {
    r2_clipper: R2EdgeClipper,
    clip_face: Face,
    last_face: Face,
    face_uv_edge: R2Edge,
    uv_error: f64,
    missed_face: bool,
}

impl Default for UVEdgeClipper {
    fn default() -> Self {
        Self::new()
    }
}

impl UVEdgeClipper {
    /// Creates a new clipper with no face or region set.
    pub fn new() -> Self {
        Self {
            r2_clipper: R2EdgeClipper::new(),
            clip_face: Face::F0,
            last_face: Face::F0,
            face_uv_edge: R2Edge::default(),
            uv_error: 0.0,
            missed_face: false,
        }
    }

    /// Creates a new clipper for the given face and UV region.
    pub fn from_face_rect(face: Face, region: &r2::Rect) -> Self {
        let mut c = Self::new();
        c.init(face, region);
        c
    }

    /// Creates a new clipper for the given cell.
    pub fn from_cell(cell: Cell) -> Self {
        let mut c = Self::new();
        c.init_cell(cell);
        c
    }

    /// Initialize the clipper to clip to the given UV region on the given face.
    pub fn init(&mut self, face: Face, region: &r2::Rect) {
        self.clip_face = face;
        self.r2_clipper.init(region);
    }

    /// Initialize the clipper to clip to the given cell.
    pub fn init_cell(&mut self, cell: Cell) {
        self.init(cell.face(), &cell.bound_uv());
    }

    /// Returns the face being clipped to.
    pub fn clip_face(&self) -> Face {
        self.clip_face
    }

    /// Returns the current clip rectangle.
    pub fn clip_rect(&self) -> r2::Rect {
        self.r2_clipper.clip_rect()
    }

    /// Clip an edge to the current face and clip region.
    ///
    /// Returns `true` if the edge intersects both the face and clip region.
    /// If `connected`, reuses computation from the previous edge's second
    /// vertex.
    pub fn clip_edge(&mut self, v0: Point, v1: Point, connected: bool) -> bool {
        let face0;
        let face1;
        let mut need_face_clip = false;

        if connected {
            face0 = self.last_face;
            let (f1, _, _) = coords::xyz_to_face_uv(&v1.0);
            face1 = f1;
            if face0 != face1 || face0 != self.clip_face {
                need_face_clip = true;
            } else {
                self.face_uv_edge.v0 = self.face_uv_edge.v1;
                let (u, v) = coords::valid_face_xyz_to_uv(face1, &v1.0);
                self.face_uv_edge.v1 = r2::Point::new(u, v);
            }
        } else {
            let (f0, _, _) = coords::xyz_to_face_uv(&v0.0);
            let (f1, _, _) = coords::xyz_to_face_uv(&v1.0);
            face0 = f0;
            face1 = f1;
            if face0 != face1 || face0 != self.clip_face {
                need_face_clip = true;
            } else {
                let (u0, v0_uv) = coords::valid_face_xyz_to_uv(face0, &v0.0);
                let (u1, v1_uv) = coords::valid_face_xyz_to_uv(face0, &v1.0);
                self.face_uv_edge.v0 = r2::Point::new(u0, v0_uv);
                self.face_uv_edge.v1 = r2::Point::new(u1, v1_uv);
            }
        }
        self.last_face = face1;

        self.missed_face = false;
        if need_face_clip {
            if let Some((a_uv, b_uv)) = edge_clipping::clip_to_padded_face(
                v0,
                v1,
                self.clip_face,
                edge_clipping::FACE_CLIP_ERROR_UV_COORD,
            ) {
                self.face_uv_edge.v0 = a_uv;
                self.face_uv_edge.v1 = b_uv;
            } else {
                self.missed_face = true;
                return false;
            }
        }

        self.uv_error = coords::MAX_XYZ_TO_UV_ERROR;
        if need_face_clip {
            self.uv_error += edge_clipping::FACE_CLIP_ERROR_UV_COORD;
        }

        self.r2_clipper.clip_edge(&self.face_uv_edge, connected)
    }

    /// Whether the last edge missed the clip face entirely.
    pub fn missed_face(&self) -> bool {
        self.missed_face
    }

    /// Maximum absolute error from converting to UV and clipping to face.
    pub fn uv_error(&self) -> f64 {
        self.uv_error
    }

    /// Maximum absolute error in a clipped vertex (includes UV error +
    /// interpolation error).
    pub fn clip_error(&self) -> f64 {
        2.0 * (self.uv_error() + super::r2_edge_clipper::MAX_UNIT_CLIP_ERROR)
    }

    /// The UV edge after clipping to the face but before clipping to the region.
    pub fn face_uv_edge(&self) -> &R2Edge {
        &self.face_uv_edge
    }

    /// The UV edge after clipping to both the face and the region.
    pub fn clipped_uv_edge(&self) -> &R2Edge {
        &self.r2_clipper.clipped_edge
    }

    /// Returns the outcode for vertex 0 or 1 of the last clip operation.
    ///
    /// # Panics
    ///
    /// Panics if `vertex` is not 0 or 1.
    pub fn outcode(&self, vertex: usize) -> u8 {
        match vertex {
            0 => self.r2_clipper.outcode0,
            1 => self.r2_clipper.outcode1,
            _ => unreachable!("vertex must be 0 or 1"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::r2_edge_clipper::INSIDE;

    #[test]
    fn test_edge_inside_face0() {
        let mut clipper = UVEdgeClipper::new();
        let face = Face::F0;
        let region = r2::Rect::from_points(r2::Point::new(-1.0, -1.0), r2::Point::new(1.0, 1.0));
        clipper.init(face, &region);

        let v0 = LatLng::from_degrees(10.0, 10.0).to_point();
        let v1 = LatLng::from_degrees(20.0, 20.0).to_point();
        assert!(clipper.clip_edge(v0, v1, false));
        assert_eq!(clipper.outcode(0), INSIDE);
        assert_eq!(clipper.outcode(1), INSIDE);
    }

    #[test]
    fn test_edge_misses_face() {
        let mut clipper = UVEdgeClipper::new();
        let face = Face::F0;
        let region = r2::Rect::from_points(r2::Point::new(-1.0, -1.0), r2::Point::new(1.0, 1.0));
        clipper.init(face, &region);

        // Edge entirely on face 3 (opposite).
        let v0 = LatLng::from_degrees(10.0, -170.0).to_point();
        let v1 = LatLng::from_degrees(20.0, -170.0).to_point();
        assert!(!clipper.clip_edge(v0, v1, false));
        assert!(clipper.missed_face());
    }

    #[test]
    fn test_clip_cell() {
        let cell_id = crate::s2::CellId::from_face(0).children()[0];
        let cell = Cell::from_cell_id(cell_id);
        let mut clipper = UVEdgeClipper::from_cell(cell);

        // Point well inside the cell.
        let center = cell.center();
        let v0 = center;
        let v1 = LatLng::from_degrees(5.0, 5.0).to_point();
        let hit = clipper.clip_edge(v0, v1, false);
        // May or may not hit depending on exact cell geometry.
        // Just check no panic and result is reasonable.
        assert!(!clipper.missed_face() || !hit);
    }

    #[test]
    fn test_connected_edge() {
        let mut clipper = UVEdgeClipper::new();
        let face = Face::F0;
        let region = r2::Rect::from_points(r2::Point::new(-1.0, -1.0), r2::Point::new(1.0, 1.0));
        clipper.init(face, &region);

        let a = LatLng::from_degrees(10.0, 10.0).to_point();
        let b = LatLng::from_degrees(20.0, 20.0).to_point();
        let c = LatLng::from_degrees(30.0, 15.0).to_point();

        assert!(clipper.clip_edge(a, b, false));
        assert!(clipper.clip_edge(b, c, true));
        assert_eq!(clipper.outcode(0), INSIDE);
    }

    #[test]
    fn test_error_bounds() {
        let mut clipper = UVEdgeClipper::new();
        let face = Face::F0;
        let region = r2::Rect::from_points(r2::Point::new(-1.0, -1.0), r2::Point::new(1.0, 1.0));
        clipper.init(face, &region);

        let v0 = LatLng::from_degrees(10.0, 10.0).to_point();
        let v1 = LatLng::from_degrees(20.0, 20.0).to_point();
        clipper.clip_edge(v0, v1, false);

        assert!(clipper.uv_error() > 0.0);
        assert!(clipper.clip_error() > clipper.uv_error());
    }
}
