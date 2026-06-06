# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for ClosestEdgeQuery / FurthestEdgeQuery and EdgeQueryResult."""

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _square_index(lat=0.0, lng=0.0, radius=5.0, n=8):
    idx = s2rst.ShapeIndex()
    idx.add(
        s2rst.Loop.make_regular(_point(lat, lng), s2rst.Angle.from_degrees(radius), n)
    )
    idx.build()
    return idx


def test_closest_edge_interior():
    idx = _square_index()
    r = s2rst.ClosestEdgeQuery(idx).find_closest_edge(_point(0, 0))
    assert r.is_interior()
    assert not r.is_empty()
    assert r.distance.radians == pytest.approx(0.0)
    assert r.edge_id == -1


def test_closest_edge_exterior():
    idx = _square_index()
    q = s2rst.ClosestEdgeQuery(idx)
    r = q.find_closest_edge(_point(0, 20))
    assert not r.is_interior()
    assert r.shape_id == 0
    assert r.edge_id >= 0
    assert r.distance.radians > 0
    # get_distance agrees with the single-result distance.
    assert q.get_distance(_point(0, 20)).radians == pytest.approx(r.distance.radians)


def test_closest_edges_multi_sorted():
    idx = _square_index(n=8)
    q = s2rst.ClosestEdgeQuery(idx)
    rs = q.find_closest_edges(_point(0, 20), max_results=3, include_interiors=False)
    assert len(rs) == 3
    dists = [r.distance.radians for r in rs]
    assert dists == sorted(dists)


def test_is_distance_less():
    idx = _square_index()
    q = s2rst.ClosestEdgeQuery(idx)
    assert q.is_distance_less(_point(0, 6), s2rst.ChordAngle.from_degrees(5))
    assert not q.is_distance_less(_point(0, 60), s2rst.ChordAngle.from_degrees(5))


def test_edge_tuple_target():
    idx = _square_index()
    q = s2rst.ClosestEdgeQuery(idx)
    # An edge passing through the interior touches it (distance 0).
    d = q.get_distance((_point(0, -1), _point(0, 1)))
    assert d.radians == pytest.approx(0.0)


def test_cell_target():
    idx = _square_index()
    q = s2rst.ClosestEdgeQuery(idx)
    cell = s2rst.Cell.from_point(_point(0, 0))
    assert q.get_distance(cell).radians == pytest.approx(0.0)


def test_shapeindex_target():
    idx = _square_index(0, 0)
    other = _square_index(0, 30)
    q = s2rst.ClosestEdgeQuery(idx)
    assert q.get_distance(other).radians > 0


def test_bad_target_raises():
    q = s2rst.ClosestEdgeQuery(_square_index())
    with pytest.raises(TypeError):
        q.get_distance("not a target")


def test_furthest_edge():
    idx = _square_index()
    p = _point(0, 20)
    closest = s2rst.ClosestEdgeQuery(idx).find_closest_edge(p)
    furthest = s2rst.FurthestEdgeQuery(idx).find_furthest_edge(p)
    assert furthest.distance.radians >= closest.distance.radians
    assert s2rst.FurthestEdgeQuery(idx).is_distance_greater(
        p, s2rst.ChordAngle.from_degrees(1)
    )


def test_repr():
    idx = _square_index()
    assert "ClosestEdgeQuery" in repr(s2rst.ClosestEdgeQuery(idx))
    assert "FurthestEdgeQuery" in repr(s2rst.FurthestEdgeQuery(idx))
    r = s2rst.ClosestEdgeQuery(idx).find_closest_edge(_point(0, 0))
    assert "EdgeQueryResult" in repr(r)


@given(
    lat=st.floats(min_value=-80, max_value=80),
    lng=st.floats(min_value=-170, max_value=170),
)
def test_closest_le_furthest(lat, lng):
    idx = _square_index(0, 0, radius=5.0, n=6)
    p = _point(lat, lng)
    closest = s2rst.ClosestEdgeQuery(idx).get_distance(p)
    furthest = s2rst.FurthestEdgeQuery(idx).get_distance(p)
    assert closest.radians <= furthest.radians + 1e-12


@given(k=st.integers(min_value=1, max_value=10))
def test_multi_count_bounded_by_edges(k):
    idx = _square_index(0, 0, radius=3.0, n=6)
    rs = s2rst.ClosestEdgeQuery(idx).find_closest_edges(
        _point(0, 40), max_results=k, include_interiors=False
    )
    assert len(rs) == min(k, 6)  # one shape, 6 edges
