// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 012 planning probe (research R0): what `Cell`'s bounding laws for its own points cost,
//! and whether the region adaptors' preservation laws prove over abstract members
//! (`any_region`). Included in the verification crate through a temporary `#[path]` module, as
//! spec 011's R0; run with `VERIFY_PLAYBACK=0 VERIFY_RUN_DIR=target/run-r012 ./run-verify.sh
//! --harness r0::r0_`.

use s2rst::s2::coords::face_uv_to_xyz;
use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;
use s2rst::s2::{CellVertex, Point, Region};

use crate::any_region::{members, reset};
use crate::laws;
use crate::value_domain::{any_cell, any_plain_triple, any_triple};

/// A point of the cell's own u-v bound, normalised.
fn own_point(c: s2rst::s2::Cell) -> Point {
    let uv = c.bound_uv();
    let (u, v): (f64, f64) = (kani::any(), kani::any());
    kani::assume(uv.x.lo <= u && u <= uv.x.hi && uv.y.lo <= v && v <= uv.y.hi);
    Point(face_uv_to_xyz(c.face(), u, v).normalize())
}

// ---- Cell's bounding laws for its own points -------------------------------------------------

/// The cap bound contains every point of the cell's own u-v bound (CBMC's `sqrt`).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_cell_cap_own() {
    let (c, _) = any_cell();
    let p = own_point(c);
    assert!(c.cap_bound().contains_point(p));
}

/// The same with `sqrt_between`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
#[kani::stub(f64::sqrt, crate::stubs::sqrt_between)]
fn r0_cell_cap_own_sqrtb() {
    let (c, _) = any_cell();
    let p = own_point(c);
    assert!(c.cap_bound().contains_point(p));
}

/// A vertex only (the cap's `add_point` makes the same chord computation).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
#[kani::stub(f64::sqrt, crate::stubs::pure_sqrt)]
fn r0_cell_cap_vertex() {
    let (c, _) = any_cell();
    let k: u8 = kani::any();
    kani::assume(k < 4);
    let v = c.vertex(CellVertex::ALL[usize::from(k)]);
    assert!(c.cap_bound().contains_point(v));
}

/// The rectangle bound contains every point of the cell's own u-v bound (`atan2_contract`).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn r0_cell_rect_own() {
    let (c, _) = any_cell();
    let p = own_point(c);
    assert!(c.rect_bound().contains_point(p));
}

// ---- the adaptors: preservation over abstract members ----------------------------------------

macro_rules! preserve_ci {
    ($name:ident, $adaptor:ident, $n:literal) => {
        /// Containment implies intersection, preserved.
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            reset();
            let ms = members::<$n>();
            let (c, _) = any_cell();
            for m in &ms {
                kani::assume(!m.contains_cell(&c) || m.intersects_cell(&c));
            }
            let a = $adaptor::from_regions(ms);
            laws::region::contains_implies_intersects(&a, &c);
        }
    };
}
preserve_ci!(r0_ru_ci__n1, RegionUnion, 1);
preserve_ci!(r0_ru_ci__n3, RegionUnion, 3);
preserve_ci!(r0_ri_ci__n3, RegionIntersection, 3);

macro_rules! preserve_vertex {
    ($name:ident, $adaptor:ident, $n:literal) => {
        /// The vertex law, preserved (abstract cell vertices).
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        #[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
        fn $name() {
            reset();
            crate::stubs::draw_abstract_cell();
            let ms = members::<$n>();
            let (c, _) = any_cell();
            for m in &ms {
                if !m.intersects_cell(&c) {
                    for k in CellVertex::ALL {
                        kani::assume(!m.contains_point(&c.vertex(k)));
                    }
                }
            }
            let a = $adaptor::from_regions(ms);
            laws::region::no_intersect_no_vertex(&a, &c);
        }
    };
}
preserve_vertex!(r0_ru_vertex__n3, RegionUnion, 3);
preserve_vertex!(r0_ri_vertex__n3, RegionIntersection, 3);

macro_rules! preserve_rect {
    ($name:ident, $adaptor:ident, $n:literal) => {
        /// The bounding-rectangle law, preserved (pure `from_point`).
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
        fn $name() {
            reset();
            crate::stubs::reset();
            let ms = members::<$n>();
            let p = any_plain_triple();
            for m in &ms {
                kani::assume(!m.contains_point(&p) || m.rect_bound().contains_point(p));
            }
            let a = $adaptor::from_regions(ms);
            laws::region::rect_bound_contains_points(&a, &p);
        }
    };
}
preserve_rect!(r0_ru_rect__n1, RegionUnion, 1);
preserve_rect!(r0_ru_rect__n2, RegionUnion, 2);
preserve_rect!(r0_ru_rect__n3, RegionUnion, 3);
preserve_rect!(r0_ri_rect__n2, RegionIntersection, 2);
preserve_rect!(r0_ri_rect__n3, RegionIntersection, 3);

macro_rules! adaptor_crash {
    ($name:ident, $adaptor:ident, $n:literal) => {
        /// Every `Region` method of the adaptor completes, over abstract members.
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        #[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
        #[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
        #[kani::stub(f64::asin, crate::stubs::asin_contract)]
        fn $name() {
            reset();
            crate::stubs::reset();
            let a = $adaptor::from_regions(members::<$n>());
            let (c, _) = any_cell();
            let p = any_triple();
            let _ = (a.contains_cell(&c), a.intersects_cell(&c), a.contains_point(&p));
            let r = a.rect_bound();
            let cap = a.cap_bound();
            kani::cover!(!r.is_empty() && !cap.is_empty(), "non-empty bounds");
        }
    };
}
adaptor_crash!(r0_ru_crash__n3, RegionUnion, 3);
adaptor_crash!(r0_ri_crash__n3, RegionIntersection, 3);

/// `RegionIntersection::region(i)` panics past the end (K2).
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(5)]
fn r0_ri_region_past_end() {
    reset();
    let a = RegionIntersection::from_regions(members::<2>());
    let i: usize = kani::any();
    kani::assume(i >= 2);
    let _ = a.region(i);
    kani::cover!(true, "returns");
}

// ---- R0b: smaller member counts ---------------------------------------------------------------

preserve_vertex!(r0b_ru_vertex__n1, RegionUnion, 1);
preserve_vertex!(r0b_ru_vertex__n2, RegionUnion, 2);
preserve_vertex!(r0b_ri_vertex__n2, RegionIntersection, 2);
adaptor_crash!(r0b_ru_crash__n1, RegionUnion, 1);
adaptor_crash!(r0b_ri_crash__n2, RegionIntersection, 2);
