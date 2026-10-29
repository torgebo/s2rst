// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 011 planning probe (research R0): Kani harnesses that measure what `PointRegion`'s
//! operations and `Region` laws, and `CellUnion`'s floating-point methods, cost the checker,
//! and which stubs they need.
//!
//! They run inside the verification crate, so they reuse its domains and stubs: a temporary
//! `#[cfg(kani)] #[path = "../../specs/011-formal-verify-point-region-cell-union/probe/r0_harnesses.rs"] mod r0;`
//! in `verify/src/lib.rs` (not committed), run with
//! `VERIFY_RUN_DIR=target/run-r011 ./run-verify.sh --harness r0::`. Results go to research.md R0.

use s2rst::s2::point_region::PointRegion;
use s2rst::s2::{Cap, Cell, CellId, CellUnion, Point, Rect, Region};
use std::sync::Mutex;

use crate::domain;
use crate::union_domain::any_valid_union;
use crate::value_domain::{
    any_box_point, any_cell, any_cell_vertex, any_plain_triple, any_triple, any_valid_cap,
};

// ---- local pure stubs ---------------------------------------------------------------------------

static LEAF: Mutex<([[u64; 3]; 4], [u64; 4], usize)> = Mutex::new(([[0; 3]; 4], [0; 4], 0));

/// `CellId::from_point`, pure: any valid leaf cell, one per point.
pub fn pure_leaf(p: &Point) -> CellId {
    let k = [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()];
    let mut t = LEAF.lock().unwrap();
    for i in 0..4 {
        if i < t.2 && t.0[i][0] == k[0] && t.0[i][1] == k[1] && t.0[i][2] == k[2] {
            return CellId(t.1[i]);
        }
    }
    let (id, _) = domain::any_valid_at(30);
    if t.2 < 4 {
        let n = t.2;
        t.0[n] = k;
        t.1[n] = id.id();
        t.2 += 1;
    }
    id
}

static CAPS: Mutex<([u64; 4], [[u64; 4]; 4], usize)> = Mutex::new(([0; 4], [[0; 4]; 4], 0));

/// `Cell::cap_bound`, pure: any cap with a unit centre and a radius in [0, 4], one per cell.
pub fn pure_cell_cap(c: Cell) -> Cap {
    let id = c.id().id();
    let mut t = CAPS.lock().unwrap();
    for i in 0..4 {
        if i < t.2 && t.0[i] == id {
            let v = t.1[i];
            return Cap::from_center_chord_angle(
                Point(s2rst::r3::Vector {
                    x: f64::from_bits(v[0]),
                    y: f64::from_bits(v[1]),
                    z: f64::from_bits(v[2]),
                }),
                s2rst::s1::ChordAngle::from_length2(f64::from_bits(v[3])),
            );
        }
    }
    let cap = any_valid_cap();
    kani::assume(!cap.is_empty());
    if t.2 < 4 {
        let n = t.2;
        t.0[n] = id;
        let p = cap.center();
        t.1[n] = [
            p.0.x.to_bits(),
            p.0.y.to_bits(),
            p.0.z.to_bits(),
            cap.chord_radius().length2().to_bits(),
        ];
        t.2 += 1;
    }
    cap
}

/// `CellId::to_point`, abstracted: any box point.
pub fn any_center(_id: CellId) -> Point {
    any_box_point()
}

static AREAS: Mutex<([u64; 4], [u64; 4], usize)> = Mutex::new(([0; 4], [0; 4], 0));

/// `Cell::approx_area`, pure: any positive finite value, one per cell.
pub fn pure_area(c: Cell) -> f64 {
    let id = c.id().id();
    let mut t = AREAS.lock().unwrap();
    for i in 0..4 {
        if i < t.2 && t.0[i] == id {
            return f64::from_bits(t.1[i]);
        }
    }
    let a: f64 = kani::any();
    kani::assume(a > 0.0 && a <= 4.0 * std::f64::consts::PI);
    if t.2 < 4 {
        let n = t.2;
        t.0[n] = id;
        t.1[n] = a.to_bits();
        t.2 += 1;
    }
    a
}

fn same(a: Point, b: Point) -> bool {
    a.0.x.to_bits() == b.0.x.to_bits()
        && a.0.y.to_bits() == b.0.y.to_bits()
        && a.0.z.to_bits() == b.0.z.to_bits()
}

// ---- PointRegion: crash freedom -----------------------------------------------------------------

