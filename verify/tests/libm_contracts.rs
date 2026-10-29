// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The function contracts of `src/stubs.rs` against the real `asin`, `atan2` and `cos` (spec 008
//! FR-003, research R5), `tan`, `atan`, `sin`, `sin_cos` (gap 2, 2026-10-26), `sqrt` and the
//! library's `Cell::rect_bound` (spec 010 research R5, R6), and `sqrt_between`, `pure_leaf`
//! (`CellId::from_point`), `pure_cell_cap` and `any_center` (`Cell::cap_bound`,
//! `CellId::to_point`) (spec 011 research R5): every special value, and
//! 10⁶ random inputs per function from a fixed seed, as uniform bit patterns and as uniform values
//! in the function's domain.
//!
//! The predicates below restate `asin_contract`, `atan2_contract` and `cos_contract`; a drift
//! between a predicate and its stub is a review finding (contracts/harness-conventions.md). A
//! proof that uses a contract holds for every value the contract allows; these tests show the
//! real function's value is one of them, on this platform's `libm`.

use std::f64::consts::{FRAC_PI_2, PI};

/// `asin_contract`: NaN outside [−1, 1] and for NaN; ±0 for ±0 (bit for bit); otherwise in
/// [−π/2, π/2] with the argument's sign.
fn asin_ok(x: f64, r: f64) -> bool {
    if x.is_nan() || x.abs() > 1.0 {
        return r.is_nan();
    }
    if x == 0.0 {
        return r.to_bits() == x.to_bits();
    }
    r.abs() <= FRAC_PI_2 && if x > 0.0 { r >= 0.0 } else { r <= 0.0 }
}

/// `atan2_contract`: NaN if either argument is NaN; otherwise in [−π, π], with `y`'s sign bit,
/// and within [−π/2, π/2] when `x` has a clear sign bit.
fn atan2_ok(y: f64, x: f64, r: f64) -> bool {
    if y.is_nan() || x.is_nan() {
        return r.is_nan();
    }
    r.abs() <= PI
        && if y.is_sign_positive() {
            r >= 0.0
        } else {
            r <= 0.0
        }
        && (!x.is_sign_positive() || r.abs() <= FRAC_PI_2)
}

/// `cos_contract`: NaN for a non-finite argument; 1 at ±0; otherwise in [−1, 1], and at least
/// 6·10⁻¹⁷ where |x| ≤ `FRAC_PI_2`.
fn cos_ok(x: f64, r: f64) -> bool {
    if !x.is_finite() {
        return r.is_nan();
    }
    if x == 0.0 {
        return r == 1.0;
    }
    r.abs() <= 1.0 && (x.abs() > FRAC_PI_2 || r >= 6.0e-17)
}

/// `tan_contract`: NaN for a non-finite argument; ±0 for ±0 (bit for bit); otherwise finite, at
/// least 0 on (0, `FRAC_PI_2`], at most 0 on [−`FRAC_PI_2`, 0), and at most 1.7·10¹⁶ in magnitude
/// where |x| ≤ 2.
fn tan_ok(x: f64, r: f64) -> bool {
    if !x.is_finite() {
        return r.is_nan();
    }
    if x == 0.0 {
        return r.to_bits() == x.to_bits();
    }
    r.is_finite()
        && (x.abs() > 2.0 || r.abs() <= 1.7e16)
        && !(x > 0.0 && x <= FRAC_PI_2 && r < 0.0)
        && !(x < 0.0 && x >= -FRAC_PI_2 && r > 0.0)
}

/// `atan_contract`: NaN for NaN; ±0 for ±0 (bit for bit); otherwise in [−π/2, π/2] with the
/// argument's sign.
fn atan_ok(x: f64, r: f64) -> bool {
    if x.is_nan() {
        return r.is_nan();
    }
    if x == 0.0 {
        return r.to_bits() == x.to_bits();
    }
    r.abs() <= FRAC_PI_2 && if x > 0.0 { r >= 0.0 } else { r <= 0.0 }
}

/// `sin_contract`: NaN for a non-finite argument; ±0 for ±0 (bit for bit); otherwise in
/// [−1, 1], at least 0 on (0, π] and at most 0 on [−π, 0).
fn sin_ok(x: f64, r: f64) -> bool {
    if !x.is_finite() {
        return r.is_nan();
    }
    if x == 0.0 {
        return r.to_bits() == x.to_bits();
    }
    r.abs() <= 1.0 && !(x > 0.0 && x <= PI && r < 0.0) && !(x < 0.0 && x >= -PI && r > 0.0)
}

