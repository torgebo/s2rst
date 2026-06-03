// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! [`RegionTermIndexer`] converts spatial regions into string index/query
//! terms for spatial database applications.
//!
//! This enables spatial queries on top of key-value stores, inverted indexes,
//! or any system that supports exact string matching. Regions are decomposed
//! into two types of terms:
//!
//! - **Ancestor terms**: Match documents contained within or equal to the query
//!   region (for small documents relative to the query).
//! - **Covering terms**: Match documents that contain the query region (for
//!   large documents relative to the query).
//!
//! Corresponds to C++ `s2region_term_indexer.h/cc`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "level_mod (usize->u8) — always <= MAX_CELL_LEVEL"
)]
use crate::s2::coords::Level;
use crate::s2::region_coverer::RegionCoverer;
use crate::s2::{CellId, CellUnion, Point, Region};

/// Options for the [`RegionTermIndexer`].
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Maximum number of cells in the region covering (default: 8).
    pub max_cells: usize,
    /// Minimum cell level for coverings (default: 4, ~600km).
    pub min_level: Level,
    /// Maximum cell level for coverings (default: 16, ~150m).
    pub max_level: Level,
    /// Level modifier for cell levels (default: 1).
    pub level_mod: usize,
    /// If true, optimize for point-only indexes (skips covering terms).
    pub index_contains_points_only: bool,
    /// If true, trade index size for query speed (~1.3x improvement).
    pub optimize_for_space: bool,
    /// Marker character separating term type from cell token (default: '$').
    pub marker_character: char,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_cells: 8,
            min_level: Level::new(4),
            max_level: Level::new(16),
            level_mod: 1,
            index_contains_points_only: false,
            optimize_for_space: false,
            marker_character: '$',
        }
    }
}

/// Converts spatial regions into string terms for database indexing and querying.
///
/// # Usage
///
/// To index a document with a spatial region:
/// 1. Call `get_index_terms()` for the document's region
/// 2. Store each returned term in your inverted index, associated with the document
///
/// To query for documents intersecting a spatial region:
/// 1. Call `get_query_terms()` for the query region
/// 2. Look up each term in your inverted index
/// 3. Union the results
#[derive(Debug)]
pub struct RegionTermIndexer {
    options: Options,
}

impl RegionTermIndexer {
    /// Creates a new indexer with default options.
    pub fn new() -> Self {
        RegionTermIndexer {
            options: Options::default(),
        }
    }

    /// Creates a new indexer with the given options.
    pub fn with_options(options: Options) -> Self {
        RegionTermIndexer { options }
    }

    /// Returns a reference to the current options.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Returns a mutable reference to the options.
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    /// Returns the true maximum level, accounting for `level_mod`.
    fn true_max_level(&self) -> Level {
        let opts = &self.options;
        if opts.level_mod == 1 {
            opts.max_level
        } else {
            // Round down to nearest level_mod boundary.
            let range = (opts.max_level - opts.min_level) as usize;
            let adjusted = range - (range % opts.level_mod);
            opts.min_level + adjusted as u8
        }
    }

    /// Generates index terms for a region.
    ///
    /// These terms should be stored in the database for each document.
    pub fn get_index_terms(&self, region: &dyn Region) -> Vec<String> {
        let covering = self.get_covering(region);
        self.get_index_terms_for_covering(&covering)
    }

    /// Generates index terms for a point.
    pub fn get_index_terms_for_point(&self, point: Point) -> Vec<String> {
        let cell_id = CellId::from_point(&point);
        let level = self.true_max_level();
        let id = cell_id.parent_at_level(level);
        let mut terms = Vec::new();

        // Ancestor terms: the cell and all its ancestors down to min_level.
        let mut current = id;
        while current.level() >= self.options.min_level {
            terms.push(self.ancestor_term(current));
            if current.level() == 0 {
                break;
            }
            current = current.parent();
        }

        // Covering term (unless point-only).
        if !self.options.index_contains_points_only {
            terms.push(self.covering_term(id));
        }

        terms
    }

    /// Generates query terms for a region.
    ///
    /// Look up each term in the database and union the results.
    pub fn get_query_terms(&self, region: &dyn Region) -> Vec<String> {
        let covering = self.get_covering(region);
        self.get_query_terms_for_covering(&covering)
    }

    /// Generates query terms for a point.
    pub fn get_query_terms_for_point(&self, point: Point) -> Vec<String> {
        let cell_id = CellId::from_point(&point);
        let level = self.true_max_level();
        let id = cell_id.parent_at_level(level);
        let mut terms = Vec::new();

        // Ancestor term at the point's level.
        terms.push(self.ancestor_term(id));

        // Covering terms for all ancestors.
        let mut current = id;
        while current.level() >= self.options.min_level {
            terms.push(self.covering_term(current));
            if current.level() == 0 {
                break;
            }
            current = current.parent();
        }

        terms
    }

