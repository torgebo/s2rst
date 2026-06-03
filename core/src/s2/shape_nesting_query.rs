// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry

//! Determines the nesting relationships between chains in a shape.
//!
//! On a sphere, polygon hierarchy is ambiguous. If two chains encircle the
//! sphere at +/- 10 degrees latitude, either one could be considered a shell
//! with the other being its hole. A datum strategy is used to resolve this
//! ambiguity by choosing a reference chain that is always a shell.
//!
//! Corresponds to C++ `s2shape_nesting_query.h/cc`.

#![expect(clippy::cast_sign_loss, reason = "EdgeId (i32) used as Vec indices")]
#![expect(
    clippy::cast_possible_truncation,
    reason = "shape index (usize->i32) for ShapeId"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for ShapeId — always in range"
)]
use crate::s2::Point;
use crate::s2::crossing_edge_query::{CrossingEdgeQuery, CrossingType};
use crate::s2::predicates;
use crate::s2::shape::{Dimension, Shape, ShapeId};
use crate::s2::shape_index::ShapeIndex;

/// A function that selects the datum chain for a shape.
/// The datum chain is always treated as a shell.
pub type DatumStrategy = fn(&dyn Shape) -> usize;

/// Returns the first chain (index 0) as the datum. This is the default strategy.
pub fn first_chain_strategy(_shape: &dyn Shape) -> usize {
    0
}

/// Options for [`ShapeNestingQuery`].
#[derive(Clone, Debug)]
pub struct Options {
    datum_strategy: DatumStrategy,
}

impl PartialEq for Options {
    fn eq(&self, other: &Self) -> bool {
        // Compare function pointers by address (best effort).
        self.datum_strategy as usize == other.datum_strategy as usize
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            datum_strategy: first_chain_strategy,
        }
    }
}

impl Options {
    /// Sets the datum strategy function.
    pub fn set_datum_strategy(&mut self, strategy: DatumStrategy) -> &mut Self {
        self.datum_strategy = strategy;
        self
    }

    /// Returns the current datum strategy.
    pub fn datum_strategy(&self) -> DatumStrategy {
        self.datum_strategy
    }
}

/// Models the parent/child relationship for a chain.
///
/// Shells have no parent (`parent_id` is `None`) and may have holes.
/// Holes have a parent shell and no holes of their own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChainRelation {
    parent: Option<usize>,
    holes: Vec<usize>,
}

impl ChainRelation {
    /// Creates a shell with the given holes.
    pub fn make_shell(holes: &[usize]) -> Self {
        ChainRelation {
            parent: None,
            holes: holes.to_vec(),
        }
    }

    /// Creates a shell with no holes.
    fn new_shell() -> Self {
        ChainRelation {
            parent: None,
            holes: Vec::new(),
        }
    }

    /// Returns the parent chain ID, or `None` if this is a shell.
    pub fn parent_id(&self) -> Option<usize> {
        self.parent
    }

    /// Returns the parent chain ID as i32 (-1 for shells).
    /// Matches C++ API convention.
    pub fn parent_id_signed(&self) -> i32 {
        match self.parent {
            Some(id) => id as i32,
            None => -1,
        }
    }

    /// Returns true if this chain is a shell.
    pub fn is_shell(&self) -> bool {
        self.parent.is_none()
    }

    /// Returns true if this chain is a hole.
    pub fn is_hole(&self) -> bool {
        self.parent.is_some()
    }

    /// Returns the number of holes.
    pub fn num_holes(&self) -> usize {
        self.holes.len()
    }

    /// Returns the hole chain IDs.
    pub fn holes(&self) -> &[usize] {
        &self.holes
    }

    fn set_parent(&mut self, id: usize) {
        self.parent = Some(id);
    }

    fn clear_parent(&mut self) {
        self.parent = None;
    }

    fn add_hole(&mut self, id: usize) {
        self.holes.push(id);
    }
}

/// A simple bitset backed by a `Vec<bool>`.
struct Bitset {
    bits: Vec<bool>,
}

impl Bitset {
    fn new(size: usize) -> Self {
        Bitset {
            bits: vec![false; size],
        }
    }

    fn get(&self, index: usize) -> bool {
        self.bits[index]
    }

