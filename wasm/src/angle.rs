// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Angle
// ---------------------------------------------------------------------------

/// A one-dimensional angle (stored as radians).
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Angle(pub(crate) s2rst::s1::Angle);

#[wasm_bindgen]
impl Angle {
    /// Zero angle.
    #[wasm_bindgen(js_name = "zero")]
    pub fn zero() -> Angle {
        Angle(s2rst::s1::Angle::ZERO)
    }

    /// Infinite angle.
    #[wasm_bindgen(js_name = "infinity")]
    pub fn infinity_val() -> Angle {
        Angle(s2rst::s1::Angle::INFINITY)
    }

    /// Create from radians.
    #[wasm_bindgen(js_name = "fromRadians")]
    pub fn from_radians(radians: f64) -> Angle {
        Angle(s2rst::s1::Angle::from_radians(radians))
    }

    /// Create from degrees.
    #[wasm_bindgen(js_name = "fromDegrees")]
    pub fn from_degrees(degrees: f64) -> Angle {
        Angle(s2rst::s1::Angle::from_degrees(degrees))
    }

    /// Create from E5 representation (degrees × 10⁵).
    #[wasm_bindgen(js_name = "fromE5")]
    pub fn from_e5(e5: i32) -> Angle {
        Angle(s2rst::s1::Angle::from_e5(e5))
    }

    /// Create from E6 representation (degrees × 10⁶).
    #[wasm_bindgen(js_name = "fromE6")]
    pub fn from_e6(e6: i32) -> Angle {
        Angle(s2rst::s1::Angle::from_e6(e6))
    }

    /// Create from E7 representation (degrees × 10⁷).
    #[wasm_bindgen(js_name = "fromE7")]
    pub fn from_e7(e7: i32) -> Angle {
        Angle(s2rst::s1::Angle::from_e7(e7))
    }

    #[wasm_bindgen(js_name = "fromUnsignedE6")]
    pub fn from_unsigned_e6(e6: u32) -> Angle {
        Angle(s2rst::s1::Angle::from_unsigned_e6(e6))
    }

    #[wasm_bindgen(js_name = "fromUnsignedE7")]
    pub fn from_unsigned_e7(e7: u32) -> Angle {
        Angle(s2rst::s1::Angle::from_unsigned_e7(e7))
    }

    /// The angle in radians.
    #[wasm_bindgen(getter)]
    pub fn radians(&self) -> f64 {
        self.0.radians()
    }

    /// The angle in degrees.
    #[wasm_bindgen(getter)]
    pub fn degrees(&self) -> f64 {
        self.0.degrees()
    }

    /// E5 representation.
    pub fn e5(&self) -> i32 {
        self.0.e5()
    }

    /// E6 representation.
    pub fn e6(&self) -> i32 {
        self.0.e6()
    }

    /// E7 representation.
    pub fn e7(&self) -> i32 {
        self.0.e7()
    }

    /// Absolute value.
    pub fn abs(&self) -> Angle {
        Angle(self.0.abs())
    }

    /// Equivalent angle in (−π, π].
    pub fn normalized(&self) -> Angle {
        Angle(self.0.normalized())
    }

    pub fn sin(&self) -> f64 {
        self.0.sin()
    }
    pub fn cos(&self) -> f64 {
        self.0.cos()
    }
    pub fn tan(&self) -> f64 {
        self.0.tan()
    }

    /// Whether this angle is infinite.
    #[wasm_bindgen(js_name = "isInfinite")]
    pub fn is_infinite(&self) -> bool {
        self.0.is_infinite()
    }

    /// Whether approximately equal (within 1e-15 radians).
    #[wasm_bindgen(js_name = "approxEq")]
    pub fn approx_eq(&self, other: &Angle) -> bool {
        self.0.approx_eq(other.0)
    }

    /// Add two angles.
    pub fn add(&self, other: &Angle) -> Angle {
        Angle(self.0 + other.0)
    }

    /// Subtract another angle.
    pub fn sub(&self, other: &Angle) -> Angle {
        Angle(self.0 - other.0)
    }

    /// Multiply by scalar.
    pub fn mul(&self, scalar: f64) -> Angle {
        Angle(self.0 * scalar)
    }

    /// Divide by scalar.
    pub fn div(&self, scalar: f64) -> Angle {
        Angle(self.0 / scalar)
    }

    /// Ratio of two angles.
    pub fn ratio(&self, other: &Angle) -> f64 {
        self.0 / other.0
    }

    /// Negate.
    pub fn neg(&self) -> Angle {
        Angle(-self.0)
    }

