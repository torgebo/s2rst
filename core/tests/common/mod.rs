// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Helpers shared by the differential tests against the C++ reference.

use s2rst::s1::Angle;
use s2rst::s2::Cap;

/// Upstream's cap, widened as `Rect::cap_bound` documents (spec 010 FR-019, `UPSTREAM.md`
/// U-016): by `10⁻¹⁵` rad, then by `8ε` of its squared chord length. Written out here, not
/// called from the library, so that the tests check the library's widening against its
/// documentation.
pub(crate) fn widened(up: Cap) -> Cap {
    if up.is_empty() {
        return up;
    }
    let wide = up.expanded(Angle::from_radians(1e-15));
    let radius = wide.chord_radius();
    Cap::from_center_chord_angle(
        wide.center(),
        radius.plus_error(8.0 * f64::EPSILON * radius.length2()),
    )
}
