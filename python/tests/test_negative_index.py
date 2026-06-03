# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tier B3: negative indexing on __getitem__.

Every sequence-like type now accepts Python-style negative indices on
subscription (obj[-1] == obj[len-1]); out-of-range indices in either
direction raise IndexError instead of OverflowError (the old behaviour of
the usize parameter rejecting negatives).
"""

import pytest

import s2rst


def _triangle():
    return [
        s2rst.S2Point(1, 0, 0),
        s2rst.S2Point(0, 1, 0),
        s2rst.S2Point(0, 0, 1),
    ]


def _containers():
    pts = _triangle()
    return [
        s2rst.R1Interval(1.0, 2.0),
        s2rst.S1Interval(0.5, 1.5),
        s2rst.R2Point(1.0, 2.0),
        s2rst.Vector(1.0, 2.0, 3.0),
        s2rst.S2Point(1, 0, 0),
        s2rst.CellUnion.from_cell_ids(
            [s2rst.CellId.from_face(0), s2rst.CellId.from_face(2)]
        ),
        s2rst.Polyline(pts),
        s2rst.Loop(pts),
        s2rst.Polygon([s2rst.Loop(pts)]),
        s2rst.LaxLoop(pts),
        s2rst.LaxPolyline(pts),
        s2rst.PointVector(pts),
        s2rst.EdgeVectorShape.from_edges(
            [s2rst.Edge(pts[0], pts[1]), s2rst.Edge(pts[1], pts[2])]
        ),
    ]


@pytest.mark.parametrize("obj", _containers(), ids=lambda o: type(o).__name__)
def test_negative_index_maps_to_positive(obj):
    n = len(obj)
    assert n >= 1
    for k in range(n):
        assert obj[k - n] == obj[k]
    assert obj[-1] == obj[n - 1]


@pytest.mark.parametrize("obj", _containers(), ids=lambda o: type(o).__name__)
def test_index_out_of_range_raises(obj):
    n = len(obj)
    with pytest.raises(IndexError):
        _ = obj[n]
    with pytest.raises(IndexError):
        _ = obj[-n - 1]


def test_explicit_negative_values():
    r = s2rst.R1Interval(1.0, 2.0)
    assert r[-1] == r[1] == 2.0
    assert r[-2] == r[0] == 1.0
    v = s2rst.Vector(1.0, 2.0, 3.0)
    assert v[-1] == 3.0
    assert v[-2] == 2.0
    assert v[-3] == 1.0
