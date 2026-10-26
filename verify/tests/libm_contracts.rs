// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The function contracts of `src/stubs.rs` against the real `asin`, `atan2` and `cos` (spec 008
//! FR-003, research R5): every special value, and 10⁶ random inputs per function from a fixed
//! seed, as uniform bit patterns and as uniform values in the function's domain.
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
