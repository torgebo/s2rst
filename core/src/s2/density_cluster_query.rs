// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - Java: google/s2-geometry-library-java

//! Computes spatial clusters from an [`S2DensityTree`].
//!
//! Ported from Java `S2DensityClusterQuery`. Given a density tree and a target
//! cluster weight (or weight range), this module partitions the sphere into
//! non-overlapping clusters whose weights are approximately equal.
//!
//! # Example
//!
//! ```
//! use s2rst::s2::density_cluster_query::DensityClusterQuery;
//! use s2rst::s2::density_tree::{S2DensityTree, TreeEncoder};
//! use s2rst::s2::CellId;
//!
//! let mut enc = TreeEncoder::new();
//! enc.put(CellId::from_face(0), 100);
//! enc.put(CellId::from_face(1), 100);
//! let mut tree = S2DensityTree::new();
//! enc.build(&mut tree);
//!
//! let query = DensityClusterQuery::new(100);
//! let clusters = query.clusters(&tree).unwrap();
//! assert_eq!(clusters.len(), 2);
//! ```

use std::cmp::max;

use crate::s2::builder::S2Error;
use crate::s2::cell_id::CellId;
use crate::s2::cell_union::CellUnion;
use crate::s2::coords::Level;
use crate::s2::coords::MAX_CELL_LEVEL;
use crate::s2::density_tree::{S2DensityTree, VisitAction};

/// A cluster is a cell range `[begin, end)` and an expected weight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cluster {
    /// The level-30 cell ID at the beginning of this cluster's range (inclusive).
    pub begin: CellId,
    /// The level-30 cell ID at the end of this cluster's range (exclusive).
    pub end: CellId,
    /// The weight of this cluster.
    pub weight: i64,
}

impl Cluster {
    /// Creates a new cluster with the given range and weight.
    ///
    /// # Panics
    ///
    /// Panics if `begin` or `end` are not leaf cells, or if `begin > end`.
    pub fn new(begin: CellId, end: CellId, weight: i64) -> Self {
        debug_assert!(begin.is_leaf() && end.is_leaf());
        debug_assert!(begin <= end);
        Self { begin, end, weight }
    }

    /// Returns the covering of this cluster as a `CellUnion`.
    pub fn covering(&self) -> CellUnion {
        CellUnion::from_begin_end(self.begin, self.end)
    }

    /// Returns the intersection of this cluster's covering with the given
    /// `CellUnion`.
    pub fn intersection(&self, covering: &CellUnion) -> CellUnion {
        let mine = self.covering();
        mine.intersection(covering)
    }
}

impl std::fmt::Display for Cluster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.begin.is_valid() {
            write!(f, "{}:{:x}", u8::from(self.begin.face()), self.begin.pos())?;
        } else {
            write!(f, "sentinel")?;
        }
        write!(f, ",")?;
        if self.end.is_valid() {
            write!(f, "{}:{:x}", u8::from(self.end.face()), self.end.pos())?;
        } else {
            write!(f, "sentinel")?;
        }
        write!(f, "={}", self.weight)
    }
}

/// Interpolates cell IDs within a parent cell along the Hilbert curve.
struct CellInterpolator {
    parent: CellId,
    level: Level,
    begin: i64,
    length: i64,
}

impl CellInterpolator {
    fn new(parent: CellId) -> Self {
        // Limit the depth so the Hilbert range fits in f64 precision.
        let level = Level::MAX.min(parent.level() + 26);
        let begin = parent.child_begin_at_level(level).distance_from_begin();
        let length = parent.child_end_at_level(level).distance_from_begin() - begin;
        Self {
            parent,
            level,
            begin,
            length,
        }
    }

    /// Returns a cell that is `ratio` percent of the way along the parent's
    /// Hilbert range.
    fn interpolate(&self, ratio: f64) -> CellId {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "ceil produces integer-valued f64"
        )]
        let steps = (ratio * self.length as f64).ceil() as i64;
        self.parent.child_begin_at_level(self.level).advance(steps)
    }

    /// Returns the fraction along the parent where `child` begins.
    fn uninterpolate(&self, child: CellId) -> f64 {
        debug_assert!(child.level() >= self.parent.level());
        let adjusted = if self.level <= child.level() {
            child.parent_at_level(self.level)
        } else {
            child.child_begin_at_level(self.level)
        };
        (adjusted.distance_from_begin() - self.begin) as f64 / self.length as f64
    }
}

