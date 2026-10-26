// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 008 planning probe, second round (research R0b). R0 showed two cost patterns: a law
//! that recomputes float products the operation already computed times out (`add_point`,
//! `is_unit` assumed then checked, `contains` against `intersects`), and the checker's `cos`
//! and `log2` models are loose. This round measures three remedies:
//!
//! - **pure-function stubs** (`memo_*`): a deterministic function's repeated calls on one input
//!   return one freely chosen value, within a range a separate harness proves. Sound, because
//!   the library's functions are pure; dropping the memo only adds behaviours;
//! - **lemma stubs**: an operation replaced by any value satisfying a lemma proven separately
//!   on the real code (`lemma_*`);
//! - **a `cos` contract**, as R0 needed for `rect_bound`.

use s2rst::r3::Vector;
use s2rst::s1::{self, ChordAngle};
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::{Cap, Cell, CellEdge, LatLng, Point, Rect};
use std::sync::Mutex;
use std::f64::consts::{FRAC_PI_2, PI};

// ---- contracts --------------------------------------------------------------------------------

fn asin_contract(x: f64) -> f64 {
    if x.is_nan() || x.abs() > 1.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return x;
    }
    let r: f64 = kani::any();
    kani::assume(r.abs() <= FRAC_PI_2);
    kani::assume(if x > 0.0 { r >= 0.0 } else { r <= 0.0 });
    r
}

fn atan2_contract(y: f64, x: f64) -> f64 {
    if y.is_nan() || x.is_nan() {
        return f64::NAN;
    }
    let r: f64 = kani::any();
    kani::assume(r.abs() <= PI);
    kani::assume(if y.is_sign_positive() { r >= 0.0 } else { r <= 0.0 });
    if x.is_sign_positive() {
        kani::assume(r.abs() <= FRAC_PI_2);
    }
    r
}

/// `cos`'s contract: NaN for a non-finite argument; otherwise in [−1, 1], and at least
/// 6·10⁻¹⁷ (cos of the `f64` nearest π/2 is 6.12·10⁻¹⁷) wherever |x| ≤ `FRAC_PI_2`; 1 at ±0.
fn cos_contract(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 1.0;
    }
    let r: f64 = kani::any();
    kani::assume(r.abs() <= 1.0);
    if x.abs() <= FRAC_PI_2 {
        kani::assume(r >= 6.0e-17);
    }
    r
}

// ---- pure-function stubs --------------------------------------------------------------------

static IS_UNIT: Mutex<Vec<([u64; 3], bool)>> = Mutex::new(Vec::new());
static CHORD: Mutex<Vec<([u64; 6], f64)>> = Mutex::new(Vec::new());
static ADD: Mutex<Vec<([u64; 2], f64)>> = Mutex::new(Vec::new());

/// Element-wise key comparison: `==` on arrays lowers to `compare_bytes`, which crashes Kani
/// 0.68.0's compiler inside a stub.
fn same<const N: usize>(a: &[u64; N], b: &[u64; N]) -> bool {
    let mut eq = true;
    for i in 0..N {
        eq &= a[i] == b[i];
    }
    eq
}

fn key3(p: Point) -> [u64; 3] {
    [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()]
}

/// `Point::is_unit` as a pure function of its bits, with the sign of each coordinate ignored
/// (lemma `lemma_norm2_neg`: negating a coordinate keeps the squared length bit for bit).
fn memo_is_unit(p: Point) -> bool {
    let k = [p.0.x.abs().to_bits(), p.0.y.abs().to_bits(), p.0.z.abs().to_bits()];
    {
        let mut m = IS_UNIT.lock().unwrap();
        for (kk, v) in m.iter().take(4) {
            if same(kk, &k) {
                return *v;
            }
        }
        let v: bool = kani::any();
        m.push((k, v));
        v
    }
}

/// `Point::chord_angle` as a pure function: any length in [0, 4] (its range, lemma
/// `lemma_chord_range`), the same for the same pair.
fn memo_chord(a: Point, b: Point) -> ChordAngle {
    let ka = key3(a);
    let kb = key3(b);
    let k = [ka[0], ka[1], ka[2], kb[0], kb[1], kb[2]];
    {
        let mut m = CHORD.lock().unwrap();
        for (kk, v) in m.iter().take(4) {
            if same(kk, &k) {
                return ChordAngle::from_length2(*v);
            }
        }
        let v: f64 = kani::any();
        kani::assume((0.0..=4.0).contains(&v));
        m.push((k, v));
        ChordAngle::from_length2(v)
    }
}

