// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a point region and of a cell union's floating-point methods
//! (spec 011 data-model §1; research R4).
//!
//! Written from upstream's `s2point_region.{h,cc}` and `s2cell_union.{h,cc}` (`v0.14.0-40`):
//!
//! - **A point region is its point.** It contains a point exactly when the coordinates are
//!   equal as numbers ([`pr_contains`]): ±0 agree, and a NaN coordinate contains nothing. It
//!   contains no cell.
//! - **A union is the set of points whose leaf cell it contains**: upstream's
//!   `Contains(S2CellId(p))`, through 002's [`crate::union_spec::contains_id`] on the leaf.
//! - **A bound contains what it bounds**: a union's rectangle bound contains each cell's
//!   rectangle bound, its cap bound each cell's cap bound, and its covering is the cap bound's.
//!
//! Upstream's constructions (the area-weighted centroid, the fallback centre, the folds, the
//! sums from +0, `Expand`'s level arithmetic) are not restated as arithmetic: a restatement that
//! recomputes float products times out in the checker (specs 008 and 010). `region_spec_tests`
//! pins the library's text of each to upstream's (the drift guards), and the C++ vectors check
//! the values (`core/tests/cpp_region_diff.rs`).
//!
//! The description never calls the operations it describes (FR-007). Booleans are combined with
//! `&&`, `||` and `==` only.

use s2rst::s2::Point;
use std::f64::consts::PI;

/// Upstream's `S2PointRegion::Contains(p)`: `point_ == p`, coordinate by coordinate as numbers.
pub fn pr_contains(p: Point, q: Point) -> bool {
    p.0.x == q.0.x && p.0.y == q.0.y && p.0.z == q.0.z
}

/// Upstream's `S2Cell::AverageArea(level)`, `kAvgArea.GetValue(level)`:
/// `ldexp(4π/6, −2·level)`, a power-of-two scaling of `4π/6` (exact for every level, 0 to 30).
pub fn upstream_average_area(level: u8) -> f64 {
    assert!(level <= 30, "a level is 0..=30");
    let scale = f64::from_bits((1023 - 2 * u64::from(level)) << 52);
    (4.0 * PI / 6.0) * scale
}

#[cfg(test)]
#[path = "region_spec_tests.rs"]
mod region_spec_tests;
