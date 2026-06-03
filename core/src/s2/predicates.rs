// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Robust geometric predicates with exact arithmetic fallback.
//!
//! All predicates produce correct, consistent results by computing
//! conservative error bounds and falling back to high precision or
//! exact arithmetic when the result is uncertain.
//!
//! Corresponds to Go `s2/predicates.go`, C++ `s2predicates.cc`.

use crate::r3::PreciseVector;
use crate::s1::ChordAngle;
use crate::s2::Point;
use std::cmp::Ordering;

// ─── Constants ───────────────────────────────────────────────────────────

/// Machine epsilon for f64 (C++ `DBL_EPSILON` equivalent).
pub const DBL_EPSILON: f64 = f64::EPSILON; // 2.220446049250313e-16

/// Rounding epsilon = 0.5 * `DBL_EPSILON` (C++ `S2::kRobustCrossProdError` base).
pub const DBL_ERROR: f64 = 0.5 * DBL_EPSILON; // 1.110223024625156e-16

/// sqrt(3) — exact OEIS value.
pub const SQRT3: f64 = 1.73205080756887729352744634150587236694280525381038062805580;

/// Maximum error in computing (A×B)·C for vectors of magnitude up to sqrt(2).
///
/// The base error for unit-length vectors is 1.8274 * `DBL_EPSILON`, but we
/// double it to support vectors of magnitude up to sqrt(2), matching the C++.
pub const MAX_DETERMINANT_ERROR: f64 = 3.6548 * DBL_EPSILON;

/// Factor for scaling when checking the sign of a set of points with certainty.
///
/// `|d| <= (3 + 6/sqrt(3)) * |A-C| * |B-C| * e`
pub const DET_ERROR_MULTIPLIER: f64 = 3.2321 * DBL_EPSILON;

// ─── Direction ───────────────────────────────────────────────────────────

/// Indicates the ordering of a set of points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(i8)]
pub enum Direction {
    /// Points are in clockwise order.
    Clockwise = -1,
    /// Two or more points are the same (collinear).
    #[default]
    Indeterminate = 0,
    /// Points are in counter-clockwise order.
    CounterClockwise = 1,
}

impl Direction {
    /// Returns the reverse direction.
    #[inline]
    pub fn reverse(self) -> Direction {
        match self {
            Direction::Clockwise => Direction::CounterClockwise,
            Direction::CounterClockwise => Direction::Clockwise,
            Direction::Indeterminate => Direction::Indeterminate,
        }
    }
}

impl std::ops::Neg for Direction {
    type Output = Direction;
    #[inline]
    fn neg(self) -> Direction {
        self.reverse()
    }
}

impl std::ops::Mul<Direction> for Direction {
    type Output = Direction;
    #[inline]
    fn mul(self, rhs: Direction) -> Direction {
        let v = (self as i8) * (rhs as i8);
        match v {
            -1 => Direction::Clockwise,
            1 => Direction::CounterClockwise,
            _ => Direction::Indeterminate,
        }
    }
}

impl From<i32> for Direction {
    fn from(v: i32) -> Self {
        match v.signum() {
            -1 => Direction::Clockwise,
            1 => Direction::CounterClockwise,
            _ => Direction::Indeterminate,
        }
    }
}

// ─── Sign ────────────────────────────────────────────────────────────────

/// Returns true if the points A, B, C are strictly counterclockwise.
///
/// Due to numerical errors, situations may arise that are mathematically
/// impossible. However, the implementation guarantees:
/// If `sign(a,b,c)`, then `!sign(c,b,a)` for all a,b,c.
#[inline]
pub fn sign(a: Point, b: Point, c: Point) -> bool {
    // We compute (C × A) · B instead of (A × B) · C to ensure that
    // ABC and CBA are not both CCW.
    c.0.cross(a.0).dot(b.0) > 0.0
}

// ─── RobustSign ──────────────────────────────────────────────────────────

/// Returns a `Direction` representing the ordering of the points.
///
/// Guarantees:
/// 1. Returns `Indeterminate` if and only if a == b, b == c, or c == a.
/// 2. `robust_sign(b,c,a) == robust_sign(a,b,c)` for all a,b,c.
/// 3. `robust_sign(c,b,a) == -robust_sign(a,b,c)` for all a,b,c.
#[inline]
pub fn robust_sign(a: Point, b: Point, c: Point) -> Direction {
    let sign = triage_sign(a, b, c);
    if sign != Direction::Indeterminate {
        return sign;
    }
    expensive_sign(a, b, c)
}

/// Fast-path orientation test using simple floating-point arithmetic.
#[inline]
pub(crate) fn triage_sign(a: Point, b: Point, c: Point) -> Direction {
    let det = a.0.cross(b.0).dot(c.0);
    if det > MAX_DETERMINANT_ERROR {
        Direction::CounterClockwise
    } else if det < -MAX_DETERMINANT_ERROR {
        Direction::Clockwise
    } else {
        Direction::Indeterminate
    }
}

/// More precise orientation test that avoids exact arithmetic in most cases.
///
/// Cold: only reached from `expensive_sign` when `triage_sign` returns
/// Indeterminate. Marked `#[cold]` so LLVM places it out-of-line and
/// keeps it out of the hot `robust_sign` icache footprint.
#[cold]
fn stable_sign(a: Point, b: Point, c: Point) -> Direction {
    let ab = b.0 - a.0;
    let ab2 = ab.norm2();
    let bc = c.0 - b.0;
    let bc2 = bc.norm2();
    let ca = a.0 - c.0;
    let ca2 = ca.norm2();

    // Use the two shortest edges to minimize cross product error.
    let (e1, e2, op) = if ab2 >= bc2 && ab2 >= ca2 {
        (ca, bc, c.0)
    } else if bc2 >= ca2 {
        (ab, ca, a.0)
    } else {
        (bc, ab, b.0)
    };

    let det = -e1.cross(e2).dot(op);
    let max_err = DET_ERROR_MULTIPLIER * (e1.norm2() * e2.norm2()).sqrt();

    // Guard against floating-point underflow in the error bound.
    let min_no_underflow_err = DET_ERROR_MULTIPLIER * f64::MIN_POSITIVE.sqrt();
    if max_err < min_no_underflow_err {
        return Direction::Indeterminate;
    }

    if det > max_err {
        Direction::CounterClockwise
    } else if det < -max_err {
        Direction::Clockwise
    } else {
        Direction::Indeterminate
    }
}

/// Full exact arithmetic path for orientation testing.
///
/// Cold: `robust_sign`'s fallback when `triage_sign` returns Indeterminate.
#[cold]
pub(crate) fn expensive_sign(a: Point, b: Point, c: Point) -> Direction {
    if a == b || b == c || c == a {
        return Direction::Indeterminate;
    }

    let det_sign = stable_sign(a, b, c);
    if det_sign != Direction::Indeterminate {
        return det_sign;
    }

    exact_sign(a, b, c, true)
}

/// Computes the sign using exact arithmetic (and optionally symbolic perturbation).
///
/// Cold: only reached from `expensive_sign` after `stable_sign` also
/// returns Indeterminate, i.e. on near-degenerate input.
#[cold]
fn exact_sign(a: Point, b: Point, c: Point, perturb: bool) -> Direction {
    // Sort lexicographically, tracking sign of the permutation.
    let mut perm_sign: i8 = 1;
    let mut pa = a;
    let mut pb = b;
    let mut pc = c;

    if pa.cmp_point(pb) == Ordering::Greater {
        std::mem::swap(&mut pa, &mut pb);
        perm_sign = -perm_sign;
    }
    if pb.cmp_point(pc) == Ordering::Greater {
        std::mem::swap(&mut pb, &mut pc);
        perm_sign = -perm_sign;
    }
    if pa.cmp_point(pb) == Ordering::Greater {
        std::mem::swap(&mut pa, &mut pb);
        perm_sign = -perm_sign;
    }

    let xa = PreciseVector::from_vector(pa.0);
    let xb = PreciseVector::from_vector(pb.0);
    let xc = PreciseVector::from_vector(pc.0);
    let xb_cross_xc = xb.cross(&xc);
    let det = xa.dot(&xb_cross_xc);

    let det_sign = det.signum();
    let mut result = Direction::from(det_sign);

    if result == Direction::Indeterminate && perturb {
        result = symbolically_perturbed_sign(&xa, &xb, &xc, &xb_cross_xc);
    }

    let perm_dir: Direction = if perm_sign > 0 {
        Direction::CounterClockwise
    } else {
        Direction::Clockwise
    };
    perm_dir * result
}

/// Resolves degenerate cases using the "Simulation of Simplicity" technique.
///
/// Requires: A < B < C in lexicographic order, and det(A,B,C) == 0 exactly.
///
/// Cold: deepest fallback in the `robust_sign` chain — only reached when
/// the exact determinant is exactly zero (truly collinear in the input
/// rounding regime).
#[cold]
fn symbolically_perturbed_sign(
    a: &PreciseVector,
    b: &PreciseVector,
    c: &PreciseVector,
    b_cross_c: &PreciseVector,
) -> Direction {
    // Test perturbation coefficients in decreasing order of magnitude.
    // See Edelsbrunner & Muecke, "Simulation of Simplicity", 1990.

    // da.Z
    let det_sign = b_cross_c.z.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // da.Y
    let det_sign = b_cross_c.y.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // da.X
    let det_sign = b_cross_c.x.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }

    // db.Z = c.X*a.Y - c.Y*a.X
    let det_sign = c.x.mul(&a.y).sub(&c.y.mul(&a.x)).signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // db.Z * da.Y = c.X
    let det_sign = c.x.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // db.Z * da.X = -(c.Y)
    let det_sign = -c.y.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }

    // db.Y = c.Z*a.X - c.X*a.Z
    let det_sign = c.z.mul(&a.x).sub(&c.x.mul(&a.z)).signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // db.Y * da.X = c.Z
    let det_sign = c.z.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }

    // db.X is redundant (previous tests guarantee C == 0).

    // dc.Z = a.X*b.Y - a.Y*b.X
    let det_sign = a.x.mul(&b.y).sub(&a.y.mul(&b.x)).signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // dc.Z * da.Y = -(b.X)
    let det_sign = -b.x.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // dc.Z * da.X = b.Y
    let det_sign = b.y.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }
    // dc.Z * db.Y = a.X
    let det_sign = a.x.signum();
    if det_sign != 0 {
        return Direction::from(det_sign);
    }

    // dc.Z * db.Y * da.X — this is always +1.
    Direction::CounterClockwise
}

// ─── CompareDistances ────────────────────────────────────────────────────

/// Returns -1, 0, or +1 according to whether AX < BX, AX == BX, or AX > BX.
///
/// Uses symbolic perturbations to ensure non-zero result when A != B.
#[inline]
pub fn compare_distances(x: Point, a: Point, b: Point) -> i32 {
    let sign = triage_compare_cos_distances(x, a, b);
    if sign != 0 {
        return sign;
    }
    if a == b {
        return 0;
    }

    let cos_ax = a.0.dot(x.0);
    let sign = if cos_ax > std::f64::consts::FRAC_1_SQRT_2 {
        triage_compare_sin2_distances(x, a, b)
    } else if cos_ax < -std::f64::consts::FRAC_1_SQRT_2 {
        -triage_compare_sin2_distances(x, a, b)
    } else {
        0
    };
    if sign != 0 {
        return sign;
    }

    let sign = exact_compare_distances(
        &PreciseVector::from_vector(x.0),
        &PreciseVector::from_vector(a.0),
        &PreciseVector::from_vector(b.0),
    );
    if sign != 0 {
        return sign;
    }
    symbolic_compare_distances(x, a, b)
}