/// `ChordAngle + ChordAngle` as a pure function satisfying `lemma_add_bounds`: the sum is in
/// [0, 4] and at least each operand.
fn memo_add(a: ChordAngle, b: ChordAngle) -> ChordAngle {
    let k = [a.length2().to_bits(), b.length2().to_bits()];
    {
        let mut m = ADD.lock().unwrap();
        for (kk, v) in m.iter().take(4) {
            if same(kk, &k) {
                return ChordAngle::from_length2(*v);
            }
        }
        let v: f64 = kani::any();
        kani::assume((0.0..=4.0).contains(&v));
        kani::assume(v >= a.length2() && v >= b.length2());
        m.push((k, v));
        ChordAngle::from_length2(v)
    }
}

// ---- domains ----------------------------------------------------------------------------------

fn any_box_point() -> Point {
    let (x, y, z): (f64, f64, f64) = (kani::any(), kani::any(), kani::any());
    kani::assume(x.abs() <= 1.0 && y.abs() <= 1.0 && z.abs() <= 1.0);
    Point(Vector { x, y, z })
}

fn any_radius() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume(x == -1.0 || (0.0..=4.0).contains(&x));
    ChordAngle::from_length2(x)
}

fn any_length() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume((0.0..=4.0).contains(&x));
    ChordAngle::from_length2(x)
}

fn any_box_cap() -> Cap {
    Cap::from_center_chord_angle(any_box_point(), any_radius())
}

/// A valid cap through the pure `is_unit`: the assumption and every later check agree.
fn any_valid_cap_memo() -> Cap {
    let c = any_box_cap();
    kani::assume(c.center().is_unit());
    c
}

// ---- 1. lemmas on the real code -------------------------------------------------------------

/// `lemma_chord_range`: the chord between two box points is a length in [0, 4].
#[kani::proof]
fn lemma_chord_range() {
    let (a, b) = (any_box_point(), any_box_point());
    let l = a.chord_angle(b).length2();
    assert!((0.0..=4.0).contains(&l));
}

/// `lemma_add_bounds`: the sum of two lengths is in [0, 4] and at least each operand.
#[kani::proof]
fn lemma_add_bounds() {
    let (a, b) = (any_length(), any_length());
    let s = (a + b).length2();
    assert!((0.0..=4.0).contains(&s));
    assert!(s >= a.length2() && s >= b.length2());
}

/// `lemma_neg_sub`: `b − a` is `−(a − b)` bit for bit (finite operands).
#[kani::proof]
fn lemma_neg_sub() {
    let (a, b): (f64, f64) = (kani::any(), kani::any());
    kani::assume(a.is_finite() && b.is_finite());
    assert!((b - a).to_bits() == (-(a - b)).to_bits() || (a - b) == 0.0);
}

/// `lemma_sq_neg`: `(−u)·(−u)` is `u·u` bit for bit.
#[kani::proof]
fn lemma_sq_neg() {
    let u: f64 = kani::any();
    kani::assume(u.is_finite());
    assert!(((-u) * (-u)).to_bits() == (u * u).to_bits());
}

/// `lemma_norm2_neg`: the squared length of `−p` is that of `p`, bit for bit.
#[kani::proof]
fn lemma_norm2_neg() {
    let p = any_box_point();
    assert!((-p).vector().norm2().to_bits() == p.vector().norm2().to_bits());
}

// ---- 2. the laws through pure-function stubs ---------------------------------------------------

/// After `add_point(p)` the cap contains `p` (R0: timeout).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
fn r0b_add_point_contains() {
    let c = any_box_cap();
    let p = any_box_point();
    assert!(c.add_point(p).contains_point(p));
}

/// `contains` a non-empty cap implies `intersects` it (R0: timeout).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, memo_add)]
fn r0b_contains_implies_intersects() {
    let a = any_box_cap();
    let b = Cap::from_center_chord_angle(any_box_point(), any_length());
    if a.contains(b) {
        assert!(a.intersects(b));
    }
}

/// After `add_cap(b)` the cap contains `b` (R0: timeout).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, memo_add)]
fn r0b_add_cap_contains() {
    let a = any_box_cap();
    let b = any_box_cap();
    assert!(a.add_cap(b).contains(b));
}

/// `from_center_angle` over a unit centre (R0: timeout): its validity check never fires.
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, memo_is_unit)]
fn r0b_from_center_angle_crash() {
    let c = any_box_point();
    kani::assume(c.is_unit());
    let a: f64 = kani::any();
    kani::assume(!a.is_nan());
    let _ = Cap::from_center_angle(c, s2rst::s1::Angle::from_radians(a));
}