    fn set(&mut self, index: usize, value: bool) {
        self.bits[index] = value;
    }

    fn toggle(&mut self, index: usize) {
        self.bits[index] = !self.bits[index];
    }

    fn count_ones(&self) -> usize {
        self.bits.iter().filter(|&&b| b).count()
    }

    fn find_first_set(&self) -> Option<usize> {
        self.bits.iter().position(|&b| b)
    }

    /// Iterates over set bit positions starting from `start` (inclusive).
    fn iter_set_from(&self, start: usize) -> impl Iterator<Item = usize> + '_ {
        self.bits[start..]
            .iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .map(move |(i, _)| start + i)
    }
}

/// Finds the closest of `num_points` equally spaced points on a chain to the target.
fn closest_of_n_points(target: Point, shape: &dyn Shape, chain: usize, num_points: usize) -> usize {
    let chain_len = shape.chain(chain).length;
    let step = (chain_len / num_points).max(1);

    let mut min_dist2 = f64::INFINITY;
    let mut closest_idx = 0;
    for i in 0..num_points {
        let idx = (i * step) % chain_len;
        let point = shape.chain_edge(chain, idx).v0;
        let diff = target.0 - point.0;
        let dist2 = diff.norm2();
        if dist2 < min_dist2 {
            min_dist2 = dist2;
            closest_idx = idx;
        }
    }
    closest_idx
}

/// Returns the next edge in the chain (wrapping around).
fn next_chain_edge(shape: &dyn Shape, chain: usize, edge: usize) -> crate::s2::shape::Edge {
    shape.chain_edge(chain, (edge + 1) % shape.chain(chain).length)
}

/// Returns the previous edge in the chain (wrapping around).
fn prev_chain_edge(shape: &dyn Shape, chain: usize, edge: usize) -> crate::s2::shape::Edge {
    let len = shape.chain(chain).length;
    let index = if edge == 0 { len - 1 } else { edge - 1 };
    shape.chain_edge(chain, index)
}

/// Determines nesting relationships between chains in a shape.
///
/// Chains are classified as either shells or holes. Shells have no parent and
/// may have zero or more holes. Holes belong to a single parent shell.
#[derive(Debug)]
pub struct ShapeNestingQuery<'a> {
    index: &'a ShapeIndex,
    options: Options,
}

impl<'a> ShapeNestingQuery<'a> {
    /// Creates a new query with default options.
    pub fn new(index: &'a ShapeIndex) -> Self {
        ShapeNestingQuery {
            index,
            options: Options::default(),
        }
    }

    /// Creates a new query with the given options.
    pub fn with_options(index: &'a ShapeIndex, options: Options) -> Self {
        ShapeNestingQuery { index, options }
    }

    /// Returns the index being queried.
    pub fn index(&self) -> &ShapeIndex {
        self.index
    }

