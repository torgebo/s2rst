// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python bindings for richer S2 encoding: compact vector codecs for
//! `S2Point`/`CellId` collections, the de-duplicating `SequenceLexicon` and
//! `ValueLexicon`, and a read-only `EncodedS2ShapeIndex` loaded from bytes.
//!
//! Like `encoding.rs`, this adapts core's reader/writer-based `Encode`/`Decode`
//! to Python `bytes`: encoding collects into a `Vec<u8>`, decoding reads from a
//! `std::io::Cursor` over the input slice.

use std::io::{self, Cursor};

use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyType;

use s2rst::s2::CellId;
use s2rst::s2::encoded_s2cell_id_vector;
use s2rst::s2::encoded_s2point_vector::{self, CodingHint};
use s2rst::s2::encoded_s2shape_index::EncodedS2ShapeIndex;
use s2rst::s2::sequence_lexicon::SequenceLexicon;
use s2rst::s2::value_lexicon::ValueLexicon;

use crate::cells::PyCellId;
use crate::s2point::PyS2Point;

fn io_err(e: &io::Error) -> PyErr {
    PyValueError::new_err(format!("s2 codec error: {e}"))
}

// ---------------------------------------------------------------------------
// Vector codecs (module-level functions)
// ---------------------------------------------------------------------------

/// Encode a list of `S2Point`s using the compact `CELL_IDS` format when
/// beneficial. Read it back with `decode_s2point_vector`.
#[pyfunction]
pub fn encode_s2point_vector(points: Vec<PyS2Point>) -> PyResult<Vec<u8>> {
    let raw: Vec<_> = points.into_iter().map(|p| p.0).collect();
    let mut buf = Vec::new();
    encoded_s2point_vector::encode_s2point_vector(&raw, CodingHint::Compact, &mut buf)
        .map_err(|e| io_err(&e))?;
    Ok(buf)
}

/// Decode a list of `S2Point`s encoded by `encode_s2point_vector`.
#[pyfunction]
pub fn decode_s2point_vector(data: &[u8]) -> PyResult<Vec<PyS2Point>> {
    let mut cursor = Cursor::new(data);
    let points =
        encoded_s2point_vector::decode_s2point_vector(&mut cursor).map_err(|e| io_err(&e))?;
    Ok(points.into_iter().map(PyS2Point).collect())
}

/// Encode a list of `CellId`s compactly (delta-encoded with a shared base).
/// Read it back with `decode_s2cell_id_vector`.
#[pyfunction]
pub fn encode_s2cell_id_vector(ids: Vec<PyCellId>) -> PyResult<Vec<u8>> {
    let raw: Vec<_> = ids.into_iter().map(|c| c.0).collect();
    let mut buf = Vec::new();
    encoded_s2cell_id_vector::encode_s2cell_id_vector(&raw, &mut buf).map_err(|e| io_err(&e))?;
    Ok(buf)
}

/// Decode a list of `CellId`s encoded by `encode_s2cell_id_vector`.
#[pyfunction]
pub fn decode_s2cell_id_vector(data: &[u8]) -> PyResult<Vec<PyCellId>> {
    let mut cursor = Cursor::new(data);
    let ids =
        encoded_s2cell_id_vector::decode_s2cell_id_vector(&mut cursor).map_err(|e| io_err(&e))?;
    Ok(ids.into_iter().map(PyCellId).collect())
}

// ---------------------------------------------------------------------------
// SequenceLexicon
// ---------------------------------------------------------------------------

/// Maps distinct sequences of `CellId`s to sequential integer IDs, eliminating
/// duplicates. Sequence-shaped: `len(lex)` is the number of distinct sequences
/// and `lex[i]` is the i-th sequence.
#[pyclass(name = "SequenceLexicon", module = "s2rst")]
pub struct PySequenceLexicon(SequenceLexicon<CellId>);

#[pymethods]
impl PySequenceLexicon {
    /// Create an empty lexicon.
    #[new]
    fn new() -> Self {
        PySequenceLexicon(SequenceLexicon::new())
    }