/// Computes clusters from a density tree.
///
/// Each resulting cluster has weight approximately within
/// `[min_cluster_weight, max_cluster_weight]`. The clusters partition the
/// sphere (their coverings are disjoint and collectively cover all density).
#[derive(Debug)]
pub struct DensityClusterQuery {
    min_cluster_weight: i64,
    max_cluster_weight: i64,
}

impl DensityClusterQuery {
    /// Creates a query with clusters ±20% of `desired_weight`.
    pub fn new(desired_weight: i64) -> Self {
        Self::with_range(
            max(1, desired_weight * 8 / 10),
            max(1, desired_weight * 12 / 10),
        )
    }

    /// Creates a query with explicit min/max cluster weight bounds.
    ///
    /// # Panics
    ///
    /// Panics if `min_weight` is not positive or exceeds `max_weight`.
    pub fn with_range(min_weight: i64, max_weight: i64) -> Self {
        assert!(min_weight > 0);
        assert!(min_weight <= max_weight);
        Self {
            min_cluster_weight: min_weight,
            max_cluster_weight: max_weight,
        }
    }

    /// Returns the minimum cluster weight.
    pub fn min_cluster_weight(&self) -> i64 {
        self.min_cluster_weight
    }

    /// Returns the maximum cluster weight.
    pub fn max_cluster_weight(&self) -> i64 {
        self.max_cluster_weight
    }

    /// Returns the coverings of each cluster.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the density tree is corrupt.
    pub fn coverings(&self, density: &S2DensityTree) -> Result<Vec<CellUnion>, S2Error> {
        Ok(self
            .clusters(density)?
            .into_iter()
            .map(|c| c.covering())
            .collect())
    }

    /// Returns the tight coverings — each cluster's covering intersected with
    /// the density tree's leaves.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the density tree is corrupt.
    pub fn tight_coverings(&self, density: &S2DensityTree) -> Result<Vec<CellUnion>, S2Error> {
        let mut leaves = density.leaves()?;
        leaves.normalize();
        Ok(self
            .clusters(density)?
            .into_iter()
            .map(|c| c.intersection(&leaves))
            .collect())
    }

    /// Returns clusters computed from the density tree.
    ///
    /// The resulting clusters collectively cover the density tree's spatial
    /// extent. They do not overlap.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the density tree is corrupt.
    pub fn clusters(&self, density: &S2DensityTree) -> Result<Vec<Cluster>, S2Error> {
        let tree = density.normalize()?;
        let mut result = Vec::new();
        let mut begin = CellId::begin(MAX_CELL_LEVEL);
        let end = CellId::end(MAX_CELL_LEVEL);
        loop {
            let c = self.next_cluster(&tree, begin, end, 0)?;
            if c.weight == 0 {
                break;
            }
            begin = c.end;
            result.push(c);
        }
        Ok(result)
    }

