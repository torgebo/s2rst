// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A closed latitude-longitude rectangle on the sphere.
//!
//! Corresponds to C++ `S2LatLngRect`, Go `s2.Rect`, Java `S2LatLngRect`.
//!
//! A `Rect` represents a closed latitude-longitude rectangle. It consists
//! of a latitude interval (an `r1::Interval`) and a longitude interval
//! (an `s1::Interval`). Latitude is measured from the equator towards
//! the poles (range [-π/2, π/2]), and longitude is measured from the
//! prime meridian (range [-π, π]).

#![cfg_attr(
    test,
    expect(
        clippy::cast_possible_truncation,
        reason = "sample counts (i32->usize) in test code — always positive"
    )
)]
use crate::r1;
use crate::s1;
use crate::s1::{Angle, ChordAngle};
use crate::s2::edge_crossings::{self, Crossing};
use crate::s2::edge_distances;
use crate::s2::{Cap, CellId, LatLng, Point};
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;

/// Identifies a vertex of an `s2::Rect` in CCW order.
///
/// ```text
///   UpperLeft (3) ─── UpperRight (2)
///        │                  │
///   LowerLeft (0) ─── LowerRight (1)
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RectVertex {
    /// Vertex 0: lower-left (lat.lo, lng.lo).
    #[default]
    LowerLeft = 0,
    /// Vertex 1: lower-right (lat.lo, lng.hi).
    LowerRight = 1,
    /// Vertex 2: upper-right (lat.hi, lng.hi).
    UpperRight = 2,
    /// Vertex 3: upper-left (lat.hi, lng.lo).
    UpperLeft = 3,
}

impl RectVertex {
    /// All four vertices in CCW order.
    pub const ALL: [RectVertex; 4] = [
        RectVertex::LowerLeft,
        RectVertex::LowerRight,
        RectVertex::UpperRight,
        RectVertex::UpperLeft,
    ];

    /// Returns the next vertex in CCW order.
    #[inline]
    pub fn next(self) -> RectVertex {
        RectVertex::ALL[((self as usize) + 1) & 3]
    }

    /// Returns the previous vertex in CCW order.
    #[inline]
    pub fn prev(self) -> RectVertex {
        RectVertex::ALL[((self as usize) + 3) & 3]
    }

    /// Returns an iterator over all four vertices in CCW order,
    /// starting from `LowerLeft`.
    pub fn iter() -> impl Iterator<Item = RectVertex> {
        RectVertex::ALL.iter().copied()
    }
}

/// Valid latitude range: [-π/2, π/2].
const VALID_LAT_RANGE: r1::Interval = r1::Interval {
    lo: -FRAC_PI_2,
    hi: FRAC_PI_2,
};

/// A closed latitude-longitude rectangle on the sphere.
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::s2::{LatLng, Rect};
///
/// // Build a rectangle from two corner points.
/// let lo = LatLng::from_degrees(40.0, -75.0);
/// let hi = LatLng::from_degrees(42.0, -73.0);
/// let rect = Rect::from_lat_lng(lo).add_point(hi);
/// assert!(rect.is_valid());
/// assert!(!rect.is_empty());
///
/// // Containment: a point inside the rectangle.
/// let nyc = LatLng::from_degrees(40.7128, -74.0060);
/// assert!(rect.contains_lat_lng(nyc));
///
/// // A point outside the rectangle.
/// let london = LatLng::from_degrees(51.5074, -0.1278);
/// assert!(!rect.contains_lat_lng(london));
///
/// // Area of the rectangle (in steradians on the unit sphere).
/// let area = rect.area();
/// assert!(area > 0.0);
///
/// // Union of two rectangles.
/// let other = Rect::from_lat_lng(london);
/// let merged = rect.union(other);
/// assert!(merged.contains_lat_lng(nyc));
/// assert!(merged.contains_lat_lng(london));
/// ```
#[must_use]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    /// Latitude interval in radians, range [-π/2, π/2].
    pub lat: r1::Interval,
    /// Longitude interval in radians.
    pub lng: s1::Interval,
}

impl PartialEq for Rect {
    fn eq(&self, other: &Self) -> bool {
        self.lat == other.lat && self.lng == other.lng
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::empty()
    }
}

impl Rect {
    // --- Constructors ---

    /// Creates a rect from latitude and longitude intervals.
    #[inline]
    pub fn new(lat: r1::Interval, lng: s1::Interval) -> Self {
        Rect { lat, lng }
    }

    /// Returns the empty rectangle.
    #[inline]
    pub fn empty() -> Self {
        Rect {
            lat: r1::Interval::empty(),
            lng: s1::Interval::empty(),
        }
    }

    /// Returns the full rectangle covering the entire sphere.
    #[inline]
    pub fn full() -> Self {
        Rect {
            lat: VALID_LAT_RANGE,
            lng: s1::Interval::full(),
        }
    }

    /// Constructs a rectangle containing a single point.
    pub fn from_lat_lng(ll: LatLng) -> Self {
        Rect {
            lat: r1::Interval::from_point(ll.lat.radians()),
            lng: s1::Interval::from_point(ll.lng.radians()),
        }
    }

    /// Constructs a rectangle with the given center and size.
    ///
    /// The latitude interval is clamped to [-π/2, π/2], and the longitude
    /// interval becomes full if the longitude size is ≥ 2π.
    pub fn from_center_size(center: LatLng, size: LatLng) -> Self {
        let half = LatLng::new(size.lat * 0.5, size.lng * 0.5);
        Self::from_lat_lng(center).expanded(half)
    }

    // --- Predicates ---

    /// Reports whether the rectangle is valid.
    #[inline]
    pub fn is_valid(self) -> bool {
        self.lat.lo.abs() <= FRAC_PI_2
            && self.lat.hi.abs() <= FRAC_PI_2
            && self.lng.is_valid()
            && self.lat.is_empty() == self.lng.is_empty()
    }

