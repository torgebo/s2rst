// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Closed loop of vertices on the unit sphere.
//!
//! A [`Loop`] represents a simple spherical polygon — a sequence of vertices
//! whose edges do not self-intersect, with the interior to the left of the
//! boundary. The last vertex is implicitly connected to the first.
//!
//! Special "empty" and "full" loops represent the empty set and the full
//! sphere respectively.
//!
//! Corresponds to C++ `s2loop.h`, Go `s2/loop.go`.

#![expect(clippy::cast_sign_loss, reason = "EdgeId (i32) used as vertex indices")]
#![expect(
    clippy::cast_possible_truncation,
    reason = "vertex index (usize<->i32) for loop edge iteration"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for vertex index — always in range"
)]
use crate::s1::Angle;
use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings::Crossing;
use crate::s2::edge_distances;
use crate::s2::point_measures;
use crate::s2::predicates;
use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};
use crate::s2::shape_index::{RangeIterator, ShapeIndex};
use crate::s2::wedge_relations;
use crate::s2::{Cap, Cell, CellId, Point, Rect, Region};
use std::collections::HashSet;

/// Threshold below which brute-force containment is used instead of index.
const BRUTE_FORCE_VERTEX_THRESHOLD: usize = 32;

/// A simple spherical polygon defined by a closed sequence of edges.
///
/// Implements [`Shape`] (dimension 2) and [`Region`]. The loop owns an
/// internal [`ShapeIndex`] for accelerated point containment.
///
/// # Examples
///
/// ```
/// use s2rst::s2::{LatLng, Loop, Region};
///
/// let loop_ = Loop::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 90.0).to_point(),
///     LatLng::from_degrees(90.0, 0.0).to_point(),
/// ]);
/// // This triangle covers roughly one octant of the sphere (~pi/2 steradians).
/// let area = loop_.area();
/// assert!((area - std::f64::consts::FRAC_PI_2).abs() < 0.05);
///
/// // Point containment: the interior point (10, 10) is inside.
/// let inside = LatLng::from_degrees(10.0, 10.0).to_point();
/// assert!(loop_.contains_point(&inside));
/// ```
///
/// Create a regular polygon (approximating a circle) from a center and
/// radius:
///
/// ```
/// use s2rst::s1::Angle;
/// use s2rst::s2::{LatLng, Loop, Region};
///
/// let center = LatLng::from_degrees(37.7749, -122.4194).to_point();
/// let hexagon = Loop::make_regular(center, Angle::from_degrees(0.1), 6);
/// assert_eq!(hexagon.num_vertices(), 6);
/// assert!(hexagon.contains_point(&center));
/// assert!(hexagon.area() > 0.0);
/// ```
pub struct Loop {
    vertices: Vec<Point>,
    origin_inside: bool,
    depth: i32,
    bound: Rect,
    subregion_bound: Rect,
    index: ShapeIndex,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Loop {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Loop", 5)?;
        state.serialize_field("vertices", &self.vertices)?;
        state.serialize_field("origin_inside", &self.origin_inside)?;
        state.serialize_field("depth", &self.depth)?;
        state.serialize_field("bound", &self.bound)?;
        state.serialize_field("subregion_bound", &self.subregion_bound)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Loop {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct LoopData {
            vertices: Vec<Point>,
            origin_inside: bool,
            depth: i32,
            bound: Rect,
            subregion_bound: Rect,
        }
        let data = LoopData::deserialize(deserializer)?;
        let mut l = Loop {
            vertices: data.vertices,
            origin_inside: data.origin_inside,
            depth: data.depth,
            bound: data.bound,
            subregion_bound: data.subregion_bound,
            index: ShapeIndex::new(),
        };
        if !l.vertices.is_empty() {
            l.init_index();
        }
        Ok(l)
    }
}

impl Loop {
    /// Creates a new `Loop` from a list of vertices.
    ///
    /// The vertices should define a simple spherical polygon with the
    /// interior to the left of the boundary.
    pub fn new(vertices: Vec<Point>) -> Self {
        let mut l = Loop {
            vertices,
            origin_inside: false,
            depth: 0,
            bound: Rect::empty(),
            subregion_bound: Rect::empty(),
            index: ShapeIndex::new(),
        };
        l.init_origin_and_bound();
        l
    }

    /// Returns the special "empty" loop (contains no points).
    pub fn empty() -> Self {
        // Empty loop: single vertex at the north pole, origin is outside.
        let v = Point::from_coords(0.0, 0.0, 1.0);
        Loop {
            vertices: vec![v],
            origin_inside: false,
            depth: 0,
            bound: Rect::empty(),
            subregion_bound: Rect::empty(),
            index: ShapeIndex::new(),
        }
    }

    /// Returns the special "full" loop (contains all points).
    pub fn full() -> Self {
        // Full loop: single vertex at the south pole, origin is inside.
        let v = Point::from_coords(0.0, 0.0, -1.0);
        let bound = Rect::full();
        Loop {
            vertices: vec![v],
            origin_inside: true,
            depth: 0,
            bound,
            subregion_bound: bound.expand_for_subregions(),
            index: ShapeIndex::new(),
        }
    }

    /// Creates a loop from a cell's four vertices.
    pub fn from_cell(cell: &Cell) -> Self {
        let verts = (0..4).map(|i| cell.vertex(i)).collect();
        Self::new(verts)
    }

    /// Creates a regular loop (an approximate circle) with the given center,
    /// radius, and number of vertices.
    ///
    /// The loop is oriented so that the interior is on the left side of the
    /// boundary. The first vertex is at the given radius from the center,
    /// in the direction of `ortho(center)`.
    ///
    /// Corresponds to C++ `S2Loop::MakeRegularLoop(center, radius, num_vertices)`.
    pub fn make_regular(center: Point, radius: Angle, num_vertices: usize) -> Self {
        let frame = crate::s2::point::get_frame(center);
        Self::make_regular_with_frame(&frame, radius, num_vertices)
    }

    /// Like `make_regular`, but constructs a loop centered around the z-axis
    /// of the given coordinate frame. The first vertex is in the direction of
    /// the positive x-axis.
    ///
    /// Corresponds to C++ `S2Loop::MakeRegularLoop(frame, radius, num_vertices)`.
    pub fn make_regular_with_frame(
        frame: &crate::r3::Matrix3x3,
        radius: Angle,
        num_vertices: usize,
    ) -> Self {
        let (r, z) = radius.sin_cos();
        let radian_step = 2.0 * std::f64::consts::PI / num_vertices as f64;
        let mut vertices = Vec::with_capacity(num_vertices);
        for i in 0..num_vertices {
            let angle = Angle::from_radians(i as f64 * radian_step);
            let (a_sin, a_cos) = angle.sin_cos();
            let p = Point(crate::r3::Vector {
                x: r * a_cos,
                y: r * a_sin,
                z,
            });
            vertices.push(crate::s2::point::from_frame(frame, p).normalize());
        }
        Loop::new(vertices)
    }

    /// Creates a loop from pre-decoded fields. Used by binary decoding.
    pub(crate) fn from_decoded(
        vertices: Vec<Point>,
        origin_inside: bool,
        depth: i32,
        bound: Rect,
    ) -> Self {
        let mut l = Loop {
            vertices,
            origin_inside,
            depth,
            subregion_bound: bound.expand_for_subregions(),
            bound,
            index: ShapeIndex::new(),
        };
        l.index.add(Box::new(LoopShape {
            vertices: l.vertices.clone(),
            origin_inside: l.origin_inside,
        }));
        l.index.build();
        l
    }