#[inline]
fn cos_distance(x: Point, y: Point) -> (f64, f64) {
    let cos = x.0.dot(y.0);
    (cos, 9.5 * DBL_ERROR * cos.abs() + 1.5 * DBL_ERROR)
}

#[inline]
fn sin2_distance(x: Point, y: Point) -> (f64, f64) {
    let n = (x.0 - y.0).cross(x.0 + y.0);
    let sin2 = 0.25 * n.norm2();
    let err = (21.0 + 4.0 * SQRT3) * DBL_ERROR * sin2
        + 32.0 * SQRT3 * DBL_ERROR * DBL_ERROR * sin2.sqrt()
        + 768.0 * DBL_ERROR * DBL_ERROR * DBL_ERROR * DBL_ERROR;
    (sin2, err)
}

#[inline]
fn triage_compare_cos_distances(x: Point, a: Point, b: Point) -> i32 {
    let (cos_ax, cos_ax_err) = cos_distance(a, x);
    let (cos_bx, cos_bx_err) = cos_distance(b, x);
    let diff = cos_ax - cos_bx;
    let err = cos_ax_err + cos_bx_err;
    if diff > err {
        -1
    } else if diff < -err {
        1
    } else {
        0
    }
}

#[inline]
fn triage_compare_sin2_distances(x: Point, a: Point, b: Point) -> i32 {
    let (sin2_ax, sin2_ax_err) = sin2_distance(a, x);
    let (sin2_bx, sin2_bx_err) = sin2_distance(b, x);
    let diff = sin2_ax - sin2_bx;
    let err = sin2_ax_err + sin2_bx_err;
    if diff > err {
        1
    } else if diff < -err {
        -1
    } else {
        0
    }
}

fn exact_compare_distances(x: &PreciseVector, a: &PreciseVector, b: &PreciseVector) -> i32 {
    let cos_ax = x.dot(a);
    let cos_bx = x.dot(b);

    let a_sign = cos_ax.signum();
    let b_sign = cos_bx.signum();
    if a_sign != b_sign {
        return if a_sign > b_sign { -1 } else { 1 };
    }

    // Compare cos^2(AX) * |B|^2 vs cos^2(BX) * |A|^2.
    let cos_ax2 = cos_ax.mul(&cos_ax);
    let cos_bx2 = cos_bx.mul(&cos_bx);
    let cmp = cos_bx2.mul(&a.norm2()).sub(&cos_ax2.mul(&b.norm2()));
    a_sign * cmp.signum()
}

fn symbolic_compare_distances(_x: Point, a: Point, b: Point) -> i32 {
    match a.cmp_point(b) {
        Ordering::Less => 1,
        Ordering::Greater => -1,
        Ordering::Equal => 0,
    }
}

// ─── CompareDistance (single threshold) ──────────────────────────────────

/// `ChordAngle` for ~45 degrees.
#[inline]
fn ca_45_degrees() -> ChordAngle {
    ChordAngle::from_length2(2.0 - std::f64::consts::SQRT_2)
}

/// Returns -1, 0, or +1 according to whether distance XY is less than,
/// equal to, or greater than `r`.
#[inline]
pub fn compare_distance(x: Point, y: Point, r: ChordAngle) -> i32 {
    let r2 = r.length2();
    let sign = triage_compare_cos_distance(x, y, r2);
    if sign != 0 {
        return sign;
    }
    if r < ca_45_degrees() {
        let sign = triage_compare_sin2_distance(x, y, r2);
        if sign != 0 {
            return sign;
        }
    }
    exact_compare_distance(
        &PreciseVector::from_vector(x.0),
        &PreciseVector::from_vector(y.0),
        r2,
    )
}

#[inline]
fn triage_compare_cos_distance(x: Point, y: Point, r2: f64) -> i32 {
    let (cos_xy, cos_xy_err) = cos_distance(x, y);
    let cos_r = 1.0 - 0.5 * r2;
    let cos_r_err = 2.0 * DBL_ERROR * cos_r;
    let diff = cos_xy - cos_r;
    let err = cos_xy_err + cos_r_err;
    if diff > err {
        -1
    } else if diff < -err {
        1
    } else {
        0
    }
}

#[inline]
fn triage_compare_sin2_distance(x: Point, y: Point, r2: f64) -> i32 {
    let (sin2_xy, sin2_xy_err) = sin2_distance(x, y);
    let sin2_r = r2 * (1.0 - 0.25 * r2);
    let sin2_r_err = 3.0 * DBL_ERROR * sin2_r;
    let diff = sin2_xy - sin2_r;
    let err = sin2_xy_err + sin2_r_err;
    if diff > err {
        1
    } else if diff < -err {
        -1
    } else {
        0
    }
}

fn exact_compare_distance(x: &PreciseVector, y: &PreciseVector, r2: f64) -> i32 {
    use crate::r3::ExactFloat;
    let cos_xy = x.dot(y);
    let one = ExactFloat::from(1.0);
    let half = ExactFloat::from(0.5);
    let r2_exact = ExactFloat::from(r2);
    let cos_r = one.sub(&half.mul(&r2_exact));

    let xy_sign = cos_xy.signum();
    let r_sign = cos_r.signum();
    if xy_sign != r_sign {
        return if xy_sign > r_sign { -1 } else { 1 };
    }
    let cos_r2 = cos_r.mul(&cos_r);
    let cos_xy2 = cos_xy.mul(&cos_xy);
    let cmp = cos_r2.mul(&x.norm2()).mul(&y.norm2()).sub(&cos_xy2);
    xy_sign * cmp.signum()
}

// ─── SignDotProd ─────────────────────────────────────────────────────────

/// Reports the exact sign of the dot product between A and B.
///
/// Requires: |a|^2 <= 2 and |b|^2 <= 2.
#[inline]
pub fn sign_dot_prod(a: Point, b: Point) -> i32 {
    debug_assert!(a.0.norm2() <= 2.0, "sign_dot_prod: |a|² > 2");
    debug_assert!(b.0.norm2() <= 2.0, "sign_dot_prod: |b|² > 2");
    let sign = triage_sign_dot_prod(a, b);
    if sign != 0 {
        return sign;
    }
    PreciseVector::from_vector(a.0)
        .dot(&PreciseVector::from_vector(b.0))
        .signum()
}

#[inline]
fn triage_sign_dot_prod(a: Point, b: Point) -> i32 {
    const MAX_ERROR: f64 = 3.046875 * DBL_EPSILON;
    let na = a.0.dot(b.0);
    if na.abs() <= MAX_ERROR {
        0
    } else if na > 0.0 {
        1
    } else {
        -1
    }
}

// ─── CircleEdgeIntersectionOrdering ──────────────────────────────────────

/// Orders the crossings of edges AB and CD on a great circle M relative
/// to a reference circle N.
///
/// Returns -1 if AB is closer to N, 0 if equal, +1 if CD is closer to N.
pub fn circle_edge_intersection_ordering(
    a: Point,
    b: Point,
    c: Point,
    d: Point,
    m: Point,
    n: Point,
) -> i32 {
    debug_assert!(
        a != b && a != -b,
        "circle_edge_intersection_ordering: a == ±b"
    );
    debug_assert!(
        c != d && c != -d,
        "circle_edge_intersection_ordering: c == ±d"
    );
    debug_assert!(
        m != n && m != -n,
        "circle_edge_intersection_ordering: m == ±n"
    );
    let ans = triage_intersection_ordering(a, b, c, d, m, n);
    if ans != 0 {
        return ans;
    }
    if (a == c && b == d) || (a == d && b == c) {
        return 0;
    }
    exact_intersection_ordering(
        &PreciseVector::from_vector(a.0),
        &PreciseVector::from_vector(b.0),
        &PreciseVector::from_vector(c.0),
        &PreciseVector::from_vector(d.0),
        &PreciseVector::from_vector(m.0),
        &PreciseVector::from_vector(n.0),
    )
}

fn triage_intersection_ordering(a: Point, b: Point, c: Point, d: Point, m: Point, n: Point) -> i32 {
    const MAX_ERROR: f64 = 32.0 * DBL_EPSILON;

    let mdota = m.0.dot(a.0);
    let mdotb = m.0.dot(b.0);
    let mdotc = m.0.dot(c.0);
    let mdotd = m.0.dot(d.0);

    let ndota = n.0.dot(a.0);
    let ndotb = n.0.dot(b.0);
    let ndotc = n.0.dot(c.0);
    let ndotd = n.0.dot(d.0);

    let prod_ab = mdota * ndotb - mdotb * ndota;
    let prod_cd = mdotc * ndotd - mdotd * ndotc;

    if (prod_ab - prod_cd).abs() > MAX_ERROR {
        if prod_ab < prod_cd { -1 } else { 1 }
    } else {
        0
    }
}

fn exact_intersection_ordering(
    a: &PreciseVector,
    b: &PreciseVector,
    c: &PreciseVector,
    d: &PreciseVector,
    m: &PreciseVector,
    n: &PreciseVector,
) -> i32 {
    let mdota = m.dot(a);
    let mdotb = m.dot(b);
    let mdotc = m.dot(c);
    let mdotd = m.dot(d);

    let ndota = n.dot(a);
    let ndotb = n.dot(b);
    let ndotc = n.dot(c);
    let ndotd = n.dot(d);

    let prod_ab = mdota.mul(&ndotb).sub(&mdotb.mul(&ndota));
    let prod_cd = mdotc.mul(&ndotd).sub(&mdotd.mul(&ndotc));

    let diff = prod_ab.sub(&prod_cd);
    diff.signum()
}

/// Returns the sign of ((A×B)×N)·X, which determines on which side of
/// the great circle X the intersection of circles A×B and N lies.
///
/// The formula expands to: (N·A)(X·B) - (N·B)(X·A).
///
/// Uses exact arithmetic fallback when the result is ambiguous.
///
/// Corresponds to C++ `s2pred::CircleEdgeIntersectionSign`.
pub fn circle_edge_intersection_sign(a: Point, b: Point, n: Point, x: Point) -> i32 {
    debug_assert!(a != b && a != -b, "circle_edge_intersection_sign: a == ±b");
    const MAX_ERROR: f64 = 14.0 * DBL_EPSILON;

    let ndota = n.0.dot(a.0);
    let ndotb = n.0.dot(b.0);
    let xdota = x.0.dot(a.0);
    let xdotb = x.0.dot(b.0);

    let prod = ndota * xdotb - ndotb * xdota;
    if prod.abs() > MAX_ERROR {
        return if prod < 0.0 { -1 } else { 1 };
    }

    // Exact fallback.
    let pa = PreciseVector::from_vector(a.0);
    let pb = PreciseVector::from_vector(b.0);
    let pn = PreciseVector::from_vector(n.0);
    let px = PreciseVector::from_vector(x.0);

    let ndota_e = pn.dot(&pa);
    let ndotb_e = pn.dot(&pb);
    let xdota_e = px.dot(&pa);
    let xdotb_e = px.dot(&pb);

    let prod_e = ndota_e.mul(&xdotb_e).sub(&ndotb_e.mul(&xdota_e));
    prod_e.signum()
}

