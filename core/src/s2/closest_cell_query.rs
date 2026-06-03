// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry

#![expect(
    clippy::cast_sign_loss,
    reason = "max_results (i32) used as Vec capacity"
)]
// S2ClosestCellQuery: find closest cell(s) in an S2CellIndex.
//
// Given a set of (CellId, label) pairs stored in an S2CellIndex, provides
// methods to find the closest cells to a given target (point, edge, cell).
//
// C++ ref: s2closest_cell_query.h, s2closest_cell_query_base.h

use std::collections::{BTreeSet, BinaryHeap, HashSet};

use crate::s1::ChordAngle;
use crate::s2::cell_index::{CellIndex, CellIndexContentsIterator, CellIndexRangeIterator};
use crate::s2::coords::Level;
use crate::s2::distance_target::DistanceTarget;
use crate::s2::region::Region;
use crate::s2::{Cap, Cell, CellId, Point};

// ─── Result ──────────────────────────────────────────────────────────────────

/// A result from a closest cell query.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Result {
    /// Distance from the target to this cell.
    pub distance: ChordAngle,
    /// The `CellId`.
    pub cell_id: CellId,
    /// The label associated with this `CellId`.
    pub label: i32,
}

impl Result {
    fn empty() -> Self {
        Result {
            distance: ChordAngle::INFINITY,
            cell_id: CellId::none(),
            label: -1,
        }
    }

    /// Returns true if this result is empty (no cell found).
    pub fn is_empty(&self) -> bool {
        self.cell_id == CellId::none()
    }
}

impl PartialEq for Result {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
            && self.cell_id == other.cell_id
            && self.label == other.label
    }
}

impl Eq for Result {}

impl PartialOrd for Result {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Result {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance
            .length2()
            .partial_cmp(&other.distance.length2())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.cell_id.cmp(&other.cell_id))
            .then_with(|| self.label.cmp(&other.label))
    }
}

// ─── Options ─────────────────────────────────────────────────────────────────

/// Options for a closest cell query.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Maximum number of results to return.
    pub max_results: i32,
    /// Maximum distance to any result.
    pub max_distance: ChordAngle,
    /// Maximum additional error allowed for distance computation.
    pub max_error: ChordAngle,
    /// If true, always use brute-force search.
    pub use_brute_force: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_results: i32::MAX,
            max_distance: ChordAngle::INFINITY,
            max_error: ChordAngle::ZERO,
            use_brute_force: false,
        }
    }
}

impl Options {
    /// Sets `max_distance` so that cells at exactly `limit` are also
    /// returned. Equivalent to `limit.successor()`.
    pub fn inclusive_max_distance(&mut self, limit: ChordAngle) {
        self.max_distance = limit.successor();
    }

    /// Sets `max_distance` so that all cells whose true distance is ≤
    /// `limit` are returned, accounting for maximum distance computation
    /// error. Matches C++ `set_conservative_max_distance`.
    pub fn conservative_max_distance(&mut self, limit: ChordAngle) {
        use crate::s2::edge_distances;
        self.max_distance = limit
            .plus_error(edge_distances::update_min_distance_max_error(limit))
            .successor();
    }
}

// ─── Target ──────────────────────────────────────────────────────────────────

/// Target geometry to measure distance to.
/// The target geometry to measure distance to.
///
/// Extends [`DistanceTarget`] with
/// methods specific to closest-cell queries.
pub trait Target: DistanceTarget {
    /// Updates the minimum distance to `cell`; returns the new distance and
    /// whether it was updated.
    fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool);

    /// Maximum index size for which brute force is faster than an indexed
    /// search. The default is 200; subtypes override with values tuned
    /// to their geometry.
    fn max_brute_force_index_size(&self) -> i32 {
        200
    }
}

/// Target: closest cells to a point.
#[derive(Debug)]
pub struct PointTarget {
    point: Point,
}

impl PointTarget {
    /// Creates a target for finding closest cells to `point`.
    pub fn new(point: Point) -> Self {
        PointTarget { point }
    }
}

impl DistanceTarget for PointTarget {
    fn cap_bound(&self) -> Cap {
        Cap::from_point(self.point)
    }
}

impl Target for PointTarget {
    fn max_brute_force_index_size(&self) -> i32 {
        9 // C++ benchmark: 18/16 FindClosest, 8/9 IsDistanceLess
    }

    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = cell.distance_to_point(self.point);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
}

/// Target: closest cells to an edge.
#[derive(Debug)]
pub struct EdgeTarget {
    a: Point,
    b: Point,
}

impl EdgeTarget {
    /// Creates a target for finding closest cells to edge `(a, b)`.
    pub fn new(a: Point, b: Point) -> Self {
        EdgeTarget { a, b }
    }
}

impl DistanceTarget for EdgeTarget {
    fn cap_bound(&self) -> Cap {
        Cap::from_point(Point((self.a.0 + self.b.0).normalize()))
            .expanded(self.a.chord_angle(self.b).to_angle() * 0.5)
    }
}

impl Target for EdgeTarget {
    fn max_brute_force_index_size(&self) -> i32 {
        5 // C++ benchmark: 14/16 FindClosest, 5/5 IsDistanceLess
    }

    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = cell.distance_to_edge(self.a, self.b);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
}