    /// Generates index terms from a pre-computed canonical cell covering.
    pub fn get_index_terms_for_covering(&self, covering: &CellUnion) -> Vec<String> {
        let true_max = self.true_max_level();
        let mut terms = Vec::new();

        for &id in covering.cell_ids() {
            let level = id.level();
            debug_assert!(level >= self.options.min_level);
            debug_assert!(level <= self.options.max_level);
            debug_assert_eq!(
                0,
                (level - self.options.min_level) % self.options.level_mod as u8
            );
            // Ancestor terms: the cell and all ancestors.
            let mut current = id;
            while current.level() >= self.options.min_level {
                if !self.options.optimize_for_space || current == id {
                    terms.push(self.ancestor_term(current));
                }
                if current.level() == 0 {
                    break;
                }
                current = current.parent();
            }

            // Covering term (unless at true_max_level or point-only).
            if !self.options.index_contains_points_only && id.level() < true_max {
                terms.push(self.covering_term(id));
            }
        }

        terms.sort_unstable();
        terms.dedup();
        terms
    }

    /// Generates query terms from a pre-computed canonical cell covering.
    pub fn get_query_terms_for_covering(&self, covering: &CellUnion) -> Vec<String> {
        let _true_max = self.true_max_level();
        let mut terms = Vec::new();

        for &id in covering.cell_ids() {
            let level = id.level();
            debug_assert!(level >= self.options.min_level);
            debug_assert!(level <= self.options.max_level);
            debug_assert_eq!(
                0,
                (level - self.options.min_level) % self.options.level_mod as u8
            );
            // Ancestor term at this cell's level.
            terms.push(self.ancestor_term(id));

            // Covering terms for all ancestors.
            if !self.options.index_contains_points_only {
                let mut current = id;
                while current.level() >= self.options.min_level {
                    terms.push(self.covering_term(current));
                    if current.level() == 0 {
                        break;
                    }
                    current = current.parent();
                }
            }

            // If optimizing for space, add covering terms for proper ancestors.
            if self.options.optimize_for_space {
                let mut current = id;
                while current.level() > self.options.min_level {
                    current = current.parent();
                    terms.push(self.ancestor_term(current));
                }
            }
        }

        terms.sort_unstable();
        terms.dedup();
        terms
    }

    fn get_covering(&self, region: &dyn Region) -> CellUnion {
        let coverer = RegionCoverer::new()
            .max_cells(self.options.max_cells)
            .min_level(self.options.min_level)
            .max_level(self.options.max_level)
            .level_mod(self.options.level_mod as u8);
        coverer.covering(region)
    }

    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn ancestor_term(&self, id: CellId) -> String {
        id.to_token()
    }

    fn covering_term(&self, id: CellId) -> String {
        format!("{}{}", self.options.marker_character, id.to_token())
    }
}