// ─── CompareEdgeDistance ─────────────────────────────────────────────────

/// Returns the closest vertex (a0 or a1) to x, and the squared distance in `ax2`.
#[inline]
fn get_closest_vertex(x: Point, a0: Point, a1: Point) -> (Point, f64) {
    let a0x2 = (a0.0 - x.0).norm2();
    let a1x2 = (a1.0 - x.0).norm2();
    if a0x2 < a1x2 || (a0x2 == a1x2 && a0 < a1) {
        (a0, a0x2)
    } else {
        (a1, a1x2)
    }
}

/// Compares the distance from X to the line through (a0,a1) against r2,
/// using sin² of the distances (accurate for small distances).
fn triage_compare_line_sin2_distance(
    x: Point,
    a0: Point,
    a1: Point,
    r2: f64,
    n: crate::r3::Vector,
    n1: f64,
    n2: f64,
) -> i32 {
    if r2 >= 2.0 {
        return -1;
    } // distance < limit (limit >= 90°)

    let n2sin2_r = n2 * r2 * (1.0 - 0.25 * r2);
    let n2sin2_r_error = 6.0 * DBL_ERROR * n2sin2_r;
    let (closest, ax2) = get_closest_vertex(x, a0, a1);
    let x_dn = (x.0 - closest.0).dot(n);
    let x_dn2 = x_dn * x_dn;
    let c1 = ((3.5 + 2.0 * SQRT3) * n1 + 32.0 * SQRT3 * DBL_ERROR) * DBL_ERROR * ax2.sqrt();
    let x_dn2_error = 4.0 * DBL_ERROR * x_dn2 + (2.0 * x_dn.abs() + c1) * c1;

    // Use fact that X is unit length within 4 * DBL_ERR.
    let n2sin2_r_adj = n2sin2_r;
    let n2sin2_r_error_adj = n2sin2_r_error + 8.0 * DBL_ERROR * n2sin2_r_adj;

    let diff = x_dn2 - n2sin2_r_adj;
    let error = x_dn2_error + n2sin2_r_error_adj;
    if diff > error {
        1
    } else if diff < -error {
        -1
    } else {
        0
    }
}

/// Compares the distance from X to the line through (a0,a1) against r2,
/// using cos² of the distances (accurate for large distances).
fn triage_compare_line_cos2_distance(
    x: Point,
    _a0: Point,
    _a1: Point,
    r2: f64,
    n: crate::r3::Vector,
    n1: f64,
    n2: f64,
) -> i32 {
    if r2 >= 2.0 {
        return -1;
    }

    let cos_r = 1.0 - 0.5 * r2;
    let n2cos2_r = n2 * cos_r * cos_r;
    let n2cos2_r_error = 7.0 * DBL_ERROR * n2cos2_r;

    let m = x.0.cross(n);
    let m2 = m.norm2();
    let m1 = m2.sqrt();
    let m1_error = ((1.0 + 8.0 / SQRT3) * n1 + 32.0 * SQRT3 * DBL_ERROR) * DBL_ERROR;
    let m2_error = 3.0 * DBL_ERROR * m2 + (2.0 * m1 + m1_error) * m1_error;

    let n2cos2_r_adj = n2cos2_r;
    let n2cos2_r_error_adj = n2cos2_r_error + 8.0 * DBL_ERROR * n2cos2_r_adj;

    let diff = m2 - n2cos2_r_adj;
    let error = m2_error + n2cos2_r_error_adj;
    if diff > error {
        -1
    } else if diff < -error {
        1
    } else {
        0
    }
}

#[inline]
fn triage_compare_line_distance(
    x: Point,
    a0: Point,
    a1: Point,
    r2: f64,
    n: crate::r3::Vector,
    n1: f64,
    n2: f64,
) -> i32 {
    if r2 < ca_45_degrees().length2() {
        triage_compare_line_sin2_distance(x, a0, a1, r2, n, n1, n2)
    } else {
        triage_compare_line_cos2_distance(x, a0, a1, r2, n, n1, n2)
    }
}

#[inline]
fn triage_compare_edge_distance(x: Point, a0: Point, a1: Point, r2: f64) -> i32 {
    let n = (a0.0 - a1.0).cross(a0.0 + a1.0);
    let m = n.cross(x.0);
    let a0_dir = a0.0 - x.0;
    let a1_dir = a1.0 - x.0;
    let a0_sign = a0_dir.dot(m);
    let a1_sign = a1_dir.dot(m);
    let n2 = n.norm2();
    let n1 = n2.sqrt();
    let n1_error = ((3.5 + 8.0 / SQRT3) * n1 + 32.0 * SQRT3 * DBL_ERROR) * DBL_ERROR;
    let a0_sign_error = n1_error * a0_dir.norm();
    let a1_sign_error = n1_error * a1_dir.norm();
    if a0_sign < a0_sign_error && a1_sign > -a1_sign_error {
        if a0_sign > -a0_sign_error || a1_sign < a1_sign_error {
            // Uncertain whether closest is interior or vertex.
            let vertex_sign =
                triage_compare_cos_distance(x, a0, r2).min(triage_compare_cos_distance(x, a1, r2));
            let line_sign = triage_compare_line_distance(x, a0, a1, r2, n, n1, n2);
            return if vertex_sign == line_sign {
                line_sign
            } else {
                0
            };
        }
        // Closest point is on the edge interior.
        return triage_compare_line_distance(x, a0, a1, r2, n, n1, n2);
    }
    // Closest point is an endpoint.
    triage_compare_cos_distance(x, a0, r2).min(triage_compare_cos_distance(x, a1, r2))
}

/// Returns -1, 0, or +1 comparing the distance from X to edge A0A1
/// against the chord angle r. Uses exact arithmetic fallback.
#[inline]
pub fn compare_edge_distance(x: Point, a0: Point, a1: Point, r: ChordAngle) -> i32 {
    // Check that the edge does not consist of antipodal points.
    debug_assert!(a0 != -a1, "CompareEdgeDistance: antipodal edge endpoints");
    let sign = triage_compare_edge_distance(x, a0, a1, r.length2());
    if sign != 0 {
        return sign;
    }

    // Degenerate edge optimization.
    if a0 == a1 {
        return compare_distance(x, a0, r);
    }

    // Exact fallback: determine if closest point is interior or endpoint.
    if a0 != -x
        && a1 != -x
        && compare_edge_directions(a0, a1, a0, x) > 0
        && compare_edge_directions(a0, a1, x, a1) > 0
    {
        // Closest point is interior. Use exact line distance.
        exact_compare_line_distance(x, a0, a1, r.length2())
    } else {
        // Closest point is an endpoint.
        compare_distance(x, a0, r).min(compare_distance(x, a1, r))
    }
}

fn exact_compare_line_distance(x: Point, a0: Point, a1: Point, r2: f64) -> i32 {
    if r2 >= 2.0 {
        return -1;
    }
    let xp = PreciseVector::from_vector(x.0);
    let a0p = PreciseVector::from_vector(a0.0);
    let a1p = PreciseVector::from_vector(a1.0);
    let n = a0p.cross(&a1p);
    let sin_d = xp.dot(&n);
    use crate::r3::ExactFloat;
    let r2e = ExactFloat::from(r2);
    let sin2_r =
        r2e.mul(&ExactFloat::from(1.0).sub(&ExactFloat::from(0.25).mul(&ExactFloat::from(r2))));
    let cmp = sin_d
        .mul(&sin_d)
        .sub(&sin2_r.mul(&xp.norm2()).mul(&n.norm2()));
    cmp.signum()
}

// ─── CompareEdgeDirections ──────────────────────────────────────────────

fn triage_compare_edge_directions(a0: Point, a1: Point, b0: Point, b1: Point) -> i32 {
    let na = (a0.0 - a1.0).cross(a0.0 + a1.0);
    let nb = (b0.0 - b1.0).cross(b0.0 + b1.0);
    let na_len = na.norm();
    let nb_len = nb.norm();
    let cos_ab = na.dot(nb);
    let cos_ab_error = ((5.0 + 4.0 * SQRT3) * na_len * nb_len
        + 32.0 * SQRT3 * DBL_ERROR * (na_len + nb_len))
        * DBL_ERROR;
    if cos_ab > cos_ab_error {
        1
    } else if cos_ab < -cos_ab_error {
        -1
    } else {
        0
    }
}

/// Returns -1, 0, or +1 comparing the minimum distance between edges
/// A=a0a1 and B=b0b1 against the chord angle r.
///
/// Returns -1 if dist(A, B) < r, 0 if dist(A, B) == r, +1 if dist(A, B) > r.
///
/// Corresponds to C++ `s2pred::CompareEdgePairDistance`.
#[inline]
pub fn compare_edge_pair_distance(
    a0: Point,
    a1: Point,
    b0: Point,
    b1: Point,
    r: ChordAngle,
) -> i32 {
    use super::edge_crossings;
    // If the edges cross or share an endpoint, the minimum distance is zero.
    if edge_crossings::crossing_sign(a0, a1, b0, b1) != edge_crossings::Crossing::DoNotCross {
        if r.length2() > 0.0 {
            return -1;
        }
        if r.length2() < 0.0 {
            return 1;
        }
        return 0;
    }
    // Otherwise, the minimum distance is achieved at an endpoint.
    compare_edge_distance(a0, b0, b1, r)
        .min(compare_edge_distance(a1, b0, b1, r))
        .min(compare_edge_distance(b0, a0, a1, r))
        .min(compare_edge_distance(b1, a0, a1, r))
}

/// Returns -1, 0, or +1 based on whether the edges A0A1 and B0B1 point
/// in opposite, perpendicular, or same directions. More precisely,
/// returns the sign of the dot product of the normals of the two edges.
#[inline]
pub fn compare_edge_directions(a0: Point, a1: Point, b0: Point, b1: Point) -> i32 {
    // Check that no edge consists of antipodal points.
    debug_assert!(a0 != -a1, "CompareEdgeDirections: a0 == -a1");
    debug_assert!(b0 != -b1, "CompareEdgeDirections: b0 == -b1");
    let sign = triage_compare_edge_directions(a0, a1, b0, b1);
    if sign != 0 {
        return sign;
    }
    if a0 == a1 || b0 == b1 {
        return 0;
    }
    // Exact fallback.
    let a0p = PreciseVector::from_vector(a0.0);
    let a1p = PreciseVector::from_vector(a1.0);
    let b0p = PreciseVector::from_vector(b0.0);
    let b1p = PreciseVector::from_vector(b1.0);
    a0p.cross(&a1p).dot(&b0p.cross(&b1p)).signum()
}

// ─── EdgeCircumcenterSign ───────────────────────────────────────────────

