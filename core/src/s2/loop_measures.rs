// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Angle and area measures for loops on the unit sphere.
//!
//! These are low-level methods that work directly with slices of [`Point`]s.
//! They are used to implement methods in [`shape_measures`](super::shape_measures),
//! [`Loop`](super::Loop), and [`Polygon`](super::Polygon).
//!
//! Corresponds to C++ `s2loop_measures.h/cc`.

#![expect(
    clippy::cast_sign_loss,
    reason = "vertex index (i32) used as Vec indices"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "vertex index (i32) <-> usize for loop vertex access"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for vertex index — always in range"
)]
use std::f64::consts::PI;

use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::centroids;
use crate::s2::point_measures;

// ---------------------------------------------------------------------------
// KahanSum: compensated summation using Kahan's algorithm.
// ---------------------------------------------------------------------------

/// Compensated sum using Kahan's algorithm.
///
/// Provides much better accuracy than naive summation for long sequences
/// of additions, with approximately constant error of 2*epsilon regardless
/// of sequence length (for well-conditioned sums).
#[derive(Clone, Debug, PartialEq)]
pub struct KahanSum {
    sum: f64,
    err: f64,
}

impl KahanSum {
    /// Creates a new `KahanSum` initialized to zero.
    pub fn new() -> Self {
        KahanSum { sum: 0.0, err: 0.0 }
    }

    /// Adds a value to the running total with compensated summation.
    pub fn add(&mut self, value: f64) {
        let tmp1 = value - self.err;
        let tmp2 = self.sum + tmp1;
        self.err = (tmp2 - self.sum) - tmp1;
        self.sum = tmp2;
    }

    /// Returns the current sum.
    pub fn value(&self) -> f64 {
        self.sum
    }
}

impl Default for KahanSum {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Surface integral: triangle-fan decomposition with origin relocation.
// ---------------------------------------------------------------------------

/// The maximum length of an edge for it to be considered numerically stable
/// in the surface integral. Fairly conservative.
const MAX_LENGTH: f64 = PI - 1e-5;

/// Returns the oriented surface integral of a function `f_tri` over the loop
/// interior. `f_tri(A, B, C)` should return the integral of some quantity
/// over the spherical triangle ABC (positive if CCW, negative if CW).
///
/// The result is the integral of the function over the loop interior plus or
/// minus some multiple of the integral over the entire sphere.
pub fn get_surface_integral(
    loop_vertices: &[Point],
    f_tri: impl Fn(Point, Point, Point) -> f64,
) -> f64 {
    let mut sum = 0.0_f64;
    get_surface_integral_acc(loop_vertices, &f_tri, &mut sum);
    sum
}

/// Like [`get_surface_integral`] but uses Kahan summation for better accuracy
/// on long vertex sequences.
pub fn get_surface_integral_kahan(
    loop_vertices: &[Point],
    f_tri: impl Fn(Point, Point, Point) -> f64,
) -> f64 {
    let mut sum = KahanSum::new();
    get_surface_integral_acc(loop_vertices, &f_tri, &mut sum);
    sum.value()
}

/// Trait for accumulating values (either f64 or `KahanSum`).
trait Accumulator {
    fn acc_add(&mut self, value: f64);
}

impl Accumulator for f64 {
    fn acc_add(&mut self, value: f64) {
        *self += value;
    }
}

impl Accumulator for KahanSum {
    fn acc_add(&mut self, value: f64) {
        self.add(value);
    }
}

/// Core surface integral implementation with a generic accumulator.
fn get_surface_integral_acc(
    loop_vertices: &[Point],
    f_tri: &dyn Fn(Point, Point, Point) -> f64,
    sum: &mut dyn Accumulator,
) {
    let n = loop_vertices.len();
    if n < 3 {
        return;
    }

    let mut origin = loop_vertices[0];
    for i in 1..n - 1 {
        // Invariants:
        //  1. length(origin, loop[i]) < MAX_LENGTH for all (i > 1).
        //  2. Either origin == loop[0], or origin is approx perpendicular to loop[0].
        //  3. "sum" is the oriented integral over (origin, loop[0], ..., loop[i]).
        debug_assert!(i == 1 || origin.0.angle(loop_vertices[i].0) < MAX_LENGTH);
        debug_assert!(origin == loop_vertices[0] || origin.0.dot(loop_vertices[0].0).abs() < 1e-15);

        if loop_vertices[i + 1].0.angle(origin.0) > MAX_LENGTH {
            // We are about to create an unstable edge — choose a new origin.
            let old_origin = origin;
            if origin == loop_vertices[0] {
                // O' is well-separated from V_i and V_0 (and V_i+1).
                origin =
                    super::edge_crossings::robust_cross_prod(loop_vertices[0], loop_vertices[i])
                        .normalize();
            } else if loop_vertices[i].0.angle(loop_vertices[0].0) < MAX_LENGTH {
                // All edges of triangle (O, V_0, V_i) are stable; revert to V_0.
                origin = loop_vertices[0];
            } else {
                // (O, V_i+1) and (V_0, V_i) are antipodal pairs, and O ⊥ V_0.
                // Choose O' = V_0 × O, approximately perpendicular to all of them.
                origin = Point(loop_vertices[0].0.cross(old_origin.0));
                // Advance edge (V_0, O) to (V_0, O').
                sum.acc_add(f_tri(loop_vertices[0], old_origin, origin));
            }
            // Advance edge (O, V_i) to (O', V_i).
            sum.acc_add(f_tri(old_origin, loop_vertices[i], origin));
        }
        // Advance edge (O, V_i) to (O, V_i+1).
        sum.acc_add(f_tri(origin, loop_vertices[i], loop_vertices[i + 1]));
    }
    // If origin is not V_0, close the fan.
    if origin != loop_vertices[0] {
        sum.acc_add(f_tri(origin, loop_vertices[n - 1], loop_vertices[0]));
    }
}

// ---------------------------------------------------------------------------
// LoopOrder: canonical ordering for loop traversal.
// ---------------------------------------------------------------------------

/// Represents a cyclic ordering of loop vertices, starting at index `first`
/// and proceeding in direction `dir` (+1 or -1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoopOrder {
    /// The index of the starting vertex.
    pub first: i32,
    /// The direction of traversal (+1 or -1).
    pub dir: i32,
}