/// Target: closest cells to a cell.
#[derive(Debug)]
pub struct CellTarget {
    cell: Cell,
}

impl CellTarget {
    /// Creates a target for finding closest cells to `cell`.
    pub fn new(cell: Cell) -> Self {
        CellTarget { cell }
    }
}

impl DistanceTarget for CellTarget {
    fn cap_bound(&self) -> Cap {
        self.cell.cap_bound()
    }
}

impl Target for CellTarget {
    fn max_brute_force_index_size(&self) -> i32 {
        6 // C++ benchmark: 12/13 FindClosest, 6/6 IsDistanceLess
    }

    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = self.cell.distance_to_cell(*cell);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
}

/// Target: closest cells to a cell union.
///
/// Wraps each cell in the `CellUnion` into a `CellIndex` and uses
/// an internal `ClosestCellQuery` to find the closest cell. Matches
/// C++ `S2ClosestCellQuery::CellUnionTarget`.
#[derive(Debug)]
pub struct CellUnionTarget {
    cell_union: crate::s2::cell_union::CellUnion,
    index: CellIndex,
    use_brute_force: bool,
}

impl CellUnionTarget {
    /// Creates a target from a `CellUnion`.
    pub fn new(cell_union: crate::s2::cell_union::CellUnion) -> Self {
        let mut index = CellIndex::new();
        for &cell_id in cell_union.cell_ids() {
            index.add(cell_id, 0);
        }
        index.build();
        CellUnionTarget {
            cell_union,
            index,
            use_brute_force: false,
        }
    }

    /// Sets whether the internal query should use brute force.
    pub fn set_use_brute_force(&mut self, use_brute_force: bool) {
        self.use_brute_force = use_brute_force;
    }
}

impl DistanceTarget for CellUnionTarget {
    fn cap_bound(&self) -> Cap {
        use crate::s2::region::Region as _;
        self.cell_union.cap_bound()
    }
    fn set_max_error(&mut self, max_error: ChordAngle) -> bool {
        // Error is passed to internal query per-call via Options.
        let _ = max_error;
        true
    }
}

impl Target for CellUnionTarget {
    fn max_brute_force_index_size(&self) -> i32 {
        8 // C++ benchmark value
    }

    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let opts = Options {
            max_results: 1,
            max_distance: min_dist,
            use_brute_force: self.use_brute_force,
            ..Options::default()
        };
        let query = ClosestCellQuery::new(&self.index, opts);
        let mut target = CellTarget::new(*cell);
        let result = query.find_closest_cell(&mut target);
        if result.is_empty() {
            (min_dist, false)
        } else {
            (result.distance, true)
        }
    }
}

/// Target: closest cells to the shapes in a `ShapeIndex`.
///
/// Uses an internal `ClosestEdgeQuery` to find the closest edge in
/// the target index, then reports its distance. Matches C++
/// `S2ClosestCellQuery::ShapeIndexTarget`.
#[derive(Debug)]
pub struct ShapeIndexTarget<'a> {
    index: &'a crate::s2::shape_index::ShapeIndex,
    /// Whether to include polygon interiors in the target.
    pub include_interiors: bool,
    /// Whether the internal query should use brute force.
    pub use_brute_force: bool,
}

impl<'a> ShapeIndexTarget<'a> {
    /// Creates a target from a `ShapeIndex`.
    pub fn new(index: &'a crate::s2::shape_index::ShapeIndex) -> Self {
        ShapeIndexTarget {
            index,
            include_interiors: true,
            use_brute_force: false,
        }
    }
}

impl DistanceTarget for ShapeIndexTarget<'_> {
    fn cap_bound(&self) -> Cap {
        use crate::s2::region::Region as _;
        use crate::s2::shape_index_region::ShapeIndexRegion;
        let region = ShapeIndexRegion::new(self.index);
        region.cap_bound()
    }
    fn set_max_error(&mut self, _max_error: ChordAngle) -> bool {
        true
    }
}

impl Target for ShapeIndexTarget<'_> {
    fn max_brute_force_index_size(&self) -> i32 {
        7 // C++ benchmark value
    }

    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        use crate::s2::closest_edge_query::{
            CellTarget as CECellTarget, ClosestEdgeQuery, Options as CEOptions,
        };
        let query = ClosestEdgeQuery::new(self.index);
        let target = CECellTarget::new(*cell);
        let opts = CEOptions {
            max_results: 1,
            max_distance: min_dist,
            include_interiors: self.include_interiors,
            use_brute_force: self.use_brute_force,
            ..CEOptions::default()
        };
        let result = query.find_closest_edge_with_options(&target, &opts);
        if result.is_empty() {
            (min_dist, false)
        } else {
            (result.distance, true)
        }
    }
}

// ─── Query ───────────────────────────────────────────────────────────────────

const MIN_RANGES_TO_ENQUEUE: usize = 6;

#[derive(Clone, Debug)]
struct QueueEntry {
    distance: ChordAngle,
    id: CellId,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}
impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is max-heap; reverse for min-distance.
        other
            .distance
            .length2()
            .partial_cmp(&self.distance.length2())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Finds the closest cell(s) in an `S2CellIndex` to a given target.