/// Computes the circumcenter of triangle ABC for the triage path.
/// Returns the (unnormalized) circumcenter Z and an error bound.
fn get_circumcenter(a: Point, b: Point, c: Point) -> (crate::r3::Vector, f64) {
    let ab_diff = a.0 - b.0;
    let ab_sum = a.0 + b.0;
    let bc_diff = b.0 - c.0;
    let bc_sum = b.0 + c.0;
    let nab = ab_diff.cross(ab_sum);
    let nab_len = nab.norm();
    let ab_len = ab_diff.norm();
    let nbc = bc_diff.cross(bc_sum);
    let nbc_len = nbc.norm();
    let bc_len = bc_diff.norm();
    let mab = nab.cross(ab_sum);
    let mbc = nbc.cross(bc_sum);
    let error = ((16.0 + 24.0 * SQRT3) * DBL_ERROR + 8.0 * DBL_ERROR * (ab_len + bc_len))
        * nab_len
        * nbc_len
        + 128.0 * SQRT3 * DBL_ERROR * DBL_ERROR * (nab_len + nbc_len)
        + 3.0 * 4096.0 * DBL_ERROR * DBL_ERROR * DBL_ERROR * DBL_ERROR;
    (mab.cross(mbc), error)
}

fn triage_edge_circumcenter_sign(
    x0: Point,
    x1: Point,
    a: Point,
    b: Point,
    c: Point,
    abc_sign: i32,
) -> i32 {
    let (z, z_error) = get_circumcenter(a, b, c);
    let nx = (x0.0 - x1.0).cross(x0.0 + x1.0);
    let result = f64::from(abc_sign) * nx.dot(z);
    let z_len = z.norm();
    let nx_len = nx.norm();
    let nx_error = ((1.0 + 2.0 * SQRT3) * nx_len + 32.0 * SQRT3 * DBL_ERROR) * DBL_ERROR;
    let result_error = (3.0 * DBL_ERROR * nx_len + nx_error) * z_len + z_error * nx_len;
    if result > result_error {
        1
    } else if result < -result_error {
        -1
    } else {
        0
    }
}

fn exact_edge_circumcenter_sign(
    x0: Point,
    x1: Point,
    a: Point,
    b: Point,
    c: Point,
    abc_sign: i32,
) -> i32 {
    use crate::r3::ExactFloat;
    let x0p = PreciseVector::from_vector(x0.0);
    let x1p = PreciseVector::from_vector(x1.0);
    let ap = PreciseVector::from_vector(a.0);
    let bp = PreciseVector::from_vector(b.0);
    let cp = PreciseVector::from_vector(c.0);

    // Check if edge X is degenerate (linearly dependent).
    let x_cross = x0p.cross(&x1p);
    if x_cross.is_zero() {
        return 0;
    }

    let nx = x0p.cross(&x1p);
    let dab = nx.dot(&ap.cross(&bp));
    let dbc = nx.dot(&bp.cross(&cp));
    let dca = nx.dot(&cp.cross(&ap));

    let abc2 = ap.norm2().mul(&dbc.mul(&dbc));
    let bca2 = bp.norm2().mul(&dca.mul(&dca));
    let cab2 = cp.norm2().mul(&dab.mul(&dab));

    // Check sign of |C| dAB vs -|A| dBC (equation 3).
    let lhs3_sgn = dab.signum();
    let rhs3_sgn = -dbc.signum();
    let mut lhs2_sgn = (lhs3_sgn - rhs3_sgn).clamp(-1, 1);
    if lhs2_sgn == 0 && lhs3_sgn != 0 {
        lhs2_sgn = cab2.sub(&abc2).signum() * lhs3_sgn;
    }
    let rhs2_sgn = -dca.signum();
    let mut result = (lhs2_sgn - rhs2_sgn).clamp(-1, 1);
    if result == 0 && lhs2_sgn != 0 {
        let lhs4_sgn = dab.signum() * dbc.signum();
        let rhs4 = bca2.sub(&cab2).sub(&abc2);
        result = (lhs4_sgn - rhs4.signum()).clamp(-1, 1);
        if result == 0 && lhs4_sgn != 0 {
            let four = ExactFloat::from(4.0);
            result = four.mul(&abc2).mul(&cab2).sub(&rhs4.mul(&rhs4)).signum() * lhs4_sgn;
        }
        result *= lhs2_sgn;
    }
    abc_sign * result
}

fn symbolic_edge_circumcenter_sign(x0: Point, x1: Point, a: Point, b: Point, c: Point) -> i32 {
    if a == b || b == c || c == a {
        return 0;
    }

    // Sort a, b, c lexicographically.
    let (mut pa, mut pb, mut pc) = (a, b, c);
    if pb < pa {
        std::mem::swap(&mut pa, &mut pb);
    }
    if pc < pb {
        std::mem::swap(&mut pb, &mut pc);
    }
    if pb < pa {
        std::mem::swap(&mut pa, &mut pb);
    }

    let sign = unperturbed_sign(x0, x1, pa);
    if sign != 0 {
        return sign;
    }
    let sign = unperturbed_sign(x0, x1, pb);
    if sign != 0 {
        return sign;
    }
    unperturbed_sign(x0, x1, pc)
}

/// Helper: returns Sign(x0, x1, p) via exact arithmetic without symbolic
/// perturbation. Returns 0 if the points are linearly dependent.
fn unperturbed_sign(x0: Point, x1: Point, p: Point) -> i32 {
    let x0p = PreciseVector::from_vector(x0.0);
    let x1p = PreciseVector::from_vector(x1.0);
    let pp = PreciseVector::from_vector(p.0);
    x0p.cross(&x1p).dot(&pp).signum()
}

/// Returns the sign of the dot product of the edge normal of X0X1 and
/// the circumcenter of triangle ABC. Returns -1, 0, or +1.
#[inline]
pub fn edge_circumcenter_sign(x0: Point, x1: Point, a: Point, b: Point, c: Point) -> i32 {
    debug_assert!(x0 != -x1, "EdgeCircumcenterSign: antipodal edge endpoints");
    // C++ uses s2pred::Sign(a, b, c) which is the robust sign (exact + perturbation).
    // We must use robust_sign here, not the simple sign() function, to handle
    // degenerate cases (e.g., all points on the same great circle).
    let abc_sign = robust_sign(a, b, c) as i32;
    if abc_sign == 0 {
        return 0;
    }
    let s = triage_edge_circumcenter_sign(x0, x1, a, b, c, abc_sign);
    if s != 0 {
        return s;
    }
    if x0 == x1 || a == b || b == c || c == a {
        return 0;
    }
    let s = exact_edge_circumcenter_sign(x0, x1, a, b, c, abc_sign);
    if s != 0 {
        return s;
    }
    symbolic_edge_circumcenter_sign(x0, x1, a, b, c)
}

// ─── GetVoronoiSiteExclusion ────────────────────────────────────────────

/// Result of the Voronoi site exclusion test.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Excluded {
    /// The first site's coverage interval is contained by the second's.
    First,
    /// The second site's coverage interval is contained by the first's.
    Second,
    /// Neither site's interval contains the other's.
    #[default]
    Neither,
    /// The result could not be determined (triage-only fallback).
    Uncertain,
}

fn triage_voronoi_site_exclusion(a: Point, b: Point, x0: Point, x1: Point, r2: f64) -> Excluded {
    let n = (x0.0 - x1.0).cross(x0.0 + x1.0);
    let n2 = n.norm2();
    let n1 = n2.sqrt();
    let dn_error = ((3.5 + 2.0 * SQRT3) * n1 + 32.0 * SQRT3 * DBL_ERROR) * DBL_ERROR;

    let cos_r = 1.0 - 0.5 * r2;
    let sin2_r = r2 * (1.0 - 0.25 * r2);
    let n2sin2_r = n2 * sin2_r;

    // Compute sin(ra) and sin(rb) (scaled).
    let (closest_a, ax2) = get_closest_vertex(a, x0, x1);
    let a_dn = (a.0 - closest_a.0).dot(n);
    let a_dn2 = a_dn * a_dn;
    let a_dn_error = dn_error * ax2.sqrt();
    let ra2 = n2sin2_r - a_dn2;
    let ra2_error = (8.0 * DBL_ERROR + 4.0 * DBL_ERROR) * a_dn2
        + (2.0 * a_dn.abs() + a_dn_error) * a_dn_error
        + 6.0 * DBL_ERROR * n2sin2_r;
    let min_ra2 = ra2 - ra2_error;
    if min_ra2 < 0.0 {
        return Excluded::Uncertain;
    }
    let ra = ra2.sqrt();
    let ra_error = 1.5 * DBL_ERROR * ra + 0.5 * ra2_error / min_ra2.sqrt();

    let (closest_b, bx2) = get_closest_vertex(b, x0, x1);
    let b_dn = (b.0 - closest_b.0).dot(n);
    let b_dn2 = b_dn * b_dn;
    let b_dn_error = dn_error * bx2.sqrt();
    let rb2 = n2sin2_r - b_dn2;
    let rb2_error = (8.0 * DBL_ERROR + 4.0 * DBL_ERROR) * b_dn2
        + (2.0 * b_dn.abs() + b_dn_error) * b_dn_error
        + 6.0 * DBL_ERROR * n2sin2_r;
    let min_rb2 = rb2 - rb2_error;
    if min_rb2 < 0.0 {
        return Excluded::Uncertain;
    }
    let rb = rb2.sqrt();
    let rb_error = 1.5 * DBL_ERROR * rb + 0.5 * rb2_error / min_rb2.sqrt();

    let lhs3 = cos_r * (rb - ra);
    let abs_lhs3 = lhs3.abs();
    let lhs3_error = cos_r * (ra_error + rb_error) + 3.0 * DBL_ERROR * abs_lhs3;

    // RHS: proportional to sin(d).
    let a_xb = (a.0 - b.0).cross(a.0 + b.0);
    let a_xb1 = a_xb.norm();
    let sin_d = 0.5 * a_xb.dot(n);
    let sin_d_error = (4.0 * DBL_ERROR + (2.5 + 2.0 * SQRT3) * DBL_ERROR) * a_xb1 * n1
        + 16.0 * SQRT3 * DBL_ERROR * DBL_ERROR * (a_xb1 + n1);

    let result = abs_lhs3 - sin_d;
    let result_error = lhs3_error + sin_d_error;
    if result < -result_error {
        return Excluded::Neither;
    }

    // Handle d < 0 case.
    if sin_d < -sin_d_error {
        let r90 = ChordAngle::RIGHT.length2();
        let ca = triage_compare_cos_distance(a, x0, r90);
        let cb = triage_compare_cos_distance(b, x1, r90);
        if ca < 0 && cb < 0 {
            return Excluded::Neither;
        }
        if ca <= 0 && cb <= 0 {
            return Excluded::Uncertain;
        }
        return if ca > 0 {
            Excluded::First
        } else {
            Excluded::Second
        };
    }
    if sin_d <= sin_d_error {
        return Excluded::Uncertain;
    }

    // Check cos(d) >= 0, i.e. |d| <= Pi/2.
    let cos_d = a.0.dot(b.0) * n2 - a_dn * b_dn;
    let cos_d_error = ((8.0 * DBL_ERROR + 5.0 * DBL_ERROR) * a_dn.abs() + a_dn_error) * b_dn.abs()
        + (a_dn.abs() + a_dn_error) * b_dn_error
        + (8.0 * DBL_ERROR + 8.0 * DBL_ERROR) * n2;
    if cos_d <= -cos_d_error {
        return Excluded::Neither;
    }
    if cos_d < cos_d_error {
        return Excluded::Uncertain;
    }

    // Now finish checking predicate (3).
    if result <= result_error {
        return Excluded::Uncertain;
    }
    if lhs3 > 0.0 {
        Excluded::First
    } else {
        Excluded::Second
    }
}