impl LoopOrder {
    /// Creates a new `LoopOrder` with the given starting vertex and direction.
    pub fn new(first: i32, dir: i32) -> Self {
        LoopOrder { first, dir }
    }
}

/// Compares points lexicographically by (x, y, z) coordinates.
fn point_less(a: Point, b: Point) -> bool {
    if a.0.x != b.0.x {
        return a.0.x < b.0.x;
    }
    if a.0.y != b.0.y {
        return a.0.y < b.0.y;
    }
    a.0.z < b.0.z
}

fn point_le(a: Point, b: Point) -> bool {
    a == b || point_less(a, b)
}

/// Helper: index into a loop with wraparound (index range [0, 2*n)).
fn loop_at(loop_vertices: &[Point], i: i32) -> Point {
    let n = loop_vertices.len() as i32;
    let j = i - n;
    if j < 0 {
        loop_vertices[i as usize]
    } else {
        loop_vertices[j as usize]
    }
}

fn is_order_less(order1: LoopOrder, order2: LoopOrder, loop_vertices: &[Point]) -> bool {
    if order1 == order2 {
        return false;
    }
    let mut i1 = order1.first;
    let mut i2 = order2.first;
    let n = loop_vertices.len() as i32;
    for _ in 1..n {
        i1 += order1.dir;
        i2 += order2.dir;
        let p1 = loop_at(loop_vertices, i1);
        let p2 = loop_at(loop_vertices, i2);
        if point_less(p1, p2) {
            return true;
        }
        if point_less(p2, p1) {
            return false;
        }
    }
    false
}

