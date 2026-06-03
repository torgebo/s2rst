// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// Indexed layer variants that build shapes and add them to a ShapeIndex.
//
// Each `Indexed*Layer` is a thin wrapper around the corresponding base layer.
// After the base layer builds its output, the indexed layer adds the result
// to its owned `ShapeIndex`.
//
// C++ equivalents: `IndexedLaxPolygonLayer`, `IndexedLaxPolylineLayer`,
// `IndexedS2PolygonLayer`, `IndexedS2PolylineLayer`,
// `IndexedS2PolylineVectorLayer`, `IndexedS2PointVectorLayer`.

use crate::s2::lax_polygon::LaxPolygon;
use crate::s2::lax_polyline::LaxPolyline;
use crate::s2::point_vector::PointVector;
use crate::s2::polygon::Polygon;
use crate::s2::polyline::Polyline;
use crate::s2::shape_index::ShapeIndex;

use super::S2Error;
use super::graph::{Graph, GraphOptions};
use super::layer::Layer;
use super::{lax_polygon_layer, lax_polyline_layer, point_vector_layer, polygon_layer};
use super::{polyline_layer, polyline_vector_layer};

// ─── IndexedLaxPolygonLayer ─────────────────────────────────────────────

/// A layer that builds a `LaxPolygon` and adds it to a `ShapeIndex`.
///
/// C++: `s2builderutil::IndexedLaxPolygonLayer`
#[derive(Debug)]
pub struct IndexedLaxPolygonLayer {
    index: ShapeIndex,
    layer: lax_polygon_layer::LaxPolygonLayer,
}

impl IndexedLaxPolygonLayer {
    /// Creates a new indexed layer.
    pub fn new() -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: lax_polygon_layer::LaxPolygonLayer::new(),
        }
    }

    /// Creates a new indexed layer with options.
    pub fn with_options(options: lax_polygon_layer::Options) -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: lax_polygon_layer::LaxPolygonLayer::with_options(options),
        }
    }

    /// Consumes this layer and returns the built `ShapeIndex`.
    pub fn into_output(self) -> ShapeIndex {
        self.index
    }

    /// Returns a reference to the built `ShapeIndex`.
    pub fn output(&self) -> &ShapeIndex {
        &self.index
    }
}

impl Layer for IndexedLaxPolygonLayer {
    fn graph_options(&self) -> GraphOptions {
        self.layer.graph_options()
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        self.layer.build(graph, error);
        if error.is_ok()
            && let Some(polygon) = self.layer.output()
            && polygon.num_loops() > 0
        {
            let polygon = self.layer.take_output().unwrap_or_else(LaxPolygon::empty);
            self.index.add(Box::new(polygon));
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ─── IndexedLaxPolylineLayer ────────────────────────────────────────────

/// A layer that builds a `LaxPolyline` and adds it to a `ShapeIndex`.
///
/// C++: `s2builderutil::IndexedLaxPolylineLayer`
#[derive(Debug)]
pub struct IndexedLaxPolylineLayer {
    index: ShapeIndex,
    layer: lax_polyline_layer::LaxPolylineLayer,
}

impl IndexedLaxPolylineLayer {
    /// Creates a new indexed layer.
    pub fn new() -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: lax_polyline_layer::LaxPolylineLayer::new(),
        }
    }

    /// Creates a new indexed layer with options.
    pub fn with_options(options: lax_polyline_layer::Options) -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: lax_polyline_layer::LaxPolylineLayer::with_options(options),
        }
    }

    /// Consumes this layer and returns the built `ShapeIndex`.
    pub fn into_output(self) -> ShapeIndex {
        self.index
    }

    /// Returns a reference to the built `ShapeIndex`.
    pub fn output(&self) -> &ShapeIndex {
        &self.index
    }
}

impl Layer for IndexedLaxPolylineLayer {
    fn graph_options(&self) -> GraphOptions {
        self.layer.graph_options()
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        self.layer.build(graph, error);
        if error.is_ok()
            && let Some(polyline) = self.layer.output()
            && polyline.num_vertices() > 0
        {
            let polyline = self
                .layer
                .take_output()
                .unwrap_or_else(|| LaxPolyline::new(vec![]));
            self.index.add(Box::new(polyline));
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ─── IndexedS2PolygonLayer ──────────────────────────────────────────────

/// A layer that builds an `S2Polygon` and adds it to a `ShapeIndex`.
///
/// C++: `s2builderutil::IndexedS2PolygonLayer`
#[derive(Debug)]
pub struct IndexedS2PolygonLayer {
    index: ShapeIndex,
    layer: polygon_layer::S2PolygonLayer,
}

impl IndexedS2PolygonLayer {
    /// Creates a new indexed layer.
    pub fn new() -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: polygon_layer::S2PolygonLayer::new(),
        }
    }

    /// Creates a new indexed layer with options.
    pub fn with_options(options: polygon_layer::Options) -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: polygon_layer::S2PolygonLayer::with_options(options),
        }
    }

    /// Consumes this layer and returns the built `ShapeIndex`.
    pub fn into_output(self) -> ShapeIndex {
        self.index
    }

    /// Returns a reference to the built `ShapeIndex`.
    pub fn output(&self) -> &ShapeIndex {
        &self.index
    }
}

