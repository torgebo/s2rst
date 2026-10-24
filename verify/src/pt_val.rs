// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Point construction, conversions and accessors (P-PTVAL, spec 006 data-model §4.2), against
//! the point description (`point_spec`).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::r3::Vector;
use s2rst::s2::Point;

#[cfg(feature = "slow")]
use crate::value_domain::any_finite_triple;
use crate::value_domain::{any_plain_triple, any_triple};
use crate::{laws, point_spec};

fn bits(p: Point) -> [u64; 3] {
    [p.0.x, p.0.y, p.0.z].map(f64::to_bits)
}

/// P-PTVAL-01 (correct result): building from a vector and the two conversions keep the bits;
/// the accessors return the coordinates; the default is `(0, 0, 0)`; `origin` is upstream's
/// point, and unit length.
///
/// For every triple `p`, bit for bit: `Point::new(p.0)`, `Point::from(p.0)` and
/// `Vector::from(p)` are `p`; `p.vector()`, `p.x()`, `p.y()`, `p.z()` are its fields;
/// `Point::default()` is `(+0, +0, +0)`; `Point::origin()` is `point_spec::ORIGIN`, and
/// `point_spec::upstream_is_unit(origin)`.
#[kani::proof]
fn p_ptval_01() {
    let p = any_triple();
    let want = bits(p);
    assert_eq!(bits(Point::new(p.0)), want);
    assert_eq!(bits(Point::from(p.0)), want);
    assert_eq!(bits(Point(Vector::from(p))), want);
    assert_eq!(bits(Point(p.vector())), want);
    assert_eq!([p.x(), p.y(), p.z()].map(f64::to_bits), want);
    assert_eq!(bits(Point::default()), [0.0_f64.to_bits(); 3]);
    let o = Point::origin();
    assert_eq!(bits(o), point_spec::ORIGIN.map(f64::to_bits));
    assert!(point_spec::upstream_is_unit(o));
    kani::cover!(
        p.x().is_nan() && p.y().is_infinite(),
        "NaN and infinite coordinates"
    );
}

/// P-PTVAL-02 (correct result), the clauses without a square root: `from_coords` maps every
/// zero triple to `origin`, and where the squared length is zero, `normalize` returns
/// `(+0, +0, +0)`, as Go, where C++ keeps each zero's sign (K12); `from_coords` of a non-zero
/// triple with a zero squared length is that too.
///
/// For every `p` in `FiniteTriple` with `upstream_norm2(p) == 0`: if `point_spec::zero(p)`,
/// `from_coords(p)` is `origin`, else `(+0, +0, +0)`; `p.normalize()` is `(+0, +0, +0)`, the same
/// as `upstream_normalize(p)` (`point_spec::same`); all bit for bit.
///
/// The rest of P-PTVAL-02, `normalize` against upstream's formula where the squared length is
/// not zero, is a tool limit: the checker models `sqrt` without fixing its result, so two
/// square roots of one number may differ and no two normalisations compare bit for bit (spec
/// 006, T042's measurement). The C++ vectors check it on every row
/// (`core/tests/cpp_values_diff.rs`, `verify/tests/values_spec_vs_cpp.rs`).
///
/// Weekly: it took 516 s per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptval_02__zero_length() {
    let p = any_finite_triple();
    kani::assume(point_spec::upstream_norm2(p) == 0.0);
    let positive_zeros = [0.0_f64.to_bits(); 3];
    let f = Point::from_coords(p.x(), p.y(), p.z());
    if point_spec::zero(p) {
        assert_eq!(bits(f), bits(Point::origin()));
    } else {
        assert_eq!(bits(f), positive_zeros);
    }
    let n = p.normalize();
    assert_eq!(bits(n), positive_zeros);
    assert!(point_spec::same(n, point_spec::upstream_normalize(p)));
    kani::cover!(
        point_spec::zero(p) && p.x().is_sign_negative(),
        "a signed zero triple"
    );
    kani::cover!(!point_spec::zero(p), "an underflowing squared length");
}

/// P-PTVAL-03 (law): the generic equality laws hold for points without NaN.
///
/// For every `a`, `b`, `c` in `PlainTriple`: `laws::eq::all(a, b, c)`.
#[kani::proof]
fn p_ptval_03() {
    let (a, b, c) = (any_plain_triple(), any_plain_triple(), any_plain_triple());
    laws::eq::all(&a, &b, &c);
    kani::cover!(a == b && a.x().to_bits() != b.x().to_bits(), "0 and −0");
}