///
/// # Examples
///
/// ```
/// use s2rst::s2::closest_cell_query::{ClosestCellQuery, Options, PointTarget};
/// use s2rst::s2::cell_index::CellIndex;
/// use s2rst::s2::{CellId, LatLng};
///
/// let mut index = CellIndex::new();
/// let id = CellId::from_point(&LatLng::from_degrees(0.0, 0.0).to_point())
///     .parent_at_level(10);
/// index.add(id, 42);
/// index.build();
///
/// let query = ClosestCellQuery::new(&index, Options::default());
/// let mut target = PointTarget::new(LatLng::from_degrees(0.0, 0.0).to_point());
/// let result = query.find_closest_cell(&mut target);
/// assert!(!result.is_empty());
/// ```
#[derive(Debug)]
pub struct ClosestCellQuery<'a> {
    index: &'a CellIndex,
    options: Options,
}

impl<'a> ClosestCellQuery<'a> {
    /// Creates a new query over the given index with the given options.
    pub fn new(index: &'a CellIndex, options: Options) -> Self {
        ClosestCellQuery { index, options }
    }

    /// Returns a mutable reference to the options.
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    /// Returns the options.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Returns the closest cells to the target, sorted by distance.
    pub fn find_closest_cells(&self, target: &mut dyn Target) -> Vec<Result> {
        self.find_closest_cells_in_region(target, None)
    }

    /// Like [`find_closest_cells`](Self::find_closest_cells) but only
    /// returns cells that intersect the given region. Matches C++
    /// `Options::set_region()`.
    pub fn find_closest_cells_in_region(
        &self,
        target: &mut dyn Target,
        region: Option<&dyn Region>,
    ) -> Vec<Result> {
        debug_assert!(self.options.max_results >= 1, "max_results must be >= 1");
        let mut state = QueryState::new(self.index, &self.options, target, region);
        state.find();
        state.collect_results()
    }

    /// Returns the single closest cell to the target.
    pub fn find_closest_cell(&self, target: &mut dyn Target) -> Result {
        let mut opts = self.options.clone();
        opts.max_results = 1;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state.result_singleton.unwrap_or_else(Result::empty)
    }

    /// Returns the distance to the closest cell.
    pub fn get_distance(&self, target: &mut dyn Target) -> ChordAngle {
        self.find_closest_cell(target).distance
    }

    /// Returns true if the distance to any cell is less than `limit`.
    pub fn is_distance_less(&self, target: &mut dyn Target, limit: ChordAngle) -> bool {
        let mut opts = self.options.clone();
        opts.max_results = 1;
        opts.max_distance = limit;
        opts.max_error = ChordAngle::STRAIGHT;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state.result_singleton.is_some()
    }

    /// Returns true if the distance to any cell is at most `limit`.
    pub fn is_distance_less_or_equal(&self, target: &mut dyn Target, limit: ChordAngle) -> bool {
        let mut opts = self.options.clone();
        opts.max_results = 1;
        opts.max_distance = limit.successor();
        opts.max_error = ChordAngle::STRAIGHT;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state.result_singleton.is_some()
    }

    /// Like [`is_distance_less_or_equal`](Self::is_distance_less_or_equal)
    /// but `limit` is increased by the maximum error in distance
    /// computation, ensuring all truly-within-limit cells are found.
    /// Matches C++ `IsConservativeDistanceLessOrEqual`.
    pub fn is_conservative_distance_less_or_equal(
        &self,
        target: &mut dyn Target,
        limit: ChordAngle,
    ) -> bool {
        let mut opts = self.options.clone();
        opts.max_results = 1;
        opts.conservative_max_distance(limit);
        opts.max_error = ChordAngle::STRAIGHT;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state.result_singleton.is_some()
    }
}

struct QueryState<'a> {
    index: &'a CellIndex,
    options: &'a Options,
    target: &'a mut dyn Target,
    distance_limit: ChordAngle,
    use_conservative_cell_distance: bool,
    avoid_duplicates: bool,
    tested_cells: HashSet<(CellId, i32)>,
    /// Optional region filter: only cells intersecting this region are returned.
    region: Option<&'a dyn Region>,

    result_singleton: Option<Result>,
    result_vector: Vec<Result>,
    result_set: BTreeSet<Result>,
}

impl<'a> QueryState<'a> {
    fn new(
        index: &'a CellIndex,
        options: &'a Options,
        target: &'a mut dyn Target,
        region: Option<&'a dyn Region>,
    ) -> Self {
        let distance_limit = options.max_distance;

        let target_uses_max_error =
            options.max_error > ChordAngle::ZERO && target.set_max_error(options.max_error);

        let use_conservative_cell_distance = target_uses_max_error
            && (distance_limit == ChordAngle::INFINITY || {
                let reduced = ChordAngle::from_length2(
                    (distance_limit.length2() - options.max_error.length2()).max(0.0),
                );
                reduced > ChordAngle::ZERO
            });

        QueryState {
            index,
            options,
            target,
            distance_limit,
            use_conservative_cell_distance,
            avoid_duplicates: target_uses_max_error && options.max_results > 1,
            tested_cells: HashSet::new(),
            region,
            result_singleton: None,
            result_vector: Vec::new(),
            result_set: BTreeSet::new(),
        }
    }

