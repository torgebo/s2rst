# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for PolylineSimplifier and the polyline_alignment free functions."""

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _polyline(*coords):
    return s2rst.Polyline([_point(lat, lng) for lat, lng in coords])


# ---------------------------------------------------------------------------
# Vertex alignment free functions
# ---------------------------------------------------------------------------


def test_exact_cost_identical_is_zero():
    p = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    assert s2rst.get_exact_vertex_alignment_cost(p, p) == 0.0


def test_exact_alignment_identical_is_diagonal():
    p = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    warp_path, cost = s2rst.get_exact_vertex_alignment(p, p)
    assert cost == 0.0
    # warp_path is a list of (i, j) index pairs; identical inputs pair i==j.
    assert isinstance(warp_path, list)
    assert all(isinstance(pair, tuple) and len(pair) == 2 for pair in warp_path)
    assert (0, 0) in warp_path
    assert (2, 2) in warp_path


def test_exact_cost_positive_for_distinct():
    a = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    b = s2rst.Polyline([_point(1, 0), _point(1, 1), _point(1, 2)])
    assert s2rst.get_exact_vertex_alignment_cost(a, b) > 0.0


def test_approx_alignment_returns_pair():
    a = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    b = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    warp_path, cost = s2rst.get_approx_vertex_alignment(a, b, 2)
    assert isinstance(warp_path, list)
    assert cost == pytest.approx(0.0, abs=1e-12)


def test_medoid_of_identical_is_first():
    p = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    assert s2rst.get_medoid_polyline([p, p, p]) == 0


def test_medoid_picks_central_polyline():
    # Three parallel polylines at latitudes 0, 1, 2: the middle (index 1) is
    # closest to both others, so it is the medoid.
    low = _polyline((0, 0), (0, 1), (0, 2))
    mid = _polyline((1, 0), (1, 1), (1, 2))
    high = _polyline((2, 0), (2, 1), (2, 2))
    assert s2rst.get_medoid_polyline([low, mid, high]) == 1


def test_consensus_returns_polyline():
    p = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    result = s2rst.get_consensus_polyline([p, p])
    assert isinstance(result, s2rst.Polyline)
    assert result.num_vertices() == p.num_vertices()


def test_consensus_of_identical_matches_input():
    p = s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)])
    result = s2rst.get_consensus_polyline([p, p, p])
    # Averaging identical polylines reproduces the same vertices.
    assert result.num_vertices() == 3
    for i in range(3):
        assert result.vertex(i).approx_eq(p.vertex(i))


# ---------------------------------------------------------------------------
# PolylineSimplifier
# ---------------------------------------------------------------------------


def test_simplifier_basic_extend_collinear():
    s = s2rst.PolylineSimplifier()
    s.init(_point(0, 0))
    # No constraints; a short edge well under 90 degrees can be extended.
    assert s.extend(_point(0, 2)) is True


def test_simplifier_src_roundtrip():
    s = s2rst.PolylineSimplifier()
    src = _point(0, 0)
    s.init(src)
    assert s.src().approx_eq(src)


def test_simplifier_rejects_long_edge():
    s = s2rst.PolylineSimplifier()
    s.init(_point(0, 0))
    # Edges longer than 90 degrees are unsupported and rejected.
    assert s.extend(_point(0, 91)) is False


def test_simplifier_target_disc_on_path():
    s = s2rst.PolylineSimplifier()
    s.init(_point(0, 0))
    radius = s2rst.ChordAngle.from_angle(s2rst.Angle.from_degrees(1.0))
    # A target disc straddling the straight path is reachable.
    assert s.target_disc(_point(0, 1), radius) is True
    assert s.extend(_point(0, 2)) is True


def test_simplifier_avoid_disc_blocks_edge():
    s = s2rst.PolylineSimplifier()
    s.init(_point(0, 0))
    radius = s2rst.ChordAngle.from_angle(s2rst.Angle.from_degrees(1e-10))
    # A tiny disc sitting on the straight path, required to stay on the left,
    # makes the straight edge to (0, 2) impossible. avoid_disc itself can
    # still return True (the constraint is recorded as a pending range when no
    # target has narrowed the window yet); the rejection shows up in extend.
    s.avoid_disc(_point(0, 1), radius, True)
    assert s.extend(_point(0, 2)) is False


def test_simplifier_repr():
    assert "PolylineSimplifier" in repr(s2rst.PolylineSimplifier())


# ---------------------------------------------------------------------------
# Properties
# ---------------------------------------------------------------------------


@given(
    a_lat=st.integers(min_value=-5, max_value=5),
    b_lat=st.integers(min_value=-5, max_value=5),
    radius=st.integers(min_value=0, max_value=4),
)
def test_approx_never_beats_exact(a_lat, b_lat, radius):
    # FastDTW is an upper bound on the optimal alignment cost, so the
    # approximate cost is never (meaningfully) less than the exact cost.
    a = _polyline((a_lat, 0), (a_lat, 1), (a_lat, 2))
    b = _polyline((b_lat, 0), (b_lat, 1), (b_lat, 2))
    exact = s2rst.get_exact_vertex_alignment_cost(a, b)
    _, approx = s2rst.get_approx_vertex_alignment(a, b, radius)
    assert approx >= exact - 1e-9
