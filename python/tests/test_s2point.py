# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for S2Point, LatLng, s2_ortho, s2_rotate."""

import math
import pytest
import s2rst


class TestS2Point:
    def test_new_normalizes(self):
        p = s2rst.S2Point(0.0, 0.0, 5.0)
        assert p.z == pytest.approx(1.0)
        assert p.is_unit()

    def test_from_vector(self):
        v = s2rst.Vector(1.0, 0.0, 0.0)
        p = s2rst.S2Point.from_vector(v)
        assert p.x == pytest.approx(1.0)

    def test_origin(self):
        o = s2rst.S2Point.origin()
        assert o.is_unit()

    def test_xyz(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        assert p.x == pytest.approx(1.0)
        assert p.y == pytest.approx(0.0)
        assert p.z == pytest.approx(0.0)

    def test_vector(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        v = p.vector()
        assert isinstance(v, s2rst.Vector)
        assert v.x == pytest.approx(1.0)

    def test_distance(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        d = a.distance(b)
        assert isinstance(d, s2rst.Angle)
        assert d.degrees == pytest.approx(90.0)

    def test_chord_angle(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        ca = a.chord_angle(b)
        assert isinstance(ca, s2rst.ChordAngle)
        assert ca.degrees == pytest.approx(90.0)

    def test_stable_angle(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        sa = a.stable_angle(b)
        assert sa.degrees == pytest.approx(90.0)

    def test_approx_eq(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(1.0, 0.0, 0.0)
        assert a.approx_eq(b)

    def test_point_cross(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        c = a.point_cross(b)
        # point_cross returns a numerically stable cross product
        # The result is orthogonal to both inputs
        assert abs(c.x * a.x + c.y * a.y + c.z * a.z) < 1e-10
        assert abs(c.x * b.x + c.y * b.y + c.z * b.z) < 1e-10

    def test_arithmetic(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        s = a + b
        assert s.x == pytest.approx(1.0)
        assert s.y == pytest.approx(1.0)
        d = a - b
        assert d.x == pytest.approx(1.0)
        neg = -a
        assert neg.x == pytest.approx(-1.0)

    def test_comparisons(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        assert a == a
        # ordering is lexicographic by (x, y, z)
        assert (a < b) or (a > b) or (a == b)

    def test_indexing(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        assert len(p) == 3
        assert p[0] == pytest.approx(1.0)
        assert p[1] == pytest.approx(0.0)
        assert p[2] == pytest.approx(0.0)

    def test_repr(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        assert "S2Point" in repr(p)


class TestS2Ortho:
    def test_ortho(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        o = s2rst.s2_ortho(p)
        # Should be orthogonal
        assert abs(p.x * o.x + p.y * o.y + p.z * o.z) < 1e-14


class TestS2Rotate:
    def test_rotate_90(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        axis = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(90.0)
        r = s2rst.s2_rotate(p, axis, angle)
        assert r.x == pytest.approx(0.0, abs=1e-14)
        assert r.y == pytest.approx(1.0)

    def test_rotate_360(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        axis = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(360.0)
        r = s2rst.s2_rotate(p, axis, angle)
        assert r.approx_eq(p)


class TestLatLng:
    def test_from_degrees(self):
        ll = s2rst.LatLng.from_degrees(45.0, 90.0)
        assert ll.lat.degrees == pytest.approx(45.0)
        assert ll.lng.degrees == pytest.approx(90.0)

    def test_from_radians(self):
        ll = s2rst.LatLng.from_radians(math.pi / 4, math.pi / 2)
        assert ll.lat.degrees == pytest.approx(45.0)
        assert ll.lng.degrees == pytest.approx(90.0)

    def test_from_e5(self):
        ll = s2rst.LatLng.from_e5(4500000, 9000000)
        assert ll.lat.degrees == pytest.approx(45.0)
        assert ll.lng.degrees == pytest.approx(90.0)

    def test_from_e6(self):
        ll = s2rst.LatLng.from_e6(45000000, 90000000)
        assert ll.lat.degrees == pytest.approx(45.0)

    def test_from_e7(self):
        ll = s2rst.LatLng.from_e7(450000000, 900000000)
        assert ll.lat.degrees == pytest.approx(45.0)

    def test_from_point(self):
        p = s2rst.S2Point(0.0, 0.0, 1.0)
        ll = s2rst.LatLng.from_point(p)
        assert ll.lat.degrees == pytest.approx(90.0)
        assert ll.lng.degrees == pytest.approx(0.0)

    def test_to_point(self):
        ll = s2rst.LatLng.from_degrees(0.0, 0.0)
        p = ll.to_point()
        assert p.x == pytest.approx(1.0)
        assert p.y == pytest.approx(0.0, abs=1e-15)
        assert p.z == pytest.approx(0.0, abs=1e-15)

    def test_roundtrip(self):
        ll = s2rst.LatLng.from_degrees(37.7749, -122.4194)
        p = ll.to_point()
        ll2 = s2rst.LatLng.from_point(p)
        assert ll2.lat.degrees == pytest.approx(37.7749, abs=1e-10)
        assert ll2.lng.degrees == pytest.approx(-122.4194, abs=1e-10)

    def test_is_valid(self):
        ll = s2rst.LatLng.from_degrees(45.0, 90.0)
        assert ll.is_valid()

    def test_invalid(self):
        ll = s2rst.LatLng.invalid()
        assert not ll.is_valid()

    def test_normalized(self):
        ll = s2rst.LatLng.from_degrees(100.0, 200.0)
        n = ll.normalized()
        assert n.is_valid()

    def test_get_distance(self):
        a = s2rst.LatLng.from_degrees(0.0, 0.0)
        b = s2rst.LatLng.from_degrees(0.0, 90.0)
        d = a.get_distance(b)
        assert d.degrees == pytest.approx(90.0)

    def test_approx_equal(self):
        a = s2rst.LatLng.from_degrees(45.0, 90.0)
        b = s2rst.LatLng.from_degrees(45.0, 90.0)
        assert a.approx_equal(b)

    def test_arithmetic(self):
        a = s2rst.LatLng.from_degrees(10.0, 20.0)
        b = s2rst.LatLng.from_degrees(5.0, 10.0)
        s = a + b
        assert s.lat.degrees == pytest.approx(15.0)
        d = a - b
        assert d.lat.degrees == pytest.approx(5.0)
        m = a * 2.0
        assert m.lat.degrees == pytest.approx(20.0)
        rm = 2.0 * a
        assert rm.lng.degrees == pytest.approx(40.0)

    def test_latitude_longitude(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        lat = s2rst.LatLng.latitude(p)
        lng = s2rst.LatLng.longitude(p)
        assert lat.degrees == pytest.approx(0.0)
        assert lng.degrees == pytest.approx(0.0)

    def test_repr(self):
        ll = s2rst.LatLng.from_degrees(45.0, 90.0)
        r = repr(ll)
        assert "LatLng" in r
        assert "45" in r