/// `new`, `point`, `cap_bound`, `contains_cell`, `contains_point` over every triple; `point`
/// returns `new`'s argument bit for bit.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_pr_basic() {
    let p = any_triple();
    let q = any_triple();
    let r = PointRegion::new(p);
    assert!(same(r.point(), p));
    let _ = r.cap_bound();
    let (c, _) = any_cell();
    assert!(!r.contains_cell(&c));
    assert!(r.contains_point(&q) == (p == q));
    kani::cover!(r.contains_point(&q), "contained");
}

/// `rect_bound` over every triple, under `atan2`'s contract.
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn r0_pr_rect_bound() {
    let r = PointRegion::new(any_triple());
    let b = r.rect_bound();
    kani::cover!(!b.is_empty(), "non-empty");
}

/// `intersects_cell` over every triple and every valid cell (abstract u-v bound).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_pr_intersects() {
    let r = PointRegion::new(any_triple());
    let (c, _) = any_cell();
    let x = r.intersects_cell(&c);
    kani::cover!(x, "intersects");
}

/// `cell_union_bound` over every triple, the real `CellId::from_point`, `asin`'s contract.
#[kani::proof]
#[kani::unwind(8)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn r0_pr_cub_real() {
    let r = PointRegion::new(any_triple());
    let ids = r.cell_union_bound();
    kani::cover!(ids.len() == 4, "four cells");
}

/// `cell_union_bound` over every triple, `CellId::from_point` as any leaf.
#[kani::proof]
#[kani::unwind(8)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::CellId::from_point, pure_leaf)]
fn r0_pr_cub_stub() {
    let r = PointRegion::new(any_triple());
    let ids = r.cell_union_bound();
    kani::cover!(ids.len() == 4, "four cells");
}

// ---- PointRegion: the laws ----------------------------------------------------------------------

/// The vertex law: the region of each of a cell's vertices intersects the cell (the real vertex,
/// the real `contains_point`, the abstract u-v bound, CBMC's own `sqrt`).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_pr_law_vertex() {
    let (c, _) = any_cell();
    let v = c.vertex(any_cell_vertex());
    assert!(PointRegion::new(v).intersects_cell(&c));
}

/// The same with `sqrt`'s contract.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
#[kani::stub(f64::sqrt, crate::stubs::sqrt_contract)]
fn r0_pr_law_vertex_sqrtc() {
    let (c, _) = any_cell();
    let v = c.vertex(any_cell_vertex());
    assert!(PointRegion::new(v).intersects_cell(&c));
}

/// The bounding cap contains the point, every box point.
#[kani::proof]
fn r0_pr_law_cap() {
    let p = any_box_point();
    let r = PointRegion::new(p);
    crate::laws::region::cap_bound_contains_points(&r, &p);
}

/// The bounding rectangle contains the point, every triple without NaN (pure `from_point`).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
fn r0_pr_law_rect() {
    crate::stubs::reset();
    let p = any_plain_triple();
    let r = PointRegion::new(p);
    assert!(r.rect_bound().contains_point(p));
}

/// The covering contains the point: some id of the bound contains the point's leaf (pure
/// `from_point`, `asin`'s contract), every box point.
#[kani::proof]
#[kani::unwind(8)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::CellId::from_point, pure_leaf)]
fn r0_pr_law_cover() {
    let p = any_box_point();
    let r = PointRegion::new(p);
    let ids = r.cell_union_bound();
    let leaf = CellId::from_point(&p);
    let mut covered = false;
    for k in 0..6 {
        if k < ids.len() && ids[k].contains(leaf) {
            covered = true;
        }
    }
    assert!(ids.len() <= 6);
    assert!(covered);
}

// ---- CellUnion: the floating-point methods ------------------------------------------------------

macro_rules! cu_contains_point {
    ($name:ident, $n:literal) => {
        /// `contains_point` is `contains_cell_id` of the point's leaf (pure `from_point`).
        #[kani::proof]
        #[kani::unwind(5)]
        #[kani::stub(s2rst::s2::CellId::from_point, pure_leaf)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v);
            let p = any_triple();
            let leaf = CellId::from_point(&p);
            assert!(u.contains_point(p) == u.contains_cell_id(leaf));
            kani::cover!(u.contains_point(p), "contained");
        }
    };
}
cu_contains_point!(r0_cu_contains_point__n1, 1);
cu_contains_point!(r0_cu_contains_point__n3, 3);