/// `sin_cos_contract`: NaN twice for a non-finite argument; (±0, 1) for ±0 (bit for bit);
/// otherwise both in [−1, 1], the larger in magnitude at least 0.7.
fn sin_cos_ok(x: f64, (s, c): (f64, f64)) -> bool {
    if !x.is_finite() {
        return s.is_nan() && c.is_nan();
    }
    if x == 0.0 {
        return s.to_bits() == x.to_bits() && c == 1.0;
    }
    s.abs() <= 1.0 && c.abs() <= 1.0 && (s.abs() >= 0.7 || c.abs() >= 0.7)
}

/// xorshift64*, fixed seed: no dependency, the same sequence everywhere.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Any `f64` bit pattern, NaNs and infinities included.
    fn bits(&mut self) -> f64 {
        f64::from_bits(self.next())
    }

    /// Uniform in [lo, hi].
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        let u = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
        lo + (hi - lo) * u
    }
}

const RANDOM: usize = 1_000_000;

/// Special values every contract is checked on.
fn specials() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        1.0 + f64::EPSILON,
        -1.0 - f64::EPSILON,
        1.0 - f64::EPSILON / 2.0,
        -(1.0 - f64::EPSILON / 2.0),
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        FRAC_PI_2,
        -FRAC_PI_2,
        FRAC_PI_2.next_up(),
        FRAC_PI_2.next_down(),
        -FRAC_PI_2.next_up(),
        -FRAC_PI_2.next_down(),
        PI,
        -PI,
        1.1e-308,
        f64::MAX,
        -f64::MAX,
    ];
    v.extend([0.5, -0.5, 1e-300, -1e-300, 3.0, 1e20]);
    v
}

#[test]
fn asin_satisfies_its_contract() {
    for x in specials() {
        assert!(asin_ok(x, x.asin()), "asin({x:e}) = {:e}", x.asin());
    }
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for i in 0..RANDOM {
        let x = if i % 2 == 0 {
            rng.bits()
        } else {
            rng.uniform(-1.0, 1.0)
        };
        assert!(asin_ok(x, x.asin()), "asin({x:e}) = {:e}", x.asin());
    }
}

#[test]
fn atan2_satisfies_its_contract() {
    let s = specials();
    for &y in &s {
        for &x in &s {
            assert!(
                atan2_ok(y, x, y.atan2(x)),
                "atan2({y:e}, {x:e}) = {:e}",
                y.atan2(x)
            );
        }
    }
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for i in 0..RANDOM {
        let (y, x) = if i % 2 == 0 {
            (rng.bits(), rng.bits())
        } else {
            (rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0))
        };
        assert!(
            atan2_ok(y, x, y.atan2(x)),
            "atan2({y:e}, {x:e}) = {:e}",
            y.atan2(x)
        );
    }
}

#[test]
fn cos_satisfies_its_contract() {
    for x in specials() {
        assert!(cos_ok(x, x.cos()), "cos({x:e}) = {:e}", x.cos());
    }
    let mut rng = Rng(0x94D0_49BB_1331_11EB);
    for i in 0..RANDOM {
        let x = if i % 2 == 0 {
            rng.bits()
        } else {
            rng.uniform(-FRAC_PI_2, FRAC_PI_2)
        };
        assert!(cos_ok(x, x.cos()), "cos({x:e}) = {:e}", x.cos());
    }
}

/// The contracts' bounds are tight where they matter: cos at the `f64` nearest π/2 is just
/// above the 6·10⁻¹⁷ floor, and asin at 1 is the `f64` nearest π/2.
#[test]
fn the_bounds_are_tight() {
    assert!(FRAC_PI_2.cos() > 6.0e-17 && FRAC_PI_2.cos() < 6.2e-17);
    assert_eq!(1.0f64.asin(), FRAC_PI_2);
    assert_eq!(0.0f64.atan2(-1.0), PI);
}

/// The doubles nearest the odd multiples of π/2 in [−2, 2], where `tan` is largest.
fn near_half_pi() -> Vec<f64> {
    let mut v = Vec::new();
    for x in [FRAC_PI_2, -FRAC_PI_2] {
        let mut y = x;
        for _ in 0..64 {
            v.push(y);
            y = y.next_up();
        }
        y = x;
        for _ in 0..64 {
            y = y.next_down();
            v.push(y);
        }
    }
    v
}