    /// Add a sequence, returning its ID. Re-adding an equal sequence returns
    /// the same ID. IDs are assigned sequentially from 0.
    fn add(&mut self, seq: Vec<PyCellId>) -> usize {
        let raw: Vec<CellId> = seq.into_iter().map(|c| c.0).collect();
        self.0.add(&raw) as usize
    }

    /// The sequence with the given ID.
    fn sequence(&self, id: usize) -> PyResult<Vec<PyCellId>> {
        if id >= self.0.size() as usize {
            return Err(PyIndexError::new_err("sequence id out of range"));
        }
        Ok(self.0.sequence(id as u32).map(|c| PyCellId(*c)).collect())
    }

    /// The number of distinct sequences.
    fn size(&self) -> usize {
        self.0.size() as usize
    }

    fn __len__(&self) -> usize {
        self.0.size() as usize
    }

    fn __getitem__(&self, i: isize) -> PyResult<Vec<PyCellId>> {
        let n = self.0.size() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            return Err(PyIndexError::new_err("index out of range"));
        }
        Ok(self.0.sequence(idx as u32).map(|c| PyCellId(*c)).collect())
    }

    fn __repr__(&self) -> String {
        format!("SequenceLexicon({} sequences)", self.0.size())
    }
}

// ---------------------------------------------------------------------------
// ValueLexicon
// ---------------------------------------------------------------------------

/// Maps distinct `CellId` values to sequential integer IDs, eliminating
/// duplicates. Sequence-shaped: `len(lex)` is the number of distinct values
/// and `lex[i]` is the i-th value.
#[pyclass(name = "ValueLexicon", module = "s2rst")]
pub struct PyValueLexicon(ValueLexicon<CellId>);

#[pymethods]
impl PyValueLexicon {
    /// Create an empty lexicon.
    #[new]
    fn new() -> Self {
        PyValueLexicon(ValueLexicon::new())
    }

    /// Add a value, returning its ID. Re-adding an equal value returns the same
    /// ID. IDs are assigned sequentially from 0.
    fn add(&mut self, v: PyCellId) -> usize {
        self.0.add(v.0) as usize
    }

    /// The value with the given ID.
    fn value(&self, id: usize) -> PyResult<PyCellId> {
        if id >= self.0.size() as usize {
            return Err(PyIndexError::new_err("value id out of range"));
        }
        Ok(PyCellId(*self.0.value(id as u32)))
    }

    /// The number of distinct values.
    fn size(&self) -> usize {
        self.0.size() as usize
    }

    fn __len__(&self) -> usize {
        self.0.size() as usize
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyCellId> {
        let n = self.0.size() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            return Err(PyIndexError::new_err("index out of range"));
        }
        Ok(PyCellId(*self.0.value(idx as u32)))
    }

    fn __repr__(&self) -> String {
        format!("ValueLexicon({} values)", self.0.size())
    }
}

// ---------------------------------------------------------------------------
// EncodedS2ShapeIndex
// ---------------------------------------------------------------------------

/// A read-only S2 shape index loaded from encoded bytes (as produced by
/// encoding a `ShapeIndex`). Exposes shape/edge counts; `len(idx)` is the
/// number of shape ids.
#[pyclass(name = "EncodedS2ShapeIndex", module = "s2rst")]
pub struct PyEncodedS2ShapeIndex(EncodedS2ShapeIndex);

#[pymethods]
impl PyEncodedS2ShapeIndex {
    /// Load an index from encoded bytes. Raises `ValueError` if the data is
    /// malformed or uses an unsupported version.
    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, PyType>, data: &[u8]) -> PyResult<Self> {
        let mut index = EncodedS2ShapeIndex::new();
        index.init(data).map_err(|e| io_err(&e))?;
        Ok(PyEncodedS2ShapeIndex(index))
    }

    /// The number of distinct shape ids in the index.
    fn num_shape_ids(&self) -> usize {
        self.0.num_shape_ids()
    }

    /// The total number of edges across all shapes.
    fn num_edges(&self) -> usize {
        self.0.num_edges()
    }

    fn __len__(&self) -> usize {
        self.0.num_shape_ids()
    }

    fn __repr__(&self) -> String {
        format!(
            "EncodedS2ShapeIndex({} shapes, {} edges)",
            self.0.num_shape_ids(),
            self.0.num_edges()
        )
    }
}
