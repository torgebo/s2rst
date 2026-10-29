// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UAREA-04 (correct result), proven by enumeration (spec 011 data-model §4.2): the library's
//! `Cell::average_area_for_level(l)`, `4π / (6 · 4^l)`, is upstream's
//! `S2Cell::AverageArea(l)`, `ldexp(4π/6, −2l)`, bit for bit, for each of the 31 levels. The
//! checker cannot state it (`powi` is not one value per input to it, KI-013), and 31 cases are
//! the whole domain.

use s2rst::s2::{Cell, Level};
use s2rst_verify::region_spec::upstream_average_area;

#[test]
fn average_area_for_level_is_upstreams_at_every_level() {
    for l in 0..=30u8 {
        assert_eq!(
            Cell::average_area_for_level(Level::new(l)).to_bits(),
            upstream_average_area(l).to_bits(),
            "level {l}"
        );
    }
}