macro_rules! cu_rect_bound {
    ($name:ident, $n:literal) => {
        /// `rect_bound` contains each cell's rectangle bound (pure any bound per cell).
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::Cell::rect_bound, crate::stubs::any_cell_bound)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v.clone());
            let b: Rect = u.rect_bound();
            for k in 0..$n {
                assert!(b.contains(Cell::from_cell_id(v[k]).rect_bound()));
            }
        }
    };
}
cu_rect_bound!(r0_cu_rect_bound__n1, 1);
cu_rect_bound!(r0_cu_rect_bound__n2, 2);
cu_rect_bound!(r0_cu_rect_bound__n3, 3);

/// `rect_bound` with the real `Cell::rect_bound` (crash only), one cell, `atan2`'s contract.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_cu_rect_bound_real__n1() {
    let (v, _) = any_valid_union::<1>();
    let _ = CellUnion::from_verbatim(v).rect_bound();
}

macro_rules! cu_cap_bound {
    ($name:ident, $n:literal) => {
        /// `cap_bound` contains each cell's cap bound (pure any cap per cell, any centre).
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::Cell::cap_bound, pure_cell_cap)]
        #[kani::stub(s2rst::s2::CellId::to_point, any_center)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v.clone());
            let b: Cap = u.cap_bound();
            for k in 0..$n {
                assert!(b.contains(Cell::from_cell_id(v[k]).cap_bound()));
            }
        }
    };
}
cu_cap_bound!(r0_cu_cap_bound__n1, 1);
cu_cap_bound!(r0_cu_cap_bound__n2, 2);

/// `cap_bound` with the stubs, crash only, two cells.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::Cell::cap_bound, pure_cell_cap)]
#[kani::stub(s2rst::s2::CellId::to_point, any_center)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_cu_cap_bound_crash__n2() {
    let (v, _) = any_valid_union::<2>();
    let _ = CellUnion::from_verbatim(v).cap_bound();
}

macro_rules! cu_avg_area {
    ($name:ident, $n:literal) => {
        /// `average_based_area` is the leaf count times the leaf area.
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v);
            let a = u.average_based_area();
            let e = Cell::average_area_for_level(s2rst::s2::Level::MAX)
                * u.leaf_cells_covered() as f64;
            assert!(a.to_bits() == e.to_bits());
        }
    };
}
cu_avg_area!(r0_cu_avg_area__n1, 1);
cu_avg_area!(r0_cu_avg_area__n3, 3);

/// The empty union's areas are +0 (K2: fails before FR-018).
#[kani::proof]
fn r0_cu_area_empty() {
    let u = CellUnion::new();
    assert!(u.approx_area().to_bits() == 0);
    assert!(u.exact_area().to_bits() == 0);
}

/// `approx_area` is the sum of the cells' areas in order (pure any area per cell).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::Cell::approx_area, pure_area)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_cu_approx_area__n3() {
    let (v, _) = any_valid_union::<3>();
    let u = CellUnion::from_verbatim(v.clone());
    let mut s = 0.0;
    for k in 0..3 {
        s += Cell::from_cell_id(v[k]).approx_area();
    }
    assert!(u.approx_area().to_bits() == s.to_bits());
}

/// The vertex law fails for `CellUnion` (expected FAILED: a shared vertex of a neighbour).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::CellId::from_point, pure_leaf)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_cu_law_vertex__n1() {
    crate::stubs::draw_abstract_cell();
    let (v, _) = any_valid_union::<1>();
    let u = CellUnion::from_verbatim(v);
    let (c, _) = any_cell();
    crate::laws::region::no_intersect_no_vertex(&u, &c);
}

/// Containment implies intersection for `CellUnion` (integer).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0_cu_law_contains__n3() {
    let (v, _) = any_valid_union::<3>();
    let u = CellUnion::from_verbatim(v);
    let (c, _) = any_cell();
    crate::laws::region::contains_implies_intersects(&u, &c);
}

// ---- R0b: the remedies --------------------------------------------------------------------------

/// Is `powi` a function to the checker? (`average_area_for_level` is `4π / (6 · 4^level)`.)
#[kani::proof]
fn r0b_powi_deterministic() {
    let n: i32 = kani::any();
    kani::assume((0..=30).contains(&n));
    assert!(4.0_f64.powi(n).to_bits() == 4.0_f64.powi(n).to_bits());
}

static AVG: Mutex<([u8; 4], [u64; 4], usize)> = Mutex::new(([0; 4], [0; 4], 0));

/// `Cell::average_area_for_level`, pure: any positive value, one per level.
pub fn pure_avg_area(level: s2rst::s2::Level) -> f64 {
    let l = level.as_u8();
    let mut t = AVG.lock().unwrap();
    for i in 0..4 {
        if i < t.2 && t.0[i] == l {
            return f64::from_bits(t.1[i]);
        }
    }
    let a: f64 = kani::any();
    kani::assume(a > 0.0 && a.is_finite());
    if t.2 < 4 {
        let n = t.2;
        t.0[n] = l;
        t.1[n] = a.to_bits();
        t.2 += 1;
    }
    a
}

