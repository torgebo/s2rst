// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Cell-size metrics at each S2 subdivision level.
//!
//! A [`Metric`] describes how a geometric quantity (length or area) scales
//! with cell level. Use [`Metric::value`] to compute the quantity at a
//! given level, and [`Metric::min_level`] / [`Metric::max_level`] to find
//! the finest or coarsest level that satisfies a constraint.
//!
//! All constants below are for the **quadratic projection** (the default in
//! both C++ and Go).
//!
//! Corresponds to C++ `s2metrics.h`, Go `s2/metric.go`.

#![expect(
    clippy::cast_sign_loss,
    reason = "level (i32) clamped to valid range then cast to u8; exponent computation always non-negative"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "level (i32->u8) after clamping and exponent arithmetic"
)]
use crate::s2::coords::{Level, MAX_CELL_LEVEL};

/// Returns the binary exponent of `x` (equivalent to C's `ilogb`).
///
/// For normal positive floats this is `floor(log2(x))`.
fn ilogb(x: f64) -> i32 {
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    exp - 1023
}

/// A measure for cells that scales exponentially with level.
///
/// `dim` is 1 for length metrics and 2 for area metrics.
/// `deriv` is the scaling factor (the metric's value at level 0 is `deriv`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Metric {
    /// 1 for a length metric, 2 for an area metric.
    pub dim: u8,
    /// The scaling factor for the metric.
    pub deriv: f64,
}

impl Metric {
    /// Creates a new metric with the given dimension and derivative.
    pub const fn new(dim: u8, deriv: f64) -> Self {
        Metric { dim, deriv }
    }

    /// Returns the value of the metric at the given cell level.
    ///
    /// Equivalent to `deriv * 2^(-dim * level)`.
    pub fn value(&self, level: impl Into<Level>) -> f64 {
        // ldexp(deriv, -dim*level)
        let exp = -(i32::from(self.dim) * level.into().as_i32());
        self.deriv * f64::from_bits(((1023i64 + i64::from(exp)) as u64) << 52)
    }

    /// Returns the minimum level such that the metric is at most `val`,
    /// or `MAX_CELL_LEVEL` if there is no such level.
    ///
    /// For example, `MIN_WIDTH.min_level(0.1)` returns the minimum level
    /// such that all cell widths are 0.1 or smaller.
    ///
    /// In C++, this is `GetLevelForMaxValue`.
    pub fn min_level(&self, val: f64) -> Level {
        debug_assert!(!val.is_nan(), "min_level: value must not be NaN");
        if val <= 0.0 || val.is_nan() {
            return Level::MAX;
        }
        let level = -(ilogb(val / self.deriv) >> (i32::from(self.dim) - 1));
        let level = Level::new(level.clamp(0, i32::from(MAX_CELL_LEVEL)) as u8);
        debug_assert!(level == Level::MAX || self.value(level) <= val);
        debug_assert!(level == Level::MIN || self.value(level - 1) > val);
        level
    }

    /// Returns the maximum level such that the metric is at least `val`,
    /// or 0 if there is no such level.
    ///
    /// For example, `MIN_WIDTH.max_level(0.1)` returns the maximum level
    /// such that all cells have a minimum width of 0.1 or larger.
    ///
    /// In C++, this is `GetLevelForMinValue`.
    pub fn max_level(&self, val: f64) -> Level {
        debug_assert!(!val.is_nan(), "max_level: value must not be NaN");
        if val <= 0.0 || val.is_nan() {
            return Level::MAX;
        }
        let level = Level::new(
            (ilogb(self.deriv / val) >> (i32::from(self.dim) - 1))
                .clamp(0, i32::from(MAX_CELL_LEVEL)) as u8,
        );
        debug_assert!(level == Level::MIN || self.value(level) >= val);
        debug_assert!(level == Level::MAX || self.value(level + 1) < val);
        level
    }

    /// Returns the level at which the metric has approximately the given
    /// value. For example, `AVG_EDGE.closest_level(0.1)` returns the level
    /// at which the average cell edge length is approximately 0.1.
    pub fn closest_level(&self, val: f64) -> Level {
        let factor = if self.dim == 2 {
            2.0
        } else {
            std::f64::consts::SQRT_2
        };
        self.min_level(factor * val)
    }
}

// ─── Angle span metrics ─────────────────────────────────────────────────

/// Minimum angle span over all cells at any level (quadratic projection).
pub const MIN_ANGLE_SPAN: Metric = Metric::new(1, 4.0 / 3.0);

/// Average angle span over all cells at any level.
pub const AVG_ANGLE_SPAN: Metric = Metric::new(1, std::f64::consts::FRAC_PI_2);

