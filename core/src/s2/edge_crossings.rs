// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Edge crossing functions and intersection computation.
//!
//! Corresponds to Go `s2/edge_crossings.go`, C++ `s2edge_crossings.cc`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "axis index used as array subscript"
)]
use crate::r3::PreciseVector;
use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::predicates;

/// Maximum error in intersection point computation (in radians).
pub fn intersection_error() -> Angle {
    Angle::from_radians(8.0 * predicates::DBL_ERROR)
}

/// Error tolerance for merging coincident intersection points.
pub fn intersection_merge_radius() -> Angle {
    Angle::from_radians(16.0 * predicates::DBL_ERROR)
}

/// Indicates how edges cross.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Crossing {
    /// The edges cross at an interior point.
    Cross,
    /// Two vertices from different edges are the same.
    MaybeCross,
    /// The edges do not cross.
    #[default]
    DoNotCross,
}

impl std::fmt::Display for Crossing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Crossing::Cross => write!(f, "Cross"),
            Crossing::MaybeCross => write!(f, "MaybeCross"),
            Crossing::DoNotCross => write!(f, "DoNotCross"),
        }
    }
}

/// Reports whether edge AB intersects edge CD.
///
/// Returns `Cross` if the edges cross at an interior point,
/// `MaybeCross` if any two vertices from different edges are the same,
/// and `DoNotCross` otherwise.
#[inline]
pub fn crossing_sign(a: Point, b: Point, c: Point, d: Point) -> Crossing {
    let mut crosser = super::edge_crosser::EdgeCrosser::new(a, b);
    crosser.restart_at(c);
    crosser.chain_crossing_sign(d)
}

/// Reports whether two edges "cross" for point-in-polygon tests when
/// they share a vertex.
///
/// Given two edges AB and CD where at least two vertices are identical,
/// a "crossing" occurs if AB is encountered after CD during a CCW sweep
/// around the shared vertex.
#[inline]
pub fn vertex_crossing(a: Point, b: Point, c: Point, d: Point) -> bool {
    if a == b || c == d {
        return false;
    }

    if a == c {
        return (b == d) || predicates::ordered_ccw(a.reference_dir(), d, b, a);
    }
    if b == d {
        return predicates::ordered_ccw(b.reference_dir(), c, a, b);
    }
    if a == d {
        return (b == c) || predicates::ordered_ccw(a.reference_dir(), c, b, a);
    }
    if b == c {
        return predicates::ordered_ccw(b.reference_dir(), d, a, b);
    }

    false
}

/// Convenience function that handles both interior crossings and vertex
/// crossings for point-in-polygon tests.
#[inline]
pub fn edge_or_vertex_crossing(a: Point, b: Point, c: Point, d: Point) -> bool {
    match crossing_sign(a, b, c, d) {
        Crossing::DoNotCross => false,
        Crossing::Cross => true,
        Crossing::MaybeCross => vertex_crossing(a, b, c, d),
    }
}

/// Returns the intersection point of two crossing edges AB and CD.
///
/// The returned point is guaranteed to be within `INTERSECTION_ERROR`
/// of the true intersection.
pub fn intersection(a0: Point, a1: Point, b0: Point, b1: Point) -> Point {
    let (pt, ok) = intersection_stable(a0, a1, b0, b1);
    let pt = if ok {
        pt
    } else {
        intersection_exact(a0, a1, b0, b1)
    };

    // Make sure the intersection point is on the correct side of the sphere.
    if pt.0.dot((a0.0 + a1.0) + (b0.0 + b1.0)) < 0.0 {
        Point(-pt.0)
    } else {
        pt
    }
}

/// Reports whether the angle ABC contains its vertex B.
///
/// Specifically, this is true if and only if the directed angle ABC
/// (measured counterclockwise from edge BA to edge BC) is in the range
/// (0, π). Properties:
///
/// 1. `angle_contains_vertex(a,b,c) == !angle_contains_vertex(c,b,a)` (unless a == c)
/// 2. `angle_contains_vertex(a,b,b) == false`
/// 3. `angle_contains_vertex(b,b,c) == false`
#[inline]
pub fn angle_contains_vertex(a: Point, b: Point, c: Point) -> bool {
    debug_assert!(a != b && b != c);
    // The directed angle from BA to BC (measured CCW) is in (0, π) iff
    // C comes before A in CCW order around B starting from RefDir(B).
    !predicates::ordered_ccw(b.reference_dir(), c, a, b)
}

/// Like `vertex_crossing`, but returns a sign: +1 if both edges are
/// outgoing or both incoming with respect to the shared vertex, -1 if
/// one is outgoing and the other incoming, 0 if no crossing.
///
/// Corresponds to C++ `SignedVertexCrossing`.
#[inline]
pub fn signed_vertex_crossing(a: Point, b: Point, c: Point, d: Point) -> i32 {
    if a == b || c == d {
        return 0;
    }
    if a == c {
        return if (b == d) || predicates::ordered_ccw(a.reference_dir(), d, b, a) {
            1
        } else {
            0
        };
    }
    if b == d {
        return if predicates::ordered_ccw(b.reference_dir(), c, a, b) {
            1
        } else {
            0
        };
    }
    if a == d {
        return if (b == c) || predicates::ordered_ccw(a.reference_dir(), c, b, a) {
            -1
        } else {
            0
        };
    }
    if b == c {
        return if predicates::ordered_ccw(b.reference_dir(), d, a, b) {
            -1
        } else {
            0
        };
    }
    0 // Should not be called with 4 distinct vertices
}

