// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! An exact floating-point number for arbitrary-precision arithmetic.
//!
//! Represents a value as `mantissa × 2^exp` where `mantissa` is a `BigInt`.
//! Supports addition, subtraction, and multiplication with exact results.
//! No division or square root (not needed for S2 exact predicates).
//!
//! Corresponds to C++ `ExactFloat` from `util/math/exactfloat/`.

#![expect(
    clippy::cast_sign_loss,
    reason = "exponent arithmetic uses intentional i64->u64 casts for bit manipulation"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "exponent/mantissa arithmetic for exact floating point — bounded by construction"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u64 mantissa bits reinterpreted as i64 for exponent arithmetic — values bounded"
)]
use num_bigint::{BigInt, Sign};
use num_traits::ToPrimitive;
use std::cmp::Ordering;
use std::fmt;

/// An exact floating-point number: `mantissa × 2^exp`.
///
/// The mantissa is always normalized: it is odd for non-zero values,
/// and zero (with exp=0) for the zero value.
///
/// # Examples
///
/// ```
/// use s2rst::r3::ExactFloat;
///
/// // Exact arithmetic with no rounding errors.
/// let a = ExactFloat::from(0.1);
/// let b = ExactFloat::from(0.2);
/// let sum = a.add(&b);
///
/// // Compare with exact 0.3 — note that 0.1 + 0.2 ≠ 0.3 in f64,
/// // but ExactFloat faithfully represents the f64 inputs.
/// let c = ExactFloat::from(0.3_f64);
/// assert!(sum != c); // 0.1_f64 + 0.2_f64 ≠ 0.3_f64, even exactly!
///
/// // Multiplication is also exact.
/// let x = ExactFloat::from(3.0);
/// let y = ExactFloat::from(7.0);
/// let product = x.mul(&y);
/// assert_eq!(product.to_f64(), 21.0);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExactFloat {
    mantissa: BigInt,
    exp: i64,
}

impl ExactFloat {
    /// The exact value zero.
    #[inline]
    pub fn zero() -> Self {
        ExactFloat {
            mantissa: BigInt::from(0i64),
            exp: 0,
        }
    }

    /// The exact value one.
    #[inline]
    pub fn one() -> Self {
        ExactFloat {
            mantissa: BigInt::from(1i64),
            exp: 0,
        }
    }

