// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The power-of-two scaling `Point::ensure_normalizable` (P-PTSCL, spec 006 data-model §4.2;
//! FR-020, K5), against the description's exact scaling and upstream's arithmetic.

use s2rst::s2::Point;

use crate::point_spec;
use crate::value_domain::any_finite_triple;

fn bits(p: Point) -> [u64; 3] {
    [p.0.x, p.0.y, p.0.z].map(f64::to_bits)
}

/// P-PTSCL-01 (correct result): `ensure_normalizable` returns a normalizable vector for every
/// non-zero finite input, the input itself when it already is normalizable, and the zero vector
/// for zero (SC-010).
///
/// For every `p` in `FiniteTriple`, with `r = p.ensure_normalizable()`: if `point_spec::zero(p)`
/// or `point_spec::normalizable(p)`, `r` is `p`, bit for bit; otherwise
/// `point_spec::normalizable(r)`.
#[kani::proof]
fn p_ptscl_01() {
    let p = any_finite_triple();
    let r = p.ensure_normalizable();
    if point_spec::zero(p) || point_spec::normalizable(p) {
        assert_eq!(bits(r), bits(p));
    } else {
        assert!(point_spec::normalizable(r));
    }
    kani::cover!(
        !point_spec::zero(p) && point_spec::max_abs(p) < f64::MIN_POSITIVE,
        "a subnormal vector (K5)"
    );
}

/// P-PTSCL-02 (exact output): a vector that is not normalizable is scaled by a power of two,
/// exactly, so that its largest coordinate is in [1, 2), and the result is upstream's wherever
/// upstream's is finite (U-012).
///
/// For every non-zero `p` in `FiniteTriple` with `¬point_spec::normalizable(p)`, with
/// `r = p.ensure_normalizable()`: `r` is `point_spec::scaled_by(p, −exponent(max_abs(p)))`, bit
/// for bit; `1 ≤ max_abs(r) < 2`; and where upstream's factor is finite
/// (`exponent(max_abs(p)) ≥ −1023`), `r` is `point_spec::upstream_scaled(p)`, bit for bit.
#[kani::proof]
fn p_ptscl_02() {
    let p = any_finite_triple();
    kani::assume(!point_spec::zero(p) && !point_spec::normalizable(p));
    let r = p.ensure_normalizable();
    let k = -point_spec::exponent(point_spec::max_abs(p));
    assert_eq!(bits(r), bits(point_spec::scaled_by(p, k)));
    let m = point_spec::max_abs(r);
    assert!((1.0..2.0).contains(&m));
    // Upstream's factor `2^(−ilogb(p_max))` is finite exactly for `ilogb(p_max) ≥ −1023`; below,
    // it overflows and its `∞ · 0` is a real NaN (U-012), so it is not computed there.
    let finite = point_spec::exponent(point_spec::max_abs(p)) >= -1023;
    if finite {
        assert_eq!(bits(r), bits(point_spec::upstream_scaled(p)));
    }
    kani::cover!(!finite, "upstream overflows (U-012)");
    kani::cover!(finite, "upstream's result");
}