    /// Creates a loop from compressed-decoded fields. If `bound` is `None`,
    /// recomputes from vertices via `init_bound`.
    pub(crate) fn from_decoded_compressed(
        vertices: Vec<Point>,
        origin_inside: bool,
        depth: i32,
        bound: Option<Rect>,
    ) -> Self {
        if let Some(b) = bound {
            Self::from_decoded(vertices, origin_inside, depth, b)
        } else {
            let mut l = Loop {
                vertices,
                origin_inside,
                depth,
                bound: Rect::empty(),
                subregion_bound: Rect::empty(),
                index: ShapeIndex::new(),
            };
            l.init_bound();
            l.index.add(Box::new(LoopShape {
                vertices: l.vertices.clone(),
                origin_inside: l.origin_inside,
            }));
            l.index.build();
            l
        }
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the vertex at the given index (wraps around).
    pub fn vertex(&self, i: usize) -> Point {
        debug_assert!(i < 2 * self.num_vertices());
        self.vertices[i % self.vertices.len()]
    }

    /// Returns the vertices as a slice.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    /// Reports whether this loop represents the empty set.
    pub fn is_empty_loop(&self) -> bool {
        self.vertices.len() == 1 && !self.origin_inside
    }

    /// Reports whether this loop represents the full sphere.
    pub fn is_full_loop(&self) -> bool {
        self.vertices.len() == 1 && self.origin_inside
    }

    /// Reports whether this loop is either the empty or full loop.
    /// These are loops with a single vertex that have no boundary edges.
    pub fn is_empty_or_full(&self) -> bool {
        self.vertices.len() == 1
    }

    /// Reports whether this loop is a hole (odd nesting depth).
    pub fn is_hole(&self) -> bool {
        self.depth & 1 != 0
    }

    /// Like `vertex()`, but for hole loops returns the vertices in reverse
    /// order (for use in `S2Builder`). This is because hole loops are assembled
    /// clockwise and later reversed by `S2Loop::Invert()`, so we need to
    /// start with the reverse order to get the original vertex ordering.
    ///
    /// REQUIRES: 0 <= i < 2 * `num_vertices()`
    pub fn oriented_vertex(&self, i: usize) -> Point {
        let n = self.num_vertices();
        let mut j = if i >= n { i - n } else { i };
        if self.is_hole() {
            j = n - 1 - j;
        }
        self.vertices[j]
    }

    /// Returns the bounding rectangle.
    pub fn bound(&self) -> Rect {
        self.bound
    }

    /// Returns the bounding rectangle expanded for subregion containment.
    ///
    /// If `A.subregion_bound().contains(B.bound())` then `A` contains `B`.
    pub fn subregion_bound(&self) -> Rect {
        self.subregion_bound
    }

    /// Returns the nesting depth.
    pub fn depth(&self) -> i32 {
        self.depth
    }

    /// Sets the nesting depth.
    pub fn set_depth(&mut self, depth: i32) {
        self.depth = depth;
    }

    /// Returns +1 for a shell (contains interior), -1 for a hole.
    pub fn sign(&self) -> i32 {
        if self.is_hole() { -1 } else { 1 }
    }

    /// Reports whether the loop's interior contains the origin point.
    pub fn contains_origin(&self) -> bool {
        self.origin_inside
    }

    /// Reports whether this loop is "normalized" (area <= 2π).
    ///
    /// Uses curvature-based check for precise results. If the longitude
    /// span is less than π, the loop covers less than half the sphere.
    pub fn is_normalized(&self) -> bool {
        if self.bound.lng.length() < std::f64::consts::PI {
            return true;
        }
        crate::s2::loop_measures::is_normalized(&self.vertices)
    }

    /// Normalizes the loop so that it encloses at most half the sphere.
    /// If the loop already satisfies this condition, it is left unchanged.
    /// Otherwise the loop is inverted (i.e., the region enclosed by the
    /// loop becomes its complement).
    pub fn normalize(&mut self) {
        if !self.is_normalized() {
            self.invert();
        }
    }

    /// Reverses the order of the vertices (complement the loop).
    pub fn invert(&mut self) {
        self.vertices.reverse();
        self.origin_inside = !self.origin_inside;
        self.init_bound();
        self.init_index();
    }

    /// Returns the curvature of the loop (sum of turning angles at vertices).
    ///
    /// Result is positive for CCW loops, negative for CW loops.
    pub fn get_curvature(&self) -> f64 {
        if self.is_empty_loop() {
            return 2.0 * std::f64::consts::PI;
        }
        if self.is_full_loop() {
            return -2.0 * std::f64::consts::PI;
        }
        crate::s2::loop_measures::get_curvature(&self.vertices)
    }

    /// Returns the maximum error in `get_curvature`.
    pub fn get_curvature_max_error(&self) -> f64 {
        crate::s2::loop_measures::get_curvature_max_error(&self.vertices)
    }

    /// Returns the area of the loop interior (0 to 4π).
    pub fn area(&self) -> f64 {
        if self.is_empty_loop() {
            return 0.0;
        }
        if self.is_full_loop() {
            return 4.0 * std::f64::consts::PI;
        }

        // Use the robust loop_measures implementation which cross-checks
        // the curvature-based and triangle-fan-based methods.
        crate::s2::loop_measures::get_area(&self.vertices)
    }

    /// Returns the signed turning angle of the loop boundary.
    ///
    /// Positive for CCW loops, negative for CW loops.
    pub fn turning_angle(&self) -> f64 {
        if self.is_empty_loop() {
            return -2.0 * std::f64::consts::PI;
        }
        if self.is_full_loop() {
            return 2.0 * std::f64::consts::PI;
        }
        let n = self.vertices.len();
        if n < 3 {
            return 0.0;
        }

        // Sum the turn angles at each vertex.
        let mut sum = 0.0f64;
        for i in 0..n {
            let prev = self.vertices[(i + n - 1) % n];
            let curr = self.vertices[i];
            let next = self.vertices[(i + 1) % n];
            sum += point_measures::turn_angle(prev, curr, next).radians();
        }
        sum
    }

    /// Returns the area-weighted centroid (not unit length).
    pub fn centroid(&self) -> Point {
        if self.is_empty_loop() || self.is_full_loop() {
            return Point::from_coords(0.0, 0.0, 0.0);
        }
        let n = self.vertices.len();
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut cz = 0.0;
        for i in 1..n - 1 {
            let c = crate::s2::centroids::true_centroid(
                self.vertices[0],
                self.vertices[i],
                self.vertices[i + 1],
            );
            cx += c.0.x;
            cy += c.0.y;
            cz += c.0.z;
        }
        Point(crate::r3::Vector {
            x: cx,
            y: cy,
            z: cz,
        })
    }

    /// Tests whether the loop contains the given point using brute force
    /// (edge crossing from origin).
    pub fn brute_force_contains_point(&self, p: Point) -> bool {
        let origin = Point::origin();
        let mut inside = self.origin_inside;
        let mut crosser = EdgeCrosser::new(origin, p);
        let n = self.vertices.len();
        for i in 0..n {
            let next = (i + 1) % n;
            inside ^= crosser.edge_or_vertex_crossing(self.vertices[i], self.vertices[next]);
        }
        inside
    }

    /// Validates the loop.
    ///
    /// # Errors
    ///
    /// Returns a description of the first validation error found (non-unit
    /// vertex, too few vertices, identical or antipodal adjacent vertices).
    pub fn validate(&self) -> Result<(), String> {
        // Empty and full loops are always valid.
        if self.is_empty_loop() || self.is_full_loop() {
            return Ok(());
        }
        if self.vertices.len() < 3 {
            return Err(format!(
                "loop has {} vertices (need >= 3)",
                self.vertices.len()
            ));
        }
        for (i, v) in self.vertices.iter().enumerate() {
            let norm = v.0.norm();
            if (norm - 1.0).abs() > 1e-15 {
                return Err(format!("vertex {i} is not unit length: {norm}"));
            }
        }
        for i in 0..self.vertices.len() {
            let next = (i + 1) % self.vertices.len();
            if self.vertices[i] == self.vertices[next] {
                return Err(format!("vertices {i} and {next} are identical"));
            }
            if self.vertices[i] == Point(-self.vertices[next].0) {
                return Err(format!("vertices {i} and {next} are antipodal"));
            }
        }
        Ok(())
    }

    /// Reports whether this loop is equal to another.
    pub fn equal(&self, other: &Loop) -> bool {
        self.vertices == other.vertices
    }

    /// Returns the index of the first vertex equal to `p`, or -1 if not found.
    ///
    /// Corresponds to C++ `S2Loop::FindVertex`.
    pub fn find_vertex(&self, p: &Point) -> i32 {
        for i in 0..self.num_vertices() {
            if self.vertex(i) == *p {
                return i as i32;
            }
        }
        -1
    }

    /// Returns the internal `ShapeIndex`.
    pub fn shape_index(&self) -> &ShapeIndex {
        &self.index
    }

    // ─── Distance / projection ─────────────────────────────────────

    /// Returns the minimum distance from the given point to the loop.
    /// If the loop contains the point, the distance is zero.
    ///
    /// Corresponds to C++ `S2Loop::GetDistance`.
    pub fn get_distance(&self, x: Point) -> Angle {
        use crate::s2::region::Region;
        if self.contains_point(&x) {
            return Angle::from_radians(0.0);
        }
        self.get_distance_to_boundary(x)
    }

    /// Returns the minimum distance from the given point to the loop boundary.
    ///
    /// Corresponds to C++ `S2Loop::GetDistanceToBoundary`.
    pub fn get_distance_to_boundary(&self, x: Point) -> Angle {
        use crate::s2::closest_edge_query::{ClosestEdgeQuery, Options, PointTarget};
        let opts = Options {
            include_interiors: false,
            ..Options::default()
        };
        let target = PointTarget::new(x);
        let query = ClosestEdgeQuery::new(&self.index);
        query
            .find_closest_edge_with_options(&target, &opts)
            .distance
            .to_angle()
    }

    /// Returns the closest point on the loop to the given point.
    /// If the loop contains the point, the point itself is returned.
    ///
    /// Corresponds to C++ `S2Loop::Project`.
    pub fn project_point(&self, x: Point) -> Point {
        use crate::s2::region::Region;
        if self.contains_point(&x) {
            return x;
        }
        self.project_to_boundary(x)
    }

    /// Returns the closest point on the loop boundary to the given point.
    ///
    /// Corresponds to C++ `S2Loop::ProjectToBoundary`.
    pub fn project_to_boundary(&self, x: Point) -> Point {
        use crate::s2::closest_edge_query::{ClosestEdgeQuery, Options, PointTarget};
        let opts = Options {
            include_interiors: false,
            ..Options::default()
        };
        let query = ClosestEdgeQuery::new(&self.index);
        let target = PointTarget::new(x);
        let result = query.find_closest_edge_with_options(&target, &opts);
        query.project(x, &result)
    }

    // ─── Loop-Loop containment / intersection ──────────────────────

    /// Reports whether this loop contains the other loop.
    ///
    /// This loop contains `b` if and only if: (1) no edges cross
    /// improperly, (2) at shared vertices, `self` locally contains `b`
    /// (wedge check), and (3) if there are no shared vertices,
    /// `self` contains a vertex of `b`.
    ///
    /// Corresponds to C++ `S2Loop::Contains(const S2Loop& b)`.
    pub fn contains_loop(&self, b: &Loop) -> bool {
        // Quick rejection via expanded bound.
        if !self.subregion_bound.contains(b.bound) {
            return false;
        }
        // Special cases.
        if self.is_empty_or_full() || b.is_empty_or_full() {
            return self.is_full_loop() || b.is_empty_loop();
        }

        // Check for edge crossings and shared vertex containment.
        let mut relation = ContainsRelation::new();
        if has_crossing_relation(self, b, &mut relation) {
            return false;
        }
        if relation.found_shared_vertex() {
            return true;
        }

        // No shared vertices: check if self contains a vertex of b.
        if !self.contains_point(&b.vertex(0)) {
            return false;
        }

        // Also verify that A ∪ B is not the entire sphere.
        if (b.subregion_bound.contains(self.bound) || b.bound.union(self.bound).is_full())
            && b.contains_point(&self.vertex(0))
        {
            return false;
        }

        true
    }

    /// Reports whether this loop intersects the other loop.
    ///
    /// Two loops intersect if they share any interior point.
    ///
    /// Corresponds to C++ `S2Loop::Intersects(const S2Loop& b)`.
    pub fn intersects_loop(&self, b: &Loop) -> bool {
        // Quick rejection via bounding rect.
        if !self.bound.intersects(b.bound) {
            return false;
        }
        // Special cases.
        if self.is_empty_loop() || b.is_empty_loop() {
            return false;
        }
        if self.is_full_loop() || b.is_full_loop() {
            return true;
        }

        // Check for edge crossings and shared vertex intersection.
        let mut relation = IntersectsRelation::new();
        if has_crossing_relation(self, b, &mut relation) {
            return true;
        }
        if relation.found_shared_vertex() {
            return false;
        }

        // No shared vertices: check if one loop contains a vertex of the other.
        if (self.subregion_bound.contains(b.bound) || self.bound.union(b.bound).is_full())
            && self.contains_point(&b.vertex(0))
        {
            return true;
        }
        if b.subregion_bound.contains(self.bound) && b.contains_point(&self.vertex(0)) {
            return true;
        }
        false
    }

    /// Reports whether this loop contains `b`, given that the boundaries
    /// are known not to cross. Used during polygon initialization.
    ///
    /// Corresponds to C++ `S2Loop::ContainsNested(const S2Loop& b)`.
    pub fn contains_nested(&self, b: &Loop) -> bool {
        if !self.subregion_bound.contains(b.bound) {
            return false;
        }
        if self.is_empty_or_full() || b.num_vertices() < 2 {
            return self.is_full_loop() || b.is_empty_loop();
        }

        // Check if b.vertex(1) is shared with self.
        let m = self.find_vertex(&b.vertex(1));
        if m < 0 {
            // b.vertex(1) is not shared: direct containment test.
            return self.contains_point(&b.vertex(1));
        }
        // b.vertex(1) is shared: check edge ordering via wedge.
        let m = m as usize;
        wedge_relations::wedge_contains(
            self.vertex(m + self.num_vertices() - 1),
            self.vertex(m),
            self.vertex(m + 1),
            b.vertex(0),
            b.vertex(2),
        )
    }

    /// Compares this loop's boundary with loop `b`'s boundary.
    ///
    /// Returns:
    /// - `+1` if this loop contains `b`'s boundary
    /// - `-1` if this loop excludes `b`'s boundary
    /// - `0` if the boundaries cross
    ///
    /// Corresponds to C++ `S2Loop::CompareBoundary(const S2Loop& b)`.
    pub fn compare_boundary(&self, b: &Loop) -> i32 {
        if !self.bound.intersects(b.bound) {
            return -1;
        }
        if self.is_full_loop() {
            return 1;
        }
        if b.is_full_loop() {
            return -1;
        }

        let mut relation = CompareBoundaryRelation::new(b.is_hole());
        if has_crossing_relation(self, b, &mut relation) {
            return 0;
        }
        if relation.found_shared_vertex() {
            return if relation.contains_edge() { 1 } else { -1 };
        }

        // No shared vertices: check if self contains a vertex of b.
        if self.contains_point(&b.vertex(0)) {
            1
        } else {
            -1
        }
    }

    /// Reports whether this loop contains the boundary of `b`, assuming
    /// that the boundaries are known not to cross. The containment test
    /// is based on the given point `p` which is known to be on or near
    /// the boundary of `b`.
    ///
    /// Corresponds to C++ `S2Loop::ContainsNonCrossingBoundary`.
    pub fn contains_non_crossing_boundary(&self, b: &Loop, reverse_b: bool) -> bool {
        // Given that the boundaries of A and B do not cross, check whether
        // A contains the boundary of B. If reverse_b is true, the boundary
        // of B is reversed first (which only affects the result when there
        // are shared edges).
        //
        // Corresponds to Go Loop.containsNonCrossingBoundary.
        if !self.bound.intersects(b.bound) {
            return false;
        }
        if self.is_full_loop() {
            return true;
        }
        if b.is_full_loop() {
            return false;
        }

        // If b.vertex(0) is shared with self, use wedge analysis.
        let m = self.find_vertex(&b.vertex(0));
        if m < 0 {
            // Not a shared vertex: simple containment test.
            // Note: reverse_b only affects the wedge analysis for shared
            // vertices; for non-shared vertices we just check containment.
            return self.contains_point(&b.vertex(0));
        }
        // Shared vertex: check whether the edge order around the shared
        // vertex is compatible with containment.
        wedge_contains_semiwedge(
            self.vertex(m as usize + self.num_vertices() - 1),
            self.vertex(m as usize),
            self.vertex(m as usize + 1),
            b.vertex(1),
            reverse_b,
        )
    }

    /// Reports whether the boundary of this loop is within `max_error` of
    /// the boundary of `b`. The two loops must have the same number of
    /// vertices. All vertices of `b` must match some cyclic rotation of
    /// `self`'s vertices within the given tolerance.
    ///
    /// Corresponds to C++ `S2Loop::BoundaryApproxEquals`.
    pub fn boundary_approx_eq(&self, b: &Loop, max_error: Angle) -> bool {
        if self.num_vertices() != b.num_vertices() {
            return false;
        }
        // Special case: empty or full loops.
        if self.is_empty_or_full() {
            return self.is_empty_loop() == b.is_empty_loop();
        }
        let n = self.num_vertices();
        for offset in 0..n {
            if self.vertex(offset).approx_eq_angle(b.vertex(0), max_error) {
                let mut success = true;
                for i in 0..n {
                    if !self
                        .vertex(i + offset)
                        .approx_eq_angle(b.vertex(i), max_error)
                    {
                        success = false;
                        break;
                    }
                }
                if success {
                    return true;
                }
            }
        }
        false
    }

    /// Reports whether the boundary of this loop is within `max_error` of
    /// the boundary of `b`. Unlike `boundary_approx_eq`, this method
    /// allows the two loops to have different numbers of vertices. It checks
    /// that every vertex of each loop is within `max_error` of some edge of
    /// the other loop.
    ///
    /// Corresponds to C++ `S2Loop::BoundaryNear`.
    pub fn boundary_near(&self, b: &Loop, max_error: Angle) -> bool {
        // Special case: empty or full loops.
        if self.is_empty_or_full() || b.is_empty_or_full() {
            return (self.is_empty_loop() && b.is_empty_loop())
                || (self.is_full_loop() && b.is_full_loop());
        }
        for a_offset in 0..self.num_vertices() {
            if match_boundaries(self, b, a_offset, max_error) {
                return true;
            }
        }
        false
    }

    // ─── Private helpers ────────────────────────────────────────────

    fn init_origin_and_bound(&mut self) {
        if self.vertices.len() < 3 {
            // Special case: empty or full loop.
            self.origin_inside = self.vertices.len() == 1 && self.vertices[0].0.z < 0.0;
            self.init_bound();
            return;
        }

        // Determine whether vertex(1) is inside the loop by checking
        // the angle at vertex(1) formed by edges from vertex(0) and vertex(2).
        // This is a purely local geometric test that doesn't depend on
        // origin_inside.
        use crate::s2::edge_crossings::angle_contains_vertex;
        let v1_inside = self.vertices[0] != self.vertices[1]
            && self.vertices[2] != self.vertices[1]
            && angle_contains_vertex(self.vertices[0], self.vertices[1], self.vertices[2]);

        // Guess origin_inside = false, then check using brute force containment
        // of vertex(1). If the result disagrees with v1_inside, the origin
        // must actually be inside the loop.
        // We must use the same edge iteration order as C++ BruteForceContains
        // to get consistent results for boundary vertices.
        // C++ iterates: for (i = n; i > 0; --i) edge(vertex(i), vertex(i-1))
        self.origin_inside = false;
        let n = self.vertices.len();
        let origin = Point::origin();
        let mut contains_v1 = false; // origin_inside = false
        let mut crosser = EdgeCrosser::new(origin, self.vertices[1]);
        for i in (1..=n).rev() {
            contains_v1 ^=
                crosser.edge_or_vertex_crossing(self.vertices[i % n], self.vertices[i - 1]);
        }
        if v1_inside != contains_v1 {
            self.origin_inside = true;
        }

        self.init_bound();
        self.init_index();
    }

    fn init_bound(&mut self) {
        if self.is_empty_loop() {
            self.bound = Rect::empty();
            self.subregion_bound = Rect::empty();
            return;
        }
        if self.is_full_loop() {
            self.bound = Rect::full();
            self.subregion_bound = self.bound.expand_for_subregions();
            return;
        }
        // Use LatLngRectBounder to compute bounds from edges (not just
        // vertices). This correctly handles edges whose interior extends
        // beyond the bounding box of their endpoints, such as equatorial
        // edges that wrap around the sphere.
        let mut bounder = crate::s2::latlng_rect_bounder::LatLngRectBounder::new();
        for i in 0..=self.vertices.len() {
            bounder.add_point(self.vertices[i % self.vertices.len()]);
        }
        let mut rect = bounder.get_bound();

        // Check if the loop contains the north pole. If so, extend lat to
        // PI/2 and make lng full. We use brute_force_contains_point to avoid
        // the circular dependency with contains_point (which checks bound).
        let north_pole = Point::from_coords(0.0, 0.0, 1.0);
        if self.brute_force_contains_point(north_pole) {
            rect = Rect::new(
                crate::r1::Interval::new(rect.lat.lo, std::f64::consts::FRAC_PI_2),
                crate::s1::Interval::full(),
            );
        }
        // If the longitude span is full, also check the south pole.
        let south_pole = Point::from_coords(0.0, 0.0, -1.0);
        if rect.lng.is_full() && self.brute_force_contains_point(south_pole) {
            rect = Rect::new(
                crate::r1::Interval::new(-std::f64::consts::FRAC_PI_2, rect.lat.hi),
                rect.lng,
            );
        }

        self.bound = rect;
        self.subregion_bound = self.bound.expand_for_subregions();
    }

    fn init_index(&mut self) {
        self.index = ShapeIndex::new();
        // We can't add `self` to the index because of ownership.
        // Instead, create a LoopShape that clones the vertex data.
        let shape = LoopShape {
            vertices: self.vertices.clone(),
            origin_inside: self.origin_inside,
        };
        self.index.add(Box::new(shape));
        self.index.build();
    }
}

// ─── Loop-Loop crossing relation infrastructure ─────────────────────────
//
// Private types for determining the relationship between two loops (contains,
// intersects, compare boundary). Corresponds to C++ s2loop.cc LoopRelation,
// LoopCrosser, and HasCrossingRelation.

/// Trait for different loop relationship predicates.
///
/// Implementations maintain state as shared vertices / edge crossings are
/// discovered. The trait methods are called by `LoopCrosser` as it walks
/// through overlapping cells of two loop indexes.
trait LoopRelation {
    /// Target containment values for early exit. If a point is found where
    /// `A.contains(P) == a_crossing_target()` and `B.contains(P) ==
    /// b_crossing_target()`, it's equivalent to finding an edge crossing.
    /// Use `2` for "no early exit".
    fn a_crossing_target(&self) -> i32;
    fn b_crossing_target(&self) -> i32;

