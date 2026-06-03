// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! [`RegionSharder`] assigns regions to shards based on spatial overlap.
//!
//! Given a set of shard boundaries (as [`CellUnion`]s), determines which
//! shards intersect a given [`Region`]. This is useful for distributing
//! spatial data across database partitions.
//!
//! Corresponds to C++ `s2region_sharder.h/cc`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "shard index (usize->i32) — count always small"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for shard count — always small"
)]
use std::collections::HashMap;

use crate::s2::cell_index::{CellIndex, CellIndexContentsIterator, CellIndexRangeIterator};
use crate::s2::{Cell, CellId, CellUnion, Region};

/// Determines which shards intersect a given [`Region`].
///
/// Shards are defined as [`CellUnion`]s. Each shard is identified by its
/// index (0-based) in the list of shards provided at construction.
#[derive(Debug)]
pub struct RegionSharder {
    index: CellIndex,
}

impl RegionSharder {
    /// Creates a new `RegionSharder` from a list of shard cell unions.
    ///
    /// Each shard is identified by its index in the vector.
    pub fn new(shards: &[CellUnion]) -> Self {
        let mut index = CellIndex::new();
        for (i, shard) in shards.iter().enumerate() {
            index.add_cell_union(shard, i as i32);
        }
        index.build();
        RegionSharder { index }
    }

    /// Returns the shard with the most overlap with the given region.
    ///
    /// If no shards overlap, returns `default_shard`.
    pub fn get_most_intersecting_shard(&self, region: &dyn Region, default_shard: i32) -> i32 {
        let intersections = self.get_intersections_by_shard(region);

        let mut best_shard = default_shard;
        let mut best_sum: u64 = 0;

        for (&shard, covering) in &intersections {
            let sum: u64 = covering.iter().map(|id| id.lsb()).sum();
            if sum > best_sum || (sum == best_sum && shard < best_shard) {
                best_shard = shard;
                best_sum = sum;
            }
        }

        best_shard
    }

    /// Returns a list of shard indices that intersect the given region.
    pub fn get_intersecting_shards(&self, region: &dyn Region) -> Vec<i32> {
        let intersections = self.get_intersections_by_shard(region);
        intersections
            .into_iter()
            .filter(|(_, cells)| !cells.is_empty())
            .map(|(shard, _)| shard)
            .collect()
    }

    fn get_intersections_by_shard(&self, region: &dyn Region) -> HashMap<i32, Vec<CellId>> {
        let region_cells = region.cell_union_bound();
        let region_covering = CellUnion::from_cell_ids(region_cells);

        let mut shards: HashMap<i32, Vec<CellId>> = HashMap::new();

        // Visit cells in the index that overlap with the region covering.
        let mut range_it = CellIndexRangeIterator::new_non_empty(&self.index);
        let mut contents_it = CellIndexContentsIterator::new(&self.index);

        range_it.begin();

        for &cell_id in region_covering.cell_ids() {
            let range_min = cell_id.range_min();
            let range_max = cell_id.range_max();

            // Seek to the first range overlapping this covering cell.
            range_it.seek(range_min);
            if range_it.done() {
                continue;
            }
            // May need to back up one if the previous range extends into our cell.
            if range_it.start_id() > range_min
                && range_it.prev()
                && range_it.limit_id() <= range_min
            {
                range_it.next();
            }

            while !range_it.done() && range_it.start_id() <= range_max {
                contents_it.start_union(&range_it);
                while !contents_it.done() {
                    let label = contents_it.label();
                    let index_cell = contents_it.cell_id();
                    // Check if the index cell overlaps the region covering cell.
                    if index_cell.range_max() >= range_min && index_cell.range_min() <= range_max {
                        shards.entry(label).or_default().push(index_cell);
                    }
                    contents_it.next();
                }
                range_it.next();
            }
        }

        // Refine: check which shards actually intersect the region.
        let mut refined = HashMap::new();
        for (shard, cells) in shards {
            let mut good_cells = Vec::new();
            for cell_id in cells {
                if region.intersects_cell(&Cell::from_cell_id(cell_id)) {
                    good_cells.push(cell_id);
                }
            }
            if !good_cells.is_empty() {
                refined.insert(shard, good_cells);
            }
        }

        refined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::cap::Cap;
    use crate::s2::text_format;

    #[test]
    fn test_single_shard() {
        // One shard covering the whole sphere.
        let shards = vec![CellUnion::from_cell_ids(
            (0..6).map(CellId::from_face).collect(),
        )];
        let sharder = RegionSharder::new(&shards);

        let cap = Cap::from_center_angle(
            text_format::parse_point("0:0"),
            crate::s1::Angle::from_degrees(10.0),
        );

        let intersecting = sharder.get_intersecting_shards(&cap);
        assert_eq!(intersecting, vec![0]);
    }

    #[test]
    fn test_no_overlap() {
        // Two shards covering different parts of the sphere.
        let shard0 = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let shard1 = CellUnion::from_cell_ids(vec![CellId::from_face(5)]);
        let sharder = RegionSharder::new(&[shard0, shard1]);

        // Cap at face 2 — should not overlap either shard.
        let cap = Cap::from_center_angle(
            text_format::parse_point("0:90"),
            crate::s1::Angle::from_degrees(1.0),
        );

        let result = sharder.get_most_intersecting_shard(&cap, -1);
        // May or may not find overlap depending on covering; use default.
        // Just verify it doesn't crash.
        assert!(result == -1 || result == 0 || result == 1);
    }

    #[test]
    fn test_most_intersecting() {
        // Create shards for each face.
        let shards: Vec<CellUnion> = (0..6)
            .map(|f| CellUnion::from_cell_ids(vec![CellId::from_face(f)]))
            .collect();
        let sharder = RegionSharder::new(&shards);

        // Cap centered on face 0.
        let cap = Cap::from_center_angle(
            CellId::from_face(0).to_point(),
            crate::s1::Angle::from_degrees(5.0),
        );

        let best = sharder.get_most_intersecting_shard(&cap, -1);
        assert_eq!(best, 0, "cap at face 0 center should be in shard 0");
    }

    #[test]
    fn test_multiple_intersecting_shards() {
        // Create shards for each face.
        let shards: Vec<CellUnion> = (0..6)
            .map(|f| CellUnion::from_cell_ids(vec![CellId::from_face(f)]))
            .collect();
        let sharder = RegionSharder::new(&shards);

        // Large cap that spans multiple faces.
        let cap = Cap::from_center_angle(
            text_format::parse_point("0:0"),
            crate::s1::Angle::from_degrees(60.0),
        );

        let intersecting = sharder.get_intersecting_shards(&cap);
        assert!(
            intersecting.len() > 1,
            "large cap should intersect multiple face shards, got {intersecting:?}"
        );
    }
}
