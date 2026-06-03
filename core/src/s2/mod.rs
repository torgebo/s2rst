// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library — a derivative work of the
// upstream Apache-2.0 implementations (Copyright Google Inc.):
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java
// See LICENSE.

//! Spherical geometry on the unit sphere (S²).
//!
//! This is the main module of the library. It provides types and algorithms
//! for representing, indexing, and manipulating geometry on the surface of a
//! unit sphere. All edges are geodesics (great-circle arcs), so operations
//! work uniformly across the entire sphere with no singularities at the poles
//! or discontinuities at the antimeridian.
//!
//! # Geometry types
//!
//! The core geometry types are re-exported at the top of this module:
//!
//! | Type | Description |
//! |------|-------------|
//! | [`Point`] | A point on the unit sphere (unit-length 3D vector). |
//! | [`LatLng`] | A point expressed as a latitude-longitude pair. |
//! | [`CellId`] | A 64-bit identifier for a cell in the S2 hierarchy. |
//! | [`Cell`] | A concrete cell with precomputed vertices and bounds. |
//! | [`CellUnion`] | A sorted, normalized collection of `CellId`s approximating a region. |
//! | [`Cap`] | A spherical cap (disc-shaped region defined by center and radius). |
//! | [`Rect`] | A closed latitude-longitude rectangle. |
//! | [`Loop`] | A simple spherical polygon (single closed boundary). |
//! | [`Polygon`] | A multi-loop polygon, possibly with holes. |
//! | [`Region`] | A trait for any region that supports bounding and containment tests. |
//!
//! Additional geometry types include [`polyline::Polyline`] (open paths),
//! [`lax_polygon::LaxPolygon`] and [`lax_polyline::LaxPolyline`] (relaxed
//! validity), and [`edge_vector_shape::EdgeVectorShape`] (arbitrary edge
//! collections).
//!
//! # The S2 cell hierarchy
//!
//! The sphere is decomposed into a hierarchy of cells by projecting the six
//! faces of a cube onto the sphere. Each face is recursively subdivided into
//! four children, producing 31 levels of cells (level 0 = face cells, level
//! 30 = leaf cells roughly 1 cm across on Earth). Cells at each level tile the
//! sphere without gaps or overlaps.
//!
//! Cell identifiers ([`CellId`]) are 64-bit integers that encode both the face
//! and the position along a space-filling Hilbert curve. Cells that are close
//! in `CellId` order are also spatially close, which makes range scans over
//! sorted cell IDs efficient for spatial queries. The [`coords`] module
//! documents the coordinate systems used internally: `(face, i, j)`,
//! `(face, s, t)`, `(face, u, v)`, and `(x, y, z)`.
//!
//! # The Shape interface and `ShapeIndex`
//!
//! All geometry can be viewed through the [`shape::Shape`] trait, which
//! presents geometry as a collection of edges optionally defining an interior.
//! A [`shape_index::ShapeIndex`] indexes one or more shapes for fast spatial
//! lookups — given a point or region, it can quickly determine which shapes
//! contain it or are nearby.
//!
//! # Spatial queries
//!
//! - [`closest_edge_query`] — find the nearest edges or test distance
//!   thresholds between geometries.
//! - [`contains_point_query`] — determine which shapes contain a given point,
//!   with configurable boundary semantics (open, semi-open, closed).
//! - [`crossing_edge_query`] — find edges in an index that cross a given edge.
//! - [`convex_hull_query`] — compute the spherical convex hull.
//! - [`hausdorff_distance_query`] — compute the Hausdorff distance between
//!   edge chains.
//!
//! # Constructive operations
//!
//! - [`boolean_operation`] — compute union, intersection, difference, or
//!   symmetric difference of regions.
//! - [`winding_operation`] — N-way boolean operations using winding numbers.
//! - [`buffer_operation`] — expand or contract geometry by a fixed radius.
//! - [`builder`] — assemble edges into valid geometry with vertex snapping and
//!   topological repair.
//!
//! # Covering and indexing
//!
//! - [`region_coverer`] — approximate any [`Region`] as a [`CellUnion`] for
//!   use in spatial database indexes.
//! - [`region_term_indexer`] — generate index/query terms for point or region
//!   containment queries in inverted indexes.

