# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for CrossingEdgeQuery and shape_util free functions."""

import pytest

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _polyline_index():
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.LaxPolyline([_point(0, 0), _point(0, 10)]))
    idx.build()
    return idx


def test_crossings_hit_and_miss():
    q = s2rst.CrossingEdgeQuery(_polyline_index())
    assert q.crossings(_point(-5, 5), _point(5, 5)) == {0: [0]}
    assert q.crossings(_point(-5, 50), _point(5, 50)) == {}


def test_crossing_type_at_endpoint():
    q = s2rst.CrossingEdgeQuery(_polyline_index())
    # An edge that only touches the shared vertex (0,0).
    a, b = _point(0, 0), _point(-5, -5)
    assert q.crossings(a, b, cross_type=s2rst.CrossingType.INTERIOR) == {}
    assert q.crossings(a, b, cross_type=s2rst.CrossingType.ALL) != {}


def test_crossing_type_enum():
    assert s2rst.CrossingType.INTERIOR != s2rst.CrossingType.ALL
    assert len({s2rst.CrossingType.INTERIOR, s2rst.CrossingType.ALL}) == 2


def test_shape_to_points():
    pv = s2rst.PointVector([_point(0, 0), _point(1, 1), _point(2, 2)])
    pts = s2rst.shape_to_points(pv.as_shape())
    assert len(pts) == 3
    assert pts[0].approx_eq(_point(0, 0))


def test_shape_to_points_requires_point_shape():
    loop = s2rst.LaxLoop([_point(0, 0), _point(0, 1), _point(1, 0)])
    with pytest.raises(ValueError):
        s2rst.shape_to_points(loop.as_shape())


def test_find_self_intersection_valid():
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.Loop.make_regular(_point(0, 0), s2rst.Angle.from_degrees(5), 6))
    idx.build()
    assert s2rst.find_self_intersection(idx) is None


def test_find_self_intersection_bowtie():
    # A self-crossing "bow-tie" loop is invalid.
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.LaxLoop([_point(0, 0), _point(2, 2), _point(0, 2), _point(2, 0)]))
    idx.build()
    assert s2rst.find_self_intersection(idx) is not None


def test_visit_crossing_edge_pairs():
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.LaxLoop([_point(0, 0), _point(2, 2), _point(0, 2), _point(2, 0)]))
    idx.build()
    pairs = s2rst.visit_crossing_edge_pairs(idx)
    assert len(pairs) >= 1
    # Each pair is (shape_a, edge_a, shape_b, edge_b).
    for p in pairs:
        assert len(p) == 4
