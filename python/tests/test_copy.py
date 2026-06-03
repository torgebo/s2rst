# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tier B2: __copy__ / __deepcopy__ for every exposed value type.

Before this, `copy.copy(x)` / `copy.deepcopy(x)` fell back to the pickle
protocol and raised (no class implements __reduce__). Each type now defines
both, returning an independent object.

For types that also define __eq__ we assert value-equality with the original;
the handful of shape types without __eq__ (Shape, Lax*, PointVector,
EdgeVectorShape) are only checked for type + distinct identity.
"""

import copy

import pytest

import s2rst


def _triangle():
    return [
        s2rst.S2Point(1, 0, 0),
        s2rst.S2Point(0, 1, 0),
        s2rst.S2Point(0, 0, 1),
    ]


def _samples():
    pts = _triangle()
    return [
        s2rst.Angle.from_degrees(30.0),
        s2rst.ChordAngle.from_degrees(30.0),
        s2rst.R1Interval(1.0, 2.0),
        s2rst.S1Interval(0.5, 1.5),
        s2rst.R2Point(1.0, 2.0),
        s2rst.Vector(1.0, 2.0, 3.0),
        s2rst.Matrix3x3.identity(),
        s2rst.R2Rect(s2rst.R1Interval(0.0, 1.0), s2rst.R1Interval(0.0, 2.0)),
        s2rst.S2Point(1, 0, 0),
        s2rst.LatLng.from_degrees(10.0, 20.0),
        s2rst.CellId.from_face(0),
        s2rst.Cell(s2rst.CellId.from_face(0)),
        s2rst.CellUnion.from_cell_ids([s2rst.CellId.from_face(0)]),
        s2rst.Cap.from_point(pts[0]),
        s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(10.0, 20.0)),
        s2rst.Polyline(pts),
        s2rst.Loop(pts),
        s2rst.Polygon([s2rst.Loop(pts)]),
        s2rst.Edge(pts[0], pts[1]),
        s2rst.LaxLoop(pts).as_shape().reference_point(),
        s2rst.LaxLoop(pts),
        s2rst.LaxPolyline(pts),
        s2rst.LaxPolygon([pts]),
        s2rst.PointVector(pts),
        s2rst.EdgeVectorShape.from_edge(pts[0], pts[1]),
        s2rst.LaxLoop(pts).as_shape(),
    ]


def _has_eq(obj):
    return type(obj).__eq__ is not object.__eq__


@pytest.mark.parametrize("obj", _samples(), ids=lambda o: type(o).__name__)
def test_copy(obj):
    c = copy.copy(obj)
    assert type(c) is type(obj)
    assert c is not obj
    if _has_eq(obj):
        assert c == obj


@pytest.mark.parametrize("obj", _samples(), ids=lambda o: type(o).__name__)
def test_deepcopy(obj):
    d = copy.deepcopy(obj)
    assert type(d) is type(obj)
    assert d is not obj
    if _has_eq(obj):
        assert d == obj


def test_edge_vector_shape_copy_is_independent():
    a = s2rst.EdgeVectorShape.from_edge(s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0))
    b = copy.copy(a)
    assert len(a) == 1 and len(b) == 1
    b.add(s2rst.S2Point(0, 0, 1), s2rst.S2Point(1, 0, 0))
    assert len(b) == 2
    assert len(a) == 1  # the original must not see the copy's mutation


def test_edge_vector_shape_deepcopy_is_independent():
    a = s2rst.EdgeVectorShape.from_edge(s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0))
    b = copy.deepcopy(a)
    b.add(s2rst.S2Point(0, 0, 1), s2rst.S2Point(1, 0, 0))
    assert len(b) == 2
    assert len(a) == 1