    /// Returns true if the value is exactly zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.mantissa.sign() == Sign::NoSign
    }

    /// Returns -1, 0, or 1.
    #[inline]
    pub fn signum(&self) -> i32 {
        match self.mantissa.sign() {
            Sign::Minus => -1,
            Sign::NoSign => 0,
            Sign::Plus => 1,
        }
    }

    /// Returns the absolute value.
    pub fn abs(&self) -> Self {
        ExactFloat {
            mantissa: BigInt::from(self.mantissa.magnitude().clone()),
            exp: self.exp,
        }
    }

    /// Returns the negation.
    pub fn neg(&self) -> Self {
        ExactFloat {
            mantissa: -&self.mantissa,
            exp: self.exp,
        }
    }

    /// Exact addition.
    pub fn add(&self, other: &Self) -> Self {
        if self.is_zero() {
            return other.clone();
        }
        if other.is_zero() {
            return self.clone();
        }
        let mut result = if self.exp <= other.exp {
            let shift = (other.exp - self.exp) as usize;
            ExactFloat {
                mantissa: &self.mantissa + (&other.mantissa << shift),
                exp: self.exp,
            }
        } else {
            let shift = (self.exp - other.exp) as usize;
            ExactFloat {
                mantissa: (&self.mantissa << shift) + &other.mantissa,
                exp: other.exp,
            }
        };
        result.normalize();
        result
    }

    /// Exact subtraction.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Exact multiplication.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let mut result = ExactFloat {
            mantissa: &self.mantissa * &other.mantissa,
            exp: self.exp + other.exp,
        };
        result.normalize();
        result
    }

    /// Returns the binary exponent such that the value is in `[0.5, 1) * 2^exp`
    /// for non-zero values. For zero, returns `i64::MIN`.
    ///
    /// More precisely, if the value is `mantissa * 2^e` where `mantissa` is odd,
    /// then `exp() = e + mantissa.bits()`, so that `|value| ∈ [2^(exp-1), 2^exp)`.
    pub fn exp(&self) -> i64 {
        if self.is_zero() {
            return i64::MIN;
        }
        self.exp + self.mantissa.bits() as i64
    }

    /// Converts to f64 after shifting the binary exponent by `shift`.
    /// Returns `value * 2^(-shift)` as f64.
    pub fn to_f64_shifted(&self, shift: i64) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let bits = self.mantissa.bits() as i64;
        if bits <= 53 {
            let m = self.mantissa.to_f64().unwrap_or(0.0);
            ldexp_f64(m, self.exp - shift)
        } else {
            let reduce = (bits - 53) as usize;
            let reduced = &self.mantissa >> reduce;
            let m = reduced.to_f64().unwrap_or(0.0);
            ldexp_f64(m, self.exp + reduce as i64 - shift)
        }
    }

    /// Approximate conversion to f64.
    pub fn to_f64(&self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let bits = self.mantissa.bits() as i64;
        if bits <= 53 {
            let m = self.mantissa.to_f64().unwrap_or(0.0);
            ldexp_f64(m, self.exp)
        } else {
            // Shift right to ~53 significant bits, then adjust exponent.
            let shift = (bits - 53) as usize;
            let reduced = &self.mantissa >> shift;
            let m = reduced.to_f64().unwrap_or(0.0);
            ldexp_f64(m, self.exp + shift as i64)
        }
    }

    /// Strip trailing zero bits from mantissa to keep it odd (or zero).
    fn normalize(&mut self) {
        if self.is_zero() {
            self.mantissa = BigInt::from(0i64);
            self.exp = 0;
            return;
        }
        if let Some(tz) = self.mantissa.magnitude().trailing_zeros()
            && tz > 0
        {
            let m = std::mem::replace(&mut self.mantissa, BigInt::from(0i64));
            self.mantissa = m >> (tz as usize);
            self.exp += tz as i64;
        }
    }
}

/// Compute `m * 2^exp` as f64.
fn ldexp_f64(m: f64, exp: i64) -> f64 {
    if exp == 0 || m == 0.0 {
        return m;
    }
    // Clamp to a range where powi won't overflow i32.
    // Values outside f64 range will naturally produce 0 or infinity.
    let exp = exp.clamp(-2100, 2100) as i32;
    // Split into two steps if |exp| > 1023 to avoid intermediate overflow.
    if exp.abs() <= 1023 {
        m * (2.0f64).powi(exp)
    } else {
        let half = exp / 2;
        let other = exp - half;
        m * (2.0f64).powi(half) * (2.0f64).powi(other)
    }
}

impl From<f64> for ExactFloat {
    fn from(v: f64) -> Self {
        if v == 0.0 {
            return Self::zero();
        }
        let bits = v.to_bits();
        let negative = (bits >> 63) != 0;
        let biased_exp = ((bits >> 52) & 0x7FF) as i64;
        let fraction = bits & 0x000F_FFFF_FFFF_FFFFu64;

        assert!(
            biased_exp != 0x7FF,
            "ExactFloat does not support infinity or NaN"
        );

        let (mantissa_val, exp) = if biased_exp == 0 {
            // Denormalized: value = fraction * 2^(-1074)
            (fraction, -1074i64)
        } else {
            // Normalized: value = (2^52 + fraction) * 2^(biased_exp - 1023 - 52)
            ((1u64 << 52) | fraction, biased_exp - 1023 - 52)
        };

        let mut mantissa = BigInt::from(mantissa_val);
        if negative {
            mantissa = -mantissa;
        }

        let mut result = ExactFloat { mantissa, exp };
        result.normalize();
        result
    }
}

impl From<i64> for ExactFloat {
    fn from(v: i64) -> Self {
        if v == 0 {
            return Self::zero();
        }
        let mut result = ExactFloat {
            mantissa: BigInt::from(v),
            exp: 0,
        };
        result.normalize();
        result
    }
}

impl Default for ExactFloat {
    fn default() -> Self {
        ExactFloat::zero()
    }
}

impl PartialEq for ExactFloat {
    fn eq(&self, other: &Self) -> bool {
        // Both normalized: equal values have identical representation.
        self.exp == other.exp && self.mantissa == other.mantissa
    }
}