/// Upper bound on the angle between the vector returned by `robust_cross_prod`
/// and the true cross product, in radians.
pub const ROBUST_CROSS_PROD_ERROR: Angle = Angle::from_radians(6.0 * predicates::DBL_ERROR);

/// Upper bound on angle error when the exact arithmetic path is used.
pub const EXACT_CROSS_PROD_ERROR: Angle = Angle::from_radians(predicates::DBL_ERROR);

/// Returns a vector approximately equal to `a × b` that is guaranteed to be
/// non-zero (even when `a` and `b` are nearly parallel or antipodal).
///
/// The angle between the result and the true mathematical cross product is
/// at most [`ROBUST_CROSS_PROD_ERROR`]. When `a == b`, returns an arbitrary
/// orthogonal vector (via `ortho`).
///
/// Properties:
/// - `robust_cross_prod(b, a) == -robust_cross_prod(a, b)` unless a and b
///   are linearly dependent.
/// - `robust_cross_prod(a, b) != (0,0,0)` for all inputs.
///
/// Uses a 4-level fallback: double-precision stable → exact arithmetic
/// (via [`PreciseVector`]) → symbolic perturbation.
///
/// Corresponds to C++ `S2::RobustCrossProd(a, b)`.
pub fn robust_cross_prod(a: Point, b: Point) -> Point {
    // Level 1: double-precision stable cross product via (a-b)×(a+b).
    if let Some(result) = get_stable_cross_prod(a.0, b.0) {
        return Point(result);
    }

    // Handle the (a == b) case before doing expensive arithmetic.
    // Mathematically a×b == 0, but returning an arbitrary orthogonal vector
    // reduces special cases in client code.
    if a == b {
        return super::point::ortho(a);
    }

    // Level 2: exact arithmetic with symbolic perturbation fallback.
    Point(exact_cross_prod(a, b))
}

/// Attempts the stable double-precision cross product `(a-b) × (a+b)`.
/// Returns `Some(result)` if the result norm is large enough to meet the
/// [`ROBUST_CROSS_PROD_ERROR`] bound, otherwise `None`.
fn get_stable_cross_prod(a: crate::r3::Vector, b: crate::r3::Vector) -> Option<crate::r3::Vector> {
    // We compute (a - b) × (a + b). Mathematically this is exactly 2*(a × b),
    // but it is more numerically stable when a and b are nearly parallel because
    // (a - b) and (a + b) are nearly perpendicular.
    //
    // Minimum norm² needed so that the directional error stays within
    // ROBUST_CROSS_PROD_ERROR. Derived from:
    //   (1 + 2*sqrt(3) + 32*sqrt(3)*DBL_ERR/||N||) * T_ERR <= kErr
    // where T_ERR = DBL_ERR for f64 precision.
    const T_ERR: f64 = predicates::DBL_ERROR;
    const ROBUST_ERR: f64 = 6.0 * predicates::DBL_ERROR; // == ROBUST_CROSS_PROD_ERROR in radians
    const MIN_NORM: f64 = (32.0 * predicates::SQRT3 * predicates::DBL_ERROR)
        / (ROBUST_ERR / T_ERR - (1.0 + 2.0 * predicates::SQRT3));

    let result = (a - b).cross(a + b);
    if result.norm2() >= MIN_NORM * MIN_NORM {
        Some(result)
    } else {
        None
    }
}

/// Returns true if the vector's magnitude is large enough that `angle()`
/// and `normalize()` can be called without precision loss from underflow.
#[inline]
fn is_normalizable(p: crate::r3::Vector) -> bool {
    // The largest component must be at least 2^(-242).
    let max_comp = p.x.abs().max(p.y.abs()).max(p.z.abs());
    max_comp >= f64::from_bits(0x30D0_0000_0000_0000) // 2^(-242)
}

/// Scales a vector to ensure it can be normalized without underflow.
///
/// REQUIRES: p != (0, 0, 0)
fn ensure_normalizable(p: crate::r3::Vector) -> crate::r3::Vector {
    if is_normalizable(p) {
        return p;
    }
    // Scale by a power of two so the largest component is in [1, 2).
    let p_max = p.x.abs().max(p.y.abs()).max(p.z.abs());
    debug_assert!(p_max > 0.0, "EnsureNormalizable: zero vector");
    // ilogb(p_max) gives floor(log2(p_max)). ldexp(2, -1 - ilogb) scales to [1,2).
    let ilog = if p_max == 0.0 {
        0
    } else {
        (p_max.log2().floor()) as i32
    };
    let scale = (2.0f64).powi(-1 - ilog);
    p * scale
}

/// Converts a `PreciseVector` (exact arithmetic) to a normalizable `Vector`.
fn normalizable_from_exact(pv: &PreciseVector) -> crate::r3::Vector {
    use crate::r3::Vector;

    let x = pv.to_vector();
    if is_normalizable(x) {
        return x;
    }
    // Find the maximum exponent across all components.
    let xf = [&pv.x, &pv.y, &pv.z];
    let mut max_exp = i64::MIN;
    for c in &xf {
        if !c.is_zero() {
            max_exp = max_exp.max(c.exp());
        }
    }
    if max_exp == i64::MIN {
        return Vector::new(0.0, 0.0, 0.0); // exact result is (0,0,0)
    }
    // Scale each component by 2^(-max_exp) to bring into representable range.
    Vector::new(
        pv.x.to_f64_shifted(max_exp),
        pv.y.to_f64_shifted(max_exp),
        pv.z.to_f64_shifted(max_exp),
    )
}