/// The codec round trip under `Cap`'s equality (R0: timeout).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, memo_is_unit)]
fn r0b_codec_round_trip_eq() {
    let c = any_valid_cap_memo();
    let mut buf = Vec::with_capacity(32);
    c.encode(&mut buf).unwrap();
    let d = Cap::decode(&mut buf.as_slice()).unwrap();
    assert!(d == c);
}

/// The draft complement of a valid cap is valid (R0: timeout).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, memo_is_unit)]
fn r0b_complement_draft_valid() {
    let c = any_valid_cap_memo();
    let k = if c.is_full() {
        Cap::empty()
    } else if c.is_empty() {
        Cap::full()
    } else {
        Cap::from_center_chord_angle(-c.center(), ChordAngle::from_length2(4.0 - c.chord_radius().length2()))
    };
    assert!(k.is_valid());
}

// ---- 3. rect_bound with the cos contract ----------------------------------------------------

fn reduce_draft(x: f64) -> f64 {
    let y = 2.0 * PI;
    if x > PI {
        x - y
    } else if x < -PI {
        x + y
    } else {
        x
    }
}

fn rect_bound_draft(c: Cap) -> Rect {
    if c.is_empty() {
        return Rect::empty();
    }
    let cap_angle = c.angle_radius().radians();
    let center_lat = LatLng::latitude(c.center()).radians();
    let center_lng = LatLng::longitude(c.center()).radians();
    let mut all_longitudes = false;
    let mut lat = s2rst::r1::Interval::new(center_lat - cap_angle, center_lat + cap_angle);
    let mut lng = s1::Interval::full();
    if lat.lo <= -FRAC_PI_2 {
        lat.lo = -FRAC_PI_2;
        all_longitudes = true;
    }
    if lat.hi >= FRAC_PI_2 {
        lat.hi = FRAC_PI_2;
        all_longitudes = true;
    }
    if !all_longitudes {
        let sin_a = c.chord_radius().sin();
        let sin_c = center_lat.cos();
        if sin_a <= sin_c {
            let angle_a = (sin_a / sin_c).asin();
            lng = s1::Interval::new(reduce_draft(center_lng - angle_a), reduce_draft(center_lng + angle_a));
        }
    }
    Rect::new(lat, lng)
}

/// FR-020's draft returns a valid rectangle, under the asin, atan2 and cos contracts.
#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
#[kani::stub(f64::atan2, atan2_contract)]
#[kani::stub(f64::cos, cos_contract)]
fn r0b_rect_bound_draft_valid() {
    let c = any_box_cap();
    let r = rect_bound_draft(c);
    assert!(r.is_valid());
    assert!(r.is_empty() == c.is_empty());
}

/// The library's `rect_bound` under the same three contracts.
#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
#[kani::stub(f64::atan2, atan2_contract)]
#[kani::stub(f64::cos, cos_contract)]
fn r0b_rect_bound_library_valid() {
    let c = any_box_cap();
    assert!(c.rect_bound().is_valid());
}

// ---- 4. the cell laws, cell abstracted -------------------------------------------------------

static VERTS: Mutex<Vec<(usize, [u64; 3])>> = Mutex::new(Vec::new());
static EDGES: Mutex<Vec<(u8, [u64; 3])>> = Mutex::new(Vec::new());

/// `Cell::vertex` as a pure function of `k`: any box point (the cell abstracted away).
fn memo_vertex(_c: Cell, k: usize) -> Point {
    {
        let mut m = VERTS.lock().unwrap();
        for (kk, v) in m.iter().take(4) {
            if *kk == k {
                return Point(Vector { x: f64::from_bits(v[0]), y: f64::from_bits(v[1]), z: f64::from_bits(v[2]) });
            }
        }
        let p = any_box_point();
        m.push((k, key3(p)));
        p
    }
}

/// `Cell::edge_raw` as a pure function of the edge: any box vector.
fn memo_edge(_c: Cell, e: CellEdge) -> Point {
    let k = e as u8;
    {
        let mut m = EDGES.lock().unwrap();
        for (kk, v) in m.iter().take(4) {
            if *kk == k {
                return Point(Vector { x: f64::from_bits(v[0]), y: f64::from_bits(v[1]), z: f64::from_bits(v[2]) });
            }
        }
        let p = any_box_point();
        m.push((k, key3(p)));
        p
    }
}