    /// Returns the options.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Returns a mutable reference to the options.
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    /// Computes the nesting relationships between chains in the given shape.
    ///
    /// Returns a vector of [`ChainRelation`]s in 1:1 correspondence with the
    /// chains in the shape: chain *i*'s relation is at index *i*.
    pub fn compute_shape_nesting(&self, shape_id: impl Into<ShapeId>) -> Vec<ChainRelation> {
        let shape_id = shape_id.into();
        let Some(shape) = self.index.shape(shape_id) else {
            return Vec::new();
        };

        let num_chains = shape.num_chains();
        if num_chains == 0 {
            return Vec::new();
        }

        debug_assert_eq!(shape.dimension(), Dimension::Polygon);

        // A single chain is always a shell.
        if num_chains == 1 {
            return vec![ChainRelation::make_shell(&[])];
        }

        // Bitsets to track possible parents and children for each chain.
        let mut parents: Vec<Bitset> = (0..num_chains).map(|_| Bitset::new(num_chains)).collect();
        let mut children: Vec<Bitset> = (0..num_chains).map(|_| Bitset::new(num_chains)).collect();

        // Get reference vertices from the datum shell.
        let datum_shell = (self.options.datum_strategy)(shape);
        debug_assert!(shape.chain(datum_shell).length >= 3);

        let vertices = [
            shape.chain_edge(datum_shell, 0).v0,
            shape.chain_edge(datum_shell, 1).v0,
            shape.chain_edge(datum_shell, 2).v0,
        ];
        let start_point = vertices[1];
        debug_assert_ne!(start_point, vertices[0]);
        debug_assert_ne!(start_point, vertices[2]);

        let mut crossing_query = CrossingEdgeQuery::new(self.index);

        #[expect(
            clippy::needless_range_loop,
            reason = "index needed for parallel array access"
        )]
        // `chain` indexes multiple arrays and is passed to functions
        for chain in 0..num_chains {
            if chain == datum_shell {
                continue;
            }

            debug_assert!(shape.chain(chain).length >= 3);

            // Find a close point on the target chain.
            let end_idx = closest_of_n_points(start_point, shape, chain, 4);
            let end_point = shape.chain_edge(chain, end_idx).v0;

            // Two chains may share a vertex.
            let start_end_same = end_point == start_point;

            let next = next_chain_edge(shape, chain, end_idx).v0;
            let prev = prev_chain_edge(shape, chain, end_idx).v0;
            let safe_end = if start_end_same { prev } else { end_point };

            // Check if the ray starts into the interior of the datum shell.
            if predicates::ordered_ccw(vertices[2], safe_end, vertices[0], start_point) {
                parents[chain].set(datum_shell, true);
                children[datum_shell].set(chain, true);
            }

            // Check if the ray arrives from the interior of the target chain.
            let safe_start = if start_end_same {
                vertices[0]
            } else {
                start_point
            };
            if predicates::ordered_ccw(next, safe_start, prev, end_point) {
                parents[chain].set(chain, true);
            }

            if !start_end_same {
                // Find all crossing edges from this shape along our ray.
                let crossing_edge_ids = crossing_query.crossings(
                    start_point,
                    end_point,
                    shape,
                    shape_id,
                    CrossingType::Interior,
                );

                // Toggle bits for each crossed chain.
                for &edge_id in &crossing_edge_ids {
                    let other_chain = shape.chain_position(edge_id as usize).chain_id;
                    parents[chain].toggle(other_chain);
                    if other_chain != chain {
                        children[other_chain].toggle(chain);
                    }
                }
            }

            // Final state: datum shell is a parent only if both datum and target
            // chain bits are set.
            let datum_and_chain = parents[chain].get(datum_shell) && parents[chain].get(chain);
            parents[chain].set(datum_shell, datum_and_chain);
            parents[chain].set(chain, false);
        }

        // Remove transitive parents: if A is parent of B and B is parent of C,
        // remove A as direct parent of C.
        let mut current_chain = 0;
        while current_chain < num_chains {
            if parents[current_chain].count_ones() != 1 {
                current_chain += 1;
                continue;
            }

            let Some(parent_chain) = parents[current_chain].find_first_set() else {
                current_chain += 1;
                continue;
            };

            let mut next_chain = current_chain;
            let child_bits: Vec<usize> = children[current_chain].iter_set_from(0).collect();
            for child in child_bits {
                if parents[child].get(parent_chain) {
                    parents[child].set(parent_chain, false);

                    // If this child now has a single parent and we've already passed it,
                    // back up to reprocess it.
                    if parents[child].count_ones() == 1 && child < next_chain {
                        next_chain = child;
                    }
                }
            }

            if next_chain == current_chain {
                current_chain += 1;
            } else {
                current_chain = next_chain;
            }
        }

        // Build ChainRelations from the parent bitsets.
        let mut relations: Vec<ChainRelation> = (0..num_chains)
            .map(|_| ChainRelation::new_shell())
            .collect();

        for chain in 0..num_chains {
            debug_assert!(parents[chain].count_ones() <= 1);

            if let Some(parent) = parents[chain].find_first_set() {
                relations[chain].set_parent(parent);
                relations[parent].add_hole(chain);
            }
        }

        // Apply even-odd rule: detach chains at even depth from their parent.
        for chain in 0..num_chains {
            let mut depth = 0;
            let mut current = chain;
            while let Some(p) = relations[current].parent {
                depth += 1;
                current = p;
                if depth >= num_chains {
                    break;
                }
            }
            debug_assert!(depth < num_chains);

            if depth > 0 && depth % 2 == 0 {
                relations[chain].clear_parent();
            }
        }

        relations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::lax_polygon::LaxPolygon;
    use crate::s2::shape_index::ShapeIndex;
    use std::f64::consts::PI;

    /// Builds a `LaxPolygon` from ring specs (center, `radius_deg`, reversed).
    /// Each ring is a regular polygon with `vertices_per_loop` vertices.
    fn ring_shape(vertices_per_loop: usize, specs: &[(LatLng, f64, bool)]) -> LaxPolygon {
        let radian_step = 2.0 * PI / vertices_per_loop as f64;
        let mut loops: Vec<Vec<Point>> = Vec::new();

        for &(center, radius_deg, reverse) in specs {
            let radius = radius_deg.abs();
            assert!(center.lat.degrees() + radius < 90.0);
            assert!(center.lat.degrees() - radius > -90.0);

            let mut vertices = Vec::with_capacity(vertices_per_loop);
            for i in 0..vertices_per_loop {
                let angle = i as f64 * radian_step;
                let pnt = LatLng::from_degrees(radius * angle.sin(), radius * angle.cos());
                let ll = LatLng::from_degrees(
                    center.lat.degrees() + pnt.lat.degrees(),
                    center.lng.degrees() + pnt.lng.degrees(),
                );
                vertices.push(ll.normalized().to_point());
            }

            if reverse {
                vertices.reverse();
            }

            loops.push(vertices);
        }

        LaxPolygon::from_loops_owned(loops)
    }

    /// Specification for a circular arc about a center point.
    ///
    /// The arc has the given `thickness` and extends from `start_deg` to
    /// `end_deg` in angular measure. The inner radius is `radius_deg -
    /// thickness` and the outer radius is `radius_deg + thickness`.
    ///
    /// Corresponds to C++ `ArcSpec` in `s2shape_nesting_query_test.cc`.
    struct ArcSpec {
        center: LatLng,
        radius_deg: f64,
        thickness: f64,
        start_deg: f64,
        end_deg: f64,
        /// If non-zero, rotate ring vertices by this many positions.
        offset: usize,
        /// If true, reverse vertex order (CW instead of CCW).
        reverse: bool,
    }

    /// Builds a `LaxPolygon` from one or more `ArcSpec`s. Each spec yields
    /// an arc on a circle made to have the specified thickness. The inner and
    /// outer edges have their ends connected with a butt cap.
    ///
    /// `vertices_per_loop` must be even. Half the vertices trace the outer
    /// edge and half trace the inner edge (in reverse order).
    ///
    /// Corresponds to C++ `ArcShape()` in `s2shape_nesting_query_test.cc`.
    fn arc_shape(vertices_per_loop: usize, specs: &[ArcSpec]) -> LaxPolygon {
        assert!(
            vertices_per_loop.is_multiple_of(2),
            "vertices_per_loop must be even"
        );

        let deg2rad = |degrees: f64| degrees * PI / 180.0;
        let mut loops: Vec<Vec<Point>> = Vec::new();

        for spec in specs {
            let start_rad = deg2rad(spec.start_deg);
            let end_rad = deg2rad(spec.end_deg);

            assert!(start_rad < end_rad, "start_deg must be < end_deg");
            assert!(spec.radius_deg > 0.0);
            assert!(spec.thickness > 0.0);

            let radius_inner = spec.radius_deg - spec.thickness;
            let radius_outer = spec.radius_deg + spec.thickness;
            let half = vertices_per_loop / 2;
            let radian_step = (end_rad - start_rad) / (half - 1) as f64;

            // Pole safety check.
            assert!(
                spec.center.lat.degrees() + spec.radius_deg + spec.thickness < 90.0,
                "arc too close to north pole"
            );
            assert!(
                spec.center.lat.degrees() - spec.radius_deg - spec.thickness > -90.0,
                "arc too close to south pole"
            );

            // Generate outer edge (first half) and inner edge (second half,
            // reversed) with implied butt joints at the ends.
            let mut vertices = vec![Point::default(); vertices_per_loop];
            for i in 0..half {
                let angle = start_rad + i as f64 * radian_step;
                let (sina, cosa) = angle.sin_cos();

                let pnt_outer = LatLng::from_degrees(radius_outer * sina, radius_outer * cosa);
                let pnt_inner = LatLng::from_degrees(radius_inner * sina, radius_inner * cosa);

                let ll_outer = LatLng::from_degrees(
                    spec.center.lat.degrees() + pnt_outer.lat.degrees(),
                    spec.center.lng.degrees() + pnt_outer.lng.degrees(),
                );
                let ll_inner = LatLng::from_degrees(
                    spec.center.lat.degrees() + pnt_inner.lat.degrees(),
                    spec.center.lng.degrees() + pnt_inner.lng.degrees(),
                );

                vertices[i] = ll_outer.normalized().to_point();
                vertices[vertices_per_loop - i - 1] = ll_inner.normalized().to_point();
            }

            // Rotate if offset is specified.
            if spec.offset > 0 {
                let shift = spec.offset % vertices_per_loop;
                vertices.rotate_left(shift);
            }

            if spec.reverse {
                vertices.reverse();
            }

            loops.push(vertices);
        }

        LaxPolygon::from_loops_owned(loops)
    }

    #[test]
    fn test_one_chain_always_shell() {
        let num_edges = 100;
        let mut index = ShapeIndex::new();
        let shape = ring_shape(num_edges, &[(LatLng::from_degrees(0.0, 0.0), 1.0, false)]);
        let id = index.add(Box::new(shape));
        index.build();

        let query = ShapeNestingQuery::new(&index);
        let relations = query.compute_shape_nesting(id);

        assert_eq!(relations.len(), 1);
        assert!(relations[0].is_shell());
        assert!(!relations[0].is_hole());
        assert!(relations[0].parent_id().is_none());
        assert_eq!(relations[0].num_holes(), 0);
    }

    #[test]
    fn test_two_chains_form_pair() {
        let num_edges = 100;
        let center = LatLng::from_degrees(0.0, 0.0);

        // Nested rings, like a donut.
        {
            let mut index = ShapeIndex::new();
            let shape = ring_shape(num_edges, &[(center, 1.0, false), (center, 0.5, true)]);
            let id = index.add(Box::new(shape));
            index.build();

            let query = ShapeNestingQuery::new(&index);
            let relations = query.compute_shape_nesting(id);

            assert_eq!(relations.len(), 2);
            assert!(relations[0].is_shell());
            assert!(relations[1].is_hole());
            assert!(!relations[0].is_hole());
            assert!(!relations[1].is_shell());

            assert!(relations[0].parent_id().is_none());
            assert_eq!(relations[0].num_holes(), 1);
            assert_eq!(relations[0].holes()[0], 1);

            assert_eq!(relations[1].parent_id(), Some(0));
            assert_eq!(relations[1].num_holes(), 0);
        }

        // Swapping ring ordering shouldn't change anything.
        {
            let mut index = ShapeIndex::new();
            let shape = ring_shape(num_edges, &[(center, 0.5, true), (center, 1.0, false)]);
            let id = index.add(Box::new(shape));
            index.build();

            let query = ShapeNestingQuery::new(&index);
            let relations = query.compute_shape_nesting(id);

            assert_eq!(relations.len(), 2);
            assert!(relations[0].is_shell());
            assert!(relations[1].is_hole());

            assert!(relations[0].parent_id().is_none());
            assert_eq!(relations[0].num_holes(), 1);
            assert_eq!(relations[0].holes()[0], 1);

            assert_eq!(relations[1].parent_id(), Some(0));
            assert_eq!(relations[1].num_holes(), 0);
        }

        // Reversed vertex order: both face the same way, so both are shells.
        {
            let mut index = ShapeIndex::new();
            let shape = ring_shape(num_edges, &[(center, 1.0, true), (center, 0.5, false)]);
            let id = index.add(Box::new(shape));
            index.build();

            let query = ShapeNestingQuery::new(&index);
            let relations = query.compute_shape_nesting(id);

            assert_eq!(relations.len(), 2);
            for rel in &relations {
                assert!(rel.is_shell());
                assert!(!rel.is_hole());
                assert!(rel.parent_id().is_none());
                assert_eq!(rel.num_holes(), 0);
            }
        }
    }

    #[test]
    fn test_two_chains_with_shared_vertex() {
        let p = |lat: f64, lng: f64| -> Point { LatLng::from_degrees(lat, lng).to_point() };

        // A quadrangle and a pentagon sharing a vertex.
        let loop1 = vec![p(0.0, 0.0), p(0.0, -1.0), p(-1.0, -1.0), p(-1.0, 0.0)];
        let loop2 = vec![
            p(0.0, 0.0),
            p(0.0, 1.0),
            p(1.0, 2.0),
            p(2.0, 1.0),
            p(1.0, 0.0),
        ];

        // Check all rotations of the two loops.
        for i in 0..loop1.len() {
            for j in 0..loop2.len() {
                let mut l1 = loop1.clone();
                l1.rotate_left(i);
                let mut l2 = loop2.clone();
                l2.rotate_left(j);

                let shape = LaxPolygon::from_loops(&[&l1, &l2]);
                let mut index = ShapeIndex::new();
                let id = index.add(Box::new(shape));
                index.build();

                let query = ShapeNestingQuery::new(&index);
                let relations = query.compute_shape_nesting(id);

                assert_eq!(relations.len(), 2, "rotation ({i},{j})");
                assert!(
                    relations[0].is_shell(),
                    "rotation ({i},{j}): chain 0 should be shell"
                );
                assert!(
                    relations[1].is_shell(),
                    "rotation ({i},{j}): chain 1 should be shell"
                );
            }
        }
    }

    #[test]
    fn test_can_set_datum_shell_option() {
        let num_edges = 100;
        let center = LatLng::from_degrees(0.0, 0.0);

        let mut index = ShapeIndex::new();
        let shape = ring_shape(num_edges, &[(center, 1.0, false), (center, 0.5, true)]);
        let id = index.add(Box::new(shape));
        index.build();

        let mut options = Options::default();
        options.set_datum_strategy(|_shape: &dyn Shape| -> usize { 1 });

        let query = ShapeNestingQuery::with_options(&index, options);
        let relations = query.compute_shape_nesting(id);

        assert_eq!(relations.len(), 2);
        assert!(relations[1].is_shell());
        assert!(relations[0].is_hole());
        assert!(!relations[1].is_hole());
        assert!(!relations[0].is_shell());
    }

    #[test]
    fn test_shell_can_have_multiple_holes() {
        let num_edges = 16;

        // A ring with four holes in it like a shirt button.
        let mut index = ShapeIndex::new();
        let shape = ring_shape(
            num_edges,
            &[
                (LatLng::from_degrees(0.5, 0.5), 2.0, false),
                (LatLng::from_degrees(1.0, 0.5), 0.25, true),
                (LatLng::from_degrees(0.0, 0.5), 0.25, true),
                (LatLng::from_degrees(0.5, 1.0), 0.25, true),
                (LatLng::from_degrees(0.5, 0.0), 0.25, true),
            ],
        );
        let id = index.add(Box::new(shape));
        index.build();

        let query = ShapeNestingQuery::new(&index);
        let relations = query.compute_shape_nesting(id);

        assert_eq!(relations.len(), 5);
        assert!(relations[0].is_shell());
        assert!(!relations[0].is_hole());
        assert!(relations[0].parent_id().is_none());
        assert_eq!(relations[0].num_holes(), 4);

        for i in 1..5 {
            assert_eq!(relations[0].holes()[i - 1], i);
            assert!(relations[i].is_hole());
            assert!(!relations[i].is_shell());
            assert_eq!(relations[i].parent_id(), Some(0));
            assert_eq!(relations[i].num_holes(), 0);
        }
    }

    #[test]
    fn test_nested_chains_partition_correctly() {
        let num_edges = 16;
        let center = LatLng::from_degrees(0.0, 0.0);

        // Test with the outer ring as first chain, no shuffling.
        for depth in &[3, 4, 5, 8] {
            let depth = *depth;
            let mut specs: Vec<(LatLng, f64, bool)> = Vec::with_capacity(depth);
            for i in 0..depth {
                specs.push((center, 2.0 / (i as f64 + 1.0), i % 2 == 1));
            }

            let mut index = ShapeIndex::new();
            let shape = ring_shape(num_edges, &specs);
            let id = index.add(Box::new(shape));
            index.build();

            let query = ShapeNestingQuery::new(&index);
            let relations = query.compute_shape_nesting(id);
            assert_eq!(relations.len(), depth, "depth={depth}");

            // With outer ring first and no shuffling: alternates shell/hole.
            assert!(relations[0].is_shell(), "depth={depth}");
            assert_eq!(relations[0].num_holes(), 1, "depth={depth}");
            assert_eq!(relations[0].holes()[0], 1, "depth={depth}");

            for (chain, rel) in relations.iter().enumerate().skip(1).take(depth - 1) {
                if chain % 2 == 1 {
                    assert!(rel.is_hole(), "depth={depth}, chain={chain}: expected hole");
                    assert_eq!(
                        rel.parent_id(),
                        Some(chain - 1),
                        "depth={depth}, chain={chain}"
                    );
                } else {
                    assert!(
                        rel.is_shell(),
                        "depth={depth}, chain={chain}: expected shell"
                    );
                    assert!(rel.parent_id().is_none(), "depth={depth}, chain={chain}");
                }
            }

            // Verify all chains are accounted for.
            let mut num_shells = 0;
            let mut num_holes = 0;
            for chain in 0..depth {
                if relations[chain].is_shell() {
                    num_shells += 1;
                    for &child in relations[chain].holes() {
                        assert_eq!(relations[child].parent_id(), Some(chain));
                    }
                }
                if relations[chain].is_hole() {
                    num_holes += 1;
                    let parent = relations[chain].parent_id().unwrap();
                    assert!(relations[parent].holes().contains(&chain));
                }
            }
            assert_eq!(num_holes + num_shells, depth, "depth={depth}");
        }
    }

    #[test]
    fn test_exact_path_is_irrelevant() {
        // C++: S2ShapeNestingQuery::ExactPathIsIrrelevant
        //
        // The path we take from the datum shell to the inner shell shouldn't
        // matter for the final classification. Build nested C-shaped arcs
        // (highly concave) and shift the datum ring and other rings a point
        // at a time to cover all vertex permutations.
        let num_edges = 32;
        let center = LatLng::from_degrees(0.0, 0.0);

        for offset0 in 0..num_edges {
            for offset1 in 0..num_edges {
                let shape = arc_shape(
                    num_edges,
                    &[
                        ArcSpec {
                            center,
                            radius_deg: 0.3,
                            thickness: 0.15,
                            start_deg: -240.0,
                            end_deg: 60.0,
                            offset: offset0,
                            reverse: false,
                        },
                        ArcSpec {
                            center,
                            radius_deg: 0.3,
                            thickness: 0.05,
                            start_deg: -230.0,
                            end_deg: 50.0,
                            offset: offset1,
                            reverse: true,
                        },
                        ArcSpec {
                            center,
                            radius_deg: 1.0,
                            thickness: 0.15,
                            start_deg: -85.0,
                            end_deg: 265.0,
                            offset: offset1,
                            reverse: false,
                        },
                        ArcSpec {
                            center,
                            radius_deg: 1.0,
                            thickness: 0.05,
                            start_deg: -80.0,
                            end_deg: 260.0,
                            offset: offset1,
                            reverse: true,
                        },
                    ],
                );

                let mut index = ShapeIndex::new();
                let id = index.add(Box::new(shape));
                index.build();

                let query = ShapeNestingQuery::new(&index);
                let relations = query.compute_shape_nesting(id);

                assert_eq!(
                    relations.len(),
                    4,
                    "offset=({offset0},{offset1}): expected 4 chains"
                );
                assert!(
                    relations[0].is_shell(),
                    "offset=({offset0},{offset1}): chain 0 should be shell"
                );
                assert!(
                    relations[1].is_hole(),
                    "offset=({offset0},{offset1}): chain 1 should be hole"
                );
                assert_eq!(
                    relations[1].parent_id(),
                    Some(0),
                    "offset=({offset0},{offset1}): chain 1 parent should be 0"
                );
                assert!(
                    relations[2].is_shell(),
                    "offset=({offset0},{offset1}): chain 2 should be shell"
                );
                assert!(
                    relations[3].is_hole(),
                    "offset=({offset0},{offset1}): chain 3 should be hole"
                );
                assert_eq!(
                    relations[3].parent_id(),
                    Some(2),
                    "offset=({offset0},{offset1}): chain 3 parent should be 2"
                );
            }
        }
    }

    // ─── ArcShape unit tests ────────────────────────────────────────────

    #[test]
    fn test_arc_shape_basic() {
        // Verify that arc_shape produces a valid polygon with the expected
        // number of chains and vertices.
        let center = LatLng::from_degrees(0.0, 0.0);
        let shape = arc_shape(
            16,
            &[ArcSpec {
                center,
                radius_deg: 1.0,
                thickness: 0.5,
                start_deg: -90.0,
                end_deg: 90.0,
                offset: 0,
                reverse: false,
            }],
        );
        assert_eq!(shape.num_loops(), 1);
        assert_eq!(shape.num_loop_vertices(0), 16);
    }

    #[test]
    fn test_arc_shape_two_arcs() {
        // Two arcs → two chains.
        let center = LatLng::from_degrees(0.0, 0.0);
        let shape = arc_shape(
            8,
            &[
                ArcSpec {
                    center,
                    radius_deg: 1.0,
                    thickness: 0.3,
                    start_deg: -90.0,
                    end_deg: 90.0,
                    offset: 0,
                    reverse: false,
                },
                ArcSpec {
                    center,
                    radius_deg: 0.5,
                    thickness: 0.2,
                    start_deg: -80.0,
                    end_deg: 80.0,
                    offset: 0,
                    reverse: true,
                },
            ],
        );
        assert_eq!(shape.num_loops(), 2);
        assert_eq!(shape.num_loop_vertices(0), 8);
        assert_eq!(shape.num_loop_vertices(1), 8);
    }

    #[test]
    fn test_arc_shape_offset_and_reverse() {
        // Verify that offset rotates vertices and reverse reverses them.
        let center = LatLng::from_degrees(0.0, 0.0);
        let base = arc_shape(
            8,
            &[ArcSpec {
                center,
                radius_deg: 1.0,
                thickness: 0.3,
                start_deg: -90.0,
                end_deg: 90.0,
                offset: 0,
                reverse: false,
            }],
        );
        let rotated = arc_shape(
            8,
            &[ArcSpec {
                center,
                radius_deg: 1.0,
                thickness: 0.3,
                start_deg: -90.0,
                end_deg: 90.0,
                offset: 3,
                reverse: false,
            }],
        );
        let reversed = arc_shape(
            8,
            &[ArcSpec {
                center,
                radius_deg: 1.0,
                thickness: 0.3,
                start_deg: -90.0,
                end_deg: 90.0,
                offset: 0,
                reverse: true,
            }],
        );

        // Same number of vertices in all cases.
        assert_eq!(base.num_loop_vertices(0), 8);
        assert_eq!(rotated.num_loop_vertices(0), 8);
        assert_eq!(reversed.num_loop_vertices(0), 8);

        // Rotated: vertex 0 of rotated should equal vertex 3 of base.
        assert_eq!(rotated.loop_vertex(0, 0), base.loop_vertex(0, 3));

        // Reversed: first vertex of reversed should be last of base.
        assert_eq!(reversed.loop_vertex(0, 0), base.loop_vertex(0, 7));
    }

    #[test]
    fn test_arc_shape_nesting_two_concentric_arcs() {
        // Two concentric C-shaped arcs: outer shell + inner hole.
        let center = LatLng::from_degrees(0.0, 0.0);
        let shape = arc_shape(
            16,
            &[
                ArcSpec {
                    center,
                    radius_deg: 1.0,
                    thickness: 0.3,
                    start_deg: -170.0,
                    end_deg: 170.0,
                    offset: 0,
                    reverse: false,
                },
                ArcSpec {
                    center,
                    radius_deg: 1.0,
                    thickness: 0.1,
                    start_deg: -160.0,
                    end_deg: 160.0,
                    offset: 0,
                    reverse: true,
                },
            ],
        );

        let mut index = ShapeIndex::new();
        let id = index.add(Box::new(shape));
        index.build();

        let query = ShapeNestingQuery::new(&index);
        let relations = query.compute_shape_nesting(id);

        assert_eq!(relations.len(), 2);
        assert!(relations[0].is_shell());
        assert!(relations[1].is_hole());
        assert_eq!(relations[1].parent_id(), Some(0));
    }
}
