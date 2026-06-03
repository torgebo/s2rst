// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// Shared supertrait for all distance query targets.
//
// Corresponds to the common interface of C++ `S2DistanceTarget<Distance>`.
// Each query module (`closest_edge_query`, `closest_cell_query`,
// `closest_point_query`, `furthest_edge_query`) defines its own `Target`
// trait that extends `DistanceTarget` with geometry-specific methods.

use crate::s1::ChordAngle;
use crate::s2::Cap;

/// Common interface shared by all distance query targets.
///
/// This trait captures the methods that every distance target must provide,
/// regardless of the specific query type (closest edge, closest cell,
/// closest point, furthest edge).
///
/// # Examples
///
/// ```
/// use s2rst::s2::distance_target::DistanceTarget;
/// use s2rst::s2::closest_edge_query::PointTarget;
/// use s2rst::s2::LatLng;
///
/// let target = PointTarget::new(LatLng::from_degrees(0.0, 0.0).to_point());
/// let cap = target.cap_bound();
/// assert!(!cap.is_empty());
/// ```
pub trait DistanceTarget {
    /// Returns a bounding cap for the target geometry.
    ///
    /// The cap must contain all points whose distance to the target is
    /// zero. For example, for a point target the cap is a point cap;
    /// for an edge target it's a cap bounding both endpoints.
    fn cap_bound(&self) -> Cap;

    /// Specifies that distances may be up to `max_error` larger than the
    /// true optimum. Returns `true` if this target takes advantage of the
    /// error allowance (e.g., by propagating it to an internal query).
    ///
    /// The default returns `false`, meaning the target always computes
    /// exact distances.
    fn set_max_error(&mut self, _max_error: ChordAngle) -> bool {
        false
    }
}