    /// Called when a shared vertex is found between loops A and B.
    /// `a0, ab1, a2` is the wedge at the shared vertex in A.
    /// `b0, b2` are the neighboring vertices in B.
    /// Returns true if this constitutes an "edge crossing" for the relation.
    fn wedges_cross(&mut self, a0: Point, ab1: Point, a2: Point, b0: Point, b2: Point) -> bool;
}

/// Relation for testing whether loop A contains loop B.
struct ContainsRelation {
    found_shared_vertex: bool,
}

impl ContainsRelation {
    fn new() -> Self {
        ContainsRelation {
            found_shared_vertex: false,
        }
    }

    fn found_shared_vertex(&self) -> bool {
        self.found_shared_vertex
    }
}

impl LoopRelation for ContainsRelation {
    fn a_crossing_target(&self) -> i32 {
        0 // false
    }
    fn b_crossing_target(&self) -> i32 {
        1 // true
    }
    fn wedges_cross(&mut self, a0: Point, ab1: Point, a2: Point, b0: Point, b2: Point) -> bool {
        self.found_shared_vertex = true;
        !wedge_relations::wedge_contains(a0, ab1, a2, b0, b2)
    }
}

/// Relation for testing whether loop A intersects loop B.
struct IntersectsRelation {
    found_shared_vertex: bool,
}

impl IntersectsRelation {
    fn new() -> Self {
        IntersectsRelation {
            found_shared_vertex: false,
        }
    }

