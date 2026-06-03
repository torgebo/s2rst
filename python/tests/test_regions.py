# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for Cap and Rect."""

import math
import pytest
import s2rst


class TestCap:
    def test_empty(self):
        cap = s2rst.Cap.empty()
        assert cap.is_empty()
        assert not cap.is_full()

    def test_full(self):
        cap = s2rst.Cap.full()
        assert cap.is_full()
        assert not cap.is_empty()

    def test_from_point(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        cap = s2rst.Cap.from_point(p)
        assert cap.contains_point(p)
        assert cap.is_valid()

    def test_from_center_angle(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(45.0)
        cap = s2rst.Cap.from_center_angle(center, angle)
        assert cap.center.approx_eq(center)
        assert cap.angle_radius().degrees == pytest.approx(45.0)

    def test_from_center_chord_angle(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        radius = s2rst.ChordAngle.from_degrees(45.0)
        cap = s2rst.Cap(center, radius)
        assert cap.is_valid()

    def test_from_center_height(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        cap = s2rst.Cap.from_center_height(center, 1.0)
        assert cap.height() == pytest.approx(1.0)

    def test_from_center_area(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        cap = s2rst.Cap.from_center_area(center, 2 * math.pi)
        assert cap.area() == pytest.approx(2 * math.pi, rel=1e-10)

    def test_area(self):
        cap = s2rst.Cap.full()
        assert cap.area() == pytest.approx(4 * math.pi)
        cap = s2rst.Cap.empty()
        assert cap.area() == pytest.approx(0.0)

    def test_centroid(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        cap = s2rst.Cap.from_point(center)
        c = cap.centroid()
        assert isinstance(c, s2rst.S2Point)

    def test_contains_point(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(10.0)
        cap = s2rst.Cap.from_center_angle(center, angle)
        assert cap.contains_point(center)
        assert not cap.contains_point(s2rst.S2Point(0.0, 0.0, -1.0))

    def test_interior_contains_point(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(10.0)
        cap = s2rst.Cap.from_center_angle(center, angle)
        assert cap.interior_contains_point(center)

    def test_contains_cap(self):
        big = s2rst.Cap.from_center_angle(
            s2rst.S2Point(0.0, 0.0, 1.0), s2rst.Angle.from_degrees(90.0)
        )
        small = s2rst.Cap.from_center_angle(
            s2rst.S2Point(0.0, 0.0, 1.0), s2rst.Angle.from_degrees(10.0)
        )
        assert big.contains_cap(small)
        assert not small.contains_cap(big)

    def test_intersects(self):
        a = s2rst.Cap.from_center_angle(
            s2rst.S2Point(1.0, 0.0, 0.0), s2rst.Angle.from_degrees(45.0)
        )
        b = s2rst.Cap.from_center_angle(
            s2rst.S2Point(0.0, 1.0, 0.0), s2rst.Angle.from_degrees(45.0)
        )
        # 45° caps on orthogonal axes meet at the 45° midpoint
        assert a.intersects(b)
        # But 30° caps on orthogonal axes don't
        c = s2rst.Cap.from_center_angle(
            s2rst.S2Point(1.0, 0.0, 0.0), s2rst.Angle.from_degrees(30.0)
        )
        d = s2rst.Cap.from_center_angle(
            s2rst.S2Point(0.0, 1.0, 0.0), s2rst.Angle.from_degrees(30.0)
        )
        assert not c.intersects(d)

    def test_complement(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(45.0)
        cap = s2rst.Cap.from_center_angle(center, angle)
        comp = cap.complement()
        assert not comp.contains_point(center)

    def test_expanded(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(45.0)
        cap = s2rst.Cap.from_center_angle(center, angle)
        expanded = cap.expanded(s2rst.Angle.from_degrees(10.0))
        assert expanded.angle_radius().degrees > cap.angle_radius().degrees

    def test_union(self):
        a = s2rst.Cap.from_center_angle(
            s2rst.S2Point(1.0, 0.0, 0.0), s2rst.Angle.from_degrees(10.0)
        )
        b = s2rst.Cap.from_center_angle(
            s2rst.S2Point(0.0, 1.0, 0.0), s2rst.Angle.from_degrees(10.0)
        )
        u = a.union(b)
        assert u.contains_cap(a)
        assert u.contains_cap(b)

    def test_add_point(self):
        cap = s2rst.Cap.empty()
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        cap2 = cap.add_point(p)
        assert cap2.contains_point(p)

    def test_bounds(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(45.0)
        cap = s2rst.Cap.from_center_angle(center, angle)
        cb = cap.cap_bound()
        assert isinstance(cb, s2rst.Cap)
        rb = cap.rect_bound()
        assert isinstance(rb, s2rst.Rect)
        cub = cap.cell_union_bound()
        assert len(cub) >= 1

    def test_approx_equal(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        angle = s2rst.Angle.from_degrees(45.0)
        a = s2rst.Cap.from_center_angle(center, angle)
        b = s2rst.Cap.from_center_angle(center, angle)
        assert a.approx_equal(b)


class TestRect:
    def test_empty(self):
        r = s2rst.Rect.empty()
        assert r.is_empty()

    def test_full(self):
        r = s2rst.Rect.full()
        assert r.is_full()

    def test_from_lat_lng(self):
        ll = s2rst.LatLng.from_degrees(45.0, 90.0)
        r = s2rst.Rect.from_lat_lng(ll)
        assert r.is_point()

    def test_from_center_size(self):
        center = s2rst.LatLng.from_degrees(45.0, 90.0)
        size = s2rst.LatLng.from_degrees(10.0, 20.0)
        r = s2rst.Rect.from_center_size(center, size)
        assert r.center().lat.degrees == pytest.approx(45.0)
        assert r.center().lng.degrees == pytest.approx(90.0)

    def test_lat_lng_getters(self):
        lat = s2rst.R1Interval(0.0, 1.0)
        lng = s2rst.S1Interval(0.0, 1.0)
        r = s2rst.Rect(lat, lng)
        assert r.lat.lo == pytest.approx(0.0)
        assert r.lng.hi == pytest.approx(1.0)

    def test_lo_hi(self):
        ll = s2rst.LatLng.from_degrees(10.0, 20.0)
        r = s2rst.Rect.from_lat_lng(ll)
        lo = r.lo()
        hi = r.hi()
        assert isinstance(lo, s2rst.LatLng)
        assert isinstance(hi, s2rst.LatLng)

    def test_center_size(self):
        center = s2rst.LatLng.from_degrees(45.0, 90.0)
        size = s2rst.LatLng.from_degrees(10.0, 20.0)
        r = s2rst.Rect.from_center_size(center, size)
        s = r.size()
        assert s.lat.degrees == pytest.approx(10.0, abs=1e-10)

    def test_vertex(self):
        center = s2rst.LatLng.from_degrees(45.0, 90.0)
        size = s2rst.LatLng.from_degrees(10.0, 20.0)
        r = s2rst.Rect.from_center_size(center, size)
        v = r.vertex(0)
        assert isinstance(v, s2rst.LatLng)

    def test_area(self):
        r = s2rst.Rect.full()
        assert r.area() == pytest.approx(4 * math.pi)

    def test_contains_lat_lng(self):
        center = s2rst.LatLng.from_degrees(45.0, 90.0)
        size = s2rst.LatLng.from_degrees(10.0, 20.0)
        r = s2rst.Rect.from_center_size(center, size)
        assert r.contains_lat_lng(center)
        assert not r.contains_lat_lng(s2rst.LatLng.from_degrees(0.0, 0.0))

    def test_contains_point(self):
        center = s2rst.LatLng.from_degrees(45.0, 90.0)
        size = s2rst.LatLng.from_degrees(10.0, 20.0)
        r = s2rst.Rect.from_center_size(center, size)
        p = center.to_point()
        assert r.contains_point(p)

    def test_contains_rect(self):
        big = s2rst.Rect.full()
        small = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(45.0, 90.0))
        assert big.contains_rect(small)

    def test_intersects(self):
        a = s2rst.Rect.from_center_size(
            s2rst.LatLng.from_degrees(45.0, 90.0),
            s2rst.LatLng.from_degrees(10.0, 10.0),
        )
        b = s2rst.Rect.from_center_size(
            s2rst.LatLng.from_degrees(47.0, 92.0),
            s2rst.LatLng.from_degrees(10.0, 10.0),
        )
        assert a.intersects(b)

    def test_union(self):
        a = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(10.0, 20.0))
        b = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(30.0, 40.0))
        u = a.union(b)
        assert u.contains_rect(a)
        assert u.contains_rect(b)

    def test_intersection(self):
        a = s2rst.Rect.from_center_size(
            s2rst.LatLng.from_degrees(45.0, 90.0),
            s2rst.LatLng.from_degrees(20.0, 20.0),
        )
        b = s2rst.Rect.from_center_size(
            s2rst.LatLng.from_degrees(50.0, 95.0),
            s2rst.LatLng.from_degrees(20.0, 20.0),
        )
        i = a.intersection(b)
        assert not i.is_empty()

    def test_expanded(self):
        r = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(45.0, 90.0))
        margin = s2rst.LatLng.from_degrees(5.0, 5.0)
        e = r.expanded(margin)
        assert not e.is_point()

    def test_add_point(self):
        r = s2rst.Rect.empty()
        ll = s2rst.LatLng.from_degrees(45.0, 90.0)
        r2 = r.add_point(ll)
        assert r2.contains_lat_lng(ll)

    def test_bounds(self):
        ll = s2rst.LatLng.from_degrees(45.0, 90.0)
        r = s2rst.Rect.from_lat_lng(ll)
        cap = r.cap_bound()
        assert isinstance(cap, s2rst.Cap)
        rb = r.rect_bound()
        assert isinstance(rb, s2rst.Rect)
        cub = r.cell_union_bound()
        assert len(cub) >= 1

    def test_approx_equal(self):
        a = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(45.0, 90.0))
        b = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(45.0, 90.0))
        assert a.approx_equal(b)

    def test_eq(self):
        a = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(45.0, 90.0))
        b = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(45.0, 90.0))
        assert a == b

    def test_repr(self):
        r = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(45.0, 90.0))
        assert "Rect" in repr(r)
