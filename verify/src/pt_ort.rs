// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The orthogonal directions `ortho`, `reference_dir` and `point_cross` (P-PTORT, spec 006
//! data-model §4.2), over the box `[−1, 1]³`, which holds every unit point. Every member is
//! weekly: `ortho` and `point_cross` cost several minutes per call (2026-10-14).
//!
//! The laws comparing two orthogonal directions are a tool limit: each normalises, and the
//! checker models `sqrt` without fixing its result, so two square roots of one number may
//! differ (spec 006, T042's measurement). So P-PTORT-01 (`ortho` is odd, `reference_dir` is
//! `ortho`, `ortho` is upstream's) and P-PTORT-03's `ortho` branch have no harness; the C++
//! vectors check `ortho` bit for bit (`core/tests/cpp_values_diff.rs`), and
//! `core/src/s2/point_tests.rs` pins the branch (K10, K11). Laws that hold as values compare
//! with `point_spec::same`: a coordinate computed as `0 − 0` is `+0` for both signs of the
//! input (research R0, finding 2).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

#[cfg(feature = "slow")]
use s2rst::s2::{Point, ortho};

#[cfg(feature = "slow")]
use crate::point_spec;
#[cfg(feature = "slow")]
use crate::value_domain::{any_box_point, any_normalizable_box_point};

/// P-PTORT-02 (correct result), `ortho` (weekly): never the zero vector for a normalizable
/// point. It took 606 s with the `point_cross` clause per change (2026-10-14).
///
/// For every `a` in `NormalizableBoxPoint`: `ortho(a)` is not `point_spec::zero`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptort_02__ortho() {
    let a = any_normalizable_box_point();
    assert!(!point_spec::zero(ortho(a)));
}

/// P-PTORT-02 (correct result), `point_cross` (weekly): never the zero vector when its first
/// point is normalizable. It took 400 s per change (2026-10-14).
///
/// For every `a` in `NormalizableBoxPoint` and `q` in `BoxPoint`: `a.point_cross(q)` is not
/// `point_spec::zero`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptort_02__point_cross() {
    let (a, q) = (any_normalizable_box_point(), any_box_point());
    assert!(!point_spec::zero(a.point_cross(q)));
    kani::cover!(point_spec::same(a, q), "equal points");
}

/// The points of P-PTORT-03 off the `ortho` branch: `(p + q) × (q − p)` is not zero.
#[cfg(feature = "slow")]
fn crossing_pair() -> (Point, Point) {
    let (p, q) = (any_box_point(), any_box_point());
    kani::assume(!point_spec::zero(point_spec::go_cross_kernel(p, q)));
    (p, q)
}

/// P-PTORT-03 (law), swapping: `point_cross` is antisymmetric, as values, wherever
/// `(p + q) × (q − p)` is not the zero vector (`UPSTREAM.md` U-013: Go's `PointCross` documents
/// the narrower exception "p == op or p == −op").
///
/// For every `p`, `q` in `BoxPoint` with `point_spec::go_cross_kernel(p, q)` not zero:
/// `same(q.point_cross(p), −p.point_cross(q))`.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptort_03__swap() {
    let (p, q) = crossing_pair();
    assert!(point_spec::same(q.point_cross(p), -p.point_cross(q)));
}

/// P-PTORT-03 (law), negating the first point, as `__swap`.
///
/// For every `p`, `q` in `BoxPoint` with `point_spec::go_cross_kernel(p, q)` not zero:
/// `same((−p).point_cross(q), −p.point_cross(q))`.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptort_03__neg_p() {
    let (p, q) = crossing_pair();
    assert!(point_spec::same((-p).point_cross(q), -p.point_cross(q)));
}

/// P-PTORT-03 (law), negating the second point, as `__swap`.
///
/// For every `p`, `q` in `BoxPoint` with `point_spec::go_cross_kernel(p, q)` not zero:
/// `same(p.point_cross(−q), −p.point_cross(q))`.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptort_03__neg_q() {
    let (p, q) = crossing_pair();
    assert!(point_spec::same(p.point_cross(-q), -p.point_cross(q)));
}