/// Maximum angle span over all cells at any level (quadratic projection).
pub const MAX_ANGLE_SPAN: Metric = Metric::new(1, 1.704_897_179_199_218_5);

// ─── Width metrics ──────────────────────────────────────────────────────

/// Minimum width over all cells at any level (quadratic projection).
pub const MIN_WIDTH: Metric = Metric::new(1, 2.0 * std::f64::consts::SQRT_2 / 3.0);

/// Average width over all cells at any level (quadratic projection).
pub const AVG_WIDTH: Metric = Metric::new(1, 1.434_523_672_886_099_5);

/// Maximum width over all cells at any level. Equal to max angle span
/// for all projections.
pub const MAX_WIDTH: Metric = Metric::new(1, MAX_ANGLE_SPAN.deriv);

// ─── Edge metrics ───────────────────────────────────────────────────────

/// Minimum edge length over all cells at any level (quadratic projection).
pub const MIN_EDGE: Metric = Metric::new(1, 2.0 * std::f64::consts::SQRT_2 / 3.0);

/// Average edge length over all cells at any level (quadratic projection).
pub const AVG_EDGE: Metric = Metric::new(1, 1.459_213_746_386_106_1);

/// Maximum edge length over all cells at any level. Equal to max angle span
/// for all projections.
pub const MAX_EDGE: Metric = Metric::new(1, MAX_ANGLE_SPAN.deriv);

/// Maximum edge aspect ratio over all cells at any level (quadratic
/// projection). This is the ratio of the longest edge to the shortest edge.
pub const MAX_EDGE_ASPECT: f64 = 1.442_615_274_452_683;

// ─── Diagonal metrics ───────────────────────────────────────────────────

/// Minimum diagonal length over all cells at any level (quadratic projection).
pub const MIN_DIAG: Metric = Metric::new(1, 8.0 * std::f64::consts::SQRT_2 / 9.0);

/// Average diagonal length over all cells at any level (quadratic projection).
pub const AVG_DIAG: Metric = Metric::new(1, 2.060_422_738_998_471_7);

/// Maximum diagonal length over all cells at any level (quadratic projection).
pub const MAX_DIAG: Metric = Metric::new(1, 2.438_654_594_434_021);

/// Maximum diagonal aspect ratio over all cells at any level.
/// Equal to √3 for all projections.
pub const MAX_DIAG_ASPECT: f64 = 1.7320508075688772; // sqrt(3)

// ─── Area metrics ───────────────────────────────────────────────────────

/// Minimum area over all cells at any level (quadratic projection).
pub const MIN_AREA: Metric = Metric::new(2, 8.0 * std::f64::consts::SQRT_2 / 9.0);

/// Average area over all cells at any level. Equal to 4π/6 for all projections.
pub const AVG_AREA: Metric = Metric::new(2, 4.0 * std::f64::consts::PI / 6.0);

/// Maximum area over all cells at any level (quadratic projection).
pub const MAX_AREA: Metric = Metric::new(2, 2.635_799_256_963_161_4);

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[expect(
    clippy::assertions_on_constants,
    reason = "compile-time metric sanity checks"
)]
mod tests {
    use super::*;

    #[test]
    fn test_value_at_level_zero() {
        assert_eq!(MIN_ANGLE_SPAN.value(0), MIN_ANGLE_SPAN.deriv);
        assert_eq!(AVG_AREA.value(0), AVG_AREA.deriv);
    }

    #[test]
    fn test_value_halves_per_level_for_length() {
        let v0 = AVG_EDGE.value(0);
        let v1 = AVG_EDGE.value(1);
        assert!((v1 - v0 / 2.0).abs() < 1e-15);
    }

    #[test]
    fn test_value_quarters_per_level_for_area() {
        let v0 = AVG_AREA.value(0);
        let v1 = AVG_AREA.value(1);
        assert!((v1 - v0 / 4.0).abs() < 1e-15);
    }

