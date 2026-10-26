// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 008 planning probe, third round (research R0c). R0b's pure-function stubs used growable
//! tables (`Vec` in a static), which ran the cell laws out of memory, and they lacked three
//! facts of the real code (`chord(p, p) = 0`, adding zero returns the other operand, the
//! constants' centre is unit), which made three laws fail spuriously. This round uses fixed-size
//! tables and states those facts, each a lemma proven here on the real code.

use s2rst::r3::Vector;
use s2rst::s1::ChordAngle;
use s2rst::s2::{Cap, Cell, CellEdge, Point};
use std::sync::Mutex;

// ---- fixed-size pure-function tables ----------------------------------------------------------

const SLOTS: usize = 10;

struct Table<const K: usize> {
    keys: [[u64; K]; SLOTS],
    vals: [u64; SLOTS],
    used: usize,
}

impl<const K: usize> Table<K> {
    const fn new() -> Self {
        Table { keys: [[0; K]; SLOTS], vals: [0; SLOTS], used: 0 }
    }

    /// The value stored for `key`, or `fresh()` stored and returned. Once full, a new key gets
    /// an unrecorded fresh value (fewer constraints: still sound).
    fn get(&mut self, key: [u64; K], fresh: impl FnOnce() -> u64) -> u64 {
        for i in 0..SLOTS {
            if i < self.used {
                let mut eq = true;
                for j in 0..K {
                    eq &= self.keys[i][j] == key[j];
                }
                if eq {
                    return self.vals[i];
                }
            }
        }
        let v = fresh();
        if self.used < SLOTS {
            self.keys[self.used] = key;
            self.vals[self.used] = v;
            self.used += 1;
        }
        v
    }
}

static CHORD: Mutex<Table<6>> = Mutex::new(Table::new());
static ADD: Mutex<Table<2>> = Mutex::new(Table::new());
static IS_UNIT: Mutex<Table<3>> = Mutex::new(Table::new());

fn bits3(p: Point) -> [u64; 3] {
    [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()]
}

/// `Point::chord_angle`, pure: 0 for a point and itself (`lemma_chord_self`), otherwise any
/// length in [0, 4] (`r0b::lemma_chord_range`).
fn memo_chord(a: Point, b: Point) -> ChordAngle {
    let (ka, kb) = (bits3(a), bits3(b));
    if ka[0] == kb[0] && ka[1] == kb[1] && ka[2] == kb[2] {
        return ChordAngle::ZERO;
    }
    let v = CHORD.lock().unwrap().get([ka[0], ka[1], ka[2], kb[0], kb[1], kb[2]], || {
        let v: f64 = kani::any();
        kani::assume((0.0..=4.0).contains(&v));
        v.to_bits()
    });
    ChordAngle::from_length2(f64::from_bits(v))
}

/// `ChordAngle + ChordAngle`, pure: the other operand when one is zero (007's P-CAARI-01),
/// otherwise any length in [0, 4] at least each operand (`r0b::lemma_add_bounds`).
fn memo_add(a: ChordAngle, b: ChordAngle) -> ChordAngle {
    if b.length2() == 0.0 {
        return a;
    }
    if a.length2() == 0.0 {
        return b;
    }
    let v = ADD.lock().unwrap().get([a.length2().to_bits(), b.length2().to_bits()], || {
        let v: f64 = kani::any();
        kani::assume((0.0..=4.0).contains(&v));
        kani::assume(v >= a.length2() && v >= b.length2());
        v.to_bits()
    });
    ChordAngle::from_length2(f64::from_bits(v))
}

/// `Point::is_unit`, pure, ignoring coordinate signs (`r0b::lemma_sq_neg`), and true for the
/// constants' centre (1, 0, 0) (`lemma_axis_unit`).
fn memo_is_unit(p: Point) -> bool {
    let k = [p.0.x.abs().to_bits(), p.0.y.abs().to_bits(), p.0.z.abs().to_bits()];
    if k[0] == 1.0f64.to_bits() && k[1] == 0 && k[2] == 0 {
        return true;
    }
    IS_UNIT.lock().unwrap().get(k, || u64::from(kani::any::<bool>())) == 1
}

// ---- fixed vertices and edges ------------------------------------------------------------------

static VERTS: Mutex<[[u64; 3]; 4]> = Mutex::new([[0; 3]; 4]);
static EDGES: Mutex<[[u64; 3]; 4]> = Mutex::new([[0; 3]; 4]);

fn any_box_point() -> Point {
    let (x, y, z): (f64, f64, f64) = (kani::any(), kani::any(), kani::any());
    kani::assume(x.abs() <= 1.0 && y.abs() <= 1.0 && z.abs() <= 1.0);
    Point(Vector { x, y, z })
}

fn from3(v: [u64; 3]) -> Point {
    Point(Vector { x: f64::from_bits(v[0]), y: f64::from_bits(v[1]), z: f64::from_bits(v[2]) })
}

