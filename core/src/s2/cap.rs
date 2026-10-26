// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A spherical cap (disc-shaped region on a unit sphere).
//!
//! Corresponds to C++ `S2Cap`, Go `s2.Cap`, Java `S2Cap`.
//!
//! A `Cap` is defined by a center point and a radius. Technically this shape
//! is called a "spherical cap" because it is not planar; the cap represents a
//! portion of the sphere that has been cut off by a plane. The boundary of
//! the cap is the circle defined by the intersection of the sphere and the
//! plane. For containment purposes, the cap is a closed set, i.e. it
//! contains its boundary.
//!
//! The radius of the cap is measured along the surface of the sphere (rather
//! than the straight-line distance through the interior). Thus a cap of
//! radius π/2 is a hemisphere, and a cap of radius π covers the entire
//! sphere.
//!
//! Here are some useful relationships between the cap height (h), the cap
//! radius (r), the maximum chord length from the cap's center (d), and the
//! radius of the cap's base (a):
//!
//! ```text
//!   h = 1 - cos(r)
//!     = 2 * sin²(r/2)
//!  d² = 2 * h
//!     = a² + h²
//! ```

#![expect(
    clippy::cast_sign_loss,
    reason = "MAX_CELL_LEVEL (u8) cast to i32 — always fits"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "MAX_CELL_LEVEL (u8->i32) clamping — always fits"
)]
use crate::r3::Vector;
use crate::s1::{Angle, ChordAngle};
use crate::s2::metric::MIN_WIDTH;
use crate::s2::{Cell, CellEdge, CellId, LatLng, Point};
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;

/// A spherical cap (disc-shaped region on a unit sphere).
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::s1::Angle;
/// use s2rst::s2::{Cap, Point};
///
/// // Create a cap centered on the north pole with a 45-degree radius.
/// let center = Point::from_coords(0.0, 0.0, 1.0);
/// let cap = Cap::from_center_angle(center, Angle::from_degrees(45.0));
/// assert!(!cap.is_empty());
/// assert!(!cap.is_full());
///
/// // Containment: a point near the north pole is inside.
/// let near_pole = Point::from_coords(0.0, 0.1, 1.0);
/// assert!(cap.contains_point(near_pole));
///
/// // A point on the equator is outside a 45-degree polar cap.
/// let equator = Point::from_coords(1.0, 0.0, 0.0);
/// assert!(!cap.contains_point(equator));
///
/// // Area of the cap (in steradians).
/// let area = cap.area();
/// assert!(area > 0.0 && area < 4.0 * std::f64::consts::PI);
///
/// // The complement covers the rest of the sphere.
/// let comp = cap.complement();
/// assert!(comp.contains_point(equator));
/// assert!(!comp.contains_point(near_pole));
/// ```
#[must_use]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Cap {
    center: Point,
    radius: ChordAngle,
}

impl Cap {
    // --- Constructors ---

    /// Creates a cap with the given center and chord angle radius.
    /// This is the most efficient constructor.
    #[inline]
    pub fn from_center_chord_angle(center: Point, radius: ChordAngle) -> Self {
        Cap { center, radius }
    }

    /// Creates a cap with the given center and angle radius.
    ///
    /// The angle is clamped to at most π. A negative angle yields an empty
    /// cap, and an angle of π or more yields a full cap.
    ///
    /// # Panics
    ///
    /// In debug builds, when the result is not valid: the centre is not unit length (or the
    /// angle is NaN, which gives the radius `STRAIGHT`, still valid). Upstream's constructor
    /// checks validity in debug builds too.
    #[inline]
    pub fn from_center_angle(center: Point, angle: Angle) -> Self {
        // Match C++: clamp to π before converting to chord angle.
        let clamped = if angle.radians() > PI {
            Angle::from_radians(PI)
        } else {
            angle
        };
        let cap = Self::from_center_chord_angle(center, ChordAngle::from_angle(clamped));
        debug_assert!(cap.is_valid());
        cap
    }

    /// Creates a cap containing a single point.
    #[inline]
    pub fn from_point(p: Point) -> Self {
        Self::from_center_chord_angle(p, ChordAngle::ZERO)
    }

    /// Creates a cap with the given center and height. The height is the
    /// distance from the center point to the cutoff plane. A negative height
    /// yields an empty cap; a height of 2 or more yields a full cap.
    #[inline]
    pub fn from_center_height(center: Point, height: f64) -> Self {
        Self::from_center_chord_angle(center, ChordAngle::from_length2(2.0 * height))
    }

    /// Creates a cap with the given center and surface area. The area can
    /// also be interpreted as the solid angle subtended by the cap (because
    /// the sphere has unit radius). A negative area yields an empty cap;
    /// an area of 4π or more yields a full cap.
    #[inline]
    pub fn from_center_area(center: Point, area: f64) -> Self {
        Self::from_center_chord_angle(center, ChordAngle::from_length2(area / PI))
    }

    /// Returns an empty cap (contains no points).
    #[inline]
    pub fn empty() -> Self {
        Self::from_center_chord_angle(
            Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
            ChordAngle::NEGATIVE,
        )
    }

    /// Returns a full cap (contains all points).
    #[inline]
    pub fn full() -> Self {
        Self::from_center_chord_angle(
            Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
            ChordAngle::STRAIGHT,
        )
    }

    // --- Accessors ---

    /// Returns the center of the cap.
    #[inline]
    pub fn center(self) -> Point {
        self.center
    }

    /// Returns the radius as a chord angle.
    #[inline]
    pub fn chord_radius(self) -> ChordAngle {
        self.radius
    }

    /// Returns the cap radius as an [`Angle`]. This requires a trigonometric
    /// operation and may yield a slightly different result than the value
    /// passed to [`Cap::from_center_angle`].
    #[inline]
    pub fn angle_radius(self) -> Angle {
        self.radius.to_angle()
    }

    /// Returns the height of the cap. This is the distance from the center
    /// point to the cutoff plane.
    #[inline]
    pub fn height(self) -> f64 {
        0.5 * self.radius.length2()
    }

    /// Returns the surface area of the cap on the unit sphere.
    #[inline]
    pub fn area(self) -> f64 {
        // Upstream's `std::max(0.0, height)`, which gives +0 for a height of −0;
        // `0.0_f64.max(h)` leaves that sign unspecified (spec 008 D-009).
        let h = self.height();
        2.0 * PI * if 0.0 < h { h } else { 0.0 }
    }

    // --- Predicates ---

    /// Reports whether the cap is valid.
    #[inline]
    pub fn is_valid(self) -> bool {
        self.center.is_unit() && self.radius.length2() <= 4.0
    }

    /// Reports whether the cap is empty (contains no points).
    #[inline]
    pub fn is_empty(self) -> bool {
        self.radius.is_negative()
    }

    /// Reports whether the cap is full (contains all points).
    #[inline]
    pub fn is_full(self) -> bool {
        self.radius == ChordAngle::STRAIGHT
    }

    // --- Containment & intersection ---

    /// Reports whether the cap contains the given point.
    ///
    /// `p` should be unit length, as upstream requires (`S2Cap::Contains` checks it in debug
    /// builds; the library does not). For another point the result compares the straight-line
    /// distance from the centre to `p` with the radius's chord, which has no meaning on the
    /// sphere.
    #[inline]
    pub fn contains_point(self, p: Point) -> bool {
        self.center.chord_angle(p) <= self.radius
    }

