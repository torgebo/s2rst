# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for ShapeIndex and its queries."""

import pytest

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _regular_loop(lat, lng, radius_deg=1.0, n=8):
    return s2rst.Loop.make_regular(
        _point(lat, lng), s2rst.Angle.from_degrees(radius_deg), n
    )


def test_empty_index():
    idx = s2rst.ShapeIndex()
    assert len(idx) == 0
    assert idx.is_empty()
    assert idx.num_edges() == 0


def test_add_build_counts():
    idx = s2rst.ShapeIndex()
    assert idx.add(_regular_loop(0, 0, 1.0, 8)) == 0
    idx.build()
    assert len(idx) == 1
    assert not idx.is_empty()
    assert idx.num_edges() == 8
    assert "shapes=1" in repr(idx)


def test_add_multiple_shape_types():
    idx = s2rst.ShapeIndex()
    ids = [
        idx.add(_regular_loop(0, 0)),
        idx.add(s2rst.Polyline([_point(0, 0), _point(0, 5)])),
        idx.add(s2rst.Polygon([_regular_loop(20, 20)])),
        idx.add(s2rst.LaxPolyline([_point(10, 0), _point(10, 5)])),
    ]
    assert ids == [0, 1, 2, 3]
    idx.build()
    assert len(idx) == 4


def test_add_rejects_non_shape():
    idx = s2rst.ShapeIndex()
    with pytest.raises(TypeError):
        idx.add(42)
    with pytest.raises(TypeError):
        idx.add(_point(0, 0))  # an S2Point is not a shape


def test_contains_point():
    center = _point(30, 40)
    idx = s2rst.ShapeIndex()
    idx.add(_regular_loop(30, 40, 2.0, 16))
    idx.build()
    assert idx.contains_point(center)
    assert not idx.contains_point(_point(30, 100))
    assert idx.containing_shape_ids(center) == [0]
    assert idx.containing_shape_ids(_point(30, 100)) == []


def test_distance_queries():
    idx = s2rst.ShapeIndex()
    idx.add(_regular_loop(0, 0, 1.0, 8))
    idx.build()
    near = idx.distance_to_point(_point(0, 3)).radians
    far = idx.distance_to_point(_point(0, 45)).radians
    assert 0 < near < far
    assert idx.is_distance_less_to_point(_point(0, 3), s2rst.ChordAngle.from_degrees(5))
    assert not idx.is_distance_less_to_point(
        _point(0, 45), s2rst.ChordAngle.from_degrees(5)
    )


def _indexed_loop(lat, lng, radius_deg=5.0, n=6):
    loop = _regular_loop(lat, lng, radius_deg, n)
    idx = s2rst.ShapeIndex()
    sid = idx.add(loop)
    idx.build()
    return loop, idx, sid


def test_contains_point_vertex_model():
    loop, idx, _ = _indexed_loop(0, 0)
    interior = _point(0, 0)
    # Interior points are contained under every model.
    for model in (
        s2rst.VertexModel.OPEN,
        s2rst.VertexModel.SEMI_OPEN,
        s2rst.VertexModel.CLOSED,
    ):
        assert idx.contains_point(interior, model=model)
    # A boundary vertex: closed contains it, open does not.
    v = loop.vertex(0)
    assert idx.contains_point(v, model=s2rst.VertexModel.CLOSED)
    assert not idx.contains_point(v, model=s2rst.VertexModel.OPEN)
    # The default is semi-open.
    assert idx.contains_point(v) == idx.contains_point(
        v, model=s2rst.VertexModel.SEMI_OPEN
    )


def test_containing_shape_ids_vertex_model():
    loop, idx, sid = _indexed_loop(20, 30)
    assert idx.containing_shape_ids(_point(20, 30)) == [sid]
    v = loop.vertex(0)
    assert idx.containing_shape_ids(v, model=s2rst.VertexModel.CLOSED) == [sid]
    assert idx.containing_shape_ids(v, model=s2rst.VertexModel.OPEN) == []


def test_vertex_model_enum():
    assert s2rst.VertexModel.OPEN != s2rst.VertexModel.CLOSED
    assert s2rst.VertexModel.SEMI_OPEN == s2rst.VertexModel.SEMI_OPEN
    d = {s2rst.VertexModel.OPEN: 1, s2rst.VertexModel.CLOSED: 2}
    assert d[s2rst.VertexModel.OPEN] == 1