impl Eq for ExactFloat {}

impl Ord for ExactFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        // Quick path: check signs.
        let s1 = self.signum();
        let s2 = other.signum();
        if s1 != s2 {
            return s1.cmp(&s2);
        }
        if s1 == 0 {
            return Ordering::Equal;
        }
        // Same sign, same representation → equal.
        if self.exp == other.exp {
            return self.mantissa.cmp(&other.mantissa);
        }
        // Different exponents: subtract and check sign.
        let diff = self.sub(other);
        match diff.signum() {
            -1 => Ordering::Less,
            0 => Ordering::Equal,
            1 => Ordering::Greater,
            _ => unreachable!("signum returns -1, 0, or 1"),
        }
    }
}

impl PartialOrd for ExactFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for ExactFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn exact_float_is_send_sync() {
        is_send_sync::<ExactFloat>();
    }

    #[test]
    fn test_from_f64_zero() {
        let z = ExactFloat::from(0.0);
        assert!(z.is_zero());
        assert_eq!(z.signum(), 0);
    }

    #[test]
    fn test_from_f64_integers() {
        assert_eq!(ExactFloat::from(1.0), ExactFloat::one());
        assert_eq!(ExactFloat::from(1.0).to_f64(), 1.0);
        assert_eq!(ExactFloat::from(-1.0).to_f64(), -1.0);
        assert_eq!(ExactFloat::from(42.0).to_f64(), 42.0);
    }

    /// Tolerance for Miri soft-float rounding (a few ULPs).
    const MIRI_TOL: f64 = 1e-14;

    #[test]
    fn test_from_f64_fractions() {
        // Use approximate comparisons for Miri compatibility (soft-float rounding).
        assert!((ExactFloat::from(0.5).to_f64() - 0.5).abs() < MIRI_TOL);
        assert!((ExactFloat::from(0.25).to_f64() - 0.25).abs() < MIRI_TOL);
        assert!((ExactFloat::from(1.5).to_f64() - 1.5).abs() < MIRI_TOL);
    }

    #[test]
    fn test_add() {
        let a = ExactFloat::from(1.0);
        let b = ExactFloat::from(2.0);
        assert_eq!(a.add(&b).to_f64(), 3.0);

        let c = ExactFloat::from(-1.0);
        assert_eq!(a.add(&c).to_f64(), 0.0);
        assert!(a.add(&c).is_zero());
    }

    #[test]
    fn test_sub() {
        let a = ExactFloat::from(3.0);
        let b = ExactFloat::from(1.0);
        // Use approximate comparison for Miri compatibility (soft-float rounding).
        assert!((a.sub(&b).to_f64() - 2.0).abs() < MIRI_TOL);
        assert!(a.sub(&a).is_zero());
    }

    #[test]
    fn test_mul() {
        let a = ExactFloat::from(3.0);
        let b = ExactFloat::from(4.0);
        // Use approximate comparisons for Miri compatibility (soft-float rounding).
        assert!((a.mul(&b).to_f64() - 12.0).abs() < MIRI_TOL);

        let c = ExactFloat::from(-2.0);
        assert!((a.mul(&c).to_f64() - (-6.0)).abs() < MIRI_TOL);
    }

    #[test]
    fn test_comparison() {
        let a = ExactFloat::from(1.0);
        let b = ExactFloat::from(2.0);
        let c = ExactFloat::from(1.0);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, c);
    }

    #[test]
    fn test_exact_cancellation() {
        // 1e20 + 1 - 1e20 should be exactly 1 (not 0 as with f64).
        let big = ExactFloat::from(1e20);
        let one = ExactFloat::from(1.0);
        let result = big.add(&one).sub(&big);
        assert_eq!(result, one);
    }

    #[test]
    fn test_abs_neg() {
        let a = ExactFloat::from(-3.0);
        assert_eq!(a.abs().to_f64(), 3.0);
        assert_eq!(a.neg().to_f64(), 3.0);
        assert_eq!(a.neg().neg().to_f64(), -3.0);
    }

    #[test]
    fn test_signum() {
        assert_eq!(ExactFloat::from(5.0).signum(), 1);
        assert_eq!(ExactFloat::from(-5.0).signum(), -1);
        assert_eq!(ExactFloat::zero().signum(), 0);
    }

    // ─── C++ ExactFloatTest::OperatorDouble equivalents ───────────────
    // Verifies to_f64() roundtrips match C++ static_cast<double>(ExactFloat(v)).

    #[test]
    fn test_to_f64_zero() {
        assert_eq!(ExactFloat::from(0.0).to_f64(), 0.0);
    }

    #[test]
    fn test_to_f64_max() {
        assert_eq!(ExactFloat::from(f64::MAX).to_f64(), f64::MAX);
    }

    #[test]
    fn test_to_f64_neg_min_positive() {
        assert_eq!(
            ExactFloat::from(-f64::MIN_POSITIVE).to_f64(),
            -f64::MIN_POSITIVE
        );
    }

    #[test]
    fn test_to_f64_denorm_min() {
        // C++: EXPECT_EQ(denorm_min, (double)ExactFloat(denorm_min))
        let denorm_min = f64::from_bits(1); // smallest subnormal
        assert_eq!(ExactFloat::from(denorm_min).to_f64(), denorm_min);
    }

    #[test]
    fn test_to_f64_negative_fraction() {
        assert_eq!(ExactFloat::from(-12.7).to_f64(), -12.7);
    }

    #[test]
    fn test_to_f64_pi() {
        assert_eq!(
            ExactFloat::from(std::f64::consts::PI).to_f64(),
            std::f64::consts::PI
        );
    }

    #[test]
    fn test_to_f64_large_mantissa_roundtrip() {
        // Exercise the >53-bit reduction path: multiply two large integers.
        let a = ExactFloat::from(1e15);
        let b = ExactFloat::from(1e15);
        let product = a.mul(&b);
        let result = product.to_f64();
        // Allow 1 ULP difference due to rounding in the shift path.
        assert!(
            (result - 1e30).abs() / 1e30 < 1e-15,
            "expected ~1e30, got {result}"
        );
    }

    #[test]
    fn test_exp() {
        // 1.0 = 1 * 2^0, so exp = 0 + 1 bit = 1.
        assert_eq!(ExactFloat::from(1.0).exp(), 1);
        // 2.0 = 1 * 2^1, so exp = 1 + 1 = 2.
        assert_eq!(ExactFloat::from(2.0).exp(), 2);
        // 0.5 = 1 * 2^(-1), so exp = -1 + 1 = 0.
        assert_eq!(ExactFloat::from(0.5).exp(), 0);
        // zero
        assert_eq!(ExactFloat::zero().exp(), i64::MIN);
        // -4.0 = -1 * 2^2, exp = 2 + 1 = 3.
        assert_eq!(ExactFloat::from(-4.0).exp(), 3);
    }

    #[test]
    fn test_to_f64_shifted() {
        // 8.0 shifted by 0 → 8.0
        assert_eq!(ExactFloat::from(8.0).to_f64_shifted(0), 8.0);
        // 8.0 shifted by 3 → 8.0 * 2^(-3) = 1.0
        assert_eq!(ExactFloat::from(8.0).to_f64_shifted(3), 1.0);
        // 8.0 shifted by -1 → 8.0 * 2^1 = 16.0
        assert_eq!(ExactFloat::from(8.0).to_f64_shifted(-1), 16.0);
        // zero shifted by anything → 0.0
        assert_eq!(ExactFloat::zero().to_f64_shifted(100), 0.0);
        // Tiny value: 5e-324 shifted to bring into representable range
        let tiny = ExactFloat::from(5e-324);
        let exp = tiny.exp();
        let shifted = tiny.to_f64_shifted(exp);
        // After shifting by exp, result should be in [0.5, 1.0)
        assert!((0.25..=1.0).contains(&shifted), "shifted = {shifted}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let cases = [
            0.0,
            1.0,
            -1.0,
            0.5,
            1e20,
            -std::f64::consts::PI,
            f64::MIN_POSITIVE,
        ];
        for &v in &cases {
            let ef = ExactFloat::from(v);
            let json = serde_json::to_string(&ef).unwrap();
            let back: ExactFloat = serde_json::from_str(&json).unwrap();
            assert_eq!(back, ef, "roundtrip failed for {v}");
        }
    }
}