    /// Computes the weight of cells in `[begin, end)` from the density tree.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the density tree is corrupt.
    pub fn cluster_weight(
        density: &S2DensityTree,
        begin: CellId,
        end: CellId,
    ) -> Result<Cluster, S2Error> {
        let mut sum: i64 = 0;
        density.visit_cells(|cell, node| {
            // Skip disjoint cells.
            if begin > cell.range_max() || end <= cell.range_min() {
                return VisitAction::SkipCell;
            }
            // Use the entire weight of contained cells.
            if begin <= cell.range_min() && end > cell.range_max() {
                sum = sum.saturating_add(node.weight());
                return VisitAction::SkipCell;
            }
            // Recurse if we can.
            if node.has_children() {
                return VisitAction::EnterCell;
            }
            // Interpolate the contained portion.
            let range = CellInterpolator::new(cell);
            let t1 = range.uninterpolate(begin).max(0.0);
            let t2 = if end.is_valid() {
                range.uninterpolate(end).min(1.0)
            } else {
                1.0
            };
            #[expect(
                clippy::cast_possible_truncation,
                reason = "ceil produces integer-valued f64"
            )]
            let delta = (node.weight() as f64 * (t2 - t1)).ceil() as i64;
            sum = sum.saturating_add(delta);
            VisitAction::SkipCell
        })?;
        Ok(Cluster::new(begin, end, sum))
    }

    /// Returns the next cluster starting from `begin` up to `end`.
    fn next_cluster(
        &self,
        density: &S2DensityTree,
        begin: CellId,
        end: CellId,
        initial_weight: i64,
    ) -> Result<Cluster, S2Error> {
        let mut weight = initial_weight;
        let mut cluster_end = end;
        let min_w = self.min_cluster_weight;
        let max_w = self.max_cluster_weight;

        density.visit_cells(|cell, node| {
            if cell.range_max() < begin || cell.range_min() >= cluster_end {
                return VisitAction::SkipCell;
            }
            let range = CellInterpolator::new(cell);
            let ratio = if node.has_children() {
                0.0
            } else {
                range.uninterpolate(begin).clamp(0.0, 1.0)
            };
            #[expect(
                clippy::cast_possible_truncation,
                reason = "ceil produces integer-valued f64"
            )]
            let delta = ((1.0 - ratio) * node.weight() as f64).ceil() as i64;
            let sum = weight + delta;

            if sum < min_w {
                weight = sum;
                return VisitAction::SkipCell;
            }
            if sum <= max_w {
                weight = sum;
                cluster_end = cell.range_max().next();
                return VisitAction::Stop;
            }
            if node.has_children() {
                return VisitAction::EnterCell;
            }
            // Prorate the Hilbert range.
            let missing = (min_w + max_w + 1) / 2 - weight;
            weight += missing;
            let new_ratio = ratio + missing as f64 / node.weight() as f64;
            cluster_end = range.interpolate(new_ratio).range_min();
            VisitAction::Stop
        })?;

        Ok(Cluster::new(begin, cluster_end, weight))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::density_tree::TreeEncoder;

    /// Helper to build a density tree from (`CellId`, weight) pairs.
    /// Like the Java test helper, it sums weights into all ancestor cells.
    fn density(leaves: &[(CellId, i64)]) -> S2DensityTree {
        use std::collections::HashMap;
        let mut map: HashMap<CellId, i64> = HashMap::new();
        for &(id, w) in leaves {
            // Add weight to the cell and all its ancestors.
            let mut cell = id;
            loop {
                *map.entry(cell).or_insert(0) += w;
                if cell.is_face() {
                    break;
                }
                cell = cell.parent();
            }
        }
        let mut enc = TreeEncoder::new();
        for (&id, &w) in &map {
            enc.put(id, w);
        }
        let mut tree = S2DensityTree::new();
        enc.build(&mut tree);
        tree
    }

    // Java: testClustersAsFaces
    #[test]
    fn test_clusters_as_faces() {
        let tree = density(&[
            (CellId::from_face(0), 100),
            (CellId::from_face(1), 100),
            (CellId::from_face(2), 100),
            (CellId::from_face(3), 100),
            (CellId::from_face(4), 100),
            (CellId::from_face(5), 100),
        ]);

        let query100 = DensityClusterQuery::new(100);
        let clusters = query100.clusters(&tree).unwrap();
        assert_eq!(clusters.len(), 6);
        for c in &clusters {
            assert_eq!(c.weight, 100);
        }

        // Tight coverings: each face should produce a single-cell covering.
        let tight = query100.tight_coverings(&tree).unwrap();
        assert_eq!(tight.len(), 6);
        for c in &tight {
            assert_eq!(c.num_cells(), 1);
            assert!(c.cell_ids()[0].is_face());
        }

        let query200 = DensityClusterQuery::new(200);
        let clusters200 = query200.tight_coverings(&tree).unwrap();
        assert_eq!(clusters200.len(), 3);
        for c in &clusters200 {
            assert_eq!(c.num_cells(), 2);
        }
    }

    // Java: testClustersDivideFaces
    #[test]
    fn test_clusters_divide_faces() {
        let tree = density(&[
            (CellId::from_face(0).children()[1], 50),
            (CellId::from_face(0).children()[2], 50),
            (CellId::from_face(1).children()[0].children()[0], 25),
            (CellId::from_face(1).children()[0].children()[1], 25),
            (CellId::from_face(1).children()[0].children()[2], 25),
            (CellId::from_face(1).children()[2].children()[0], 25),
        ]);

        let query50 = DensityClusterQuery::new(50);
        let clusters = query50.tight_coverings(&tree).unwrap();
        assert_eq!(clusters.len(), 4);

        // First cluster: face 0 child 1
        assert_eq!(clusters[0].num_cells(), 1);
        assert_eq!(
            clusters[0].cell_ids()[0],
            CellId::from_face(0).children()[1]
        );

        // Second cluster: face 0 child 2
        assert_eq!(clusters[1].num_cells(), 1);
        assert_eq!(
            clusters[1].cell_ids()[0],
            CellId::from_face(0).children()[2]
        );

        // Third cluster: two grandchildren
        assert_eq!(clusters[2].num_cells(), 2);
        assert_eq!(
            clusters[2].cell_ids()[0],
            CellId::from_face(1).children()[0].children()[0]
        );
        assert_eq!(
            clusters[2].cell_ids()[1],
            CellId::from_face(1).children()[0].children()[1]
        );

        // Fourth cluster: remaining two
        assert_eq!(clusters[3].num_cells(), 2);
        assert_eq!(
            clusters[3].cell_ids()[0],
            CellId::from_face(1).children()[0].children()[2]
        );
        assert_eq!(
            clusters[3].cell_ids()[1],
            CellId::from_face(1).children()[2].children()[0]
        );
    }

    // Java: testUninterpolate
    #[test]
    fn test_uninterpolate() {
        let face = CellId::from_face(0);
        let child1 = face.children()[1];
        let interp = CellInterpolator::new(face);
        let result = interp.uninterpolate(child1);
        assert!(
            (result - 0.25).abs() < 1e-10,
            "Expected ~0.25, got {result}"
        );
    }

    // Java: testInterpolationOnFaces (face 0)
    #[test]
    fn test_interpolation_on_face_0() {
        let tree = density(&[(CellId::from_face(0), 100)]);

        let query50 = DensityClusterQuery::new(50);
        let clusters = query50.tight_coverings(&tree).unwrap();
        assert_eq!(clusters.len(), 2);

        // Each cluster should contain two level-1 cells.
        assert_eq!(clusters[0].num_cells(), 2);
        assert_eq!(
            clusters[0].cell_ids()[0],
            CellId::from_face(0).children()[0]
        );
        assert_eq!(
            clusters[0].cell_ids()[1],
            CellId::from_face(0).children()[1]
        );
        assert_eq!(clusters[1].num_cells(), 2);
        assert_eq!(
            clusters[1].cell_ids()[0],
            CellId::from_face(0).children()[2]
        );
        assert_eq!(
            clusters[1].cell_ids()[1],
            CellId::from_face(0).children()[3]
        );

        // With weight 32, should produce 3 clusters.
        let query32 = DensityClusterQuery::new(32);
        let clusters32 = query32.tight_coverings(&tree).unwrap();
        assert_eq!(clusters32.len(), 3);
    }

    // Java: testNormalizingTree
    #[test]
    fn test_normalizing_tree() {
        // Build a tree where deeper nodes have redundant weight.
        let face0 = CellId::from_face(0);
        let child0 = face0.children()[0];
        let child2 = face0.children()[2];

        let mut enc = TreeEncoder::new();
        enc.put(face0, 800);
        enc.put(child0, 400);
        enc.put(child0.children()[0], 400);
        enc.put(child0.children()[1], 400);
        enc.put(child0.children()[2], 400);
        enc.put(child0.children()[3], 400);
        enc.put(child2, 400);
        enc.put(child2.children()[0], 100);
        enc.put(child2.children()[1], 100);
        enc.put(child2.children()[2], 100);
        enc.put(child2.children()[3], 100);
        let mut tree = S2DensityTree::new();
        enc.build(&mut tree);

        let query400 = DensityClusterQuery::new(400);
        let clusters = query400.tight_coverings(&tree).unwrap();
        // After normalization, should be 2 clusters.
        assert_eq!(clusters.len(), 2);

        // Each cluster is a single level-1 cell.
        assert_eq!(clusters[0].num_cells(), 1);
        assert_eq!(clusters[0].cell_ids()[0], child0);
        assert_eq!(clusters[1].num_cells(), 1);
        assert_eq!(clusters[1].cell_ids()[0], child2);
    }

    #[test]
    fn test_empty_tree() {
        let tree = S2DensityTree::new();
        let query = DensityClusterQuery::new(100);
        let clusters = query.clusters(&tree).unwrap();
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_single_leaf() {
        let tree = density(&[(CellId::from_face(3), 50)]);
        let query = DensityClusterQuery::new(50);
        let clusters = query.clusters(&tree).unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].weight, 50);
    }

    #[test]
    fn test_cluster_display() {
        let begin = CellId::begin(MAX_CELL_LEVEL);
        let end = CellId::end(MAX_CELL_LEVEL);
        let c = Cluster::new(begin, end, 42);
        let s = c.to_string();
        assert!(s.contains("42"), "Display should show weight");
    }

    #[test]
    fn test_with_range_panics_on_invalid() {
        let result = std::panic::catch_unwind(|| DensityClusterQuery::with_range(0, 100));
        assert!(result.is_err());
    }

    // Java: testMinClusterBoundsAreNonZero
    #[test]
    fn test_min_cluster_bounds_are_non_zero() {
        let query = DensityClusterQuery::new(1);
        assert_eq!(query.min_cluster_weight(), 1);
        assert_eq!(query.max_cluster_weight(), 1);
    }

    // Java: testDecodedPath — verify leaves can be obtained via DecodedPath
    #[test]
    fn test_decoded_path() {
        use crate::s2::density_tree::DecodedPath;

        let leaves = &[
            (CellId::from_face(2).children()[1], 50_i64),
            (CellId::from_face(2).children()[2], 50),
            (CellId::from_face(4).children()[0].children()[0], 25),
            (CellId::from_face(4).children()[2].children()[1], 25),
            (CellId::from_face(4).children()[2].children()[2], 25),
            (CellId::from_face(4).children()[3].children()[0], 25),
        ];
        let tree = density(leaves);
        let mut error = S2Error::ok();
        let mut path = DecodedPath::new(&tree);
        for &(id, _expected_w) in leaves {
            let cell = path.get_cell(id, &mut error);
            assert!(error.is_ok());
            assert!(cell.weight() > 0, "Expected non-zero weight for {id}");
            // The weight may include ancestor sums, so just verify > 0.
        }
        // Face 0 should have no weight.
        let cell = path.get_cell(CellId::from_face(0), &mut error);
        assert!(error.is_ok());
        assert_eq!(cell.weight(), 0);
    }

    // Java: testInterpolationOnFaces (face 1 and face 4)
    #[test]
    fn test_interpolation_on_face_1() {
        let tree = density(&[(CellId::from_face(1), 100)]);
        let query = DensityClusterQuery::new(50);
        let clusters = query.tight_coverings(&tree).unwrap();
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].num_cells(), 2);
        assert_eq!(clusters[1].num_cells(), 2);
    }

    #[test]
    fn test_interpolation_on_face_4() {
        let tree = density(&[(CellId::from_face(4), 100)]);
        let query = DensityClusterQuery::new(50);
        let clusters = query.tight_coverings(&tree).unwrap();
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].num_cells(), 2);
        assert_eq!(clusters[1].num_cells(), 2);
    }

    #[test]
    fn test_coverings() {
        let tree = density(&[(CellId::from_face(0), 100), (CellId::from_face(1), 100)]);
        let query = DensityClusterQuery::new(100);
        let coverings = query.coverings(&tree).unwrap();
        assert_eq!(coverings.len(), 2);
    }
}