    /// Reports whether the point is within the interior of this cap (i.e.
    /// the cap minus its boundary). `p` should be unit length, as for
    /// [`contains_point`](Cap::contains_point).
    #[inline]
    pub fn interior_contains_point(self, p: Point) -> bool {
        self.is_full() || self.center.chord_angle(p) < self.radius
    }

    /// Reports whether this cap contains the other cap.
    pub fn contains(self, other: Cap) -> bool {
        if self.is_full() || other.is_empty() {
            return true;
        }
        self.radius >= self.center.chord_angle(other.center) + other.radius
    }

    /// Reports whether this cap intersects the other cap (i.e. they have any
    /// points in common).
    pub fn intersects(self, other: Cap) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        self.radius + other.radius >= self.center.chord_angle(other.center)
    }

    /// Reports whether the interior of this cap intersects the other cap.
    pub fn interior_intersects(self, other: Cap) -> bool {
        // Make sure this cap has an interior and the other cap is non-empty.
        if self.radius <= ChordAngle::ZERO || other.is_empty() {
            return false;
        }
        self.radius + other.radius > self.center.chord_angle(other.center)
    }

    // --- Set operations ---

    /// Returns the complement of the interior of the cap. A cap and its
    /// complement have the same boundary but do not share any interior
    /// points. The complement operator is not a bijection because the
    /// complement of a singleton cap (containing a single point) is the
    /// same as the complement of an empty cap.
    pub fn complement(self) -> Cap {
        if self.is_full() {
            return Cap::empty();
        }
        if self.is_empty() {
            return Cap::full();
        }
        // Upstream's radius, `FromLength2(4 − r²)`. (`STRAIGHT − radius` squared a square root
        // and differed in the last bits; spec 008 K4, FR-019.)
        Cap::from_center_chord_angle(
            -self.center,
            ChordAngle::from_length2(4.0 - self.radius.length2()),
        )
    }

    /// Returns a cap expanded by the given distance. If the cap is empty,
    /// it returns an empty cap.
    ///
    /// # Panics
    ///
    /// In debug builds, when `distance` is negative or NaN (upstream's `ABSL_DCHECK_GE`), and
    /// when it is +∞: its chord angle is `INFINITY`, and the chord-angle sum checks for special
    /// operands (spec 007 FR-019), as upstream's does.
    pub fn expanded(self, distance: Angle) -> Cap {
        debug_assert!(distance.radians() >= 0.0);
        if self.is_empty() {
            return Cap::empty();
        }
        Cap::from_center_chord_angle(self.center, self.radius + ChordAngle::from_angle(distance))
    }

    /// Returns the smallest cap which encloses both this cap and `other`.
    ///
    /// **Deviation from upstream:** `S2Cap::Union` returns the cap through the computed centre
    /// and radius as they are; the library then adds the chord angle's conversion and point
    /// errors (`max_angle_error`, `max_point_error`) to the radius, to absorb the rounding of the
    /// conversion between `Angle` and `ChordAngle` and of the interpolated centre. The radius can
    /// therefore be a few ulps larger than C++'s.
    ///
    /// Neither version is guaranteed to contain both caps: on the C++ reference vectors
    /// (`core/tests/cpp_cap_diff.rs`), upstream's result misses part of one cap on 337 of 1,089
    /// rows, by rounding, and the library's on 7, all where one cap is a single point or nearly
    /// one (spec 008 K9; upstream's comment says "encloses", `UPSTREAM.md` U-015). Use
    /// [`add_cap`](Cap::add_cap), whose margin keeps the centre, when containment matters.
    pub fn union(self, other: Cap) -> Cap {
        // If the other cap is larger, swap.
        if self.radius < other.radius {
            return other.union(self);
        }
        if self.is_full() || other.is_empty() {
            return self;
        }

        // This calculation works in terms of s1::Angle for simplicity.
        let this_radius = self.angle_radius();
        let other_radius = other.angle_radius();
        let distance = Angle::from_radians(self.center.vector().angle(other.center.vector()));

        if this_radius >= distance + other_radius {
            return self;
        }

        let result_radius = (distance + this_radius + other_radius) * 0.5;
        let result_center = interpolate_at_distance(
            (distance - this_radius + other_radius) * 0.5,
            self.center,
            other.center,
        );
        // Add a small error margin to account for floating-point imprecision
        // in the Angle → ChordAngle conversion and point interpolation.
        let mut cap = Cap::from_center_angle(result_center, result_radius);
        cap.radius = cap
            .radius
            .plus_error(cap.radius.max_angle_error() + cap.radius.max_point_error());
        cap
    }

    /// Expands this cap if necessary to include the given point. If the cap
    /// is empty, the center is set to the point with a zero radius.
    pub fn add_point(self, p: Point) -> Cap {
        if self.is_empty() {
            return Cap::from_point(p);
        }
        let new_rad = self.center.chord_angle(p);
        if new_rad > self.radius {
            Cap::from_center_chord_angle(self.center, new_rad)
        } else {
            self
        }
    }

    /// Expands this cap if necessary to include the other cap.
    pub fn add_cap(self, other: Cap) -> Cap {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        // We round up the distance to ensure that the cap is actually
        // contained.
        let dist = self.center.chord_angle(other.center) + other.radius;
        let new_rad = dist.plus_error((2.0 * f64::EPSILON + 2.02 * f64::EPSILON) * dist.length2());
        if new_rad > self.radius {
            Cap::from_center_chord_angle(self.center, new_rad)
        } else {
            self
        }
    }

    // --- Region-like methods ---

    /// Returns this cap as its own bounding cap.
    #[inline]
    pub fn cap_bound(self) -> Cap {
        self
    }

    /// Returns a bounding latitude-longitude rectangle for this cap.
    pub fn rect_bound(self) -> crate::s2::Rect {
        use crate::s2::Rect;

        if self.is_empty() {
            return Rect::empty();
        }

        let cap_angle = self.angle_radius().radians();
        let center_lat = LatLng::latitude(self.center).radians();
        let center_lng = LatLng::longitude(self.center).radians();
        let mut all_longitudes = false;

        let mut lat = crate::r1::Interval::new(center_lat - cap_angle, center_lat + cap_angle);
        let mut lng = crate::s1::Interval::full();

        if lat.lo <= -FRAC_PI_2 {
            lat.lo = -FRAC_PI_2;
            all_longitudes = true;
        }
        if lat.hi >= FRAC_PI_2 {
            lat.hi = FRAC_PI_2;
            all_longitudes = true;
        }

        if !all_longitudes {
            let sin_a = self.radius.sin();
            let sin_c = center_lat.cos();
            if sin_a <= sin_c {
                let angle_a = (sin_a / sin_c).asin();
                lng = crate::s1::Interval::new(
                    reduce_longitude(center_lng - angle_a),
                    reduce_longitude(center_lng + angle_a),
                );
            }
        }

        Rect::new(lat, lng)
    }

    /// Computes a covering of the cap (at most 4-6 cells): the six faces, or the four cells at
    /// upstream's level around the cell vertex nearest the centre.
    ///
    /// As upstream's `GetCellUnionBound`, the empty cap is not treated apart: its radius, −1
    /// radian, gives level 29 and the four cells around its centre (spec 008 K8).
    pub fn cell_union_bound(self) -> Vec<CellId> {
        // Upstream: the maximum level such that the cap contains at most one cell vertex,
        // `kMinWidth.GetLevelForMinValue(radius) - 1`, through the exact `ilogb` of
        // `MIN_WIDTH.max_level`. (`floor(log2(...))` rounds a quotient just below a power of
        // two up to it, which gave a level one too fine; spec 008 K1, FR-017.)
        let radius = self.angle_radius().radians();
        let level = i32::from(MIN_WIDTH.max_level(radius).as_u8()) - 1;

        if level < 0 {
            return (0..6).map(CellId::from_face).collect();
        }

        CellId::from_point(&self.center).vertex_neighbors(level as u8)
    }

    /// Returns the true centroid of the cap multiplied by its surface area.
    ///
    /// The result lies on the ray from the origin through the cap's center,
    /// but it is not unit length. For caps that contain a single point (zero
    /// radius), this method returns the origin.
    pub fn centroid(self) -> Point {
        if self.is_empty() {
            return Point::default();
        }
        let r = 1.0 - 0.5 * self.height();
        Point(self.center.vector() * (r * self.area()))
    }

    // --- Cell tests (upstream's `Contains(const S2Cell&)` and `MayIntersect`) ---

    /// Reports whether the cap contains the cell: upstream's `S2Cap::Contains(const S2Cell&)`.
    /// The four vertices first; then the complement of the cap must not intersect the cell's
    /// edges (slightly conservative, as upstream: technically the complement's interior). The
    /// vertices are tested before the complement is taken, because the complement of a very
    /// small cap cannot be represented accurately.
    pub(crate) fn contains_cell_upstream(self, cell: &Cell) -> bool {
        let vertices = [
            cell.vertex(0),
            cell.vertex(1),
            cell.vertex(2),
            cell.vertex(3),
        ];
        if !vertices.iter().all(|&v| self.contains_point(v)) {
            return false;
        }
        !self.complement().intersects_cell_edges(cell, &vertices)
    }

    /// Reports whether the cap may intersect the cell: upstream's `S2Cap::MayIntersect`. True if
    /// the cap contains a vertex, otherwise the edge test.
    pub(crate) fn may_intersect_cell(self, cell: &Cell) -> bool {
        let vertices = [
            cell.vertex(0),
            cell.vertex(1),
            cell.vertex(2),
            cell.vertex(3),
        ];
        if vertices.iter().any(|&v| self.contains_point(v)) {
            return true;
        }
        self.intersects_cell_edges(cell, &vertices)
    }

    /// Reports whether the cap intersects any point of the cell other than its vertices, which
    /// the caller has already tested: upstream's private
    /// `S2Cap::Intersects(const S2Cell&, const S2Point* vertices)` (`s2cap.cc:225–264`).
    fn intersects_cell_edges(self, cell: &Cell, vertices: &[Point; 4]) -> bool {
        // If the cap is a hemisphere or larger, the cell and the complement of the cap are both
        // convex. Therefore, since no vertex of the cell is contained, no other interior point
        // of the cell is contained either.
        if self.radius >= ChordAngle::RIGHT {
            return false;
        }
        // We need to check for empty caps due to the center check just below.
        if self.is_empty() {
            return false;
        }
        // Optimization: return true if the cell contains the cap center. (This allows half of
        // the edge checks below to be skipped.)
        if cell.contains_point(self.center) {
            return true;
        }
        // At this point we know that the cell does not contain the cap center, and the cap
        // does not contain any cell vertex. The only way that they can intersect is if the cap
        // intersects the interior of some edge.
        let sin2_angle = self.radius.sin2();
        let center = self.center.vector();
        for (k, &edge_id) in CellEdge::ALL.iter().enumerate() {
            let edge = cell.edge_raw(edge_id).vector();
            let dot = center.dot(edge);
            if dot > 0.0 {
                // The center is in the interior half-space defined by the edge. We don't need
                // to consider these edges, since if the cap intersects this edge then it also
                // intersects the edge on the opposite side of the cell (because we know the
                // center is not contained with the cell).
                continue;
            }
            // The norm2() factor is necessary because "edge" is not normalized.
            if dot * dot > sin2_angle * edge.norm2() {
                return false; // Entire cap is on the exterior side of this edge.
            }
            // Otherwise, the great circle containing this edge intersects the interior of the
            // cap. We just need to check whether the point of closest approach occurs between
            // the two edge endpoints.
            let dir = edge.cross(center);
            if dir.dot(vertices[k].vector()) < 0.0 && dir.dot(vertices[(k + 1) & 3].vector()) > 0.0
            {
                return true;
            }
        }
        false
    }

    // --- Comparison ---

    /// Reports whether this cap is equal to the other cap, handling the
    /// special cases where empty and full caps can have different centers.
    pub fn equal(self, other: Cap) -> bool {
        (self.radius == other.radius && self.center == other.center)
            || (self.is_empty() && other.is_empty())
            || (self.is_full() && other.is_full())
    }

    /// Reports whether this cap is approximately equal to the other cap within upstream's
    /// default tolerance of 10⁻¹⁴, for the centres' angle and the radii's squared lengths alike
    /// (upstream's `ApproxEquals` with its default `max_error`).
    pub fn approx_eq(self, other: Cap) -> bool {
        self.approx_eq_with(other, 1e-14)
    }

    /// Reports whether this cap is approximately equal to the other cap
    /// within the given tolerance.
    pub fn approx_eq_with(self, other: Cap, max_error: f64) -> bool {
        let r2 = self.radius.length2();
        let other_r2 = other.radius.length2();
        // Upstream compares the centres within `max_error` radians too (`S2::ApproxEquals`);
        // a fixed 10⁻¹⁵ ignored the tolerance (spec 008 K6, FR-022).
        (self
            .center
            .approx_eq_angle(other.center, Angle::from_radians(max_error))
            && (r2 - other_r2).abs() <= max_error)
            || (self.is_empty() && other_r2 <= max_error)
            || (other.is_empty() && r2 <= max_error)
            || (self.is_full() && other_r2 >= 2.0 - max_error)
            || (other.is_full() && r2 >= 2.0 - max_error)
    }
}