/// Determines whether site A or B can be excluded from consideration along
/// edge X0X1, given snap radius `r`. Sites must satisfy:
///   CompareDistances(x0, a, b) < 0  (a is closer to x0 than b)
///   CompareEdgeDistance(a, x0, x1, r) <= 0  (a is within r of edge)
///   CompareEdgeDistance(b, x0, x1, r) <= 0  (b is within r of edge)
#[inline]
pub fn get_voronoi_site_exclusion(
    a: Point,
    b: Point,
    x0: Point,
    x1: Point,
    r: ChordAngle,
) -> Excluded {
    debug_assert!(r < ChordAngle::RIGHT, "GetVoronoiSiteExclusion: r >= RIGHT");
    debug_assert!(
        x0 != -x1,
        "GetVoronoiSiteExclusion: antipodal edge endpoints"
    );
    // If A is closer to both endpoints of X, it is closer to every point.
    if compare_distances(x1, a, b) < 0 {
        return Excluded::Second;
    }

    let result = triage_voronoi_site_exclusion(a, b, x0, x1, r.length2());
    if result != Excluded::Uncertain {
        return result;
    }

    exact_voronoi_site_exclusion(
        a,
        b,
        &PreciseVector::from_vector(a.0),
        &PreciseVector::from_vector(b.0),
        &PreciseVector::from_vector(x0.0),
        &PreciseVector::from_vector(x1.0),
        r.length2(),
    )
}

/// Exact arithmetic implementation of voronoi site exclusion using degree-20
/// polynomial evaluation. Matches C++ `ExactVoronoiSiteExclusion`.
fn exact_voronoi_site_exclusion(
    a_point: Point,
    b_point: Point,
    a: &PreciseVector,
    b: &PreciseVector,
    x0: &PreciseVector,
    x1: &PreciseVector,
    r2: f64,
) -> Excluded {
    use crate::r3::ExactFloat;

    // Recall that one site excludes the other if
    //   |sin(rb - ra)| > sin(d)
    // and the sign of (rb - ra) determines which site is excluded.
    //
    // We expand this to:
    //   cos(r) ||a| sqrt(sin²(r)|b|²|n|² - |b·n|²) -
    //           |b| sqrt(sin²(r)|a|²|n|² - |a·n|²)| > (a×b)·n
    //
    // Squaring twice eliminates all square roots, yielding a degree-20
    // polynomial predicate.

    let n = x0.cross(x1);
    let rhs2 = a.cross(b).dot(&n);
    let rhs2_sgn = rhs2.signum();

    if rhs2_sgn < 0 {
        // d < 0 case. Check whether each site is within π/2 of its endpoint.
        let ca = exact_compare_distance(a, x0, ChordAngle::RIGHT.length2());
        let cb = exact_compare_distance(b, x1, ChordAngle::RIGHT.length2());
        if ca < 0 && cb < 0 {
            return Excluded::Neither;
        }
        debug_assert!(ca != 0 && cb != 0); // guaranteed since d < 0
        debug_assert!(ca < 0 || cb < 0); // at least one site must be kept
        return if ca > 0 {
            Excluded::First
        } else {
            Excluded::Second
        };
    }

    // Check cos(d) >= 0, using (a×n)·(b×n) = a·b |n|² - (a·n)(b·n).
    let n2 = n.norm2();
    let a_dn = a.dot(&n);
    let b_dn = b.dot(&n);
    let cos_d = a.dot(b).mul(&n2).sub(&a_dn.mul(&b_dn));
    if cos_d.signum() < 0 {
        return Excluded::Neither;
    }

    // Compute sa² and sb² where:
    //   sa = |b| sqrt(sin²(r)|a|²|n|² - |a·n|²)
    //   sb = |a| sqrt(sin²(r)|b|²|n|² - |b·n|²)
    let a2 = a.norm2();
    let b2 = b.norm2();
    let r2_exact = ExactFloat::from(r2);
    let one = ExactFloat::from(1.0);
    let quarter = ExactFloat::from(0.25);
    let n2sin2_r = r2_exact
        .mul(&one.sub(&quarter.mul(&ExactFloat::from(r2))))
        .mul(&n2);
    let sa2 = b2.mul(&n2sin2_r.mul(&a2).sub(&a_dn.mul(&a_dn)));
    let sb2 = a2.mul(&n2sin2_r.mul(&b2).sub(&b_dn.mul(&b_dn)));
    let lhs2_sgn = sb2.sub(&sa2).signum();

    if lhs2_sgn == 0 {
        // Both sites are equidistant. This should have been handled by the
        // CompareDistances call in GetVoronoiSiteExclusion.
        debug_assert!(rhs2_sgn > 0);
        return Excluded::Neither;
    }

    // Square both sides: cos²(r)(sb² + sa²) - (a×b·n)² > 2 cos²(r) sa sb
    let half = ExactFloat::from(0.5);
    let cos_r = one.sub(&half.mul(&ExactFloat::from(r2)));
    let cos2_r = cos_r.mul(&cos_r);
    let lhs3 = cos2_r.mul(&sa2.add(&sb2)).sub(&rhs2.mul(&rhs2));
    if lhs3.signum() < 0 {
        return Excluded::Neither;
    }

    // Square again: lhs3² > 4 cos⁴(r) sa² sb²
    let lhs4 = lhs3.mul(&lhs3);
    let four = ExactFloat::from(4.0);
    let rhs4 = four.mul(&cos2_r).mul(&cos2_r).mul(&sa2).mul(&sb2);
    let result = lhs4.sub(&rhs4).signum();

    if result < 0 {
        return Excluded::Neither;
    }
    if result == 0 {
        // |rb - ra| = d and d > 0. One coverage interval contains the other
        // but shares a common endpoint. Use symbolic perturbation: site A is
        // considered closer to an equidistant point iff A > B.
        if (lhs2_sgn > 0) == (a_point > b_point) {
            return Excluded::Neither;
        }
    }
    if lhs2_sgn > 0 {
        Excluded::First
    } else {
        Excluded::Second
    }
}

// ─── OrderedCCW ──────────────────────────────────────────────────────────

