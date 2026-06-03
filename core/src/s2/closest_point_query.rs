// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

#![expect(
    clippy::cast_sign_loss,
    reason = "max_results (i32) used as Vec capacity"
)]
#![cfg_attr(
    test,
    expect(
        clippy::cast_possible_truncation,
        reason = "max_results (i32) -> usize for Vec capacity and test index values"
    )
)]
#![cfg_attr(
    test,
    expect(
        clippy::cast_possible_wrap,
        reason = "usize -> i32 for test index values — always small"
    )
)]
// S2ClosestPointQuery: find closest point(s) in an S2PointIndex.
//
// Given a set of points stored in an S2PointIndex, provides methods to find
// the closest point(s) to a given target (point, edge, cell, etc.).
//
// C++ ref: s2closest_point_query.h, s2closest_point_query_base.h

use std::collections::BinaryHeap;

use crate::s1::ChordAngle;
use crate::s2::coords::Level;
use crate::s2::distance_target::DistanceTarget;
use crate::s2::edge_distances;
use crate::s2::point_index::{PointIndexIterator, S2PointIndex};
use crate::s2::region::Region;
use crate::s2::{Cap, Cell, CellId, Point};

// ─── Result ──────────────────────────────────────────────────────────────────

/// A result from a closest point query.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound(
        serialize = "D: serde::Serialize",
        deserialize = "D: serde::de::DeserializeOwned"
    ))
)]
pub struct Result<D: Clone> {
    /// The distance from the target to this point.
    pub distance: ChordAngle,
    /// The point itself.
    pub point: Point,
    /// The client-specified data associated with this point.
    pub data: D,
}

impl<D: Clone> Result<D> {
    fn empty_with_data(default: D) -> Self {
        Result {
            distance: ChordAngle::INFINITY,
            point: Point::origin(),
            data: default,
        }
    }

    /// Returns true if this result is empty (no point found).
    pub fn is_empty(&self) -> bool {
        self.distance == ChordAngle::INFINITY
    }
}

impl<D: Clone + PartialEq> PartialEq for Result<D> {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.point == other.point && self.data == other.data
    }
}

impl<D: Clone + PartialOrd> Eq for Result<D> {}

impl<D: Clone + PartialOrd> Ord for Result<D> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance
            .length2()
            .partial_cmp(&other.distance.length2())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl<D: Clone + PartialOrd> PartialOrd for Result<D> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ─── Options ─────────────────────────────────────────────────────────────────

/// Options controlling which points are returned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// Maximum number of results (default: `i32::MAX`).
    pub max_results: i32,
    /// Maximum distance for inclusion (default: infinity).
    pub max_distance: ChordAngle,
    /// Acceptable error — allows early termination.
    pub max_error: ChordAngle,
    /// Use brute force (for testing/benchmarking).
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
    /// Sets `max_distance` so that points at exactly `limit` are also
    /// returned. Equivalent to `limit.successor()`.
    pub fn inclusive_max_distance(&mut self, limit: ChordAngle) {
        self.max_distance = limit.successor();
    }

    /// Sets `max_distance` so that all points whose true distance is ≤
    /// `limit` are returned, accounting for maximum distance computation
    /// error. Matches C++ `set_conservative_max_distance`.
    pub fn conservative_max_distance(&mut self, limit: ChordAngle) {
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
/// methods specific to closest-point queries: distance to points and cells.
pub trait Target: DistanceTarget {
    /// Updates `dist_limit` if distance from `p` to target is less.
    /// Returns (`new_dist`, true) if updated, or (`dist_limit`, false).
    fn update_distance_to_point(&self, p: Point, dist_limit: ChordAngle) -> (ChordAngle, bool);

    /// Updates `dist_limit` if distance from cell to target is less.
    fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool);

    /// Maximum index size for which brute force is faster than an indexed
    /// search. The default is 200; subtypes override with values tuned
    /// to their geometry.
    fn max_brute_force_index_size(&self) -> i32 {
        200
    }
}

/// Target: find closest points to a given point.
#[derive(Debug)]
pub struct PointTarget {
    point: Point,
}

impl PointTarget {
    /// Creates a target from a point.
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
        // C++: ~150 for grid/fractal/regular geometry.
        150
    }

    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = p.chord_angle(self.point);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
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

/// Target: find closest points to a given edge.
#[derive(Debug)]
pub struct EdgeTarget {
    a: Point,
    b: Point,
}

impl EdgeTarget {
    /// Creates a target from an edge.
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
        // C++: ~100 for grid/fractal/regular geometry.
        100
    }

    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        edge_distances::update_min_distance(p, self.a, self.b, min_dist)
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

/// Target: find closest points to a given cell.
#[derive(Debug)]
pub struct CellTarget {
    cell: Cell,
}

impl CellTarget {
    /// Creates a target from a cell.
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
        50
    }

    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = self.cell.distance_to_point(p);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
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

/// Target: find closest points to any edge in a `ShapeIndex`.
///
/// This wraps a [`ClosestEdgeQuery`](crate::s2::closest_edge_query::ClosestEdgeQuery)
/// internally to measure point-to-index distance. Matches C++
/// `S2ClosestPointQueryShapeIndexTarget`.
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
        // C++: ~30 for grid/fractal/regular geometry.
        30
    }

    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        use crate::s2::closest_edge_query::{
            ClosestEdgeQuery, Options as CEOptions, PointTarget as CEPointTarget,
        };
        let query = ClosestEdgeQuery::new(self.index);
        let target = CEPointTarget::new(p);
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