    /// Reports whether the rectangle is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.lat.is_empty()
    }

    /// Reports whether the rectangle is full.
    #[inline]
    pub fn is_full(self) -> bool {
        self.lat == VALID_LAT_RANGE && self.lng.is_full()
    }

    /// Reports whether the rectangle is a single point.
    #[inline]
    pub fn is_point(self) -> bool {
        self.lat.lo == self.lat.hi && self.lng.lo == self.lng.hi
    }

    // --- Accessors ---

    /// Returns the specified vertex of the rectangle.
    pub fn vertex(self, v: RectVertex) -> LatLng {
        match v {
            RectVertex::LowerLeft => LatLng::from_radians(self.lat.lo, self.lng.lo),
            RectVertex::LowerRight => LatLng::from_radians(self.lat.lo, self.lng.hi),
            RectVertex::UpperRight => LatLng::from_radians(self.lat.hi, self.lng.hi),
            RectVertex::UpperLeft => LatLng::from_radians(self.lat.hi, self.lng.lo),
        }
    }

    /// Returns the low corner of the rectangle.
    #[inline]
    pub fn lo(self) -> LatLng {
        LatLng::from_radians(self.lat.lo, self.lng.lo)
    }

    /// Returns the high corner of the rectangle.
    #[inline]
    pub fn hi(self) -> LatLng {
        LatLng::from_radians(self.lat.hi, self.lng.hi)
    }

    /// Returns the center of the rectangle.
    #[inline]
    pub fn center(self) -> LatLng {
        LatLng::from_radians(self.lat.center(), self.lng.center())
    }

    /// Returns the size of the rectangle as a `LatLng`.
    #[inline]
    pub fn size(self) -> LatLng {
        LatLng::from_radians(self.lat.length(), self.lng.length())
    }

    /// Returns the surface area of the rectangle on the unit sphere.
    pub fn area(self) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        let cap_diff = (self.lat.hi.sin() - self.lat.lo.sin()).abs();
        self.lng.length() * cap_diff
    }

    // --- Containment ---

    /// Reports whether this rectangle contains the given `LatLng`.
    #[inline]
    pub fn contains_lat_lng(self, ll: LatLng) -> bool {
        if !ll.is_valid() {
            return false;
        }
        self.lat.contains(ll.lat.radians()) && self.lng.contains(ll.lng.radians())
    }

    /// Reports whether this rectangle contains the given Point.
    #[inline]
    pub fn contains_point(self, p: Point) -> bool {
        self.contains_lat_lng(LatLng::from_point(p))
    }

    /// Reports whether this rectangle contains the other rectangle.
    #[inline]
    pub fn contains(self, other: Rect) -> bool {
        self.lat.contains_interval(other.lat) && self.lng.contains_interval(other.lng)
    }

    /// Reports whether this rectangle intersects the other rectangle.
    #[inline]
    pub fn intersects(self, other: Rect) -> bool {
        self.lat.intersects(other.lat) && self.lng.intersects(other.lng)
    }

    /// Constructs the minimal bounding rectangle containing two points.
    pub fn from_point_pair(a: LatLng, b: LatLng) -> Self {
        Rect {
            lat: r1::Interval::from_point_pair(a.lat.radians(), b.lat.radians()),
            lng: s1::Interval::from_point_pair(a.lng.radians(), b.lng.radians()),
        }
    }

    /// Returns the full latitude interval [-π/2, π/2].
    #[inline]
    pub fn full_lat() -> r1::Interval {
        VALID_LAT_RANGE
    }

    /// Returns the full longitude interval [-π, π].
    #[inline]
    pub fn full_lng() -> s1::Interval {
        s1::Interval::full()
    }

    /// Reports whether the longitude interval of this rectangle is inverted.
    #[inline]
    pub fn is_inverted(self) -> bool {
        self.lng.is_inverted()
    }

    /// Reports whether the interior of this rectangle contains the given `LatLng`.
    #[inline]
    pub fn interior_contains_lat_lng(self, ll: LatLng) -> bool {
        self.lat.interior_contains(ll.lat.radians()) && self.lng.interior_contains(ll.lng.radians())
    }

    /// Reports whether the interior of this rectangle contains the given Point.
    #[inline]
    pub fn interior_contains_point(self, p: Point) -> bool {
        self.interior_contains_lat_lng(LatLng::from_point(p))
    }

    /// Reports whether the interior of this rectangle contains all of the
    /// other rectangle, including its boundary.
    #[inline]
    pub fn interior_contains(self, other: Rect) -> bool {
        self.lat.interior_contains_interval(other.lat)
            && self.lng.interior_contains_interval(other.lng)
    }

    /// Reports whether the interior of this rectangle intersects any point
    /// (including the boundary) of the other rectangle.
    #[inline]
    pub fn interior_intersects(self, other: Rect) -> bool {
        self.lat.interior_intersects(other.lat) && self.lng.interior_intersects(other.lng)
    }

    // --- Set operations ---

    /// Increases the size of the rectangle to include the given point.
    pub fn add_point(self, ll: LatLng) -> Rect {
        if !ll.is_valid() {
            return self;
        }
        Rect {
            lat: self.lat.add_point(ll.lat.radians()),
            lng: self.lng.add_point(ll.lng.radians()),
        }
    }

    /// Returns the rectangle unmodified if it does not include either pole.
    /// If it includes either pole, returns an expansion along the longitudinal
    /// range to include all possible representations of the contained poles.
    pub fn polar_closure(self) -> Rect {
        if self.lat.lo == -FRAC_PI_2 || self.lat.hi == FRAC_PI_2 {
            return Rect {
                lat: self.lat,
                lng: s1::Interval::full(),
            };
        }
        self
    }

    /// Returns a rectangle expanded by margin.lat on each side in the
    /// latitude direction and by margin.lng on each side in the longitude
    /// direction.
    pub fn expanded(self, margin: LatLng) -> Rect {
        let lat = self.lat.expanded(margin.lat.radians());
        let lng = self.lng.expanded(margin.lng.radians());

        if lat.is_empty() || lng.is_empty() {
            return Self::empty();
        }

        Rect {
            lat: lat.intersection(VALID_LAT_RANGE),
            lng,
        }
    }

    /// Returns a rectangle that has been expanded by the given distance
    /// on all sides. For positive distances, the expansion is computed by
    /// building a cap at each vertex and taking the union of all bounding
    /// rectangles. For negative distances (shrinking), a simpler linear
    /// approach is used.
    ///
    /// Corresponds to C++ `S2LatLngRect::ExpandedByDistance`.
    pub fn expanded_by_distance(self, distance: Angle) -> Rect {
        if distance.radians() >= 0.0 {
            // Build a cap at each vertex and take the union.
            let radius = ChordAngle::from_angle(distance);
            let mut r = self;
            for v in RectVertex::iter() {
                let cap = Cap::from_center_chord_angle(Point::from(self.vertex(v)), radius);
                r = r.union(cap.rect_bound());
            }
            r
        } else {
            // Shrink the rectangle.
            let lat_lo = if self.lat.lo <= VALID_LAT_RANGE.lo && self.lng.is_full() {
                VALID_LAT_RANGE.lo
            } else {
                self.lat.lo - distance.radians()
            };
            let lat_hi = if self.lat.hi >= VALID_LAT_RANGE.hi && self.lng.is_full() {
                VALID_LAT_RANGE.hi
            } else {
                self.lat.hi + distance.radians()
            };
            let lat_result = r1::Interval {
                lo: lat_lo,
                hi: lat_hi,
            };
            if lat_result.is_empty() {
                return Self::empty();
            }
            let max_abs_lat = (-lat_result.lo).max(lat_result.hi);
            let sin_a = (-distance.radians()).sin();
            let sin_c = max_abs_lat.cos();
            let max_lng_margin = if sin_a < sin_c {
                (sin_a / sin_c).asin()
            } else {
                FRAC_PI_2
            };
            let lng_result = self.lng.expanded(-max_lng_margin);
            if lng_result.is_empty() {
                return Self::empty();
            }
            Rect {
                lat: lat_result,
                lng: lng_result,
            }
        }
    }

    /// Returns the smallest rectangle containing the union of this
    /// rectangle and the other rectangle.
    pub fn union(self, other: Rect) -> Rect {
        Rect {
            lat: self.lat.union(other.lat),
            lng: self.lng.union(other.lng),
        }
    }

    /// Returns the smallest rectangle containing the intersection of
    /// this rectangle and the other rectangle.
    pub fn intersection(self, other: Rect) -> Rect {
        let lat = self.lat.intersection(other.lat);
        let lng = self.lng.intersection(other.lng);

        if lat.is_empty() || lng.is_empty() {
            return Self::empty();
        }
        Rect { lat, lng }
    }

    // --- Subregion expansion ---

    /// Returns a bound `B` such that if `A.contains(B)` (where `A` is a
    /// `Rect`), then `A` contains all regions whose bounds are `B`.
    ///
    /// This accounts for the fact that `AddPoint` expands bounds by
    /// varying amounts depending on where a point lies on the sphere.
    /// In particular, it handles edge effects near poles and the
    /// antimeridian.
    ///
    /// Corresponds to C++ `ExpandForSubregions()` in
    /// `s2latlng_rect_bounder.cc`.
    pub fn expand_for_subregions(self) -> Rect {
        if self.is_empty() {
            return self;
        }

        // Constants from C++: nearly-antipodal thresholds.
        const DBL_EPSILON: f64 = 2.220_446_049_250_313e-16;

        // Check whether the bound B can contain nearly-antipodal points
        // (within 4.309 * DBL_EPSILON). If so, return full rect.
        //
        // Case 1: B does not straddle the equator.
        let lng_gap = (PI - self.lng.length() - 2.5 * DBL_EPSILON).max(0.0);
        let min_abs_lat = self.lat.lo.max(-self.lat.hi);
        // Euclidean lower bound on distance z >= (x+y)/sqrt(2).
        if 2.0 * min_abs_lat + lng_gap < 1.354e-15 {
            return Rect::full();
        }

        // Case 2: lng span <= PI/2, check distance from B to both poles.
        let lat_gap1 = FRAC_PI_2 + self.lat.lo; // distance to south pole
        let lat_gap2 = FRAC_PI_2 - self.lat.hi; // distance to north pole
        if lng_gap >= FRAC_PI_2 {
            // Obtuse triangle: z >= (x + y) / sqrt(2).
            if lat_gap1 + lat_gap2 < 1.687e-15 {
                return Rect::full();
            }
        } else {
            // Case 3: general case using spherical law of sines.
            if lat_gap1.max(lat_gap2) * lng_gap < 1.765e-15 {
                return Rect::full();
            }
        }

        // Expand latitude by 9 * DBL_EPSILON (accounts for AddPoint error
        // in both directions).
        let lat_expansion = 9.0 * DBL_EPSILON;
        // Expand longitude by PI if the gap is <= 0 (subregion edges could
        // span nearly full longitude), otherwise 0.
        let lng_expansion = if lng_gap <= 0.0 { PI } else { 0.0 };

        let expanded = self.expanded(LatLng::from_radians(lat_expansion, lng_expansion));
        expanded.polar_closure()
    }

    // --- Region-like methods ---

    /// Returns a bounding cap for this rectangle.
    pub fn cap_bound(self) -> Cap {
        if self.is_empty() {
            return Cap::empty();
        }

        let (pole_z, pole_angle) = if self.lat.hi + self.lat.lo < 0.0 {
            (-1.0, FRAC_PI_2 + self.lat.hi)
        } else {
            (1.0, FRAC_PI_2 - self.lat.lo)
        };
        // Ensure that the bounding cap is conservative taking into account
        // errors in the arithmetic and the S1Angle/S1ChordAngle conversion.
        let pole_cap = Cap::from_center_angle(
            Point::new(crate::r3::Vector {
                x: 0.0,
                y: 0.0,
                z: pole_z,
            }),
            Angle::from_radians((1.0 + 2.0 * f64::EPSILON) * pole_angle),
        );

        // For bounding rectangles that span ≤ 180° in longitude, the maximum
        // cap size is achieved at one of the rectangle vertices.  For
        // rectangles that are larger than 180°, we punt and always return a
        // bounding cap centered at one of the two poles.
        let lng_span = self.lng.hi - self.lng.lo;
        if lng_span.rem_euclid(2.0 * PI) >= 0.0 && lng_span <= PI {
            let mut mid_cap = Cap::from_point(self.center().to_point());
            for v in RectVertex::iter() {
                mid_cap = mid_cap.add_point(self.vertex(v).to_point());
            }
            if mid_cap.height() < pole_cap.height() {
                return mid_cap;
            }
        }
        pole_cap
    }

    /// Returns itself as the bounding rectangle.
    #[inline]
    pub fn rect_bound(self) -> Rect {
        self
    }

    /// Returns a small set of cells that cover this rectangle.
    pub fn cell_union_bound(self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    // --- Centroid ---

    /// Returns the true centroid of the rectangle multiplied by its surface area.
    pub fn centroid(self) -> Point {
        if self.is_empty() {
            return Point::default();
        }

        let z1 = self.lat.lo.sin();
        let z2 = self.lat.hi.sin();
        let r1 = self.lat.lo.cos();
        let r2 = self.lat.hi.cos();

        let alpha = 0.5 * self.lng.length();
        let r0 = alpha.sin() * (r2 * z2 - r1 * z1 + self.lat.length());
        let lng = self.lng.center();
        let z = alpha * (z2 + z1) * (z2 - z1);

        Point::new(crate::r3::Vector {
            x: r0 * lng.cos(),
            y: r0 * lng.sin(),
            z,
        })
    }

    // --- Boundary intersection ---

    /// Reports whether the edge from v0 to v1 intersects the boundary of
    /// this rectangle. The edge may partially overlap the boundary.
    ///
    /// Corresponds to C++ `S2LatLngRect::BoundaryIntersects`.
    pub fn boundary_intersects(self, v0: Point, v1: Point) -> bool {
        if self.is_empty() {
            return false;
        }
        if !self.lng.is_full() {
            if intersects_lng_edge(v0, v1, self.lat, self.lng.lo) {
                return true;
            }
            if intersects_lng_edge(v0, v1, self.lat, self.lng.hi) {
                return true;
            }
        }
        if self.lat.lo != -FRAC_PI_2 && intersects_lat_edge(v0, v1, self.lat.lo, self.lng) {
            return true;
        }
        if self.lat.hi != FRAC_PI_2 && intersects_lat_edge(v0, v1, self.lat.hi, self.lng) {
            return true;
        }
        false
    }

    // --- Distance ---

    /// Returns the minimum distance (in angle) from a point to this rectangle.
    ///
    /// The rectangle must not be empty, and the point must be valid.
    ///
    /// Corresponds to C++ `S2LatLngRect::GetDistance(S2LatLng)`.
    pub fn get_distance_to_latlng(self, p: LatLng) -> Angle {
        debug_assert!(!self.is_empty());
        debug_assert!(p.is_valid());

        if self.lng.contains(p.lng.radians()) {
            return Angle::from_radians(
                (p.lat.radians() - self.lat.hi)
                    .max(self.lat.lo - p.lat.radians())
                    .max(0.0),
            );
        }

        let interval = s1::Interval::new(self.lng.hi, self.lng.complement_center());
        let a_lng = if interval.contains(p.lng.radians()) {
            self.lng.hi
        } else {
            self.lng.lo
        };
        let lo = LatLng::from_radians(self.lat.lo, a_lng).to_point();
        let hi = LatLng::from_radians(self.lat.hi, a_lng).to_point();
        edge_distances::distance_from_segment(p.to_point(), lo, hi)
    }

    /// Returns the minimum distance (in angle) between this rectangle and
    /// the other rectangle.
    ///
    /// Both rectangles must not be empty.
    ///
    /// Corresponds to C++ `S2LatLngRect::GetDistance(S2LatLngRect)`.
    pub fn get_distance(self, other: Rect) -> Angle {
        debug_assert!(!self.is_empty());
        debug_assert!(!other.is_empty());

        // First, handle the trivial cases where the longitude intervals overlap.
        if self.lng.intersects(other.lng) {
            if self.lat.intersects(other.lat) {
                return Angle::from_radians(0.0);
            }
            let (lo, hi) = if self.lat.lo > other.lat.hi {
                (other.lat.hi, self.lat.lo)
            } else {
                (self.lat.hi, other.lat.lo)
            };
            return Angle::from_radians(hi - lo);
        }

        // The longitude intervals don't overlap. Find the nearest longitude edges.
        let lo_hi = s1::Interval::from_point_pair(self.lng.lo, other.lng.hi);
        let hi_lo = s1::Interval::from_point_pair(self.lng.hi, other.lng.lo);
        let (a_lng, b_lng) = if lo_hi.length() < hi_lo.length() {
            (self.lng.lo, other.lng.hi)
        } else {
            (self.lng.hi, other.lng.lo)
        };

        let a_lo = LatLng::from_radians(self.lat.lo, a_lng).to_point();
        let a_hi = LatLng::from_radians(self.lat.hi, a_lng).to_point();
        let b_lo = LatLng::from_radians(other.lat.lo, b_lng).to_point();
        let b_hi = LatLng::from_radians(other.lat.hi, b_lng).to_point();
        min_angle(
            edge_distances::distance_from_segment(a_lo, b_lo, b_hi),
            min_angle(
                edge_distances::distance_from_segment(a_hi, b_lo, b_hi),
                min_angle(
                    edge_distances::distance_from_segment(b_lo, a_lo, a_hi),
                    edge_distances::distance_from_segment(b_hi, a_lo, a_hi),
                ),
            ),
        )
    }

    // --- Hausdorff distance ---

    /// Returns the Hausdorff distance between this rectangle and the other
    /// rectangle, which is defined as:
    ///   `max(directed_hausdorff(self, other), directed_hausdorff(other, self))`
    ///
    /// Corresponds to C++ `S2LatLngRect::GetHausdorffDistance`.
    pub fn get_hausdorff_distance(self, other: Rect) -> Angle {
        max_angle(
            self.get_directed_hausdorff_distance(other),
            other.get_directed_hausdorff_distance(self),
        )
    }

    /// Returns the directed Hausdorff distance from this rectangle to the
    /// other rectangle, which is the maximum over all points p in self of
    /// the distance from p to other.
    ///
    /// Corresponds to C++ `S2LatLngRect::GetDirectedHausdorffDistance`.
    pub fn get_directed_hausdorff_distance(self, other: Rect) -> Angle {
        if self.is_empty() {
            return Angle::from_radians(0.0);
        }
        if other.is_empty() {
            return Angle::from_radians(PI); // maximum possible distance on S2
        }
        let lng_distance = self.lng.directed_hausdorff_distance(other.lng);
        debug_assert!(lng_distance >= 0.0);
        get_directed_hausdorff_distance_lng(lng_distance, self.lat, other.lat)
    }

    // --- Approximate equality ---

    /// Reports whether the latitude and longitude intervals of the two
    /// rectangles are the same up to a small tolerance.
    pub fn approx_eq(self, other: Rect) -> bool {
        self.lat.approx_eq_with(other.lat, 1e-15) && self.lng.approx_eq_with(other.lng, 1e-15)
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Lo{}, Hi{}]", self.lo(), self.hi())
    }
}

