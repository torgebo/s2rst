// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// Layer trait for S2Builder.
//
// A Layer receives an assembled Graph and produces output geometry.

use super::S2Error;
use super::graph::{Graph, GraphOptions};

/// A label attached to input edges.
pub type Label = super::Label;

/// Predicate that determines whether a polygon graph represents
/// the full polygon (the entire sphere).
pub type IsFullPolygonPredicate =
    std::sync::Arc<dyn Fn(&Graph) -> Result<bool, S2Error> + Send + Sync>;

/// A layer receives an assembled Graph from `S2Builder` and produces
/// output geometry (points, polylines, polygons, etc.).
///
/// After `S2Builder::build()` completes, layers are returned to the caller
/// who can downcast them via [`Layer::into_any`] or use the concrete type's
/// `into_output()` method to extract the built geometry.
pub trait Layer: std::fmt::Debug {
    /// Returns the `GraphOptions` that control how edges are processed
    /// for this layer (edge type, degenerate handling, etc.).
    fn graph_options(&self) -> GraphOptions;

    /// Assembles the given graph into the output geometry type
    /// implemented by this layer.
    fn build(&mut self, graph: &Graph, error: &mut S2Error);

    /// Converts this layer into a `Box<dyn Any>` for downcasting to the
    /// concrete type. This enables extracting typed output after
    /// `S2Builder::build()` returns the layers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let layers = builder.build()?;
    /// let layer = layers.into_iter().next().unwrap()
    ///     .into_any().downcast::<S2PolygonLayer>().unwrap();
    /// let polygon = layer.into_output();
    /// ```
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
}
