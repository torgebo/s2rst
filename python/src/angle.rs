// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::s1;

use crate::hash_util::hash_f64s;

// ---------------------------------------------------------------------------
// Angle
// ---------------------------------------------------------------------------

/// A one-dimensional angle, stored internally as radians.
#[pyclass(name = "Angle", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyAngle(pub(crate) s1::Angle);

#[pymethods]
impl PyAngle {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    #[classattr]
    const ZERO: PyAngle = PyAngle(s1::Angle::ZERO);

    #[classattr]
    const INFINITY: PyAngle = PyAngle(s1::Angle::INFINITY);

    /// Create from radians.
    #[classmethod]
    fn from_radians(_cls: &Bound<'_, PyType>, radians: f64) -> Self {
        PyAngle(s1::Angle::from_radians(radians))
    }

    /// Create from degrees.
    #[classmethod]
    fn from_degrees(_cls: &Bound<'_, PyType>, degrees: f64) -> Self {
        PyAngle(s1::Angle::from_degrees(degrees))
    }

    /// Create from E5 representation (degrees * 10^5).
    #[classmethod]
    fn from_e5(_cls: &Bound<'_, PyType>, e5: i32) -> Self {
        PyAngle(s1::Angle::from_e5(e5))
    }

    /// Create from E6 representation (degrees * 10^6).
    #[classmethod]
    fn from_e6(_cls: &Bound<'_, PyType>, e6: i32) -> Self {
        PyAngle(s1::Angle::from_e6(e6))
    }

    /// Create from E7 representation (degrees * 10^7).
    #[classmethod]
    fn from_e7(_cls: &Bound<'_, PyType>, e7: i32) -> Self {
        PyAngle(s1::Angle::from_e7(e7))
    }

    /// The angle in radians.
    #[getter]
    fn radians(&self) -> f64 {
        self.0.radians()
    }

    /// The angle in degrees.
    #[getter]
    fn degrees(&self) -> f64 {
        self.0.degrees()
    }

    /// The angle in E5 representation.
    fn e5(&self) -> i32 {
        self.0.e5()
    }

    /// The angle in E6 representation.
    fn e6(&self) -> i32 {
        self.0.e6()
    }

    /// The angle in E7 representation.
    fn e7(&self) -> i32 {
        self.0.e7()
    }

    /// Absolute value of this angle.
    fn abs(&self) -> Self {
        PyAngle(self.0.abs())
    }

    /// Equivalent angle in (-pi, pi].
    fn normalized(&self) -> Self {
        PyAngle(self.0.normalized())
    }

    fn sin(&self) -> f64 {
        self.0.sin()
    }

    fn cos(&self) -> f64 {
        self.0.cos()
    }

    fn tan(&self) -> f64 {
        self.0.tan()
    }

    /// Whether this angle is infinite.
    fn is_infinite(&self) -> bool {
        self.0.is_infinite()
    }

    /// Whether two angles are approximately equal (within 1e-15 radians).
    fn approx_eq(&self, other: &PyAngle) -> bool {
        self.0.approx_eq(other.0)
    }

    // -- Python operators --

    fn __add__(&self, other: &PyAngle) -> Self {
        PyAngle(self.0 + other.0)
    }

    fn __sub__(&self, other: &PyAngle) -> Self {
        PyAngle(self.0 - other.0)
    }

    fn __mul__(&self, scalar: f64) -> Self {
        PyAngle(self.0 * scalar)
    }

    fn __rmul__(&self, scalar: f64) -> Self {
        PyAngle(scalar * self.0)
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            // Check Angle first — Angle has __float__ so extract::<f64> would succeed
            if let Ok(angle) = other.extract::<PyRef<'_, PyAngle>>() {
                Ok((self.0 / angle.0).into_pyobject(py)?.into_any().unbind())
            } else if let Ok(scalar) = other.extract::<f64>() {
                Ok(PyAngle(self.0 / scalar)
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            } else {
                Err(pyo3::exceptions::PyTypeError::new_err(
                    "unsupported operand type for /",
                ))
            }
        })
    }

    fn __neg__(&self) -> Self {
        PyAngle(-self.0)
    }

    fn __eq__(&self, other: &PyAngle) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.radians()])
    }

    fn __lt__(&self, other: &PyAngle) -> bool {
        self.0 < other.0
    }

    fn __le__(&self, other: &PyAngle) -> bool {
        self.0 <= other.0
    }

    fn __gt__(&self, other: &PyAngle) -> bool {
        self.0 > other.0
    }

    fn __ge__(&self, other: &PyAngle) -> bool {
        self.0 >= other.0
    }

    fn __repr__(&self) -> String {
        format!("Angle({:.7} deg)", self.0.degrees())
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __float__(&self) -> f64 {
        self.0.radians()
    }
}