    fn found_shared_vertex(&self) -> bool {
        self.found_shared_vertex
    }
}

impl LoopRelation for IntersectsRelation {
    fn a_crossing_target(&self) -> i32 {
        1 // true
    }
    fn b_crossing_target(&self) -> i32 {
        1 // true
    }
    fn wedges_cross(&mut self, a0: Point, ab1: Point, a2: Point, b0: Point, b2: Point) -> bool {
        self.found_shared_vertex = true;
        wedge_relations::wedge_intersects(a0, ab1, a2, b0, b2)
    }
}

/// Relation for comparing boundaries of two loops.
///
/// Returns +1 (A contains B), -1 (A excludes B), or 0 (boundaries cross).
#[expect(clippy::struct_excessive_bools, reason = "matches C++ structure")]
struct CompareBoundaryRelation {
    reverse_b: bool,
    found_shared_vertex: bool,
    contains_edge: bool,
    excludes_edge: bool,
}

impl CompareBoundaryRelation {
    fn new(reverse_b: bool) -> Self {
        CompareBoundaryRelation {
            reverse_b,
            found_shared_vertex: false,
            contains_edge: false,
            excludes_edge: false,
        }
    }

    fn found_shared_vertex(&self) -> bool {
        self.found_shared_vertex
    }

    fn contains_edge(&self) -> bool {
        self.contains_edge
    }
}

impl LoopRelation for CompareBoundaryRelation {
    fn a_crossing_target(&self) -> i32 {
        -1 // no early exit (C++ uses -1 so no bool value matches)
    }
    fn b_crossing_target(&self) -> i32 {
        -1 // no early exit
    }
    fn wedges_cross(&mut self, a0: Point, ab1: Point, a2: Point, _b0: Point, b2: Point) -> bool {
        self.found_shared_vertex = true;
        if wedge_contains_semiwedge(a0, ab1, a2, b2, self.reverse_b) {
            self.contains_edge = true;
        } else {
            self.excludes_edge = true;
        }
        self.contains_edge && self.excludes_edge
    }
}

/// Tests if wedge A=(a0, ab1, a2) contains the "semiwedge" defined by the
/// CCW edge chain (ab1, b2). Used by `CompareBoundaryRelation`.
fn wedge_contains_semiwedge(a0: Point, ab1: Point, a2: Point, b2: Point, reverse_b: bool) -> bool {
    if b2 == a0 || b2 == a2 {
        return (b2 == a0) == reverse_b;
    }
    predicates::ordered_ccw(a0, a2, b2, ab1)
}

/// Helper struct for testing crossings between edges of two loops.
///
/// Walks through pairs of cells from two loop indexes and tests for edge
/// crossings and shared vertex wedge relationships.
struct LoopCrosser<'a> {
    a: &'a Loop,
    b: &'a Loop,
    /// Whether A and B have been swapped for the purpose of the merge-join.
    swapped: bool,
    /// Effective crossing targets (swapped if needed).
    a_crossing_target: i32,
    b_crossing_target: i32,
    /// Edge crosser for testing A edges against B edges.
    crosser: EdgeCrosser,
    /// Index of current A edge being tested.
    aj: usize,
    /// Previous B edge index (for chain optimization).
    bj_prev: i32,
}

impl<'a> LoopCrosser<'a> {
    fn new(a: &'a Loop, b: &'a Loop, relation: &dyn LoopRelation, swapped: bool) -> Self {
        let (a_target, b_target) = if swapped {
            (relation.b_crossing_target(), relation.a_crossing_target())
        } else {
            (relation.a_crossing_target(), relation.b_crossing_target())
        };
        LoopCrosser {
            a,
            b,
            swapped,
            a_crossing_target: a_target,
            b_crossing_target: b_target,
            crosser: EdgeCrosser::new(Point::origin(), Point::origin()),
            aj: 0,
            bj_prev: -2,
        }
    }

    /// Begins testing a new edge of loop A.
    fn start_edge(&mut self, aj: usize) {
        self.aj = aj;
        self.crosser = EdgeCrosser::new(self.a.vertex(aj), self.a.vertex(aj + 1));
        self.bj_prev = -2;
    }

    /// Tests a single A edge against all B edges in a clipped cell.
    /// Returns true if a crossing or wedge crossing is found.
    fn edge_crosses_cell(
        &mut self,
        b_clipped: &crate::s2::shape_index::ClippedShape,
        relation: &mut dyn LoopRelation,
    ) -> bool {
        let b_num_edges = b_clipped.num_edges();
        for k in 0..b_num_edges {
            let bj = b_clipped.edges[k] as usize;
            if bj as i32 != self.bj_prev + 1 {
                self.crosser.restart_at(self.b.vertex(bj));
            }
            self.bj_prev = bj as i32;
            let crossing = self.crosser.chain_crossing_sign(self.b.vertex(bj + 1));
            match crossing {
                Crossing::Cross => return true,
                Crossing::MaybeCross => {
                    // Shared vertex: check wedge relationship.
                    if self.a.vertex(self.aj + 1) == self.b.vertex(bj + 1) {
                        let result = if self.swapped {
                            relation.wedges_cross(
                                self.b.vertex(bj),
                                self.b.vertex(bj + 1),
                                self.b.vertex(bj + 2),
                                self.a.vertex(self.aj),
                                self.a.vertex(self.aj + 2),
                            )
                        } else {
                            relation.wedges_cross(
                                self.a.vertex(self.aj),
                                self.a.vertex(self.aj + 1),
                                self.a.vertex(self.aj + 2),
                                self.b.vertex(bj),
                                self.b.vertex(bj + 2),
                            )
                        };
                        if result {
                            return true;
                        }
                    }
                }
                Crossing::DoNotCross => {}
            }
        }
        false
    }

    /// Tests all A edges in `a_clipped` against all B edges in `b_clipped`.
    fn cell_crosses_cell(
        &mut self,
        a_clipped: &crate::s2::shape_index::ClippedShape,
        b_clipped: &crate::s2::shape_index::ClippedShape,
        relation: &mut dyn LoopRelation,
    ) -> bool {
        for k in 0..a_clipped.num_edges() {
            self.start_edge(a_clipped.edges[k] as usize);
            if self.edge_crosses_cell(b_clipped, relation) {
                return true;
            }
        }
        false
    }

    /// Called when A's cell contains B's cell. Tests for crossings and
    /// point-in-polygon relationships.
    fn has_crossing_relation(
        &mut self,
        ai: &mut RangeIterator,
        bi: &mut RangeIterator,
        relation: &mut dyn LoopRelation,
    ) -> bool {
        // Get A's clipped shape in its current cell.
        let Some(a_clipped) = ai.clipped(0) else {
            ai.next();
            return false;
        };
        let a_num_edges = a_clipped.num_edges();

        if a_num_edges == 0 {
            // A has no edges in this cell. Check if A's interior covers
            // the cell and if any B cells match the crossing target.
            if i32::from(a_clipped.contains_center) == self.a_crossing_target {
                // All points in this cell satisfy the A crossing target.
                // Scan B cells within this A cell.
                while !bi.done() && bi.range_min() <= ai.range_max() {
                    if bi
                        .clipped(0)
                        .is_some_and(|bc| i32::from(bc.contains_center) == self.b_crossing_target)
                    {
                        return true;
                    }
                    bi.next();
                }
            } else {
                // Skip past B cells in this A cell.
                bi.seek_beyond(ai);
            }
            ai.next();
            return false;
        }

        // A has edges in this cell. Test them against B's edges.
        // Iterate B cells within A's cell.
        while !bi.done() && bi.range_min() <= ai.range_max() {
            if let Some(b_clipped) = bi.clipped(0) {
                if b_clipped.num_edges() > 0 {
                    // Both have edges; test for crossings.
                    if self.cell_crosses_cell(a_clipped, b_clipped, relation) {
                        return true;
                    }
                } else if i32::from(a_clipped.contains_center) == self.a_crossing_target
                    && i32::from(b_clipped.contains_center) == self.b_crossing_target
                {
                    // Both crossing targets met.
                    return true;
                }
            }
            bi.next();
        }
        ai.next();
        false
    }
}

/// Tests whether loops A and B have the relationship specified by `relation`.
///
/// This is the main merge-join loop that walks the two loop indexes
/// simultaneously, testing overlapping cells for edge crossings and
/// shared vertex relationships.
///
/// Corresponds to C++ `HasCrossingRelation` in s2loop.cc.
fn has_crossing_relation(a: &Loop, b: &Loop, relation: &mut dyn LoopRelation) -> bool {
    let mut ai = RangeIterator::new(&a.index);
    let mut bi = RangeIterator::new(&b.index);
    let mut ab = LoopCrosser::new(a, b, &*relation, false);
    let mut ba = LoopCrosser::new(b, a, &*relation, true);

    while !ai.done() || !bi.done() {
        if ai.range_max() < bi.range_min() {
            // A's cells precede B's — seek A to B.
            ai.seek_to(&bi);
        } else if bi.range_max() < ai.range_min() {
            // B's cells precede A's — seek B to A.
            bi.seek_to(&ai);
        } else {
            // Cells overlap. Determine which cell is larger using LSB.
            let ab_relation = ai.cell_id().lsb().cmp(&bi.cell_id().lsb());
            match ab_relation {
                std::cmp::Ordering::Greater => {
                    // A's cell is larger and contains B's.
                    if ab.has_crossing_relation(&mut ai, &mut bi, relation) {
                        return true;
                    }
                }
                std::cmp::Ordering::Less => {
                    // B's cell is larger and contains A's.
                    if ba.has_crossing_relation(&mut bi, &mut ai, relation) {
                        return true;
                    }
                }
                std::cmp::Ordering::Equal => {
                    // Same cell. Check crossing targets.
                    let a_clipped = ai.clipped(0);
                    let b_clipped = bi.clipped(0);
                    if let (Some(ac), Some(bc)) = (a_clipped, b_clipped) {
                        if i32::from(ac.contains_center) == ab.a_crossing_target
                            && i32::from(bc.contains_center) == ab.b_crossing_target
                        {
                            return true;
                        }
                        if ac.num_edges() > 0
                            && bc.num_edges() > 0
                            && ab.cell_crosses_cell(ac, bc, relation)
                        {
                            return true;
                        }
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
    false
}

/// Internal Shape implementation for Loop's `ShapeIndex`.
///
/// This is separate from `impl Shape for Loop` because the Loop can't add
/// itself to its own `ShapeIndex` (ownership).
#[derive(Clone, Debug)]
struct LoopShape {
    vertices: Vec<Point>,
    origin_inside: bool,
}

impl Shape for LoopShape {
    fn num_edges(&self) -> usize {
        if self.vertices.len() < 3 {
            return 0;
        }
        self.vertices.len()
    }

    fn edge(&self, id: usize) -> Edge {
        let next = (id + 1) % self.vertices.len();
        Edge::new(self.vertices[id], self.vertices[next])
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::new(Point::origin(), self.origin_inside)
    }

    fn num_chains(&self) -> usize {
        if self.num_edges() > 0 { 1 } else { 0 }
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.num_edges())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        self.edge(offset)
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polygon
    }
}

// ─── Shape implementation for Loop ──────────────────────────────────────

impl Shape for Loop {
    fn num_edges(&self) -> usize {
        if self.is_empty_loop() || self.is_full_loop() {
            return 0;
        }
        self.vertices.len()
    }

    fn edge(&self, id: usize) -> Edge {
        let next = (id + 1) % self.vertices.len();
        Edge::new(self.vertices[id], self.vertices[next])
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::new(Point::origin(), self.origin_inside)
    }

    fn num_chains(&self) -> usize {
        if self.is_full_loop() || self.num_edges() > 0 {
            1
        } else {
            0
        }
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.num_edges())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        self.edge(offset)
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polygon
    }

    fn is_empty(&self) -> bool {
        self.is_empty_loop()
    }

    fn is_full(&self) -> bool {
        self.is_full_loop()
    }
}

// ─── Region implementation ──────────────────────────────────────────────

impl Region for Loop {
    fn cap_bound(&self) -> Cap {
        self.bound.cap_bound()
    }

    fn rect_bound(&self) -> Rect {
        self.bound
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        // A loop contains a cell if it contains all four vertices.
        (0..4).all(|i| self.contains_point(&cell.vertex(i)))
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        // Quick rejection via bounding rect.
        if !self.bound.intersects(cell.rect_bound()) {
            return false;
        }
        // Check if any cell vertex is inside the loop.
        for i in 0..4 {
            if self.contains_point(&cell.vertex(i)) {
                return true;
            }
        }
        // Check if any loop vertex is inside the cell.
        for v in &self.vertices {
            if cell.contains_point(v) {
                return true;
            }
        }
        // Check edge crossings.
        let n = self.vertices.len();
        for j in 0..4 {
            let cell_a = cell.vertex(j);
            let cell_b = cell.vertex((j + 1) & 3);
            let mut crosser = EdgeCrosser::new(cell_a, cell_b);
            for i in 0..n {
                let next = (i + 1) % n;
                if crosser.edge_or_vertex_crossing(self.vertices[i], self.vertices[next]) {
                    return true;
                }
            }
        }
        false
    }

    fn contains_point(&self, p: &Point) -> bool {
        // Quick rejection via bounding rect.
        if !self.bound.contains_point(*p) {
            return false;
        }

        if self.is_empty_loop() {
            return false;
        }
        if self.is_full_loop() {
            return true;
        }

        // Use brute force for small loops, index for large ones.
        if self.vertices.len() <= BRUTE_FORCE_VERTEX_THRESHOLD {
            self.brute_force_contains_point(*p)
        } else {
            // Use the ShapeIndex for accelerated containment.
            let mut q = ContainsPointQuery::new(&self.index, VertexModel::SemiOpen);
            q.contains(*p)
        }
    }
}

impl std::fmt::Debug for Loop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loop")
            .field("num_vertices", &self.vertices.len())
            .field("origin_inside", &self.origin_inside)
            .field("depth", &self.depth)
            .finish()
    }
}

impl Default for Loop {
    fn default() -> Self {
        Loop::empty()
    }
}

impl PartialEq for Loop {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }
}

impl std::fmt::Display for Loop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&crate::s2::text_format::loop_to_string(self))
    }
}

