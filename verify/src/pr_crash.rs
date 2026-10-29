// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash freedom of `PointRegion` (spec 011 US1, FR-005; data-model §5.1; research R0).
//!
//! A `PointRegion` holds any point: `new` does not check upstream's unit-length precondition
//! (spec 011 K3), so the domain is every triple of `f64`, narrowed only where a documented
//! debug-build check of the code it calls applies (spec 011 K4): `rect_bound` for a NaN `x` or
//! `y`, `intersects_cell` where `Cell::contains_point`'s face check fails, `cell_union_bound`
//! where `CellId::from_point`'s does. The `n_*` harnesses prove those panics exact, the `r_*`
//! ones that a release build returns on the same inputs (`make verify-release`).

use s2rst::s2::Region;
use s2rst::s2::point_region::PointRegion;

use crate::value_domain::{any_box_point, any_cell, any_plain_triple, any_triple};

/// C-PointRegion::new, point, cap_bound, contains_cell, contains_point, `PartialEq`: complete
/// without panic, overflow or undefined behaviour for every triple and every valid cell.
///
/// Stubs: `any_uv_bound` (the abstract u-v bound; the cell is only passed through).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_basic() {
    let (p, q) = (any_triple(), any_triple());
    let (r, s) = (PointRegion::new(p), PointRegion::new(q));
    let _ = r.point();
    let _ = r.cap_bound();
    let (c, _) = any_cell();
    let _ = r.contains_cell(&c);
    let x = r.contains_point(&q);
    let e = r == s;
    kani::cover!(x, "a contained point");
    kani::cover!(!e, "two regions that differ");
}

/// C-PointRegion::rect_bound: completes for every triple without a NaN coordinate.
///
/// Under contract: `atan2_contract` (`LatLng::from_point`).
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn c_rect_bound() {
    let r = PointRegion::new(any_plain_triple());
    let b = r.rect_bound();
    kani::cover!(!b.is_empty(), "a non-empty bound");
}

/// N-PointRegion::rect_bound: in debug builds, panics for every triple with a NaN `x` or `y`:
/// the longitude `atan2(y + 0, x + 0)` is NaN, which `s1::Interval::from_point` rejects (spec
/// 011 K4; upstream's `S1Interval` constructor `DCHECK`s it).
///
/// Under contract: `atan2_contract` (NaN for a NaN argument, as C11 specifies).
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn n_rect_bound_nan() {
    let p = any_triple();
    kani::assume(p.0.x.is_nan() || p.0.y.is_nan());
    let _ = PointRegion::new(p).rect_bound();
    kani::cover!(true, "returns");
}

/// R-PointRegion::rect_bound: in a build without debug assertions, completes for every triple.
///
/// Under contract: `atan2_contract`.
#[cfg(not(debug_assertions))]
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn r_rect_bound() {
    let b = PointRegion::new(any_triple()).rect_bound();
    kani::cover!(b.lng.lo.is_nan(), "a NaN longitude");
}

/// C-PointRegion::intersects_cell: completes for every box point (which holds every unit point)
/// and every valid cell, as `Cell::contains_point`'s own proof (`cl_crash::c_fg_contains_point`).
///
/// Stubs: `any_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_intersects_cell() {
    let r = PointRegion::new(any_box_point());
    let (c, _) = any_cell();
    let x = r.intersects_cell(&c);
    kani::cover!(x, "a cell that holds the point");
}

/// N-PointRegion::intersects_cell, N-Cell::contains_point: in debug builds, panics for every
/// valid cell and every triple whose coordinate on the cell face's axis is NaN, or has the face's
/// sign while another coordinate is NaN: the face projection checks `p · n > 0` (spec 011 K4;
/// upstream's `ABSL_DCHECK` in `S2::ValidFaceXYZtoUV`). The documented third case, another
/// coordinate infinite, panics too (`∞ · 0` is NaN), but the checker reports that product as a
/// NaN check before the panic (KI-006's kind; the first run, 2026-10-28), so it is the native
/// `cell_tests::contains_point_panic_tests::infinity_off_the_axis_on_the_faces_side` and the
/// probe's sweep (`probe/rust/src/bin/panics.rs`, 0 mismatches over 1,000 special triples).
///
/// Stubs: `any_uv_bound`.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn n_intersects_cell() {
    let p = any_triple();
    let (c, _) = any_cell();
    let f = c.face().as_u8();
    let coords = [p.0.x, p.0.y, p.0.z];
    let a = coords[usize::from(f % 3)];
    let side = if f < 3 { a > 0.0 } else { a < 0.0 };
    let mut other = false;
    for (j, &t) in coords.iter().enumerate() {
        if j != usize::from(f % 3) {
            kani::assume(!t.is_infinite());
            if t.is_nan() {
                other = true;
            }
        }
    }
    kani::assume(a.is_nan() || (side && other));
    let _ = PointRegion::new(p).intersects_cell(&c);
    kani::cover!(true, "returns");
}

/// R-PointRegion::intersects_cell: in a build without debug assertions, completes for every
/// triple and every valid cell.
///
/// Stubs: `any_uv_bound`.
#[cfg(not(debug_assertions))]
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r_intersects_cell() {
    let r = PointRegion::new(any_triple());
    let (c, _) = any_cell();
    let x = r.intersects_cell(&c);
    kani::cover!(x, "a cell that holds the point");
}

/// C-PointRegion::cell_union_bound: completes for every triple, given `CellId::from_point`'s
/// leaf (whose own crash proof, `cl_crash::c_fg_from_point`, and documented panics,
/// `cl_crash::n_from_point` and the native tests of two infinite coordinates, are its own): the covering of the
/// zero-radius cap, the cells around the leaf at level 29.
///
/// Under contract: `pure_leaf` (L-PR-01), `asin_contract` (the cap's angle). Weekly (R0b:
/// 563.4 s).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::CellId::from_point, crate::stubs::pure_leaf)]
fn c_cell_union_bound() {
    crate::stubs::reset();
    let ids = PointRegion::new(any_triple()).cell_union_bound();
    kani::cover!(ids.len() == 4, "four cells");
    kani::cover!(ids.len() == 3, "three cells, at a face corner");
}