/// The minimum number of points in a cell before we enqueue it rather
/// than processing its contents immediately.
const MIN_POINTS_TO_ENQUEUE: usize = 13;

/// A priority queue entry for the optimized algorithm.
#[derive(Clone, Debug)]
struct QueueEntry {
    /// Lower bound on distance from target to this cell.
    distance: ChordAngle,
    /// The cell to process.
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
        // BinaryHeap is max-heap; we want min-distance first, so reverse.
        other
            .distance
            .length2()
            .partial_cmp(&self.distance.length2())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Finds the closest point(s) in an `S2PointIndex` to a given target.
///
/// # Examples
///
/// ```
/// use s2rst::s2::closest_point_query::{ClosestPointQuery, Options, PointTarget};
/// use s2rst::s2::point_index::S2PointIndex;
/// use s2rst::s2::LatLng;
///
/// let mut index = S2PointIndex::new();
/// index.add(LatLng::from_degrees(0.0, 0.0).to_point(), 0u32);
/// index.add(LatLng::from_degrees(1.0, 0.0).to_point(), 1u32);
/// index.add(LatLng::from_degrees(2.0, 0.0).to_point(), 2u32);
///
/// let query = ClosestPointQuery::new(&index, Options::default());
/// let mut target = PointTarget::new(LatLng::from_degrees(0.5, 0.0).to_point());
/// let results = query.find_closest_points(&mut target);
/// assert!(!results.is_empty());
/// ```
#[derive(Debug)]
pub struct ClosestPointQuery<'a, D: Clone> {
    index: &'a S2PointIndex<D>,
    options: Options,
}

impl<'a, D: Clone + PartialOrd> ClosestPointQuery<'a, D> {
    /// Creates a new query over the given index.
    pub fn new(index: &'a S2PointIndex<D>, options: Options) -> Self {
        ClosestPointQuery { index, options }
    }

    /// Returns a mutable reference to the options.
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    /// Returns a reference to the options.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Finds the closest points to the target that satisfy the options.
    pub fn find_closest_points(&self, target: &mut dyn Target) -> Vec<Result<D>> {
        self.find_closest_points_in_region(target, None)
    }

    /// Like [`find_closest_points`](Self::find_closest_points) but only
    /// returns points that are contained by the given region. Matches
    /// C++ `Options::set_region()`.
    pub fn find_closest_points_in_region(
        &self,
        target: &mut dyn Target,
        region: Option<&dyn Region>,
    ) -> Vec<Result<D>> {
        debug_assert!(self.options.max_results >= 1, "max_results must be >= 1");
        let mut state = QueryState::new(self.index, &self.options, target, region);
        state.find();
        state.collect_results()
    }

    /// Returns the single closest point to the target.
    pub fn find_closest_point(&self, target: &mut dyn Target) -> Result<D>
    where
        D: Default,
    {
        let mut opts = self.options;
        opts.max_results = 1;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state
            .result_singleton
            .unwrap_or_else(|| Result::empty_with_data(D::default()))
    }

    /// Returns the minimum distance to the target.
    pub fn get_distance(&self, target: &mut dyn Target) -> ChordAngle
    where
        D: Default,
    {
        self.find_closest_point(target).distance
    }

    /// Returns true if the distance to the target is less than `limit`.
    pub fn is_distance_less(&self, target: &mut dyn Target, limit: ChordAngle) -> bool
    where
        D: Default,
    {
        let mut opts = self.options;
        opts.max_results = 1;
        opts.max_distance = limit;
        opts.max_error = ChordAngle::STRAIGHT;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state.result_singleton.is_some()
    }

    /// Returns true if the distance to the target is less than or equal to `limit`.
    pub fn is_distance_less_or_equal(&self, target: &mut dyn Target, limit: ChordAngle) -> bool
    where
        D: Default,
    {
        let mut opts = self.options;
        opts.max_results = 1;
        opts.max_distance = limit.successor();
        opts.max_error = ChordAngle::STRAIGHT;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state.result_singleton.is_some()
    }

    /// Like [`is_distance_less_or_equal`](Self::is_distance_less_or_equal)
    /// but `limit` is increased by the maximum error in distance
    /// computation, ensuring all truly-within-limit points are found.
    /// Matches C++ `IsConservativeDistanceLessOrEqual`.
    pub fn is_conservative_distance_less_or_equal(
        &self,
        target: &mut dyn Target,
        limit: ChordAngle,
    ) -> bool
    where
        D: Default,
    {
        let mut opts = self.options;
        opts.max_results = 1;
        opts.conservative_max_distance(limit);
        opts.max_error = ChordAngle::STRAIGHT;
        let mut state = QueryState::new(self.index, &opts, target, None);
        state.find();
        state.result_singleton.is_some()
    }
}

/// Internal query execution state.
struct QueryState<'a, D: Clone> {
    index: &'a S2PointIndex<D>,
    options: &'a Options,
    target: &'a mut dyn Target,
    /// Optional region filter: only points contained by this region are
    /// returned. Matches C++ `Options::region()`.
    region: Option<&'a dyn Region>,

    distance_limit: ChordAngle,
    use_conservative_cell_distance: bool,

    // Result storage (same three modes as C++):
    // 1. max_results == 1: result_singleton
    // 2. max_results == MAX: result_vector (sorted at end)
    // 3. Otherwise: result_set (priority queue, furthest on top)
    result_singleton: Option<Result<D>>,
    result_vector: Vec<Result<D>>,
    result_set: BinaryHeap<Result<D>>,
}