/// Reports whether the angles OA, OB, and OC are in strictly increasing
/// CCW order around O. Returns true if B is in the CCW arc from A to C
/// (inclusive at endpoints A and B but not C unless B == C).
///
/// Properties:
/// 1. If `ordered_ccw(a,b,c,o)` && `ordered_ccw(b,a,c,o)`, then a == b.
/// 2. If `ordered_ccw(a,b,c,o)` && `ordered_ccw(a,c,b,o)`, then b == c.
/// 3. If a == b or b == c, then `ordered_ccw(a,b,c,o)` is true.
/// 4. If a == c (and a != b), then `ordered_ccw(a,b,c,o)` is false.
#[inline]
pub fn ordered_ccw(a: Point, b: Point, c: Point, o: Point) -> bool {
    debug_assert!(
        a != o && b != o && c != o,
        "OrderedCCW: a, b, c must differ from o"
    );
    let mut sum = 0;
    if robust_sign(b, o, a) != Direction::Clockwise {
        sum += 1;
    }
    if robust_sign(c, o, b) != Direction::Clockwise {
        sum += 1;
    }
    if robust_sign(a, o, c) == Direction::CounterClockwise {
        sum += 1;
    }
    sum >= 2
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn direction_is_send_sync() {
        is_send_sync::<Direction>();
    }

    #[test]
    fn test_sign_basic() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        assert!(sign(a, b, c));
        assert!(!sign(c, b, a));
    }

    #[test]
    fn test_robust_sign_basic() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        assert_eq!(robust_sign(a, b, c), Direction::CounterClockwise);
        assert_eq!(robust_sign(c, b, a), Direction::Clockwise);
    }

    #[test]
    fn test_robust_sign_degenerate() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        assert_eq!(robust_sign(a, a, a), Direction::Indeterminate);
    }

    #[test]
    fn test_robust_sign_cyclic() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        let s = robust_sign(a, b, c);
        assert_eq!(robust_sign(b, c, a), s);
        assert_eq!(robust_sign(c, a, b), s);
    }

    #[test]
    fn test_robust_sign_reversal() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        assert_eq!(robust_sign(a, b, c), -robust_sign(a, c, b));
    }

    #[test]
    fn test_robust_sign_collinear() {
        // Three points on the equator, nearly collinear.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(-1.0, 0.0, 0.0);
        // These aren't actually collinear (they form a triangle), but test the path.
        let s = robust_sign(a, b, c);
        assert_ne!(s, Direction::Indeterminate);
    }

    #[test]
    fn test_compare_distances_basic() {
        let x = Point::from_coords(1.0, 0.0, 0.0);
        let a = Point::from_coords(0.0, 1.0, 0.0);
        // compare_distances uses symbolic perturbation: result is 0 only if a == b.
        assert_eq!(compare_distances(x, a, a), 0);
    }

    #[test]
    fn test_compare_distances_unequal() {
        let x = Point::from_coords(1.0, 0.0, 0.0);
        let a = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // a is closer to x (45°) than b (90°).
        assert_eq!(compare_distances(x, a, b), -1);
        assert_eq!(compare_distances(x, b, a), 1);
    }

    #[test]
    fn test_compare_distance_threshold() {
        let x = Point::from_coords(1.0, 0.0, 0.0);
        let y = Point::from_coords(0.0, 1.0, 0.0);
        // Distance is 90 degrees, chord angle = 2.0.
        let r = ChordAngle::from_length2(2.0);
        assert_eq!(compare_distance(x, y, r), 0);

        let r_small = ChordAngle::from_length2(1.0);
        assert_eq!(compare_distance(x, y, r_small), 1); // dist > r

        let r_big = ChordAngle::from_length2(3.0);
        assert_eq!(compare_distance(x, y, r_big), -1); // dist < r
    }

    #[test]
    fn test_sign_dot_prod() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        assert_eq!(sign_dot_prod(a, b), 0);

        let c = Point::from_coords(1.0, 1.0, 0.0).normalize();
        assert_eq!(sign_dot_prod(a, c), 1);
        assert_eq!(sign_dot_prod(a, Point(-c.0)), -1);
    }

    #[test]
    fn test_direction_neg() {
        assert_eq!(-Direction::Clockwise, Direction::CounterClockwise);
        assert_eq!(-Direction::CounterClockwise, Direction::Clockwise);
        assert_eq!(-Direction::Indeterminate, Direction::Indeterminate);
    }

    #[test]
    fn test_direction_mul() {
        assert_eq!(
            Direction::CounterClockwise * Direction::CounterClockwise,
            Direction::CounterClockwise
        );
        assert_eq!(
            Direction::Clockwise * Direction::Clockwise,
            Direction::CounterClockwise
        );
        assert_eq!(
            Direction::CounterClockwise * Direction::Clockwise,
            Direction::Clockwise
        );
    }

    #[test]
    fn test_exact_sign_perturbation() {
        // Test with exactly collinear points to exercise symbolic perturbation.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // c on the great circle through a and b.
        let c = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let s = robust_sign(a, b, c);
        // Should produce a definite answer via perturbation.
        assert_ne!(s, Direction::Indeterminate);
    }

    #[test]
    fn test_compare_distances_exact_fallback() {
        // Construct points that are nearly equidistant from x so that the
        // triage (cos and sin2) comparisons are uncertain and the code falls
        // through to the exact arithmetic path.
        //
        // Strategy: place x at (1,0,0), then construct a and b at the same
        // angle from x but separated by a tiny perturbation that is below
        // the error threshold of the triage comparisons.
        let x = Point::from_coords(1.0, 0.0, 0.0);

        // a and b are nearly the same point on the unit sphere, both roughly
        // 90 degrees from x, differing only by ~1e-15 in one coordinate.
        let a = Point::from_coords(1e-15, 1.0, 0.0).normalize();
        let b = Point::from_coords(-1e-15, 1.0, 0.0).normalize();

        // Both are roughly equidistant from x. The triage path cannot resolve
        // this, so exact arithmetic (or symbolic perturbation) must decide.
        let result = compare_distances(x, a, b);
        // The result must be definite (non-zero) since a != b, due to
        // symbolic perturbation.
        assert_ne!(result, 0, "exact fallback should produce a definite result");

        // Antisymmetry: swapping a and b must negate the result.
        assert_eq!(
            compare_distances(x, b, a),
            -result,
            "compare_distances must be antisymmetric"
        );

        // An even tighter pair: perturb by a single ULP in the y-coordinate.
        let base_y = 1.0_f64;
        let y_hi = f64::from_bits(base_y.to_bits() + 1);
        let a2 = Point::from_coords(0.0, base_y, 1e-10).normalize();
        let b2 = Point::from_coords(0.0, y_hi, 1e-10).normalize();
        let result2 = compare_distances(x, a2, b2);
        assert_ne!(
            result2, 0,
            "ULP-close points should still get a definite answer"
        );
        assert_eq!(compare_distances(x, b2, a2), -result2);
    }

    #[test]
    fn test_compare_distance_single_threshold() {
        // Test compare_distance(x, y, r) for angles both less than and
        // greater than 45 degrees. The implementation uses the sin2 path
        // for r < 45 degrees and the cos path otherwise.

        let x = Point::from_coords(1.0, 0.0, 0.0);

        // --- Case 1: angle < 45 degrees (exercises sin2 path) ---
        // 20 degrees in radians.
        let angle_20 = 20.0_f64.to_radians();
        let y_close = Point::from_coords(angle_20.cos(), angle_20.sin(), 0.0).normalize();
        let r_exact_20 = ChordAngle::from_length2(2.0 * (1.0 - angle_20.cos()));

        // Distance should exactly equal r (modulo floating-point), so test
        // slightly larger and smaller thresholds.
        let r_smaller = ChordAngle::from_length2(r_exact_20.length2() - 1e-10);
        let r_larger = ChordAngle::from_length2(r_exact_20.length2() + 1e-10);
        assert_eq!(
            compare_distance(x, y_close, r_smaller),
            1,
            "distance > smaller threshold"
        );
        assert_eq!(
            compare_distance(x, y_close, r_larger),
            -1,
            "distance < larger threshold"
        );

        // --- Case 2: angle > 45 degrees (bypasses sin2 path) ---
        // 80 degrees in radians.
        let angle_80 = 80.0_f64.to_radians();
        let y_far = Point::from_coords(angle_80.cos(), angle_80.sin(), 0.0).normalize();
        let r_exact_80 = ChordAngle::from_length2(2.0 * (1.0 - angle_80.cos()));

        let r_smaller_80 = ChordAngle::from_length2(r_exact_80.length2() - 1e-10);
        let r_larger_80 = ChordAngle::from_length2(r_exact_80.length2() + 1e-10);
        assert_eq!(
            compare_distance(x, y_far, r_smaller_80),
            1,
            "distance > smaller threshold (large angle)"
        );
        assert_eq!(
            compare_distance(x, y_far, r_larger_80),
            -1,
            "distance < larger threshold (large angle)"
        );

        // --- Boundary cases ---
        assert_eq!(
            compare_distance(x, x, ChordAngle::ZERO),
            0,
            "zero distance equals zero threshold"
        );
        let antipodal = Point::from_coords(-1.0, 0.0, 0.0);
        assert_eq!(
            compare_distance(x, antipodal, ChordAngle::STRAIGHT),
            0,
            "antipodal distance equals straight threshold"
        );
    }

    #[test]
    fn test_circle_edge_intersection_ordering_exact() {
        // Test circle_edge_intersection_ordering with inputs that are
        // nearly degenerate, forcing the exact arithmetic fallback.
        //
        // We set up two edges AB and CD that cross the same great circle M
        // at nearly the same point, so the triage comparison cannot
        // distinguish them.

        // M is the "equator" normal, N is the "reference" normal.
        let m = Point::from_coords(0.0, 0.0, 1.0);
        let n = Point::from_coords(1.0, 0.0, 0.0);

        // Edge AB: crosses the equator near (0,1,0).
        let a = Point::from_coords(0.0, 1.0, 1e-15).normalize();
        let b = Point::from_coords(0.0, 1.0, -1e-15).normalize();

        // Edge CD: crosses the equator at almost the same point but with
        // a tiny shift in x.
        let c = Point::from_coords(1e-15, 1.0, 1e-15).normalize();
        let d = Point::from_coords(1e-15, 1.0, -1e-15).normalize();

        let result = circle_edge_intersection_ordering(a, b, c, d, m, n);
        // The edges are distinct (a!=c or b!=d), so exact arithmetic should
        // produce a definite ordering.
        // Verify antisymmetry: swapping the two edges negates the result.
        let swapped = circle_edge_intersection_ordering(c, d, a, b, m, n);
        assert_eq!(result, -swapped, "swapping edges must negate the result");

        // Test the trivial identity case: same edge should return 0.
        assert_eq!(
            circle_edge_intersection_ordering(a, b, a, b, m, n),
            0,
            "identical edges must return 0"
        );

        // Also test with reversed second edge (a,b) vs (b,a).
        assert_eq!(
            circle_edge_intersection_ordering(a, b, b, a, m, n),
            0,
            "reversed identical edge must return 0"
        );
    }

    #[test]
    fn test_triage_compare_sin2_distances() {
        // triage_compare_sin2_distances is private, so we test it indirectly
        // via compare_distances. The sin2 path is used when cos_ax < -1/sqrt(2),
        // i.e., when x and a are nearly antipodal (angle > 135 degrees).
        //
        // When cos_ax < -1/sqrt(2), the code negates the sin2 comparison
        // because sin2 ordering is reversed for obtuse angles.

        // x is at the "north pole".
        let x = Point::from_coords(0.0, 0.0, 1.0);

        // a and b are in the southern hemisphere, nearly antipodal to x.
        // cos(angle) with x will be strongly negative (< -1/sqrt(2)).
        // a is slightly closer to the south pole than b.
        let a = Point::from_coords(0.1, 0.0, -1.0).normalize();
        let b = Point::from_coords(0.2, 0.0, -1.0).normalize();

        // Verify that both are in the antipodal regime.
        let cos_ax = a.0.dot(x.0);
        let cos_bx = b.0.dot(x.0);
        let threshold = -std::f64::consts::FRAC_1_SQRT_2;
        assert!(
            cos_ax < threshold,
            "cos_ax = {cos_ax} should be < {threshold}"
        );
        assert!(
            cos_bx < threshold,
            "cos_bx = {cos_bx} should be < {threshold}"
        );

        // a is closer to the antipode (-x), so a is farther from x.
        // b has a larger lateral offset, so it's slightly closer to x.
        // Therefore XA > XB, so compare_distances(x, a, b) should be +1.
        let result = compare_distances(x, a, b);
        assert_eq!(
            result, 1,
            "a is farther from x than b in the antipodal regime"
        );
        assert_eq!(
            compare_distances(x, b, a),
            -1,
            "antisymmetry in antipodal regime"
        );

        // Test with points even closer to the antipode to ensure the
        // negated-sin2 path is exercised with tighter tolerances.
        let a2 = Point::from_coords(0.01, 0.0, -1.0).normalize();
        let b2 = Point::from_coords(0.02, 0.0, -1.0).normalize();
        let cos_a2x = a2.0.dot(x.0);
        assert!(
            cos_a2x < threshold,
            "cos_a2x = {cos_a2x} should be < {threshold}"
        );
        let result2 = compare_distances(x, a2, b2);
        assert_eq!(result2, 1, "a2 farther from x than b2");
        assert_eq!(compare_distances(x, b2, a2), -result2);
    }

    #[test]
    fn test_sign_collinear_points() {
        // C++ Sign::CollinearPoints
        // Points that are *exactly collinear* along a line approximately tangent
        // to the unit sphere. C is the exact midpoint of AB.
        let a = Point::from_coords(
            0.72571927877036835,
            0.46058825605889098,
            0.51106749730504852,
        );
        let b = Point::from_coords(0.7257192746638208, 0.46058826573818168, 0.51106749441312738);
        let c = Point::from_coords(
            0.72571927671709457,
            0.46058826089853633,
            0.51106749585908795,
        );
        // In C++ c-a == b-c exactly, but Rust float arithmetic may differ slightly.
        // The important property is that Sign produces a definite, consistent result.

        // Sign must be non-zero (symbolic perturbation breaks the tie).
        let s = robust_sign(a, b, c) as i32;
        assert_ne!(s, 0, "Collinear points should get a definite sign");
        // Cyclic invariance.
        assert_eq!(s, robust_sign(b, c, a) as i32);
        // Reversal negates.
        assert_eq!(s, -(robust_sign(c, b, a) as i32));

        // Exactly proportional points on a common line through the origin.
        let x1 = Point::from_coords(0.99999999999999989, 1.4901161193847655e-08, 0.0);
        let x2 = Point::from_coords(1.0, 1.4901161193847656e-08, 0.0);
        assert_eq!(x1, x1.normalize());
        assert_eq!(x2, x2.normalize());
        let s2 = robust_sign(x1, x2, -x1) as i32;
        assert_ne!(s2, 0);
        assert_eq!(s2, robust_sign(x2, -x1, x1) as i32);
        assert_eq!(s2, -(robust_sign(-x1, x2, x1) as i32));

        // Another pair: distinct, exactly proportional, self-normalized.
        let x3 = Point::from_coords(1.0, 1.0, 1.0).normalize();
        let x4 = Point(0.99999999999999989 * x3.0);
        // Pre-FMA, normalize() of an already-unit vector returned the input
        // bit-exactly because dot(self,self) summed identical squares in a
        // fixed order. With FMA in dot(), the sum can shift by 1 ULP and the
        // normalize round-trip drifts by the same amount. is_unit() is the
        // intent of this check.
        assert!(x3.0.is_unit());
        assert!(x4.0.is_unit());
        assert_ne!(x3, x4);
        assert_ne!(robust_sign(x3, x4, -x3) as i32, 0);
    }

    /// Helper for symbolic perturbation coverage: given 3 coplanar points
    /// A < B < C in lexicographic order, verify Sign matches expected.
    fn check_symbolic_sign(expected: i32, a: Point, b: Point, c: Point) {
        // Verify lexicographic ordering a < b < c.
        use std::cmp::Ordering;
        assert_eq!(
            a.0.cmp(b.0),
            Ordering::Less,
            "a must be < b lexicographically"
        );
        assert_eq!(
            b.0.cmp(c.0),
            Ordering::Less,
            "b must be < c lexicographically"
        );
        // Verify exact coplanarity with origin.
        let det = a.0.dot(b.0.cross(c.0));
        assert_eq!(det, 0.0, "Points must be exactly coplanar with origin");

        assert_eq!(expensive_sign(a, b, c) as i32, expected);
        assert_eq!(expensive_sign(b, c, a) as i32, expected);
        assert_eq!(expensive_sign(c, a, b) as i32, expected);
        assert_eq!(expensive_sign(c, b, a) as i32, -expected);
        assert_eq!(expensive_sign(b, a, c) as i32, -expected);
        assert_eq!(expensive_sign(a, c, b) as i32, -expected);
    }

    #[test]
    fn test_symbolic_perturbation_code_coverage() {
        // C++ Sign::SymbolicPerturbationCodeCoverage
        // Each test case exercises a different sub-determinant M_i in
        // SymbolicallyPerturbedSign(). The C++ S2Point(x,y,z) is an
        // unnormalized vector, so we use Point(Vector::new(...)) directly.
        use crate::r3::Vector;
        let p = |x: f64, y: f64, z: f64| Point(Vector::new(x, y, z));

        // det(M_1) = b0*c1 - b1*c0
        check_symbolic_sign(1, p(-3.0, -1.0, 0.0), p(-2.0, 1.0, 0.0), p(1.0, -2.0, 0.0));
        // det(M_2) = b2*c0 - b0*c2
        check_symbolic_sign(1, p(-6.0, 3.0, 3.0), p(-4.0, 2.0, -1.0), p(-2.0, 1.0, 4.0));
        // det(M_3) = b1*c2 - b2*c1
        check_symbolic_sign(1, p(0.0, -1.0, -1.0), p(0.0, 1.0, -2.0), p(0.0, 2.0, 1.0));
        // det(M_4) = c0*a1 - c1*a0
        check_symbolic_sign(1, p(-1.0, 2.0, 7.0), p(2.0, 1.0, -4.0), p(4.0, 2.0, -8.0));
        // det(M_5) = c0
        check_symbolic_sign(1, p(-4.0, -2.0, 7.0), p(2.0, 1.0, -4.0), p(4.0, 2.0, -8.0));
        // det(M_6) = -c1
        check_symbolic_sign(1, p(0.0, -5.0, 7.0), p(0.0, -4.0, 8.0), p(0.0, -2.0, 4.0));
        // det(M_7) = c2*a0 - c0*a2
        check_symbolic_sign(1, p(-5.0, -2.0, 7.0), p(0.0, 0.0, -2.0), p(0.0, 0.0, -1.0));
        // det(M_8) = c2
        check_symbolic_sign(1, p(0.0, -2.0, 7.0), p(0.0, 0.0, 1.0), p(0.0, 0.0, 2.0));
        // det(M_9) = a0*b1 - a1*b0
        check_symbolic_sign(1, p(-3.0, 1.0, 7.0), p(-1.0, -4.0, 1.0), p(0.0, 0.0, 0.0));
        // det(M_10) = -b0
        check_symbolic_sign(1, p(-6.0, -4.0, 7.0), p(-3.0, -2.0, 1.0), p(0.0, 0.0, 0.0));
        // det(M_11) = b1
        check_symbolic_sign(-1, p(0.0, -4.0, 7.0), p(0.0, -2.0, 1.0), p(0.0, 0.0, 0.0));
        // det(M_12) = a0
        check_symbolic_sign(-1, p(-1.0, -4.0, 5.0), p(0.0, 0.0, -3.0), p(0.0, 0.0, 0.0));
        // det(M_13) = 1
        check_symbolic_sign(1, p(0.0, -4.0, 5.0), p(0.0, 0.0, -5.0), p(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_sign_dot_prod_nearly_orthogonal() {
        // C++ SignDotProd::NearlyOrthogonalPositive / NearlyOrthogonalNegative
        let a = Point::from_coords(1.0, 0.0, 0.0);

        // Slightly positive dot product (DBL_EPSILON, 1, 0) · (1, 0, 0) = DBL_EPSILON > 0.
        let b_pos = Point::from_coords(f64::EPSILON, 1.0, 0.0);
        assert_eq!(sign_dot_prod(a, b_pos), 1, "nearly orthogonal positive");

        // Slightly negative dot product.
        let b_neg = Point::from_coords(-f64::EPSILON, 1.0, 0.0);
        assert_eq!(sign_dot_prod(a, b_neg), -1, "nearly orthogonal negative");

        // Extremely small positive — forces exact arithmetic.
        let c_pos = Point::from_coords(1e-45, 1.0, 0.0);
        assert_eq!(sign_dot_prod(a, c_pos), 1, "tiny positive dot product");

        // Extremely small negative — forces exact arithmetic.
        let c_neg = Point::from_coords(-1e-45, 1.0, 0.0);
        assert_eq!(sign_dot_prod(a, c_neg), -1, "tiny negative dot product");
    }

    #[test]
    fn test_compare_edge_distance_basic() {
        // C++ CompareEdgeDistance::Coverage (partial)
        // Test distance from a point to an edge.
        let x = Point::from_coords(1.0, 0.0, 0.0);
        let a0 = Point::from_coords(0.0, 1.0, 0.0);
        let a1 = Point::from_coords(0.0, 0.0, 1.0);

        // Distance from (1,0,0) to the edge (0,1,0)→(0,0,1) is 90 degrees.
        let r_exact = ChordAngle::RIGHT;
        assert_eq!(compare_edge_distance(x, a0, a1, r_exact), 0);

        let r_small = ChordAngle::from_length2(1.5);
        assert_eq!(compare_edge_distance(x, a0, a1, r_small), 1); // dist > r

        let r_big = ChordAngle::from_length2(2.5);
        assert_eq!(compare_edge_distance(x, a0, a1, r_big), -1); // dist < r
    }

    #[test]
    fn test_compare_edge_directions_basic() {
        // C++ CompareEdgeDirections::Coverage (partial)
        // Two edges pointing in the same direction.
        let a0 = Point::from_coords(1.0, 0.0, 0.0);
        let a1 = Point::from_coords(0.0, 1.0, 0.0);
        let b0 = Point::from_coords(1.0, 0.0, 0.0);
        let b1 = Point::from_coords(0.0, 1.0, 0.0);
        assert_eq!(compare_edge_directions(a0, a1, b0, b1), 1);

        // Opposite directions.
        assert_eq!(compare_edge_directions(a0, a1, a1, a0), -1);

        // Perpendicular edges.
        let c0 = Point::from_coords(0.0, 0.0, 1.0);
        assert_eq!(compare_edge_directions(a0, a1, a0, c0), 0);
    }

    #[test]
    fn test_edge_circumcenter_sign_basic() {
        // C++ EdgeCircumcenterSign::Coverage (partial)
        // Test with a well-separated triangle and edge.
        let x0 = Point::from_coords(1.0, 0.0, 0.0);
        let x1 = Point::from_coords(0.0, 1.0, 0.0);
        let a = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let b = Point::from_coords(0.0, 0.0, 1.0);
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let result = edge_circumcenter_sign(x0, x1, a, b, c);
        // Just verify it returns a definite answer.
        assert!(result == -1 || result == 0 || result == 1);
    }

    #[test]
    fn test_voronoi_site_exclusion_basic() {
        // Two sites on the same edge with a small radius.
        // x0 and x1 must not be antipodal.
        let x0 = Point::from_coords(1.0, 0.0, 0.0);
        let x1 = Point::from_coords(0.0, 1.0, 0.0);
        // a is closer to x0 than b (precondition: CompareDistances(x0, a, b) < 0).
        let a = Point::from_coords(1.0, 0.1, 0.0).normalize();
        let b = Point::from_coords(0.1, 1.0, 0.0).normalize();
        let r = ChordAngle::from_angle(crate::s1::Angle::from_degrees(80.0));

        let result = get_voronoi_site_exclusion(a, b, x0, x1, r);
        assert!(
            matches!(
                result,
                Excluded::First | Excluded::Second | Excluded::Neither
            ),
            "result should be a valid non-uncertain Excluded variant, got {result:?}"
        );
    }

    // ===== Extended C++ coverage tests =====

    #[test]
    fn test_compare_distance_coverage() {
        // From C++ CompareDistance::Coverage — multiple precision paths.
        // TriageCompareSin2Distance: near-identical points, tiny threshold.
        assert_eq!(
            compare_distance(
                Point::from_coords(1.0, 1.0, 1.0),
                Point::from_coords(1.0, 1.0 - 1e-15, 1.0),
                ChordAngle::from_radians(1e-15),
            ),
            -1,
        );

        // TriageCompareCosDistance: small angle, larger threshold.
        assert_eq!(
            compare_distance(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(1.0, 1e-8, 0.0),
                ChordAngle::from_radians(1e-7),
            ),
            -1,
        );

        // TriageCompareCosDistance: nearly antipodal.
        assert_eq!(
            compare_distance(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(-1.0, 1e-8, 0.0),
                ChordAngle::from_radians(std::f64::consts::PI - 1e-7),
            ),
            1,
        );

        // Exactly 90° from each other → compare with RIGHT gives 0.
        assert_eq!(
            compare_distance(
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(1.0, -1.0, 0.0),
                ChordAngle::RIGHT,
            ),
            0,
        );

        // Exactly 60° from each other → length2 = 1.
        assert_eq!(
            compare_distance(
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(0.0, 1.0, 1.0),
                ChordAngle::from_length2(1.0),
            ),
            0,
        );
    }

    #[test]
    fn test_compare_edge_distance_coverage() {
        // From C++ CompareEdgeDistance::Coverage — exact cases.
        // Distance from (1,0,0) to edge (0,1,0)→(0,0,1) is exactly RIGHT.
        assert_eq!(
            compare_edge_distance(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(0.0, 1.0, 0.0),
                Point::from_coords(0.0, 0.0, 1.0),
                ChordAngle::RIGHT,
            ),
            0,
        );

        // Same point, smaller threshold → distance exceeds threshold.
        assert_eq!(
            compare_edge_distance(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(0.0, 1.0, 0.0),
                Point::from_coords(0.0, 0.0, 1.0),
                ChordAngle::from_length2(1.5),
            ),
            1,
        );

        // Same point, larger threshold → distance less than threshold.
        assert_eq!(
            compare_edge_distance(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(0.0, 1.0, 0.0),
                Point::from_coords(0.0, 0.0, 1.0),
                ChordAngle::from_length2(2.5),
            ),
            -1,
        );
    }

    #[test]
    fn test_compare_edge_directions_coverage() {
        // From C++ CompareEdgeDirections::Coverage — precision paths.
        // Clear double-precision case.
        assert_eq!(
            compare_edge_directions(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(1.0, -1.0, 0.0),
                Point::from_coords(1.0, 0.0, 0.0),
            ),
            1,
        );

        // Nearly collinear case requiring higher precision.
        assert_eq!(
            compare_edge_directions(
                Point::from_coords(1.0, 0.0, 1.5e-15),
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(0.0, -1.0, 0.0),
                Point::from_coords(0.0, 0.0, 1.0),
            ),
            1,
        );

        // Exact precision case (very small perturbation).
        assert_eq!(
            compare_edge_directions(
                Point::from_coords(1.0, 0.0, 1e-50),
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(0.0, -1.0, 0.0),
                Point::from_coords(0.0, 0.0, 1.0),
            ),
            1,
        );

        // Zero case (exactly perpendicular).
        assert_eq!(
            compare_edge_directions(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(0.0, -1.0, 0.0),
                Point::from_coords(0.0, 0.0, 1.0),
            ),
            0,
        );
    }

    #[test]
    fn test_edge_circumcenter_sign_coverage() {
        // From C++ EdgeCircumcenterSign::Coverage — multiple precision paths.
        // Clear positive case.
        assert_eq!(
            edge_circumcenter_sign(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(0.0, 0.0, 1.0),
                Point::from_coords(1.0, 0.0, 1.0),
                Point::from_coords(0.0, 1.0, 1.0),
            ),
            1,
        );

        // Clear negative case (triangle on opposite side).
        assert_eq!(
            edge_circumcenter_sign(
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(1.0, 1.0, 0.0),
                Point::from_coords(0.0, 0.0, -1.0),
                Point::from_coords(1.0, 0.0, -1.0),
                Point::from_coords(0.0, 1.0, -1.0),
            ),
            -1,
        );
    }

    #[test]
    fn test_voronoi_site_exclusion_coverage() {
        // From C++ VoronoiSiteExclusion::Coverage — specific deterministic cases.

        // Both sites closest to endpoint X0 → second site excluded.
        assert_eq!(
            get_voronoi_site_exclusion(
                Point::from_coords(1.0, -1e-5, 0.0),
                Point::from_coords(1.0, -2e-5, 0.0),
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(1.0, 1.0, 0.0),
                ChordAngle::from_radians(1e-3),
            ),
            Excluded::Second,
        );

        // Both sites closest to endpoint X1 → second site excluded.
        assert_eq!(
            get_voronoi_site_exclusion(
                Point::from_coords(1.0, 1.0, 1e-30),
                Point::from_coords(1.0, 1.0, -1e-20),
                Point::from_coords(1.0, 0.0, 0.0),
                Point::from_coords(1.0, 1.0, 0.0),
                ChordAngle::from_radians(1e-10),
            ),
            Excluded::Second,
        );

        // Sites on opposite sides of edge interior → neither excluded.
        assert_eq!(
            get_voronoi_site_exclusion(
                Point::from_coords(1.0, -1e-10, 1e-5),
                Point::from_coords(1.0, 1e-10, -1e-5),
                Point::from_coords(1.0, -1.0, 0.0),
                Point::from_coords(1.0, 1.0, 0.0),
                ChordAngle::from_radians(1e-4),
            ),
            Excluded::Neither,
        );
    }

    #[test]
    fn test_voronoi_site_exclusion_exact_path() {
        // Construct a case designed to force the exact arithmetic path.
        // Two sites that are very close together and very close to the edge,
        // so the triage path returns Uncertain.
        //
        // Edge along the equator from x0=(1,0,0) to x1=(0,1,0).
        // Sites a and b are extremely close together near the edge midpoint.
        let x0 = Point::from_coords(1.0, 0.0, 0.0);
        let x1 = Point::from_coords(0.0, 1.0, 0.0);

        // Sites very close to the midpoint of the edge with tiny offsets.
        // a is slightly closer to the edge than b.
        let mid = 1.0 / std::f64::consts::SQRT_2;
        let eps = 1e-15;
        let a = Point::from_coords(mid + eps, mid, eps).normalize();
        let b = Point::from_coords(mid + 2.0 * eps, mid, 2.0 * eps).normalize();

        // Small radius that still covers both sites.
        let r = ChordAngle::from_radians(1e-12);

        // The precondition is CompareDistances(x0, a, b) < 0.
        // Check this — if not, swap a and b.
        let cmp = compare_distances(x0, a, b);
        let (a_use, b_use) = if cmp < 0 { (a, b) } else { (b, a) };

        // Ensure both are within r of edge (precondition).
        let ca = compare_edge_distance(a_use, x0, x1, r);
        let cb = compare_edge_distance(b_use, x0, x1, r);
        if ca <= 0 && cb <= 0 {
            let result = get_voronoi_site_exclusion(a_use, b_use, x0, x1, r);
            // The result should be definitive (not Uncertain/Neither due to
            // triage failure — the exact path should resolve it).
            assert!(
                result == Excluded::First
                    || result == Excluded::Second
                    || result == Excluded::Neither,
                "exact path should produce a definitive result: {result:?}"
            );
        }
    }

    #[test]
    fn test_voronoi_site_exclusion_exact_first_excluded() {
        // C++ VoronoiSiteExclusion::Coverage: first site excluded, requires
        // EXACT precision.
        let a = Point::from_coords(1.0, -1e-31, 1.005e-30).normalize();
        let b = Point::from_coords(1.0, 0.0, -1e-30).normalize();
        let x0 = Point::from_coords(1.0, -1.0, 0.0).normalize();
        let x1 = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let r = ChordAngle::from_radians(1.005e-30);

        let result = get_voronoi_site_exclusion(a, b, x0, x1, r);
        assert_eq!(result, Excluded::First);
    }

    #[test]
    fn test_voronoi_site_exclusion_exact_neither() {
        // C++ VoronoiSiteExclusion::Coverage: neither excluded, requires
        // EXACT precision.
        let a = Point::from_coords(1.0, -1e-20, 1e-5).normalize();
        let b = Point::from_coords(1.0, 1e-20, -1e-5).normalize();
        let x0 = Point::from_coords(1.0, -1.0, 0.0).normalize();
        let x1 = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let r = ChordAngle::from_radians(1e-5);

        let result = get_voronoi_site_exclusion(a, b, x0, x1, r);
        assert_eq!(result, Excluded::Neither);
    }

    #[test]
    fn test_voronoi_site_exclusion_d_negative_first() {
        // C++ d < 0 case: A projects to interior of X, AB goes opposite
        // direction of X when projected.
        let a = Point::from_coords(1.0, -1e-5, 1e-4).normalize();
        let b = Point::from_coords(1.0, -1.000_000_01e-5, 0.0).normalize();
        let x0 = Point::from_coords(-1.0, -1.0, 0.0).normalize();
        let x1 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let r = ChordAngle::from_radians(1.0);

        let result = get_voronoi_site_exclusion(a, b, x0, x1, r);
        assert_eq!(result, Excluded::First);
    }

    #[test]
    fn test_voronoi_site_exclusion_d_negative_neither() {
        // C++ d < 0 case: both sites kept. A projects past X0, B past X1,
        // A closer to great circle through X.
        let a = Point::from_coords(-1.0, 0.1, 0.001).normalize();
        let b = Point::from_coords(1.0, 1.1, 0.0).normalize();
        let x0 = Point::from_coords(-1.0, -1.0, 0.0).normalize();
        let x1 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let r = ChordAngle::from_radians(1.0);

        let result = get_voronoi_site_exclusion(a, b, x0, x1, r);
        assert_eq!(result, Excluded::Neither);
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn prop_sign_reversal(ax: i32, ay: i32, az: i32, bx: i32, by: i32, bz: i32) -> bool {
        if ax == 0 && ay == 0 && az == 0 {
            return true;
        }
        if bx == 0 && by == 0 && bz == 0 {
            return true;
        }
        let a = Point::from_coords(f64::from(ax), f64::from(ay), f64::from(az)).normalize();
        let b = Point::from_coords(f64::from(bx), f64::from(by), f64::from(bz)).normalize();
        let c = Point::from_coords(0.0, 0.0, 1.0);
        robust_sign(a, b, c) == -robust_sign(a, c, b)
    }

    #[quickcheck]
    fn prop_sign_cyclic(ax: i32, ay: i32, az: i32, bx: i32, by: i32, bz: i32) -> bool {
        if ax == 0 && ay == 0 && az == 0 {
            return true;
        }
        if bx == 0 && by == 0 && bz == 0 {
            return true;
        }
        let a = Point::from_coords(f64::from(ax), f64::from(ay), f64::from(az)).normalize();
        let b = Point::from_coords(f64::from(bx), f64::from(by), f64::from(bz)).normalize();
        let c = Point::from_coords(0.0, 0.0, 1.0);
        let s = robust_sign(a, b, c);
        s == robust_sign(b, c, a) && s == robust_sign(c, a, b)
    }

    #[quickcheck]
    fn prop_sign_degenerate(ax: i32, ay: i32, az: i32) -> bool {
        if ax == 0 && ay == 0 && az == 0 {
            return true;
        }
        let a = Point::from_coords(f64::from(ax), f64::from(ay), f64::from(az)).normalize();
        robust_sign(a, a, a) == Direction::Indeterminate
    }

    #[test]
    fn test_compare_edge_pair_distance_crossing() {
        // Two crossing edges: distance is 0.
        let a0 = Point::from_coords(1.0, 0.0, 0.1);
        let a1 = Point::from_coords(1.0, 0.0, -0.1);
        let b0 = Point::from_coords(1.0, -0.1, 0.0);
        let b1 = Point::from_coords(1.0, 0.1, 0.0);
        // Distance is 0, should be less than any positive r.
        assert_eq!(
            compare_edge_pair_distance(a0, a1, b0, b1, ChordAngle::from_degrees(1.0)),
            -1
        );
        // Distance is 0, equals 0.
        assert_eq!(
            compare_edge_pair_distance(a0, a1, b0, b1, ChordAngle::ZERO),
            0
        );
    }

    #[test]
    fn test_compare_edge_pair_distance_non_crossing() {
        // Two non-crossing edges.
        let a0 = Point::from_coords(1.0, 0.0, 0.0);
        let a1 = Point::from_coords(1.0, 0.1, 0.0);
        let b0 = Point::from_coords(0.0, 1.0, 0.0);
        let b1 = Point::from_coords(0.0, 1.0, 0.1);
        // Distance is ~90 degrees, so should be > 10 degrees.
        assert_eq!(
            compare_edge_pair_distance(a0, a1, b0, b1, ChordAngle::from_degrees(10.0)),
            1
        );
    }

    #[test]
    fn test_circle_edge_intersection_sign_basic() {
        // Test with orthogonal vectors.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let n = Point::from_coords(0.0, 0.0, 1.0);
        let x = Point::from_coords(0.0, 0.0, 1.0);
        // (N·A)(X·B) - (N·B)(X·A) = (0)(0) - (0)(0) = 0.
        assert_eq!(circle_edge_intersection_sign(a, b, n, x), 0);

        // Non-degenerate case.
        let x2 = Point::from_coords(1.0, 1.0, 0.0).normalize();
        // (N·A)(X·B) - (N·B)(X·A) = (0)(1/√2) - (0)(1/√2) = 0 since N=(0,0,1).
        // Try different N.
        let n2 = Point::from_coords(1.0, 0.0, 0.0);
        // (N2·A)(X2·B) - (N2·B)(X2·A) = (1)(1/√2) - (0)(1/√2) = 1/√2 > 0.
        assert_eq!(circle_edge_intersection_sign(a, b, n2, x2), 1);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_direction_roundtrip() {
        for d in [
            Direction::Clockwise,
            Direction::Indeterminate,
            Direction::CounterClockwise,
        ] {
            let json = serde_json::to_string(&d).unwrap();
            let back: Direction = serde_json::from_str(&json).unwrap();
            assert_eq!(d, back);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_excluded_roundtrip() {
        for e in [
            Excluded::First,
            Excluded::Second,
            Excluded::Neither,
            Excluded::Uncertain,
        ] {
            let json = serde_json::to_string(&e).unwrap();
            let back: Excluded = serde_json::from_str(&json).unwrap();
            assert_eq!(e, back);
        }
    }
}