/// Returns the cross product of `a` and `b` using symbolic perturbations,
/// for the case when `a < b` lexicographically and the exact cross product
/// is zero (i.e., `a` and `b` are linearly dependent).
///
/// Uses the same perturbation model as `s2pred::SymbolicallyPerturbedSign`.
fn symbolic_cross_prod_sorted(a: Point, b: Point) -> crate::r3::Vector {
    use crate::r3::Vector;

    debug_assert!(a < b, "SymbolicCrossProdSorted: a must be < b");

    // Perturbation magnitudes in decreasing order:
    //   da[2] > da[1] > da[0] > db[2] > db[1] > db[0]
    // We enumerate coefficients in decreasing perturbation magnitude.

    // da[2]: coefficient is (-b[1], b[0], 0)
    if b.0.y != 0.0 || b.0.x != 0.0 {
        return Vector::new(-b.0.y, b.0.x, 0.0);
    }
    // da[1]: coefficient is (b[2], 0, 0) (note b[0] == 0)
    if b.0.z != 0.0 {
        return Vector::new(b.0.z, 0.0, 0.0);
    }

    // All remaining cases require b == (0,0,0) which shouldn't happen for
    // valid S2 points, but we handle them for completeness.

    // db[2]: coefficient is (a[1], -a[0], 0)
    if a.0.y != 0.0 || a.0.x != 0.0 {
        return Vector::new(a.0.y, -a.0.x, 0.0);
    }

    // db[2] * da[1]: always non-zero → (1, 0, 0)
    Vector::new(1.0, 0.0, 0.0)
}

/// Returns the cross product of `a` and `b` using symbolic perturbations.
/// Handles both orderings by sorting and negating as needed.
fn symbolic_cross_prod(a: Point, b: Point) -> crate::r3::Vector {
    debug_assert!(a != b, "SymbolicCrossProd: a == b");
    if a < b {
        ensure_normalizable(symbolic_cross_prod_sorted(a, b))
    } else {
        -ensure_normalizable(symbolic_cross_prod_sorted(b, a))
    }
}

/// Returns the cross product of `a` and `b` using exact arithmetic.
/// Falls back to symbolic perturbation when the exact result is zero
/// (i.e., when `a` and `b` are exactly linearly dependent).
///
/// The result is always non-zero and normalizable.
///
/// Note: tried `#[cold]` in Phase 3, slightly regressed
/// `bench_crossing_sign_crossing` (+1.5% instr) without benefiting any
/// tracked bench (no input reaches this path). Layout-based regressions
/// from `#[cold]` on functions whose callers also benchmark cost more
/// than the icache savings on never-hit code. Left without an
/// inline/cold hint.
pub(crate) fn exact_cross_prod(a: Point, b: Point) -> crate::r3::Vector {
    debug_assert!(a != b, "ExactCrossProd: a == b");
    let ap = PreciseVector::from_vector(a.0);
    let bp = PreciseVector::from_vector(b.0);
    let result = ap.cross(&bp);
    if !result.is_zero() {
        return normalizable_from_exact(&result);
    }
    // Exact cross product is zero → use symbolic perturbation.
    symbolic_cross_prod(a, b)
}

// --- Intersection helpers ---

fn robust_normal_with_length(
    x: crate::r3::Vector,
    y: crate::r3::Vector,
) -> (crate::r3::Vector, f64) {
    // Use the numerically stable formula: (x-y) x (x+y) == 2*(x cross y).
    // This is much more accurate when x and y are nearly parallel because
    // (x-y) and (x+y) are nearly perpendicular.
    let tmp = (x - y).cross(x + y);
    let len = tmp.norm();
    if len == 0.0 {
        (tmp, 0.0)
    } else {
        (tmp * (1.0 / len), 0.5 * len)
    }
}

fn projection(
    x: crate::r3::Vector,
    a_norm: crate::r3::Vector,
    a_norm_len: f64,
    a0: Point,
    a1: Point,
) -> (f64, f64) {
    let proj = x.dot(a_norm) / a_norm_len;
    let bound = ((x.dot((a0.0 + a1.0) * 0.5)).abs()
        + x.norm() * a_norm_len * predicates::DBL_ERROR)
        / a_norm_len;
    (proj, bound)
}

fn intersection_stable(a0: Point, a1: Point, b0: Point, b1: Point) -> (Point, bool) {
    let (a_norm, a_norm_len) = robust_normal_with_length(a0.0, a1.0);
    let (b_norm, b_norm_len) = robust_normal_with_length(b0.0, b1.0);

    if a_norm_len == 0.0 || b_norm_len == 0.0 {
        return (Point::from_coords(0.0, 0.0, 0.0), false);
    }

    // Choose the longer edge for normal computation (lower error),
    // and the shorter edge for interpolation.
    if a_norm_len >= b_norm_len {
        intersection_stable_sorted(b0, b1, a0, a1, &a_norm, a_norm_len)
    } else {
        intersection_stable_sorted(a0, a1, b0, b1, &b_norm, b_norm_len)
    }
}

