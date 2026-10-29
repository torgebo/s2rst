// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `PointRegion`'s cell and point tests (spec 011 data-model §4.1): upstream's
//! `Contains(cell)` (false), `MayIntersect(cell)` (`cell.Contains(point_)`) and `Contains(p)`
//! (`point_ == p`).

use s2rst::s2::Region;
use s2rst::s2::point_region::PointRegion;

use crate::region_spec::pr_contains;
use crate::value_domain::{any_cell, any_triple};

/// P-PRCEL-01 (correct result): the region contains no cell.
///
/// Exactly: for every triple and every valid cell, `contains_cell(c)` is false.
///
/// Stubs: `any_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn p_prcel_01() {
    let r = PointRegion::new(any_triple());
    let (c, _) = any_cell();
    assert!(!r.contains_cell(&c));
}

/// P-PRCEL-02 (correct result): the region intersects a cell exactly when the cell contains the
/// point (upstream's `MayIntersect`).
///
/// Exactly: for every triple and every valid cell, `intersects_cell(c) == c.contains_point(p)`.
///
/// Stubs: `pure_cell_contains` (uninterpreted, one answer per cell and point: comparing two
/// copies of the real test's divisions timed out at 12 minutes, 2026-10-28), `any_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
#[kani::stub(s2rst::s2::Cell::contains_point, crate::stubs::pure_cell_contains)]
fn p_prcel_02() {
    crate::stubs::reset();
    let p = any_triple();
    let (c, _) = any_cell();
    let x = PointRegion::new(p).intersects_cell(&c);
    assert!(x == c.contains_point(p));
    kani::cover!(x, "a cell that holds the point");
}

/// P-PRCEL-03 (correct result): the region contains exactly the points equal to its own as
/// numbers ([`pr_contains`]).
///
/// Exactly: for every pair of triples, `contains_point(q) == pr_contains(p, q)`.
#[kani::proof]
fn p_prcel_03() {
    let (p, q) = (any_triple(), any_triple());
    let x = PointRegion::new(p).contains_point(&q);
    assert!(x == pr_contains(p, q));
    kani::cover!(
        x && q.0.x.to_bits() != p.0.x.to_bits(),
        "equal with another zero's sign"
    );
}