impl<'a, D: Clone + PartialOrd> QueryState<'a, D> {
    fn new(
        index: &'a S2PointIndex<D>,
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
            region,
            distance_limit,
            use_conservative_cell_distance,
            result_singleton: None,
            result_vector: Vec::new(),
            result_set: BinaryHeap::new(),
        }
    }

    fn find(&mut self) {
        if self.distance_limit == ChordAngle::ZERO {
            return;
        }

        if self.options.use_brute_force
            || self.target.max_brute_force_index_size() >= 0
                && self.index.num_points() <= self.target.max_brute_force_index_size() as usize
        {
            self.find_brute_force();
        } else {
            self.find_optimized();
        }
    }

    fn find_brute_force(&mut self) {
        let mut iter = self.index.iter();
        while !iter.done() {
            self.maybe_add_result(iter.point(), iter.data());
            iter.next();
        }
    }

    fn find_optimized(&mut self) {
        let mut iter = self.index.iter();
        let mut queue: BinaryHeap<QueueEntry> = BinaryHeap::new();

        // For max_results==1, try the adjacent points first as an optimization.
        if self.options.max_results == 1 {
            let cap = self.target.cap_bound();
            if !cap.is_empty() {
                iter.seek(CellId::from_point(&cap.center()));
                if !iter.done() {
                    self.maybe_add_result(iter.point(), iter.data());
                }
                if iter.prev() {
                    self.maybe_add_result(iter.point(), iter.data());
                }
                if self.distance_limit == ChordAngle::ZERO {
                    return;
                }
            }
        }

        // Build the index covering: a small set of CellIds that cover all
        // indexed points.
        let index_covering = self.build_index_covering(&mut iter);

        // Intersect with region if specified (C++: Options::region()).
        let after_region = if let Some(region) = self.region {
            use crate::s2::region_coverer::RegionCoverer;
            let coverer = RegionCoverer::new().max_cells(4);
            let cu = coverer.covering(region);
            let region_covering = cu.cell_ids();
            intersect_sorted(&index_covering, region_covering)
        } else {
            index_covering.clone()
        };

        // Intersect with distance limit if finite.
        let initial_cells = if self.distance_limit < ChordAngle::INFINITY {
            let cap = self.target.cap_bound();
            if cap.is_empty() {
                return;
            }
            let search_radius = cap.chord_radius().to_angle() + self.distance_limit.to_angle();
            let search_cap = Cap::from_center_angle(cap.center(), search_radius);
            let search_covering = self.get_fast_covering(&search_cap);
            intersect_sorted(&after_region, &search_covering)
        } else {
            after_region
        };

        // Process initial cells.
        iter.begin();
        for &id in &initial_cells {
            if iter.done() {
                break;
            }
            let seek = id.range_min() > iter.id();
            self.process_or_enqueue(id, &mut iter, &mut queue, seek);
        }

        // Process the priority queue.
        while let Some(entry) = queue.pop() {
            if entry.distance >= self.distance_limit {
                break;
            }
            let mut child = entry.id.child_begin();
            let mut seek = true;
            for _ in 0..4 {
                let did_enqueue = self.process_or_enqueue(child, &mut iter, &mut queue, seek);
                seek = did_enqueue;
                child = child.next();
            }
        }
    }

    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn build_index_covering(&self, iter: &mut PointIndexIterator<'_, D>) -> Vec<CellId> {
        let mut covering = Vec::with_capacity(6);
        iter.finish();
        if !iter.prev() {
            return covering; // Empty index.
        }
        let last_id = iter.id();
        iter.begin();
        let first_id = iter.id();

        if first_id != last_id {
            let level = first_id
                .common_ancestor_level(last_id)
                .unwrap_or(Level::MIN)
                + 1u8;
            let last_parent = last_id.parent_at_level(level);
            let mut id = first_id.parent_at_level(level);
            while id != last_parent {
                if id.range_max() < iter.id() {
                    id = id.next();
                    continue;
                }
                let cell_first = iter.id();
                iter.seek(id.range_max().next());
                iter.prev();
                let cell_last = iter.id();
                iter.next();
                let anc_level = cell_first
                    .common_ancestor_level(cell_last)
                    .unwrap_or(Level::MIN);
                covering.push(cell_first.parent_at_level(anc_level));
                id = id.next();
            }
        }
        // Add the last range.
        let anc_level = iter
            .id()
            .common_ancestor_level(last_id)
            .unwrap_or(Level::MIN);
        covering.push(iter.id().parent_at_level(anc_level));
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
        iter: &mut PointIndexIterator<'_, D>,
        queue: &mut BinaryHeap<QueueEntry>,
        seek: bool,
    ) -> bool {
        if seek {
            iter.seek(id.range_min());
        }

        if id.is_leaf() {
            while !iter.done() && iter.id() == id {
                self.maybe_add_result(iter.point(), iter.data());
                iter.next();
            }
            return false;
        }

        let last = id.range_max();
        let mut num_points = 0;
        let mut temp: Vec<(Point, D)> = Vec::new();
        while !iter.done() && iter.id() <= last {
            if num_points == MIN_POINTS_TO_ENQUEUE - 1 {
                // Too many points — enqueue this cell.
                let cell = Cell::from(id);
                let (dist, updated) = self
                    .target
                    .update_distance_to_cell(&cell, self.distance_limit);
                // C++: check "region_" second because it may be expensive.
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
                // Skip remaining points in this cell.
                iter.seek(last.next());
                return true;
            }
            temp.push((iter.point(), iter.data().clone()));
            num_points += 1;
            iter.next();
        }
        // Few enough points — process them directly.
        for (point, data) in &temp {
            self.maybe_add_result(*point, data);
        }
        false
    }

    fn maybe_add_result(&mut self, point: Point, data: &D) {
        let (dist, updated) = self
            .target
            .update_distance_to_point(point, self.distance_limit);
        if !updated {
            return;
        }

        // Region filter: only accept points contained by the region.
        // Matches C++ MaybeAddResult: "if (region && !region->Contains(point)) return;"
        if let Some(region) = self.region
            && !region.contains_point(&point)
        {
            return;
        }

        let result = Result {
            distance: dist,
            point,
            data: data.clone(),
        };

        if self.options.max_results == 1 {
            self.result_singleton = Some(result);
            self.distance_limit = ChordAngle::from_length2(
                (dist.length2() - self.options.max_error.length2()).max(0.0),
            );
        } else if self.options.max_results == i32::MAX {
            self.result_vector.push(result);
        } else {
            if self.result_set.len() >= self.options.max_results as usize {
                self.result_set.pop(); // Remove furthest.
            }
            self.result_set.push(result);
            if self.result_set.len() >= self.options.max_results as usize
                && let Some(top) = self.result_set.peek()
            {
                self.distance_limit = ChordAngle::from_length2(
                    (top.distance.length2() - self.options.max_error.length2()).max(0.0),
                );
            }
        }
    }

    fn collect_results(&mut self) -> Vec<Result<D>> {
        if self.options.max_results == 1 {
            match self.result_singleton.take() {
                Some(r) => vec![r],
                None => vec![],
            }
        } else if self.options.max_results == i32::MAX {
            let mut results = std::mem::take(&mut self.result_vector);
            results.sort();
            results.dedup();
            results
        } else {
            let mut results: Vec<Result<D>> = Vec::new();
            while let Some(r) = self.result_set.pop() {
                results.push(r);
            }
            results.reverse();
            results
        }
    }
}