/// Returns a [`LoopOrder`] such that the vertex sequence does not change when
/// the loop vertex order is rotated or reversed.
///
/// This allows loop vertices to be traversed in a canonical order.
pub fn get_canonical_loop_order(loop_vertices: &[Point]) -> LoopOrder {
    let n = loop_vertices.len() as i32;
    if n == 0 {
        return LoopOrder::new(0, 1);
    }

    // Find all indices where the minimum vertex occurs.
    let mut min_indices = vec![0i32];
    for i in 1..n {
        if point_le(
            loop_vertices[i as usize],
            loop_vertices[min_indices[0] as usize],
        ) {
            if point_less(
                loop_vertices[i as usize],
                loop_vertices[min_indices[0] as usize],
            ) {
                min_indices.clear();
            }
            min_indices.push(i);
        }
    }

    let mut min_order = LoopOrder::new(min_indices[0], 1);
    for &min_index in &min_indices {
        let order1 = LoopOrder::new(min_index, 1);
        let order2 = LoopOrder::new(min_index + n, -1);
        if is_order_less(order1, min_order, loop_vertices) {
            min_order = order1;
        }
        if is_order_less(order2, min_order, loop_vertices) {
            min_order = order2;
        }
    }
    min_order
}

// ---------------------------------------------------------------------------
// PruneDegeneracies: remove AA and ABA subsequences.
// ---------------------------------------------------------------------------

/// Returns a new loop obtained by removing all degeneracies that can be
/// detected by comparing adjacent vertices and edges for equality.
///
/// Specifically, repeatedly finds vertex subsequences of the form AA or ABA,
/// and collapses them to A. A loop of length 1 or 2 becomes empty.
///
/// The resulting pruned loop is uniquely determined (up to cyclic permutation),
/// regardless of the order in which degeneracies are processed.
pub fn prune_degeneracies(loop_vertices: &[Point]) -> Vec<Point> {
    let mut vertices: Vec<Point> = Vec::with_capacity(loop_vertices.len());

    // Move vertices, checking for degeneracies as we go.
    // Invariant: the partially-constructed `vertices` contains no AAs or ABAs.
    for &v in loop_vertices {
        if !vertices.is_empty() {
            if v == vertices[vertices.len() - 1] {
                // De-dup: AA -> A.
                continue;
            }
            if vertices.len() >= 2 && v == vertices[vertices.len() - 2] {
                // Remove whisker: ABA -> A.
                vertices.pop();
                continue;
            }
        }
        vertices.push(v);
    }

    // Remove AA that wraps from end to beginning.
    if vertices.len() >= 2 && vertices[0] == vertices[vertices.len() - 1] {
        vertices.pop();
    }

    if vertices.len() < 3 {
        return Vec::new();
    }

    // Remove ABA patterns that span the wrap-around boundary.
    let mut k = 0usize;
    while vertices[k + 1] == vertices[vertices.len() - 1 - k]
        || vertices[k] == vertices[vertices.len() - 2 - k]
    {
        k += 1;
    }
    if k > 0 {
        vertices.drain(0..k);
        vertices.truncate(vertices.len() - k);
    }
    vertices
}

// ---------------------------------------------------------------------------
// Loop measure functions.
// ---------------------------------------------------------------------------

/// Returns the perimeter of the loop.
pub fn get_perimeter(loop_vertices: &[Point]) -> Angle {
    let n = loop_vertices.len();
    if n <= 1 {
        return Angle::from_radians(0.0);
    }
    let mut perimeter = 0.0;
    for i in 0..n {
        let next = if i + 1 < n { i + 1 } else { 0 };
        perimeter += loop_vertices[i].distance(loop_vertices[next]).radians();
    }
    Angle::from_radians(perimeter)
}

/// Returns the area of the loop interior (region on the left side of the loop).
///
/// The result is between 0 and 4*PI. Nearly-degenerate CW loops have areas
/// close to zero; nearly-degenerate CCW loops have areas close to 4*PI.
pub fn get_area(loop_vertices: &[Point]) -> f64 {
    let mut area = get_signed_area(loop_vertices);
    debug_assert!(area.abs() <= 2.0 * PI + 1e-10);
    if area < 0.0 {
        area += 4.0 * PI;
    }
    area
}