impl Layer for IndexedS2PolygonLayer {
    fn graph_options(&self) -> GraphOptions {
        self.layer.graph_options()
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        self.layer.build(graph, error);
        if error.is_ok()
            && let Some(polygon) = self.layer.output()
            && !polygon.is_empty_polygon()
        {
            let polygon = self.layer.take_output().unwrap_or_else(Polygon::empty);
            self.index.add(Box::new(polygon));
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ─── IndexedS2PolylineLayer ────────────────────────────────────────────

/// A layer that builds an `S2Polyline` and adds it to a `ShapeIndex`.
///
/// C++: `s2builderutil::IndexedS2PolylineLayer`
#[derive(Debug)]
pub struct IndexedS2PolylineLayer {
    index: ShapeIndex,
    layer: polyline_layer::S2PolylineLayer,
}

impl IndexedS2PolylineLayer {
    /// Creates a new indexed layer.
    pub fn new() -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: polyline_layer::S2PolylineLayer::new(),
        }
    }

    /// Creates a new indexed layer with options.
    pub fn with_options(options: polyline_layer::Options) -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: polyline_layer::S2PolylineLayer::with_options(options),
        }
    }

    /// Consumes this layer and returns the built `ShapeIndex`.
    pub fn into_output(self) -> ShapeIndex {
        self.index
    }

    /// Returns a reference to the built `ShapeIndex`.
    pub fn output(&self) -> &ShapeIndex {
        &self.index
    }
}

impl Layer for IndexedS2PolylineLayer {
    fn graph_options(&self) -> GraphOptions {
        self.layer.graph_options()
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        self.layer.build(graph, error);
        if error.is_ok()
            && let Some(polyline) = self.layer.output()
            && polyline.num_vertices() > 0
        {
            let polyline = self
                .layer
                .take_output()
                .unwrap_or_else(|| Polyline::new(vec![]));
            self.index.add(Box::new(polyline));
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ─── IndexedS2PolylineVectorLayer ──────────────────────────────────────

/// A layer that builds a vector of `S2Polyline` and adds each to a `ShapeIndex`.
///
/// C++: `s2builderutil::IndexedS2PolylineVectorLayer`
#[derive(Debug)]
pub struct IndexedS2PolylineVectorLayer {
    index: ShapeIndex,
    layer: polyline_vector_layer::S2PolylineVectorLayer,
}

impl IndexedS2PolylineVectorLayer {
    /// Creates a new indexed layer.
    pub fn new() -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: polyline_vector_layer::S2PolylineVectorLayer::new(),
        }
    }

    /// Creates a new indexed layer with options.
    pub fn with_options(options: polyline_vector_layer::Options) -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: polyline_vector_layer::S2PolylineVectorLayer::with_options(options),
        }
    }

    /// Consumes this layer and returns the built `ShapeIndex`.
    pub fn into_output(self) -> ShapeIndex {
        self.index
    }

    /// Returns a reference to the built `ShapeIndex`.
    pub fn output(&self) -> &ShapeIndex {
        &self.index
    }
}

impl Layer for IndexedS2PolylineVectorLayer {
    fn graph_options(&self) -> GraphOptions {
        self.layer.graph_options()
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        self.layer.build(graph, error);
        if error.is_ok()
            && let Some(polylines) = self.layer.take_output()
        {
            for polyline in polylines {
                if polyline.num_vertices() > 0 {
                    self.index.add(Box::new(polyline));
                }
            }
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ─── IndexedS2PointVectorLayer ─────────────────────────────────────────

/// A layer that builds a vector of `S2Point` and adds a `PointVector` shape
/// to a `ShapeIndex`.
///
/// C++: `s2builderutil::IndexedS2PointVectorLayer`
#[derive(Debug)]
pub struct IndexedS2PointVectorLayer {
    index: ShapeIndex,
    layer: point_vector_layer::S2PointVectorLayer,
}

impl IndexedS2PointVectorLayer {
    /// Creates a new indexed layer.
    pub fn new() -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: point_vector_layer::S2PointVectorLayer::new(),
        }
    }

    /// Creates a new indexed layer with options.
    pub fn with_options(options: point_vector_layer::Options) -> Self {
        Self {
            index: ShapeIndex::new(),
            layer: point_vector_layer::S2PointVectorLayer::with_options(options),
        }
    }

    /// Consumes this layer and returns the built `ShapeIndex`.
    pub fn into_output(self) -> ShapeIndex {
        self.index
    }

    /// Returns a reference to the built `ShapeIndex`.
    pub fn output(&self) -> &ShapeIndex {
        &self.index
    }
}

impl Layer for IndexedS2PointVectorLayer {
    fn graph_options(&self) -> GraphOptions {
        self.layer.graph_options()
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        self.layer.build(graph, error);
        if error.is_ok()
            && let Some(points) = self.layer.take_output()
            && !points.is_empty()
        {
            self.index.add(Box::new(PointVector::new(points)));
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

impl Default for IndexedLaxPolygonLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IndexedLaxPolylineLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IndexedS2PolygonLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IndexedS2PolylineLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IndexedS2PolylineVectorLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IndexedS2PointVectorLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::text_format::{make_polyline, parse_point};

    #[test]
    fn test_indexed_lax_polygon_layer() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedLaxPolygonLayer::new()));
        builder.add_edge(parse_point("0:0"), parse_point("0:10"));
        builder.add_edge(parse_point("0:10"), parse_point("10:0"));
        builder.add_edge(parse_point("10:0"), parse_point("0:0"));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedLaxPolygonLayer>()
            .expect("wrong layer type");
        assert_eq!(layer.into_output().len(), 1, "expected 1 shape in index");
    }