#[test]
fn tan_satisfies_its_contract() {
    for x in specials().into_iter().chain(near_half_pi()) {
        assert!(tan_ok(x, x.tan()), "tan({x:e}) = {:e}", x.tan());
    }
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for i in 0..RANDOM {
        let x = if i % 2 == 0 {
            rng.bits()
        } else {
            rng.uniform(-2.0, 2.0)
        };
        assert!(tan_ok(x, x.tan()), "tan({x:e}) = {:e}", x.tan());
    }
}

#[test]
fn atan_satisfies_its_contract() {
    for x in specials() {
        assert!(atan_ok(x, x.atan()), "atan({x:e}) = {:e}", x.atan());
    }
    let mut rng = Rng(0x94D0_49BB_1331_11EB);
    for i in 0..RANDOM {
        let x = if i % 2 == 0 {
            rng.bits()
        } else {
            rng.uniform(-10.0, 10.0)
        };
        assert!(atan_ok(x, x.atan()), "atan({x:e}) = {:e}", x.atan());
    }
}

#[test]
fn sin_cos_satisfies_its_contract() {
    for x in specials() {
        assert!(
            sin_cos_ok(x, x.sin_cos()),
            "sin_cos({x:e}) = {:?}",
            x.sin_cos()
        );
    }
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for i in 0..RANDOM {
        let x = if i % 2 == 0 {
            rng.bits()
        } else {
            rng.uniform(-4.0, 4.0)
        };
        assert!(
            sin_cos_ok(x, x.sin_cos()),
            "sin_cos({x:e}) = {:?}",
            x.sin_cos()
        );
    }
}

#[test]
fn sin_satisfies_its_contract() {
    for x in
        specials()
            .into_iter()
            .chain([PI.next_down(), -PI.next_down(), PI.next_up(), -PI.next_up()])
    {
        assert!(sin_ok(x, x.sin()), "sin({x:e}) = {:e}", x.sin());
    }
    let mut rng = Rng(0x1405_7B7E_F767_814F);
    for i in 0..RANDOM {
        let x = if i % 2 == 0 {
            rng.bits()
        } else {
            rng.uniform(-4.0, 4.0)
        };
        assert!(sin_ok(x, x.sin()), "sin({x:e}) = {:e}", x.sin());
    }
}

/// `sqrt_contract` (spec 010 research R5): NaN for NaN and for `x < 0`; `x` itself for ±0 (bit
/// for bit) and +∞; otherwise positive and finite, at most 1 when `x ≤ 1`, at least 1 when
/// `x ≥ 1`.
fn sqrt_ok(x: f64, r: f64) -> bool {
    if x.is_nan() || x < 0.0 {
        return r.is_nan();
    }
    if x == 0.0 || x == f64::INFINITY {
        return r.to_bits() == x.to_bits();
    }
    r > 0.0 && r.is_finite() && (x > 1.0 || r <= 1.0) && (x < 1.0 || r >= 1.0)
}

/// `sqrt_between`'s extra clause (spec 011): the root lies between `x` and 1.
fn sqrt_between_ok(x: f64, r: f64) -> bool {
    sqrt_ok(x, r) && (!x.is_finite() || x <= 0.0 || if x >= 1.0 { r <= x } else { r >= x })
}

#[test]
fn sqrt_satisfies_its_contract() {
    for x in
        specials()
            .into_iter()
            .chain([1.0f64.next_up(), 1.0f64.next_down(), f64::MAX.next_down()])
    {
        assert!(sqrt_between_ok(x, x.sqrt()), "sqrt({x:e}) = {:e}", x.sqrt());
    }
    let mut rng = Rng(0x5851_F42D_4C95_7F2D);
    for i in 0..RANDOM {
        let x = if i % 2 == 0 {
            rng.bits()
        } else {
            rng.uniform(0.0, 4.0)
        };
        assert!(sqrt_between_ok(x, x.sqrt()), "sqrt({x:e}) = {:e}", x.sqrt());
    }
}