// --- Private free functions for boundary/distance computation ---

#[inline]
fn min_angle(a: Angle, b: Angle) -> Angle {
    if a.radians() <= b.radians() { a } else { b }
}

#[inline]
fn max_angle(a: Angle, b: Angle) -> Angle {
    if a.radians() >= b.radians() { a } else { b }
}

/// Reports whether the edge AB intersects the given edge of constant longitude.
/// Longitude edges are geodesics (great circles), so this is a simple crossing test.
fn intersects_lng_edge(a: Point, b: Point, lat: r1::Interval, lng: f64) -> bool {
    let c = LatLng::from_radians(lat.lo, lng).to_point();
    let d = LatLng::from_radians(lat.hi, lng).to_point();
    edge_crossings::crossing_sign(a, b, c, d) == Crossing::Cross
}

/// Reports whether the edge AB intersects the given edge of constant latitude.
/// Lines of constant latitude are curves on the sphere, so this is more complex.
fn intersects_lat_edge(a: Point, b: Point, lat: f64, lng: s1::Interval) -> bool {
    // Compute the normal to the plane AB that points vaguely north.
    let mut z = edge_crossings::robust_cross_prod(a, b).0.normalize();
    if z.z < 0.0 {
        z = -z;
    }

    // Extend this to an orthonormal frame (x,y,z) where x is the direction
    // where the great circle through AB achieves its maximum latitude.
    let north = crate::r3::Vector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    let y = edge_crossings::robust_cross_prod(Point(z), Point(north))
        .0
        .normalize();
    let x = y.cross(z);
    debug_assert!(x.z >= 0.0);

    // Compute the angle "theta" from the x-axis where the great circle
    // intersects the given line of latitude.
    let sin_lat = lat.sin();
    if sin_lat.abs() >= x.z {
        return false; // The great circle does not reach the given latitude.
    }
    debug_assert!(x.z > 0.0);
    let cos_theta = sin_lat / x.z;
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
    let theta = sin_theta.atan2(cos_theta);

    // The candidate intersection points are located ±theta in the x-y plane.
    // Check that each is contained in both the edge AB and the longitude interval.
    let ab_theta =
        s1::Interval::from_point_pair(a.0.dot(y).atan2(a.0.dot(x)), b.0.dot(y).atan2(b.0.dot(x)));

    if ab_theta.contains(theta) {
        let isect = x * cos_theta + y * sin_theta;
        if lng.contains(isect.y.atan2(isect.x)) {
            return true;
        }
    }
    if ab_theta.contains(-theta) {
        let isect = x * cos_theta - y * sin_theta;
        if lng.contains(isect.y.atan2(isect.x)) {
            return true;
        }
    }
    false
}

/// Returns the directed Hausdorff distance from one longitudinal edge spanning
/// latitude range `a` to another spanning latitude range `b`, with their
/// longitudinal difference given by `lng_diff`.
fn get_directed_hausdorff_distance_lng(lng_diff: f64, a: r1::Interval, b: r1::Interval) -> Angle {
    debug_assert!(lng_diff >= 0.0);
    debug_assert!(lng_diff <= PI);

    if lng_diff == 0.0 {
        return Angle::from_radians(a.directed_hausdorff_distance(b));
    }

    // Assumed longitude of b.
    let b_lng = lng_diff;
    let b_lo = LatLng::from_radians(b.lo, b_lng).to_point();
    let b_hi = LatLng::from_radians(b.hi, b_lng).to_point();

    // Cases A1 and B1: endpoints of a.
    let a_lo = LatLng::from_radians(a.lo, 0.0).to_point();
    let a_hi = LatLng::from_radians(a.hi, 0.0).to_point();
    let mut max_distance = edge_distances::distance_from_segment(a_lo, b_lo, b_hi);
    max_distance = max_angle(
        max_distance,
        edge_distances::distance_from_segment(a_hi, b_lo, b_hi),
    );

    if lng_diff <= FRAC_PI_2 {
        // Case A2: intersection of a with the equator.
        if a.contains(0.0) && b.contains(0.0) {
            max_distance = max_angle(max_distance, Angle::from_radians(lng_diff));
        }
    } else {
        // Case B2: intersection of a with E3 (Voronoi edge).
        let p = get_bisector_intersection(b, b_lng);
        let p_lat = LatLng::latitude(p).radians();
        if a.contains(p_lat) {
            max_distance = max_angle(max_distance, p.distance(b_lo));
        }

        // Case B3: interior max distances.
        if p_lat > a.lo {
            let d = get_interior_max_distance(r1::Interval::new(a.lo, p_lat.min(a.hi)), b_lo);
            if d.radians() >= 0.0 {
                max_distance = max_angle(max_distance, d);
            }
        }
        if p_lat < a.hi {
            let d = get_interior_max_distance(r1::Interval::new(p_lat.max(a.lo), a.hi), b_hi);
            if d.radians() >= 0.0 {
                max_distance = max_angle(max_distance, d);
            }
        }
    }

    max_distance
}