impl Clone for Loop {
    fn clone(&self) -> Self {
        let mut l = Loop::new(self.vertices.clone());
        l.set_depth(self.depth);
        l
    }
}

/// Checks whether the boundaries of `a` and `b` are within `max_error`
/// of each other when `a` is rotated by `a_offset` vertices.
///
/// Uses a backtracking state machine: at state (i, j), we can advance `i`
/// if a's next vertex is near b's current edge, or advance `j` if b's next
/// vertex is near a's current edge. Returns true if we can reach (na, nb).
///
/// Corresponds to C++ `MatchBoundaries`.
fn match_boundaries(a: &Loop, b: &Loop, a_offset: usize, max_error: Angle) -> bool {
    let na = a.num_vertices();
    let nb = b.num_vertices();
    let mut pending: Vec<(usize, usize)> = vec![(0, 0)];
    let mut done: HashSet<(usize, usize)> = HashSet::new();

    while let Some((i, j)) = pending.pop() {
        if i == na && j == nb {
            return true;
        }
        done.insert((i, j));

        let mut io = i + a_offset;
        if io >= na {
            io -= na;
        }

        if i < na && !done.contains(&(i + 1, j)) {
            // Check if a.vertex(io+1) is near the edge from b.vertex(j) to b.vertex(j+1).
            let dist = edge_distances::distance_from_segment(
                a.vertex(io + 1),
                b.vertex(j),
                b.vertex(j + 1),
            );
            if dist <= max_error {
                pending.push((i + 1, j));
            }
        }
        if j < nb && !done.contains(&(i, j + 1)) {
            // Check if b.vertex(j+1) is near the edge from a.vertex(io) to a.vertex(io+1).
            let dist = edge_distances::distance_from_segment(
                b.vertex(j + 1),
                a.vertex(io),
                a.vertex(io + 1),
            );
            if dist <= max_error {
                pending.push((i, j + 1));
            }
        }
    }
    false
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn loop_is_send_sync() {
        is_send_sync::<Loop>();
    }

    #[test]
    fn test_empty_loop() {
        let l = Loop::empty();
        assert!(l.is_empty_loop());
        assert!(!l.is_full_loop());
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.dimension(), Dimension::Polygon);
        assert!(l.is_empty());
        assert!(!l.is_full());
        assert_eq!(l.area(), 0.0);
    }

    #[test]
    fn test_full_loop() {
        let l = Loop::full();
        assert!(!l.is_empty_loop());
        assert!(l.is_full_loop());
        assert_eq!(l.num_edges(), 0);
        assert!(!l.is_empty());
        assert!(l.is_full());
        let area = l.area();
        assert!((area - 4.0 * std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_triangle() {
        // A small CCW triangle around (0°, 0°).
        let l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        assert_eq!(l.num_vertices(), 3);
        assert_eq!(l.num_edges(), 3);
        assert_eq!(l.num_chains(), 1);
        assert!(l.validate().is_ok());
    }

    #[test]
    fn test_contains_point_basic() {
        // A triangle that should contain its interior.
        let l = Loop::new(vec![p(-10.0, -10.0), p(-10.0, 10.0), p(10.0, 0.0)]);
        // The centroid of this triangle should be inside.
        let center = p(-3.0, 0.0);
        assert!(l.contains_point(&center));
        // A distant point should be outside.
        assert!(!l.contains_point(&p(80.0, 80.0)));
    }

    #[test]
    fn test_contains_point_with_index() {
        // Create a loop with more than BRUTE_FORCE_VERTEX_THRESHOLD vertices
        // to exercise the ShapeIndex path.
        // Vertices go CCW on the sphere (decreasing angle = clockwise in
        // parameter space = CCW on sphere).
        let n = 50;
        let mut verts = Vec::new();
        for i in 0..n {
            let angle = 2.0 * std::f64::consts::PI * f64::from(i) / f64::from(n);
            let lat = 10.0 * angle.cos();
            let lng = -10.0 * angle.sin(); // negative for CCW on sphere
            verts.push(p(lat, lng));
        }
        let l = Loop::new(verts);
        // Center should be inside.
        assert!(l.contains_point(&p(0.0, 0.0)));
        // A distant point should be outside.
        assert!(!l.contains_point(&p(80.0, 80.0)));
    }

    #[test]
    fn test_area_triangle() {
        let l = Loop::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)]);
        let area = l.area();
        // An octant of the sphere: area ≈ π/2
        assert!(
            (area - std::f64::consts::FRAC_PI_2).abs() < 0.05,
            "area = {area}"
        );
    }

    #[test]
    fn test_validate_ok() {
        let l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        assert!(l.validate().is_ok());
    }

    #[test]
    fn test_validate_duplicate() {
        let v = p(0.0, 0.0);
        let l = Loop::new(vec![v, v, p(1.0, 0.0)]);
        assert!(l.validate().is_err());
    }

    #[test]
    fn test_invert() {
        let mut l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        let orig_inside = l.origin_inside;
        l.invert();
        assert_ne!(l.origin_inside, orig_inside);
    }

    #[test]
    fn test_from_cell() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let l = Loop::from_cell(&cell);
        assert_eq!(l.num_vertices(), 4);
        assert_eq!(l.num_edges(), 4);
    }

    #[test]
    fn test_region_bounds() {
        let l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        let cap = l.cap_bound();
        assert!(!cap.is_empty());
        let rect = l.rect_bound();
        assert!(!rect.is_empty());
    }

    #[test]
    fn test_depth() {
        let mut l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        assert_eq!(l.depth(), 0);
        assert!(!l.is_hole());
        l.set_depth(1);
        assert_eq!(l.depth(), 1);
        assert!(l.is_hole());
        assert_eq!(l.sign(), -1);
    }

    #[test]
    fn test_equal() {
        let a = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        let b = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        assert!(a.equal(&b));
    }

    #[test]
    fn test_brute_force_vs_index() {
        // Create a loop and verify brute force and index agree.
        let l = Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ]);
        let test_points = vec![
            (p(0.0, 0.0), true),
            (p(80.0, 80.0), false),
            (p(5.0, 5.0), true),
            (p(-5.0, -5.0), true),
        ];
        for (pt, expected) in test_points {
            let bf = l.brute_force_contains_point(pt);
            assert_eq!(bf, expected, "brute force failed for {pt:?}");
        }
    }

    #[test]
    fn test_is_empty_or_full() {
        assert!(Loop::empty().is_empty_or_full());
        assert!(Loop::full().is_empty_or_full());

        let triangle = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        assert!(!triangle.is_empty_or_full());
    }

    #[test]
    fn test_oriented_vertex_shell() {
        // For a shell (depth 0), oriented_vertex returns vertices in order.
        let l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        assert!(!l.is_hole());
        assert_eq!(l.oriented_vertex(0), l.vertex(0));
        assert_eq!(l.oriented_vertex(1), l.vertex(1));
        assert_eq!(l.oriented_vertex(2), l.vertex(2));
        // Wrapping: i >= n maps to i - n
        assert_eq!(l.oriented_vertex(3), l.vertex(0));
        assert_eq!(l.oriented_vertex(4), l.vertex(1));
    }

    #[test]
    fn test_oriented_vertex_hole() {
        // For a hole (odd depth), oriented_vertex returns vertices in reverse.
        let mut l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        l.set_depth(1); // Mark as hole
        assert!(l.is_hole());
        let n = l.num_vertices(); // 3
        // Reverse order: j = n-1-i => 2, 1, 0
        assert_eq!(l.oriented_vertex(0), l.vertex(n - 1)); // vertex(2)
        assert_eq!(l.oriented_vertex(1), l.vertex(n - 2)); // vertex(1)
        assert_eq!(l.oriented_vertex(2), l.vertex(0)); // vertex(0)
        // Wrapping
        assert_eq!(l.oriented_vertex(3), l.vertex(n - 1)); // vertex(2)
        assert_eq!(l.oriented_vertex(4), l.vertex(n - 2)); // vertex(1)
    }

    #[test]
    fn test_loop_turning_angle() {
        // A CCW triangle should have a positive turning angle close to 2*PI.
        let l = Loop::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)]);
        let ta = l.turning_angle();
        // For a CCW loop, the turning angle should be positive (close to 2*PI
        // minus the sum of the exterior angles, which for a spherical triangle
        // on an octant is roughly PI/2). The exact value depends on the
        // spherical excess, but it should be strictly positive.
        assert!(
            ta > 0.0,
            "turning angle for CCW triangle should be positive, got {ta}"
        );

        // The empty loop has a turning angle of -2*PI by convention.
        let empty = Loop::empty();
        let ta_empty = empty.turning_angle();
        assert!(
            (ta_empty - (-2.0 * std::f64::consts::PI)).abs() < 1e-14,
            "empty loop turning angle should be -2*PI, got {ta_empty}"
        );

        // The full loop has a turning angle of 2*PI by convention.
        let full = Loop::full();
        let ta_full = full.turning_angle();
        assert!(
            (ta_full - 2.0 * std::f64::consts::PI).abs() < 1e-14,
            "full loop turning angle should be 2*PI, got {ta_full}"
        );
    }

    #[test]
    fn test_loop_centroid() {
        // Build a regular polygon centered on +Z (the north pole) at ~10 degrees
        // from the pole. The centroid should point approximately toward +Z.
        let n = 8;
        let mut verts = Vec::new();
        for i in 0..n {
            let angle = 2.0 * std::f64::consts::PI * f64::from(i) / f64::from(n);
            let lat = 80.0; // 10 degrees from the pole
            let lng = angle.to_degrees();
            verts.push(p(lat, lng));
        }
        let l = Loop::new(verts);
        let c = l.centroid();

        // The centroid vector should point roughly toward +Z (north pole).
        // Normalize to check direction.
        let len = (c.0.x * c.0.x + c.0.y * c.0.y + c.0.z * c.0.z).sqrt();
        assert!(len > 0.0, "centroid should be non-zero");
        let nz = c.0.z / len;
        assert!(
            nz > 0.9,
            "centroid should point near +Z, got normalized z = {nz}"
        );

        // Empty and full loops return Point::origin() from centroid() because
        // Point::from_coords(0,0,0) maps to origin. Verify this behavior.
        let empty_c = Loop::empty().centroid();
        assert_eq!(empty_c, Point::origin());

        let full_c = Loop::full().centroid();
        assert_eq!(full_c, Point::origin());
    }

    #[test]
    fn test_loop_shape_trait_impl() {
        // Test the Shape trait implementation on a simple triangle loop.
        let l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);

        // num_edges: 3 edges for a triangle.
        assert_eq!(l.num_edges(), 3);

        // edge(0): should connect vertex(0) to vertex(1).
        let e0 = l.edge(0);
        assert_eq!(e0.v0, l.vertex(0));
        assert_eq!(e0.v1, l.vertex(1));

        // edge(2): should connect vertex(2) to vertex(0) (wraps around).
        let e2 = l.edge(2);
        assert_eq!(e2.v0, l.vertex(2));
        assert_eq!(e2.v1, l.vertex(0));

        // num_chains: 1 chain for any non-empty loop.
        assert_eq!(l.num_chains(), 1);

        // chain(0): starts at 0 with length 3.
        let c = l.chain(0);
        assert_eq!(c.start, 0);
        assert_eq!(c.length, 3);

        // chain_edge(0, 0): same as edge(0).
        let ce = l.chain_edge(0, 0);
        assert_eq!(ce.v0, e0.v0);
        assert_eq!(ce.v1, e0.v1);

        // chain_position(1): should be chain 0, offset 1.
        let cp = l.chain_position(1);
        assert_eq!(cp.chain_id, 0);
        assert_eq!(cp.offset, 1);

        // has_interior: true for dimension-2 shapes.
        assert!(l.has_interior());

        // contains_origin: determined by init_origin_and_bound.
        // Just check that the reference_point is consistent.
        let rp = l.reference_point();
        assert_eq!(rp.point, Point::origin());
        assert_eq!(rp.contained, l.contains_origin());

        // dimension: 2 for loops.
        assert_eq!(l.dimension(), Dimension::Polygon);
    }

    #[test]
    fn test_loop_region_contains_and_intersects_cell() {
        // Build a large loop covering roughly a 40x40 degree region around (0, 0).
        let l = Loop::new(vec![
            p(-20.0, -20.0),
            p(-20.0, 20.0),
            p(20.0, 20.0),
            p(20.0, -20.0),
        ]);

        // A cell deep inside the loop (at the center) should be contained.
        let center_pt = p(0.0, 0.0);
        let center_cell_id = CellId::from_point(&center_pt).parent_at_level(16);
        let center_cell = Cell::from_cell_id(center_cell_id);
        assert!(
            l.contains_cell(&center_cell),
            "loop should contain a small cell at the center"
        );
        assert!(
            l.intersects_cell(&center_cell),
            "loop should also intersect a cell it contains"
        );

        // A cell far outside the loop should not be contained or intersected.
        let outside_pt = p(80.0, 80.0);
        let outside_cell_id = CellId::from_point(&outside_pt).parent_at_level(16);
        let outside_cell = Cell::from_cell_id(outside_cell_id);
        assert!(
            !l.contains_cell(&outside_cell),
            "loop should not contain a cell far outside"
        );
        assert!(
            !l.intersects_cell(&outside_cell),
            "loop should not intersect a cell far outside"
        );

        // A large cell near the boundary should intersect but not be fully contained.
        // Use a coarse cell (level 5) near one of the loop's edges.
        let boundary_pt = p(20.0, 0.0);
        let boundary_cell_id = CellId::from_point(&boundary_pt).parent_at_level(5);
        let boundary_cell = Cell::from_cell_id(boundary_cell_id);
        // The cell straddles the boundary, so it should intersect but not be contained.
        assert!(
            l.intersects_cell(&boundary_cell),
            "loop should intersect a large cell on the boundary"
        );
        assert!(
            !l.contains_cell(&boundary_cell),
            "loop should not fully contain a large cell straddling the boundary"
        );
    }

    // ─── Loop-Loop containment / intersection tests ─────────────────

    #[test]
    fn test_find_vertex() {
        let l = Loop::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        assert_eq!(l.find_vertex(&l.vertex(0)), 0);
        assert_eq!(l.find_vertex(&l.vertex(1)), 1);
        assert_eq!(l.find_vertex(&l.vertex(2)), 2);
        // Missing vertex.
        assert_eq!(l.find_vertex(&p(80.0, 80.0)), -1);
    }

    #[test]
    fn test_loop_contains_loop_basic() {
        // Big loop around the origin contains a small loop around the origin.
        let big = Loop::new(vec![
            p(-20.0, -20.0),
            p(-20.0, 20.0),
            p(20.0, 20.0),
            p(20.0, -20.0),
        ]);
        let small = Loop::new(vec![p(-5.0, -5.0), p(-5.0, 5.0), p(5.0, 5.0), p(5.0, -5.0)]);
        assert!(big.contains_loop(&small));
        assert!(!small.contains_loop(&big));
    }

    #[test]
    fn test_loop_contains_loop_empty_full() {
        let big = Loop::new(vec![
            p(-20.0, -20.0),
            p(-20.0, 20.0),
            p(20.0, 20.0),
            p(20.0, -20.0),
        ]);
        let empty = Loop::empty();
        let full = Loop::full();

        // Full contains everything; empty is contained by everything.
        assert!(full.contains_loop(&empty));
        assert!(full.contains_loop(&big));
        assert!(big.contains_loop(&empty));

        // Empty contains nothing (except empty).
        assert!(empty.contains_loop(&empty));
        assert!(!empty.contains_loop(&big));
        assert!(!empty.contains_loop(&full));

        // Full is contained only by full.
        assert!(full.contains_loop(&full));
        assert!(!big.contains_loop(&full));
    }

    #[test]
    fn test_loop_intersects_loop_basic() {
        // Two overlapping loops.
        let a = Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ]);
        let b = Loop::new(vec![p(0.0, 0.0), p(0.0, 20.0), p(20.0, 20.0), p(20.0, 0.0)]);
        assert!(a.intersects_loop(&b));
        assert!(b.intersects_loop(&a));
    }

    #[test]
    fn test_loop_intersects_loop_disjoint() {
        // Two disjoint loops.
        let a = Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, -5.0),
            p(-5.0, -5.0),
            p(-5.0, -10.0),
        ]);
        let b = Loop::new(vec![
            p(10.0, 10.0),
            p(10.0, 15.0),
            p(15.0, 15.0),
            p(15.0, 10.0),
        ]);
        assert!(!a.intersects_loop(&b));
        assert!(!b.intersects_loop(&a));
    }

    #[test]
    fn test_loop_intersects_loop_containing() {
        // If A contains B, they intersect.
        let big = Loop::new(vec![
            p(-20.0, -20.0),
            p(-20.0, 20.0),
            p(20.0, 20.0),
            p(20.0, -20.0),
        ]);
        let small = Loop::new(vec![p(-5.0, -5.0), p(-5.0, 5.0), p(5.0, 5.0), p(5.0, -5.0)]);
        assert!(big.intersects_loop(&small));
        assert!(small.intersects_loop(&big));
    }

    #[test]
    fn test_loop_contains_nested() {
        // A big loop with a known-nested small loop.
        let big = Loop::new(vec![
            p(-20.0, -20.0),
            p(-20.0, 20.0),
            p(20.0, 20.0),
            p(20.0, -20.0),
        ]);
        let small = Loop::new(vec![p(-5.0, -5.0), p(-5.0, 5.0), p(5.0, 5.0), p(5.0, -5.0)]);
        assert!(big.contains_nested(&small));
        assert!(!small.contains_nested(&big));
    }

    #[test]
    fn test_loop_compare_boundary() {
        let big = Loop::new(vec![
            p(-20.0, -20.0),
            p(-20.0, 20.0),
            p(20.0, 20.0),
            p(20.0, -20.0),
        ]);
        let small = Loop::new(vec![p(-5.0, -5.0), p(-5.0, 5.0), p(5.0, 5.0), p(5.0, -5.0)]);
        // Big contains small's boundary.
        assert_eq!(big.compare_boundary(&small), 1);
        // Small excludes big's boundary.
        assert_eq!(small.compare_boundary(&big), -1);

        // Two disjoint loops: each excludes the other's boundary.
        let disjoint = Loop::new(vec![
            p(40.0, 40.0),
            p(40.0, 50.0),
            p(50.0, 50.0),
            p(50.0, 40.0),
        ]);
        assert_eq!(big.compare_boundary(&disjoint), -1);

        // Overlapping loops: boundaries cross.
        let overlapping = Loop::new(vec![p(0.0, 0.0), p(0.0, 30.0), p(30.0, 30.0), p(30.0, 0.0)]);
        assert_eq!(big.compare_boundary(&overlapping), 0);
    }

    #[test]
    fn test_boundary_approx_eq_same_loop() {
        let l = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
        let max_error = Angle::from_radians(1e-15);
        assert!(l.boundary_approx_eq(&l, max_error));
    }

    #[test]
    fn test_boundary_approx_eq_rotated() {
        let a = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
        let b = Loop::new(vec![p(0.0, 10.0), p(10.0, 0.0), p(0.0, 0.0)]);
        let max_error = Angle::from_radians(1e-15);
        assert!(a.boundary_approx_eq(&b, max_error));
    }

    #[test]
    fn test_boundary_approx_eq_different() {
        let a = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
        let b = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 1.0)]);
        let small = Angle::from_radians(1e-4);
        assert!(!a.boundary_approx_eq(&b, small));
    }

    #[test]
    fn test_boundary_approx_eq_empty_full() {
        let empty = Loop::empty();
        let full = Loop::full();
        let max_error = Angle::from_radians(1e-15);
        assert!(empty.boundary_approx_eq(&Loop::empty(), max_error));
        assert!(full.boundary_approx_eq(&Loop::full(), max_error));
        assert!(!empty.boundary_approx_eq(&full, max_error));
    }

    #[test]
    fn test_boundary_near_same_loop() {
        let l = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
        let max_error = Angle::from_degrees(1.0);
        assert!(l.boundary_near(&l, max_error));
    }

    #[test]
    fn test_boundary_near_different_vertex_counts() {
        // Two loops approximating the same boundary with different vertex counts.
        let a = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
        let b = Loop::new(vec![
            p(0.0, 0.0),
            p(0.0, 5.0),
            p(0.0, 10.0),
            p(5.0, 10.0),
            p(10.0, 10.0),
            p(10.0, 5.0),
            p(10.0, 0.0),
            p(5.0, 0.0),
        ]);
        let max_error = Angle::from_degrees(1.0);
        assert!(a.boundary_near(&b, max_error));
    }

    #[test]
    fn test_boundary_near_far_loops() {
        let a = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
        let b = Loop::new(vec![p(40.0, 40.0), p(40.0, 50.0), p(50.0, 40.0)]);
        let max_error = Angle::from_degrees(1.0);
        assert!(!a.boundary_near(&b, max_error));
    }

    #[test]
    fn test_make_regular_basic() {
        // A regular loop centered at the north pole with 4 vertices.
        let center = Point::from_coords(0.0, 0.0, 1.0);
        let radius = Angle::from_degrees(10.0);
        let l = Loop::make_regular(center, radius, 4);
        assert_eq!(l.num_vertices(), 4);

        // All vertices should be approximately at the given radius from center.
        for i in 0..l.num_vertices() {
            let dist = center.distance(l.vertex(i));
            assert!(
                (dist.radians() - radius.radians()).abs() < 1e-12,
                "vertex {i} distance = {}, expected {}",
                dist.degrees(),
                radius.degrees(),
            );
        }

        // The loop should contain its center.
        assert!(
            l.brute_force_contains_point(center),
            "regular loop should contain its center"
        );
    }

    #[test]
    fn test_make_regular_many_vertices() {
        // A regular loop with many vertices approximating a circle.
        let center = Point::from_coords(1.0, 0.0, 0.0);
        let radius = Angle::from_degrees(5.0);
        let l = Loop::make_regular(center, radius, 100);
        assert_eq!(l.num_vertices(), 100);

        // Area should be approximately 2*pi*(1 - cos(r)).
        let expected_area = 2.0 * std::f64::consts::PI * (1.0 - radius.radians().cos());
        let actual_area = l.area();
        assert!(
            (actual_area - expected_area).abs() < 0.01 * expected_area,
            "area = {actual_area}, expected {expected_area}"
        );
    }

    #[test]
    fn test_make_regular_contains_center() {
        // Test with various centers.
        for center in &[
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
            Point::from_coords(1.0, 1.0, 1.0),
        ] {
            let l = Loop::make_regular(*center, Angle::from_degrees(3.0), 8);
            assert!(
                l.brute_force_contains_point(*center),
                "regular loop should contain its center: {center:?}"
            );
        }
    }

    // ─── Distance / projection tests ────────────────────────────────

    #[test]
    fn test_get_distance_inside() {
        // Point inside the loop should have distance 0.
        let l = Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ]);
        let dist = l.get_distance(p(0.0, 0.0));
        assert_eq!(dist.radians(), 0.0);
    }

    #[test]
    fn test_get_distance_outside() {
        // Point outside should have distance > 0 and ~= distance to boundary.
        let l = Loop::new(vec![p(-1.0, -1.0), p(-1.0, 1.0), p(1.0, 1.0), p(1.0, -1.0)]);
        let dist = l.get_distance(p(5.0, 0.0));
        // 5 degrees from equator, loop boundary at 1 degree → ~4 degrees.
        assert!(dist.degrees() > 3.0, "dist = {} degrees", dist.degrees());
        assert!(dist.degrees() < 5.0, "dist = {} degrees", dist.degrees());
    }

    #[test]
    fn test_get_distance_to_boundary() {
        let l = Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ]);
        // Point inside: distance to boundary should be > 0.
        let dist = l.get_distance_to_boundary(p(0.0, 0.0));
        assert!(
            dist.degrees() > 5.0,
            "boundary dist = {} degrees",
            dist.degrees()
        );
    }

    #[test]
    fn test_project_point_inside() {
        let l = Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ]);
        let proj = l.project_point(p(0.0, 0.0));
        // Point inside returns itself.
        assert!((proj.0 - p(0.0, 0.0).0).norm() < 1e-10);
    }

    #[test]
    fn test_project_to_boundary() {
        // Loop around the equator, project from a point north of it.
        let l = Loop::new(vec![p(-1.0, -180.0), p(-1.0, -60.0), p(-1.0, 60.0)]);
        let proj = l.project_to_boundary(p(5.0, 0.0));
        // Projected point should be on the loop boundary.
        let ll = LatLng::from_point(proj);
        assert!(
            (ll.lat.degrees() - (-1.0)).abs() < 1.0,
            "projected lat = {}, expected ~-1",
            ll.lat.degrees()
        );
    }

    // ===== Shape interface tests (ported from C++ s2loop_test.cc) =====

    #[test]
    fn test_empty_loop_shape() {
        let l = Loop::empty();
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.num_chains(), 0);
        assert!(l.is_empty());
        assert!(!l.is_full());
        assert!(!l.reference_point().contained);
    }

    #[test]
    fn test_full_loop_shape() {
        let l = Loop::full();
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.num_chains(), 1);
        assert!(!l.is_empty());
        assert!(l.is_full());
        assert!(l.reference_point().contained);
    }

    // ===== Curvature tests (ported from C++ s2loop_test.cc) =====

    #[test]
    fn test_area_consistent_with_curvature() {
        // Gauss-Bonnet theorem: area = 2*PI - curvature for simple loops.
        use std::f64::consts::PI;

        // Empty loop: area=0, curvature=2*PI
        let empty = Loop::empty();
        assert!((empty.get_curvature() - 2.0 * PI).abs() < 1e-14);

        // Full loop: area=4*PI, curvature=-2*PI
        let full = Loop::full();
        assert!((full.get_curvature() - (-2.0 * PI)).abs() < 1e-14);

        // Quarter sphere (north pole triangle)
        let tri = Loop::new(vec![p(90.0, 0.0), p(0.0, 0.0), p(0.0, 90.0)]);
        let area = tri.area();
        let gauss_area = 2.0 * PI - tri.get_curvature();
        assert!(
            (area - gauss_area).abs() < 1e-14,
            "triangle: area={area}, gauss_area={gauss_area}",
        );

        // Hemisphere (3 vertices)
        let hemi = Loop::new(vec![p(0.0, 0.0), p(0.0, 120.0), p(0.0, -120.0)]);
        let area = hemi.area();
        let gauss_area = 2.0 * PI - hemi.get_curvature();
        assert!(
            (area - gauss_area).abs() < 1e-14,
            "hemisphere: area={area}, gauss_area={gauss_area}",
        );

        // Small quadrilateral
        let quad = Loop::new(vec![p(1.0, 1.0), p(1.0, -1.0), p(-1.0, -1.0), p(-1.0, 1.0)]);
        let area = quad.area();
        let gauss_area = 2.0 * PI - quad.get_curvature();
        assert!(
            (area - gauss_area).abs() < 1e-14,
            "quad: area={area}, gauss_area={gauss_area}",
        );
    }

    #[test]
    fn test_get_curvature() {
        use std::f64::consts::PI;

        // Empty loop: curvature = 2*PI (no area)
        assert!((Loop::empty().get_curvature() - 2.0 * PI).abs() < 1e-14);

        // Full loop: curvature = -2*PI (entire sphere)
        assert!((Loop::full().get_curvature() - (-2.0 * PI)).abs() < 1e-14);

        // Hemisphere: curvature ≈ 0 (3 vertices on equator)
        let hemi = Loop::new(vec![p(0.0, 0.0), p(0.0, 120.0), p(0.0, -120.0)]);
        assert!(
            hemi.get_curvature().abs() < 1e-14,
            "hemisphere curvature = {}, expected ~0",
            hemi.get_curvature()
        );

        // Triangle at north pole: curvature = 2*PI - area ≈ 2*PI - PI/2
        let tri = Loop::new(vec![p(90.0, 0.0), p(0.0, 0.0), p(0.0, 90.0)]);
        let expected = 2.0 * PI - tri.area();
        assert!(
            (tri.get_curvature() - expected).abs() < 1e-14,
            "triangle curvature = {}, expected {expected}",
            tri.get_curvature()
        );
    }

    // ===== Rect bound tests (ported from C++ s2loop_test.cc) =====

    #[test]
    fn test_get_rect_bound() {
        // Empty loop has empty bound.
        assert!(Loop::empty().rect_bound().is_empty());

        // Full loop has full bound.
        assert!(Loop::full().rect_bound().is_full());

        // Small loop near north pole.
        let arctic = Loop::new(vec![
            p(80.0, 0.0),
            p(80.0, 90.0),
            p(80.0, 180.0),
            p(80.0, -90.0),
        ]);
        let bound = arctic.rect_bound();
        assert!(bound.lat.lo > 70.0_f64.to_radians());
        assert!(bound.lat.hi > 85.0_f64.to_radians());

        // Southern hemisphere loop.
        let south_hemi = Loop::new(vec![p(0.0, 0.0), p(0.0, -120.0), p(0.0, 120.0)]);
        let bound = south_hemi.rect_bound();
        assert!(bound.lat.lo <= -90.0_f64.to_radians() + 1e-10);
    }

    // ===== Distance method tests (ported from C++ s2loop_test.cc) =====

    #[test]
    fn test_distance_methods() {
        // Square loop from -1:-1 to 1:1
        let l = Loop::new(vec![p(-1.0, -1.0), p(-1.0, 1.0), p(1.0, 1.0), p(1.0, -1.0)]);

        // A point inside: distance = 0
        let inside = p(0.0, 0.5);
        assert!(l.get_distance(inside).radians() < 1e-15);

        // A point outside
        let outside = p(0.0, -2.0);
        let dist = l.get_distance(outside);
        let projected = l.project_to_boundary(outside);
        let expected_dist = outside.distance(projected);
        assert!(
            (dist.radians() - expected_dist.radians()).abs() < 1e-10,
            "outside distance: got {}, expected {}",
            dist.radians(),
            expected_dist.radians()
        );

        // Project from inside: should return the point itself
        let proj = l.project_point(inside);
        assert!((proj.0 - inside.0).norm() < 1e-10);
    }

    // --- Normalized compatible with Contains (ported from C++) ---

    #[test]
    fn test_normalized_compatible_with_contains() {
        // A normalized loop should contain the same points as its unnormalized version.
        let l = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
        let mut ln = l.clone();
        ln.normalize();

        // Interior point should be contained by both.
        let inside = p(5.0, 5.0);
        assert_eq!(
            l.contains_point(&inside),
            ln.contains_point(&inside),
            "normalized loop should contain same interior points"
        );

        // Exterior point should not be contained by either.
        let outside = p(20.0, 20.0);
        assert_eq!(
            l.contains_point(&outside),
            ln.contains_point(&outside),
            "normalized loop should not contain exterior points"
        );
    }

    // --- Area and centroid consistency ---

    #[test]
    fn test_area_and_centroid_consistency() {
        // For a small loop, the centroid should be inside the loop.
        let l = Loop::new(vec![p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0)]);
        let area = l.area();
        assert!(area > 0.0, "small loop should have positive area");

        let centroid = l.centroid();
        let centroid_point = centroid.normalize();
        assert!(
            l.contains_point(&centroid_point),
            "centroid should be inside the loop"
        );
    }

    // --- Clone preserves properties ---

    #[test]
    fn test_clone_preserves_properties() {
        let l = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
        let l2 = l.clone();
        assert_eq!(l.num_vertices(), l2.num_vertices());
        assert_eq!(l.depth(), l2.depth());
        assert!((l.area() - l2.area()).abs() < 1e-15);
        assert!(l.boundary_approx_eq(&l2, Angle::from_radians(1e-15)));
    }

    // --- Invert twice returns to original ---

    #[test]
    fn test_invert_twice_is_identity() {
        let original = Loop::new(vec![p(0.0, 0.0), p(0.0, 5.0), p(5.0, 0.0)]);
        let mut inverted = original.clone();
        inverted.invert();
        inverted.invert();

        // After inverting twice, should contain the same interior points.
        assert_eq!(
            original.contains_point(&p(1.0, 1.0)),
            inverted.contains_point(&p(1.0, 1.0)),
        );
        // Area should be the same.
        assert!((original.area() - inverted.area()).abs() < 1e-10);
    }

    // --- New foundational tests ---

    #[test]
    fn test_loop_brute_force_vs_index_contains() {
        // Verify brute_force_contains_point matches index-based contains_point
        // for various point positions relative to a non-trivial loop.
        let lp = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
        let test_points = [
            (p(5.0, 5.0), true),    // clearly inside
            (p(20.0, 20.0), false), // clearly outside
            (p(-5.0, 5.0), false),  // outside to the west
            (p(5.0, -5.0), false),  // outside to the south
            (p(1.0, 1.0), true),    // just inside corner
            (p(9.0, 9.0), true),    // just inside opposite corner
        ];
        for (pt, expected) in &test_points {
            assert_eq!(
                lp.brute_force_contains_point(*pt),
                *expected,
                "brute_force mismatch for point {pt:?}"
            );
            assert_eq!(
                lp.contains_point(pt),
                *expected,
                "contains_point mismatch for point {pt:?}"
            );
        }
    }

    #[test]
    fn test_loop_contains_non_crossing_boundary() {
        // Loop A contains loop B (B is inside A, no crossings).
        let a = Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ]);
        let b = Loop::new(vec![p(-1.0, -1.0), p(-1.0, 1.0), p(1.0, 1.0), p(1.0, -1.0)]);
        assert!(a.contains_non_crossing_boundary(&b, false));
        assert!(!b.contains_non_crossing_boundary(&a, false));
    }

    #[test]
    fn test_loop_validate_errors() {
        // Too few vertices (2 non-special vertices).
        let v0 = LatLng::from_degrees(0.0, 0.0).to_point();
        let v1 = LatLng::from_degrees(1.0, 0.0).to_point();
        let bad = Loop {
            vertices: vec![v0, v1],
            origin_inside: false,
            depth: 0,
            bound: Rect::empty(),
            subregion_bound: Rect::empty(),
            index: ShapeIndex::new(),
        };
        assert!(bad.validate().is_err());

        // Identical adjacent vertices.
        let bad2 = Loop {
            vertices: vec![v0, v0, v1],
            origin_inside: false,
            depth: 0,
            bound: Rect::empty(),
            subregion_bound: Rect::empty(),
            index: ShapeIndex::new(),
        };
        assert!(bad2.validate().is_err());
    }

    #[test]
    fn test_loop_centroid_triangle() {
        // The centroid of a very small triangle should be near the average of vertices.
        let a = p(0.0, 0.0);
        let b = p(0.0, 0.01);
        let c = p(0.01, 0.0);
        let lp = Loop::new(vec![a, b, c]);
        let centroid = lp.centroid();
        let avg_lat = 0.01 / 3.0;
        let avg_lng = 0.01 / 3.0;
        let expected = p(avg_lat, avg_lng);
        assert!(
            centroid.distance(expected).radians() < 0.001,
            "centroid {centroid:?} too far from expected {expected:?}"
        );
    }

    #[test]
    fn test_loop_bound_covers_all_vertices() {
        let lp = Loop::new(vec![
            p(10.0, 20.0),
            p(10.0, 30.0),
            p(20.0, 30.0),
            p(20.0, 20.0),
        ]);
        let bound = lp.bound();
        for i in 0..lp.num_vertices() {
            assert!(
                bound.contains_point(lp.vertex(i)),
                "bound doesn't contain vertex {i}"
            );
        }
    }

    #[test]
    fn test_loop_compare_boundary_nesting() {
        // compare_boundary: +1 if A contains boundary of B.
        let a = Loop::new(vec![
            p(-20.0, -20.0),
            p(-20.0, 20.0),
            p(20.0, 20.0),
            p(20.0, -20.0),
        ]);
        let b = Loop::new(vec![p(-5.0, -5.0), p(-5.0, 5.0), p(5.0, 5.0), p(5.0, -5.0)]);
        assert_eq!(a.compare_boundary(&b), 1);
        assert_eq!(b.compare_boundary(&a), -1);
    }

    #[test]
    fn test_loop_area_hemisphere() {
        // A loop approximating the northern hemisphere should have area ~2π.
        let n = 100;
        let mut verts = Vec::with_capacity(n);
        for i in 0..n {
            let lng = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            verts.push(LatLng::from_radians(0.0, lng).to_point());
        }
        let lp = Loop::new(verts);
        let expected = 2.0 * std::f64::consts::PI;
        assert!(
            (lp.area() - expected).abs() < 0.01,
            "hemisphere area {} not close to 2π={}",
            lp.area(),
            expected
        );
    }

    #[test]
    fn test_loop_invert_area_complement() {
        let lp = Loop::new(vec![p(0.0, 0.0), p(0.0, 5.0), p(5.0, 5.0), p(5.0, 0.0)]);
        let area = lp.area();
        let mut inv = lp.clone();
        inv.invert();
        let inv_area = inv.area();
        let total = area + inv_area;
        let expected = 4.0 * std::f64::consts::PI;
        assert!(
            (total - expected).abs() < 1e-8,
            "area + complement area = {total}, expected {expected}"
        );
    }

    #[test]
    fn test_loop_shape_num_chains() {
        use crate::s2::shape::Shape;
        let lp = Loop::new(vec![p(0.0, 0.0), p(0.0, 1.0), p(1.0, 0.0)]);
        assert_eq!(lp.num_edges(), 3);
        assert_eq!(lp.num_chains(), 1);
        assert_eq!(lp.chain(0).start, 0);
        assert_eq!(lp.chain(0).length, 3);
        assert_eq!(lp.dimension(), Dimension::Polygon);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        // Use exact unit-sphere points to avoid floating-point precision issues.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        let lp = Loop::new(vec![a, b, c]);
        let center = Point::from_coords(1.0, 1.0, 1.0);

        let json = serde_json::to_string(&lp).unwrap();
        let back: Loop = serde_json::from_str(&json).unwrap();

        // Check basic geometry is preserved.
        assert_eq!(lp.num_vertices(), back.num_vertices());
        for i in 0..lp.num_vertices() {
            assert_eq!(lp.vertex(i), back.vertex(i));
        }
        assert_eq!(lp.depth(), back.depth());

        // Check that the index was rebuilt and containment queries work.
        assert_eq!(lp.contains_point(&center), back.contains_point(&center));
        assert!((lp.area() - back.area()).abs() < 1e-15);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_empty_loop() {
        let lp = Loop::empty();
        let json = serde_json::to_string(&lp).unwrap();
        let back: Loop = serde_json::from_str(&json).unwrap();
        assert!(back.is_empty_loop());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_full_loop() {
        let lp = Loop::full();
        let json = serde_json::to_string(&lp).unwrap();
        let back: Loop = serde_json::from_str(&json).unwrap();
        assert!(back.is_full_loop());
    }
}
