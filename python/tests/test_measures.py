# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for geometric measures: ShapeIndex measures and triangle free functions."""

import math

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst

# The three axis points span one octant of the sphere = 1/8 of 4*pi = pi/2 sr.
A = s2rst.S2Point(1, 0, 0)
B = s2rst.S2Point(0, 1, 0)
C = s2rst.S2Point(0, 0, 1)


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def test_point_area_octant():
    assert s2rst.point_area(A, B, C) == pytest.approx(math.pi / 2, abs=1e-9)


def test_signed_area_orientation():
    pos = s2rst.signed_area(A, B, C)
    assert pos > 0
    assert s2rst.signed_area(A, C, B) == pytest.approx(-pos, abs=1e-12)


def test_true_centroid_points_into_octant():
    centroid = s2rst.true_centroid(A, B, C).normalize()
    # The octant centroid direction has equal positive components.
    assert centroid.x > 0 and centroid.y > 0 and centroid.z > 0
    assert centroid.x == pytest.approx(centroid.y, abs=1e-9)


def test_turn_angle_is_angle():
    ta = s2rst.turn_angle(A, B, C)
    assert isinstance(ta, s2rst.Angle)


def test_index_dimension_and_area():
    idx = s2rst.ShapeIndex()
    assert idx.get_dimension() is None  # empty
    loop = s2rst.Loop([A, B, C])  # the octant triangle
    idx.add(loop)
    idx.build()
    assert idx.get_dimension() == 2
    assert idx.get_area() == pytest.approx(math.pi / 2, abs=1e-7)
    assert idx.get_perimeter().radians > 0
    assert idx.get_centroid().vector().norm() > 0


def test_index_length_for_polyline():
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.Polyline([_point(0, 0), _point(0, 1), _point(0, 2)]))
    idx.build()
    assert idx.get_dimension() == 1
    assert idx.get_length().degrees == pytest.approx(2.0, abs=1e-6)
    assert idx.get_area() == 0.0


@given(
    a=st.tuples(st.floats(-80, 80), st.floats(-170, 170)),
    b=st.tuples(st.floats(-80, 80), st.floats(-170, 170)),
    c=st.tuples(st.floats(-80, 80), st.floats(-170, 170)),
)
def test_point_area_cyclic_and_nonnegative(a, b, c):
    pa, pb, pc = _point(*a), _point(*b), _point(*c)
    area = s2rst.point_area(pa, pb, pc)
    assert area >= 0
    # point_area is invariant under cyclic rotation of the vertices.
    assert s2rst.point_area(pb, pc, pa) == pytest.approx(area, abs=1e-12)