    fn find(&mut self) {
        if self.distance_limit == ChordAngle::ZERO {
            return;
        }

        if self.options.use_brute_force {
            self.avoid_duplicates = false;
            self.find_brute_force();
        } else {
            self.find_optimized();
        }
    }

    fn find_brute_force(&mut self) {
        let mut range_iter = CellIndexRangeIterator::new(self.index);
        let mut contents_iter = CellIndexContentsIterator::new(self.index);
        range_iter.begin();
        while !range_iter.done() {
            if !range_iter.is_empty() {
                contents_iter.start_union(&range_iter);
                while !contents_iter.done() {
                    self.maybe_add_result(contents_iter.cell_id(), contents_iter.label());
                    contents_iter.next();
                }
            }
            range_iter.next();
        }
    }

    fn find_optimized(&mut self) {
        let mut queue: BinaryHeap<QueueEntry> = BinaryHeap::new();
        let mut contents_iter = CellIndexContentsIterator::new(self.index);

        // For max_results==1, try adjacent ranges first.
        if self.options.max_results == 1 {
            let cap = self.target.cap_bound();
            if !cap.is_empty() {
                let target_id = CellId::from_point(&cap.center());
                let mut range = CellIndexRangeIterator::new_non_empty(self.index);
                range.seek(target_id);
                if !range.done() {
                    self.add_range(&range, &mut contents_iter);
                    if self.distance_limit == ChordAngle::ZERO {
                        return;
                    }
                }
                if range.start_id() > target_id && range.prev() {
                    self.add_range(&range, &mut contents_iter);
                    if self.distance_limit == ChordAngle::ZERO {
                        return;
                    }
                }
            }
        }

        // Build index covering.
        let index_covering = self.build_index_covering();

        // Intersect with distance limit if finite.
        let initial_cells = if self.distance_limit < ChordAngle::INFINITY {
            let cap = self.target.cap_bound();
            if cap.is_empty() {
                return;
            }
            let search_radius = cap.chord_radius().to_angle() + self.distance_limit.to_angle();
            let search_cap = Cap::from_center_angle(cap.center(), search_radius);
            let search_covering = self.get_fast_covering(&search_cap);
            intersect_sorted(&index_covering, &search_covering)
        } else {
            index_covering.clone()
        };

        // Process initial cells.
        let mut range = CellIndexRangeIterator::new_non_empty(self.index);
        range.begin();
        for (i, &id) in initial_cells.iter().enumerate() {
            if range.done() {
                break;
            }
            let seek = i == 0 || id.range_min() >= range.limit_id();
            self.process_or_enqueue(id, &mut range, &mut queue, &mut contents_iter, seek);
        }

        // Process priority queue.
        while let Some(entry) = queue.pop() {
            if entry.distance >= self.distance_limit {
                break;
            }
            let mut child = entry.id.child_begin();
            let mut range = CellIndexRangeIterator::new_non_empty(self.index);
            let mut seek = true;
            for _ in 0..4 {
                let did_enqueue = self.process_or_enqueue(
                    child,
                    &mut range,
                    &mut queue,
                    &mut contents_iter,
                    seek,
                );
                seek = did_enqueue;
                child = child.next();
            }
        }
    }

    fn add_range(
        &mut self,
        range: &CellIndexRangeIterator,
        contents_iter: &mut CellIndexContentsIterator,
    ) {
        contents_iter.start_union(range);
        while !contents_iter.done() {
            self.maybe_add_result(contents_iter.cell_id(), contents_iter.label());
            contents_iter.next();
        }
    }

    fn build_index_covering(&self) -> Vec<CellId> {
        let mut covering = Vec::with_capacity(6);
        let mut it = CellIndexRangeIterator::new_non_empty(self.index);
        let mut last = CellIndexRangeIterator::new_non_empty(self.index);
        it.begin();
        last.finish();
        if !last.prev() {
            return covering; // Empty.
        }
        let last_id = last.limit_id().prev();
        if it.start_id() != last.start_id() {
            let level = it
                .start_id()
                .common_ancestor_level(last_id)
                .unwrap_or(Level::MIN)
                + 1u8;
            let start_parent = it.start_id().parent_at_level(level);
            let last_parent = last_id.parent_at_level(level);
            let mut id = start_parent;
            while id != last_parent {
                if id.range_max() < it.start_id() {
                    id = id.next();
                    continue;
                }
                let cell_first = it.start_id();
                it.seek(id.range_max().next());
                // Find the last non-empty range before this position.
                let mut prev = CellIndexRangeIterator::new_non_empty(self.index);
                prev.seek(it.start_id());
                if !prev.prev() {
                    // Shouldn't happen if there are ranges in [cell_first..id.range_max()]
                    id = id.next();
                    continue;
                }
                let cell_last = prev.limit_id().prev();
                let anc = cell_first
                    .common_ancestor_level(cell_last)
                    .unwrap_or(Level::MIN);
                covering.push(cell_first.parent_at_level(anc));
                id = id.next();
            }
        }
        let anc = it
            .start_id()
            .common_ancestor_level(last_id)
            .unwrap_or(Level::MIN);
        covering.push(it.start_id().parent_at_level(anc));
        covering
    }

    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn get_fast_covering(&self, cap: &Cap) -> Vec<CellId> {
        use crate::s2::region_coverer::RegionCoverer;
        let coverer = RegionCoverer::new().max_cells(4);
        coverer.fast_covering(cap).cell_ids().to_vec()
    }