/// Reduces a longitude bound of [`Cap::rect_bound`] to [−π, π]: the IEEE remainder by the
/// `f64` nearest 2π, as upstream's `remainder(x, 2 * M_PI)`, exactly, without `%`.
///
/// The argument is a centre longitude in [−π, π] plus or minus an `asin` result in [0, π/2], so
/// |x| ≤ 3π/2 (plus a rounding). Then one correction by 2π is the remainder, and it is exact:
/// for π < |x| ≤ 4π, `x ∓ 2π` has no rounding error (Sterbenz). At |x| = π exactly, the
/// remainder's quotient rounds to the even 0, so `x` is kept, as here. (The previous
/// `(x + π).rem_euclid(2π) − π` rounded `x + π`: a cap at longitude 10⁻¹⁴° got the rectangle
/// `[0, 0]`, without its own centre; spec 008 K5, FR-020.)
fn reduce_longitude(x: f64) -> f64 {
    let two_pi = 2.0 * PI;
    if x > PI {
        x - two_pi
    } else if x < -PI {
        x + two_pi
    } else {
        x
    }
}

/// Interpolates along the great circle from `a` toward `b` by the given
/// angle distance. Both `a` and `b` must be unit length.
///
/// This is a simplified version of `InterpolateAtDistance` from `edge_distances`
/// that suffices for `Cap::union`.
fn interpolate_at_distance(ax: Angle, a: Point, b: Point) -> Point {
    let normal = a.point_cross(b);
    let tangent = normal.vector().cross(a.vector());

    let (sin_a, cos_a) = ax.sin_cos();
    let t_norm = tangent.norm();
    if t_norm == 0.0 {
        return a;
    }
    Point((a.vector() * cos_a + tangent * (sin_a / t_norm)).normalize())
}