    #[test]
    fn test_indexed_lax_polyline_layer() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedLaxPolylineLayer::new()));
        let polyline = make_polyline("0:0, 1:1, 2:2");
        builder.add_polyline(&polyline);
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedLaxPolylineLayer>()
            .expect("wrong layer type");
        assert_eq!(layer.into_output().len(), 1, "expected 1 shape in index");
    }

    #[test]
    fn test_indexed_s2_polygon_layer() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PolygonLayer::new()));
        builder.add_edge(parse_point("0:0"), parse_point("0:10"));
        builder.add_edge(parse_point("0:10"), parse_point("10:0"));
        builder.add_edge(parse_point("10:0"), parse_point("0:0"));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PolygonLayer>()
            .expect("wrong layer type");
        assert_eq!(layer.into_output().len(), 1, "expected 1 shape in index");
    }

    #[test]
    fn test_indexed_s2_polyline_layer() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PolylineLayer::new()));
        let polyline = make_polyline("0:0, 1:1, 2:2");
        builder.add_polyline(&polyline);
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PolylineLayer>()
            .expect("wrong layer type");
        assert_eq!(layer.into_output().len(), 1, "expected 1 shape in index");
    }

    #[test]
    fn test_indexed_s2_polyline_vector_layer() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PolylineVectorLayer::new()));
        let p1 = make_polyline("0:0, 1:1, 2:2");
        let p2 = make_polyline("5:5, 6:6");
        builder.add_polyline(&p1);
        builder.add_polyline(&p2);
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PolylineVectorLayer>()
            .expect("wrong layer type");
        assert_eq!(layer.into_output().len(), 2, "expected 2 shapes in index");
    }

    #[test]
    fn test_indexed_s2_point_vector_layer() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PointVectorLayer::new()));
        builder.add_point(parse_point("0:0"));
        builder.add_point(parse_point("1:1"));
        builder.add_point(parse_point("2:2"));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PointVectorLayer>()
            .expect("wrong layer type");
        assert_eq!(
            layer.into_output().len(),
            1,
            "expected 1 shape (PointVector) in index"
        );
    }

    #[test]
    fn test_indexed_lax_polygon_layer_empty_not_added() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedLaxPolygonLayer::new()));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedLaxPolygonLayer>()
            .expect("wrong layer type");
        assert_eq!(
            layer.into_output().len(),
            0,
            "empty output should not be added to index"
        );
    }

    #[test]
    fn test_indexed_lax_polyline_layer_empty_not_added() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedLaxPolylineLayer::new()));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedLaxPolylineLayer>()
            .expect("wrong layer type");
        assert_eq!(
            layer.into_output().len(),
            0,
            "empty polyline should not be added"
        );
    }

    #[test]
    fn test_indexed_s2_polygon_layer_empty_not_added() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PolygonLayer::new()));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PolygonLayer>()
            .expect("wrong layer type");
        assert_eq!(
            layer.into_output().len(),
            0,
            "empty polygon should not be added"
        );
    }

    #[test]
    fn test_indexed_s2_polyline_layer_empty_not_added() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PolylineLayer::new()));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PolylineLayer>()
            .expect("wrong layer type");
        assert_eq!(
            layer.into_output().len(),
            0,
            "empty polyline should not be added"
        );
    }

    #[test]
    fn test_indexed_s2_polyline_vector_layer_empty_not_added() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PolylineVectorLayer::new()));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PolylineVectorLayer>()
            .expect("wrong layer type");
        assert_eq!(
            layer.into_output().len(),
            0,
            "empty output should not be added"
        );
    }

    #[test]
    fn test_indexed_s2_point_vector_layer_empty_not_added() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(IndexedS2PointVectorLayer::new()));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<IndexedS2PointVectorLayer>()
            .expect("wrong layer type");
        assert_eq!(
            layer.into_output().len(),
            0,
            "empty points should not be added"
        );
    }
}