// ---------------------------------------------------------------------------
// ChordAngle
// ---------------------------------------------------------------------------

/// An angle represented as the squared chord length on the unit sphere.
#[pyclass(name = "ChordAngle", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyChordAngle(pub(crate) s1::ChordAngle);

#[pymethods]
impl PyChordAngle {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    #[classattr]
    const ZERO: PyChordAngle = PyChordAngle(s1::ChordAngle::ZERO);

    #[classattr]
    const RIGHT: PyChordAngle = PyChordAngle(s1::ChordAngle::RIGHT);

    #[classattr]
    const STRAIGHT: PyChordAngle = PyChordAngle(s1::ChordAngle::STRAIGHT);

    #[classattr]
    const INFINITY: PyChordAngle = PyChordAngle(s1::ChordAngle::INFINITY);

    #[classattr]
    const NEGATIVE: PyChordAngle = PyChordAngle(s1::ChordAngle::NEGATIVE);

    /// Create from squared chord length.
    #[classmethod]
    fn from_length2(_cls: &Bound<'_, PyType>, length2: f64) -> Self {
        PyChordAngle(s1::ChordAngle::from_length2(length2))
    }

    /// Create from an Angle.
    #[classmethod]
    fn from_angle(_cls: &Bound<'_, PyType>, angle: &PyAngle) -> Self {
        PyChordAngle(s1::ChordAngle::from_angle(angle.0))
    }

    /// Create from radians.
    #[classmethod]
    fn from_radians(_cls: &Bound<'_, PyType>, radians: f64) -> Self {
        PyChordAngle(s1::ChordAngle::from_radians(radians))
    }

    /// Create from degrees.
    #[classmethod]
    fn from_degrees(_cls: &Bound<'_, PyType>, degrees: f64) -> Self {
        PyChordAngle(s1::ChordAngle::from_degrees(degrees))
    }

    /// The squared chord length.
    #[getter]
    fn length2(&self) -> f64 {
        self.0.length2()
    }

    /// Convert to an Angle.
    #[allow(clippy::wrong_self_convention)]
    fn to_angle(&self) -> PyAngle {
        PyAngle(self.0.to_angle())
    }

    /// The angle in radians.
    #[getter]
    fn radians(&self) -> f64 {
        self.0.radians()
    }

    /// The angle in degrees.
    #[getter]
    fn degrees(&self) -> f64 {
        self.0.degrees()
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    fn is_infinity(&self) -> bool {
        self.0.is_infinity()
    }

    fn is_special(&self) -> bool {
        self.0.is_special()
    }

    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    fn successor(&self) -> Self {
        PyChordAngle(self.0.successor())
    }

    fn predecessor(&self) -> Self {
        PyChordAngle(self.0.predecessor())
    }

    fn sin(&self) -> f64 {
        self.0.sin()
    }

    fn sin2(&self) -> f64 {
        self.0.sin2()
    }

    fn cos(&self) -> f64 {
        self.0.cos()
    }

    fn tan(&self) -> f64 {
        self.0.tan()
    }

    fn __add__(&self, other: &PyChordAngle) -> Self {
        PyChordAngle(self.0 + other.0)
    }

    fn __sub__(&self, other: &PyChordAngle) -> Self {
        PyChordAngle(self.0 - other.0)
    }

    fn __eq__(&self, other: &PyChordAngle) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.length2()])
    }

    fn __lt__(&self, other: &PyChordAngle) -> bool {
        self.0 < other.0
    }

    fn __le__(&self, other: &PyChordAngle) -> bool {
        self.0 <= other.0
    }

    fn __gt__(&self, other: &PyChordAngle) -> bool {
        self.0 > other.0
    }

    fn __ge__(&self, other: &PyChordAngle) -> bool {
        self.0 >= other.0
    }

    fn __repr__(&self) -> String {
        format!("ChordAngle({:.7} deg)", self.0.degrees())
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}