macro_rules! cu_avg_area_pure {
    ($name:ident, $n:literal) => {
        /// `average_based_area` is the leaf count times the leaf area (pure leaf area).
        #[kani::proof]
        #[kani::unwind(5)]
        #[kani::stub(s2rst::s2::Cell::average_area_for_level, pure_avg_area)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v);
            let a = u.average_based_area();
            let e = Cell::average_area_for_level(s2rst::s2::Level::MAX)
                * u.leaf_cells_covered() as f64;
            assert!(a.to_bits() == e.to_bits());
        }
    };
}
cu_avg_area_pure!(r0b_cu_avg_area__n1, 1);
cu_avg_area_pure!(r0b_cu_avg_area__n3, 3);

/// The covering contains the point, unwind 11.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::CellId::from_point, pure_leaf)]
fn r0b_pr_law_cover() {
    let p = any_box_point();
    let r = PointRegion::new(p);
    let ids = r.cell_union_bound();
    let leaf = CellId::from_point(&p);
    let mut covered = false;
    for k in 0..6 {
        if k < ids.len() && ids[k].contains(leaf) {
            covered = true;
        }
    }
    assert!(ids.len() <= 6);
    assert!(covered);
}

/// `rect_bound` over every triple without NaN (R0: a NaN `x` or `y` fails `s1::Interval`'s
/// debug check, as upstream's `DCHECK`).
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn r0b_pr_rect_bound_plain() {
    let r = PointRegion::new(any_plain_triple());
    let b = r.rect_bound();
    kani::cover!(!b.is_empty(), "non-empty");
}

/// `intersects_cell` over every box point (R0: a NaN on the face's axis fails
/// `valid_face_xyz_to_uv`'s debug check, as upstream's `DCHECK`).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn r0b_pr_intersects_box() {
    let r = PointRegion::new(any_box_point());
    let (c, _) = any_cell();
    let x = r.intersects_cell(&c);
    kani::cover!(x, "intersects");
}

/// `cell_union_bound`, pure leaf, unwind 11.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::CellId::from_point, pure_leaf)]
fn r0b_pr_cub_stub() {
    let r = PointRegion::new(any_triple());
    let ids = r.cell_union_bound();
    kani::cover!(ids.len() == 4, "four cells");
}

/// `cell_union_bound`, the real `from_point`, every box point, unwind 11.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn r0b_pr_cub_real() {
    let r = PointRegion::new(any_box_point());
    let ids = r.cell_union_bound();
    kani::cover!(ids.len() == 4, "four cells");
}

// ---- R0c: a tighter sqrt contract for the vertex law --------------------------------------------

/// `sqrt_contract` plus: the root lies between `x` and 1 (true of the correctly rounded root,
/// since `x` and 1 are representable). R0b: the plain contract's root may be 10³⁰⁰, which makes
/// the normalised vertex underflow.
pub fn sqrt_between(x: f64) -> f64 {
    let r = crate::stubs::sqrt_contract(x);
    if x.is_finite() && x > 0.0 {
        kani::assume(if x >= 1.0 { r <= x } else { r >= x });
    }
    r
}

/// The vertex law with `sqrt_between`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
#[kani::stub(f64::sqrt, sqrt_between)]
fn r0c_pr_law_vertex() {
    let (c, _) = any_cell();
    let v = c.vertex(any_cell_vertex());
    assert!(PointRegion::new(v).intersects_cell(&c));
}

macro_rules! cu_cap_bound_pure {
    ($name:ident, $n:literal) => {
        /// `cap_bound` contains each cell's cap bound, with 008's `pure_chord` and `pure_add`
        /// (P-CPGRO-02's stubs) besides `pure_cell_cap` and `any_center`.
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::Cell::cap_bound, pure_cell_cap)]
        #[kani::stub(s2rst::s2::CellId::to_point, any_center)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        #[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
        #[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, crate::stubs::pure_add)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v.clone());
            let b: Cap = u.cap_bound();
            for k in 0..$n {
                assert!(b.contains(Cell::from_cell_id(v[k]).cap_bound()));
            }
        }
    };
}
cu_cap_bound_pure!(r0c_cu_cap_bound__n1, 1);
cu_cap_bound_pure!(r0c_cu_cap_bound__n2, 2);
cu_avg_area_pure!(r0c_cu_avg_area__n2, 2);