/// Returns the intersection of longitude 0 with the bisector of an edge
/// on longitude `lng` and spanning latitude range `lat`.
fn get_bisector_intersection(lat: r1::Interval, lng: f64) -> Point {
    let lng = lng.abs();
    let lat_center = lat.center();
    let ortho_bisector = if lat_center >= 0.0 {
        LatLng::from_radians(lat_center - FRAC_PI_2, lng)
    } else {
        LatLng::from_radians(-lat_center - FRAC_PI_2, lng - PI)
    };
    let ortho_lng = Point(crate::r3::Vector {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    edge_crossings::robust_cross_prod(ortho_lng, ortho_bisector.to_point())
}

/// Returns the max distance from a point `b` to the segment spanning latitude
/// range `a_lat` on longitude 0, if the max occurs in the interior of `a_lat`.
/// Otherwise returns a negative angle.
fn get_interior_max_distance(a_lat: r1::Interval, b: Point) -> Angle {
    // Longitude 0 is in the y=0 plane. b.x() >= 0 implies the maximum
    // does not occur in the interior of a_lat.
    if a_lat.is_empty() || b.x() >= 0.0 {
        return Angle::from_radians(-1.0);
    }

    // Project b to the y=0 plane. The antipodal of the normalized projection
    // is the point at which the maximum distance from b occurs.
    let intersection_point = Point(
        crate::r3::Vector {
            x: -b.x(),
            y: 0.0,
            z: -b.z(),
        }
        .normalize(),
    );
    if a_lat.interior_contains(LatLng::latitude(intersection_point).radians()) {
        b.distance(intersection_point)
    } else {
        Angle::from_radians(-1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_4;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn rect_is_send_sync() {
        is_send_sync::<Rect>();
    }

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn test_empty_full_valid() {
        let empty = Rect::empty();
        let full = Rect::full();

        assert!(empty.is_valid());
        assert!(empty.is_empty());
        assert!(!empty.is_full());

        assert!(full.is_valid());
        assert!(!full.is_empty());
        assert!(full.is_full());
    }

    #[test]
    fn test_from_lat_lng() {
        let ll = LatLng::from_degrees(23.5, 48.0);
        let r = Rect::from_lat_lng(ll);
        assert!(r.is_point());
        assert!(r.contains_lat_lng(ll));
    }

    #[test]
    fn test_from_center_size() {
        let r = Rect::from_center_size(
            LatLng::from_degrees(80.0, 170.0),
            LatLng::from_degrees(40.0, 60.0),
        );
        assert!(r.approx_eq(Rect::new(
            r1::Interval::new(60.0_f64.to_radians(), 90.0_f64.to_radians()),
            s1::Interval::new(140.0_f64.to_radians(), (-160.0_f64).to_radians()),
        )));
    }

    #[test]
    fn test_accessors() {
        let r = Rect::new(
            r1::Interval::new(0.0, FRAC_PI_2),
            s1::Interval::new(-FRAC_PI_2, FRAC_PI_2),
        );
        assert!(float64_near(r.lat.lo, 0.0, 1e-15));
        assert!(float64_near(r.lat.hi, FRAC_PI_2, 1e-15));
        assert!(float64_near(r.lng.lo, -FRAC_PI_2, 1e-15));
        assert!(float64_near(r.lng.hi, FRAC_PI_2, 1e-15));
    }

    #[test]
    fn test_area() {
        assert_eq!(Rect::empty().area(), 0.0);
        assert!(float64_near(Rect::full().area(), 4.0 * PI, 1e-14,));
    }

    #[test]
    fn test_contains_lat_lng() {
        let r = Rect::new(
            r1::Interval::new(0.0, FRAC_PI_4),
            s1::Interval::new(-PI, 0.0),
        );
        assert!(r.contains_lat_lng(LatLng::from_radians(FRAC_PI_4 / 2.0, -FRAC_PI_2)));
        assert!(!r.contains_lat_lng(LatLng::from_radians(FRAC_PI_2, -FRAC_PI_2)));
    }

    #[test]
    fn test_contains_rect() {
        let full = Rect::full();
        let empty = Rect::empty();
        let r = Rect::new(
            r1::Interval::new(0.0, FRAC_PI_4),
            s1::Interval::new(-PI, 0.0),
        );

        assert!(full.contains(r));
        assert!(!empty.contains(r));
        assert!(r.contains(empty));
        assert!(r.contains(r));
    }

    #[test]
    fn test_intersects() {
        let r1 = Rect::new(
            r1::Interval::new(0.0, FRAC_PI_4),
            s1::Interval::new(-PI, 0.0),
        );
        let r2 = Rect::new(
            r1::Interval::new(FRAC_PI_4 / 2.0, FRAC_PI_2),
            s1::Interval::new(-FRAC_PI_2, FRAC_PI_2),
        );
        assert!(r1.intersects(r2));
        assert!(!r1.intersects(Rect::empty()));
    }

    #[test]
    fn test_union() {
        let r1 = Rect::from_lat_lng(LatLng::from_degrees(10.0, 20.0));
        let r2 = Rect::from_lat_lng(LatLng::from_degrees(30.0, 40.0));
        let u = r1.union(r2);
        assert!(u.contains_lat_lng(LatLng::from_degrees(10.0, 20.0)));
        assert!(u.contains_lat_lng(LatLng::from_degrees(30.0, 40.0)));
    }

    #[test]
    fn test_polar_closure() {
        // Rectangle that doesn't include either pole.
        let r = Rect::new(
            r1::Interval::new(0.0, FRAC_PI_4),
            s1::Interval::new(0.0, FRAC_PI_2),
        );
        assert_eq!(r.polar_closure().lng.lo, r.lng.lo);

        // Rectangle that includes the north pole.
        let r2 = Rect::new(
            r1::Interval::new(0.0, FRAC_PI_2),
            s1::Interval::new(0.0, FRAC_PI_2),
        );
        assert!(r2.polar_closure().lng.is_full());
    }

    #[test]
    fn test_expanded() {
        let r = Rect::from_lat_lng(LatLng::from_degrees(0.0, 0.0));
        let e = r.expanded(LatLng::from_degrees(10.0, 20.0));
        assert!(e.contains_lat_lng(LatLng::from_degrees(5.0, 10.0)));
    }

    #[test]
    fn test_cap_bound() {
        // Full rect → full cap.
        assert!(Rect::full().cap_bound().is_full());
        // Empty rect → empty cap.
        assert!(Rect::empty().cap_bound().is_empty());
    }

    #[test]
    fn test_centroid() {
        let empty_centroid = Rect::empty().centroid();
        assert!(empty_centroid.approx_eq(Point::default()));

        // Full rect centroid should be near origin.
        assert!(Rect::full().centroid().vector().norm() < 1e-15);
    }

    #[test]
    fn test_display() {
        let r = Rect::from_lat_lng(LatLng::from_degrees(10.0, 20.0));
        let s = format!("{r}");
        assert!(s.contains("Lo"));
        assert!(s.contains("Hi"));
    }

    #[test]
    fn test_approx_equal() {
        assert!(Rect::empty().approx_eq(Rect::empty()));
        assert!(Rect::full().approx_eq(Rect::full()));
        assert!(!Rect::empty().approx_eq(Rect::full()));
    }

    /// Helper to construct a Rect from degrees (normalizes endpoints).
    /// Uses `round_ties_even` for IEEE-compatible longitude normalization,
    /// matching C++ `S2LatLng::Normalized` behavior.
    fn rect_from_degrees(lat_lo: f64, lng_lo: f64, lat_hi: f64, lng_hi: f64) -> Rect {
        fn normalize_latlng(lat_deg: f64, lng_deg: f64) -> LatLng {
            let lat = lat_deg.to_radians().clamp(-FRAC_PI_2, FRAC_PI_2);
            let lng_rad = lng_deg.to_radians();
            let lng = lng_rad - (lng_rad / (2.0 * PI)).round_ties_even() * (2.0 * PI);
            LatLng::from_radians(lat, lng)
        }
        let lo = normalize_latlng(lat_lo, lng_lo);
        let hi = normalize_latlng(lat_hi, lng_hi);
        Rect::new(
            r1::Interval::new(lo.lat.radians(), hi.lat.radians()),
            s1::Interval::new(lo.lng.radians(), hi.lng.radians()),
        )
    }

    #[test]
    fn test_vertex_basic() {
        // Test vertex ordering for a rectangle in the first quadrant.
        let r = Rect::new(
            r1::Interval::new(0.0, FRAC_PI_2),
            s1::Interval::new(-PI, 0.0),
        );
        let eps = 1e-15;
        // Vertex 0: lower-left (lat_lo, lng_lo)
        let v0 = r.vertex(RectVertex::LowerLeft);
        assert!(float64_near(v0.lat.radians(), 0.0, eps));
        // -PI and PI are equivalent longitudes
        assert!(float64_near(v0.lng.radians().abs(), PI, eps));
        // Vertex 1: lower-right (lat_lo, lng_hi)
        assert_eq!(
            r.vertex(RectVertex::LowerRight),
            LatLng::from_radians(0.0, 0.0)
        );
        // Vertex 2: upper-right (lat_hi, lng_hi)
        assert_eq!(
            r.vertex(RectVertex::UpperRight),
            LatLng::from_radians(FRAC_PI_2, 0.0)
        );
        // Vertex 3: upper-left (lat_hi, lng_lo)
        let v3 = r.vertex(RectVertex::UpperLeft);
        assert!(float64_near(v3.lat.radians(), FRAC_PI_2, eps));
        assert!(float64_near(v3.lng.radians().abs(), PI, eps));
    }

    #[test]
    fn test_vertex_ccw_order() {
        // Verify that GetVertex returns vertices in CCW order using the
        // orientation test.
        use crate::s2::predicates;

        for i in 0..4i32 {
            let lat = FRAC_PI_4 * f64::from(i - 2);
            let lng = FRAC_PI_2 * f64::from(i - 2) + 0.2;
            let r = Rect::new(
                r1::Interval::new(lat, lat + FRAC_PI_4),
                s1::Interval::new(
                    ((lng + PI) % (2.0 * PI)) - PI,
                    ((lng + FRAC_PI_2 + PI) % (2.0 * PI)) - PI,
                ),
            );
            for v in RectVertex::iter() {
                let prev = r.vertex(v.prev()).to_point();
                let curr = r.vertex(v).to_point();
                let next = r.vertex(v.next()).to_point();
                // Vertices should be in CCW order (positive orientation).
                let dir = predicates::robust_sign(prev, curr, next);
                assert_eq!(dir as i32, 1, "Vertices not in CCW order at i={i}, v={v:?}");
            }
        }
    }

    #[test]
    fn test_add_point_incremental() {
        // Start empty, add points one by one.
        let mut r = Rect::empty();
        r = r.add_point(LatLng::from_degrees(0.0, 0.0));
        assert!(r.is_point());

        r = r.add_point(LatLng::from_radians(0.0, -FRAC_PI_2));
        assert!(!r.is_point());

        r = r.add_point(LatLng::from_radians(FRAC_PI_4, -PI));
        r = r.add_point(LatLng::from_degrees(90.0, 0.0));
        assert!(r.approx_eq(rect_from_degrees(0.0, -180.0, 90.0, 0.0)));
    }

    #[test]
    fn test_cap_bound_at_center() {
        // Bounding cap at center is smaller for rectangle centered on equator.
        let r = rect_from_degrees(-45.0, -45.0, 45.0, 45.0);
        let cap = r.cap_bound();
        let expected = Cap::from_center_height(Point::from_coords(1.0, 0.0, 0.0), 0.5);
        assert!(
            cap.approx_eq(expected),
            "cap_bound for 45deg square: got center={:?} height={}, expected center={:?} height={}",
            cap.center(),
            cap.height(),
            expected.center(),
            expected.height(),
        );
    }

    #[test]
    fn test_cap_bound_near_pole() {
        // Bounding cap at north pole for a rect near the pole.
        let r = rect_from_degrees(88.0, -80.0, 89.0, 80.0);
        let cap = r.cap_bound();
        let expected =
            Cap::from_center_angle(Point::from_coords(0.0, 0.0, 1.0), Angle::from_degrees(2.0));
        assert!(
            cap.approx_eq(expected),
            "cap_bound near pole: got height={}, expected height={}",
            cap.height(),
            expected.height(),
        );
    }

    #[test]
    fn test_cap_bound_wide_longitude() {
        // Longitude span > 180 degrees: pole cap should be used.
        let r = rect_from_degrees(-30.0, -150.0, -10.0, 50.0);
        let cap = r.cap_bound();
        let expected = Cap::from_center_angle(
            Point::from_coords(0.0, 0.0, -1.0),
            Angle::from_degrees(80.0),
        );
        assert!(
            cap.approx_eq(expected),
            "cap_bound wide lng: got height={}, expected height={}",
            cap.height(),
            expected.height(),
        );
    }

    #[test]
    fn test_cap_bound_hemisphere() {
        // Hemisphere: ensure conservatively bounded (allow small FP tolerance).
        let r = rect_from_degrees(-10.0, -100.0, 0.0, 100.0);
        let cap = r.cap_bound();
        // The cap radius may fall short of RIGHT by a tiny floating-point amount.
        assert!(
            cap.chord_radius().length2() >= ChordAngle::RIGHT.length2() - 1e-14,
            "Hemisphere cap should have radius ~>= RIGHT, got {:?}",
            cap.chord_radius(),
        );
    }

    #[test]
    fn test_intersection() {
        // Two rectangles that overlap.
        let r1 = rect_from_degrees(0.0, -90.0, 45.0, 0.0);
        let r2 = rect_from_degrees(20.0, -45.0, 60.0, 45.0);
        let isect = r1.intersection(r2);
        assert!(isect.approx_eq(rect_from_degrees(20.0, -45.0, 45.0, 0.0)));

        // Non-overlapping rectangles.
        let r3 = rect_from_degrees(50.0, 10.0, 60.0, 20.0);
        assert!(r1.intersection(r3).is_empty());
    }

    #[test]
    fn test_contains_point() {
        // Test containment via Point (not LatLng).
        let r = rect_from_degrees(-30.0, -60.0, 30.0, 60.0);
        let inside = LatLng::from_degrees(0.0, 0.0).to_point();
        let outside = LatLng::from_degrees(50.0, 0.0).to_point();
        assert!(r.contains_point(inside));
        assert!(!r.contains_point(outside));
    }

    #[test]
    fn test_get_center_size() {
        // Verify center and size of a known rectangle.
        let r = rect_from_degrees(-10.0, -20.0, 30.0, 40.0);
        let center = r.center();
        assert!(float64_near(center.lat.degrees(), 10.0, 1e-10));
        assert!(float64_near(center.lng.degrees(), 10.0, 1e-10));

        let size = r.size();
        assert!(float64_near(size.lat.degrees(), 40.0, 1e-10));
        assert!(float64_near(size.lng.degrees(), 60.0, 1e-10));
    }

    #[test]
    fn test_area_specific_values() {
        // Area of a 1-degree square near the equator.
        let r = rect_from_degrees(0.0, 0.0, 1.0, 1.0);
        let expected_area = 1.0_f64.to_radians() * (1.0_f64.to_radians().sin() - 0.0_f64.sin());
        assert!(
            float64_near(r.area(), expected_area, 1e-10),
            "1° square area: got {}, expected {}",
            r.area(),
            expected_area,
        );

        // Area of the northern hemisphere.
        let north = rect_from_degrees(0.0, -180.0, 90.0, 180.0);
        assert!(
            float64_near(north.area(), 2.0 * PI, 1e-10),
            "Northern hemisphere area: got {}, expected {}",
            north.area(),
            2.0 * PI,
        );
    }

    #[test]
    fn test_contains_antimeridian() {
        // Rectangle crossing the antimeridian.
        let r = rect_from_degrees(20.0, 170.0, 40.0, -170.0);
        assert!(r.contains_lat_lng(LatLng::from_degrees(30.0, 180.0)));
        assert!(r.contains_lat_lng(LatLng::from_degrees(30.0, -180.0)));
        assert!(r.contains_lat_lng(LatLng::from_degrees(30.0, 175.0)));
        assert!(r.contains_lat_lng(LatLng::from_degrees(30.0, -175.0)));
        assert!(!r.contains_lat_lng(LatLng::from_degrees(30.0, 0.0)));
    }

    /// Helper for testing Contains, Intersects, Union, Intersection.
    fn test_interval_ops(
        x: Rect,
        y: Rect,
        expected_contains: bool,
        expected_intersects: bool,
        expected_union: Rect,
        expected_intersection: Rect,
    ) {
        assert_eq!(x.contains(y), expected_contains, "Contains({x}, {y})");
        assert_eq!(x.intersects(y), expected_intersects, "Intersects({x}, {y})");
        // Contains ↔ union equals self.
        assert_eq!(
            x.contains(y),
            x.union(y).approx_eq(x),
            "Contains ↔ Union==self for ({x}, {y})"
        );
        // Intersects ↔ intersection non-empty.
        assert_eq!(
            x.intersects(y),
            !x.intersection(y).is_empty(),
            "Intersects ↔ Intersection non-empty for ({x}, {y})"
        );
        assert!(
            x.union(y).approx_eq(expected_union),
            "Union({x}, {y}): got {}, expected {expected_union}",
            x.union(y),
        );
        assert!(
            x.intersection(y).approx_eq(expected_intersection),
            "Intersection({x}, {y}): got {}, expected {expected_intersection}",
            x.intersection(y),
        );
    }

    #[test]
    fn test_interval_ops_basic() {
        // C++ S2LatLngRect.IntervalOps
        // r1 covers one-quarter of the sphere.
        let r1 = rect_from_degrees(0.0, -180.0, 90.0, 0.0);

        // Single point at r1's center.
        let r1_mid = rect_from_degrees(45.0, -90.0, 45.0, -90.0);
        test_interval_ops(r1, r1_mid, true, true, r1, r1_mid);

        // Point at r1's corner (-180 longitude).
        let req_m180 = rect_from_degrees(0.0, -180.0, 0.0, -180.0);
        test_interval_ops(r1, req_m180, true, true, r1, req_m180);

        // Point at north pole (r1's boundary).
        let rnorth_pole = rect_from_degrees(90.0, 0.0, 90.0, 0.0);
        test_interval_ops(r1, rnorth_pole, true, true, r1, rnorth_pole);

        // Rectangle that overlaps r1 in lat but not lng (straddles equator, positive lng).
        test_interval_ops(
            r1,
            rect_from_degrees(-10.0, -1.0, 1.0, 20.0),
            false,
            true,
            rect_from_degrees(-10.0, 180.0, 90.0, 20.0),
            rect_from_degrees(0.0, -1.0, 1.0, 0.0),
        );

        // Disjoint rectangles.
        test_interval_ops(
            rect_from_degrees(-15.0, -160.0, -15.0, -150.0),
            rect_from_degrees(20.0, 145.0, 25.0, 155.0),
            false,
            false,
            rect_from_degrees(-15.0, 145.0, 25.0, -150.0),
            Rect::empty(),
        );

        // Two rectangles that overlap in lat and partially in lng.
        test_interval_ops(
            rect_from_degrees(70.0, -10.0, 90.0, -140.0),
            rect_from_degrees(60.0, 175.0, 80.0, 5.0),
            false,
            true,
            rect_from_degrees(60.0, -180.0, 90.0, 180.0),
            rect_from_degrees(70.0, 175.0, 80.0, 5.0),
        );

        // Overlapping lat, non-overlapping lng → empty intersection.
        test_interval_ops(
            rect_from_degrees(12.0, 30.0, 60.0, 60.0),
            rect_from_degrees(0.0, 0.0, 30.0, 18.0),
            false,
            false,
            rect_from_degrees(0.0, 0.0, 60.0, 60.0),
            Rect::empty(),
        );

        // Overlapping lng, non-overlapping lat → empty intersection.
        test_interval_ops(
            rect_from_degrees(0.0, 0.0, 18.0, 42.0),
            rect_from_degrees(30.0, 12.0, 42.0, 60.0),
            false,
            false,
            rect_from_degrees(0.0, 0.0, 42.0, 60.0),
            Rect::empty(),
        );
    }

    #[test]
    fn test_approx_equal_empty_rects() {
        // C++ S2LatLngRect.ApproxEquals
        // Empty rect is approximately equal to any single-point rect.
        assert!(Rect::empty().approx_eq(rect_from_degrees(1.0, 5.0, 1.0, 5.0)));
        assert!(rect_from_degrees(1.0, 5.0, 1.0, 5.0).approx_eq(Rect::empty()));
        // Two different single-point rects are not approximately equal.
        assert!(
            !rect_from_degrees(1.0, 5.0, 1.0, 5.0).approx_eq(rect_from_degrees(2.0, 7.0, 2.0, 7.0))
        );
    }

    #[test]
    fn test_union_contains_both() {
        // Verify that union of two rects contains both.
        let pairs = [
            (
                rect_from_degrees(0.0, 0.0, 10.0, 10.0),
                rect_from_degrees(5.0, 5.0, 15.0, 15.0),
            ),
            (
                rect_from_degrees(-90.0, -180.0, 0.0, 0.0),
                rect_from_degrees(0.0, 0.0, 90.0, 180.0),
            ),
            (
                rect_from_degrees(20.0, 170.0, 30.0, -170.0),
                rect_from_degrees(25.0, -175.0, 35.0, 175.0),
            ),
        ];
        for (r1, r2) in &pairs {
            let u = r1.union(*r2);
            assert!(u.contains(*r1), "Union should contain r1: {r1}");
            assert!(u.contains(*r2), "Union should contain r2: {r2}");
        }
    }

    #[test]
    fn test_expanded_basic() {
        // Test expanded (margin in lat/lng).
        let r = rect_from_degrees(10.0, 20.0, 30.0, 40.0);
        let e = r.expanded(LatLng::from_degrees(5.0, 10.0));
        assert!(e.approx_eq(rect_from_degrees(5.0, 10.0, 35.0, 50.0)));

        // Expanding an empty rect stays empty.
        assert!(
            Rect::empty()
                .expanded(LatLng::from_degrees(1.0, 1.0))
                .is_empty()
        );
    }

    #[test]
    fn test_polar_closure_at_poles() {
        // If a rect includes the north pole, its longitude should become full.
        let r = rect_from_degrees(85.0, 10.0, 90.0, 20.0);
        let pc = r.polar_closure();
        assert!(
            pc.lng.is_full(),
            "North pole rect should have full longitude"
        );
        assert!(float64_near(pc.lat.lo, 85.0_f64.to_radians(), 1e-15));

        // South pole.
        let r2 = rect_from_degrees(-90.0, 10.0, -80.0, 20.0);
        let pc2 = r2.polar_closure();
        assert!(
            pc2.lng.is_full(),
            "South pole rect should have full longitude"
        );
    }

    #[test]
    fn test_centroid_hemisphere() {
        // The centroid of the northern hemisphere should be near (0,0,something>0).
        let r = rect_from_degrees(0.0, -180.0, 90.0, 180.0);
        let c = r.centroid();
        assert!(float64_near(c.0.x, 0.0, 1e-10));
        assert!(float64_near(c.0.y, 0.0, 1e-10));
        assert!(
            c.0.z > 0.0,
            "Centroid of N hemisphere should have positive z"
        );
    }

    #[test]
    fn test_area_empty_and_full() {
        assert_eq!(Rect::empty().area(), 0.0);
        assert!(
            (Rect::full().area() - 4.0 * PI).abs() < 1e-10,
            "Full rect area should be 4π, got {}",
            Rect::full().area(),
        );
    }

    #[test]
    fn test_area_quarter_sphere() {
        // A 90×90 degree rect (0-90 lat, 0-90 lng) should have area π/2.
        let r = rect_from_degrees(0.0, 0.0, 90.0, 90.0);
        assert!(
            (r.area() - FRAC_PI_2).abs() < 1e-10,
            "Quarter sphere area should be π/2, got {}",
            r.area(),
        );
    }

    #[test]
    fn test_polar_closure_non_polar() {
        // A rect that doesn't touch either pole is unchanged by polar_closure.
        let r = rect_from_degrees(-89.0, 0.0, 89.0, 1.0);
        assert_eq!(r, r.polar_closure());
    }

    #[test]
    fn test_polar_closure_south_pole() {
        // A rect containing the south pole gets full longitude.
        let r = rect_from_degrees(-90.0, -30.0, -45.0, 100.0);
        let closed = r.polar_closure();
        assert_eq!(closed.lng.lo, -PI);
        assert_eq!(closed.lng.hi, PI);
    }

    #[test]
    fn test_polar_closure_both_poles() {
        // A rect containing both poles becomes full.
        let r = rect_from_degrees(-90.0, -145.0, 90.0, -144.0);
        assert_eq!(r.polar_closure(), Rect::full());
    }

    // ===== BoundaryIntersects tests (from C++ s2latlng_rect_test.cc) =====

    fn make_point(lat_deg: f64, lng_deg: f64) -> Point {
        LatLng::from_degrees(lat_deg, lng_deg).to_point()
    }

    fn point_rect_from_degrees(lat_deg: f64, lng_deg: f64) -> Rect {
        let ll = LatLng::from_degrees(lat_deg, lng_deg).normalized();
        Rect::from_lat_lng(ll)
    }

    #[test]
    fn test_boundary_intersects_spherical_lune() {
        // C++ BoundaryIntersects.SphericalLune
        // This rectangle only has two non-degenerate sides.
        let rect = rect_from_degrees(-90.0, 100.0, 90.0, 120.0);
        assert!(!rect.boundary_intersects(make_point(60.0, 60.0), make_point(90.0, 60.0)));
        assert!(!rect.boundary_intersects(make_point(-60.0, 110.0), make_point(60.0, 110.0)));
        assert!(rect.boundary_intersects(make_point(-60.0, 95.0), make_point(60.0, 110.0)));
        assert!(rect.boundary_intersects(make_point(60.0, 115.0), make_point(80.0, 125.0)));
    }

    #[test]
    fn test_boundary_intersects_north_hemisphere() {
        // C++ BoundaryIntersects.NorthHemisphere
        let rect = rect_from_degrees(0.0, -180.0, 90.0, 180.0);
        assert!(!rect.boundary_intersects(make_point(60.0, -180.0), make_point(90.0, -180.0)));
        assert!(!rect.boundary_intersects(make_point(60.0, -170.0), make_point(60.0, 170.0)));
        assert!(rect.boundary_intersects(make_point(-10.0, -180.0), make_point(10.0, -180.0)));
    }

    #[test]
    fn test_boundary_intersects_south_hemisphere() {
        // C++ BoundaryIntersects.SouthHemisphere
        let rect = rect_from_degrees(-90.0, -180.0, 0.0, 180.0);
        assert!(!rect.boundary_intersects(make_point(-90.0, -180.0), make_point(-60.0, -180.0)));
        assert!(!rect.boundary_intersects(make_point(-60.0, -170.0), make_point(-60.0, 170.0)));
        assert!(rect.boundary_intersects(make_point(-10.0, -180.0), make_point(10.0, -180.0)));
    }

    #[test]
    fn test_boundary_intersects_rect_crossing_antimeridian() {
        // C++ BoundaryIntersects.RectCrossingAntiMeridian
        let rect = rect_from_degrees(20.0, 170.0, 40.0, -170.0);
        assert!(rect.contains_point(make_point(30.0, 180.0)));

        // Check that crossings of all four sides are detected.
        assert!(rect.boundary_intersects(make_point(25.0, 160.0), make_point(25.0, 180.0)));
        assert!(rect.boundary_intersects(make_point(25.0, -160.0), make_point(25.0, -180.0)));
        assert!(rect.boundary_intersects(make_point(15.0, 175.0), make_point(30.0, 175.0)));
        assert!(rect.boundary_intersects(make_point(45.0, 175.0), make_point(30.0, 175.0)));

        // Edges on opposite side of the sphere at the same latitude should not intersect.
        assert!(!rect.boundary_intersects(make_point(25.0, -20.0), make_point(25.0, 0.0)));
        assert!(!rect.boundary_intersects(make_point(25.0, 20.0), make_point(25.0, 0.0)));
        assert!(!rect.boundary_intersects(make_point(15.0, -5.0), make_point(30.0, -5.0)));
        assert!(!rect.boundary_intersects(make_point(45.0, -5.0), make_point(30.0, -5.0)));
    }

    // ===== GetDirectedHausdorffDistance tests (from C++ s2latlng_rect_test.cc) =====

    /// Verification helper that samples rect `a` on a grid and checks the
    /// directed Hausdorff distance against brute-force.
    fn verify_get_directed_hausdorff_distance(a: Rect, b: Rect) {
        let hausdorff_distance = a.get_directed_hausdorff_distance(b);

        let resolution = 0.1_f64;
        let mut max_distance = Angle::from_radians(0.0);

        let sample_size_on_lat = (a.lat.length() / resolution) as i32 + 1;
        let sample_size_on_lng = (a.lng.length() / resolution) as i32 + 1;
        let delta_on_lat = a.lat.length() / f64::from(sample_size_on_lat);
        let delta_on_lng = a.lng.length() / f64::from(sample_size_on_lng);

        let mut lng = a.lng.lo;
        for _i in 0..=sample_size_on_lng {
            let mut lat = a.lat.lo;
            for _j in 0..=sample_size_on_lat {
                let latlng = LatLng::from_radians(lat, lng).normalized();
                let distance_to_b = b.get_distance_to_latlng(latlng);
                if distance_to_b.radians() >= max_distance.radians() {
                    max_distance = distance_to_b;
                }
                lat += delta_on_lat;
            }
            lng += delta_on_lng;
        }

        assert!(
            max_distance.radians() <= hausdorff_distance.radians() + 1e-10,
            "max_distance ({}) > hausdorff_distance ({}) + 1e-10 for {a} : {b}",
            max_distance.radians(),
            hausdorff_distance.radians(),
        );
        assert!(
            max_distance.radians() >= hausdorff_distance.radians() - resolution,
            "max_distance ({}) < hausdorff_distance ({}) - resolution for {a} : {b}",
            max_distance.radians(),
            hausdorff_distance.radians(),
        );
    }

    #[test]
    fn test_get_directed_hausdorff_distance_contained() {
        // C++ GetDirectedHausdorffDistanceContained
        // Caller rect is contained in callee rect. Should return 0.
        let a = rect_from_degrees(-10.0, 20.0, -5.0, 90.0);
        assert_eq!(
            0.0,
            a.get_directed_hausdorff_distance(rect_from_degrees(-10.0, 20.0, -5.0, 90.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_directed_hausdorff_distance(rect_from_degrees(-10.0, 19.0, -5.0, 91.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_directed_hausdorff_distance(rect_from_degrees(-11.0, 20.0, -4.0, 90.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_directed_hausdorff_distance(rect_from_degrees(-11.0, 19.0, -4.0, 91.0))
                .radians()
        );
    }

    #[test]
    fn test_get_directed_hausdorff_distance_point_to_rect() {
        // C++ GetDirectHausdorffDistancePointToRect
        // The Hausdorff distance from a point to a rect should be the same
        // as its distance to the rect.
        let a1 = point_rect_from_degrees(5.0, 8.0);
        let a2 = point_rect_from_degrees(90.0, 10.0); // north pole

        let b = rect_from_degrees(-85.0, -50.0, -80.0, 10.0);
        assert!(
            (a1.get_directed_hausdorff_distance(b).radians() - a1.get_distance(b).radians()).abs()
                < 1e-15
        );
        assert!(
            (a2.get_directed_hausdorff_distance(b).radians() - a2.get_distance(b).radians()).abs()
                < 1e-15
        );

        let b = rect_from_degrees(4.0, -10.0, 80.0, 10.0);
        assert!(
            (a1.get_directed_hausdorff_distance(b).radians() - a1.get_distance(b).radians()).abs()
                < 1e-15
        );
        assert!(
            (a2.get_directed_hausdorff_distance(b).radians() - a2.get_distance(b).radians()).abs()
                < 1e-15
        );

        let b = rect_from_degrees(70.0, 170.0, 80.0, -170.0);
        assert!(
            (a1.get_directed_hausdorff_distance(b).radians() - a1.get_distance(b).radians()).abs()
                < 1e-15
        );
        assert!(
            (a2.get_directed_hausdorff_distance(b).radians() - a2.get_distance(b).radians()).abs()
                < 1e-15
        );
    }

    #[test]
    fn test_get_directed_hausdorff_distance_rect_to_point() {
        // C++ GetDirectedHausdorffDistanceRectToPoint
        let a = rect_from_degrees(1.0, -8.0, 10.0, 20.0);
        verify_get_directed_hausdorff_distance(a, point_rect_from_degrees(5.0, 8.0));
        verify_get_directed_hausdorff_distance(a, point_rect_from_degrees(-6.0, -100.0));
        // south pole
        verify_get_directed_hausdorff_distance(a, point_rect_from_degrees(-90.0, -20.0));
        // north pole
        verify_get_directed_hausdorff_distance(a, point_rect_from_degrees(90.0, 0.0));
    }

    #[test]
    fn test_get_directed_hausdorff_distance_rect_to_rect_near_pole() {
        // C++ GetDirectedHausdorffDistanceRectToRectNearPole
        // Tests near south pole.
        let a = rect_from_degrees(-87.0, 0.0, -85.0, 3.0);
        verify_get_directed_hausdorff_distance(a, rect_from_degrees(-89.0, 1.0, -88.0, 2.0));
        verify_get_directed_hausdorff_distance(a, rect_from_degrees(-84.0, 1.0, -83.0, 2.0));
        verify_get_directed_hausdorff_distance(a, rect_from_degrees(-88.0, 90.0, -86.0, 91.0));
        verify_get_directed_hausdorff_distance(a, rect_from_degrees(-84.0, -91.0, -83.0, -90.0));
        verify_get_directed_hausdorff_distance(a, rect_from_degrees(-90.0, 181.0, -89.0, 182.0));
        verify_get_directed_hausdorff_distance(a, rect_from_degrees(-84.0, 181.0, -83.0, 182.0));
    }

    #[test]
    fn test_get_directed_hausdorff_distance_rect_to_rect_degenerate() {
        // C++ GetDirectedHausdorffDistanceRectToRectDegenerateCases
        // Rectangles that contain poles.
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(0.0, 10.0, 90.0, 20.0),
            rect_from_degrees(-4.0, -10.0, 4.0, 0.0),
        );
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(-4.0, -10.0, 4.0, 0.0),
            rect_from_degrees(0.0, 10.0, 90.0, 20.0),
        );

        // Two rectangles share same or complement longitudinal intervals.
        let a = rect_from_degrees(-50.0, -10.0, 50.0, 10.0);
        let b = rect_from_degrees(30.0, -10.0, 60.0, 10.0);
        verify_get_directed_hausdorff_distance(a, b);
        let c = Rect::new(a.lat, a.lng.complement());
        verify_get_directed_hausdorff_distance(c, b);

        // Rectangle a touches b_opposite_lng.
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(10.0, 170.0, 30.0, 180.0),
            rect_from_degrees(-50.0, -10.0, 50.0, 10.0),
        );
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(10.0, -180.0, 30.0, -170.0),
            rect_from_degrees(-50.0, -10.0, 50.0, 10.0),
        );

        // Rectangle b's Voronoi diagram is degenerate (lng spans 180°).
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(-30.0, 170.0, 30.0, 180.0),
            rect_from_degrees(-10.0, -90.0, 10.0, 90.0),
        );
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(-30.0, -180.0, 30.0, -170.0),
            rect_from_degrees(-10.0, -90.0, 10.0, 90.0),
        );

        // Rectangle a touches a Voronoi vertex of rectangle b.
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(-20.0, 105.0, 20.0, 110.0),
            rect_from_degrees(-30.0, 5.0, 30.0, 15.0),
        );
        verify_get_directed_hausdorff_distance(
            rect_from_degrees(-20.0, 95.0, 20.0, 105.0),
            rect_from_degrees(-30.0, 5.0, 30.0, 15.0),
        );
    }

    #[test]
    fn test_get_directed_hausdorff_distance_various_pairs() {
        // Deterministic version of C++ GetDirectedHausdorffDistanceRandomPairs.
        // Tests diverse configurations including complementary longitude intervals.
        let test_cases: [(Rect, Rect); 8] = [
            // Basic non-overlapping rects.
            (
                rect_from_degrees(10.0, 20.0, 30.0, 40.0),
                rect_from_degrees(-20.0, 60.0, -10.0, 80.0),
            ),
            // Overlapping rects.
            (
                rect_from_degrees(-30.0, -60.0, 30.0, 60.0),
                rect_from_degrees(0.0, 0.0, 45.0, 90.0),
            ),
            // Rect near pole.
            (
                rect_from_degrees(70.0, -170.0, 85.0, 170.0),
                rect_from_degrees(-85.0, -10.0, -70.0, 10.0),
            ),
            // Cross antimeridian.
            (
                rect_from_degrees(-10.0, 170.0, 10.0, -170.0),
                rect_from_degrees(20.0, -30.0, 40.0, 30.0),
            ),
            // Wide longitude.
            (
                rect_from_degrees(-45.0, -90.0, 45.0, 90.0),
                rect_from_degrees(50.0, 100.0, 60.0, 120.0),
            ),
            // Narrow sliver.
            (
                rect_from_degrees(-1.0, 0.0, 1.0, 0.5),
                rect_from_degrees(-1.0, 90.0, 1.0, 90.5),
            ),
            // One containing the other.
            (
                rect_from_degrees(-10.0, -10.0, 10.0, 10.0),
                rect_from_degrees(-5.0, -5.0, 5.0, 5.0),
            ),
            // Hemispheres.
            (
                rect_from_degrees(0.0, -180.0, 90.0, 180.0),
                rect_from_degrees(-90.0, -180.0, 0.0, 180.0),
            ),
        ];

        for (a, b) in &test_cases {
            verify_get_directed_hausdorff_distance(*a, *b);

            // Also test with complementary longitude intervals, but skip
            // if the complement would produce an empty/invalid rect.
            let a2 = Rect::new(a.lat, a.lng.complement());
            let b2 = Rect::new(b.lat, b.lng.complement());

            if b2.is_valid() && !b2.is_empty() {
                verify_get_directed_hausdorff_distance(*a, b2);
            }
            if a2.is_valid() && !a2.is_empty() {
                verify_get_directed_hausdorff_distance(a2, *b);
            }
            if a2.is_valid() && !a2.is_empty() && b2.is_valid() && !b2.is_empty() {
                verify_get_directed_hausdorff_distance(a2, b2);
            }
        }
    }

    // ===== GetDistance tests (from C++ s2latlng_rect_test.cc) =====

    /// Returns the minimum distance from point `x` to a latitude line segment
    /// defined by `lat` (in radians) and a longitude interval.
    fn distance_to_lat_edge(x: LatLng, lat: f64, interval: s1::Interval) -> Angle {
        if interval.contains(x.lng.radians()) {
            return Angle::from_radians((x.lat.radians() - lat).abs());
        }
        min_angle(
            x.get_distance(LatLng::from_radians(lat, interval.lo)),
            x.get_distance(LatLng::from_radians(lat, interval.hi)),
        )
    }

    /// Brute-force rect-to-rect distance: compares every vertex of each rect
    /// against every edge of the other.
    fn brute_force_distance(a: Rect, b: Rect) -> Angle {
        if a.intersects(b) {
            return Angle::from_radians(0.0);
        }

        let pnt_a: [LatLng; 4] = RectVertex::ALL.map(|v| a.vertex(v));
        let pnt_b: [LatLng; 4] = RectVertex::ALL.map(|v| b.vertex(v));

        let lat_a = [a.lat.lo, a.lat.hi];
        let lat_b = [b.lat.lo, b.lat.hi];
        let lng_edge_a = [
            [pnt_a[0].to_point(), pnt_a[3].to_point()],
            [pnt_a[1].to_point(), pnt_a[2].to_point()],
        ];
        let lng_edge_b = [
            [pnt_b[0].to_point(), pnt_b[3].to_point()],
            [pnt_b[1].to_point(), pnt_b[2].to_point()],
        ];

        let mut min_dist = Angle::from_degrees(180.0);
        for i in 0..4 {
            let current_a = pnt_a[i];
            let current_b = pnt_b[i];

            for j in 0..2 {
                let a_to_lat = distance_to_lat_edge(current_a, lat_b[j], b.lng);
                let b_to_lat = distance_to_lat_edge(current_b, lat_a[j], a.lng);
                let a_to_lng = edge_distances::distance_from_segment(
                    current_a.to_point(),
                    lng_edge_b[j][0],
                    lng_edge_b[j][1],
                );
                let b_to_lng = edge_distances::distance_from_segment(
                    current_b.to_point(),
                    lng_edge_a[j][0],
                    lng_edge_a[j][1],
                );

                min_dist = min_angle(
                    min_dist,
                    min_angle(a_to_lat, min_angle(b_to_lat, min_angle(a_to_lng, b_to_lng))),
                );
            }
        }
        min_dist
    }

    /// Brute-force rect-to-point distance.
    fn brute_force_rect_point_distance(a: Rect, b: LatLng) -> Angle {
        if a.contains_lat_lng(b) {
            return Angle::from_radians(0.0);
        }
        let b_to_lo_lat = distance_to_lat_edge(b, a.lat.lo, a.lng);
        let b_to_hi_lat = distance_to_lat_edge(b, a.lat.hi, a.lng);
        let b_to_lo_lng = edge_distances::distance_from_segment(
            b.to_point(),
            LatLng::from_radians(a.lat.lo, a.lng.lo).to_point(),
            LatLng::from_radians(a.lat.hi, a.lng.lo).to_point(),
        );
        let b_to_hi_lng = edge_distances::distance_from_segment(
            b.to_point(),
            LatLng::from_radians(a.lat.lo, a.lng.hi).to_point(),
            LatLng::from_radians(a.lat.hi, a.lng.hi).to_point(),
        );
        min_angle(
            b_to_lo_lat,
            min_angle(b_to_hi_lat, min_angle(b_to_lo_lng, b_to_hi_lng)),
        )
    }

    /// Verifies rect-to-rect distance against brute force.
    fn verify_get_distance(a: Rect, b: Rect) {
        let d1 = brute_force_distance(a, b);
        let d2 = a.get_distance(b);
        assert!(
            (d1.radians() - d2.radians()).abs() < 1e-10,
            "GetDistance({a}, {b}): brute_force={}, get_distance={}",
            d1.radians(),
            d2.radians(),
        );
    }

    /// Verifies rect-to-point distance against brute force.
    fn verify_get_rect_point_distance(a: Rect, p: LatLng) {
        let p = p.normalized();
        let d1 = brute_force_rect_point_distance(a, p);
        let d2 = a.get_distance_to_latlng(p);
        assert!(
            (d1.radians() - d2.radians()).abs() < 1e-10,
            "GetDistance({a}, {p}): brute_force={}, get_distance={}",
            d1.radians(),
            d2.radians(),
        );
    }

    #[test]
    fn test_get_distance_overlapping() {
        // C++ GetDistanceOverlapping
        let a = rect_from_degrees(0.0, 0.0, 2.0, 2.0);
        let b = point_rect_from_degrees(0.0, 0.0);
        assert_eq!(0.0, a.get_distance(a).radians());
        assert_eq!(0.0, a.get_distance(b).radians());
        assert_eq!(0.0, b.get_distance(b).radians());
        assert_eq!(
            0.0,
            a.get_distance_to_latlng(LatLng::from_degrees(0.0, 0.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_distance(rect_from_degrees(0.0, 1.0, 2.0, 3.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_distance(rect_from_degrees(0.0, 2.0, 2.0, 4.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_distance(rect_from_degrees(1.0, 0.0, 3.0, 2.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_distance(rect_from_degrees(2.0, 0.0, 4.0, 2.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_distance(rect_from_degrees(1.0, 1.0, 3.0, 3.0))
                .radians()
        );
        assert_eq!(
            0.0,
            a.get_distance(rect_from_degrees(2.0, 2.0, 4.0, 4.0))
                .radians()
        );
    }

    #[test]
    fn test_get_distance_rect_vs_point() {
        // C++ GetDistanceRectVsPoint
        let a = rect_from_degrees(-1.0, -1.0, 2.0, 1.0);
        verify_get_distance(a, point_rect_from_degrees(-2.0, -1.0));
        verify_get_distance(a, point_rect_from_degrees(1.0, 2.0));
        verify_get_distance(point_rect_from_degrees(-2.0, -1.0), a);
        verify_get_distance(point_rect_from_degrees(1.0, 2.0), a);
        verify_get_rect_point_distance(a, LatLng::from_degrees(-2.0, -1.0));
        verify_get_rect_point_distance(a, LatLng::from_degrees(1.0, 2.0));

        // Tests near the north pole.
        let b = rect_from_degrees(86.0, 0.0, 88.0, 2.0);
        verify_get_distance(b, point_rect_from_degrees(87.0, 3.0));
        verify_get_distance(b, point_rect_from_degrees(87.0, -1.0));
        verify_get_distance(b, point_rect_from_degrees(89.0, 1.0));
        verify_get_distance(b, point_rect_from_degrees(89.0, 181.0));
        verify_get_distance(b, point_rect_from_degrees(85.0, 1.0));
        verify_get_distance(b, point_rect_from_degrees(85.0, 181.0));
        verify_get_distance(b, point_rect_from_degrees(90.0, 0.0));

        verify_get_distance(point_rect_from_degrees(87.0, 3.0), b);
        verify_get_distance(point_rect_from_degrees(87.0, -1.0), b);
        verify_get_distance(point_rect_from_degrees(89.0, 1.0), b);
        verify_get_distance(point_rect_from_degrees(89.0, 181.0), b);
        verify_get_distance(point_rect_from_degrees(85.0, 1.0), b);
        verify_get_distance(point_rect_from_degrees(85.0, 181.0), b);
        verify_get_distance(point_rect_from_degrees(90.0, 0.0), b);

        verify_get_rect_point_distance(b, LatLng::from_degrees(87.0, 3.0));
        verify_get_rect_point_distance(b, LatLng::from_degrees(87.0, -1.0));
        verify_get_rect_point_distance(b, LatLng::from_degrees(89.0, 1.0));
        verify_get_rect_point_distance(b, LatLng::from_degrees(89.0, 181.0));
        verify_get_rect_point_distance(b, LatLng::from_degrees(85.0, 1.0));
        verify_get_rect_point_distance(b, LatLng::from_degrees(85.0, 181.0));
        verify_get_rect_point_distance(b, LatLng::from_degrees(90.0, 0.0));

        // Rect that touches the north pole.
        let c = rect_from_degrees(88.0, 0.0, 90.0, 2.0);
        verify_get_distance(c, point_rect_from_degrees(89.0, 3.0));
        verify_get_distance(c, point_rect_from_degrees(89.0, 90.0));
        verify_get_distance(c, point_rect_from_degrees(89.0, 181.0));
        verify_get_distance(point_rect_from_degrees(89.0, 3.0), c);
        verify_get_distance(point_rect_from_degrees(89.0, 90.0), c);
        verify_get_distance(point_rect_from_degrees(89.0, 181.0), c);
    }

    #[test]
    fn test_get_distance_rect_vs_rect() {
        // C++ GetDistanceRectVsRect
        let a = rect_from_degrees(-1.0, -1.0, 2.0, 1.0);
        verify_get_distance(a, rect_from_degrees(0.0, 2.0, 1.0, 3.0));
        verify_get_distance(a, rect_from_degrees(-2.0, -3.0, -1.0, -2.0));

        // Tests near the south pole.
        let b = rect_from_degrees(-87.0, 0.0, -85.0, 3.0);
        verify_get_distance(b, rect_from_degrees(-89.0, 1.0, -88.0, 2.0));
        verify_get_distance(b, rect_from_degrees(-84.0, 1.0, -83.0, 2.0));
        verify_get_distance(b, rect_from_degrees(-88.0, 90.0, -86.0, 91.0));
        verify_get_distance(b, rect_from_degrees(-84.0, -91.0, -83.0, -90.0));
        verify_get_distance(b, rect_from_degrees(-90.0, 181.0, -89.0, 182.0));
        verify_get_distance(b, rect_from_degrees(-84.0, 181.0, -83.0, 182.0));
    }

    #[test]
    fn test_from_point_pair() {
        let a = LatLng::from_degrees(10.0, 20.0);
        let b = LatLng::from_degrees(30.0, 40.0);
        let r = Rect::from_point_pair(a, b);
        assert!(r.contains_lat_lng(a));
        assert!(r.contains_lat_lng(b));
        assert!(r.contains_lat_lng(LatLng::from_degrees(20.0, 30.0)));
        assert!(!r.contains_lat_lng(LatLng::from_degrees(5.0, 25.0)));

        // Order shouldn't matter.
        let r2 = Rect::from_point_pair(b, a);
        assert_eq!(r, r2);
    }

    #[test]
    fn test_full_lat_lng() {
        let fl = Rect::full_lat();
        assert!(fl.contains(-FRAC_PI_2));
        assert!(fl.contains(FRAC_PI_2));
        assert!(!fl.contains(2.0));

        let flng = Rect::full_lng();
        assert!(flng.is_full());
    }

    #[test]
    fn test_is_inverted() {
        assert!(!Rect::full().is_inverted());

        // A rect crossing the antimeridian has an inverted lng interval.
        let r = Rect {
            lat: r1::Interval::from_point_pair(0.0, 1.0),
            lng: s1::Interval::new(2.0, -2.0), // crosses antimeridian
        };
        assert!(r.is_inverted());

        // A normal rect is not inverted.
        let r2 = rect_from_degrees(10.0, 20.0, 30.0, 40.0);
        assert!(!r2.is_inverted());
    }

    #[test]
    fn test_interior_contains_lat_lng() {
        let r = rect_from_degrees(10.0, 20.0, 30.0, 40.0);
        // Point strictly inside.
        assert!(r.interior_contains_lat_lng(LatLng::from_degrees(20.0, 30.0)));
        // Point on boundary should NOT be interior-contained.
        assert!(!r.interior_contains_lat_lng(LatLng::from_degrees(10.0, 30.0)));
        assert!(!r.interior_contains_lat_lng(LatLng::from_degrees(20.0, 20.0)));
    }

    #[test]
    fn test_interior_contains_point() {
        let r = rect_from_degrees(10.0, 20.0, 30.0, 40.0);
        let inside = LatLng::from_degrees(20.0, 30.0).to_point();
        assert!(r.interior_contains_point(inside));
    }

    #[test]
    fn test_interior_contains_rect() {
        let outer = rect_from_degrees(0.0, 0.0, 40.0, 40.0);
        let inner = rect_from_degrees(10.0, 10.0, 30.0, 30.0);
        assert!(outer.interior_contains(inner));
        // A rect cannot interior-contain itself (boundary touches).
        assert!(!outer.interior_contains(outer));
    }

    #[test]
    fn test_interior_intersects() {
        let a = rect_from_degrees(0.0, 0.0, 20.0, 20.0);
        let b = rect_from_degrees(10.0, 10.0, 30.0, 30.0);
        assert!(a.interior_intersects(b));

        // Rects that only share an edge should not interior-intersect.
        let c = rect_from_degrees(20.0, 0.0, 30.0, 20.0);
        assert!(!a.interior_intersects(c));
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn make_lat_lng(lat_deg: i32, lng_deg: i32) -> LatLng {
        let lat = f64::from(lat_deg % 90).clamp(-90.0, 90.0);
        let lng = f64::from(lng_deg % 180).clamp(-180.0, 180.0);
        LatLng::from_degrees(lat, lng)
    }

    #[quickcheck]
    fn prop_from_point_contains(lat: i32, lng: i32) -> bool {
        let ll = make_lat_lng(lat, lng);
        Rect::from_lat_lng(ll).contains_lat_lng(ll)
    }

    #[quickcheck]
    fn prop_union_contains_both(lat1: i32, lng1: i32, lat2: i32, lng2: i32) -> bool {
        let r1 = Rect::from_lat_lng(make_lat_lng(lat1, lng1));
        let r2 = Rect::from_lat_lng(make_lat_lng(lat2, lng2));
        let u = r1.union(r2);
        u.contains(r1) && u.contains(r2)
    }

    #[quickcheck]
    fn prop_empty_is_empty() -> bool {
        Rect::empty().is_empty()
    }

    #[quickcheck]
    fn prop_full_is_full() -> bool {
        Rect::full().is_full()
    }

    #[quickcheck]
    fn prop_area_non_negative(lat: i32, lng: i32) -> bool {
        let r = Rect::from_lat_lng(make_lat_lng(lat, lng));
        r.area() >= 0.0
    }

    #[quickcheck]
    fn prop_contains_self(lat: i32, lng: i32) -> bool {
        let r = Rect::from_lat_lng(make_lat_lng(lat, lng));
        r.contains(r)
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(lat_deg: i32, lng_deg: i32) -> bool {
        let lat = f64::from(lat_deg % 90).clamp(-90.0, 90.0);
        let lng = f64::from(lng_deg % 180).clamp(-180.0, 180.0);
        let ll = LatLng::from_degrees(lat, lng);
        let r = Rect::from_lat_lng(ll);
        let json1 = serde_json::to_string(&r).unwrap();
        let back: Rect = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        let back2: Rect = serde_json::from_str(&json2).unwrap();
        back == back2
    }

    // --- New foundational tests ---

    #[test]
    fn test_expand_for_subregions_normal() {
        // A small rect far from poles/antimeridian should expand slightly.
        let r = Rect::from_center_size(
            LatLng::from_degrees(45.0, 90.0),
            LatLng::from_degrees(10.0, 10.0),
        );
        let expanded = r.expand_for_subregions();
        assert!(!expanded.is_full());
        // Should be slightly larger than original.
        assert!(expanded.lat.lo <= r.lat.lo);
        assert!(expanded.lat.hi >= r.lat.hi);
    }

    #[test]
    fn test_expand_for_subregions_nearly_antipodal() {
        // A rect spanning nearly the full longitude should return full.
        let r = Rect {
            lat: r1::Interval::new(-0.01, 0.01),
            lng: s1::Interval::new(-PI + 1e-16, PI - 1e-16),
        };
        let expanded = r.expand_for_subregions();
        assert!(
            expanded.is_full(),
            "nearly-antipodal rect should expand to full"
        );
    }

    #[test]
    fn test_expand_for_subregions_empty() {
        assert!(Rect::empty().expand_for_subregions().is_empty());
    }

    #[test]
    fn test_cap_bound_all_four_vertices() {
        // For a rect spanning ≤180° in longitude, cap_bound should contain
        // all 4 corners.
        let r = Rect::from_center_size(
            LatLng::from_degrees(30.0, 50.0),
            LatLng::from_degrees(40.0, 80.0),
        );
        let cap = r.cap_bound();
        for rv in RectVertex::iter() {
            let v = r.vertex(rv).to_point();
            assert!(
                cap.contains_point(v),
                "cap_bound doesn't contain vertex {rv:?}"
            );
        }
    }

    #[test]
    fn test_expanded_by_distance() {
        use crate::s1::Angle;
        let r = Rect::from_center_size(
            LatLng::from_degrees(0.0, 0.0),
            LatLng::from_degrees(10.0, 10.0),
        );
        let expanded = r.expanded_by_distance(Angle::from_degrees(1.0));
        // Expanded rect should strictly contain the original.
        assert!(expanded.lat.lo < r.lat.lo);
        assert!(expanded.lat.hi > r.lat.hi);
        assert!(expanded.lng.lo < r.lng.lo);
        assert!(expanded.lng.hi > r.lng.hi);
    }

    #[test]
    fn test_rect_distance_to_lat_lng() {
        use crate::s1::Angle;
        let r = Rect::from_center_size(
            LatLng::from_degrees(0.0, 0.0),
            LatLng::from_degrees(10.0, 10.0),
        );
        // Center should have distance 0.
        let center_dist = r.get_distance_to_latlng(LatLng::from_degrees(0.0, 0.0));
        assert!(center_dist.radians() < 1e-10);
        // A point outside should have positive distance.
        let far_dist = r.get_distance_to_latlng(LatLng::from_degrees(20.0, 0.0));
        assert!(far_dist > Angle::from_degrees(10.0));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_rect_vertex_roundtrip() {
        for v in [
            RectVertex::LowerLeft,
            RectVertex::LowerRight,
            RectVertex::UpperRight,
            RectVertex::UpperLeft,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: RectVertex = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }
}