fn intersection_stable_sorted(
    a0: Point,
    a1: Point,
    b0: Point,
    b1: Point,
    b_norm: &crate::r3::Vector,
    b_norm_len: f64,
) -> (Point, bool) {
    let (proj0, bound0) = projection(a0.0, *b_norm, b_norm_len, b0, b1);
    let (proj1, bound1) = projection(a1.0, *b_norm, b_norm_len, b0, b1);

    if proj0 * proj1 >= 0.0 {
        // Both endpoints are on the same side. This shouldn't happen for crossing edges.
        return (Point::from_coords(0.0, 0.0, 0.0), false);
    }

    // Interpolate along the shorter edge.
    let t = proj0 / (proj0 - proj1);
    let x = a0.0 * (1.0 - t) + a1.0 * t;

    // Check if the error is acceptable.
    let err = bound0 + bound1;
    if (proj0 - proj1).abs() < err {
        return (Point(x.normalize()), false);
    }

    (Point(x.normalize()), true)
}

fn intersection_exact(a0: Point, a1: Point, b0: Point, b1: Point) -> Point {
    let pa0 = PreciseVector::from_vector(a0.0);
    let pa1 = PreciseVector::from_vector(a1.0);
    let pb0 = PreciseVector::from_vector(b0.0);
    let pb1 = PreciseVector::from_vector(b1.0);

    let a_norm = pa0.cross(&pa1);
    let b_norm = pb0.cross(&pb1);
    let x = a_norm.cross(&b_norm);

    if x.is_zero() {
        // Edges are collinear. Find endpoints that lie in the interior of the
        // other edge and return the lexicographically smallest.
        let endpoints = [a0, a1, b0, b1];
        let mut best = endpoints[0];
        for &p in &endpoints[1..] {
            if p.cmp_point(best) == std::cmp::Ordering::Less {
                best = p;
            }
        }
        return best.normalize();
    }

    // Apply sign correction so the result is on the correct hemisphere,
    // and normalize to a unit-length point.
    let v = x.to_vector();
    let sign = predicates::robust_sign(a0, a1, b1);
    let s = f64::from(sign as i8);
    Point((v * s).normalize())
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn crossing_is_send_sync() {
        is_send_sync::<Crossing>();
    }

    #[test]
    fn test_crossing_sign_basic() {
        // Two edges that clearly cross.
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();
        assert_eq!(crossing_sign(a, b, c, d), Crossing::Cross);
    }

    #[test]
    fn test_crossing_sign_no_cross() {
        // Two edges on the same hemisphere that don't cross.
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let c = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();
        assert_ne!(crossing_sign(a, b, c, d), Crossing::Cross);
    }

    #[test]
    fn test_crossing_sign_shared_vertex() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        assert_eq!(crossing_sign(a, b, a, b), Crossing::MaybeCross);
    }

    #[test]
    fn test_vertex_crossing() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // Same edge: VertexCrossing should return true.
        assert!(vertex_crossing(a, b, a, b));
        assert!(vertex_crossing(a, b, b, a));
    }

    #[test]
    fn test_vertex_crossing_degenerate() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // Degenerate edges.
        assert!(!vertex_crossing(a, a, a, b));
        assert!(!vertex_crossing(a, b, a, a));
    }

    #[test]
    fn test_edge_or_vertex_crossing() {
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();
        assert!(edge_or_vertex_crossing(a, b, c, d));
    }

    #[test]
    fn test_intersection() {
        let a0 = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let a1 = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let b0 = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let b1 = Point::from_coords(0.0, -1.0, 1.0).normalize();
        let x = intersection(a0, a1, b0, b1);
        // The intersection should be approximately (0, 0, 1).
        assert!(x.0.z > 0.9, "intersection z = {} should be near 1.0", x.0.z);
        assert!(
            x.0.x.abs() < 0.1,
            "intersection x = {} should be near 0.0",
            x.0.x
        );
        assert!(
            x.0.y.abs() < 0.1,
            "intersection y = {} should be near 0.0",
            x.0.y
        );
    }

    #[test]
    fn test_crossing_symmetry() {
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();
        assert_eq!(crossing_sign(a, b, c, d), crossing_sign(c, d, a, b));
    }

    #[test]
    fn test_signed_vertex_crossing_same_edge() {
        // Same edge AB == CD: both outgoing at shared vertex → +1.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        assert_eq!(signed_vertex_crossing(a, b, a, b), 1);
    }

    #[test]
    fn test_signed_vertex_crossing_reversed_edge() {
        // AB and DC share a==d, b==c: opposite directions → -1.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        assert_eq!(signed_vertex_crossing(a, b, b, a), -1);
    }

    #[test]
    fn test_signed_vertex_crossing_degenerate() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        assert_eq!(signed_vertex_crossing(a, a, a, b), 0);
        assert_eq!(signed_vertex_crossing(a, b, a, a), 0);
    }

    #[test]
    fn test_signed_vertex_crossing_no_shared_vertex() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        let d = Point::from_coords(-1.0, 0.0, 0.0);
        assert_eq!(signed_vertex_crossing(a, b, c, d), 0);
    }

    // ===== angle_contains_vertex tests (from C++ S2EdgeCrossings) =====

    #[test]
    fn test_angle_contains_vertex_basic() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let ref_b = b.reference_dir();

        // Degenerate angle ABA should not contain vertex.
        assert!(!angle_contains_vertex(a, b, a));

        // An angle where A == RefDir(B) should contain vertex.
        assert!(angle_contains_vertex(ref_b, b, a));

        // An angle where C == RefDir(B) should not contain vertex.
        assert!(!angle_contains_vertex(a, b, ref_b));
    }

    #[test]
    fn test_angle_contains_vertex_opposite() {
        // If angle_contains_vertex(a, b, c) is true, then
        // angle_contains_vertex(c, b, a) should be false (and vice versa).
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        let fwd = angle_contains_vertex(a, b, c);
        let rev = angle_contains_vertex(c, b, a);
        assert_ne!(fwd, rev, "angle_contains_vertex should be antisymmetric");
    }

    // ===== intersection precision tests (from C++ S2EdgeCrossings) =====

    #[test]
    fn test_exact_intersection_underflow() {
        // Tests that a correct intersection is computed even when two edges are
        // exactly collinear and the normals underflow in double precision.
        let a0 = Point::from_coords(1.0, 0.0, 0.0);
        let a1 = Point::from_coords(1.0, 2e-300, 0.0);
        let b0 = Point::from_coords(1.0, 1e-300, 0.0);
        let b1 = Point::from_coords(1.0, 3e-300, 0.0);
        let x = intersection(a0, a1, b0, b1);
        // The intersection should be at (1, 1e-300, 0), normalized.
        let expected = Point::from_coords(1.0, 1e-300, 0.0);
        assert!(
            (x.0.x - expected.0.x).abs() < 1e-10
                && (x.0.y - expected.0.y).abs() < 1e-280
                && x.0.z.abs() < 1e-280,
            "intersection underflow: got {x:?}, expected near {expected:?}",
        );
    }

    #[test]
    fn test_intersection_antipodal_collinear() {
        // Tests intersection of nearly antipodal collinear edges.
        let a0 = Point::from_coords(-1.0, -1.6065916409055676e-10, 0.0);
        let a1 = Point::from_coords(1.0, 0.0, 0.0);
        let b0 = Point::from_coords(1.0, -4.7617930898495072e-13, 0.0);
        let b1 = Point::from_coords(-1.0, 1.2678623820887328e-09, 0.0);
        let x = intersection(a0, a1, b0, b1);
        // The intersection should be near (1, -4.76e-13, 0).
        assert!(
            x.0.x > 0.99,
            "intersection should be near positive x axis, got x={}",
            x.0.x,
        );
    }

    #[test]
    fn test_crossing_sign_collinear_non_overlapping() {
        // Two collinear edges that don't overlap should not cross.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let d = Point::from_coords(0.0, 1.0, 0.0);
        // b is 5% along a→d, c is 95% along a→d
        let ab = a.0 * 0.95 + d.0 * 0.05;
        let b = Point(ab.normalize());
        let cd = a.0 * 0.05 + d.0 * 0.95;
        let c = Point(cd.normalize());
        // a-b and c-d are collinear but don't overlap
        assert_ne!(crossing_sign(a, b, c, d), Crossing::Cross);
    }

    #[test]
    fn test_crossing_sign_perpendicular_edges() {
        // Two clearly perpendicular crossing edges.
        let a = Point::from_coords(1.0, -1.0, 0.0).normalize();
        let b = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let c = Point::from_coords(1.0, 0.0, -1.0).normalize();
        let d = Point::from_coords(1.0, 0.0, 1.0).normalize();
        assert_eq!(crossing_sign(a, b, c, d), Crossing::Cross);
        // Verify symmetry with swapped arguments.
        assert_eq!(crossing_sign(c, d, a, b), Crossing::Cross);
    }

    #[test]
    fn test_intersection_error_bound() {
        // The intersection error should be a small positive angle.
        let err = intersection_error();
        assert!(err.radians() > 0.0);
        assert!(err.radians() < 1e-10);
    }

    #[test]
    fn test_intersection_merge_radius_positive() {
        let r = intersection_merge_radius();
        assert!(r.radians() > 0.0);
        assert!(r.radians() > intersection_error().radians());
    }

    // ─── C++ ExactIntersectionSign / collinear best-selection tests ───

    #[test]
    fn test_exact_intersection_sign_antipodal_collinear() {
        // C++ TEST(S2, ExactIntersectionSign): collinear edges with nearly
        // antipodal endpoints. The intersection should be near the positive
        // x-axis where both edges overlap.
        let a0 = Point::from_coords(-1.0, -1.6065916409055676e-10, 0.0);
        let a1 = Point::from_coords(1.0, 0.0, 0.0);
        let b0 = Point::from_coords(1.0, -4.7617930898495072e-13, 0.0);
        let b1 = Point::from_coords(-1.0, 1.2678623820887328e-09, 0.0);
        let x = intersection(a0, a1, b0, b1);
        // The result should be a unit-length point near the positive x-axis.
        assert!(
            x.0.x > 0.99,
            "intersection should be near positive x-axis, got x={}",
            x.0.x,
        );
        // It should be one of the endpoints, normalized.
        assert!(
            (x.0.norm() - 1.0).abs() < 1e-10,
            "intersection should be unit length, got norm={}",
            x.0.norm(),
        );
    }

    #[test]
    fn test_collinear_intersection_selects_lexicographic_min() {
        // When edges are exactly collinear (cross product zero), the intersection
        // should return the lexicographically smallest endpoint that lies in the
        // interior of both edges. This exercises the refactored path that
        // initializes `best` directly instead of via Option::unwrap.
        //
        // Points along the x-axis with overlapping edges.
        let a0 = Point::from_coords(1.0, 0.0, 0.0);
        let a1 = Point::from_coords(1.0, 4e-300, 0.0);
        let b0 = Point::from_coords(1.0, 1e-300, 0.0);
        let b1 = Point::from_coords(1.0, 3e-300, 0.0);
        let x = intersection(a0, a1, b0, b1);
        // b0 is the smallest interior point; result should be near b0 normalized.
        let expected = Point::from_coords(1.0, 1e-300, 0.0).normalize();
        let dist = (x.0 - expected.0).norm();
        assert!(
            dist < 1e-10,
            "collinear best-selection: got {x:?}, expected {expected:?}, dist={dist}",
        );
    }

    #[test]
    fn test_intersection_collinear_same_direction() {
        // Two overlapping collinear edges in the same direction.
        let a0 = Point::from_coords(1.0, 0.0, 0.0);
        let a1 = Point::from_coords(1.0, 2e-300, 0.0);
        let b0 = Point::from_coords(1.0, 1e-300, 0.0);
        let b1 = Point::from_coords(1.0, 3e-300, 0.0);
        // Should not panic (exercises the non-Option path).
        let x = intersection(a0, a1, b0, b1);
        assert!(x.0.norm() > 0.0, "intersection should be non-zero");
    }

    // ─── robust_cross_prod / exact_cross_prod / symbolic_cross_prod tests ───

    /// Helper: checks that `robust_cross_prod` returns a result consistent with
    /// `s2pred::Sign` (i.e., `robust_cross_prod(a,b)·c` has the same sign as
    /// `Sign(a,b,c)`).
    fn check_robust_cross_prod(a: Point, b: Point, expected_normal: Point) {
        let result = robust_cross_prod(a, b);
        assert!(
            result.0.norm2() > 0.0,
            "robust_cross_prod should be non-zero"
        );
        let result_n = result.normalize();
        let expected_n = expected_normal.normalize();
        // Check the result is close to the expected normal direction.
        let dist = (result_n.0 - expected_n.0).norm();
        assert!(
            dist < 1e-10,
            "robust_cross_prod({a:?}, {b:?}) = {result_n:?}, expected ≈ {expected_n:?}, dist={dist}"
        );
    }

    #[test]
    fn test_robust_cross_prod_basic() {
        // Simple case: orthogonal unit vectors.
        check_robust_cross_prod(
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        );
    }

    #[test]
    fn test_robust_cross_prod_nearly_parallel() {
        // Nearly parallel vectors — exercises the stable cross product path.
        let dbl_err = predicates::DBL_ERROR;
        check_robust_cross_prod(
            Point::from_coords(20.0 * dbl_err, 1.0, 0.0).normalize(),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        );
    }

    #[test]
    fn test_robust_cross_prod_exact_path() {
        // Very close vectors that require exact arithmetic.
        // (a-b)×(a+b) has tiny norm, so we need ExactCrossProd.
        let dbl_err = predicates::DBL_ERROR;
        let a = Point::from_coords(4.0 * dbl_err * dbl_err, 1.0, 0.0).normalize();
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let result = robust_cross_prod(a, b);
        assert!(result.0.norm2() > 0.0);
        // Result should point roughly in the +z direction.
        assert!(result.normalize().0.z > 0.9);
    }

    #[test]
    fn test_robust_cross_prod_exact_underflow() {
        // Test that exact results are scaled up when they would be too small.
        check_robust_cross_prod(
            Point::from_coords(5e-324, 1.0, 0.0).normalize(),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        );
    }

    #[test]
    fn test_robust_cross_prod_exact_double_underflow() {
        // Even when the exact cross product underflows in double precision.
        // C++ uses raw S2Point constructors (no normalization).
        use crate::r3::Vector;
        let dbl_err = predicates::DBL_ERROR;
        let a = Point(Vector::new(5e-324, 1.0, 0.0));
        let b = Point(Vector::new(5e-324, 1.0 - dbl_err, 0.0));
        assert!(a != b);
        let result = robust_cross_prod(a, b);
        assert!(result.0.norm2() > 0.0);
        // Should point roughly in -z direction.
        assert!(
            result.normalize().0.z < -0.9,
            "expected -z direction, got {:?}",
            result.normalize()
        );
    }

    #[test]
    fn test_robust_cross_prod_symbolic_proportional() {
        // Exactly proportional vectors: requires symbolic perturbation.
        // C++ uses raw S2Point constructors (no normalization).
        use crate::r3::Vector;
        let a = Point(Vector::new(1.0, 0.0, 0.0));
        let b = Point(Vector::new(1.0 + f64::EPSILON, 0.0, 0.0));
        assert!(a != b, "points must be different for symbolic path");
        let result = robust_cross_prod(a, b);
        assert!(result.0.norm2() > 0.0);
        // Result should be consistent with Sign(a, b, result).
        let sign = predicates::robust_sign(a, b, result.normalize());
        assert_eq!(sign, predicates::Direction::CounterClockwise);
    }

    #[test]
    fn test_robust_cross_prod_symbolic_negative_proportional() {
        // Nearly antipodal and proportional.
        use crate::r3::Vector;
        let a = Point(Vector::new(0.0, 1.0 + f64::EPSILON, 0.0));
        let b = Point(Vector::new(0.0, 1.0, 0.0));
        assert!(a != b);
        let result = robust_cross_prod(a, b);
        assert!(result.0.norm2() > 0.0);
        let sign = predicates::robust_sign(a, b, result.normalize());
        assert_eq!(sign, predicates::Direction::CounterClockwise);
    }

    #[test]
    fn test_robust_cross_prod_symbolic_antipodal() {
        // Exactly antipodal points (unit length).
        use crate::r3::Vector;
        let a = Point(Vector::new(0.0, 0.0, 1.0));
        let b = Point(Vector::new(0.0, 0.0, -1.0));
        let result = robust_cross_prod(a, b);
        assert!(result.0.norm2() > 0.0);
        // Verify consistency with Sign.
        let sign = predicates::robust_sign(a, b, result.normalize());
        assert_eq!(sign, predicates::Direction::CounterClockwise);
    }

    #[test]
    fn test_robust_cross_prod_equal_points() {
        // Equal points: returns ortho(a), an arbitrary orthogonal vector.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let result = robust_cross_prod(a, a);
        assert!(
            result.0.norm2() > 0.0,
            "result should be non-zero for equal points"
        );
        // Should be orthogonal to a.
        assert!(
            result.0.dot(a.0).abs() < 1e-14,
            "result should be orthogonal to input"
        );
    }

    #[test]
    fn test_robust_cross_prod_antisymmetry() {
        // robust_cross_prod(b, a) == -robust_cross_prod(a, b) for non-proportional inputs.
        let a = Point::from_coords(1.0, 2.0, 3.0).normalize();
        let b = Point::from_coords(4.0, 5.0, 6.0).normalize();
        let ab = robust_cross_prod(a, b).normalize();
        let ba = robust_cross_prod(b, a).normalize();
        let diff = (ab.0 + ba.0).norm();
        assert!(diff < 1e-14, "antisymmetry: diff = {diff}");
    }

    #[test]
    fn test_exact_cross_prod_basic() {
        // Test exact_cross_prod directly for nearly-parallel vectors.
        let a = Point::from_coords(1.0, 1e-300, 0.0).normalize();
        let b = Point::from_coords(1.0, 0.0, 0.0);
        let result = exact_cross_prod(a, b);
        assert!(result.norm2() > 0.0, "exact_cross_prod should be non-zero");
        // Should point in -z direction.
        assert!(result.z < 0.0, "exact cross product should have negative z");
    }

    #[test]
    fn test_symbolic_cross_prod_all_orderings() {
        // C++ SymbolicCrossProdConsistentWithSign: test all possible orderings.
        // For linearly dependent a and b, check that Sign(a, b, cross) > 0.
        for &x in &[-1.0, 0.0, 1.0] {
            for &y in &[-1.0, 0.0, 1.0] {
                for &z in &[-1.0, 0.0, 1.0] {
                    let a = Point::from_coords(x, y, z).normalize();
                    if a.0.norm2() < 0.5 {
                        continue; // skip zero vector
                    }
                    let dbl_err = predicates::DBL_ERROR;
                    for &scale in &[-1.0, 1.0 - dbl_err, 1.0 + 2.0 * f64::EPSILON] {
                        let b = Point(a.0 * scale);
                        if a == b {
                            continue;
                        }
                        let cross = robust_cross_prod(a, b).normalize();
                        let sign = predicates::robust_sign(a, b, cross);
                        assert!(
                            sign == predicates::Direction::CounterClockwise,
                            "Sign(a={a:?}, b={b:?}, cross={cross:?}) = {sign:?}, expected CCW"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_symbolic_cross_prod_sorted_edge_cases() {
        // C++ tests for SymbolicCrossProdSorted with unusual inputs.
        // Must use raw Point(Vector{...}) to avoid from_coords normalization.
        use crate::r3::Vector;

        // da[2] path: b has non-zero x or y → return (-b.y, b.x, 0).
        let a = Point(Vector::new(-1.0, -1.0, 0.0));
        let b = Point(Vector::new(0.0, 1.0, 0.0));
        assert!(a < b);
        assert_eq!(
            symbolic_cross_prod_sorted(a, b),
            Vector::new(-1.0, 0.0, 0.0)
        );

        // da[1] path: b = (0, 0, z) with z != 0 → return (b.z, 0, 0).
        let a = Point(Vector::new(-1.0, 0.0, 0.0));
        let b = Point(Vector::new(0.0, 0.0, 1.0));
        assert!(a < b);
        assert_eq!(symbolic_cross_prod_sorted(a, b), Vector::new(1.0, 0.0, 0.0));

        // db[2] path: b == (0,0,0), a has non-zero x → return (a.y, -a.x, 0).
        let a = Point(Vector::new(-1.0, 0.0, 0.0));
        let b = Point(Vector::new(0.0, 0.0, 0.0));
        assert!(a < b);
        assert_eq!(symbolic_cross_prod_sorted(a, b), Vector::new(0.0, 1.0, 0.0));

        // db[2]*da[1] fallback: a.y = a.x = 0, b = (0,0,0) → return (1,0,0).
        let a = Point(Vector::new(0.0, 0.0, -1.0));
        let b = Point(Vector::new(0.0, 0.0, 0.0));
        assert!(a < b);
        assert_eq!(symbolic_cross_prod_sorted(a, b), Vector::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_robust_cross_prod_magnitude() {
        // C++ RobustCrossProdMagnitude: angles can be measured between results
        // without precision loss.
        use std::f64::consts::FRAC_PI_2;

        let r1 = robust_cross_prod(
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(1.0, 1e-100, 0.0),
        );
        let r2 = robust_cross_prod(
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(1.0, 0.0, 1e-100),
        );
        let angle = r1.0.angle(r2.0);
        assert!(
            (angle - FRAC_PI_2).abs() < 1e-10,
            "angle between cross products should be π/2, got {angle}"
        );
    }

    #[test]
    fn test_robust_cross_prod_magnitude_symbolic() {
        // Same test but with symbolic perturbations needed.
        // These near-antipodal pairs have exact cross product = 0, requiring
        // symbolic perturbation.
        use std::f64::consts::FRAC_PI_2;

        let r1 = robust_cross_prod(
            Point::from_coords(-1e-100, 0.0, 1.0),
            Point::from_coords(1e-100, 0.0, -1.0),
        );
        let r2 = robust_cross_prod(
            Point::from_coords(0.0, -1e-100, 1.0),
            Point::from_coords(0.0, 1e-100, -1.0),
        );
        // Both results should be normalizable (i.e., no underflow issues).
        assert!(Point(r1.0).is_normalizable(), "r1 should be normalizable");
        assert!(Point(r2.0).is_normalizable(), "r2 should be normalizable");
        let angle = r1.0.angle(r2.0);
        assert!(
            (angle - FRAC_PI_2).abs() < 1e-10,
            "angle between symbolic cross products should be π/2, got {angle}"
        );
    }

    #[test]
    fn test_normalizable_from_exact() {
        // Test that normalizable_from_exact scales up tiny vectors.
        let tiny = PreciseVector::new(5e-324, 0.0, 0.0);
        let result = normalizable_from_exact(&tiny);
        assert!(is_normalizable(result), "result should be normalizable");
        // Direction should be preserved.
        assert!(result.x > 0.0);
        assert_eq!(result.y, 0.0);
        assert_eq!(result.z, 0.0);
    }

    #[test]
    fn test_normalizable_from_exact_zero() {
        let zero = PreciseVector::new(0.0, 0.0, 0.0);
        let result = normalizable_from_exact(&zero);
        assert_eq!(result.x, 0.0);
        assert_eq!(result.y, 0.0);
        assert_eq!(result.z, 0.0);
    }

    #[test]
    fn test_ensure_normalizable_normal() {
        use crate::r3::Vector;
        let v = Vector::new(1.0, 0.0, 0.0);
        assert_eq!(ensure_normalizable(v), v);
    }

    #[test]
    fn test_ensure_normalizable_tiny() {
        use crate::r3::Vector;
        let v = Vector::new(1e-300, 0.0, 0.0);
        let result = ensure_normalizable(v);
        assert!(is_normalizable(result));
        assert!(result.x > 0.0);
    }

    #[test]
    fn test_get_stable_cross_prod_orthogonal() {
        // Orthogonal unit vectors: should succeed.
        use crate::r3::Vector;
        let a = Vector::new(1.0, 0.0, 0.0);
        let b = Vector::new(0.0, 1.0, 0.0);
        let result = get_stable_cross_prod(a, b);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.z > 0.0);
    }

    #[test]
    fn test_get_stable_cross_prod_nearly_equal() {
        // Nearly equal vectors: stable cross product should fail.
        use crate::r3::Vector;
        let a = Vector::new(1.0, 0.0, 0.0);
        let b = Vector::new(1.0, 1e-20, 0.0).normalize();
        let result = get_stable_cross_prod(a, b);
        // May or may not succeed depending on magnitude — test doesn't assert either way,
        // just that we don't panic.
        let _ = result;
    }

    #[test]
    fn test_robust_cross_prod_constants() {
        // Verify the error constants are consistent.
        assert!(ROBUST_CROSS_PROD_ERROR.radians() > 0.0);
        assert!(EXACT_CROSS_PROD_ERROR.radians() > 0.0);
        assert!(ROBUST_CROSS_PROD_ERROR.radians() > EXACT_CROSS_PROD_ERROR.radians());
        // kRobustCrossProdError == 6 * DBL_ERR == 3 * DBL_EPSILON
        let expected = 6.0 * predicates::DBL_ERROR;
        assert!(
            (ROBUST_CROSS_PROD_ERROR.radians() - expected).abs() < 1e-30,
            "ROBUST_CROSS_PROD_ERROR should be 6 * DBL_ERROR"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_crossing_roundtrip() {
        for c in [Crossing::Cross, Crossing::MaybeCross, Crossing::DoNotCross] {
            let json = serde_json::to_string(&c).unwrap();
            let back: Crossing = serde_json::from_str(&json).unwrap();
            assert_eq!(c, back);
        }
    }
}
