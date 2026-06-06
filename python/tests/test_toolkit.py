# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for the low-level geometry toolkit.

Covers the orientation/crossing/wedge predicates, the EdgeCrosser, edge
distance helpers, and the cube-face coordinate transforms. Oracles are derived
independently of the implementation (hand-checked geometry on the unit sphere).
"""

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


# Octant corner points: (1,0,0), (0,1,0), (0,0,1) are in CCW order because
# det[a b c] = a . (b x c) = +1 > 0.
A = s2rst.S2Point(1, 0, 0)
B = s2rst.S2Point(0, 1, 0)
C = s2rst.S2Point(0, 0, 1)


# --------------------------------------------------------------------------
# Predicates
# --------------------------------------------------------------------------


def test_sign_ccw_true():
    assert s2rst.sign(A, B, C) is True


def test_sign_cw_false():
    # Reversing two vertices flips the orientation.
    assert s2rst.sign(A, C, B) is False


def test_robust_sign_ccw():
    assert s2rst.robust_sign(A, B, C) == s2rst.Direction.COUNTER_CLOCKWISE


def test_robust_sign_cw():
    assert s2rst.robust_sign(A, C, B) == s2rst.Direction.CLOCKWISE


def test_robust_sign_indeterminate_on_duplicate():
    # robust_sign returns INDETERMINATE iff two of the points coincide.
    assert s2rst.robust_sign(A, A, B) == s2rst.Direction.INDETERMINATE


def test_ordered_ccw_around_pole():
    # Three equatorial points at increasing longitude, swept CCW about the
    # north pole, are encountered in order.
    o = C  # north pole
    p0 = _point(0, 0)
    p1 = _point(0, 90)
    p2 = _point(0, 179)
    assert s2rst.ordered_ccw(p0, p1, p2, o) is True
    # Reversed sweep order is not all-ordered.
    assert s2rst.ordered_ccw(p2, p1, p0, o) is False


# --------------------------------------------------------------------------
# Edge crossings
# --------------------------------------------------------------------------


def test_crossing_sign_cross():
    # An equatorial segment and a meridian segment that intersect at (0, 0).
    a, b = _point(0, -45), _point(0, 45)
    c, d = _point(-45, 0), _point(45, 0)
    assert s2rst.crossing_sign(a, b, c, d) == s2rst.Crossing.CROSS


def test_crossing_sign_do_not_cross():
    # Two short equatorial segments on opposite sides of the sphere.
    a, b = _point(0, 0), _point(0, 10)
    c, d = _point(0, 90), _point(0, 100)
    assert s2rst.crossing_sign(a, b, c, d) == s2rst.Crossing.DO_NOT_CROSS


def test_crossing_sign_maybe_cross_shared_vertex():
    # Sharing a vertex yields MAYBE_CROSS.
    a, b = _point(0, 0), _point(0, 30)
    c, d = _point(0, 0), _point(30, 0)
    assert s2rst.crossing_sign(a, b, c, d) == s2rst.Crossing.MAYBE_CROSS


def test_edge_or_vertex_crossing_interior():
    a, b = _point(0, -45), _point(0, 45)
    c, d = _point(-45, 0), _point(45, 0)
    assert s2rst.edge_or_vertex_crossing(a, b, c, d) is True


def test_edge_or_vertex_crossing_far():
    a, b = _point(0, 0), _point(0, 10)
    c, d = _point(0, 90), _point(0, 100)
    assert s2rst.edge_or_vertex_crossing(a, b, c, d) is False


def test_intersection_at_origin_meridian():
    # The two crossing edges intersect at lat/lng (0, 0) == (1, 0, 0).
    a, b = _point(0, -45), _point(0, 45)
    c, d = _point(-45, 0), _point(45, 0)
    pt = s2rst.intersection(a, b, c, d)
    expected = _point(0, 0)
    assert pt.approx_eq(expected)


def test_robust_cross_prod_orthogonal_to_axes():
    n = s2rst.robust_cross_prod(A, B)  # x cross y ~ +z direction
    nv = n.normalize()
    assert nv.z == pytest.approx(1.0, abs=1e-9)
    assert nv.x == pytest.approx(0.0, abs=1e-9)
    assert nv.y == pytest.approx(0.0, abs=1e-9)


# --------------------------------------------------------------------------
# EdgeCrosser
# --------------------------------------------------------------------------


def test_edge_crosser_crossing_sign():
    a, b = _point(0, -45), _point(0, 45)
    crosser = s2rst.EdgeCrosser(a, b)
    c, d = _point(-45, 0), _point(45, 0)
    assert crosser.crossing_sign(c, d) == s2rst.Crossing.CROSS
    # A far edge does not cross.
    e, f = _point(0, 120), _point(0, 150)
    assert crosser.crossing_sign(e, f) == s2rst.Crossing.DO_NOT_CROSS


def test_edge_crosser_edge_or_vertex_crossing():
    a, b = _point(0, -45), _point(0, 45)
    crosser = s2rst.EdgeCrosser(a, b)
    c, d = _point(-45, 0), _point(45, 0)
    assert crosser.edge_or_vertex_crossing(c, d) is True


def test_edge_crosser_matches_free_function():
    a, b = _point(10, -30), _point(-5, 40)
    c, d = _point(-20, 5), _point(25, 5)
    crosser = s2rst.EdgeCrosser(a, b)
    assert crosser.crossing_sign(c, d) == s2rst.crossing_sign(a, b, c, d)


# --------------------------------------------------------------------------
# Edge distances
# --------------------------------------------------------------------------


def test_interpolate_endpoints():
    a, b = _point(0, 0), _point(0, 50)
    assert s2rst.interpolate(0.0, a, b).approx_eq(a)
    assert s2rst.interpolate(1.0, a, b).approx_eq(b)


def test_interpolate_midpoint_on_equator():
    a, b = _point(0, 0), _point(0, 80)
    mid = s2rst.interpolate(0.5, a, b)
    assert mid.approx_eq(_point(0, 40))


def test_interpolate_at_distance():
    a, b = _point(0, 0), _point(0, 90)
    quarter = s2rst.interpolate_at_distance(s2rst.Angle.from_degrees(30.0), a, b)
    assert quarter.approx_eq(_point(0, 30))


def test_project_onto_equator():
    a, b = _point(0, 0), _point(0, 90)
    # A point north of the segment projects down to the equator.
    x = _point(20, 40)
    proj = s2rst.project(x, a, b)
    assert proj.approx_eq(_point(0, 40))


def test_distance_from_segment_to_offset_point():
    a, b = _point(0, 0), _point(0, 90)
    x = _point(15, 45)
    dist = s2rst.distance_from_segment(x, a, b)
    # Distance to the equator from latitude 15 is exactly 15 degrees.
    assert dist.degrees == pytest.approx(15.0, abs=1e-9)


def test_distance_fraction_midpoint():
    a, b = _point(0, 0), _point(0, 90)
    x = _point(10, 45)  # nearest point is the segment midpoint
    assert s2rst.distance_fraction(x, a, b) == pytest.approx(0.5, abs=1e-9)


# --------------------------------------------------------------------------
# Wedge relations
# --------------------------------------------------------------------------


# A wedge (a0, ab1, a2) sweeps CLOCKWISE from a0 to a2 about the apex ab1.
# With apex = +z, the CCW order viewed from outside is +x -> +y -> -x -> -y,
# so a clockwise sweep goes the other way. These configurations are the
# hand-verified cases from the core library's own wedge tests.
_APEX = s2rst.S2Point(0, 0, 1)
_PX = s2rst.S2Point(1, 0, 0)
_PY = s2rst.S2Point(0, 1, 0)
_NX = s2rst.S2Point(-1, 0, 0)
_NY = s2rst.S2Point(0, -1, 0)


def test_wedge_relation_equal():
    rel = s2rst.wedge_relation(_PX, _APEX, _PY, _PX, _PY)
    assert rel == s2rst.WedgeRel.EQUAL
    assert s2rst.wedge_contains(_PX, _APEX, _PY, _PX, _PY) is True


def test_wedge_relation_properly_contains():
    # A is the large 270 deg wedge CW from +x to +y (through -y, -x).
    # B is the small 90 deg wedge CW from -y to -x, entirely inside A.
    rel = s2rst.wedge_relation(_PX, _APEX, _PY, _NY, _NX)
    assert rel == s2rst.WedgeRel.PROPERLY_CONTAINS
    assert s2rst.wedge_contains(_PX, _APEX, _PY, _NY, _NX) is True


def test_wedge_relation_is_properly_contained():
    # Swap A and B from the contains case: now A is the small wedge.
    rel = s2rst.wedge_relation(_NY, _APEX, _NX, _PX, _PY)
    assert rel == s2rst.WedgeRel.IS_PROPERLY_CONTAINED
    # The small wedge A does not contain the large wedge B.
    assert s2rst.wedge_contains(_NY, _APEX, _NX, _PX, _PY) is False


# --------------------------------------------------------------------------
# Cube-face coordinates
# --------------------------------------------------------------------------


def test_face_uv_to_xyz_face0_center():
    p = s2rst.face_uv_to_xyz(0, 0.0, 0.0)
    # Center of face 0 is the +x axis.
    assert p.normalize().approx_eq(s2rst.S2Point(1, 0, 0))


def test_xyz_to_face_uv_plus_x():
    face, u, v = s2rst.xyz_to_face_uv(s2rst.S2Point(1, 0, 0))
    assert face == 0
    assert u == pytest.approx(0.0, abs=1e-12)
    assert v == pytest.approx(0.0, abs=1e-12)


def test_st_uv_roundtrip():
    for s in (0.0, 0.25, 0.5, 0.75, 1.0):
        assert s2rst.uv_to_st(s2rst.st_to_uv(s)) == pytest.approx(s, abs=1e-12)


def test_st_to_uv_endpoints():
    assert s2rst.st_to_uv(0.0) == pytest.approx(-1.0, abs=1e-12)
    assert s2rst.st_to_uv(0.5) == pytest.approx(0.0, abs=1e-12)
    assert s2rst.st_to_uv(1.0) == pytest.approx(1.0, abs=1e-12)


def test_axes_orthonormal_face0():
    u_axis = s2rst.get_u_axis(0)
    v_axis = s2rst.get_v_axis(0)
    assert u_axis.is_unit()
    assert v_axis.is_unit()
    # The u and v axes are orthogonal.
    assert u_axis.vector().dot(v_axis.vector()) == pytest.approx(0.0, abs=1e-12)


def test_face_from_int_out_of_range():
    with pytest.raises(ValueError):
        s2rst.face_uv_to_xyz(6, 0.0, 0.0)
    with pytest.raises(ValueError):
        s2rst.get_u_axis(9)


@given(
    f=st.integers(min_value=0, max_value=5),
    u=st.floats(min_value=-1, max_value=1, exclude_min=True, exclude_max=True),
    v=st.floats(min_value=-1, max_value=1, exclude_min=True, exclude_max=True),
)
def test_face_uv_roundtrip(f, u, v):
    # Round-tripping (f, u, v) -> xyz -> (f, u, v) recovers the originals.
    # Restricted to the open square so the face assignment is unambiguous
    # (a |u| or |v| of exactly 1 ties two faces).
    p = s2rst.face_uv_to_xyz(f, u, v)
    f2, u2, v2 = s2rst.xyz_to_face_uv(p)
    assert f2 == f
    assert u2 == pytest.approx(u, abs=1e-9)
    assert v2 == pytest.approx(v, abs=1e-9)


# --------------------------------------------------------------------------
# Enums: hashable and distinct
# --------------------------------------------------------------------------


def test_direction_enum_distinct_and_hashable():
    members = {
        s2rst.Direction.CLOCKWISE,
        s2rst.Direction.INDETERMINATE,
        s2rst.Direction.COUNTER_CLOCKWISE,
    }
    assert len(members) == 3
    assert s2rst.Direction.CLOCKWISE == s2rst.Direction.CLOCKWISE
    assert s2rst.Direction.CLOCKWISE != s2rst.Direction.COUNTER_CLOCKWISE


def test_crossing_enum_distinct_and_hashable():
    members = {
        s2rst.Crossing.CROSS,
        s2rst.Crossing.MAYBE_CROSS,
        s2rst.Crossing.DO_NOT_CROSS,
    }
    assert len(members) == 3
    assert s2rst.Crossing.CROSS != s2rst.Crossing.DO_NOT_CROSS


def test_wedge_rel_enum_distinct_and_hashable():
    members = {
        s2rst.WedgeRel.EQUAL,
        s2rst.WedgeRel.PROPERLY_CONTAINS,
        s2rst.WedgeRel.IS_PROPERLY_CONTAINED,
        s2rst.WedgeRel.PROPERLY_OVERLAPS,
        s2rst.WedgeRel.IS_DISJOINT,
    }
    assert len(members) == 5
    assert s2rst.WedgeRel.EQUAL != s2rst.WedgeRel.IS_DISJOINT