/// Deserialises a cap only if it is valid (`is_valid`: a unit-length centre and a radius of
/// length² at most 4), as the binary decoder accepts only valid caps. Anything else is an error
/// naming the rule. A derived impl would accept any centre, NaN included: a way to build a cap
/// that no constructor and no decoder returns (spec 008 K7, FR-023). The serialised form is
/// unchanged.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Cap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        #[derive(serde::Deserialize)]
        #[serde(rename = "Cap")]
        struct Fields {
            center: Point,
            radius: ChordAngle,
        }

        let f = Fields::deserialize(deserializer)?;
        if !f.center.is_unit() {
            return Err(D::Error::custom("cap centre is not unit length"));
        }
        // Unreachable through `ChordAngle`'s own deserialiser (spec 007 FR-020), kept so that
        // this impl states the whole rule.
        if f.radius.length2() > 4.0 {
            return Err(D::Error::custom("cap radius is above STRAIGHT"));
        }
        Ok(Cap::from_center_chord_angle(f.center, f.radius))
    }
}

impl PartialEq for Cap {
    fn eq(&self, other: &Self) -> bool {
        self.equal(*other)
    }
}

impl fmt::Display for Cap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[center={}, radius={:.6}°]",
            self.center,
            self.angle_radius().degrees(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn cap_is_send_sync() {
        is_send_sync::<Cap>();
    }

    const EPSILON: f64 = 1e-14;

    fn float64_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-15
    }

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn x_axis_pt() -> Point {
        Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        })
    }

    fn y_axis_pt() -> Point {
        Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        })
    }

    fn x_axis() -> Cap {
        Cap::from_point(x_axis_pt())
    }

    fn y_axis() -> Cap {
        Cap::from_point(y_axis_pt())
    }

    fn x_comp() -> Cap {
        x_axis().complement()
    }

    fn hemi() -> Cap {
        Cap::from_center_height(Point::from_coords(1.0, 0.0, 1.0), 1.0)
    }

    fn tiny() -> Cap {
        Cap::from_center_angle(
            Point::from_coords(1.0, 2.0, 3.0),
            Angle::from_radians(1e-10),
        )
    }

    // --- Basic tests ---

    #[test]
    fn test_basic_empty_full_valid() {
        let empty = Cap::empty();
        let full = Cap::full();

        assert!(empty.is_empty());
        assert!(!empty.is_full());
        assert!(empty.is_valid());

        assert!(!full.is_empty());
        assert!(full.is_full());
        assert!(full.is_valid());

        assert!(!full.complement().is_full());
        assert!(full.complement().is_empty());
        assert!(full.complement().is_valid());

        assert!(empty.complement().is_full());
        assert!(!empty.complement().is_empty());
        assert!(empty.complement().is_valid());

        // x_comp is the complement of the x-axis singleton, which is full.
        assert!(x_comp().is_full());
        assert!(x_comp().is_valid());
        assert!(x_comp().complement().is_empty());

        assert!(!tiny().is_empty());
        assert!(!tiny().is_full());
        assert!(tiny().is_valid());

        assert!(!hemi().is_empty());
        assert!(!hemi().is_full());
        assert!(hemi().is_valid());
    }

    #[test]
    fn test_center_height_radius() {
        // Complement of complement shouldn't be exactly equal due to precision.
        assert_ne!(x_axis(), x_axis().complement().complement());

        assert_eq!(Cap::full().height(), 2.0);
        assert_eq!(Cap::full().angle_radius().degrees(), 180.0);

        assert_eq!(Cap::empty().center(), Cap::empty().center());
        assert_eq!(Cap::empty().height(), Cap::empty().height());

        assert_eq!(y_axis().height(), 0.0);
        assert_eq!(x_axis().height(), 0.0);
        assert_eq!(x_axis().angle_radius().radians(), 0.0);

        // Hemi center should be the negative of complement's center.
        let hc = -hemi().center();
        assert_eq!(hc, hemi().complement().center());
        assert_eq!(hemi().height(), 1.0);
    }

    #[test]
    fn test_contains_cap() {
        let empty = Cap::empty();
        let full = Cap::full();

        assert!(empty.contains(empty));
        assert!(full.contains(empty));
        assert!(full.contains(full));
        assert!(!empty.contains(x_axis()));
        assert!(full.contains(x_axis()));
        assert!(!x_axis().contains(full));
        assert!(x_axis().contains(x_axis()));
        assert!(x_axis().contains(empty));
        assert!(hemi().contains(tiny()));

        assert!(hemi().contains(Cap::from_center_angle(
            x_axis_pt(),
            Angle::from_radians(PI / 4.0 - EPSILON),
        )));
        assert!(!hemi().contains(Cap::from_center_angle(
            x_axis_pt(),
            Angle::from_radians(PI / 4.0 + EPSILON),
        )));
    }

    #[test]
    fn test_contains_point() {
        let tiny_cap = tiny();
        let tangent = Point(
            tiny_cap
                .center()
                .vector()
                .cross(Vector {
                    x: 3.0,
                    y: 2.0,
                    z: 1.0,
                })
                .normalize(),
        );
        let tiny_rad = 1e-10;

        assert!(x_axis().contains_point(x_axis_pt()));
        assert!(!x_axis().contains_point(Point(Vector {
            x: 1.0,
            y: 1e-20,
            z: 0.0
        })));
        assert!(!y_axis().contains_point(x_axis().center()));
        assert!(x_comp().contains_point(x_axis().center()));
        assert!(!x_comp().complement().contains_point(x_axis().center()));

        // Points just inside / outside tiny cap boundary.
        assert!(tiny_cap.contains_point(Point(
            tiny_cap.center().vector() + tangent.vector() * (tiny_rad * 0.99)
        )));
        assert!(!tiny_cap.contains_point(Point(
            tiny_cap.center().vector() + tangent.vector() * (tiny_rad * 1.01)
        )));

        assert!(hemi().contains_point(Point::from_coords(1.0, 0.0, -(1.0 - EPSILON))));
        assert!(hemi().contains_point(x_axis_pt()));
        assert!(!hemi().complement().contains_point(x_axis_pt()));
    }

    #[test]
    fn test_interior_intersects() {
        let empty = Cap::empty();
        let full = Cap::full();

        assert!(!empty.interior_intersects(empty));
        assert!(!empty.interior_intersects(x_axis()));
        assert!(!full.interior_intersects(empty));
        assert!(full.interior_intersects(full));
        assert!(full.interior_intersects(x_axis()));
        assert!(!x_axis().interior_intersects(full));
        assert!(!x_axis().interior_intersects(x_axis()));
        assert!(!x_axis().interior_intersects(empty));
    }

    #[test]
    fn test_interior_contains() {
        let p = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: -(1.0 + EPSILON),
        });
        assert!(!hemi().interior_contains_point(p));
    }

    #[test]
    fn test_expanded() {
        let cap50 = Cap::from_center_angle(x_axis_pt(), Angle::from_degrees(50.0));
        let cap51 = Cap::from_center_angle(x_axis_pt(), Angle::from_degrees(51.0));

        assert!(Cap::empty().expanded(Angle::from_radians(2.0)).is_empty());
        assert!(Cap::full().expanded(Angle::from_radians(2.0)).is_full());

        assert!(cap50.expanded(Angle::ZERO).approx_eq(cap50));
        assert!(cap50.expanded(Angle::from_degrees(1.0)).approx_eq(cap51));

        assert!(!cap50.expanded(Angle::from_degrees(129.99)).is_full());
        assert!(cap50.expanded(Angle::from_degrees(130.01)).is_full());
    }

    #[test]
    fn test_add_point() {
        // Cap plus its center equals itself.
        assert!(x_axis().add_point(x_axis_pt()).approx_eq(x_axis()));
        assert!(y_axis().add_point(y_axis_pt()).approx_eq(y_axis()));

        // Cap plus opposite point equals full.
        assert!(
            x_axis()
                .add_point(Point(Vector {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0
                }))
                .approx_eq(Cap::full())
        );
        assert!(
            y_axis()
                .add_point(Point(Vector {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0
                }))
                .approx_eq(Cap::full())
        );

        // Cap plus orthogonal axis equals half cap.
        let half_cap = Cap::from_center_angle(x_axis_pt(), Angle::from_radians(PI / 2.0));
        assert!(
            x_axis()
                .add_point(Point(Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0
                }))
                .approx_eq(half_cap)
        );
        assert!(
            x_axis()
                .add_point(Point(Vector {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0
                }))
                .approx_eq(half_cap)
        );

        // Hemi plus points already inside doesn't change.
        assert!(
            hemi()
                .add_point(Point(Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 1.0
                }))
                .approx_eq(hemi())
        );
        assert!(
            hemi()
                .add_point(Point(Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0
                }))
                .approx_eq(hemi())
        );

        // Hemi plus point requiring expansion.
        let want = Cap::from_center_angle(
            Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 1.0,
            })
            .normalize(),
            Angle::from_degrees(120.0),
        );
        assert!(
            hemi()
                .add_point(Point::from_coords(0.0, 1.0, -1.0))
                .approx_eq(want)
        );
        assert!(
            hemi()
                .add_point(Point::from_coords(0.0, -1.0, -1.0))
                .approx_eq(want)
        );
    }

    #[test]
    fn test_add_cap() {
        let empty = Cap::empty();
        let full = Cap::full();

        // Identity cases.
        assert!(empty.add_cap(empty).approx_eq(empty));
        assert!(full.add_cap(full).approx_eq(full));

        // Anything plus empty equals itself.
        assert!(full.add_cap(empty).approx_eq(full));
        assert!(empty.add_cap(full).approx_eq(full));
        assert!(x_axis().add_cap(empty).approx_eq(x_axis()));
        assert!(empty.add_cap(x_axis()).approx_eq(x_axis()));

        // Two halves make a whole.
        assert!(x_axis().add_cap(x_comp()).approx_eq(full));

        // Two zero-height orthogonal axis caps make a half-cap.
        let half_cap = Cap::from_center_angle(x_axis_pt(), Angle::from_radians(PI / 2.0));
        assert!(x_axis().add_cap(y_axis()).approx_eq(half_cap));
    }

    #[test]
    fn test_centroid() {
        // Empty cap centroid is zero.
        let empty_centroid = Cap::empty().centroid();
        assert!(empty_centroid.approx_eq(Point::default()));

        // Full cap centroid should be near zero.
        assert!(Cap::full().centroid().vector().norm() < 1e-15);

        // Check centroid formula for known heights.
        for i in 0..100 {
            let center = Point::from_coords(
                (f64::from(i) * 0.7).cos(),
                (f64::from(i) * 0.7).sin(),
                (f64::from(i) * 0.3).cos(),
            );
            let height = (f64::from(i) * 0.02).min(2.0);
            let c = Cap::from_center_height(center, height);
            let got = c.centroid();
            let want = center.vector() * ((1.0 - height / 2.0) * c.area());
            assert!(
                (got.vector() - want).norm() < 1e-14,
                "centroid mismatch for height={height}",
            );
        }
    }

    #[test]
    fn test_union() {
        let empty = Cap::empty();
        let full = Cap::full();

        let p1 = Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
        let p2 = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });

        // Two very large caps, whose radius sums to excess of 180°, non-antipodal.
        let f = Cap::from_center_angle(p1, Angle::from_degrees(150.0));
        let g = Cap::from_center_angle(p2, Angle::from_degrees(150.0));
        assert!(f.union(g).is_full());

        // Non-overlapping hemisphere caps with antipodal centers.
        let h = Cap::from_center_height(p1, 1.0);
        assert!(h.union(h.complement()).is_full());

        // Union with full / empty.
        let a = Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(0.2));
        assert!(a.union(full).is_full());
        assert!(a.union(empty).approx_eq(a));

        // A cap that entirely contains a.
        let b = Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(0.3));
        assert!(b.contains(a));
        assert!(b.approx_eq(a.union(b)));

        // Two entirely disjoint caps.
        let d = Cap::from_center_angle(Point::from_coords(0.0, 0.0, 1.0), Angle::from_degrees(0.1));
        assert!(!d.contains(a));
        assert!(!d.intersects(a));

        // Union should be symmetric.
        let a_union_d = a.union(d);
        assert!(a_union_d.approx_eq(d.union(a)));
    }

    #[test]
    fn test_equal() {
        assert!(Cap::empty().equal(Cap::empty()));
        assert!(!Cap::empty().equal(Cap::full()));
        assert!(Cap::full().equal(Cap::full()));

        assert!(x_axis().equal(x_axis()));
        assert!(!x_axis().equal(y_axis()));
        assert!(x_comp().equal(x_axis().complement()));
    }

    #[test]
    fn test_approx_equal() {
        assert!(Cap::empty().approx_eq(Cap::empty()));
        assert!(Cap::full().approx_eq(Cap::full()));
        assert!(!Cap::empty().approx_eq(Cap::full()));
    }

    #[test]
    fn test_area() {
        assert_eq!(Cap::empty().area(), 0.0);
        assert!(float64_near(Cap::full().area(), 4.0 * PI, 1e-14));
        assert_eq!(Cap::from_center_height(x_axis_pt(), 0.0).area(), 0.0);
        // Hemisphere area = 2π
        assert!(float64_near(
            Cap::from_center_height(x_axis_pt(), 1.0).area(),
            2.0 * PI,
            1e-14,
        ));
    }

    #[test]
    fn test_display() {
        let c = Cap::from_center_angle(x_axis_pt(), Angle::from_degrees(45.0));
        let s = format!("{c}");
        assert!(s.contains("center="));
        assert!(s.contains("radius="));
    }

    #[test]
    fn test_intersects() {
        let empty = Cap::empty();
        let full = Cap::full();

        assert!(!empty.intersects(empty));
        assert!(!empty.intersects(full));
        assert!(!full.intersects(empty));
        assert!(full.intersects(full));

        // Two singleton caps at same point.
        assert!(x_axis().intersects(x_axis()));
        // Two singleton caps at different points.
        assert!(!x_axis().intersects(y_axis()));

        // Overlapping caps.
        let cap1 = Cap::from_center_angle(x_axis_pt(), Angle::from_degrees(45.0));
        let cap2 =
            Cap::from_center_angle(Point::from_coords(1.0, 1.0, 0.0), Angle::from_degrees(10.0));
        assert!(cap1.intersects(cap2));
    }

    #[test]
    fn test_complement() {
        // Full complement is empty.
        assert!(Cap::full().complement().is_empty());
        // Empty complement is full.
        assert!(Cap::empty().complement().is_full());

        // Complement of singleton is full (same as complement of empty).
        assert!(x_axis().complement().is_full());

        // Double complement should be approximately the same for non-trivial caps.
        let c = Cap::from_center_angle(x_axis_pt(), Angle::from_degrees(45.0));
        assert!(c.complement().complement().approx_eq(c));
    }

    #[test]
    fn test_from_center_height() {
        // Negative height → empty.
        assert!(Cap::from_center_height(x_axis_pt(), -1.0).is_empty());
        // Height of 0 → point cap.
        assert_eq!(Cap::from_center_height(x_axis_pt(), 0.0).height(), 0.0);
        // Height of 1 → hemisphere.
        assert!(float64_eq(
            Cap::from_center_height(x_axis_pt(), 1.0).height(),
            1.0
        ));
        // Height of 2 → full cap.
        assert!(Cap::from_center_height(x_axis_pt(), 2.0).is_full());
        // Height > 2 → full cap (clamped by from_length2).
        assert!(Cap::from_center_height(x_axis_pt(), 3.0).is_full());
    }

    // --- Tests ported from C++ s2cap_test.cc ---

    #[test]
    fn test_from_center_angle_edge_cases() {
        // From C++: S2Cap::Basic - constructor with out-of-range S1Angle arguments.
        // Negative angle → empty cap.
        let neg = Cap::from_center_angle(x_axis_pt(), Angle::from_radians(-1.0));
        assert!(neg.is_empty());

        // Angle greater than π → full cap.
        let big = Cap::from_center_angle(x_axis_pt(), Angle::from_radians(5.0));
        assert!(big.is_full());

        // Infinite angle → full cap.
        let inf = Cap::from_center_angle(x_axis_pt(), Angle::INFINITY);
        assert!(inf.is_full());

        // Zero angle → singleton (point) cap.
        let zero = Cap::from_center_angle(x_axis_pt(), Angle::ZERO);
        assert!(!zero.is_empty());
        assert!(!zero.is_full());
        assert_eq!(zero.height(), 0.0);
    }

    #[test]
    fn test_concave_cap() {
        // From C++: S2Cap::Basic - concave cap (>90 degree radius).
        // A cap with 150° radius centered at (0,0,1).
        let center = Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
        let concave_radius = ChordAngle::from_angle(Angle::from_degrees(150.0));
        let concave = Cap::from_center_chord_angle(center, concave_radius);

        assert!(concave.is_valid());
        assert!(!concave.is_empty());
        assert!(!concave.is_full());

        // A 150° cap should contain a hemisphere (90° cap with same center).
        let hemi_same = Cap::from_center_height(center, 1.0);
        assert!(concave.contains(hemi_same));

        // A 150° cap should not contain the opposite hemisphere.
        assert!(!concave.contains_point(-center));

        // Check that a 150° cap from z-axis contains points at 140° from center
        // (i.e. at latitude -50°) but not at 160° from center (latitude -70°).
        // cos(140°) ≈ -0.766, cos(160°) ≈ -0.940
        let p140 = Point::from_coords(
            0.0,
            (140.0_f64).to_radians().sin(),
            (140.0_f64).to_radians().cos(),
        );
        let p160 = Point::from_coords(
            0.0,
            (160.0_f64).to_radians().sin(),
            (160.0_f64).to_radians().cos(),
        );
        assert!(concave.contains_point(p140));
        assert!(!concave.contains_point(p160));

        // Error-bounded containment: create min/max caps with error margins.
        let max_cap_error = concave_radius.max_point_error()
            + concave_radius.max_angle_error()
            + 3.0 * f64::EPSILON;
        let concave_max =
            Cap::from_center_chord_angle(center, concave_radius.plus_error(max_cap_error));
        let concave_min =
            Cap::from_center_chord_angle(center, concave_radius.plus_error(-max_cap_error));
        // A point just barely inside 150° should be in concave_max but not concave_min.
        let border_inside = Point::from_coords(
            0.0,
            (149.99_f64).to_radians().sin(),
            (149.99_f64).to_radians().cos(),
        );
        let border_outside = Point::from_coords(
            0.0,
            (150.01_f64).to_radians().sin(),
            (150.01_f64).to_radians().cos(),
        );
        assert!(concave_max.contains_point(border_inside));
        assert!(!concave_min.contains_point(border_outside));
    }

    #[test]
    fn test_add_cap_area_preservation() {
        // From C++: AddEmptyCapToNonEmptyCap, AddNonEmptyCapToEmptyCap.
        // Adding empty cap to non-empty preserves area.
        let non_empty = Cap::from_center_angle(x_axis_pt(), Angle::from_degrees(10.0));
        let before_area = non_empty.area();
        let after = non_empty.add_cap(Cap::empty());
        assert!(float64_near(after.area(), before_area, 1e-15));

        // Adding non-empty cap to empty takes on the non-empty cap's area.
        let result = Cap::empty().add_cap(non_empty);
        assert!(float64_near(result.area(), before_area, 1e-15));
    }

    #[test]
    fn test_from_center_area() {
        assert!(float64_near(
            Cap::from_center_area(x_axis_pt(), 0.0).area(),
            0.0,
            1e-15,
        ));
        assert!(float64_near(
            Cap::from_center_area(x_axis_pt(), 2.0 * PI).area(),
            2.0 * PI,
            1e-14,
        ));
        assert!(Cap::from_center_area(x_axis_pt(), 4.0 * PI).is_full());
        assert!(Cap::from_center_area(x_axis_pt(), 5.0 * PI).is_full());
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;
    use std::f64::consts::PI;

    fn clamp_finite(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(-1e10, 1e10)
        } else {
            0.0
        }
    }

    fn make_point(x: f64, y: f64, z: f64) -> Option<Point> {
        let x = clamp_finite(x);
        let y = clamp_finite(y);
        let z = clamp_finite(z);
        if x == 0.0 && y == 0.0 && z == 0.0 {
            return None;
        }
        Some(Point::from_coords(x, y, z))
    }

    fn make_cap(x: f64, y: f64, z: f64, radius_deg: f64) -> Option<Cap> {
        let p = make_point(x, y, z)?;
        let r = clamp_finite(radius_deg).clamp(0.0, 180.0);
        Some(Cap::from_center_angle(p, Angle::from_degrees(r)))
    }

    #[quickcheck]
    fn prop_from_point_contains(x: f64, y: f64, z: f64) -> bool {
        match make_point(x, y, z) {
            Some(p) => Cap::from_point(p).contains_point(p),
            None => true,
        }
    }

    #[quickcheck]
    fn prop_complement_complement(x: f64, y: f64, z: f64, r: f64) -> bool {
        match make_cap(x, y, z, r) {
            Some(c) => c.complement().complement().approx_eq(c),
            None => true,
        }
    }

    #[quickcheck]
    fn prop_expanded_contains(x: f64, y: f64, z: f64, r: f64, expand: f64) -> bool {
        match make_cap(x, y, z, r) {
            Some(c) => {
                let expand = clamp_finite(expand).clamp(0.0, 180.0);
                c.expanded(Angle::from_degrees(expand)).contains(c)
            }
            None => true,
        }
    }

    #[quickcheck]
    fn prop_empty_is_empty() -> bool {
        Cap::empty().is_empty()
    }

    #[quickcheck]
    fn prop_full_is_full() -> bool {
        Cap::full().is_full()
    }

    #[quickcheck]
    fn prop_area_non_negative(x: f64, y: f64, z: f64, r: f64) -> bool {
        match make_cap(x, y, z, r) {
            Some(c) => c.area() >= 0.0,
            None => true,
        }
    }

    #[quickcheck]
    fn prop_full_area_is_4pi() -> bool {
        (Cap::full().area() - 4.0 * PI).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_contains_self(x: f64, y: f64, z: f64, r: f64) -> bool {
        match make_cap(x, y, z, r) {
            Some(c) => c.contains(c),
            None => true,
        }
    }

    #[quickcheck]
    fn prop_contains_center(x: f64, y: f64, z: f64, r: f64) -> bool {
        match make_cap(x, y, z, r) {
            Some(c) => c.contains_point(c.center()),
            None => true,
        }
    }

    #[quickcheck]
    fn prop_union_contains_both(
        x1: f64,
        y1: f64,
        z1: f64,
        r1: f64,
        x2: f64,
        y2: f64,
        z2: f64,
        r2: f64,
    ) -> bool {
        match (make_cap(x1, y1, z1, r1), make_cap(x2, y2, z2, r2)) {
            (Some(a), Some(b)) => {
                // Allow a tiny tolerance for floating-point errors in the
                // center interpolation (C++ Union also does not guarantee
                // exact containment).
                let eps = Angle::from_radians(1e-15);
                let u = a.union(b).expanded(eps);
                u.contains(a) && u.contains(b)
            }
            _ => true,
        }
    }

    #[quickcheck]
    fn prop_add_point_contains(x: f64, y: f64, z: f64, r: f64, px: f64, py: f64, pz: f64) -> bool {
        match (make_cap(x, y, z, r), make_point(px, py, pz)) {
            (Some(c), Some(p)) => c.add_point(p).contains_point(p),
            _ => true,
        }
    }

    #[quickcheck]
    fn prop_intersects_self(x: f64, y: f64, z: f64, r: f64) -> bool {
        match make_cap(x, y, z, r) {
            Some(c) => c.intersects(c),
            None => true,
        }
    }

    #[quickcheck]
    fn prop_height_in_range(x: f64, y: f64, z: f64, r: f64) -> bool {
        match make_cap(x, y, z, r) {
            Some(c) => c.height() >= 0.0 && c.height() <= 2.0,
            None => true,
        }
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(x: i32, y: i32, z: i32, r: u32) -> bool {
        if x == 0 && y == 0 && z == 0 {
            return true;
        }
        let center = Point::from_coords(f64::from(x), f64::from(y), f64::from(z));
        let angle = Angle::from_degrees(f64::from(r % 180));
        let c = Cap::from_center_angle(center, angle);
        let json1 = serde_json::to_string(&c).unwrap();
        let back: Cap = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        let back2: Cap = serde_json::from_str(&json2).unwrap();
        back == back2
    }

    // ===== Comprehensive cap tests (ported from C++ s2cap_test.cc) =====

    fn latlng_point(lat_degrees: f64, lng_degrees: f64) -> Point {
        LatLng::from_degrees(lat_degrees, lng_degrees).to_point()
    }

    #[test]
    fn test_cap_basic_comprehensive() {
        use std::f64::consts::FRAC_PI_4;

        let empty = Cap::empty();
        let full = Cap::full();

        // Empty and full cap properties.
        assert!(empty.is_valid());
        assert!(empty.is_empty());
        assert!(empty.complement().is_full());
        assert!(full.is_valid());
        assert!(full.is_full());
        assert!(full.complement().is_empty());
        assert!((full.height() - 2.0).abs() < 1e-15);
        assert!((full.angle_radius().degrees() - 180.0).abs() < 1e-13);

        // Equality.
        assert_eq!(full, full);
        assert_eq!(empty, empty);
        assert_ne!(full, empty);

        // Out-of-range angle constructors.
        assert!(
            Cap::from_center_angle(
                Point::from_coords(1.0, 0.0, 0.0),
                Angle::from_radians(-20.0)
            )
            .is_empty()
        );
        assert!(
            Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_radians(5.0))
                .is_full()
        );

        // Containment of empty and full caps.
        assert!(empty.contains(empty));
        assert!(full.contains(empty));
        assert!(full.contains(full));
        assert!(!empty.interior_intersects(empty));
        assert!(full.interior_intersects(full));
        assert!(!full.interior_intersects(empty));

        // Singleton cap (x-axis).
        let xaxis = Cap::from_point(Point::from_coords(1.0, 0.0, 0.0));
        assert!(xaxis.contains_point(Point::from_coords(1.0, 0.0, 0.0)));
        assert!(!xaxis.contains_point(Point::from_coords(1.0, 1e-20, 0.0)));
        assert_eq!(xaxis.angle_radius().radians(), 0.0);
        assert_eq!(xaxis.height(), 0.0);

        // Singleton cap (y-axis).
        let yaxis = Cap::from_point(Point::from_coords(0.0, 1.0, 0.0));
        assert!(!yaxis.contains_point(xaxis.center));

        // Complement of singleton is full.
        let xcomp = xaxis.complement();
        assert!(xcomp.is_valid());
        assert!(xcomp.is_full());
        assert!(xcomp.contains_point(xaxis.center));

        // Complement of complement is empty.
        assert!(xcomp.complement().is_valid());
        assert!(xcomp.complement().is_empty());
        assert!(!xcomp.complement().contains_point(xaxis.center));

        // Tiny cap.
        let tiny_rad = 1e-10_f64;
        let tiny = Cap::from_center_angle(
            Point::from_coords(1.0, 2.0, 3.0),
            Angle::from_radians(tiny_rad),
        );
        let tangent = Point(
            tiny.center
                .0
                .cross(Point::from_coords(3.0, 2.0, 1.0).0)
                .normalize(),
        );
        assert!(tiny.contains_point(Point(
            (tiny.center.0 + tangent.0 * (0.99 * tiny_rad)).normalize()
        )));
        assert!(!tiny.contains_point(Point(
            (tiny.center.0 + tangent.0 * (1.01 * tiny_rad)).normalize()
        )));

        // Hemispherical cap.
        let hemi = Cap::from_center_height(Point::from_coords(1.0, 0.0, 1.0), 1.0);
        assert_eq!(Point((-hemi.center).0), hemi.complement().center);
        assert!((hemi.complement().height() - 1.0).abs() < 1e-15);
        assert!(hemi.contains_point(Point::from_coords(1.0, 0.0, 0.0)));
        assert!(
            !hemi
                .complement()
                .contains_point(Point::from_coords(1.0, 0.0, 0.0))
        );

        // Cap containment tests.
        assert!(!empty.contains(xaxis));
        assert!(!empty.interior_intersects(xaxis));
        assert!(full.contains(xaxis));
        assert!(full.interior_intersects(xaxis));
        assert!(!xaxis.contains(full));
        assert!(!xaxis.interior_intersects(full));
        assert!(xaxis.contains(xaxis));
        assert!(!xaxis.interior_intersects(xaxis));
        assert!(xaxis.contains(empty));
        assert!(!xaxis.interior_intersects(empty));
        assert!(hemi.contains(tiny));
        let k_eps = 1e-15_f64;
        assert!(hemi.contains(Cap::from_center_angle(
            Point::from_coords(1.0, 0.0, 0.0),
            Angle::from_radians(FRAC_PI_4 - k_eps)
        )));
        assert!(!hemi.contains(Cap::from_center_angle(
            Point::from_coords(1.0, 0.0, 0.0),
            Angle::from_radians(FRAC_PI_4 + k_eps)
        )));
    }

    #[test]
    fn test_cap_get_rect_bound() {
        use std::f64::consts::FRAC_PI_4;
        let degree_eps = 1e-13;
        let eps = 1e-15;

        // Empty and full caps.
        assert!(Cap::empty().rect_bound().is_empty());
        assert!(Cap::full().rect_bound().is_full());

        // Cap that includes the south pole.
        let cap = Cap::from_center_angle(latlng_point(-45.0, 57.0), Angle::from_degrees(50.0));
        let rect = cap.rect_bound();
        assert!((rect.lat.lo.to_degrees() - (-90.0)).abs() < degree_eps);
        assert!((rect.lat.hi.to_degrees() - 5.0).abs() < degree_eps);
        assert!(rect.lng.is_full());

        // Cap tangent to the north pole.
        let cap = Cap::from_center_angle(
            Point::from_coords(1.0, 0.0, 1.0),
            Angle::from_radians(FRAC_PI_4 + 1e-16),
        );
        let rect = cap.rect_bound();
        assert!(rect.lat.lo.abs() < eps);
        assert!((rect.lat.hi - FRAC_PI_2).abs() < eps);
        assert!(rect.lng.is_full());

        // Cap centered on the equator.
        let cap = Cap::from_center_angle(latlng_point(0.0, 50.0), Angle::from_degrees(20.0));
        let rect = cap.rect_bound();
        assert!((rect.lat.lo.to_degrees() - (-20.0)).abs() < degree_eps);
        assert!((rect.lat.hi.to_degrees() - 20.0).abs() < degree_eps);
        assert!((rect.lng.lo.to_degrees() - 30.0).abs() < degree_eps);
        assert!((rect.lng.hi.to_degrees() - 70.0).abs() < degree_eps);

        // Cap centered on the north pole.
        let cap = Cap::from_center_angle(latlng_point(90.0, 123.0), Angle::from_degrees(10.0));
        let rect = cap.rect_bound();
        assert!((rect.lat.lo.to_degrees() - 80.0).abs() < degree_eps);
        assert!((rect.lat.hi.to_degrees() - 90.0).abs() < degree_eps);
        assert!(rect.lng.is_full());
    }

    #[test]
    fn test_cell_union_bound_level1_radius() {
        // A cap whose radius ≈ the width of a level-1 cell can be covered by 3 faces.
        use crate::s2::metric::MIN_WIDTH;
        let cap = Cap::from_center_angle(
            Point::from_coords(1.0, 1.0, 1.0).normalize(),
            Angle::from_radians(MIN_WIDTH.value(1)),
        );
        let covering = cap.cell_union_bound();
        assert_eq!(
            covering.len(),
            3,
            "expected 3 cells, got {}",
            covering.len()
        );
    }

    #[test]
    fn test_cap_contains_full_range() {
        // Full cap contains everything, empty cap contains nothing.
        let full = Cap::full();
        let empty = Cap::empty();
        let small =
            Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(10.0));
        assert!(full.contains(small));
        assert!(!empty.contains(small));
        assert!(!small.contains(full));
    }

    #[test]
    fn test_cap_intersects_range() {
        let cap1 =
            Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(30.0));
        // Nearby cap: should intersect.
        let cap2 = Cap::from_center_angle(
            Point::from_coords(1.0, 0.0, 0.1).normalize(),
            Angle::from_degrees(30.0),
        );
        assert!(cap1.intersects(cap2));

        // Opposite side of sphere: should not intersect.
        let cap3 = Cap::from_center_angle(
            Point::from_coords(-1.0, 0.0, 0.0),
            Angle::from_degrees(30.0),
        );
        assert!(!cap1.intersects(cap3));

        // Full cap intersects everything except empty.
        assert!(Cap::full().intersects(cap1));
        assert!(!Cap::empty().intersects(cap1));
    }

    #[test]
    fn test_add_empty_cap_to_non_empty() {
        let mut cap =
            Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(10.0));
        let initial_area = cap.area();
        cap = cap.add_cap(Cap::empty());
        assert!((cap.area() - initial_area).abs() < 1e-15);
    }

    #[test]
    fn test_add_non_empty_cap_to_empty() {
        let non_empty =
            Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(10.0));
        let result = Cap::empty().add_cap(non_empty);
        assert!((result.area() - non_empty.area()).abs() < 1e-15);
    }

    // --- Encode/Decode ---

    #[test]
    fn test_cap_encode_decode_roundtrip() {
        use crate::s2::encoding::{S2Decode, S2Encode};
        let cap = Cap::from_center_height(Point::from_coords(3.0, 2.0, 1.0).normalize(), 1.0);
        let mut buf = Vec::new();
        cap.encode(&mut buf).expect("encode cap");
        let decoded = Cap::decode(&mut buf.as_slice()).expect("decode cap");
        assert_eq!(cap.center(), decoded.center());
        assert_eq!(cap.chord_radius(), decoded.chord_radius());
    }

    // --- GetCentroid deterministic ---

    #[test]
    fn test_centroid_empty_and_full() {
        let empty_c = Cap::empty().centroid();
        assert!(
            empty_c.0.norm() < 1e-15,
            "empty cap centroid should be zero, got {empty_c:?}",
        );

        let full_c = Cap::full().centroid();
        assert!(
            full_c.0.norm() < 1e-15,
            "full cap centroid should be near zero, got {full_c:?}",
        );
    }

    #[test]
    fn test_centroid_formula() {
        // For a cap with center and height, centroid = center * (1 - height/2) * area.
        for i in 0..20 {
            let angle = 0.7 * f64::from(i);
            let center = Point::from_coords(angle.cos(), angle.sin(), (0.3 * f64::from(i)).cos())
                .normalize();
            let height = (f64::from(i) * 0.1).min(2.0);
            let cap = Cap::from_center_height(center, height);
            let centroid = cap.centroid();
            let expected_norm = (1.0 - height / 2.0) * cap.area();
            let expected = Point(center.0 * expected_norm);
            let diff = (expected.0 - centroid.0).norm();
            assert!(diff < 1e-14, "centroid mismatch at i={i}: diff={diff}");
        }
    }
}

#[cfg(test)]
#[path = "cap_tests.rs"]
mod cap_tests;
