// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap` point membership (P-CPMEM, spec 008 data-model §4).

use s2rst::s2::Cap;

use crate::cap_spec;
use crate::stubs;
use crate::value_domain::{any_box_cap, any_box_point};

/// P-CPMEM-01 (correct result): `contains_point` is upstream's membership test and
/// `interior_contains_point` its interior; the interior test implies containment; a point cap
/// contains its point.
///
/// For every `a` in `BoxCap` and `p` in `BoxPoint`, with `d` the chord from the centre to `p`:
/// `a.contains_point(p)` is `full(a) || (!empty(a) && d ≤ r²)` (`cap_spec::member` over the
/// library's chord), `a.interior_contains_point(p)` is `full(a) || d < r²`, the second implies
/// the first, and `Cap::from_point(p).contains_point(p)`.
///
/// Stubs: `pure_chord` (L-CP-01, L-CP-02): the statement reads the distance the test read. The
/// chord itself is upstream's `S1ChordAngle(a, b)`, checked bit for bit by the C++ vectors
/// (`core/tests/cpp_values_diff.rs`, `S2POINT_*`), and `cap_spec::member` with its own chord
/// agrees with C++ on every `CAP_*` row (`tests/values_spec_vs_cpp.rs`).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
fn p_cpmem_01() {
    stubs::reset();
    let (a, p) = (any_box_cap(), any_box_point());
    let d = a.center().chord_angle(p).length2();
    let r = cap_spec::radius2(a);
    let (full, empty) = (cap_spec::full(a), cap_spec::empty(a));
    assert!(a.contains_point(p) == (full || (!empty && d <= r)));
    assert!(a.interior_contains_point(p) == (full || d < r));
    if a.interior_contains_point(p) {
        assert!(a.contains_point(p));
    }
    assert!(Cap::from_point(p).contains_point(p));
    kani::cover!(
        a.contains_point(p) && !a.interior_contains_point(p),
        "a point on the boundary"
    );
}
