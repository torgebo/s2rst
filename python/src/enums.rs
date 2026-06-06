// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Native Python enums (pyo3 `eq, eq_int`) shared across the bindings.
//!
//! Each enum is a thin, typed mirror of a core enum, exposed with
//! SCREAMING_SNAKE_CASE members (the Python convention) and a `to_core`
//! converter. Being native enums they are `==`-comparable, hashable, and
//! `mypy`-checkable.

use pyo3::prelude::*;

use s2rst::s2::boolean_operation::{OpType, PolygonModel, PolylineModel};
use s2rst::s2::contains_point_query::VertexModel;
use s2rst::s2::crossing_edge_query::CrossingType;

/// Boundary semantics for point-in-polygon containment tests.
///
/// - `OPEN`: boundary points are *not* contained.
/// - `SEMI_OPEN`: exactly one of each pair of adjacent cells owns a shared
///   boundary point (so a partition of the sphere has no double-counting).
/// - `CLOSED`: boundary points *are* contained.
#[pyclass(eq, eq_int, hash, frozen, name = "VertexModel", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyVertexModel {
    #[pyo3(name = "OPEN")]
    Open,
    #[pyo3(name = "SEMI_OPEN")]
    SemiOpen,
    #[pyo3(name = "CLOSED")]
    Closed,
}

impl PyVertexModel {
    pub(crate) fn to_core(self) -> VertexModel {
        match self {
            PyVertexModel::Open => VertexModel::Open,
            PyVertexModel::SemiOpen => VertexModel::SemiOpen,
            PyVertexModel::Closed => VertexModel::Closed,
        }
    }
}

/// The set operation performed by a boolean operation.
#[pyclass(eq, eq_int, hash, frozen, name = "OpType", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyOpType {
    #[pyo3(name = "UNION")]
    Union,
    #[pyo3(name = "INTERSECTION")]
    Intersection,
    #[pyo3(name = "DIFFERENCE")]
    Difference,
    #[pyo3(name = "SYMMETRIC_DIFFERENCE")]
    SymmetricDifference,
}

impl PyOpType {
    pub(crate) fn to_core(self) -> OpType {
        match self {
            PyOpType::Union => OpType::Union,
            PyOpType::Intersection => OpType::Intersection,
            PyOpType::Difference => OpType::Difference,
            PyOpType::SymmetricDifference => OpType::SymmetricDifference,
        }
    }
}

/// Whether polygon boundaries are treated as part of the interior.
#[pyclass(eq, eq_int, hash, frozen, name = "PolygonModel", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyPolygonModel {
    #[pyo3(name = "OPEN")]
    Open,
    #[pyo3(name = "SEMI_OPEN")]
    SemiOpen,
    #[pyo3(name = "CLOSED")]
    Closed,
}

impl PyPolygonModel {
    pub(crate) fn to_core(self) -> PolygonModel {
        match self {
            PyPolygonModel::Open => PolygonModel::Open,
            PyPolygonModel::SemiOpen => PolygonModel::SemiOpen,
            PyPolygonModel::Closed => PolygonModel::Closed,
        }
    }
}

/// Whether polyline endpoints are treated as part of the polyline.
#[pyclass(eq, eq_int, hash, frozen, name = "PolylineModel", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyPolylineModel {
    #[pyo3(name = "OPEN")]
    Open,
    #[pyo3(name = "SEMI_OPEN")]
    SemiOpen,
    #[pyo3(name = "CLOSED")]
    Closed,
}

impl PyPolylineModel {
    pub(crate) fn to_core(self) -> PolylineModel {
        match self {
            PyPolylineModel::Open => PolylineModel::Open,
            PyPolylineModel::SemiOpen => PolylineModel::SemiOpen,
            PyPolylineModel::Closed => PolylineModel::Closed,
        }
    }
}

/// Which edge crossings a `CrossingEdgeQuery` reports.
#[pyclass(eq, eq_int, hash, frozen, name = "CrossingType", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyCrossingType {
    #[pyo3(name = "INTERIOR")]
    Interior,
    #[pyo3(name = "ALL")]
    All,
}

impl PyCrossingType {
    pub(crate) fn to_core(self) -> CrossingType {
        match self {
            PyCrossingType::Interior => CrossingType::Interior,
            PyCrossingType::All => CrossingType::All,
        }
    }
}
