# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Verify Iterable / Sized / Container / Sequence ABC conformance.

- `__iter__` is structurally checked by `Iterable` / `Iterator`.
- `Sequence` has no structural hook; it requires `Sequence.register()`,
  done in s2rst/__init__.py.
- For-loop iteration must use the new `__iter__` (independent snapshot
  iterator), not the legacy `__getitem__` protocol.
"""

from collections.abc import Container, Iterable, Iterator, Sequence, Sized

import pytest

import s2rst


# Each entry: (factory, label, expected_element_type)
def _make_polyline():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1)]
    return s2rst.Polyline(pts)


def _make_loop():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1)]
    return s2rst.Loop(pts)


def _make_polygon():
    return s2rst.Polygon([_make_loop()])


def _make_lax_loop():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1)]
    return s2rst.LaxLoop(pts)


def _make_lax_polyline():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)]
    return s2rst.LaxPolyline(pts)


def _make_point_vector():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)]
    return s2rst.PointVector(pts)


def _make_edge_vector_shape():
    evs = s2rst.EdgeVectorShape()
    evs.add(s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0))
    evs.add(s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1))
    return evs


def _make_cell_union():
    return s2rst.CellUnion.from_cell_ids(
        [s2rst.CellId.from_face(0), s2rst.CellId.from_face(1)]
    )


# Sequence-typed collections that should satisfy Sequence + Iterable + Sized + Container.
_SEQUENCES = [
    (_make_polyline, "Polyline", s2rst.S2Point),
    (_make_loop, "Loop", s2rst.S2Point),
    (_make_polygon, "Polygon", s2rst.Loop),
    (_make_lax_loop, "LaxLoop", s2rst.S2Point),
    (_make_lax_polyline, "LaxPolyline", s2rst.S2Point),
    (_make_point_vector, "PointVector", s2rst.S2Point),
    (_make_edge_vector_shape, "EdgeVectorShape", s2rst.Edge),
    (_make_cell_union, "CellUnion", s2rst.CellId),
]


@pytest.mark.parametrize(
    ("factory", "label", "_elt"),
    _SEQUENCES,
    ids=[lbl for _, lbl, _ in _SEQUENCES],
)
def test_isinstance_sequence(factory, label, _elt):
    obj = factory()
    assert isinstance(obj, Sequence), f"{label} must be a Sequence"
    # Sequence implies Iterable, Sized, Container, Reversible per the ABC graph.
    assert isinstance(obj, Iterable), f"{label} must be Iterable"
    assert isinstance(obj, Sized), f"{label} must be Sized"


@pytest.mark.parametrize(
    ("factory", "label", "_elt"),
    _SEQUENCES,
    ids=[lbl for _, lbl, _ in _SEQUENCES],
)
def test_iter_returns_iterator(factory, label, _elt):
    obj = factory()
    it = iter(obj)
    assert isinstance(it, Iterator), f"{label}: iter(obj) must return Iterator"
    # Iterator must return self from __iter__
    assert iter(it) is it, f"{label}: iterator should be self-iterating"


@pytest.mark.parametrize(
    ("factory", "label", "elt"),
    _SEQUENCES,
    ids=[lbl for _, lbl, _ in _SEQUENCES],
)
def test_iter_visits_all_elements(factory, label, elt):
    obj = factory()
    n = len(obj)
    items = list(obj)
    assert len(items) == n, f"{label}: iter yielded {len(items)} items, expected {n}"
    for i, item in enumerate(items):
        assert isinstance(item, elt), f"{label}[{i}] is {type(item)}, not {elt}"


@pytest.mark.parametrize(
    ("factory", "label", "_elt"),
    _SEQUENCES,
    ids=[lbl for _, lbl, _ in _SEQUENCES],
)
def test_iter_matches_getitem(factory, label, _elt):
    obj = factory()
    via_iter = list(obj)
    via_getitem = [obj[i] for i in range(len(obj))]
    assert via_iter == via_getitem, (
        f"{label}: __iter__ and __getitem__ must yield equal sequences"
    )


@pytest.mark.parametrize(
    ("factory", "label", "_elt"),
    _SEQUENCES,
    ids=[lbl for _, lbl, _ in _SEQUENCES],
)
def test_iterator_exhausts(factory, label, _elt):
    obj = factory()
    it = iter(obj)
    for _ in range(len(obj)):
        next(it)
    with pytest.raises(StopIteration):
        next(it)


def test_edge_vector_shape_getitem_returns_edge():
    evs = _make_edge_vector_shape()
    assert len(evs) == 2
    e0 = evs[0]
    assert isinstance(e0, s2rst.Edge)
    assert e0.v0 == s2rst.S2Point(1, 0, 0)
    assert e0.v1 == s2rst.S2Point(0, 1, 0)
    with pytest.raises(IndexError):
        _ = evs[2]


def test_edge_vector_shape_iteration():
    evs = _make_edge_vector_shape()
    edges = list(evs)
    assert len(edges) == 2
    assert all(isinstance(e, s2rst.Edge) for e in edges)
    assert edges[0].v0 == s2rst.S2Point(1, 0, 0)
    assert edges[1].v0 == s2rst.S2Point(0, 1, 0)


def test_iterator_independent_of_parent():
    # Snapshot semantics: building an iterator and then dropping the parent
    # should still let the iterator complete.
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)]
    poly = s2rst.Polyline(pts)
    it = iter(poly)
    del poly
    assert next(it) == s2rst.S2Point(1, 0, 0)
    assert next(it) == s2rst.S2Point(0, 1, 0)
    with pytest.raises(StopIteration):
        next(it)


def test_polygon_iter_yields_loops():
    poly = _make_polygon()
    loops = list(poly)
    assert len(loops) == 1
    assert isinstance(loops[0], s2rst.Loop)
    assert len(loops[0]) == 3


def test_container_membership_for_cellunion():
    # CellUnion has __contains__ so it satisfies Container even without Sequence.
    cu = _make_cell_union()
    assert isinstance(cu, Container)
    assert s2rst.CellId.from_face(0) in cu
    assert s2rst.CellId.from_face(5) not in cu
