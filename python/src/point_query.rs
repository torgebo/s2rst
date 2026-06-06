// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for `S2PointIndex` (points with attached Python data) and
//! `ClosestPointQuery` (k-nearest-neighbour search over those points).

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use s2rst::s2::Point;
use s2rst::s2::closest_point_query as cpq;
use s2rst::s2::point_index::S2PointIndex;

use crate::angle::PyChordAngle;
use crate::cells::PyCell;
use crate::index::PyShapeIndex;
use crate::s2point::PyS2Point;

/// The data payload stored at each indexed point: an arbitrary Python object
/// (or `None`). Equality is value-based (Python `==`); ordering is a no-op so
/// that query results are ranked purely by distance, matching core.
#[derive(Default)]
pub struct PyData(Option<Py<PyAny>>);

impl Clone for PyData {
    fn clone(&self) -> Self {
        // `Py` cloning bumps a refcount, which needs the GIL (held during any
        // pyo3 call that triggers a core `D: Clone`).
        PyData(
            self.0
                .as_ref()
                .map(|o| Python::with_gil(|py| o.clone_ref(py))),
        )
    }
}

impl PartialEq for PyData {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Python::with_gil(|py| a.bind(py).eq(b.bind(py)).unwrap_or(false)),
            (None, None) => true,
            _ => false,
        }
    }
}

impl PartialOrd for PyData {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Equal)
    }
}

/// A spatial index over points, each carrying an arbitrary Python data object.
#[pyclass(name = "S2PointIndex", module = "s2rst")]
pub struct PyS2PointIndex(pub(crate) S2PointIndex<PyData>);

#[pymethods]
impl PyS2PointIndex {
    #[new]
    fn new() -> Self {
        Self(S2PointIndex::new())
    }

    /// Add `point` with an optional associated data object.
    #[pyo3(signature = (point, data=None))]
    fn add(&mut self, point: &PyS2Point, data: Option<Py<PyAny>>) {
        self.0.add(point.0, PyData(data));
    }

    /// Remove the entry matching `point` and `data` (by value). Returns whether
    /// an entry was removed.
    #[pyo3(signature = (point, data=None))]
    fn remove(&mut self, point: &PyS2Point, data: Option<Py<PyAny>>) -> bool {
        self.0.remove(point.0, &PyData(data))
    }

    /// Remove all points.
    fn clear(&mut self) {
        self.0.clear();
    }

    /// The number of points in the index.
    fn num_points(&self) -> usize {
        self.0.num_points()
    }

    fn __len__(&self) -> usize {
        self.0.num_points()
    }

    fn __iter__(&self, py: Python<'_>) -> PyS2PointIndexIter {
        let mut cur = self.0.iter();
        cur.begin();
        let mut items = Vec::new();
        while !cur.done() {
            let data = cur.data().0.as_ref().map(|o| o.clone_ref(py));
            items.push((cur.point(), data));
            cur.next();
        }
        PyS2PointIndexIter { items, idx: 0 }
    }

    fn __repr__(&self) -> String {
        format!("S2PointIndex(points={})", self.0.num_points())
    }
}

/// Snapshot iterator over `(point, data)` pairs of an `S2PointIndex`.
#[pyclass]
pub struct PyS2PointIndexIter {
    items: Vec<(Point, Option<Py<PyAny>>)>,
    idx: usize,
}

#[pymethods]
impl PyS2PointIndexIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> Option<(PyS2Point, PyObject)> {
        if self.idx >= self.items.len() {
            return None;
        }
        let (point, data) = &self.items[self.idx];
        self.idx += 1;
        let obj = data
            .as_ref()
            .map(|o| o.clone_ref(py))
            .unwrap_or_else(|| py.None());
        Some((PyS2Point(*point), obj))
    }
}

/// One result of a closest-point query.
#[pyclass(frozen, name = "PointQueryResult", module = "s2rst")]
pub struct PyPointQueryResult {
    distance: s2rst::s1::ChordAngle,
    point: Point,
    data: Option<Py<PyAny>>,
}

impl PyPointQueryResult {
    fn from_result(r: cpq::Result<PyData>) -> Self {
        Self {
            distance: r.distance,
            point: r.point,
            data: r.data.0,
        }
    }
}

#[pymethods]
impl PyPointQueryResult {
    /// Distance from the target to this point.
    #[getter]
    fn distance(&self) -> PyChordAngle {
        PyChordAngle(self.distance)
    }

    /// The matched point.
    #[getter]
    fn point(&self) -> PyS2Point {
        PyS2Point(self.point)
    }