/// Intersects two sorted `CellId` lists and returns cells that overlap.
fn intersect_sorted(a: &[CellId], b: &[CellId]) -> Vec<CellId> {
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].range_max() < b[j].range_min() {
            i += 1;
        } else if b[j].range_max() < a[i].range_min() {
            j += 1;
        } else {
            // a[i] and b[j] overlap. If one contains the other, use the smaller.
            if a[i].contains(b[j]) {
                result.push(b[j]);
                j += 1;
            } else if b[j].contains(a[i]) {
                result.push(a[i]);
                i += 1;
            } else {
                // Partial overlap — include both (rare, both get processed).
                result.push(a[i]);
                i += 1;
            }
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

    #[test]
    fn test_no_points() {
        let index = S2PointIndex::<i32>::new();
        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(Point::from_coords(1.0, 0.0, 0.0));
        let results = query.find_closest_points(&mut target);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_many_duplicate_points() {
        let mut index = S2PointIndex::new();
        let test_point = Point::from_coords(1.0, 0.0, 0.0);
        for i in 0..10000 {
            index.add(test_point, i);
        }
        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(test_point);
        let results = query.find_closest_points(&mut target);
        assert_eq!(results.len(), 10000);
    }

    #[test]
    fn test_find_closest_point() {
        let mut index = S2PointIndex::new();
        let p1 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(1.0, 0.0).to_point();
        let p3 = LatLng::from_degrees(2.0, 0.0).to_point();
        index.add(p1, 1);
        index.add(p2, 2);
        index.add(p3, 3);

        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(0.5, 0.0).to_point());
        let result = query.find_closest_point(&mut target);
        assert!(!result.is_empty());
        // Closest point should be either p1 (0,0) or p2 (1,0) — both at ~0.5 deg.
        assert!(result.data == 1 || result.data == 2);
    }

    #[test]
    fn test_max_distance() {
        let mut index = S2PointIndex::new();
        let p1 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(10.0, 0.0).to_point();
        let p3 = LatLng::from_degrees(20.0, 0.0).to_point();
        index.add(p1, 1);
        index.add(p2, 2);
        index.add(p3, 3);

        let mut opts = Options::default();
        opts.max_distance = ChordAngle::from_angle(Angle::from_degrees(5.0));
        let query = ClosestPointQuery::new(&index, opts);
        let mut target = PointTarget::new(LatLng::from_degrees(0.0, 0.0).to_point());
        let results = query.find_closest_points(&mut target);
        // Only p1 (0,0) should be within 5 degrees.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].data, 1);
    }

    #[test]
    fn test_max_results() {
        let mut index = S2PointIndex::new();
        for i in 0..100 {
            let p = LatLng::from_degrees(f64::from(i) * 0.1, 0.0).to_point();
            index.add(p, i);
        }
        let mut opts = Options::default();
        opts.max_results = 5;
        let query = ClosestPointQuery::new(&index, opts);
        let mut target = PointTarget::new(LatLng::from_degrees(0.0, 0.0).to_point());
        let results = query.find_closest_points(&mut target);
        assert_eq!(results.len(), 5);
        // Results should be sorted by distance.
        for i in 1..results.len() {
            assert!(results[i].distance >= results[i - 1].distance);
        }
    }

    #[test]
    fn test_get_distance() {
        let mut index = S2PointIndex::new();
        let p = LatLng::from_degrees(1.0, 0.0).to_point();
        index.add(p, 0);
        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(2.0, 0.0).to_point());
        let dist = query.get_distance(&mut target);
        assert!((dist.to_angle().degrees() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_is_distance_less() {
        let mut index = S2PointIndex::new();
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p, 0);
        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(1.0, 0.0).to_point());
        // Distance is ~1 degree
        assert!(query.is_distance_less(
            &mut target,
            ChordAngle::from_angle(Angle::from_degrees(2.0))
        ));
        assert!(!query.is_distance_less(
            &mut target,
            ChordAngle::from_angle(Angle::from_degrees(0.5))
        ));
    }

    #[test]
    fn test_edge_target() {
        let mut index = S2PointIndex::new();
        // Place a point at (0, 1).
        let p = LatLng::from_degrees(0.0, 1.0).to_point();
        index.add(p, 0);
        let query = ClosestPointQuery::new(&index, Options::default());
        // Edge from (0, 0) to (0, 2) — point should be very close to this edge.
        let a = LatLng::from_degrees(0.0, 0.0).to_point();
        let b = LatLng::from_degrees(0.0, 2.0).to_point();
        let mut target = EdgeTarget::new(a, b);
        let dist = query.get_distance(&mut target);
        assert!(dist.to_angle().degrees() < 0.01);
    }

    #[test]
    fn test_cell_target() {
        let mut index = S2PointIndex::new();
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p, 0);
        let query = ClosestPointQuery::new(&index, Options::default());
        let cell = Cell::from(CellId::from_point(&p).parent_at_level(10));
        let mut target = CellTarget::new(cell);
        let dist = query.get_distance(&mut target);
        // Point should be inside or very near the cell.
        assert!(dist.to_angle().degrees() < 0.1);
    }

    #[test]
    fn test_is_distance_less_or_equal() {
        let mut index = S2PointIndex::new();
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p, 0);
        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(1.0, 0.0).to_point());
        // Distance is ~1 degree. At exactly 1 degree it should return true.
        assert!(query.is_distance_less_or_equal(
            &mut target,
            ChordAngle::from_angle(Angle::from_degrees(1.1))
        ));
        assert!(!query.is_distance_less_or_equal(
            &mut target,
            ChordAngle::from_angle(Angle::from_degrees(0.5))
        ));
    }

    #[test]
    fn test_is_conservative_distance_less_or_equal() {
        let mut index = S2PointIndex::new();
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p, 0);
        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(1.0, 0.0).to_point());
        // Conservative check should be true for the actual distance.
        let dist = query.get_distance(&mut target);
        assert!(query.is_conservative_distance_less_or_equal(&mut target, dist));
    }

    #[test]
    fn test_inclusive_max_distance() {
        let mut opts = Options::default();
        opts.inclusive_max_distance(ChordAngle::from_angle(Angle::from_degrees(5.0)));
        // Inclusive should be strictly greater than the original.
        assert!(opts.max_distance > ChordAngle::from_angle(Angle::from_degrees(5.0)));
    }

    #[test]
    fn test_conservative_max_distance() {
        let mut opts = Options::default();
        opts.conservative_max_distance(ChordAngle::from_angle(Angle::from_degrees(5.0)));
        // Conservative should be >= inclusive.
        let mut inclusive = Options::default();
        inclusive.inclusive_max_distance(ChordAngle::from_angle(Angle::from_degrees(5.0)));
        assert!(opts.max_distance >= inclusive.max_distance);
    }

    #[test]
    fn test_brute_force_matches_optimized() {
        let mut index = S2PointIndex::new();
        // Add 500 points in a grid.
        for i in 0..25 {
            for j in 0..20 {
                let lat = -10.0 + f64::from(i) * 0.8;
                let lng = -10.0 + f64::from(j) * 1.0;
                let p = LatLng::from_degrees(lat, lng).to_point();
                index.add(p, i * 20 + j);
            }
        }

        let target_point = LatLng::from_degrees(0.0, 0.0).to_point();

        // Brute force.
        let mut opts_bf = Options::default();
        opts_bf.max_results = 5;
        opts_bf.use_brute_force = true;
        let query_bf = ClosestPointQuery::new(&index, opts_bf);
        let mut target_bf = PointTarget::new(target_point);
        let results_bf = query_bf.find_closest_points(&mut target_bf);

        // Optimized.
        let mut opts_opt = Options::default();
        opts_opt.max_results = 5;
        opts_opt.use_brute_force = false;
        let query_opt = ClosestPointQuery::new(&index, opts_opt);
        let mut target_opt = PointTarget::new(target_point);
        let results_opt = query_opt.find_closest_points(&mut target_opt);

        assert_eq!(results_bf.len(), results_opt.len());
        for (bf, opt) in results_bf.iter().zip(results_opt.iter()) {
            assert!(
                (bf.distance.length2() - opt.distance.length2()).abs() < 1e-15,
                "distance mismatch: bf={} opt={}",
                bf.distance.to_angle().degrees(),
                opt.distance.to_angle().degrees()
            );
        }
    }

    #[test]
    fn test_empty_target_optimized() {
        // Ensure that the optimized algorithm handles empty targets when a
        // distance limit is specified. Matches C++ EmptyTargetOptimized.
        use crate::s2::shape_index::ShapeIndex;
        let mut rng = StdRng::seed_from_u64(42);
        let mut index = S2PointIndex::new();
        for i in 0..1000 {
            index.add(random_point(&mut rng), i);
        }
        let mut opts = Options::default();
        opts.max_distance = ChordAngle::from_angle(Angle::from_radians(1e-5));
        let query = ClosestPointQuery::new(&index, opts);
        let target_index = ShapeIndex::new();
        let mut target = ShapeIndexTarget::new(&target_index);
        let results = query.find_closest_points(&mut target);
        assert_eq!(0, results.len());
    }

    #[test]
    fn test_shape_index_target() {
        // Test that ShapeIndexTarget finds the closest point to an index of
        // shapes (edge).
        use crate::s2::lax_polyline::LaxPolyline;
        use crate::s2::shape_index::ShapeIndex;

        let mut index = S2PointIndex::new();
        // Add a point at (0, 0).
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p, 0);
        // Add a far-away point.
        let p2 = LatLng::from_degrees(80.0, 80.0).to_point();
        index.add(p2, 1);

        // Build a shape index with a short edge near (1, 0).
        let mut target_index = ShapeIndex::new();
        let a = LatLng::from_degrees(1.0, 0.0).to_point();
        let b = LatLng::from_degrees(1.0, 0.1).to_point();
        target_index.add(Box::new(LaxPolyline::new(vec![a, b])));
        target_index.build();

        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = ShapeIndexTarget::new(&target_index);
        let result = query.find_closest_point(&mut target);
        assert_eq!(result.data, 0); // Closest to (0,0), not (80,80).
        assert!(result.distance < ChordAngle::from_angle(Angle::from_degrees(2.0)));
    }

    #[test]
    fn test_region_filter() {
        // Test that the region filter restricts which points are returned.
        use crate::s2::Rect;

        let mut index = S2PointIndex::new();
        // Point inside the filter rect.
        let p_in = LatLng::from_degrees(0.5, 0.5).to_point();
        index.add(p_in, 0);
        // Point outside the filter rect but closer to target.
        let p_out = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p_out, 1);
        // Point inside the filter rect but farther.
        let p_in2 = LatLng::from_degrees(1.0, 1.0).to_point();
        index.add(p_in2, 2);

        // Filter: rect from (0.1, 0.1) to (2.0, 2.0)  — excludes (0,0).
        let filter_rect = Rect::from_point_pair(
            LatLng::from_degrees(0.1, 0.1),
            LatLng::from_degrees(2.0, 2.0),
        );

        let target_point = LatLng::from_degrees(0.0, 0.0).to_point();
        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(target_point);

        // Without region: closest is p_out at (0,0) with data=1.
        let results_no_region = query.find_closest_points(&mut target);
        assert!(results_no_region.len() == 3);
        assert_eq!(results_no_region[0].data, 1);

        // With region: p_out is excluded, closest should be p_in with data=0.
        let results_with_region =
            query.find_closest_points_in_region(&mut target, Some(&filter_rect));
        assert_eq!(results_with_region.len(), 2); // Only p_in and p_in2.
        assert_eq!(results_with_region[0].data, 0); // p_in is closer.
        assert_eq!(results_with_region[1].data, 2); // p_in2 is farther.
    }

    #[test]
    fn test_region_filter_empty_result() {
        // Region that contains no indexed points.
        use crate::s2::Rect;

        let mut index = S2PointIndex::new();
        let p = LatLng::from_degrees(0.0, 0.0).to_point();
        index.add(p, 0);

        // Filter far away from the indexed point.
        let filter_rect = Rect::from_point_pair(
            LatLng::from_degrees(80.0, 80.0),
            LatLng::from_degrees(85.0, 85.0),
        );

        let query = ClosestPointQuery::new(&index, Options::default());
        let mut target = PointTarget::new(LatLng::from_degrees(82.0, 82.0).to_point());
        let results = query.find_closest_points_in_region(&mut target, Some(&filter_rect));
        assert_eq!(results.len(), 0);
    }

    // ─── Randomized tests ────────────────────────────────────────────────

    use crate::s1::Angle;
    use crate::s2::cap::Cap;
    use crate::s2::testing::{
        check_distance_results, frame_at, random_point, sample_point_from_cap,
    };
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const TEST_CAP_RADIUS: Angle = Angle::from_radians(0.0015696098420815537);

    /// Result format for `check_distance_results`.
    type TestingResult = (ChordAngle, i32);

    /// Run the query and extract results, verifying constraints.
    fn get_closest_points(
        target: &mut dyn Target,
        query: &ClosestPointQuery<'_, i32>,
        region: Option<&dyn Region>,
    ) -> Vec<TestingResult> {
        let results = query.find_closest_points_in_region(target, region);
        assert!(results.len() as i32 <= query.options.max_results);
        if region.is_none() && query.options.max_distance == ChordAngle::INFINITY {
            // We can predict exactly how many points should be returned.
            let expected = query
                .options
                .max_results
                .min(query.index.num_points() as i32);
            assert_eq!(expected, results.len() as i32);
        }
        let mut out = Vec::new();
        for r in &results {
            if let Some(reg) = region {
                assert!(reg.contains_point(&r.point));
            }
            assert!(r.distance < query.options.max_distance);
            out.push((r.distance, r.data));
        }
        out
    }

    fn test_find_closest_points(
        target: &mut dyn Target,
        query: &mut ClosestPointQuery<'_, i32>,
        region: Option<&dyn Region>,
    ) {
        query.options.use_brute_force = true;
        let expected = get_closest_points(target, query, region);
        query.options.use_brute_force = false;
        let actual = get_closest_points(target, query, region);
        assert!(
            check_distance_results(
                &expected,
                &actual,
                query.options.max_results,
                query.options.max_distance,
                query.options.max_error,
            ),
            "max_results={}, max_distance={:?}, max_error={:?}",
            query.options.max_results,
            query.options.max_distance,
            query.options.max_error,
        );

        if expected.is_empty() {
            return;
        }

        let max_error = query.options.max_error;
        // When max_results > 1 and max_error > 0, expected[0].distance may
        // not be the true minimum. Use a fresh single-result brute-force
        // search for a tighter bound (same approach as closest_edge_query).
        let min_distance = if max_error > ChordAngle::ZERO && query.options.max_results > 1 {
            let bf1_opts = Options {
                max_results: 1,
                use_brute_force: true,
                max_distance: query.options.max_distance,
                max_error: ChordAngle::ZERO,
            };
            let bf1 = ClosestPointQuery::new(query.index, bf1_opts);
            let bf1_results = bf1.find_closest_points_in_region(target, region);
            if bf1_results.is_empty() {
                expected[0].0
            } else {
                bf1_results[0].distance
            }
        } else {
            expected[0].0
        };
        let dist = query.get_distance(target);
        assert!(
            dist <= min_distance + max_error,
            "get_distance={dist:?} but expected min_distance={min_distance:?} + max_error={max_error:?}"
        );
        // Only check distance predicates when no region filter is active,
        // since is_distance_less/etc. don't apply the region filter.
        if region.is_none() {
            let too_close = min_distance - max_error;
            if too_close > ChordAngle::ZERO {
                assert!(
                    !query.is_distance_less(target, too_close),
                    "is_distance_less should be false for limit below min - error"
                );
            }
            assert!(query.is_distance_less_or_equal(target, expected[0].0));
            assert!(query.is_conservative_distance_less_or_equal(target, expected[0].0));
        }
    }

    fn log_uniform(rng: &mut StdRng, lo: f64, hi: f64) -> f64 {
        let log_lo = lo.ln();
        let log_hi = hi.ln();
        (rng.r#gen::<f64>() * (log_hi - log_lo) + log_lo).exp()
    }

    fn test_with_index_factory(
        add_points: impl Fn(&Cap, usize, &mut StdRng, &mut S2PointIndex<i32>),
        num_indexes: usize,
        num_points: usize,
        num_queries: usize,
    ) {
        use crate::s2::Rect;
        use crate::s2::cell_id::CellId;
        use crate::s2::shape_index::ShapeIndex;
        use rand::Rng as _;

        let mut rng = StdRng::seed_from_u64(42);
        let mut index_caps = Vec::new();
        let mut indexes = Vec::new();

        for _ in 0..num_indexes {
            let cap = Cap::from_center_angle(random_point(&mut rng), TEST_CAP_RADIUS);
            let mut idx = S2PointIndex::new();
            add_points(&cap, num_points, &mut rng, &mut idx);
            index_caps.push(cap);
            indexes.push(idx);
        }

        for _ in 0..num_queries {
            let i_index = rng.r#gen::<usize>() % num_indexes;
            let index_cap = &index_caps[i_index];
            let query_radius = 2.0 * index_cap.angle_radius().radians();
            let query_cap =
                Cap::from_center_angle(index_cap.center(), Angle::from_radians(query_radius));

            let mut opts = Options::default();
            // 80% of the time, limit results.
            if rng.r#gen::<f64>() < 0.8 {
                opts.max_results = rng.gen_range(1..=10);
            }
            // 2/3 of the time, limit distance.
            if rng.r#gen::<f64>() < 2.0 / 3.0 {
                opts.max_distance =
                    ChordAngle::from_angle(Angle::from_radians(rng.r#gen::<f64>() * query_radius));
            }
            // 50% of the time, add max_error.
            if rng.r#gen::<f64>() < 0.5 {
                opts.max_error = ChordAngle::from_angle(Angle::from_radians(
                    log_uniform(&mut rng, 1e-4, 1.0) * query_radius,
                ));
            }

            let mut query = ClosestPointQuery::new(&indexes[i_index], opts);

            // 20% of the time, apply a region filter.
            let sample_point = sample_point_from_cap(&mut rng, &query_cap);
            let lat_size = rng.r#gen::<f64>() * TEST_CAP_RADIUS.radians();
            let lng_size = rng.r#gen::<f64>() * TEST_CAP_RADIUS.radians();
            let filter_rect = Rect::from_center_size(
                LatLng::from_point(sample_point),
                LatLng::new(Angle::from_radians(lat_size), Angle::from_radians(lng_size)),
            );
            let region: Option<&dyn Region> = if rng.r#gen::<f64>() < 0.2 {
                Some(&filter_rect)
            } else {
                None
            };

            let target_type: u32 = rng.gen_range(0..4);
            match target_type {
                0 => {
                    let p = sample_point_from_cap(&mut rng, &query_cap);
                    let mut target = PointTarget::new(p);
                    test_find_closest_points(&mut target, &mut query, region);
                }
                1 => {
                    let a = sample_point_from_cap(&mut rng, &query_cap);
                    let b_cap = Cap::from_center_angle(
                        a,
                        Angle::from_radians(log_uniform(&mut rng, 1e-4, 1.0) * query_radius),
                    );
                    let b = sample_point_from_cap(&mut rng, &b_cap);
                    let mut target = EdgeTarget::new(a, b);
                    test_find_closest_points(&mut target, &mut query, region);
                }
                2 => {
                    let a = sample_point_from_cap(&mut rng, &query_cap);
                    let min_level = crate::s2::metric::MAX_DIAG.min_level(query_radius);
                    let level = Level::new(
                        rng.gen_range(min_level.as_u8()..=crate::s2::coords::MAX_CELL_LEVEL),
                    );
                    let cell = Cell::from(CellId::from_point(&a).parent_at_level(level));
                    let mut target = CellTarget::new(cell);
                    test_find_closest_points(&mut target, &mut query, region);
                }
                _ => {
                    // ShapeIndexTarget
                    let mut target_index = ShapeIndex::new();
                    crate::s2::testing::add_fractal_loop_edges(
                        index_cap,
                        100,
                        &mut rng,
                        &mut target_index,
                    );
                    target_index.build();
                    let mut target = ShapeIndexTarget::new(&target_index);
                    target.include_interiors = rng.r#gen::<bool>();
                    test_find_closest_points(&mut target, &mut query, region);
                }
            }
        }
    }

    fn add_circle_points(
        cap: &Cap,
        num_points: usize,
        _rng: &mut StdRng,
        index: &mut S2PointIndex<i32>,
    ) {
        let points =
            crate::s2::testing::make_regular_points(cap.center(), cap.angle_radius(), num_points);
        for (i, p) in points.into_iter().enumerate() {
            index.add(p, i as i32);
        }
    }

    fn add_fractal_points(
        cap: &Cap,
        num_points: usize,
        rng: &mut StdRng,
        index: &mut S2PointIndex<i32>,
    ) {
        use crate::r3::matrix::Matrix3x3;
        use crate::s2::fractal::S2Fractal;
        let mut fractal = S2Fractal::new(rng.r#gen::<u64>());
        fractal.level_for_approx_max_edges(num_points as i32);
        fractal.set_fractal_dimension(1.5);
        let (x, y, z) = frame_at(rng, cap.center());
        let mat = Matrix3x3::from_cols(x.0, y.0, z.0);
        let lp = fractal.make_loop(&mat, cap.angle_radius());
        for (i, v) in lp.vertices().iter().enumerate() {
            index.add(*v, i as i32);
        }
    }

    fn add_grid_points(
        cap: &Cap,
        num_points: usize,
        rng: &mut StdRng,
        index: &mut S2PointIndex<i32>,
    ) {
        use crate::r3::matrix::Matrix3x3;
        use crate::s2::point::from_frame;
        let sqrt_n = (num_points as f64).sqrt().ceil() as usize;
        let (x, y, z) = frame_at(rng, cap.center());
        let mat = Matrix3x3::from_cols(x.0, y.0, z.0);
        let radius = cap.angle_radius().radians();
        let spacing = 2.0 * radius / sqrt_n as f64;
        for i in 0..sqrt_n {
            for j in 0..sqrt_n {
                let p = Point::from_coords(
                    ((i as f64 + 0.5) * spacing - radius).tan(),
                    ((j as f64 + 0.5) * spacing - radius).tan(),
                    1.0,
                );
                index.add(from_frame(&mat, p.normalize()), (i * sqrt_n + j) as i32);
            }
        }
    }

    const NUM_INDEXES: usize = 10;
    const NUM_POINTS: usize = 1000;
    const NUM_QUERIES: usize = 50;

    #[test]
    fn test_circle_points() {
        test_with_index_factory(add_circle_points, NUM_INDEXES, NUM_POINTS, NUM_QUERIES);
    }

    #[test]
    fn test_fractal_points() {
        test_with_index_factory(add_fractal_points, NUM_INDEXES, NUM_POINTS, NUM_QUERIES);
    }

    #[test]
    fn test_grid_points() {
        test_with_index_factory(add_grid_points, NUM_INDEXES, NUM_POINTS, NUM_QUERIES);
    }

    #[test]
    fn test_conservative_cell_distance_is_used() {
        // Smaller values to exercise the conservative cell distance code path.
        test_with_index_factory(add_fractal_points, 5, 100, 10);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_result_roundtrip() {
        let r = Result {
            distance: ChordAngle::from_degrees(30.0),
            point: Point::from_coords(1.0, 0.0, 0.0),
            data: 42i32,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Result<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(r.distance, back.distance);
        assert_eq!(r.point, back.point);
        assert_eq!(r.data, back.data);
    }
}