pub mod boolean_operation;
pub mod buffer_operation;
pub mod builder;
mod cap;
mod cell;
mod cell_id;
pub mod cell_index;
pub mod cell_iterator_join;
pub mod cell_range_iterator;
mod cell_union;
pub mod centroids;
pub mod chain_interpolation_query;
/// Find closest cells in a `CellIndex` to a given target.
pub mod closest_cell_query;
pub mod closest_edge_query;
/// Find closest points in an `S2PointIndex` to a given target.
pub mod closest_point_query;
pub mod contains_point_query;
pub mod contains_vertex_query;
pub mod convex_hull_query;
pub mod coords;
pub mod crossing_edge_query;
pub mod density_cluster_query;
pub mod density_tree;
/// Shared supertrait for all distance query targets.
pub mod distance_target;
pub mod earth;
pub mod edge_clipping;
pub mod edge_crosser;
pub mod edge_crossings;
pub mod edge_distances;
pub mod edge_query;
pub mod edge_tessellator;
pub mod edge_vector_shape;
pub mod encoded_s2cell_id_vector;
pub mod encoded_s2point_vector;
pub mod encoded_s2shape_index;
pub mod encoded_string_vector;
pub mod encoded_uint_vector;
pub mod encoding;
pub mod fractal;
pub mod furthest_edge_query;
pub mod hausdorff_distance_query;
pub mod incident_edge_tracker;
mod latlng;
pub mod latlng_rect_bounder;
pub mod lax_loop;
pub mod lax_polygon;
pub mod lax_polyline;
pub mod loop_measures;
/// An index of points on the sphere for nearest-neighbor queries.
/// Tracks and limits memory usage of S2 operations.
pub mod memory_tracker;
pub mod metric;
pub mod padded_cell;
mod point;
pub mod point_compression;
/// Spatial index for S2 points with associated data.
pub mod point_index;
pub mod point_measures;
pub mod point_region;
pub mod point_vector;
mod polygon;
pub mod polyline;
pub mod polyline_alignment;
pub mod polyline_measures;
pub mod polyline_simplifier;
pub mod predicates;
pub mod projections;
pub mod r2_edge_clipper;
pub mod reclipped_shape;
mod rect;
mod region;
pub mod region_coverer;
pub mod region_intersection;
pub mod region_sharder;
pub mod region_term_indexer;
pub mod region_union;
pub mod robust_cell_clipper;
mod s2loop;
pub mod sequence_lexicon;
pub mod shape;
pub mod shape_index;
pub mod shape_index_buffered_region;
pub mod shape_index_encoding;
pub mod shape_index_measures;
pub mod shape_index_region;
pub mod shape_measures;
pub mod shape_nesting_query;
pub mod shape_tracker;
pub mod shape_util;
#[cfg(test)]
pub mod testing;
pub mod text_format;
pub mod uv_edge_clipper;
pub mod validation_query;
pub mod value_lexicon;
pub mod wedge_relations;
pub mod winding_operation;
pub mod wrapped_shape;

pub use cap::Cap;
pub use cell::{Cell, CellEdge};
pub use cell_id::{
    CellId, expanded_by_distance_uv, from_face_ij, ij_level_to_bound_uv, lsb_for_level, size_ij,
};
pub use cell_union::CellUnion;
pub use coords::{Face, Level};
pub use latlng::LatLng;
pub use point::{Point, ortho, rotate};
pub use polygon::Polygon;
pub use rect::{Rect, RectVertex};
pub use region::Region;
pub use s2loop::Loop;