    /// The data object stored at the matched point (or `None`).
    #[getter]
    fn data(&self, py: Python<'_>) -> PyObject {
        self.data
            .as_ref()
            .map(|o| o.clone_ref(py))
            .unwrap_or_else(|| py.None())
    }

    /// Whether no point was found.
    fn is_empty(&self) -> bool {
        self.distance == s2rst::s1::ChordAngle::INFINITY
    }

    fn __repr__(&self) -> String {
        format!(
            "PointQueryResult(distance={})",
            self.distance.to_angle().radians()
        )
    }
}

/// Build the appropriate (mutable) target and pass it to `$build`.
macro_rules! dispatch_target_mut {
    ($obj:expr, $build:expr) => {{
        let obj: &Bound<'_, PyAny> = $obj;
        if let Ok(p) = obj.downcast::<PyS2Point>() {
            let mut t = cpq::PointTarget::new(p.borrow().0);
            Ok($build(&mut t))
        } else if let Ok((a, b)) = obj.extract::<(PyS2Point, PyS2Point)>() {
            let mut t = cpq::EdgeTarget::new(a.0, b.0);
            Ok($build(&mut t))
        } else if let Ok(c) = obj.downcast::<PyCell>() {
            let mut t = cpq::CellTarget::new(c.borrow().0);
            Ok($build(&mut t))
        } else if let Ok(ix) = obj.downcast::<PyShapeIndex>() {
            let ix_ref = ix.borrow();
            let mut t = cpq::ShapeIndexTarget::new(&ix_ref.0);
            Ok($build(&mut t))
        } else {
            Err(PyTypeError::new_err(
                "target must be an S2Point, an (S2Point, S2Point) edge tuple, \
                 a Cell, or a ShapeIndex",
            ))
        }
    }};
}

/// Finds the point(s) in an `S2PointIndex` closest to a target.
#[pyclass(name = "ClosestPointQuery", module = "s2rst")]
pub struct PyClosestPointQuery {
    index: Py<PyS2PointIndex>,
}

impl PyClosestPointQuery {
    fn options(
        max_results: i32,
        max_distance: Option<&PyChordAngle>,
        max_error: Option<&PyChordAngle>,
    ) -> cpq::Options {
        let mut opts = cpq::Options {
            max_results,
            ..Default::default()
        };
        if let Some(d) = max_distance {
            opts.max_distance = d.0;
        }
        if let Some(e) = max_error {
            opts.max_error = e.0;
        }
        opts
    }
}

#[pymethods]
impl PyClosestPointQuery {
    #[new]
    fn new(index: Py<PyS2PointIndex>) -> Self {
        Self { index }
    }

    /// The single closest point to `target`.
    fn find_closest_point(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
    ) -> PyResult<PyPointQueryResult> {
        let idx = self.index.borrow(py);
        let q = cpq::ClosestPointQuery::new(&idx.0, Self::options(1, None, None));
        dispatch_target_mut!(target, |t| PyPointQueryResult::from_result(
            q.find_closest_point(t)
        ))
    }

    /// Up to `max_results` closest points, ordered nearest-first.
    #[pyo3(signature = (target, *, max_results=1, max_distance=None, max_error=None))]
    fn find_closest_points(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        max_results: i32,
        max_distance: Option<PyChordAngle>,
        max_error: Option<PyChordAngle>,
    ) -> PyResult<Vec<PyPointQueryResult>> {
        let idx = self.index.borrow(py);
        let opts = Self::options(max_results, max_distance.as_ref(), max_error.as_ref());
        let q = cpq::ClosestPointQuery::new(&idx.0, opts);
        dispatch_target_mut!(target, |t| q
            .find_closest_points(t)
            .into_iter()
            .map(PyPointQueryResult::from_result)
            .collect())
    }

    /// The distance from `target` to the nearest point.
    fn get_distance(&self, py: Python<'_>, target: &Bound<'_, PyAny>) -> PyResult<PyChordAngle> {
        let idx = self.index.borrow(py);
        let q = cpq::ClosestPointQuery::new(&idx.0, Self::options(1, None, None));
        dispatch_target_mut!(target, |t| PyChordAngle(q.get_distance(t)))
    }

    /// Whether some point is closer than `limit` to `target`.
    fn is_distance_less(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        limit: &PyChordAngle,
    ) -> PyResult<bool> {
        let idx = self.index.borrow(py);
        let q = cpq::ClosestPointQuery::new(&idx.0, Self::options(1, None, None));
        dispatch_target_mut!(target, |t| q.is_distance_less(t, limit.0))
    }

    fn __repr__(&self) -> String {
        "ClosestPointQuery(...)".to_string()
    }
}