impl Default for RegionTermIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::cap::Cap;
    use crate::s2::text_format;

    #[test]
    fn test_default_options() {
        let indexer = RegionTermIndexer::new();
        assert_eq!(indexer.options().max_cells, 8);
        assert_eq!(indexer.options().min_level, 4);
        assert_eq!(indexer.options().max_level, 16);
        assert_eq!(indexer.options().marker_character, '$');
    }

    #[test]
    fn test_point_index_terms() {
        let indexer = RegionTermIndexer::new();
        let p = text_format::parse_point("0:0");
        let terms = indexer.get_index_terms_for_point(p);

        // Should have ancestor terms for each level from max_level down to min_level.
        assert!(!terms.is_empty());
        // Should contain at least one covering term (marker + token).
        let covering_count = terms.iter().filter(|t| t.starts_with('$')).count();
        assert!(covering_count >= 1, "should have covering terms");
    }

    #[test]
    fn test_point_only_index() {
        let opts = Options {
            index_contains_points_only: true,
            ..Options::default()
        };
        let indexer = RegionTermIndexer::with_options(opts);
        let p = text_format::parse_point("0:0");
        let terms = indexer.get_index_terms_for_point(p);

        // With points_only, should have no covering terms.
        let covering_count = terms.iter().filter(|t| t.starts_with('$')).count();
        assert_eq!(
            covering_count, 0,
            "points-only should have no covering terms"
        );
    }

    #[test]
    fn test_point_query_terms() {
        let indexer = RegionTermIndexer::new();
        let p = text_format::parse_point("0:0");
        let terms = indexer.get_query_terms_for_point(p);

        // Should have terms.
        assert!(!terms.is_empty());
    }

    #[test]
    fn test_region_index_and_query() {
        let indexer = RegionTermIndexer::new();
        let cap = Cap::from_center_angle(
            text_format::parse_point("0:0"),
            crate::s1::Angle::from_degrees(1.0),
        );

        let index_terms = indexer.get_index_terms(&cap);
        let query_terms = indexer.get_query_terms(&cap);

        assert!(!index_terms.is_empty());
        assert!(!query_terms.is_empty());

        // The index and query terms should have some overlap (the ancestor terms).
        let has_overlap = index_terms.iter().any(|t| query_terms.contains(t));
        assert!(has_overlap, "index and query terms should overlap");
    }

    #[test]
    fn test_covering_terms_contain_marker() {
        let indexer = RegionTermIndexer::new();
        let p = text_format::parse_point("45:45");
        let terms = indexer.get_index_terms_for_point(p);

        for term in &terms {
            if term.starts_with('$') {
                // Covering terms should start with marker + token.
                assert!(term.len() > 1, "covering term too short: {term}");
            } else {
                // Ancestor terms should be a valid cell token.
                assert!(!term.is_empty(), "ancestor term should not be empty");
            }
        }
    }

    #[test]
    fn test_custom_marker() {
        let opts = Options {
            marker_character: '#',
            ..Options::default()
        };
        let indexer = RegionTermIndexer::with_options(opts);
        let p = text_format::parse_point("0:0");
        let terms = indexer.get_index_terms_for_point(p);

        // Covering terms should use '#' marker.
        let has_hash = terms.iter().any(|t| t.starts_with('#'));
        assert!(has_hash, "should use custom marker '#'");
        let has_dollar = terms.iter().any(|t| t.starts_with('$'));
        assert!(!has_dollar, "should not use default marker '$'");
    }

    #[test]
    fn test_point_containment() {
        // Index a point, then query with a cap containing it.
        // The intersection of index_terms and query_terms should be non-empty.
        let indexer = RegionTermIndexer::new();
        let p = text_format::parse_point("10:20");
        let index_terms = indexer.get_index_terms_for_point(p);

        let cap = Cap::from_center_angle(p, crate::s1::Angle::from_degrees(5.0));
        let query_terms = indexer.get_query_terms(&cap);

        let intersection: Vec<_> = index_terms
            .iter()
            .filter(|t| query_terms.contains(t))
            .collect();
        assert!(
            !intersection.is_empty(),
            "point inside cap should have matching terms"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2region_term_indexer_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_marker_character() {
        // C++ TEST(S2RegionTermIndexer, MarkerCharacter)
        let opts = Options {
            min_level: Level::new(20),
            max_level: Level::new(20),
            ..Options::default()
        };
        let mut indexer = RegionTermIndexer::with_options(opts);

        let point = crate::s2::LatLng::from_degrees(10.0, 20.0).to_point();
        assert_eq!(indexer.options().marker_character, '$');
        let terms = indexer.get_query_terms_for_point(point);
        // Should have the cell-id and marked version.
        assert_eq!(2, terms.len());
        assert!(terms.iter().any(|t| t.starts_with('$')));

        indexer.options_mut().marker_character = ':';
        assert_eq!(indexer.options().marker_character, ':');
        let terms2 = indexer.get_query_terms_for_point(point);
        assert_eq!(2, terms2.len());
        assert!(terms2.iter().any(|t| t.starts_with(':')));
    }

    #[test]
    fn test_max_level_set_loosely() {
        // C++ TEST(S2RegionTermIndexer, MaxLevelSetLoosely)
        // Test that correct terms are generated even when (max_level - min_level)
        // is not a multiple of level_mod.
        let opts1 = Options {
            min_level: Level::new(1),
            level_mod: 2,
            max_level: Level::new(19),
            ..Options::default()
        };
        let indexer1 = RegionTermIndexer::with_options(opts1);

        let opts2 = Options {
            min_level: Level::new(1),
            level_mod: 2,
            max_level: Level::new(20),
            ..Options::default()
        };
        let indexer2 = RegionTermIndexer::with_options(opts2);

        // Use a deterministic point.
        let point = crate::s2::LatLng::from_degrees(37.7749, -122.4194).to_point();
        assert_eq!(
            indexer1.get_index_terms_for_point(point),
            indexer2.get_index_terms_for_point(point),
        );
        assert_eq!(
            indexer1.get_query_terms_for_point(point),
            indexer2.get_query_terms_for_point(point),
        );
    }
}