/// `cell_rect_bound_contract` (spec 010 research R6): a cell's bound is valid and contains the
/// `LatLng` of each of its four vertices, with the real `rect_bound`, `vertex`, `from_point` and
/// `contains_lat_lng`.
fn cell_bound_ok(cell: s2rst::s2::Cell) -> bool {
    let r = cell.rect_bound();
    r.is_valid()
        && s2rst::s2::CellVertex::ALL
            .into_iter()
            .all(|k| r.contains_lat_lng(s2rst::s2::LatLng::from_point(cell.vertex(k))))
}

#[test]
fn cell_rect_bound_satisfies_its_contract() {
    use s2rst::s2::{Cell, CellId, Face, Level};
    // Every cell to level 5: 8,190 cells.
    let mut n = 0;
    for level in 0..=5u8 {
        let level = Level::try_from(level).unwrap();
        let mut id = CellId::begin(level);
        while id != CellId::end(level) {
            assert!(cell_bound_ok(Cell::from(id)), "{id:?}");
            id = id.next();
            n += 1;
        }
    }
    assert_eq!(n, 8_190);
    // 10⁶ random cells, every level as likely as any other.
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for _ in 0..RANDOM {
        let face = Face::ALL[(rng.next() % 6) as usize];
        let level = Level::try_from((rng.next() % 31) as u8).unwrap();
        let id = CellId::from_face_pos_level(face, rng.next() & ((1 << 61) - 1), level);
        assert!(cell_bound_ok(Cell::from(id)), "{id:?}");
    }
}

/// `pure_leaf`'s basis (spec 011 research R5, L-PR-01): wherever `CellId::from_point` returns, it
/// returns a valid leaf; it panics (debug builds) only where its documentation says: the zero
/// vector, a NaN coordinate, or two or more infinite coordinates (spec 011 K4). Every special
/// triple, and 10⁶ random triples over 600 decades.
#[test]
fn cell_id_from_point_returns_a_valid_leaf() {
    use s2rst::r3::Vector;
    use s2rst::s2::{CellId, Level, Point};
    let check = |x: f64, y: f64, z: f64| {
        let r = std::panic::catch_unwind(|| CellId::from_point(&Point(Vector { x, y, z })));
        match r {
            Ok(id) => assert!(id.is_valid() && id.level() == Level::MAX, "({x}, {y}, {z})"),
            Err(_) => {
                let c = [x, y, z];
                let documented = c.iter().all(|&t| t == 0.0)
                    || c.iter().any(|t| t.is_nan())
                    || c.iter().filter(|t| t.is_infinite()).count() >= 2;
                assert!(documented, "an undocumented panic at ({x}, {y}, {z})");
            }
        }
    };
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let v = specials();
    for &x in &v {
        for &y in &v {
            for &z in &v {
                check(x, y, z);
            }
        }
    }
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..RANDOM {
        let w =
            |rng: &mut Rng| rng.uniform(-1.0, 1.0) * 10f64.powi((rng.next() % 600) as i32 - 300);
        let (x, y, z) = (w(&mut rng), w(&mut rng), w(&mut rng));
        check(x, y, z);
    }
    std::panic::set_hook(hook);
}

/// `pure_cell_cap`'s and `any_center`'s basis (spec 011 research R5): a cell's cap bound has a
/// centre in the unit box and a squared radius in [0, 4], and its id's centre point is in the box,
/// on every cell to level 5 and 10⁶ random cells.
fn cell_cap_ok(cell: s2rst::s2::Cell) -> bool {
    let in_box =
        |p: s2rst::s2::Point| p.x().abs() <= 1.0 && p.y().abs() <= 1.0 && p.z().abs() <= 1.0;
    let cap = cell.cap_bound();
    let l = cap.chord_radius().length2();
    in_box(cap.center()) && (0.0..=4.0).contains(&l) && in_box(cell.id().to_point())
}

#[test]
fn cell_cap_bound_satisfies_its_contract() {
    use s2rst::s2::{Cell, CellId, Face, Level};
    for level in 0..=5u8 {
        let level = Level::try_from(level).unwrap();
        let mut id = CellId::begin(level);
        while id != CellId::end(level) {
            assert!(cell_cap_ok(Cell::from(id)), "{id:?}");
            id = id.next();
        }
    }
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..RANDOM {
        let face = Face::ALL[(rng.next() % 6) as usize];
        let level = Level::try_from((rng.next() % 31) as u8).unwrap();
        let id = CellId::from_face_pos_level(face, rng.next() & ((1 << 61) - 1), level);
        assert!(cell_cap_ok(Cell::from(id)), "{id:?}");
    }
}