    #[test]
    fn test_min_max_avg_ordering() {
        for level in 0..=MAX_CELL_LEVEL {
            assert!(
                MIN_ANGLE_SPAN.value(level) <= AVG_ANGLE_SPAN.value(level),
                "level {level}"
            );
            assert!(
                AVG_ANGLE_SPAN.value(level) <= MAX_ANGLE_SPAN.value(level),
                "level {level}"
            );

            assert!(
                MIN_WIDTH.value(level) <= AVG_WIDTH.value(level),
                "level {level}"
            );
            assert!(
                AVG_WIDTH.value(level) <= MAX_WIDTH.value(level),
                "level {level}"
            );

            assert!(
                MIN_EDGE.value(level) <= AVG_EDGE.value(level),
                "level {level}"
            );
            assert!(
                AVG_EDGE.value(level) <= MAX_EDGE.value(level),
                "level {level}"
            );

            assert!(
                MIN_DIAG.value(level) <= AVG_DIAG.value(level),
                "level {level}"
            );
            assert!(
                AVG_DIAG.value(level) <= MAX_DIAG.value(level),
                "level {level}"
            );

            assert!(
                MIN_AREA.value(level) <= AVG_AREA.value(level),
                "level {level}"
            );
            assert!(
                AVG_AREA.value(level) <= MAX_AREA.value(level),
                "level {level}"
            );
        }
    }

    #[test]
    fn test_metric_relationships() {
        for level in 0..=MAX_CELL_LEVEL {
            // Width <= angle span
            assert!(
                MIN_WIDTH.value(level) <= MIN_ANGLE_SPAN.value(level) + 1e-15,
                "level {level}"
            );
            // Edge <= diagonal
            assert!(
                MIN_EDGE.value(level) <= MIN_DIAG.value(level) + 1e-15,
                "level {level}"
            );
            assert!(
                MAX_EDGE.value(level) <= MAX_DIAG.value(level) + 1e-15,
                "level {level}"
            );
        }
    }

    #[test]
    fn test_min_level() {
        // For very large values, level should be 0.
        assert_eq!(MIN_WIDTH.min_level(1e10), 0);
        // For very small values, level should be MAX_CELL_LEVEL.
        assert_eq!(MIN_WIDTH.min_level(1e-30), MAX_CELL_LEVEL);
        // Non-positive values should return MAX_CELL_LEVEL.
        assert_eq!(MIN_WIDTH.min_level(-1.0), MAX_CELL_LEVEL);
        assert_eq!(MIN_WIDTH.min_level(0.0), MAX_CELL_LEVEL);
        assert_eq!(MAX_WIDTH.min_level(-1.0), MAX_CELL_LEVEL);
        assert_eq!(MAX_WIDTH.min_level(0.0), MAX_CELL_LEVEL);
    }

    #[test]
    fn test_max_level() {
        // For very large values, level should be 0.
        assert_eq!(MIN_WIDTH.max_level(1e10), 0);
        // For very small values, level should be MAX_CELL_LEVEL.
        assert_eq!(MIN_WIDTH.max_level(1e-30), MAX_CELL_LEVEL);
        // Zero and negative values should return MAX_CELL_LEVEL.
        assert_eq!(MIN_WIDTH.max_level(0.0), MAX_CELL_LEVEL);
        assert_eq!(MIN_WIDTH.max_level(-1.0), MAX_CELL_LEVEL);
        // Large values should return 0.
        assert_eq!(MIN_WIDTH.max_level(4.0), 0);
        assert_eq!(MAX_WIDTH.max_level(4.0), 0);
        assert_eq!(MIN_WIDTH.max_level(f64::INFINITY), 0);
        assert_eq!(MAX_WIDTH.max_level(f64::INFINITY), 0);
    }

    #[test]
    fn test_closest_level() {
        let level = AVG_EDGE.closest_level(0.1);
        // The closest level should give a value near 0.1.
        let v = AVG_EDGE.value(level);
        assert!(v > 0.05 && v < 0.3, "closest_level gave value {v}");
    }

    #[test]
    fn test_min_level_boundary() {
        // Test the boundary condition from C++ tests:
        // At level k, the metric value is deriv * 2^(-k).
        // min_level(val) should return the smallest k such that value(k) <= val.
        for level in 0..=MAX_CELL_LEVEL {
            let val = MIN_DIAG.value(level);
            assert_eq!(MIN_DIAG.min_level(val), level);
        }
    }

    #[test]
    fn test_max_level_boundary() {
        for level in 0..=MAX_CELL_LEVEL {
            let val = MIN_DIAG.value(level);
            assert_eq!(MIN_DIAG.max_level(val), level);
        }
    }

    #[test]
    fn test_aspect_ratio_bounds() {
        // C++: Check that the maximum aspect ratio of an individual cell is
        // consistent with the global minimums and maximums.
        assert!(MAX_EDGE_ASPECT >= 1.0);
        assert!(MAX_EDGE_ASPECT <= MAX_EDGE.deriv / MIN_EDGE.deriv);
        assert!(MAX_DIAG_ASPECT >= 1.0);
        assert!(MAX_DIAG_ASPECT <= MAX_DIAG.deriv / MIN_DIAG.deriv);
    }

