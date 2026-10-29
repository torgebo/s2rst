// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Function contracts and lemma stubs (spec 008 FR-003, FR-003a; data-model §3; research R5,
//! R6). Every `#[kani::stub]` of the suite names a function of this module, and every harness
//! that uses one names it, with what it rests on, in its doc comment.
//!
//! **Function contracts** replace a C library function the checker does not model (`asin`,
//! `atan2`; since gap 2 of the 2026-10-26 coverage review also `tan` and `atan`) or models too
//! loosely (`cos`: CBMC let `cos(1.1e-308)` be 0, research R0; since gap 2 also `sin` and
//! `sin_cos`, whose models lose the sign and `sin² + cos² = 1`) by any value in its documented
//! range. A property proven with them holds for every value the
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
use s2rst::s2::{Cap, Cell, CellEdge, CellVertex, Point};
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
static PAIR_VERTS: Mutex<[[[u64; 3]; 4]; 2]> = Mutex::new([[[0; 3]; 4]; 2]);
static PAIR_IDS: Mutex<([u64; 2], usize)> = Mutex::new(([0; 2], 0));
static EDGES: Mutex<[[u64; 3]; 4]> = Mutex::new([[0; 3]; 4]);
// Spec 010: the pure contracts and the stubs of `LatLng::from_point` and `Cell::rect_bound`.
static SIN: Mutex<Table<1>> = Mutex::new(Table::new());
static COS: Mutex<Table<1>> = Mutex::new(Table::new());
static ASIN: Mutex<Table<1>> = Mutex::new(Table::new());
static SQRT: Mutex<Table<1>> = Mutex::new(Table::new());
static ATAN2: Mutex<Table<2>> = Mutex::new(Table::new());
static SIN_COS_S: Mutex<Table<1>> = Mutex::new(Table::new());
static SIN_COS_C: Mutex<Table<1>> = Mutex::new(Table::new());
static FP_LAT: Mutex<Table<3>> = Mutex::new(Table::new());
static FP_LNG: Mutex<Table<3>> = Mutex::new(Table::new());
static CELL_BOUND: Mutex<Table<1>> = Mutex::new(Table::new());
static CELL_BOUNDS: Mutex<[[u64; 4]; SLOTS]> = Mutex::new([[0; 4]; SLOTS]);
static ANGLE_DEG: Mutex<Table<1>> = Mutex::new(Table::new());
static ANGLE_E5: Mutex<Table<1>> = Mutex::new(Table::new());
static ANGLE_E6: Mutex<Table<1>> = Mutex::new(Table::new());
static ANGLE_E7: Mutex<Table<1>> = Mutex::new(Table::new());
static ANGLE_UE6: Mutex<Table<1>> = Mutex::new(Table::new());
static ANGLE_UE7: Mutex<Table<1>> = Mutex::new(Table::new());
// Spec 011: the pure `CellId::from_point`, the cells' cap bounds, areas and average areas.
static LEAF: Mutex<Table<3>> = Mutex::new(Table::new());
static CELL_CAP: Mutex<Table<1>> = Mutex::new(Table::new());
static CELL_CAPS: Mutex<[[u64; 4]; SLOTS]> = Mutex::new([[0; 4]; SLOTS]);
static CELL_AREA: Mutex<Table<1>> = Mutex::new(Table::new());
static AVG_AREA: Mutex<Table<1>> = Mutex::new(Table::new());
static CELL_CONTAINS: Mutex<Table<4>> = Mutex::new(Table::new());

