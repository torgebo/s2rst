// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Function contracts and lemma stubs (spec 008 FR-003, FR-003a; data-model §3; research R5,
//! R6). Every `#[kani::stub]` of the suite names a function of this module, and every harness
//! that uses one names it, with what it rests on, in its doc comment.
//!
//! **Function contracts** replace a C library function the checker does not model (`asin`,
//! `atan2`) or models too loosely (`cos`: CBMC let `cos(1.1e-308)` be 0, research R0) by any
//! value in its documented range. A property proven with them holds for every value the
//! contract allows, the real one among them; `tests/libm_contracts.rs` checks that the real
//! functions satisfy the contracts.
//!
//! **Lemma stubs** replace a library function that a statement would otherwise compute twice
//! (the chord distance, `is_unit`, a chord-angle sum): the checker does not see that the two
//! computations are one value, and times out proving two copies of the same products equal
//! (research R0). A lemma stub is a pure function: it returns the same value for the same input,
//! from a table, and for a new input any value its lemmas allow. Each lemma is proven on the
//! library's real code (`cp_lem`).
//!
//! **Soundness.** For each input the real function returns one value its lemmas allow (by the
//! lemma), and the same one every time (it is pure: Rust never contracts floating-point
//! expressions on its own, and `r3::Vector`'s `mul_add` is chosen per build, not per call). So
//! every behaviour of the real code is one of the stubbed code, and a law proven with the stubs
//! holds for the real code. A stub that lacks a fact of the real code only allows more
//! behaviours: it can make a law fail, never pass (research R0b's three failures were of that
//! kind).
//!
//! **Tables** are `static Mutex<Table<K>>` with fixed slots and element-wise key comparisons.
//! A `thread_local!` table crashes Kani 0.68.0's compiler, a growable `Vec` runs the cell laws
//! out of memory, and `==` or `<` on arrays or tuples inside a stub lowers to intrinsics Kani's
//! compiler rejects (research R0b).

#![allow(
    dead_code,
    reason = "stubs are named only in `#[kani::stub]` attributes, which rustc does not count as uses"
)]

use s2rst::r3::Vector;
use s2rst::s1::ChordAngle;
use s2rst::s2::{Cap, Cell, CellEdge, Point};
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Mutex;

// ---- tables -------------------------------------------------------------------------------------

/// Slots per table. A harness's statement touches at most a handful of keys; an eleventh key
/// gets a fresh, unrecorded value (only more behaviours, research R6).
const SLOTS: usize = 10;

/// A pure function's memory: up to [`SLOTS`] keys of `K` 64-bit words and their values.
pub struct Table<const K: usize> {
    keys: [[u64; K]; SLOTS],
    vals: [u64; SLOTS],
    used: usize,
}

impl<const K: usize> Table<K> {
    const fn new() -> Self {
        Table {
            keys: [[0; K]; SLOTS],
            vals: [0; SLOTS],
            used: 0,
        }
    }

    fn clear(&mut self) {
        self.used = 0;
    }

    /// The value stored for `key`, or `fresh()`, stored if a slot is free, and returned.
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
static APPROX: Mutex<Table<7>> = Mutex::new(Table::new());
static FROM_ANGLE: Mutex<Table<1>> = Mutex::new(Table::new());
static NORMALIZE: Mutex<[([u64; 3], [u64; 3]); 2]> = Mutex::new([([0; 3], [0; 3]); 2]);
static NORMALIZE_USED: Mutex<usize> = Mutex::new(0);
static VERTS: Mutex<[[u64; 3]; 4]> = Mutex::new([[0; 3]; 4]);
static EDGES: Mutex<[[u64; 3]; 4]> = Mutex::new([[0; 3]; 4]);

/// Clears every table. Kani runs each harness from a fresh state; a harness that uses a lemma
/// stub calls this first anyway, so the intent is explicit (contracts/harness-conventions.md).
pub fn reset() {
    CHORD.lock().unwrap().clear();
    ADD.lock().unwrap().clear();
    IS_UNIT.lock().unwrap().clear();
    APPROX.lock().unwrap().clear();
    FROM_ANGLE.lock().unwrap().clear();
    *NORMALIZE_USED.lock().unwrap() = 0;
}

fn bits3(p: Point) -> [u64; 3] {
    [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()]
}

fn from3(v: [u64; 3]) -> Point {
    Point(Vector {
        x: f64::from_bits(v[0]),
        y: f64::from_bits(v[1]),
        z: f64::from_bits(v[2]),
    })
}

fn same3(a: [u64; 3], b: [u64; 3]) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2]
}

/// `a` before `b` in the lexicographic order of their bit patterns, element-wise.
fn before3(a: [u64; 3], b: [u64; 3]) -> bool {
    a[0] < b[0] || (a[0] == b[0] && (a[1] < b[1] || (a[1] == b[1] && a[2] < b[2])))
}

