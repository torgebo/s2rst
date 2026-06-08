// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python bindings for the s2rst spherical geometry library.
//!
//! This crate exposes the core S2 geometry types (Angle, Point, Cell,
//! Polygon, ...) to Python via [`pyo3`]. The bindings are thin wrappers
//! around the underlying Rust types, with operator overloads and Python
//! protocol methods (`__len__`, `__getitem__`, `__repr__`, ...) added
//! where appropriate.

use pyo3::prelude::*;

mod angle;
mod boolean;
mod buffer;
mod builder;
mod cell_query;
mod cells;
mod chain_query;
mod convex_hull;
mod coverer;
mod crossing_query;
mod density;
mod earth;
mod edge_queries;
mod encoding;
mod encoding_rich;
mod enums;
mod geometry;
mod hash_util;
mod index;
mod interval;
mod longtail;
mod measures;
mod metric;
mod misc_queries;
mod point_query;
mod points;
mod polyline_ops;
mod projections;
mod regions;
mod regions_extra;
mod s2point;
mod shapes;
mod snap;
mod term_index;
mod tessellator;
mod text;
mod toolkit;
mod winding;

use angle::{PyAngle, PyChordAngle};
use buffer::{PyBufferOptions, buffer_loop, buffer_point, buffer_polygon, buffer_polyline};
use builder::PyS2Builder;
use cell_query::{PyCellQueryResult, PyClosestCellQuery};
use cells::{PyCell, PyCellId, PyCellUnion};
use chain_query::{PyChainInterpolationQuery, PyChainInterpolationResult};
use convex_hull::PyConvexHullQuery;
use coverer::PyRegionCoverer;
use crossing_query::PyCrossingEdgeQuery;
use density::{PyDensityClusterQuery, PyS2DensityTree};
use earth::PyEarth;
use edge_queries::{PyClosestEdgeQuery, PyEdgeQueryResult, PyFurthestEdgeQuery};
use encoding_rich::{PyEncodedS2ShapeIndex, PySequenceLexicon, PyValueLexicon};
use enums::{
    PyCrossingType, PyEndCapStyle, PyOpType, PyPolygonModel, PyPolylineModel, PyPolylineSide,
    PyVertexModel, PyWindingRule,
};
use geometry::{PyLoop, PyPolygon, PyPolyline};
use index::PyShapeIndex;
use interval::{PyR1Interval, PyS1Interval};
use longtail::{PyCellIndex, PyS2Fractal, PyValidationQuery};
use metric::PyMetric;
use misc_queries::{
    PyChainRelation, PyContainsVertexQuery, PyHausdorffDistanceQuery, PyShapeNestingQuery,
};
use point_query::{PyClosestPointQuery, PyPointQueryResult, PyS2PointIndex};
use points::{PyMatrix3x3, PyR2Point, PyR2Rect, PyVector};
use polyline_ops::PyPolylineSimplifier;
use projections::{PyMercatorProjection, PyPlateCarreeProjection};
use regions::{PyCap, PyRect};
use regions_extra::{PyPointRegion, PyRegionIntersection, PyRegionUnion};
use s2point::{PyLatLng, PyS2Point, s2_ortho, s2_rotate};
use shapes::{
    PyEdge, PyEdgeVectorShape, PyLaxLoop, PyLaxPolygon, PyLaxPolyline, PyPointVector,
    PyReferencePoint, PyShape,
};
use snap::{PyIdentitySnapFunction, PyIntLatLngSnapFunction, PyS2CellIdSnapFunction};
use term_index::{PyRegionSharder, PyRegionTermIndexer};
use tessellator::PyEdgeTessellator;
use toolkit::{PyCrossing, PyDirection, PyEdgeCrosser, PyWedgeRel};
use winding::winding_operation;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    // Phase 1
    m.add_class::<PyAngle>()?;
    m.add_class::<PyChordAngle>()?;
    m.add_class::<PyR1Interval>()?;
    m.add_class::<PyS1Interval>()?;
    // Phase 2
    m.add_class::<PyR2Point>()?;
    m.add_class::<PyVector>()?;
    m.add_class::<PyMatrix3x3>()?;
    m.add_class::<PyR2Rect>()?;
    // Phase 3
    m.add_class::<PyS2Point>()?;
    m.add_class::<PyLatLng>()?;
    m.add_function(wrap_pyfunction!(s2_ortho, m)?)?;
    m.add_function(wrap_pyfunction!(s2_rotate, m)?)?;
    // Phase 4
    m.add_class::<PyCellId>()?;
    m.add_class::<PyCell>()?;
    m.add_class::<PyCellUnion>()?;
    // Phase 5
    m.add_class::<PyCap>()?;
    m.add_class::<PyRect>()?;
    m.add_class::<PyPointRegion>()?;
    m.add_class::<PyRegionUnion>()?;
    m.add_class::<PyRegionIntersection>()?;
    // Phase 6
    m.add_class::<PyPolyline>()?;
    m.add_class::<PyLoop>()?;
    m.add_class::<PyPolygon>()?;
    // Phase 7
    m.add_class::<PyEdge>()?;
    m.add_class::<PyReferencePoint>()?;
    m.add_class::<PyShape>()?;
    m.add_class::<PyLaxLoop>()?;
    m.add_class::<PyLaxPolyline>()?;
    m.add_class::<PyLaxPolygon>()?;
    m.add_class::<PyPointVector>()?;
    m.add_class::<PyEdgeVectorShape>()?;
    // Region coverer
    m.add_class::<PyRegionCoverer>()?;
    // Enums (native, eq_int)
    m.add_class::<PyVertexModel>()?;
    m.add_class::<PyOpType>()?;
    m.add_class::<PyPolygonModel>()?;
    m.add_class::<PyPolylineModel>()?;
    m.add_class::<PyCrossingType>()?;
    m.add_class::<PyEndCapStyle>()?;
    m.add_class::<PyPolylineSide>()?;
    m.add_class::<PyWindingRule>()?;
    // Snap functions
    m.add_class::<PyIdentitySnapFunction>()?;
    m.add_class::<PyS2CellIdSnapFunction>()?;
    m.add_class::<PyIntLatLngSnapFunction>()?;
    // Spatial index + queries
    m.add_class::<PyShapeIndex>()?;
    m.add_class::<PyClosestEdgeQuery>()?;
    m.add_class::<PyFurthestEdgeQuery>()?;
    m.add_class::<PyEdgeQueryResult>()?;
    m.add_class::<PyConvexHullQuery>()?;
    m.add_class::<PyS2PointIndex>()?;
    m.add_class::<PyClosestPointQuery>()?;
    m.add_class::<PyPointQueryResult>()?;
    m.add_class::<PyMetric>()?;
    m.add_class::<PyPlateCarreeProjection>()?;
    m.add_class::<PyMercatorProjection>()?;
    m.add_class::<PyContainsVertexQuery>()?;
    m.add_class::<PyHausdorffDistanceQuery>()?;
    m.add_class::<PyShapeNestingQuery>()?;
    m.add_class::<PyChainRelation>()?;
    // Earth conversions / distances
    m.add_class::<PyEarth>()?;
    // text_format: parse / format
    m.add_function(wrap_pyfunction!(text::parse_point, m)?)?;
    m.add_function(wrap_pyfunction!(text::parse_points, m)?)?;
    m.add_function(wrap_pyfunction!(text::parse_latlngs, m)?)?;
    m.add_function(wrap_pyfunction!(text::make_loop, m)?)?;
    m.add_function(wrap_pyfunction!(text::make_polygon, m)?)?;
    m.add_function(wrap_pyfunction!(text::make_polyline, m)?)?;
    m.add_function(wrap_pyfunction!(text::point_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::points_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::latlng_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::loop_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::polygon_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::polyline_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::make_rect, m)?)?;
    m.add_function(wrap_pyfunction!(text::make_lax_polyline, m)?)?;
    m.add_function(wrap_pyfunction!(text::make_lax_polygon, m)?)?;
    m.add_function(wrap_pyfunction!(text::make_index, m)?)?;
    m.add_function(wrap_pyfunction!(text::index_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::lax_polyline_to_string, m)?)?;
    m.add_function(wrap_pyfunction!(text::lax_polygon_to_string, m)?)?;
    // encoding: round-trip geometry to/from bytes
    m.add_function(wrap_pyfunction!(encoding::encode, m)?)?;
    m.add_function(wrap_pyfunction!(encoding::decode_polygon, m)?)?;
    m.add_function(wrap_pyfunction!(encoding::decode_polyline, m)?)?;
    m.add_function(wrap_pyfunction!(encoding::decode_loop, m)?)?;
    m.add_function(wrap_pyfunction!(encoding::decode_cell_union, m)?)?;
    // Geometry builder
    m.add_class::<PyS2Builder>()?;
    // Boolean operations (index-level)
    m.add_function(wrap_pyfunction!(boolean::boolean_operation, m)?)?;
    m.add_function(wrap_pyfunction!(boolean::intersects, m)?)?;
    m.add_function(wrap_pyfunction!(boolean::contains, m)?)?;
    m.add_function(wrap_pyfunction!(boolean::equals, m)?)?;
    // Buffer operations (expand/contract by a radius)
    m.add_class::<PyBufferOptions>()?;
    m.add_function(wrap_pyfunction!(buffer_point, m)?)?;
    m.add_function(wrap_pyfunction!(buffer_polyline, m)?)?;
    m.add_function(wrap_pyfunction!(buffer_loop, m)?)?;
    m.add_function(wrap_pyfunction!(buffer_polygon, m)?)?;
    // Winding operation (N-way boolean via winding numbers)
    m.add_function(wrap_pyfunction!(winding_operation, m)?)?;
    m.add_class::<PyRegionTermIndexer>()?;
    m.add_class::<PyRegionSharder>()?;
    m.add_class::<PyPolylineSimplifier>()?;
    m.add_class::<PyDirection>()?;
    m.add_class::<PyCrossing>()?;
    m.add_class::<PyWedgeRel>()?;
    m.add_class::<PyEdgeCrosser>()?;
    // Polyline ops + low-level toolkit
    m.add_function(wrap_pyfunction!(
        polyline_ops::get_exact_vertex_alignment,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        polyline_ops::get_exact_vertex_alignment_cost,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        polyline_ops::get_approx_vertex_alignment,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(polyline_ops::get_medoid_polyline, m)?)?;
    m.add_function(wrap_pyfunction!(polyline_ops::get_consensus_polyline, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::sign, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::robust_sign, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::ordered_ccw, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::crossing_sign, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::vertex_crossing, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::edge_or_vertex_crossing, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::intersection, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::robust_cross_prod, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::project, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::interpolate, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::interpolate_at_distance, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::distance_from_segment, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::distance_fraction, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::wedge_relation, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::wedge_contains, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::face_uv_to_xyz, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::xyz_to_face_uv, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::st_to_uv, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::uv_to_st, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::get_u_axis, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::get_v_axis, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::shape_to_points, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::find_self_intersection, m)?)?;
    m.add_function(wrap_pyfunction!(toolkit::visit_crossing_edge_pairs, m)?)?;
    m.add_class::<PySequenceLexicon>()?;
    m.add_class::<PyValueLexicon>()?;
    m.add_class::<PyEncodedS2ShapeIndex>()?;
    m.add_class::<PyCellIndex>()?;
    m.add_class::<PyS2Fractal>()?;
    m.add_class::<PyValidationQuery>()?;
    m.add_class::<PyClosestCellQuery>()?;
    m.add_class::<PyCellQueryResult>()?;
    m.add_class::<PyCrossingEdgeQuery>()?;
    m.add_class::<PyChainInterpolationQuery>()?;
    m.add_class::<PyChainInterpolationResult>()?;
    m.add_class::<PyEdgeTessellator>()?;
    m.add_class::<PyS2DensityTree>()?;
    m.add_class::<PyDensityClusterQuery>()?;
    // Richer encoding (vectors + lexicons + encoded index)
    m.add_function(wrap_pyfunction!(encoding_rich::encode_s2point_vector, m)?)?;
    m.add_function(wrap_pyfunction!(encoding_rich::decode_s2point_vector, m)?)?;
    m.add_function(wrap_pyfunction!(encoding_rich::encode_s2cell_id_vector, m)?)?;
    m.add_function(wrap_pyfunction!(encoding_rich::decode_s2cell_id_vector, m)?)?;
    // Geometric measures (free functions)
    m.add_function(wrap_pyfunction!(measures::point_area, m)?)?;
    m.add_function(wrap_pyfunction!(measures::signed_area, m)?)?;
    m.add_function(wrap_pyfunction!(measures::turn_angle, m)?)?;
    m.add_function(wrap_pyfunction!(measures::true_centroid, m)?)?;
    Ok(())
}
