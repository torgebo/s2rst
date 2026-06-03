// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Compile-time tests that all data types satisfy `Sized + Send + Sync + Unpin`.
//!
//! Every ported type must pass this check to ensure
//! safety for use across threads and in async contexts.

fn is_ssu<T: Sized + Send + Sync + Unpin>() {}

// ═══════════════════════════════════════════════════════════════════════════
// r1, r2, r3, s1
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn foundation_types() {
    is_ssu::<s2rst::r1::Interval>();
    is_ssu::<s2rst::r1::Endpoint>();
    is_ssu::<s2rst::r2::Point>();
    is_ssu::<s2rst::r2::Rect>();
    is_ssu::<s2rst::r2::Axis>();
    is_ssu::<s2rst::r3::Vector>();
    is_ssu::<s2rst::r3::Axis>();
    is_ssu::<s2rst::r3::Matrix3x3>();
    is_ssu::<s2rst::r3::PreciseVector>();
    is_ssu::<s2rst::r3::ExactFloat>();
    is_ssu::<s2rst::s1::Angle>();
    is_ssu::<s2rst::s1::ChordAngle>();
    is_ssu::<s2rst::s1::Interval>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Core geometry
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_core_geometry() {
    is_ssu::<s2rst::s2::Point>();
    is_ssu::<s2rst::s2::LatLng>();
    is_ssu::<s2rst::s2::CellId>();
    is_ssu::<s2rst::s2::Cell>();
    is_ssu::<s2rst::s2::CellEdge>();
    is_ssu::<s2rst::s2::CellUnion>();
    is_ssu::<s2rst::s2::Cap>();
    is_ssu::<s2rst::s2::Rect>();
    is_ssu::<s2rst::s2::RectVertex>();
    is_ssu::<s2rst::s2::Loop>();
    is_ssu::<s2rst::s2::Polygon>();
    is_ssu::<s2rst::s2::Face>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Shape types
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_shape_types() {
    is_ssu::<s2rst::s2::shape::Edge>();
    is_ssu::<s2rst::s2::shape::ShapeEdgeId>();
    is_ssu::<s2rst::s2::shape::ShapeEdge>();
    is_ssu::<s2rst::s2::shape::Chain>();
    is_ssu::<s2rst::s2::shape::ChainPosition>();
    is_ssu::<s2rst::s2::shape::ReferencePoint>();
    is_ssu::<s2rst::s2::lax_loop::LaxLoop>();
    is_ssu::<s2rst::s2::lax_polyline::LaxPolyline>();
    is_ssu::<s2rst::s2::lax_polygon::LaxPolygon>();
    is_ssu::<s2rst::s2::point_vector::PointVector>();
    is_ssu::<s2rst::s2::edge_vector_shape::EdgeVectorShape>();
    is_ssu::<s2rst::s2::polyline::Polyline>();
    is_ssu::<s2rst::s2::point_region::PointRegion>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Shape index
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_shape_index_types() {
    is_ssu::<s2rst::s2::shape_index::ShapeIndex>();
    is_ssu::<s2rst::s2::shape_index::ShapeIndexCell>();
    is_ssu::<s2rst::s2::shape_index::ClippedShape>();
    is_ssu::<s2rst::s2::shape_index::CellRelation>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Enums
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_enums() {
    is_ssu::<s2rst::s2::edge_crossings::Crossing>();
    is_ssu::<s2rst::s2::predicates::Direction>();
    is_ssu::<s2rst::s2::predicates::Excluded>();
    is_ssu::<s2rst::s2::wedge_relations::WedgeRel>();
    is_ssu::<s2rst::s2::contains_point_query::VertexModel>();
    is_ssu::<s2rst::s2::encoded_s2point_vector::CodingHint>();
    is_ssu::<s2rst::s2::crossing_edge_query::CrossingType>();
    is_ssu::<s2rst::s2::density_tree::VisitAction>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Edge operations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_edge_types() {
    is_ssu::<s2rst::s2::edge_clipping::FaceSegment>();
    is_ssu::<s2rst::s2::edge_crosser::EdgeCrosser>();
    is_ssu::<s2rst::s2::edge_query::EdgeQueryResult>();
    is_ssu::<s2rst::s2::edge_query::EdgeQueryOptions>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Query results
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_query_results() {
    is_ssu::<s2rst::s2::closest_edge_query::Result>();
    is_ssu::<s2rst::s2::closest_cell_query::Result>();
    is_ssu::<s2rst::s2::closest_point_query::Result<i32>>();
    is_ssu::<s2rst::s2::furthest_edge_query::Result>();
    is_ssu::<s2rst::s2::hausdorff_distance_query::DirectedResult>();
    is_ssu::<s2rst::s2::hausdorff_distance_query::HausdorffResult>();
    is_ssu::<s2rst::s2::hausdorff_distance_query::HausdorffOptions>();
    is_ssu::<s2rst::s2::chain_interpolation_query::ChainInterpolationResult>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Config / options types
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_config_types() {
    is_ssu::<s2rst::s2::region_coverer::RegionCoverer>();
    is_ssu::<s2rst::s2::buffer_operation::EndCapStyle>();
    is_ssu::<s2rst::s2::buffer_operation::PolylineSide>();
    is_ssu::<s2rst::s2::validation_query::TouchType>();
    is_ssu::<s2rst::s2::validation_query::ValidationOptions>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Boolean operation types
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_boolean_operation_types() {
    is_ssu::<s2rst::s2::boolean_operation::OpType>();
    is_ssu::<s2rst::s2::boolean_operation::PolygonModel>();
    is_ssu::<s2rst::s2::boolean_operation::PolylineModel>();
    is_ssu::<s2rst::s2::boolean_operation::SourceId>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Builder enums and config types
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_builder_types() {
    is_ssu::<s2rst::s2::builder::S2Error>();
    is_ssu::<s2rst::s2::builder::S2ErrorCode>();
    is_ssu::<s2rst::s2::builder::graph::EdgeType>();
    is_ssu::<s2rst::s2::builder::graph::DegenerateEdges>();
    is_ssu::<s2rst::s2::builder::graph::DuplicateEdges>();
    is_ssu::<s2rst::s2::builder::graph::SiblingPairs>();
    is_ssu::<s2rst::s2::builder::graph::LoopType>();
    is_ssu::<s2rst::s2::builder::graph::DegenerateBoundaries>();
    is_ssu::<s2rst::s2::builder::graph::PolylineType>();
    is_ssu::<s2rst::s2::builder::graph::GraphOptions>();
    is_ssu::<s2rst::s2::builder::snap::IdentitySnapFunction>();
    is_ssu::<s2rst::s2::builder::snap::S2CellIdSnapFunction>();
    is_ssu::<s2rst::s2::builder::snap::IntLatLngSnapFunction>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Winding operation types
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_winding_types() {
    is_ssu::<s2rst::s2::winding_operation::WindingRule>();
}

// ═══════════════════════════════════════════════════════════════════════════
// s2 – Miscellaneous types
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn s2_misc_types() {
    is_ssu::<s2rst::s2::metric::Metric>();
    is_ssu::<s2rst::s2::loop_measures::LoopOrder>();
    is_ssu::<s2rst::s2::loop_measures::KahanSum>();
    is_ssu::<s2rst::s2::density_tree::S2DensityTree>();
    is_ssu::<s2rst::s2::density_tree::DensityCell>();
    is_ssu::<s2rst::s2::density_tree::FeatureMap>();
    is_ssu::<s2rst::s2::padded_cell::PaddedCell>();
    is_ssu::<s2rst::s2::latlng_rect_bounder::LatLngRectBounder>();
    is_ssu::<s2rst::s2::polyline_alignment::VertexAlignment>();
    is_ssu::<s2rst::s2::polyline_alignment::MedoidOptions>();
    is_ssu::<s2rst::s2::polyline_alignment::ConsensusOptions>();
    // S2MemoryTracker has Box<dyn FnMut() + Send> — Send but not Sync.
    // is_ssu::<s2rst::s2::memory_tracker::S2MemoryTracker>();
    is_ssu::<s2rst::s2::cell_index::CellIndex>();
}