fn any_contains_point(_c: Cell, _p: Point) -> bool {
    kani::any()
}

/// Upstream's edge test (s2cap.cc:225), over the vertices given.
fn intersects_edges(c: Cap, cell: Cell, v: &[Point; 4]) -> bool {
    if c.chord_radius() >= ChordAngle::RIGHT || c.is_empty() {
        return false;
    }
    if cell.contains_point(c.center()) {
        return true;
    }
    let sin2 = c.chord_radius().sin2();
    let cv = c.center().vector();
    for k in 0..4 {
        let e = cell.edge_raw(CellEdge::ALL[k]).vector();
        let dot = cv.dot(e);
        if dot > 0.0 {
            continue;
        }
        if dot * dot > sin2 * e.norm2() {
            return false;
        }
        let dir = e.cross(cv);
        if dir.dot(v[k].vector()) < 0.0 && dir.dot(v[(k + 1) & 3].vector()) > 0.0 {
            return true;
        }
    }
    false
}

/// FR-018's shape: each public test computes the vertices itself, as the library will.
fn contains_cell_lib(c: Cap, cell: Cell) -> bool {
    let v = [cell.vertex(0), cell.vertex(1), cell.vertex(2), cell.vertex(3)];
    if !v.iter().all(|&p| c.contains_point(p)) {
        return false;
    }
    let k = if c.is_full() {
        Cap::empty()
    } else if c.is_empty() {
        Cap::full()
    } else {
        Cap::from_center_chord_angle(-c.center(), ChordAngle::from_length2(4.0 - c.chord_radius().length2()))
    };
    !intersects_edges(k, cell, &v)
}

fn may_intersect_lib(c: Cap, cell: Cell) -> bool {
    let v = [cell.vertex(0), cell.vertex(1), cell.vertex(2), cell.vertex(3)];
    if v.iter().any(|&p| c.contains_point(p)) {
        return true;
    }
    intersects_edges(c, cell, &v)
}

fn any_edge_result(_c: Cap, _cell: Cell, _v: &[Point; 4]) -> bool {
    kani::any()
}

fn some_cell() -> Cell {
    Cell::from_cell_id(s2rst::s2::CellId::from_face(0))
}

/// Containment implies intersection: vertices and edges abstracted, the edge test real.
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, memo_vertex)]
#[kani::stub(s2rst::s2::Cell::edge_raw, memo_edge)]
#[kani::stub(s2rst::s2::Cell::contains_point, any_contains_point)]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
fn r0b_region_contains_implies_intersects() {
    let c = any_box_cap();
    let cell = some_cell();
    if contains_cell_lib(c, cell) {
        assert!(may_intersect_lib(c, cell));
    }
}

/// The same, with the edge test abstracted to any result as well.
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, memo_vertex)]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
#[kani::stub(intersects_edges, any_edge_result)]
fn r0b_region_contains_implies_intersects_abstract() {
    let c = any_box_cap();
    let cell = some_cell();
    if contains_cell_lib(c, cell) {
        assert!(may_intersect_lib(c, cell));
    }
}

/// A cell the cap does not intersect has no vertex the cap contains (abstract edge test).
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, memo_vertex)]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
#[kani::stub(intersects_edges, any_edge_result)]
fn r0b_region_no_intersect_no_vertex() {
    let c = any_box_cap();
    let cell = some_cell();
    if !may_intersect_lib(c, cell) {
        for k in 0..4 {
            assert!(!c.contains_point(cell.vertex(k)));
        }
    }
}

/// Crash freedom of the edge test alone, over any box values for the cap and the cell.
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::edge_raw, memo_edge)]
#[kani::stub(s2rst::s2::Cell::contains_point, any_contains_point)]
fn r0b_edge_test_crash() {
    let c = any_box_cap();
    let v = [any_box_point(), any_box_point(), any_box_point(), any_box_point()];
    let _ = intersects_edges(c, some_cell(), &v);
}

/// Can a private function of the library be stubbed? (`union` through
/// `cap::interpolate_at_distance`, replaced by any box point.)
fn any_interp(_ax: s2rst::s1::Angle, _a: Point, _b: Point) -> Point {
    any_box_point()
}

#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
#[kani::stub(f64::atan2, atan2_contract)]
#[kani::stub(s2rst::s2::cap::interpolate_at_distance, any_interp)]
#[kani::stub(s2rst::s2::Point::is_unit, memo_is_unit)]
fn r0b_private_stub_union() {
    let a = any_valid_cap_memo();
    let b = any_valid_cap_memo();
    let _ = a.union(b);
}
