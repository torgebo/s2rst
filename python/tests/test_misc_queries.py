# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for ContainsVertexQuery, HausdorffDistanceQuery, and ShapeNestingQuery."""

import math

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _polyline_index(points):
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.LaxPolyline([_point(lat, lng) for lat, lng in points]))
    idx.build()
    return idx


def _ring(center_lat, center_lng, radius_deg, n, reverse):
    """A regular n-gon (as a list of S2Points) about a center, optionally reversed."""
    verts = []
    for i in range(n):
        angle = 2.0 * math.pi * i / n
        verts.append(
            _point(
                center_lat + radius_deg * math.sin(angle),
                center_lng + radius_deg * math.cos(angle),
            )
        )
    if reverse:
        verts.reverse()
    return verts


# ---------------------------------------------------------------------------
# ContainsVertexQuery
# ---------------------------------------------------------------------------


def test_contains_vertex_matched_pair_is_ambiguous():
    # A matched +1/-1 sibling edge pair to the same neighbour cancels out, so
    # containment is undetermined (0).
    target = _point(1.0, 2.0)
    v = _point(3.0, 4.0)
    q = s2rst.ContainsVertexQuery(target)
    q.add_edge(v, 1)
    q.add_edge(v, -1)
    assert q.contains_vertex() == 0
    assert q.duplicate_edges() is False


def test_contains_vertex_detects_duplicate_edges():
    target = _point(0.0, 0.0)
    q = s2rst.ContainsVertexQuery(target)
    v = _point(3.0, -3.0)
    # Same edge, same orientation, twice -> a duplicate.
    q.add_edge(v, -1)
    q.add_edge(v, -1)
    assert q.duplicate_edges() is True


def test_contains_vertex_nonzero_result():
    # A non-degenerate incoming/outgoing pair yields a definite +1/-1 answer.
    target = _point(89.0, 1.0)
    q = s2rst.ContainsVertexQuery(target)
    q.add_edge(_point(89.0, 0.0), -1)
    q.add_edge(_point(89.0, 2.0), 1)
    assert q.contains_vertex() in (-1, 1)
    assert q.duplicate_edges() is False


def test_contains_vertex_repr():
    q = s2rst.ContainsVertexQuery(_point(0.0, 0.0))
    assert "ContainsVertexQuery" in repr(q)


# ---------------------------------------------------------------------------
# HausdorffDistanceQuery
# ---------------------------------------------------------------------------


def test_hausdorff_parallel_polylines_about_one_degree():
    a = _polyline_index([(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)])
    b = _polyline_index([(1.0, 0.0), (1.0, 1.0), (1.0, 2.0)])
    q = s2rst.HausdorffDistanceQuery()
    d = q.get_distance(a, b)
    assert d.degrees == pytest.approx(1.0, abs=1e-3)


def test_hausdorff_is_symmetric():
    a = _polyline_index([(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)])
    b = _polyline_index([(1.0, 0.0), (1.0, 1.0), (1.0, 2.0)])
    q = s2rst.HausdorffDistanceQuery()
    assert q.get_distance(a, b).degrees == pytest.approx(q.get_distance(b, a).degrees)


def test_hausdorff_identical_is_zero():
    a = _polyline_index([(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)])
    b = _polyline_index([(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)])
    q = s2rst.HausdorffDistanceQuery()
    assert q.get_distance(a, b).degrees == pytest.approx(0.0, abs=1e-9)
    # Same object on both sides is fine (read-only access).
    assert q.get_distance(a, a).degrees == pytest.approx(0.0, abs=1e-9)


def test_hausdorff_directed_distance():
    a = _polyline_index([(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)])
    b = _polyline_index([(1.0, 0.0), (1.0, 1.0), (1.0, 2.0)])
    q = s2rst.HausdorffDistanceQuery()
    d = q.get_directed_distance(a, b)
    assert d.degrees == pytest.approx(1.0, abs=1e-3)


def test_hausdorff_is_distance_less():
    a = _polyline_index([(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)])
    b = _polyline_index([(1.0, 0.0), (1.0, 1.0), (1.0, 2.0)])
    q = s2rst.HausdorffDistanceQuery()
    assert q.is_distance_less(a, b, s2rst.ChordAngle.from_degrees(2.0))
    assert not q.is_distance_less(a, b, s2rst.ChordAngle.from_degrees(0.5))


def test_hausdorff_repr():
    assert "HausdorffDistanceQuery" in repr(s2rst.HausdorffDistanceQuery())
    assert "include_interiors" in repr(
        s2rst.HausdorffDistanceQuery(include_interiors=False)
    )


@given(
    lat=st.floats(min_value=-10, max_value=10),
    lng=st.floats(min_value=-10, max_value=10),
)
def test_hausdorff_symmetry_property(lat, lng):
    a = _polyline_index([(0.0, 0.0), (0.0, 1.0)])
    b = _polyline_index([(lat, lng), (lat, lng + 1.0)])
    q = s2rst.HausdorffDistanceQuery()
    assert q.get_distance(a, b).degrees == pytest.approx(q.get_distance(b, a).degrees)


# ---------------------------------------------------------------------------
# ShapeNestingQuery
# ---------------------------------------------------------------------------


def _donut_index():
    """A polygon: outer shell (chain 0) with one inner hole (chain 1)."""
    outer = _ring(0.0, 0.0, 1.0, 32, reverse=False)
    inner = _ring(0.0, 0.0, 0.5, 32, reverse=True)
    poly = s2rst.LaxPolygon([outer, inner])
    idx = s2rst.ShapeIndex()
    sid = idx.add(poly)
    idx.build()
    return idx, sid


def test_shape_nesting_shell_and_hole():
    idx, sid = _donut_index()
    relations = s2rst.ShapeNestingQuery(idx).compute_shape_nesting(sid)

    assert len(relations) == 2

    # Chain 0 is the shell, owning chain 1 as a hole.
    assert relations[0].is_shell()
    assert not relations[0].is_hole()
    assert relations[0].parent_id() is None
    assert relations[0].holes() == [1]

    # Chain 1 is the hole, parented by chain 0.
    assert relations[1].is_hole()
    assert not relations[1].is_shell()
    assert relations[1].parent_id() == 0
    assert relations[1].holes() == []


def test_shape_nesting_unknown_shape_is_empty():
    idx, _ = _donut_index()
    assert s2rst.ShapeNestingQuery(idx).compute_shape_nesting(999) == []


def test_shape_nesting_repr():
    idx, _ = _donut_index()
    assert "ShapeNestingQuery" in repr(s2rst.ShapeNestingQuery(idx))


def test_chain_relation_repr():
    idx, sid = _donut_index()
    relations = s2rst.ShapeNestingQuery(idx).compute_shape_nesting(sid)
    assert "ChainRelation" in repr(relations[0])
    assert "ChainRelation" in repr(relations[1])