    fn process_or_enqueue(
        &mut self,
        id: CellId,
        range: &mut CellIndexRangeIterator,
        queue: &mut BinaryHeap<QueueEntry>,
        contents_iter: &mut CellIndexContentsIterator,
        seek: bool,
    ) -> bool {
        if seek {
            range.seek(id.range_min());
        }
        let last = id.range_max();
        if range.done() || range.start_id() > last {
            return false;
        }

        // Check if there are enough ranges to enqueue.
        let mut max_it = CellIndexRangeIterator::new(self.index);
        max_it.seek(range.start_id());
        if max_it.advance(MIN_RANGES_TO_ENQUEUE - 1) && max_it.start_id() <= last {
            // Enqueue this cell.
            let cell = Cell::from(id);
            let (dist, updated) = self
                .target
                .update_distance_to_cell(&cell, self.distance_limit);
            // We check "region" second because it may be relatively expensive.
            if updated && self.region.is_none_or(|r| r.intersects_cell(&cell)) {
                let entry_dist = if self.use_conservative_cell_distance {
                    ChordAngle::from_length2(
                        (dist.length2() - self.options.max_error.length2()).max(0.0),
                    )
                } else {
                    dist
                };
                queue.push(QueueEntry {
                    distance: entry_dist,
                    id,
                });
            }
            // Advance range past this cell.
            range.seek(last.next());
            return true;
        }

        // Process ranges directly.
        while !range.done() && range.start_id() <= last {
            self.add_range(range, contents_iter);
            range.next();
        }
        false
    }

    fn maybe_add_result(&mut self, cell_id: CellId, label: i32) {
        if self.avoid_duplicates && !self.tested_cells.insert((cell_id, label)) {
            return;
        }

        let cell = Cell::from(cell_id);
        let (dist, updated) = self
            .target
            .update_distance_to_cell(&cell, self.distance_limit);
        if !updated {
            return;
        }

        // Region filter: check "region" second because it may be expensive.
        // C++: MayIntersect is used to filter cells.
        if let Some(region) = self.region
            && !region.intersects_cell(&cell)
        {
            return;
        }

        let result = Result {
            distance: dist,
            cell_id,
            label,
        };

        if self.options.max_results == 1 {
            self.result_singleton = Some(result);
            self.distance_limit = ChordAngle::from_length2(
                (dist.length2() - self.options.max_error.length2()).max(0.0),
            );
        } else if self.options.max_results == i32::MAX {
            self.result_vector.push(result);
        } else {
            self.result_set.insert(result.clone());
            if self.result_set.len() > self.options.max_results as usize {
                // Remove the furthest result.
                if let Some(last) = self.result_set.iter().next_back().cloned() {
                    self.result_set.remove(&last);
                }
            }
            if self.result_set.len() >= self.options.max_results as usize
                && let Some(last) = self.result_set.iter().next_back()
            {
                self.distance_limit = ChordAngle::from_length2(
                    (last.distance.length2() - self.options.max_error.length2()).max(0.0),
                );
            }
        }
    }

    fn collect_results(&mut self) -> Vec<Result> {
        if self.options.max_results == 1 {
            match self.result_singleton.take() {
                Some(r) => vec![r],
                None => vec![],
            }
        } else if self.options.max_results == i32::MAX {
            let mut results = std::mem::take(&mut self.result_vector);
            results.sort_unstable();
            results.dedup();
            results
        } else {
            let set = std::mem::take(&mut self.result_set);
            set.into_iter().collect()
        }
    }
}

