// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! The Python side of the core's bounded types: one conversion per kind (spec 009, FR-007).
//!
//! A method that takes a cell level declares its parameter as [`LevelArg`], one that takes a
//! cube face as [`FaceArg`], one that takes a level modulus as [`LevelModArg`], and one that
//! takes a cell vertex or edge index as [`CellVertexArg`] or [`CellEdgeArg`] (the stub keeps
//! `int`). Extraction reads any Python integer through
//! `__index__`, so `bool` and NumPy integers are accepted as Python's own indexing accepts
//! them, and a `float` or `str` keeps PyO3's `TypeError`. A value outside the kind's range, including one beyond 64 bits, raises
//! `ValueError` with the core's `OutOfRange` text, for example
//! "level must be in 0..=30, got 31". No out-of-range value reaches the core, so none can
//! panic there, and none is clamped.

use pyo3::exceptions::{PyIndexError, PyOverflowError, PyValueError};
use pyo3::prelude::*;
use pyo3::{Borrowed, PyErr};

use s2rst::s2::{BoundedKind, CellEdge, CellVertex, Face, Level, LevelMod, OutOfRange};

/// Converts a Python integer to the bounded type `T` of `kind`, raising `to_err` of the core's
/// error outside the range. A Python integer beyond 64 bits gets the same message, with the
/// value as Python prints it.
fn bounded<T>(
    obj: Borrowed<'_, '_, PyAny>,
    kind: BoundedKind,
    to_err: fn(OutOfRange) -> PyErr,
) -> PyResult<T>
where
    T: TryFrom<i64, Error = OutOfRange>,
{
    let value = match obj.extract::<i64>() {
        Ok(v) => v,
        Err(e) if e.is_instance_of::<PyOverflowError>(obj.py()) => {
            let range = kind.range();
            let message = format!(
                "{} must be in {}..={}, got {}",
                kind.name(),
                range.start(),
                range.end(),
                obj.str()?
            );
            return Err(match kind {
                BoundedKind::CellVertex | BoundedKind::CellEdge => PyIndexError::new_err(message),
                _ => PyValueError::new_err(message),
            });
        }
        Err(e) => return Err(e),
    };
    T::try_from(value).map_err(to_err)
}

/// The core's error as a Python `ValueError`.
fn value_error(e: OutOfRange) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// The core's error as a Python `IndexError`, for the index kinds.
fn index_error(e: OutOfRange) -> PyErr {
    PyIndexError::new_err(e.to_string())
}

/// A cell level argument: a Python integer in `0..=30`, else `ValueError`.
pub(crate) struct LevelArg(pub Level);

impl FromPyObject<'_, '_> for LevelArg {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        bounded(obj, BoundedKind::Level, value_error).map(LevelArg)
    }
}

/// A cube face argument: a Python integer in `0..=5`, else `ValueError`.
pub(crate) struct FaceArg(pub Face);

impl FromPyObject<'_, '_> for FaceArg {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        bounded(obj, BoundedKind::Face, value_error).map(FaceArg)
    }
}

/// A level modulus argument: a Python integer in `1..=3`, else `ValueError`.
pub(crate) struct LevelModArg(pub LevelMod);

impl FromPyObject<'_, '_> for LevelModArg {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        bounded(obj, BoundedKind::LevelMod, value_error).map(LevelModArg)
    }
}

/// Checks the one relation between two levels that their types cannot carry: `ValueError`
/// unless `min_level <= max_level`. The core checks it too, with a panic, when the levels are
/// used (spec 009, B12), so a binding checks it where the user gives them.
pub(crate) fn check_level_order(min_level: Level, max_level: Level) -> PyResult<()> {
    if min_level <= max_level {
        Ok(())
    } else {
        Err(PyValueError::new_err(format!(
            "min_level ({min_level}) must not exceed max_level ({max_level})"
        )))
    }
}

/// A cell vertex index argument: a Python integer in `0..=3`, else `IndexError`.
pub(crate) struct CellVertexArg(pub CellVertex);

impl FromPyObject<'_, '_> for CellVertexArg {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        bounded(obj, BoundedKind::CellVertex, index_error).map(CellVertexArg)
    }
}

/// A cell edge index argument: a Python integer in `0..=3`, else `IndexError`.
pub(crate) struct CellEdgeArg(pub CellEdge);

impl FromPyObject<'_, '_> for CellEdgeArg {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        bounded(obj, BoundedKind::CellEdge, index_error).map(CellEdgeArg)
    }
}