/// Returns the signed area of the loop: positive for CCW, negative for CW.
///
/// The result is between -2*PI and 2*PI. The full loop (empty vertex list)
/// returns `-f64::MIN_POSITIVE` (the "signed equivalent" of 4*PI).
///
/// Degenerate loops (composed of sibling edge pairs) have area exactly zero.
pub fn get_signed_area(loop_vertices: &[Point]) -> f64 {
    // Empty loop = full loop.
    if loop_vertices.is_empty() {
        return -f64::MIN_POSITIVE;
    }

    // Use the robust "signed sum over triangles" approach via surface integral
    // with Kahan summation, then cross-check with curvature near zero.
    let area = get_surface_integral_kahan(loop_vertices, point_measures::signed_area);
    let max_error = get_curvature_max_error(loop_vertices);

    // Normalize to (-2*PI, 2*PI]. Hemispheres always get positive area.
    // C++ uses std::remainder(area, 4*PI) = area - round(area/(4*PI)) * 4*PI.
    let mut area = area - (area / (4.0 * PI)).round() * (4.0 * PI);
    if area == -2.0 * PI {
        area = 2.0 * PI;
    }

    // If the area is near zero, cross-check with curvature.
    if area.abs() <= max_error {
        let curvature = get_curvature(loop_vertices);
        // Zero-area loops should have a curvature of approximately +/- 2*Pi.
        debug_assert!(
            !(area == 0.0 && curvature == 0.0),
            "zero area with zero curvature"
        );
        // Degenerate loop.
        if curvature == 2.0 * PI {
            return 0.0;
        }
        if area <= 0.0 && curvature > 0.0 {
            return f64::MIN_POSITIVE;
        }
        // Full loops.
        if area >= 0.0 && curvature < 0.0 {
            return -f64::MIN_POSITIVE;
        }
    }
    area
}

/// Returns the approximate area using the Gauss-Bonnet theorem.
///
/// Computed as `2*PI - curvature`. The result is between 0 and 4*PI.
/// The maximum error is about 2.22e-15 steradians per vertex.
pub fn get_approx_area(loop_vertices: &[Point]) -> f64 {
    2.0 * PI - get_curvature(loop_vertices)
}

/// Returns the geodesic curvature of the loop (sum of turning angles).
///
/// Positive for CCW loops, negative for CW loops, zero for great circles.
/// Equal to `2*PI - area`.
///
/// Special cases:
///  - Empty vertex list (full loop): returns -2*PI.
///  - Degenerate loops: returns 2*PI.
///  - All other loops: returns a value in (-2*PI, 2*PI).
pub fn get_curvature(loop_vertices: &[Point]) -> f64 {
    // By convention, a loop with no vertices contains all points.
    if loop_vertices.is_empty() {
        return -2.0 * PI;
    }

    // Remove degeneracies.
    let pruned = prune_degeneracies(loop_vertices);
    if pruned.is_empty() {
        return 2.0 * PI;
    }

    let n = pruned.len() as i32;
    let order = get_canonical_loop_order(&pruned);
    let dir = order.dir;

    // Helper: get vertex at canonical position j (j-th vertex in traversal order).
    // The mapping uses S2PointLoopSpan-style wrapping: indices in [0, 2*n).
    let v = |raw_index: i32| -> Point {
        let idx = ((raw_index % n) + n) % n;
        pruned[idx as usize]
    };

    // First turn angle: vertices at positions (first - dir, first, first + dir).
    let mut i = order.first;
    let mut sum = point_measures::turn_angle(v(i - dir), v(i), v(i + dir)).radians();

    // Kahan summation for the remaining turn angles.
    let mut compensation = 0.0_f64;
    for _ in 1..n {
        i += dir;
        let angle = point_measures::turn_angle(v(i - dir), v(i), v(i + dir)).radians();
        let old_sum = sum;
        let corrected = angle + compensation;
        sum += corrected;
        compensation = (old_sum - sum) + corrected;
    }
    sum += compensation;

    let k_max_curvature = 2.0 * PI - 4.0 * f64::EPSILON;
    (f64::from(dir) * sum).clamp(-k_max_curvature, k_max_curvature)
}

