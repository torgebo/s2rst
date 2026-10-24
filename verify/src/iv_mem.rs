// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Interval membership (P-IMEM, spec 004 data-model §4.2), against `arc_spec`.

use std::f64::consts::PI;

use crate::arc_spec;
use crate::value_domain::{any_interval, any_point};

/// P-IMEM-01 (correct result): a point is in an interval exactly when the model says it is, and
/// π and −π are one point.
///
/// For every valid `x` and point `p` in [−π, π]: `x.contains(p) == arc_spec::member(x, p)`.
#[kani::proof]
fn p_imem_01() {
    let (x, p) = (any_interval(), any_point());
    assert_eq!(x.contains(p), arc_spec::member(x, p));
    kani::cover!(
        x.contains(-PI) && !x.is_full(),
        "−π in a non-full interval, as π"
    );
    kani::cover!(x.contains(p) && x.is_inverted(), "in an inverted interval");
}

/// P-IMEM-02 (correct result): a point is in an interval's interior exactly when the model says
/// it is.
///
/// For every valid `x` and point `p`: `x.interior_contains(p) == arc_spec::interior(x, p)`.
#[kani::proof]
fn p_imem_02() {
    let (x, p) = (any_interval(), any_point());
    assert_eq!(x.interior_contains(p), arc_spec::interior(x, p));
    kani::cover!(x.contains(p) && !x.interior_contains(p), "a bound");
    kani::cover!(
        x.is_full() && x.interior_contains(PI),
        "the full interval's interior"
    );
}