    /// String representation.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// ChordAngle
// ---------------------------------------------------------------------------

/// An angle represented as the squared chord length on the unit sphere.
/// More efficient than `Angle` for distance comparisons.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct ChordAngle(pub(crate) s2rst::s1::ChordAngle);

#[wasm_bindgen]
impl ChordAngle {
    #[wasm_bindgen(js_name = "zero")]
    pub fn zero() -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::ZERO)
    }

    #[wasm_bindgen(js_name = "right")]
    pub fn right() -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::RIGHT)
    }

    #[wasm_bindgen(js_name = "straight")]
    pub fn straight() -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::STRAIGHT)
    }

    #[wasm_bindgen(js_name = "infinity")]
    pub fn infinity_val() -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::INFINITY)
    }

    #[wasm_bindgen(js_name = "negative")]
    pub fn negative() -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::NEGATIVE)
    }

    /// Create from squared chord length.
    #[wasm_bindgen(js_name = "fromLength2")]
    pub fn from_length2(length2: f64) -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::from_length2(length2))
    }

    /// Create from an `Angle`.
    #[wasm_bindgen(js_name = "fromAngle")]
    pub fn from_angle(angle: &Angle) -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::from_angle(angle.0))
    }

    /// Create from radians.
    #[wasm_bindgen(js_name = "fromRadians")]
    pub fn from_radians(radians: f64) -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::from_radians(radians))
    }

    /// Create from degrees.
    #[wasm_bindgen(js_name = "fromDegrees")]
    pub fn from_degrees(degrees: f64) -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::from_degrees(degrees))
    }

    /// The squared chord length.
    #[wasm_bindgen(getter)]
    pub fn length2(&self) -> f64 {
        self.0.length2()
    }

    /// Convert to Angle.
    #[wasm_bindgen(js_name = "toAngle")]
    pub fn to_angle(&self) -> Angle {
        Angle(self.0.to_angle())
    }

    #[wasm_bindgen(getter)]
    pub fn radians(&self) -> f64 {
        self.0.radians()
    }

    #[wasm_bindgen(getter)]
    pub fn degrees(&self) -> f64 {
        self.0.degrees()
    }

    #[wasm_bindgen(js_name = "isZero")]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[wasm_bindgen(js_name = "isNegative")]
    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    #[wasm_bindgen(js_name = "isInfinity")]
    pub fn is_infinity(&self) -> bool {
        self.0.is_infinity()
    }

    #[wasm_bindgen(js_name = "isSpecial")]
    pub fn is_special(&self) -> bool {
        self.0.is_special()
    }

    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    #[wasm_bindgen(js_name = "fromE5")]
    pub fn from_e5(e5: i32) -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::from_e5(e5))
    }

    #[wasm_bindgen(js_name = "fromE6")]
    pub fn from_e6(e6: i32) -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::from_e6(e6))
    }

    #[wasm_bindgen(js_name = "fromE7")]
    pub fn from_e7(e7: i32) -> ChordAngle {
        ChordAngle(s2rst::s1::ChordAngle::from_e7(e7))
    }

    pub fn e5(&self) -> i32 {
        self.0.e5()
    }
    pub fn e6(&self) -> i32 {
        self.0.e6()
    }
    pub fn e7(&self) -> i32 {
        self.0.e7()
    }

    pub fn successor(&self) -> ChordAngle {
        ChordAngle(self.0.successor())
    }

    pub fn predecessor(&self) -> ChordAngle {
        ChordAngle(self.0.predecessor())
    }

    /// Add an absolute error bound, returning a (clamped) chord angle.
    #[wasm_bindgen(js_name = "plusError")]
    pub fn plus_error(&self, error: f64) -> ChordAngle {
        ChordAngle(self.0.plus_error(error))
    }

    /// Maximum error (in chord-length²) when computing this from a point pair.
    #[wasm_bindgen(js_name = "maxPointError")]
    pub fn max_point_error(&self) -> f64 {
        self.0.max_point_error()
    }

    /// Maximum error (in chord-length²) when converting from an `Angle`.
    #[wasm_bindgen(js_name = "maxAngleError")]
    pub fn max_angle_error(&self) -> f64 {
        self.0.max_angle_error()
    }

    pub fn sin(&self) -> f64 {
        self.0.sin()
    }
    pub fn sin2(&self) -> f64 {
        self.0.sin2()
    }
    pub fn cos(&self) -> f64 {
        self.0.cos()
    }
    pub fn tan(&self) -> f64 {
        self.0.tan()
    }

    pub fn add(&self, other: &ChordAngle) -> ChordAngle {
        ChordAngle(self.0 + other.0)
    }

    pub fn sub(&self, other: &ChordAngle) -> ChordAngle {
        ChordAngle(self.0 - other.0)
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.0)
    }
}