/// Returns the maximum error in [`get_curvature`] for the given loop.
///
/// This is also an upper bound on the error in [`get_area`], [`get_signed_area`],
/// and [`get_approx_area`].
pub fn get_curvature_max_error(loop_vertices: &[Point]) -> f64 {
    // 3.00 * DBL_EPSILON   for RobustCrossProd(b, a)
    // 3.00 * DBL_EPSILON   for RobustCrossProd(c, b)
    // 3.25 * DBL_EPSILON   for Angle()
    // 2.00 * DBL_EPSILON   for each addition in Kahan summation
    // ----- total: 11.25 * DBL_EPSILON per vertex
    let k_max_error_per_vertex = 11.25 * f64::EPSILON;
    k_max_error_per_vertex * loop_vertices.len() as f64
}

/// Returns the true centroid of the loop multiplied by the area of the loop.
///
/// The result is not unit length. The centroid may not be contained by the loop.
/// Scaling by the area makes it easy to compute centroids of composite regions.
pub fn get_centroid(loop_vertices: &[Point]) -> Point {
    // The integral of position over the entire sphere is (0,0,0),
    // so interior vs exterior doesn't matter.
    // We use the same triangle fan as get_surface_integral but accumulate a Point.
    let n = loop_vertices.len();
    if n < 3 {
        return Point(crate::r3::Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    let mut add_tri = |a: Point, b: Point, c: Point| {
        let p = centroids::true_centroid(a, b, c);
        cx += p.0.x;
        cy += p.0.y;
        cz += p.0.z;
    };

    let mut origin = loop_vertices[0];
    for i in 1..n - 1 {
        if loop_vertices[i + 1].0.angle(origin.0) > MAX_LENGTH {
            let old_origin = origin;
            if origin == loop_vertices[0] {
                origin =
                    super::edge_crossings::robust_cross_prod(loop_vertices[0], loop_vertices[i])
                        .normalize();
            } else if loop_vertices[i].0.angle(loop_vertices[0].0) < MAX_LENGTH {
                origin = loop_vertices[0];
            } else {
                origin = Point(loop_vertices[0].0.cross(old_origin.0));
                add_tri(loop_vertices[0], old_origin, origin);
            }
            add_tri(old_origin, loop_vertices[i], origin);
        }
        add_tri(origin, loop_vertices[i], loop_vertices[i + 1]);
    }
    if origin != loop_vertices[0] {
        add_tri(origin, loop_vertices[n - 1], loop_vertices[0]);
    }
    Point(crate::r3::Vector {
        x: cx,
        y: cy,
        z: cz,
    })
}

/// Returns true if the loop area is at most 2*PI (allowing some error for
/// hemispheres).
///
/// Degenerate loops are handled consistently with `predicates::sign`.
pub fn is_normalized(loop_vertices: &[Point]) -> bool {
    get_curvature(loop_vertices) >= -get_curvature_max_error(loop_vertices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::text_format;

    // Helper: create a "test loop" from a character string where each char
    // maps to a point (ch, 0, 0). These points are NOT unit-length but are
    // useful for testing prune_degeneracies and canonical loop order.
    // NOTE: We must create unnormalized points (like C++) so distinct chars
    // produce distinct points.
    fn make_test_loop(s: &str) -> Vec<Point> {
        s.bytes()
            .map(|ch| {
                Point(crate::r3::Vector {
                    x: f64::from(ch),
                    y: 0.0,
                    z: 0.0,
                })
            })
            .collect()
    }

    fn to_test_string(pts: &[Point]) -> String {
        pts.iter().map(|p| p.0.x as u8 as char).collect()
    }

    /// Returns the lexicographically smallest cyclic permutation of s.
    fn cyclic_canonicalize(s: &str) -> String {
        if s.is_empty() {
            return String::new();
        }
        let mut best = s.to_string();
        for i in 1..s.len() {
            let candidate = format!("{}{}", &s[i..], &s[..i]);
            if candidate < best {
                best = candidate;
            }
        }
        best
    }

    fn test_prune_degeneracies(input: &str, expected: &str) {
        let loop_pts = make_test_loop(input);
        let result = prune_degeneracies(&loop_pts);
        let result_str = to_test_string(&result);
        assert_eq!(
            cyclic_canonicalize(&result_str),
            cyclic_canonicalize(expected),
            "input = \"{input}\""
        );
    }

    #[test]
    fn test_prune_completely_degenerate() {
        test_prune_degeneracies("", "");
        test_prune_degeneracies("a", "");
        test_prune_degeneracies("aaaaa", "");
        test_prune_degeneracies("ab", "");
        test_prune_degeneracies("abb", "");
        test_prune_degeneracies("aab", "");
        test_prune_degeneracies("aba", "");
        test_prune_degeneracies("abba", "");
        test_prune_degeneracies("abcb", "");
        test_prune_degeneracies("abcba", "");
        test_prune_degeneracies("abcdcdedefedcbcdcb", "");
    }

    #[test]
    fn test_prune_partially_degenerate() {
        test_prune_degeneracies("abc", "abc");
        test_prune_degeneracies("abca", "abc");
        test_prune_degeneracies("abcc", "abc");
        test_prune_degeneracies("abccaa", "abc");
        test_prune_degeneracies("aabbcc", "abc");
        test_prune_degeneracies("abcdedca", "abc");
        test_prune_degeneracies("abcbabcbcdc", "abc");
        test_prune_degeneracies("xyzabcazy", "abc");
        test_prune_degeneracies("xxyyzzaabbccaazzyyxx", "abc");
        test_prune_degeneracies("abcdb", "bcd");
        test_prune_degeneracies("abcdecb", "cde");
        test_prune_degeneracies("abcdefdcb", "def");
        test_prune_degeneracies("abcad", "bca");
        test_prune_degeneracies("abcdbae", "cdb");
        test_prune_degeneracies("abcdecbaf", "dec");
    }

    fn test_canonical_order(input: &str, expected_first: i32, expected_dir: i32) {
        let loop_pts = make_test_loop(input);
        let order = get_canonical_loop_order(&loop_pts);
        assert_eq!(
            order,
            LoopOrder::new(expected_first, expected_dir),
            "input = \"{input}\""
        );
    }

    #[test]
    fn test_canonical_loop_order() {
        test_canonical_order("", 0, 1);
        test_canonical_order("a", 0, 1);
        test_canonical_order("aaaaa", 0, 1);
        test_canonical_order("ba", 1, 1);
        test_canonical_order("bab", 1, 1);
        test_canonical_order("cbab", 2, 1);
        test_canonical_order("bacbcab", 8, -1);
    }

    #[test]
    fn test_get_perimeter_empty() {
        assert_eq!(get_perimeter(&[]).radians(), 0.0);
    }

    #[test]
    fn test_get_perimeter_octant() {
        let loop_pts = text_format::parse_points("0:0, 0:90, 90:0");
        let perimeter = get_perimeter(&loop_pts);
        let expected = 3.0 * std::f64::consts::FRAC_PI_2;
        assert!(
            (perimeter.radians() - expected).abs() < 1e-14,
            "got {}, expected {}",
            perimeter.radians(),
            expected
        );
    }

    #[test]
    fn test_get_perimeter_more_than_two_pi() {
        // Make sure GetPerimeter doesn't use ChordAngle (max 2*PI).
        let loop_pts = text_format::parse_points("0:0, 0:90, 0:180, 90:0, 0:-90");
        let perimeter = get_perimeter(&loop_pts);
        let expected = 5.0 * std::f64::consts::FRAC_PI_2;
        assert!(
            (perimeter.radians() - expected).abs() < 1e-14,
            "got {}, expected {}",
            perimeter.radians(),
            expected
        );
    }

    #[test]
    fn test_get_signed_area_small_ccw() {
        // A small CCW square should have positive signed area.
        let loop_pts = text_format::parse_points("0:0, 0:1, 1:1, 1:0");
        let signed = get_signed_area(&loop_pts);
        assert!(signed > 0.0, "signed_area = {signed:e}");
    }

    #[test]
    fn test_get_signed_area_small_cw() {
        // A small CW square should have negative signed area.
        let loop_pts = text_format::parse_points("0:0, 1:0, 1:1, 0:1");
        let signed = get_signed_area(&loop_pts);
        assert!(signed < 0.0, "signed_area = {signed:e}");
    }

    fn test_area_consistent_with_curvature(loop_pts: &[Point]) {
        let area = get_area(loop_pts);
        let gauss_area = 2.0 * PI - get_curvature(loop_pts);
        assert!(
            (area - gauss_area).abs() < 1e-14,
            "Area = {area}, Gauss Area = {gauss_area}"
        );
    }

    #[test]
    fn test_area_consistent_with_curvature_standard_loops() {
        // Full loop.
        test_area_consistent_with_curvature(&[]);

        // Northern hemisphere.
        let north_hemi = text_format::parse_points("0:-180, 0:-90, 0:0, 0:90");
        test_area_consistent_with_curvature(&north_hemi);

        // Northern hemisphere (3 points).
        let north_hemi3 = text_format::parse_points("0:-180, 0:-60, 0:60");
        test_area_consistent_with_curvature(&north_hemi3);

        // Western hemisphere.
        let west_hemi = text_format::parse_points("0:-180, -90:0, 0:0, 90:0");
        test_area_consistent_with_curvature(&west_hemi);

        // Eastern hemisphere.
        let east_hemi = text_format::parse_points("90:0, 0:0, -90:0, 0:-180");
        test_area_consistent_with_curvature(&east_hemi);

        // Candy cane.
        let candy_cane =
            text_format::parse_points("-20:150, -20:-70, 0:70, 10:-150, 10:70, -10:-70");
        test_area_consistent_with_curvature(&candy_cane);

        // Three leaf clover.
        let clover = text_format::parse_points("0:0, -3:3, 3:3, 0:0, 3:0, 3:-3, 0:0, -3:-3, -3:0");
        test_area_consistent_with_curvature(&clover);

        // Tessellated loop.
        let tessellated =
            text_format::parse_points("10:34, 5:34, 0:34, -10:34, -10:36, -5:36, 0:36, 10:36");
        test_area_consistent_with_curvature(&tessellated);
    }

    #[test]
    fn test_get_area_and_centroid_full_loop() {
        assert_eq!(get_area(&[]), 4.0 * PI);
        let centroid = get_centroid(&[]);
        assert_eq!(centroid.0.x, 0.0);
        assert_eq!(centroid.0.y, 0.0);
        assert_eq!(centroid.0.z, 0.0);
    }

    #[test]
    fn test_get_area_hemispheres() {
        let north_hemi = text_format::parse_points("0:-180, 0:-90, 0:0, 0:90");
        assert!((get_area(&north_hemi) - 2.0 * PI).abs() < 1e-13);

        let east_hemi = text_format::parse_points("90:0, 0:0, -90:0, 0:-180");
        assert!((get_area(&east_hemi) - 2.0 * PI).abs() < 1e-12);
    }

    #[test]
    fn test_get_curvature_full() {
        assert_eq!(get_curvature(&[]), -2.0 * PI);
    }

    #[test]
    fn test_get_curvature_degenerate_v() {
        let v_loop = text_format::parse_points("5:1, 0:2, 5:3, 0:2");
        assert_eq!(get_curvature(&v_loop), 2.0 * PI);
    }

    #[test]
    fn test_get_curvature_north_hemi3() {
        let north_hemi3 = text_format::parse_points("0:-180, 0:-60, 0:60");
        assert_eq!(get_curvature(&north_hemi3), 0.0);
    }

    #[test]
    fn test_get_curvature_west_hemi() {
        let west_hemi = text_format::parse_points("0:-180, -90:0, 0:0, 90:0");
        assert!(get_curvature(&west_hemi).abs() < 1e-15);
    }

    #[test]
    fn test_is_normalized() {
        // A small CCW triangle is normalized.
        let ccw = text_format::parse_points("0:0, 0:10, 10:0");
        assert!(is_normalized(&ccw));

        // A CW triangle (reversed) is not normalized.
        let cw: Vec<Point> = ccw.iter().rev().copied().collect();
        assert!(!is_normalized(&cw));
    }

    #[test]
    fn test_kahan_sum_default() {
        let sum = KahanSum::new();
        assert_eq!(sum.value(), 0.0);
    }

    #[test]
    fn test_kahan_sum_single() {
        let mut sum = KahanSum::new();
        sum.add(-3.0);
        assert_eq!(sum.value(), -3.0);
    }

    #[test]
    fn test_kahan_sum_of_squares() {
        for direction in 0..2 {
            let mut safe_sum = KahanSum::new();
            let mut naive_sum = 0.0_f64;
            let n: i64 = 1_000_000;
            for i in 0..=n {
                let v = if direction == 0 { i } else { n - i };
                safe_sum.add((v * v) as f64);
                naive_sum += (v * v) as f64;
            }
            let expected = (2 * n + 1) * n * (n + 1) / 6;
            // Kahan sum should be exact.
            assert_eq!(safe_sum.value(), expected as f64);
            // Naive sum should have significant error.
            assert!((naive_sum - expected as f64).abs() >= expected as f64 / (n * n) as f64);
        }
    }

    /// Check that curvature is *identical* when vertex order is rotated,
    /// and that the sign is inverted when vertices are reversed.
    fn check_curvature_invariants(loop_in: &[Point]) {
        let expected = get_curvature(loop_in);
        let mut loop_pts: Vec<Point> = loop_in.to_vec();

        for _ in 0..loop_pts.len() {
            // Reverse and check.
            loop_pts.reverse();
            let reversed_curvature = get_curvature(&loop_pts);
            let expected_reversed = if expected == 2.0 * PI {
                expected
            } else {
                -expected
            };
            assert_eq!(reversed_curvature, expected_reversed);

            // Reverse back, then rotate.
            loop_pts.reverse();
            loop_pts.rotate_left(1);
            assert_eq!(get_curvature(&loop_pts), expected);
        }
    }

    #[test]
    fn test_curvature_invariants() {
        let north_hemi3 = text_format::parse_points("0:-180, 0:-60, 0:60");
        check_curvature_invariants(&north_hemi3);

        let west_hemi = text_format::parse_points("0:-180, -90:0, 0:0, 90:0");
        check_curvature_invariants(&west_hemi);

        let candy_cane =
            text_format::parse_points("-20:150, -20:-70, 0:70, 10:-150, 10:70, -10:-70");
        check_curvature_invariants(&candy_cane);

        let clover = text_format::parse_points("0:0, -3:3, 3:3, 0:0, 3:0, 3:-3, 0:0, -3:-3, -3:0");
        check_curvature_invariants(&clover);
    }

    #[test]
    fn test_curvature_spiral() {
        // Build a narrow spiral loop starting at the north pole to test that
        // GetCurvature error is linear in vertex count.
        let arm_points = 10000;
        let arm_radius = 0.01;
        let mut spiral = vec![Point::from_coords(0.0, 0.0, 0.0); 2 * arm_points];
        spiral[arm_points] = Point::from_coords(0.0, 0.0, 1.0);
        for i in 0..arm_points {
            let angle = (2.0 * PI / 3.0) * i as f64;
            let x = angle.cos();
            let y = angle.sin();
            let r1 = i as f64 * arm_radius / arm_points as f64;
            let r2 = (i as f64 + 1.5) * arm_radius / arm_points as f64;
            spiral[arm_points - i - 1] = Point(
                crate::r3::Vector {
                    x: r1 * x,
                    y: r1 * y,
                    z: 1.0,
                }
                .normalize(),
            );
            spiral[arm_points + i] = Point(
                crate::r3::Vector {
                    x: r2 * x,
                    y: r2 * y,
                    z: 1.0,
                }
                .normalize(),
            );
        }

        let area = get_area(&spiral);
        let curvature = get_curvature(&spiral);
        let max_error = get_curvature_max_error(&spiral);
        assert!(
            (2.0 * PI - area - curvature).abs() < 0.01 * max_error,
            "area={area}, curvature={curvature}, max_error={max_error}"
        );
    }

    #[test]
    fn test_get_signed_area_underflow() {
        // C++ GetSignedArea::Underflow: a tiny loop should still have positive area.
        let pts = text_format::parse_points("0:0, 0:1e-88, 1e-88:1e-88, 1e-88:0");
        assert!(get_signed_area(&pts) > 0.0);
    }
}
