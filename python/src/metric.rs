// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python binding for the S2 cell-size [`Metric`] and its associated
//! constants — describing how lengths and areas scale with cell level.

use pyo3::prelude::*;

use s2rst::s2::metric;

/// A measure for cells that scales exponentially with subdivision level.
///
/// `dim` is 1 for length metrics and 2 for area metrics. `value(level)`
/// equals `deriv * 2**(-dim * level)`. The named class attributes
/// (`MIN_WIDTH`, `AVG_AREA`, ...) are the standard S2 cell metrics for the
/// quadratic projection.
#[pyclass(name = "Metric", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyMetric(pub(crate) metric::Metric);

#[pymethods]
impl PyMetric {
    // --- Angle span metrics ---

    /// Minimum angle span over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MIN_ANGLE_SPAN() -> PyMetric {
        PyMetric(metric::MIN_ANGLE_SPAN)
    }

    /// Average angle span over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn AVG_ANGLE_SPAN() -> PyMetric {
        PyMetric(metric::AVG_ANGLE_SPAN)
    }

    /// Maximum angle span over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MAX_ANGLE_SPAN() -> PyMetric {
        PyMetric(metric::MAX_ANGLE_SPAN)
    }

    // --- Width metrics ---

    /// Minimum width over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MIN_WIDTH() -> PyMetric {
        PyMetric(metric::MIN_WIDTH)
    }

    /// Average width over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn AVG_WIDTH() -> PyMetric {
        PyMetric(metric::AVG_WIDTH)
    }

    /// Maximum width over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MAX_WIDTH() -> PyMetric {
        PyMetric(metric::MAX_WIDTH)
    }

    // --- Edge metrics ---

    /// Minimum edge length over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MIN_EDGE() -> PyMetric {
        PyMetric(metric::MIN_EDGE)
    }

    /// Average edge length over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn AVG_EDGE() -> PyMetric {
        PyMetric(metric::AVG_EDGE)
    }

    /// Maximum edge length over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MAX_EDGE() -> PyMetric {
        PyMetric(metric::MAX_EDGE)
    }

    /// Maximum edge aspect ratio (longest edge / shortest edge) over all cells.
    #[classattr]
    const MAX_EDGE_ASPECT: f64 = metric::MAX_EDGE_ASPECT;

    // --- Diagonal metrics ---

    /// Minimum diagonal length over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MIN_DIAG() -> PyMetric {
        PyMetric(metric::MIN_DIAG)
    }

    /// Average diagonal length over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn AVG_DIAG() -> PyMetric {
        PyMetric(metric::AVG_DIAG)
    }

    /// Maximum diagonal length over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MAX_DIAG() -> PyMetric {
        PyMetric(metric::MAX_DIAG)
    }

    /// Maximum diagonal aspect ratio over all cells at any level.
    #[classattr]
    const MAX_DIAG_ASPECT: f64 = metric::MAX_DIAG_ASPECT;

    // --- Area metrics ---

    /// Minimum area over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MIN_AREA() -> PyMetric {
        PyMetric(metric::MIN_AREA)
    }

    /// Average area over all cells at any level (equal to 4*pi/6).
    #[classattr]
    #[allow(non_snake_case)]
    fn AVG_AREA() -> PyMetric {
        PyMetric(metric::AVG_AREA)
    }

    /// Maximum area over all cells at any level.
    #[classattr]
    #[allow(non_snake_case)]
    fn MAX_AREA() -> PyMetric {
        PyMetric(metric::MAX_AREA)
    }

    // --- Accessors ---

    /// The metric dimension: 1 for a length metric, 2 for an area metric.
    #[getter]
    fn dim(&self) -> u8 {
        self.0.dim
    }

    /// The scaling factor (the metric's value at level 0).
    #[getter]
    fn deriv(&self) -> f64 {
        self.0.deriv
    }

    // --- Methods ---

    /// The value of the metric at the given cell level (0-30).
    fn value(&self, level: u8) -> f64 {
        self.0.value(level)
    }

    /// The minimum level such that the metric is at most `value`
    /// (the finest level whose cells all satisfy the bound).
    fn min_level(&self, value: f64) -> u8 {
        u8::from(self.0.min_level(value))
    }

    /// The maximum level such that the metric is at least `value`
    /// (the coarsest level whose cells all satisfy the bound).
    fn max_level(&self, value: f64) -> u8 {
        u8::from(self.0.max_level(value))
    }

    /// The level at which the metric most closely matches `value`.
    fn closest_level(&self, value: f64) -> u8 {
        u8::from(self.0.closest_level(value))
    }

    fn __repr__(&self) -> String {
        format!("Metric(dim={}, deriv={})", self.0.dim, self.0.deriv)
    }
}
