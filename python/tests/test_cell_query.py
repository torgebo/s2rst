# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for ClosestCellQuery over a CellIndex."""

import pytest

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _cell_at(lat, lng, level=10):
    return s2rst.CellId.from_point(_point(lat, lng)).parent_at_level(level)


def _index(entries):
    idx = s2rst.CellIndex()
    for cell, label in entries:
        idx.add(cell, label)
    idx.build()
    return idx


def test_closest_cell_basic():
    c0 = _cell_at(0, 0)
    c1 = _cell_at(0, 30)
    idx = _index([(c0, 7), (c1, 9)])
    q = s2rst.ClosestCellQuery(idx)
    r = q.find_closest_cell(_point(0, 0))
    assert r.label == 7
    assert r.cell_id == c0
    assert r.distance.degrees == pytest.approx(0.0, abs=1.0)  # within the cell


def test_get_distance_and_nearest_of_two():
    idx = _index([(_cell_at(0, 0), 1), (_cell_at(0, 40), 2)])
    q = s2rst.ClosestCellQuery(idx)
    assert q.find_closest_cell(_point(0, 1)).label == 1
    assert q.find_closest_cell(_point(0, 39)).label == 2
    assert q.get_distance(_point(0, 0)).degrees < q.get_distance(_point(0, 20)).degrees


def test_find_closest_cells_multi():
    idx = _index([(_cell_at(0, i * 5), i) for i in range(4)])
    q = s2rst.ClosestCellQuery(idx)
    rs = q.find_closest_cells(_point(0, 0), max_results=3)
    assert len(rs) == 3
    dists = [r.distance.radians for r in rs]
    assert dists == sorted(dists)
    assert rs[0].label == 0


def test_cell_union_target():
    idx = _index([(_cell_at(0, 0), 5)])
    q = s2rst.ClosestCellQuery(idx)
    cu = s2rst.CellUnion.from_cell_ids([s2rst.CellId.from_face(0)])
    assert q.get_distance(cu).radians >= 0


def test_is_distance_less():
    idx = _index([(_cell_at(0, 0), 1)])
    q = s2rst.ClosestCellQuery(idx)
    assert q.is_distance_less(_point(0, 1), s2rst.ChordAngle.from_degrees(5))
    assert not q.is_distance_less(_point(0, 60), s2rst.ChordAngle.from_degrees(5))


def test_bad_target_raises():
    idx = _index([(_cell_at(0, 0), 1)])
    q = s2rst.ClosestCellQuery(idx)
    with pytest.raises(TypeError):
        q.get_distance("nope")


def test_repr():
    idx = _index([(_cell_at(0, 0), 1)])
    assert "ClosestCellQuery" in repr(s2rst.ClosestCellQuery(idx))