/// Clears every table. Kani runs each harness from a fresh state; a harness that uses a lemma
/// stub calls this first anyway, so the intent is explicit (contracts/harness-conventions.md).
pub fn reset() {
    CHORD.lock().unwrap().clear();
    ADD.lock().unwrap().clear();
    IS_UNIT.lock().unwrap().clear();
    APPROX.lock().unwrap().clear();
    FROM_ANGLE.lock().unwrap().clear();
    *NORMALIZE_USED.lock().unwrap() = 0;
    SIN.lock().unwrap().clear();
    COS.lock().unwrap().clear();
    ASIN.lock().unwrap().clear();
    SQRT.lock().unwrap().clear();
    ATAN2.lock().unwrap().clear();
    SIN_COS_S.lock().unwrap().clear();
    SIN_COS_C.lock().unwrap().clear();
    FP_LAT.lock().unwrap().clear();
    FP_LNG.lock().unwrap().clear();
    CELL_BOUND.lock().unwrap().clear();
    LEAF.lock().unwrap().clear();
    CELL_CAP.lock().unwrap().clear();
    CELL_AREA.lock().unwrap().clear();
    AVG_AREA.lock().unwrap().clear();
    CELL_CONTAINS.lock().unwrap().clear();
    for t in [
        &ANGLE_DEG, &ANGLE_E5, &ANGLE_E6, &ANGLE_E7, &ANGLE_UE6, &ANGLE_UE7,
    ] {
        t.lock().unwrap().clear();
    }
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

// ---- function contracts (FR-003)
// -----------------------------------------------------------------

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

/// `tan`'s contract: NaN for a non-finite argument; ±0 for ±0; otherwise any finite value, at
/// least 0 on (0, `FRAC_PI_2`] and at most 0 on [−`FRAC_PI_2`, 0), and at most 1.7·10¹⁶ in
/// magnitude wherever |x| ≤ 2 (tan of the `f64` nearest π/2 is 1.63·10¹⁶). Checked by
/// `libm_contracts::tan` (gap 2, 2026-10-26).
pub fn tan_contract(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return x;
    }
    let r: f64 = kani::any();
    kani::assume(r.is_finite());
    if x.abs() <= 2.0 {
        kani::assume(r.abs() <= 1.7e16);
    }
    if x > 0.0 && x <= FRAC_PI_2 {
        kani::assume(r >= 0.0);
    }
    if x < 0.0 && x >= -FRAC_PI_2 {
        kani::assume(r <= 0.0);
    }
    r
}