/// Draws the abstract cell: four box points as vertices, four box vectors as edges.
fn draw_abstract_cell() {
    let mut v = VERTS.lock().unwrap();
    let mut e = EDGES.lock().unwrap();
    for k in 0..4 {
        v[k] = bits3(any_box_point());
        e[k] = bits3(any_box_point());
    }
}

fn fixed_vertex(_c: Cell, k: usize) -> Point {
    from3(VERTS.lock().unwrap()[k & 3])
}

fn fixed_edge(_c: Cell, e: CellEdge) -> Point {
    from3(EDGES.lock().unwrap()[(e as usize) & 3])
}

fn any_contains_point(_c: Cell, _p: Point) -> bool {
    kani::any()
}

// ---- domains ----------------------------------------------------------------------------------

fn any_radius() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume(x == -1.0 || (0.0..=4.0).contains(&x));
    ChordAngle::from_length2(x)
}

fn any_box_cap() -> Cap {
    Cap::from_center_chord_angle(any_box_point(), any_radius())
}

// ---- lemmas -------------------------------------------------------------------------------------

/// `lemma_chord_self`: the chord from a box point to itself is 0.
#[kani::proof]
fn lemma_chord_self() {
    let p = any_box_point();
    assert!(p.chord_angle(p).length2() == 0.0);
}

/// `lemma_axis_unit`: the constants' centre is unit, and both constants are valid.
#[kani::proof]
fn lemma_axis_unit() {
    assert!(Cap::empty().is_valid() && Cap::full().is_valid());
}

// ---- the laws ----------------------------------------------------------------------------------

#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
fn r0c_add_point_contains() {
    let c = any_box_cap();
    let p = any_box_point();
    assert!(c.add_point(p).contains_point(p));
}

#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, memo_add)]
fn r0c_add_cap_contains() {
    let a = any_box_cap();
    let b = any_box_cap();
    assert!(a.add_cap(b).contains(b));
}

#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, memo_is_unit)]
fn r0c_complement_draft_valid() {
    let c = any_box_cap();
    kani::assume(c.is_valid());
    let k = if c.is_full() {
        Cap::empty()
    } else if c.is_empty() {
        Cap::full()
    } else {
        Cap::from_center_chord_angle(-c.center(), ChordAngle::from_length2(4.0 - c.chord_radius().length2()))
    };
    assert!(k.is_valid());
}

/// `intersects` is symmetric, given the chord's symmetry (`r0b::lemma_neg_sub`,
/// `r0b::lemma_sq_neg`: the stub keys an unordered pair) and the sum's.
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord_sym)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, memo_add_sym)]
fn r0c_intersects_symmetric() {
    let (a, b) = (any_box_cap(), any_box_cap());
    assert!(a.intersects(b) == b.intersects(a));
}

fn memo_chord_sym(a: Point, b: Point) -> ChordAngle {
    let (ka, kb) = (bits3(a), bits3(b));
    // Element-wise: `==` and `<` on arrays and tuples lower to intrinsics that crash Kani
    // 0.68.0's compiler inside a stub (R0b).
    let swap = ka[0] > kb[0]
        || (ka[0] == kb[0] && (ka[1] > kb[1] || (ka[1] == kb[1] && ka[2] > kb[2])));
    if swap { memo_chord(b, a) } else { memo_chord(a, b) }
}

fn memo_add_sym(a: ChordAngle, b: ChordAngle) -> ChordAngle {
    if a.length2().to_bits() > b.length2().to_bits() { memo_add(b, a) } else { memo_add(a, b) }
}

// ---- the cell laws over an abstract cell ---------------------------------------------------------

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

fn face0() -> Cell {
    Cell::from_cell_id(s2rst::s2::CellId::from_face(0))
}

/// Containment implies intersection, for any four vertices, the edge test abstracted.
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, fixed_vertex)]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
#[kani::stub(intersects_edges, any_edge_result)]
fn r0c_region_contains_implies_intersects_abstract() {
    draw_abstract_cell();
    let c = any_box_cap();
    let cell = face0();
    if contains_cell_lib(c, cell) {
        assert!(may_intersect_lib(c, cell));
    }
}

/// No intersection implies no contained vertex, for any four vertices, the edge test abstracted.
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, fixed_vertex)]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
#[kani::stub(intersects_edges, any_edge_result)]
fn r0c_region_no_intersect_no_vertex() {
    draw_abstract_cell();
    let c = any_box_cap();
    let cell = face0();
    if !may_intersect_lib(c, cell) {
        for k in 0..4 {
            assert!(!c.contains_point(cell.vertex(k)));
        }
    }
}

/// Containment implies intersection with the real edge test, over the abstract cell.
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, fixed_vertex)]
#[kani::stub(s2rst::s2::Cell::edge_raw, fixed_edge)]
#[kani::stub(s2rst::s2::Cell::contains_point, any_contains_point)]
#[kani::stub(s2rst::s2::Point::chord_angle, memo_chord)]
fn r0c_region_contains_implies_intersects_real_edges() {
    draw_abstract_cell();
    let c = any_box_cap();
    let cell = face0();
    if contains_cell_lib(c, cell) {
        assert!(may_intersect_lib(c, cell));
    }
}
