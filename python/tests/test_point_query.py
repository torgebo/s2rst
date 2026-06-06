# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for S2PointIndex, ClosestPointQuery, and PointQueryResult."""

from collections.abc import Iterable, Sized

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _labeled_index():
    idx = s2rst.S2PointIndex()
    idx.add(_point(0, 0), "a")
    idx.add(_point(0, 1), "b")
    idx.add(_point(0, 2), "c")
    return idx


def test_index_basics():
    idx = _labeled_index()
    assert len(idx) == 3
    assert idx.num_points() == 3
    assert isinstance(idx, Sized)
    assert isinstance(idx, Iterable)


def test_find_closest_point():
    q = s2rst.ClosestPointQuery(_labeled_index())
    r = q.find_closest_point(_point(0, 0.4))
    assert r.data == "a"
    assert r.point.approx_eq(_point(0, 0))
    assert r.distance.degrees == pytest.approx(0.4, abs=1e-3)


def test_find_closest_points_sorted():
    q = s2rst.ClosestPointQuery(_labeled_index())
    rs = q.find_closest_points(_point(0, 0), max_results=2)
    assert [r.data for r in rs] == ["a", "b"]
    dists = [r.distance.degrees for r in rs]
    assert dists == sorted(dists)


def test_get_distance_and_is_distance_less():
    q = s2rst.ClosestPointQuery(_labeled_index())
    assert q.get_distance(_point(0, 0)).degrees == pytest.approx(0.0, abs=1e-9)
    assert q.is_distance_less(_point(0, 0.1), s2rst.ChordAngle.from_degrees(0.5))
    assert not q.is_distance_less(_point(0, 40), s2rst.ChordAngle.from_degrees(0.5))


def test_iteration_and_remove_clear():
    idx = _labeled_index()
    collected = sorted(d for _, d in idx)
    assert collected == ["a", "b", "c"]
    assert idx.remove(_point(0, 1), "b")
    assert len(idx) == 2
    assert not idx.remove(_point(0, 1), "b")  # already gone
    idx.clear()
    assert len(idx) == 0


def test_data_identity_round_trip():
    obj = object()
    idx = s2rst.S2PointIndex()
    idx.add(_point(1, 1), obj)
    r = s2rst.ClosestPointQuery(idx).find_closest_point(_point(1, 1))
    assert r.data is obj


def test_none_data():
    idx = s2rst.S2PointIndex()
    idx.add(_point(0, 0))
    r = s2rst.ClosestPointQuery(idx).find_closest_point(_point(0, 0))
    assert r.data is None


def test_empty_index_result_is_empty():
    q = s2rst.ClosestPointQuery(s2rst.S2PointIndex())
    assert q.find_closest_point(_point(0, 0)).is_empty()


def test_non_point_targets():
    q = s2rst.ClosestPointQuery(_labeled_index())
    # Cell target and edge-tuple target both resolve to a finite distance.
    assert q.get_distance(s2rst.Cell.from_point(_point(0, 0))).degrees == pytest.approx(
        0.0, abs=1e-9
    )
    assert q.get_distance((_point(0, 0), _point(0, 0.5))).radians >= 0


def test_bad_target_raises():
    q = s2rst.ClosestPointQuery(_labeled_index())
    with pytest.raises(TypeError):
        q.get_distance(42)


def test_repr():
    assert "S2PointIndex" in repr(_labeled_index())
    assert "ClosestPointQuery" in repr(s2rst.ClosestPointQuery(_labeled_index()))


@given(
    n=st.integers(min_value=1, max_value=15),
    qlat=st.floats(min_value=-80, max_value=80),
    qlng=st.floats(min_value=-170, max_value=170),
)
def test_knn_sorted_and_complete(n, qlat, qlng):
    idx = s2rst.S2PointIndex()
    for i in range(n):
        idx.add(_point(0, i), i)
    q = s2rst.ClosestPointQuery(idx)
    rs = q.find_closest_points(_point(qlat, qlng), max_results=n)
    assert len(rs) == n
    dists = [r.distance.radians for r in rs]
    assert dists == sorted(dists)
    assert q.get_distance(_point(qlat, qlng)).radians == pytest.approx(dists[0])