fn any_length2() -> u64 {
    let v: f64 = kani::any();
    kani::assume((0.0..=4.0).contains(&v));
    v.to_bits()
}

// ---- function contracts (FR-003) -----------------------------------------------------------------

/// `asin`'s contract: NaN outside [−1, 1] and for NaN; ±0 for ±0; otherwise any value in
/// [−π/2, π/2] with the argument's sign. Checked by `libm_contracts::asin`.
pub fn asin_contract(x: f64) -> f64 {
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

/// `atan2(y, x)`'s contract (C11 F.10.1.4): NaN if either argument is NaN; otherwise any value
/// in [−π, π], non-negative for a `y` with a clear sign bit and non-positive for a set one, and
/// within [−π/2, π/2] when `x` has a clear sign bit. Checked by `libm_contracts::atan2`.
pub fn atan2_contract(y: f64, x: f64) -> f64 {
    if y.is_nan() || x.is_nan() {
        return f64::NAN;
    }
    let r: f64 = kani::any();
    kani::assume(r.abs() <= PI);
    kani::assume(if y.is_sign_positive() {
        r >= 0.0
    } else {
        r <= 0.0
    });
    if x.is_sign_positive() {
        kani::assume(r.abs() <= FRAC_PI_2);
    }
    r
}

/// `cos`'s contract: NaN for a non-finite argument; 1 at ±0; otherwise in [−1, 1], and at least
/// `6e-17` (cos of the `f64` nearest π/2 is 6.12·10⁻¹⁷) wherever |x| ≤ `FRAC_PI_2`. Checked by
/// `libm_contracts::cos`.
pub fn cos_contract(x: f64) -> f64 {
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

// ---- lemma stubs (FR-003a) ---------------------------------------------------------------------

/// `Point::chord_angle`, pure: 0 for a point and itself (L-CP-01), otherwise any length in
/// [0, 4] (L-CP-02), one per ordered pair.
pub fn pure_chord(a: Point, b: Point) -> ChordAngle {
    let (ka, kb) = (bits3(a), bits3(b));
    if same3(ka, kb) {
        return ChordAngle::ZERO;
    }
    let v = CHORD
        .lock()
        .unwrap()
        .get([ka[0], ka[1], ka[2], kb[0], kb[1], kb[2]], any_length2);
    ChordAngle::from_length2(f64::from_bits(v))
}

/// `Point::chord_angle`, pure and symmetric: as [`pure_chord`], one value per unordered pair
/// (L-CP-01 to L-CP-04: the chord's symmetry by `b − a = −(a − b)` and `(−u)² = u²`).
pub fn pure_chord_sym(a: Point, b: Point) -> ChordAngle {
    if before3(bits3(b), bits3(a)) {
        pure_chord(b, a)
    } else {
        pure_chord(a, b)
    }
}

/// `ChordAngle + ChordAngle`, pure: the left operand when the right is zero, the right one when
/// the left is zero, either zero `+0` or −0 (L-CP-08; 007's P-CAARI-01 for `+0` on the right), otherwise any length in [0, 4] at least each operand (L-CP-05), one per ordered pair.
pub fn pure_add(a: ChordAngle, b: ChordAngle) -> ChordAngle {
    let (la, lb) = (a.length2(), b.length2());
    if lb == 0.0 {
        return a;
    }
    if la == 0.0 {
        return b;
    }
    let v = ADD.lock().unwrap().get([la.to_bits(), lb.to_bits()], || {
        let v: f64 = kani::any();
        kani::assume((0.0..=4.0).contains(&v));
        kani::assume(v >= la && v >= lb);
        v.to_bits()
    });
    ChordAngle::from_length2(f64::from_bits(v))
}

/// `ChordAngle + ChordAngle`, pure and symmetric for non-zero operands: the real zero rules
/// first (a zero right operand returns the left one, a zero left operand the right one, either
/// zero `+0` or −0, L-CP-08, so `+0 + −0` is `+0` and `−0 + +0` is `−0`), then as [`pure_add`], one value
/// per unordered pair (L-CP-06, commutativity of non-zero lengths, by composition from
/// L-CP-06a and L-CP-06b).
pub fn pure_add_sym(a: ChordAngle, b: ChordAngle) -> ChordAngle {
    if b.length2() == 0.0 {
        return a;
    }
    if a.length2() == 0.0 {
        return b;
    }
    if b.length2().to_bits() < a.length2().to_bits() {
        pure_add(b, a)
    } else {
        pure_add(a, b)
    }
}

/// `Point::is_unit`, pure: true for (±1, 0, 0), the constants' centre (L-CP-07); otherwise one
/// value per point with each coordinate's sign ignored (L-CP-04: negating a coordinate keeps its
/// square, so the squared length, bit for bit).
pub fn pure_is_unit(p: Point) -> bool {
    let k = [
        p.0.x.abs().to_bits(),
        p.0.y.abs().to_bits(),
        p.0.z.abs().to_bits(),
    ];
    if k[0] == 1.0f64.to_bits() && k[1] == 0 && k[2] == 0 {
        return true;
    }
    IS_UNIT
        .lock()
        .unwrap()
        .get(k, || u64::from(kani::any::<bool>()))
        == 1
}

/// `ChordAngle::from_angle`, pure: `NEGATIVE` for a negative angle, `INFINITY` for +∞,
/// `STRAIGHT` for NaN, otherwise any length in [0, 4], one per angle (spec 007's P-CAVAL-04: its
/// range on the real code). The real function calls `sin`, which the checker over-approximates
/// per call, so two calls on one angle are one value only through this stub.
pub fn pure_from_angle(a: s2rst::s1::Angle) -> ChordAngle {
    let r = a.radians();
    if r < 0.0 {
        return ChordAngle::NEGATIVE;
    }
    if r == f64::INFINITY {
        return ChordAngle::INFINITY;
    }
    if r.is_nan() {
        return ChordAngle::STRAIGHT;
    }
    let v = FROM_ANGLE.lock().unwrap().get([r.to_bits()], any_length2);
    ChordAngle::from_length2(f64::from_bits(v))
}

/// `r3::Vector::normalize`, pure: any finite vector, one per input (two slots). The real
/// normalisation of a cell's centre (coordinates in [−1, 1], squared length at least 1) is
/// finite; P-CCAP-01's statement holds for any centre, so it needs no more than purity and
/// finiteness.
pub fn pure_normalize(v: Vector) -> Vector {
    let k = [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()];
    let mut used = NORMALIZE_USED.lock().unwrap();
    let mut table = NORMALIZE.lock().unwrap();
    for i in 0..2 {
        if i < *used && same3(table[i].0, k) {
            let r = table[i].1;
            return Vector {
                x: f64::from_bits(r[0]),
                y: f64::from_bits(r[1]),
                z: f64::from_bits(r[2]),
            };
        }
    }
    let (x, y, z): (f64, f64, f64) = (kani::any(), kani::any(), kani::any());
    kani::assume(x.is_finite() && y.is_finite() && z.is_finite());
    if *used < 2 {
        table[*used] = (k, [x.to_bits(), y.to_bits(), z.to_bits()]);
        *used += 1;
    }
    Vector { x, y, z }
}

/// `Point::approx_eq_angle`, pure: one boolean per pair of points and tolerance (no lemma: only
/// purity is used).
pub fn pure_approx_eq_angle(a: Point, b: Point, eps: s2rst::s1::Angle) -> bool {
    let (ka, kb) = (bits3(a), bits3(b));
    let key = [
        ka[0],
        ka[1],
        ka[2],
        kb[0],
        kb[1],
        kb[2],
        eps.radians().to_bits(),
    ];
    APPROX
        .lock()
        .unwrap()
        .get(key, || u64::from(kani::any::<bool>()))
        == 1
}

// ---- the abstract cell (research R7) -------------------------------------------------------------

fn any_box_point() -> Point {
    let (x, y, z): (f64, f64, f64) = (kani::any(), kani::any(), kani::any());
    kani::assume(x.abs() <= 1.0 && y.abs() <= 1.0 && z.abs() <= 1.0);
    Point(Vector { x, y, z })
}

/// Draws the abstract cell: four box points as its vertices and four box vectors as its edge
/// normals. A harness calls it first; `fixed_vertex` and `fixed_edge` then read them.
pub fn draw_abstract_cell() {
    let mut v = VERTS.lock().unwrap();
    let mut e = EDGES.lock().unwrap();
    for k in 0..4 {
        v[k] = bits3(any_box_point());
        e[k] = bits3(any_box_point());
    }
}

/// `Cell::vertex`, from the abstract cell: the cell's own geometry is abstracted away, so a law
/// proven over it holds for every real cell (research R7).
pub fn fixed_vertex(_c: Cell, k: usize) -> Point {
    from3(VERTS.lock().unwrap()[k & 3])
}

/// `Cell::edge_raw`, from the abstract cell.
pub fn fixed_edge(_c: Cell, e: CellEdge) -> Point {
    from3(EDGES.lock().unwrap()[(e as usize) & 3])
}

/// `Cell::contains_point`, abstracted: any answer.
pub fn any_contains_point(_c: Cell, _p: Point) -> bool {
    kani::any()
}

/// `Cap::intersects_cell_edges` (FR-018, upstream's `Intersects(cell, vertices)`), abstracted:
/// any answer. The cell laws that use it depend only on the order of the vertex tests.
pub fn any_edge_result(_c: Cap, _cell: &Cell, _v: &[Point; 4]) -> bool {
    kani::any()
}