/// `atan`'s contract: NaN for NaN; ±0 for ±0; otherwise any value in [−π/2, π/2] with the
/// argument's sign (the infinities included). Checked by `libm_contracts::atan`.
pub fn atan_contract(x: f64) -> f64 {
    if x.is_nan() {
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

/// `sin`'s contract: NaN for a non-finite argument; ±0 for ±0; otherwise in [−1, 1], at least 0
/// on (0, π] and at most 0 on [−π, 0) (sin of `PI`, the `f64` below π, is 1.2·10⁻¹⁶). The
/// checker's own `sin` loses the sign. Checked by `libm_contracts::sin`.
pub fn sin_contract(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return x;
    }
    let r: f64 = kani::any();
    kani::assume(r.abs() <= 1.0);
    if x > 0.0 && x <= PI {
        kani::assume(r >= 0.0);
    }
    if x < 0.0 && x >= -PI {
        kani::assume(r <= 0.0);
    }
    r
}

/// `sin_cos`'s contract: NaN twice for a non-finite argument; (±0, 1) for ±0; otherwise both in
/// [−1, 1], and the larger in magnitude at least 0.7 (sin² + cos² = 1 puts it at 1/√2 or more).
/// The checker's own `sin` and `cos` lose that last fact, which keeps `LatLng::to_point` off the
/// zero vector. Checked by `libm_contracts::sin_cos`.
pub fn sin_cos_contract(x: f64) -> (f64, f64) {
    if !x.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    if x == 0.0 {
        return (x, 1.0);
    }
    let (s, c): (f64, f64) = (kani::any(), kani::any());
    kani::assume(s.abs() <= 1.0 && c.abs() <= 1.0);
    kani::assume(s.abs() >= 0.7 || c.abs() >= 0.7);
    (s, c)
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

/// `ChordAngle + ChordAngle`, pure: the left operand when the right is zero, the right one when the
/// left is zero, either zero `+0` or −0 (L-CP-08; 007's P-CAARI-01 for `+0` on the right),
/// otherwise any length in [0, 4] at least each operand (L-CP-05), one per ordered pair.
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

/// `ChordAngle + ChordAngle`, pure and symmetric for non-zero operands: the real zero rules first
/// (a zero right operand returns the left one, a zero left operand the right one, either zero `+0`
/// or −0, L-CP-08, so `+0 + −0` is `+0` and `−0 + +0` is `−0`), then as [`pure_add`], one value per
/// unordered pair (L-CP-06, commutativity of non-zero lengths, by composition from L-CP-06a and
/// L-CP-06b).
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

// ---- the abstract cell (research R7)
// -------------------------------------------------------------

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
pub fn fixed_vertex(_c: Cell, k: CellVertex) -> Point {
    from3(VERTS.lock().unwrap()[k.index()])
}

/// `Cell::edge_raw`, from the abstract cell.
pub fn fixed_edge(_c: Cell, e: CellEdge) -> Point {
    from3(EDGES.lock().unwrap()[(e as usize) & 3])
}

/// `edge_distances::distance_from_segment`, abstracted: any angle. For the `n_*` harnesses whose
/// panic comes first: the code after it runs on no input, and an any-value stub cannot hide an
/// input that returns.
pub fn any_segment_distance(_x: Point, _a: Point, _b: Point) -> s2rst::s1::Angle {
    s2rst::s1::Angle::from_radians(kani::any())
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

// ---- the abstract pair of cells (gap 2)
// ----------------------------------------------------------

/// Draws an abstract pair of cells: four box points as the vertices of each of two cells, given
/// to the first two cell ids `pair_vertex` meets. A harness over two cells calls it first.
pub fn draw_abstract_pair() {
    let mut v = PAIR_VERTS.lock().unwrap();
    for c in 0..2 {
        for k in 0..4 {
            v[c][k] = bits3(any_box_point());
        }
    }
    *PAIR_IDS.lock().unwrap() = ([0; 2], 0);
}

/// `Cell::vertex`, from the abstract pair: one cell's vertices for each of the first two ids, the
/// same for one id, so equal cells keep equal vertices (`fixed_vertex` gives every cell the same
/// four, which would tie two cells together). A third id shares the second's vertices; no
/// harness has one.
pub fn pair_vertex(c: Cell, k: CellVertex) -> Point {
    let id = c.id().id();
    let mut ids = PAIR_IDS.lock().unwrap();
    let slot = if ids.1 > 0 && ids.0[0] == id {
        0
    } else if ids.1 > 1 && ids.0[1] == id {
        1
    } else if ids.1 < 2 {
        let n = ids.1;
        ids.0[n] = id;
        ids.1 += 1;
        n
    } else {
        1
    };
    from3(PAIR_VERTS.lock().unwrap()[slot][k.index()])
}

// ---- the abstract u-v bound (gap 2) -----------------------------------------------------------

/// `s2::ij_level_to_bound_uv`, abstracted: any rectangle inside [−1, 1]² with ordered bounds.
/// `Cell::from_cell_id` stores its result as the cell's u-v bound; every real cell's bound is such
/// a rectangle (the quadratic projection maps [0, 1] onto [−1, 1], increasing), so a crash proof
/// over this abstraction covers every cell, and spares the checker the projection of a symbolic
/// (i, j). The cell's face, level, orientation and id stay its own.
pub fn any_uv_bound(_i: i32, _j: i32, _level: s2rst::s2::Level) -> s2rst::r2::Rect {
    let b: [f64; 4] = kani::any();
    kani::assume(b[0] >= -1.0 && b[0] <= b[1] && b[1] <= 1.0);
    kani::assume(b[2] >= -1.0 && b[2] <= b[3] && b[3] <= 1.0);
    s2rst::r2::Rect {
        x: s2rst::r1::Interval { lo: b[0], hi: b[1] },
        y: s2rst::r1::Interval { lo: b[2], hi: b[3] },
    }
}

// ---- spec 010: sqrt's contract and the pure contracts (research R5) ----------------------------

/// `sqrt`'s contract: NaN for NaN and for `x < 0`; `x` itself for ±0 and +∞; otherwise a positive
/// finite value, at most 1 when `x ≤ 1` and at least 1 when `x ≥ 1`. IEEE-754's `sqrt` is
/// correctly rounded, so the real one satisfies it (checked by `libm_contracts::sqrt`). The
/// checker models `sqrt` itself, but not as one value per input (memory "CBMC sqrt is
/// nondeterministic"), so a restatement that takes a root again needs [`pure_sqrt`].
pub fn sqrt_contract(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 || x == f64::INFINITY {
        return x;
    }
    let r: f64 = kani::any();
    kani::assume(r > 0.0 && r.is_finite());
    if x <= 1.0 {
        kani::assume(r <= 1.0);
    }
    if x >= 1.0 {
        kani::assume(r >= 1.0);
    }
    r
}

/// One value per input: the stored one, or `fresh()`'s, stored.
fn pure1(t: &Mutex<Table<1>>, x: f64, fresh: impl FnOnce() -> f64) -> f64 {
    f64::from_bits(t.lock().unwrap().get([x.to_bits()], || fresh().to_bits()))
}

/// `sin`, pure: [`sin_contract`]'s values, one per input (spec 010 research R5: a restatement
/// that calls `sin` again gets the library's value). The C library's functions are
/// deterministic (`TRUSTED_BASE.md`), so the real `sin` is one such function.
pub fn pure_sin(x: f64) -> f64 {
    pure1(&SIN, x, || sin_contract(x))
}

/// `cos`, pure: [`cos_contract`]'s values, one per input.
pub fn pure_cos(x: f64) -> f64 {
    pure1(&COS, x, || cos_contract(x))
}

/// `asin`, pure: [`asin_contract`]'s values, one per input.
pub fn pure_asin(x: f64) -> f64 {
    pure1(&ASIN, x, || asin_contract(x))
}

/// `sqrt`, pure: [`sqrt_contract`]'s values, one per input.
pub fn pure_sqrt(x: f64) -> f64 {
    pure1(&SQRT, x, || sqrt_contract(x))
}

/// `atan2`, pure: [`atan2_contract`]'s values, one per pair.
pub fn pure_atan2(y: f64, x: f64) -> f64 {
    let v = ATAN2.lock().unwrap().get([y.to_bits(), x.to_bits()], || {
        atan2_contract(y, x).to_bits()
    });
    f64::from_bits(v)
}

/// `sin_cos`, pure: [`sin_cos_contract`]'s pairs, one per input.
pub fn pure_sin_cos(x: f64) -> (f64, f64) {
    let (s, c) = sin_cos_contract(x);
    let s = pure1(&SIN_COS_S, x, || s);
    let c = pure1(&SIN_COS_C, x, || c);
    (s, c)
}

// ---- spec 010: `LatLng::from_point` and `Cell::rect_bound` (research R6) -----------------------

/// `LatLng::from_point`, pure: for a point without a NaN coordinate any valid `LatLng`
/// (P-LLFP-02), for one with a NaN coordinate any pair with a NaN coordinate (the real function's
/// latitude or longitude is NaN there); one value per point.
pub fn pure_from_point(p: Point) -> s2rst::s2::LatLng {
    let k = bits3(p);
    let nan = p.0.x.is_nan() || p.0.y.is_nan() || p.0.z.is_nan();
    let lat = f64::from_bits(FP_LAT.lock().unwrap().get(k, || {
        let v: f64 = kani::any();
        kani::assume(nan || v.abs() <= FRAC_PI_2);
        v.to_bits()
    }));
    let lng = f64::from_bits(FP_LNG.lock().unwrap().get(k, || {
        let v: f64 = kani::any();
        kani::assume(if nan {
            lat.is_nan() || v.is_nan()
        } else {
            v.abs() <= PI
        });
        v.to_bits()
    }));
    s2rst::s2::LatLng::from_radians(lat, lng)
}

/// The stored bound of cell `c`, or `fresh()`'s, stored.
fn cell_bound(c: Cell, fresh: impl FnOnce() -> s2rst::s2::Rect) -> s2rst::s2::Rect {
    let mut t = CELL_BOUND.lock().unwrap();
    let used = t.used;
    let slot = t.get([c.id().id()], || used as u64) as usize;
    let mut b = CELL_BOUNDS.lock().unwrap();
    if slot == used && used < SLOTS {
        let r = fresh();
        b[slot] = [
            r.lat.lo.to_bits(),
            r.lat.hi.to_bits(),
            r.lng.lo.to_bits(),
            r.lng.hi.to_bits(),
        ];
        return r;
    }
    if slot >= SLOTS {
        return fresh();
    }
    let v = b[slot];
    s2rst::s2::Rect {
        lat: s2rst::r1::Interval {
            lo: f64::from_bits(v[0]),
            hi: f64::from_bits(v[1]),
        },
        lng: s2rst::s1::Interval {
            lo: f64::from_bits(v[2]),
            hi: f64::from_bits(v[3]),
        },
    }
}

/// Any rectangle `crate::rect_spec` accepts.
fn any_valid_rect() -> s2rst::s2::Rect {
    let r = s2rst::s2::Rect {
        lat: s2rst::r1::Interval {
            lo: kani::any(),
            hi: kani::any(),
        },
        lng: s2rst::s1::Interval {
            lo: kani::any(),
            hi: kani::any(),
        },
    };
    kani::assume(crate::rect_spec::valid(r));
    r
}

/// `Cell::rect_bound`, pure: one valid, non-empty rectangle per cell (L-LL-01: the real bound of
/// every valid cell is one).
pub fn any_cell_bound(c: Cell) -> s2rst::s2::Rect {
    cell_bound(c, || {
        let r = any_valid_rect();
        kani::assume(!crate::rect_spec::empty(r));
        r
    })
}

/// `Cell::rect_bound`'s contract (spec 010 research R6): one valid rectangle per cell that
/// contains the `LatLng` of each of the cell's four vertices, as [`fixed_vertex`] and
/// [`pure_from_point`] give them. The real bound contains its cell's vertices (upstream's
/// conservative bound); `libm_contracts::cell_rect_bound` checks it on every cell to level 5
/// and 10⁶ random cells, with the real `from_point` of the real vertices. A harness using it
/// draws the abstract cell first.
pub fn cell_rect_bound_contract(c: Cell) -> s2rst::s2::Rect {
    cell_bound(c, || {
        let r = any_valid_rect();
        for k in CellVertex::ALL {
            let ll = pure_from_point(fixed_vertex(c, k));
            kani::assume(crate::rect_spec::member(
                r,
                ll.lat.radians(),
                ll.lng.radians(),
            ));
        }
        r
    })
}

// ---- spec 010: `Angle`'s constructors, uninterpreted --------------------------------------------

/// One angle per key from `t`: any `f64`, the same for the same key. An uninterpreted function:
/// it allows every behaviour of the real constructor (whose meaning spec 007 proves, P-AGCNV-01,
/// P-AGEF-01), so a statement proven with it holds for the real one, and it spares the checker
/// two copies of the constructor's arithmetic when a statement compares `LatLng`'s constructors
/// with `Angle`'s (P-LLCON-01).
fn pure_angle(t: &Mutex<Table<1>>, key: u64) -> s2rst::s1::Angle {
    s2rst::s1::Angle::from_radians(f64::from_bits(
        t.lock()
            .unwrap()
            .get([key], || kani::any::<f64>().to_bits()),
    ))
}

/// `Angle::from_degrees`, uninterpreted.
pub fn pure_angle_from_degrees(x: f64) -> s2rst::s1::Angle {
    pure_angle(&ANGLE_DEG, x.to_bits())
}

/// `Angle::from_e5`, uninterpreted.
pub fn pure_angle_from_e5(e: i32) -> s2rst::s1::Angle {
    pure_angle(&ANGLE_E5, u64::from(e.cast_unsigned()))
}

/// `Angle::from_e6`, uninterpreted.
pub fn pure_angle_from_e6(e: i32) -> s2rst::s1::Angle {
    pure_angle(&ANGLE_E6, u64::from(e.cast_unsigned()))
}

/// `Angle::from_e7`, uninterpreted.
pub fn pure_angle_from_e7(e: i32) -> s2rst::s1::Angle {
    pure_angle(&ANGLE_E7, u64::from(e.cast_unsigned()))
}

/// `Angle::from_unsigned_e6`, uninterpreted.
pub fn pure_angle_from_unsigned_e6(e: u32) -> s2rst::s1::Angle {
    pure_angle(&ANGLE_UE6, u64::from(e))
}

/// `Angle::from_unsigned_e7`, uninterpreted.
pub fn pure_angle_from_unsigned_e7(e: u32) -> s2rst::s1::Angle {
    pure_angle(&ANGLE_UE7, u64::from(e))
}

// ---- spec 011: `PointRegion` and `CellUnion`'s floating-point methods (research R5) -----------

/// `sqrt_contract`, and the root lies between `x` and 1: true of the correctly rounded root, since
/// `x` and 1 are representable and the exact root lies between them (checked by
/// `libm_contracts::sqrt`). Spec 011 R0: `sqrt_contract` alone lets the root of `1 + u² + v²` be
/// 10³⁰⁰, so the normalised cell vertex underflows and leaves its cell, which the real one never
/// does.
pub fn sqrt_between(x: f64) -> f64 {
    let r = sqrt_contract(x);
    if x.is_finite() && x > 0.0 {
        kani::assume(if x >= 1.0 { r <= x } else { r >= x });
    }
    r
}

/// `CellId::from_point`, pure: one valid leaf cell per point (L-PR-01: the real function returns a
/// valid leaf wherever it returns; its debug-build panics, spec 011 K4, are its own crash proof's).
/// The covering law and the union's point test call it twice on one point, and its projection
/// takes a square root, which is not one value per input to the checker (KI-007).
pub fn pure_leaf(p: &Point) -> s2rst::s2::CellId {
    s2rst::s2::CellId(
        LEAF.lock()
            .unwrap()
            .get(bits3(*p), || crate::domain::any_valid_at(30).0.id()),
    )
}

/// `Cell::cap_bound`, pure: one non-empty cap per cell, its centre in the unit box and its
/// squared radius in [0, 4]. The real bound's centre is a normalised point and its radius a chord
/// length (`libm_contracts::cell_cap_bound` checks both on every cell to level 5 and 10⁶ random
/// cells).
pub fn pure_cell_cap(c: Cell) -> Cap {
    let mut t = CELL_CAP.lock().unwrap();
    let used = t.used;
    let slot = t.get([c.id().id()], || used as u64) as usize;
    let mut caps = CELL_CAPS.lock().unwrap();
    if slot == used && used < SLOTS {
        let cap = Cap::from_center_chord_angle(any_box_point(), any_chord_len());
        let p = cap.center();
        caps[slot] = [
            p.0.x.to_bits(),
            p.0.y.to_bits(),
            p.0.z.to_bits(),
            cap.chord_radius().length2().to_bits(),
        ];
        return cap;
    }
    if slot >= SLOTS {
        return Cap::from_center_chord_angle(any_box_point(), any_chord_len());
    }
    let v = caps[slot];
    Cap::from_center_chord_angle(
        from3([v[0], v[1], v[2]]),
        ChordAngle::from_length2(f64::from_bits(v[3])),
    )
}

/// A chord length in [0, 4].
fn any_chord_len() -> ChordAngle {
    let l: f64 = kani::any();
    kani::assume((0.0..=4.0).contains(&l));
    ChordAngle::from_length2(l)
}

/// `CellId::to_point`, abstracted: any point in the unit box (the real centre is a normalised
/// point; `libm_contracts::cell_cap_bound` checks it).
pub fn any_center(_id: s2rst::s2::CellId) -> Point {
    any_box_point()
}

/// `Cell::approx_area` or `Cell::exact_area`, pure and uninterpreted: one finite non-negative
/// value per cell. The statements that use it are about the union's sum, not the values.
pub fn pure_cell_area(c: Cell) -> f64 {
    f64::from_bits(CELL_AREA.lock().unwrap().get([c.id().id()], || {
        let a: f64 = kani::any();
        kani::assume(a.is_finite() && a >= 0.0);
        a.to_bits()
    }))
}

/// `Cell::average_area_for_level`, pure and uninterpreted: one positive finite value per level.
/// The real function's `powi` is not one value per input to the checker (KI-013), so a statement
/// that compares two of its results needs this; P-UAREA-04 pins the real function to upstream's
/// `ldexp` by enumeration.
pub fn pure_avg_area(level: s2rst::s2::Level) -> f64 {
    f64::from_bits(
        AVG_AREA
            .lock()
            .unwrap()
            .get([u64::from(level.as_u8())], || {
                let a: f64 = kani::any();
                kani::assume(a.is_finite() && a > 0.0);
                a.to_bits()
            }),
    )
}

/// `Cell::contains_point`, pure and uninterpreted: one answer per cell and point. A statement
/// that a method answers as `contains_point` does (P-PRCEL-02) is then about the method's code,
/// not two copies of the projection's divisions, which time out (spec 011, 2026-10-28).
pub fn pure_cell_contains(c: Cell, p: Point) -> bool {
    let b = bits3(p);
    CELL_CONTAINS
        .lock()
        .unwrap()
        .get([c.id().id(), b[0], b[1], b[2]], || {
            u64::from(kani::any::<bool>())
        })
        == 1
}