fn intersect_sorted(a: &[CellId], b: &[CellId]) -> Vec<CellId> {
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].range_max() < b[j].range_min() {
            i += 1;
        } else if b[j].range_max() < a[i].range_min() {
            j += 1;
        } else if a[i].contains(b[j]) {
            result.push(b[j]);
            j += 1;
        } else {
            // b[j] contains a[i], or partial overlap: keep the smaller cell.
            result.push(a[i]);
            i += 1;
        }
    }
    result
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[expect(
    clippy::field_reassign_with_default,
    reason = "clearer than a single struct literal with many fields"
)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn make_index(cells: &[(CellId, i32)]) -> CellIndex {
        let mut index = CellIndex::new();
        for &(id, label) in cells {
            index.add(id, label);
        }
        index.build();
        index
    }

    #[test]
    fn test_no_cells() {
        let index = make_index(&[]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let mut target = PointTarget::new(Point::from_coords(1.0, 0.0, 0.0));
        let results = query.find_closest_cells(&mut target);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_single_cell() {
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(10);
        let index = make_index(&[(cell_id, 0)]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let mut target = PointTarget::new(p);
        let result = query.find_closest_cell(&mut target);
        assert!(!result.is_empty());
        assert_eq!(result.cell_id, cell_id);
        assert_eq!(result.label, 0);
        assert!(result.distance.to_angle().degrees() < 0.01);
    }

    #[test]
    fn test_max_distance() {
        let p1 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(10.0, 0.0).to_point();
        let cell1 = CellId::from_point(&p1).parent_at_level(10);
        let cell2 = CellId::from_point(&p2).parent_at_level(10);
        let index = make_index(&[(cell1, 0), (cell2, 1)]);
        let mut opts = Options::default();
        opts.max_distance = ChordAngle::from_angle(crate::s1::Angle::from_degrees(5.0));
        let query = ClosestCellQuery::new(&index, opts);
        let mut target = PointTarget::new(p1);
        let results = query.find_closest_cells(&mut target);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, 0);
    }

    #[test]
    fn test_max_results() {
        let mut cells = Vec::new();
        for i in 0..20 {
            let p = LatLng::from_degrees(f64::from(i) * 0.5, 0.0).to_point();
            cells.push((CellId::from_point(&p).parent_at_level(10), i));
        }
        let index = make_index(&cells);
        let mut opts = Options::default();
        opts.max_results = 3;
        let query = ClosestCellQuery::new(&index, opts);
        let mut target = PointTarget::new(LatLng::from_degrees(0.0, 0.0).to_point());
        let results = query.find_closest_cells(&mut target);
        assert_eq!(results.len(), 3);
        for i in 1..results.len() {
            assert!(results[i].distance >= results[i - 1].distance);
        }
    }

    #[test]
    fn test_get_distance() {
        let p = LatLng::from_degrees(1.0, 0.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(15);
        let index = make_index(&[(cell_id, 0)]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(2.0, 0.0).to_point());
        let dist = query.get_distance(&mut target);
        // Distance from (2,0) to a cell near (1,0) should be about 1 degree.
        assert!((dist.to_angle().degrees() - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_is_distance_less() {
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(10);
        let index = make_index(&[(cell_id, 0)]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(1.0, 0.0).to_point());
        assert!(query.is_distance_less(
            &mut target,
            ChordAngle::from_angle(crate::s1::Angle::from_degrees(2.0))
        ));
        assert!(!query.is_distance_less(
            &mut target,
            ChordAngle::from_angle(crate::s1::Angle::from_degrees(0.5))
        ));
    }

    #[test]
    fn test_edge_target() {
        let p = LatLng::from_degrees(0.0, 1.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(10);
        let index = make_index(&[(cell_id, 0)]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let a = LatLng::from_degrees(0.0, 0.0).to_point();
        let b = LatLng::from_degrees(0.0, 2.0).to_point();
        let mut target = EdgeTarget::new(a, b);
        let dist = query.get_distance(&mut target);
        assert!(dist.to_angle().degrees() < 0.1);
    }

    #[test]
    fn test_cell_target() {
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(10);
        let index = make_index(&[(cell_id, 0)]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let target_cell = Cell::from(CellId::from_point(&p).parent_at_level(5));
        let mut target = CellTarget::new(target_cell);
        let dist = query.get_distance(&mut target);
        // Cell at level 10 is inside cell at level 5 → distance 0.
        assert!(dist.to_angle().degrees() < 0.01);
    }

    #[test]
    fn test_multiple_labels_same_cell() {
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(10);
        let index = make_index(&[(cell_id, 0), (cell_id, 1), (cell_id, 2)]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let mut target = PointTarget::new(p);
        let results = query.find_closest_cells(&mut target);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_brute_force_matches_optimized() {
        let mut cells = Vec::new();
        for i in 0..50 {
            let lat = -25.0 + f64::from(i);
            let p = LatLng::from_degrees(lat, 0.0).to_point();
            cells.push((CellId::from_point(&p).parent_at_level(10), i));
        }
        let index = make_index(&cells);
        let target_point = LatLng::from_degrees(0.0, 0.0).to_point();

        let mut opts_bf = Options::default();
        opts_bf.max_results = 5;
        opts_bf.use_brute_force = true;
        let query_bf = ClosestCellQuery::new(&index, opts_bf);
        let mut target_bf = PointTarget::new(target_point);
        let results_bf = query_bf.find_closest_cells(&mut target_bf);

        let mut opts_opt = Options::default();
        opts_opt.max_results = 5;
        let query_opt = ClosestCellQuery::new(&index, opts_opt);
        let mut target_opt = PointTarget::new(target_point);
        let results_opt = query_opt.find_closest_cells(&mut target_opt);

        assert_eq!(results_bf.len(), results_opt.len());
        for (bf, opt) in results_bf.iter().zip(results_opt.iter()) {
            assert!(
                (bf.distance.length2() - opt.distance.length2()).abs() < 1e-15,
                "distance mismatch"
            );
        }
    }

    #[test]
    fn test_options_not_modified() {
        // C++ OptionsNotModified: verify that FindClosestCell/GetDistance/
        // IsDistanceLess do not modify query.options().
        let p1 = LatLng::from_degrees(1.0, 1.0).to_point();
        let p2 = LatLng::from_degrees(1.0, 2.0).to_point();
        let p3 = LatLng::from_degrees(1.0, 3.0).to_point();
        let index = make_index(&[
            (CellId::from_point(&p1), 1),
            (CellId::from_point(&p2), 2),
            (CellId::from_point(&p3), 3),
        ]);
        let mut opts = Options::default();
        opts.max_results = 3;
        opts.max_distance = ChordAngle::from_angle(crate::s1::Angle::from_degrees(3.0));
        opts.max_error = ChordAngle::from_angle(crate::s1::Angle::from_degrees(0.001));
        let query = ClosestCellQuery::new(&index, opts.clone());
        let mut target = PointTarget::new(LatLng::from_degrees(2.0, 2.0).to_point());
        let r = query.find_closest_cell(&mut target);
        assert_eq!(r.label, 2);
        let dist = query.get_distance(&mut target);
        assert!((dist.to_angle().degrees() - 1.0).abs() < 0.1);
        assert!(query.is_distance_less(
            &mut target,
            ChordAngle::from_angle(crate::s1::Angle::from_degrees(1.5))
        ));
        // Options unchanged.
        assert_eq!(query.options().max_results, opts.max_results);
        assert_eq!(query.options().max_distance, opts.max_distance);
        assert_eq!(query.options().max_error, opts.max_error);
    }

    #[test]
    fn test_distance_equal_to_limit() {
        // C++ DistanceEqualToLimit: test boundary behavior of distance
        // predicates when distance exactly equals the limit.
        let p0 = LatLng::from_degrees(23.0, 12.0).to_point();
        let p1 = LatLng::from_degrees(47.0, 11.0).to_point();
        let id0 = CellId::from_point(&p0);
        let index = make_index(&[(id0, 0)]);
        let query = ClosestCellQuery::new(&index, Options::default());

        // Same cell → distance 0.
        let mut target0 = CellTarget::new(Cell::from(id0));
        let dist0 = ChordAngle::ZERO;
        assert!(!query.is_distance_less(&mut target0, dist0));
        assert!(query.is_distance_less_or_equal(&mut target0, dist0));
        assert!(query.is_conservative_distance_less_or_equal(&mut target0, dist0));

        // Different cell → non-zero distance.
        let id1 = CellId::from_point(&p1);
        let mut target1 = CellTarget::new(Cell::from(id1));
        let dist1 = Cell::from(id0).distance_to_cell(Cell::from(id1));
        assert!(!query.is_distance_less(&mut target1, dist1));
        assert!(query.is_distance_less_or_equal(&mut target1, dist1));
        assert!(query.is_conservative_distance_less_or_equal(&mut target1, dist1));
    }

    #[test]
    fn test_target_point_inside_indexed_cell() {
        // C++ TargetPointInsideIndexedCell.
        let cell_id = CellId::from_face(4).children()[0].children()[1].children()[2];
        let index = make_index(&[(cell_id, 1)]);
        let query = ClosestCellQuery::new(&index, Options::default());
        let mut target = PointTarget::new(cell_id.to_point());
        let result = query.find_closest_cell(&mut target);
        assert_eq!(result.distance, ChordAngle::ZERO);
        assert_eq!(result.cell_id, cell_id);
        assert_eq!(result.label, 1);
    }

    #[test]
    fn test_inclusive_max_distance() {
        let mut opts = Options::default();
        opts.inclusive_max_distance(ChordAngle::from_angle(crate::s1::Angle::from_degrees(5.0)));
        assert!(opts.max_distance > ChordAngle::from_angle(crate::s1::Angle::from_degrees(5.0)));
    }

    #[test]
    fn test_conservative_max_distance() {
        let mut opts = Options::default();
        opts.conservative_max_distance(ChordAngle::from_angle(crate::s1::Angle::from_degrees(5.0)));
        let mut inclusive = Options::default();
        inclusive
            .inclusive_max_distance(ChordAngle::from_angle(crate::s1::Angle::from_degrees(5.0)));
        assert!(opts.max_distance >= inclusive.max_distance);
    }

    #[test]
    fn test_cell_union_target() {
        // Create a CellIndex with one cell near (0,0) and one near (10,0).
        let p1 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(10.0, 0.0).to_point();
        let cell1 = CellId::from_point(&p1).parent_at_level(10);
        let cell2 = CellId::from_point(&p2).parent_at_level(10);
        let index = make_index(&[(cell1, 0), (cell2, 1)]);

        // Create a CellUnion target near (0,0).
        let target_cell = CellId::from_point(&p1).parent_at_level(5);
        let cu = crate::s2::cell_union::CellUnion::from_cell_ids(vec![target_cell]);
        let mut target = CellUnionTarget::new(cu);

        let query = ClosestCellQuery::new(&index, Options::default());
        let result = query.find_closest_cell(&mut target);
        assert!(!result.is_empty());
        assert_eq!(result.cell_id, cell1);
        assert_eq!(result.label, 0);
        // Cell1 is inside the CellUnion target → distance ~0.
        assert!(result.distance.to_angle().degrees() < 0.1);
    }

    #[test]
    fn test_cell_union_target_max_distance() {
        let p1 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(20.0, 0.0).to_point();
        let cell1 = CellId::from_point(&p1).parent_at_level(10);
        let cell2 = CellId::from_point(&p2).parent_at_level(10);
        let index = make_index(&[(cell1, 0), (cell2, 1)]);

        // CellUnion near (0,0), max_distance < distance to cell2.
        let target_cell = CellId::from_point(&p1).parent_at_level(5);
        let cu = crate::s2::cell_union::CellUnion::from_cell_ids(vec![target_cell]);
        let mut target = CellUnionTarget::new(cu);
        let mut opts = Options::default();
        opts.max_distance = ChordAngle::from_angle(crate::s1::Angle::from_degrees(5.0));
        let query = ClosestCellQuery::new(&index, opts);
        let results = query.find_closest_cells(&mut target);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, 0);
    }

    #[test]
    fn test_shape_index_target() {
        // Create a CellIndex with cells at various latitudes.
        let mut cells = Vec::new();
        for i in 0..10 {
            let p = LatLng::from_degrees(f64::from(i) * 2.0, 0.0).to_point();
            cells.push((CellId::from_point(&p).parent_at_level(10), i));
        }
        let index = make_index(&cells);

        // ShapeIndex target: a polygon near (0,0).
        use crate::s2::shape_index::ShapeIndex;
        use crate::s2::text_format;
        let poly = text_format::make_polygon("-1:-1, -1:1, 1:1, 1:-1");
        let mut target_index = ShapeIndex::new();
        target_index.add(Box::new(poly));
        target_index.build();

        let mut target = ShapeIndexTarget::new(&target_index);
        let query = ClosestCellQuery::new(&index, Options::default());
        let result = query.find_closest_cell(&mut target);
        assert!(!result.is_empty());
        // The closest cell should be the one near (0,0).
        assert_eq!(result.label, 0);
        assert!(result.distance.to_angle().degrees() < 1.5);
    }

    #[test]
    fn test_shape_index_target_include_interiors() {
        // When include_interiors is true, a cell whose center is inside a
        // polygon in the target ShapeIndex has distance 0.
        //
        // Use a very small cell so the cell center ≈ cell boundary.
        let p = LatLng::from_degrees(5.0, 5.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(28);
        let index = make_index(&[(cell_id, 42)]);

        // Polygon that contains (5,5).
        use crate::s2::shape_index::ShapeIndex;
        use crate::s2::text_format;
        let poly = text_format::make_polygon("0:0, 0:10, 10:10, 10:0");
        let mut target_index = ShapeIndex::new();
        target_index.add(Box::new(poly));
        target_index.build();

        // With include_interiors = true, the cell is inside the polygon.
        let mut target_with = ShapeIndexTarget::new(&target_index);
        target_with.include_interiors = true;
        let query = ClosestCellQuery::new(&index, Options::default());
        let result_with = query.find_closest_cell(&mut target_with);
        assert!(!result_with.is_empty());
        assert_eq!(result_with.distance, ChordAngle::ZERO);

        // With include_interiors = false, distance is to the polygon boundary.
        let mut target_without = ShapeIndexTarget::new(&target_index);
        target_without.include_interiors = false;
        let result_without = query.find_closest_cell(&mut target_without);
        assert!(!result_without.is_empty());
        assert!(result_without.distance > ChordAngle::ZERO);
    }

    #[test]
    fn test_region_filter() {
        // Create cells at several latitudes.
        let mut cells = Vec::new();
        for i in 0..10 {
            let p = LatLng::from_degrees(f64::from(i) * 5.0, 0.0).to_point();
            cells.push((CellId::from_point(&p).parent_at_level(10), i));
        }
        let index = make_index(&cells);

        // Region: a cap around (0,0) with radius 10 degrees.
        let center = LatLng::from_degrees(0.0, 0.0).to_point();
        let region = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(10.0));

        let query = ClosestCellQuery::new(&index, Options::default());
        // Target far away — but region restricts results to cells near (0,0).
        let far_point = LatLng::from_degrees(40.0, 0.0).to_point();
        let mut target = PointTarget::new(far_point);

        // Without region: closest cell is at 40 degrees (label 8 = 40/5).
        let results_no_region = query.find_closest_cells(&mut target);
        assert!(!results_no_region.is_empty());

        // With region: only cells inside the cap (lat < 10°) are returned.
        let results_with_region = query.find_closest_cells_in_region(&mut target, Some(&region));
        for r in &results_with_region {
            let cell = Cell::from(r.cell_id);
            assert!(
                region.intersects_cell(&cell),
                "result cell should intersect the region"
            );
        }
        // Labels 0,1 (lat 0,5) should be in the region; label 8 (lat 40) should not.
        assert!(
            results_with_region.len() <= 3,
            "region should restrict results: got {} results",
            results_with_region.len()
        );
    }

    #[test]
    fn test_region_filter_empty() {
        // Region that doesn't intersect any indexed cells → empty results.
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        let cell_id = CellId::from_point(&p).parent_at_level(10);
        let index = make_index(&[(cell_id, 0)]);

        let far_center = LatLng::from_degrees(80.0, 0.0).to_point();
        let region = Cap::from_center_angle(far_center, crate::s1::Angle::from_degrees(1.0));

        let query = ClosestCellQuery::new(&index, Options::default());
        let mut target = PointTarget::new(far_center);
        let results = query.find_closest_cells_in_region(&mut target, Some(&region));
        assert!(
            results.is_empty(),
            "no cells should be in the distant region"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_result_roundtrip() {
        let r = Result {
            distance: ChordAngle::from_degrees(10.0),
            cell_id: CellId::from_face(crate::s2::coords::Face::F0),
            label: 42,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Result = serde_json::from_str(&json).unwrap();
        assert_eq!(r.distance, back.distance);
        assert_eq!(r.cell_id, back.cell_id);
        assert_eq!(r.label, back.label);
    }
}