    #[test]
    fn test_width_edge_diag_relationships() {
        // C++: width <= angle_span, width <= edge, edge <= diag
        // (checked at deriv level)
        assert!(MIN_WIDTH.deriv <= MIN_ANGLE_SPAN.deriv);
        assert!(MAX_WIDTH.deriv <= MAX_ANGLE_SPAN.deriv);
        assert!(AVG_WIDTH.deriv <= AVG_ANGLE_SPAN.deriv);

        assert!(MIN_WIDTH.deriv <= MIN_EDGE.deriv);
        assert!(MAX_WIDTH.deriv <= MAX_EDGE.deriv);
        assert!(AVG_WIDTH.deriv <= AVG_EDGE.deriv);

        assert!(MIN_EDGE.deriv <= MIN_DIAG.deriv);
        assert!(MAX_EDGE.deriv <= MAX_DIAG.deriv);
        assert!(AVG_EDGE.deriv <= AVG_DIAG.deriv);
    }

    #[test]
    fn test_area_bounds() {
        // C++: min_area >= min_width * min_edge
        assert!(MIN_AREA.deriv >= MIN_WIDTH.deriv * MIN_EDGE.deriv - 1e-15);
        assert!(MAX_AREA.deriv <= MAX_WIDTH.deriv * MAX_EDGE.deriv + 1e-15);
    }

    #[test]
    fn test_level_boundary_and_non_boundary() {
        // C++ Metrics test: check boundary cases (exactly equal to threshold)
        // and non-boundary cases for all levels.
        for level in -2..=(i32::from(MAX_CELL_LEVEL) + 3) {
            let mut width_val = MIN_WIDTH.deriv * 2.0_f64.powi(-level);
            if level >= i32::from(MAX_CELL_LEVEL) + 3 {
                width_val = 0.0;
            }

            let expected = (level.clamp(0, i32::from(MAX_CELL_LEVEL))) as u8;

            // Boundary cases.
            assert_eq!(
                MIN_WIDTH.min_level(width_val),
                expected,
                "min_level boundary at level {level}"
            );
            assert_eq!(
                MIN_WIDTH.max_level(width_val),
                expected,
                "max_level boundary at level {level}"
            );
            assert_eq!(
                MIN_WIDTH.closest_level(width_val),
                expected,
                "closest_level boundary at level {level}"
            );

            // Non-boundary cases.
            assert_eq!(
                MIN_WIDTH.min_level(1.2 * width_val),
                expected,
                "min_level non-boundary 1.2x at level {level}"
            );
            assert_eq!(
                MIN_WIDTH.max_level(0.8 * width_val),
                expected,
                "max_level non-boundary 0.8x at level {level}"
            );
            assert_eq!(
                MIN_WIDTH.closest_level(1.2 * width_val),
                expected,
                "closest_level non-boundary 1.2x at level {level}"
            );
            assert_eq!(
                MIN_WIDTH.closest_level(0.8 * width_val),
                expected,
                "closest_level non-boundary 0.8x at level {level}"
            );

            // Same for area metric.
            let mut area_val = MIN_AREA.deriv * 4.0_f64.powi(-level);
            if level <= -3 {
                area_val = 0.0;
            }
            assert_eq!(
                MIN_AREA.min_level(area_val),
                expected,
                "area min_level boundary at level {level}"
            );
            assert_eq!(
                MIN_AREA.max_level(area_val),
                expected,
                "area max_level boundary at level {level}"
            );
            assert_eq!(
                MIN_AREA.closest_level(area_val),
                expected,
                "area closest_level boundary at level {level}"
            );
            assert_eq!(
                MIN_AREA.min_level(1.2 * area_val),
                expected,
                "area min_level non-boundary 1.2x at level {level}"
            );
            assert_eq!(
                MIN_AREA.max_level(0.8 * area_val),
                expected,
                "area max_level non-boundary 0.8x at level {level}"
            );
            assert_eq!(
                MIN_AREA.closest_level(1.2 * area_val),
                expected,
                "area closest_level non-boundary 1.2x at level {level}"
            );
            assert_eq!(
                MIN_AREA.closest_level(0.8 * area_val),
                expected,
                "area closest_level non-boundary 0.8x at level {level}"
            );
        }
    }

    #[test]
    fn test_avg_area_consistency() {
        // Total sphere area is 4π, divided among 6 face cells.
        let face_area = AVG_AREA.value(0);
        let total = face_area * 6.0;
        assert!(
            (total - 4.0 * std::f64::consts::PI).abs() < 1e-14,
            "6 * avg_area(0) = {total}, expected 4π"
        );
    }
}
