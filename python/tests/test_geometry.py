# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for Polyline, Loop, Polygon."""

import math
import pytest
import s2rst


def make_regular_points(center_lat, center_lng, radius_deg, n):
    """Helper: make n points in a circle around a center."""
    center = s2rst.LatLng.from_degrees(center_lat, center_lng).to_point()
    angle = s2rst.Angle.from_degrees(radius_deg)
    loop = s2rst.Loop.make_regular(center, angle, n)
    return [loop.vertex(i) for i in range(loop.num_vertices())]


class TestPolyline:
    def test_new(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        pl = s2rst.Polyline(pts)
        assert pl.num_vertices() == 3

    def test_from_lat_lngs(self):
        lls = [
            s2rst.LatLng.from_degrees(0.0, 0.0),
            s2rst.LatLng.from_degrees(0.0, 90.0),
            s2rst.LatLng.from_degrees(90.0, 0.0),
        ]
        pl = s2rst.Polyline.from_lat_lngs(lls)
        assert pl.num_vertices() == 3

    def test_vertex(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        v = pl.vertex(0)
        assert v.approx_eq(pts[0])

    def test_vertices(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        vs = pl.vertices()
        assert len(vs) == 2

    def test_length(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        assert pl.length().degrees == pytest.approx(90.0)

    def test_centroid(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        c = pl.centroid()
        assert isinstance(c, s2rst.S2Point)

    def test_interpolate(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        p, idx = pl.interpolate(0.0)
        assert p.approx_eq(pts[0])
        p2, idx2 = pl.interpolate(1.0)
        assert p2.approx_eq(pts[1])

    def test_project(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        closest, next_idx = pl.project(pts[0])
        assert closest.approx_eq(pts[0])

    def test_reverse(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        pl.reverse()
        assert pl.vertex(0).approx_eq(pts[1])

    def test_validate(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        assert pl.validate() is None

    def test_equal(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        a = s2rst.Polyline(pts)
        b = s2rst.Polyline(pts)
        assert a.equal(b)
        assert a == b

    def test_bounds(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        assert isinstance(pl.cap_bound(), s2rst.Cap)
        assert isinstance(pl.rect_bound(), s2rst.Rect)

    def test_len_getitem(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pl = s2rst.Polyline(pts)
        assert len(pl) == 2
        assert pl[0].approx_eq(pts[0])
        with pytest.raises(IndexError):
            pl[5]


class TestLoop:
    def test_empty(self):
        loop = s2rst.Loop.empty()
        assert loop.is_empty_loop()
        assert not loop.is_full_loop()

    def test_full(self):
        loop = s2rst.Loop.full()
        assert loop.is_full_loop()
        assert not loop.is_empty_loop()

    def test_new(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        assert loop.num_vertices() == 4

    def test_from_cell(self):
        cell = s2rst.Cell(s2rst.CellId.from_face(0))
        loop = s2rst.Loop.from_cell(cell)
        assert loop.num_vertices() == 4

    def test_make_regular(self):
        center = s2rst.S2Point(0.0, 0.0, 1.0)
        radius = s2rst.Angle.from_degrees(10.0)
        loop = s2rst.Loop.make_regular(center, radius, 100)
        assert loop.num_vertices() == 100

    def test_vertex_vertices(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        v = loop.vertex(0)
        assert isinstance(v, s2rst.S2Point)
        vs = loop.vertices()
        assert len(vs) == 4

    def test_depth(self):
        loop = s2rst.Loop.empty()
        assert loop.depth() == 0

    def test_is_normalized(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        assert loop.is_normalized()

    def test_sign(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        assert loop.sign() in (1, -1)

    def test_area(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        loop = s2rst.Loop(pts)
        assert loop.area() > 0

    def test_centroid(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        c = loop.centroid()
        assert isinstance(c, s2rst.S2Point)

    def test_contains_point(self):
        center = s2rst.LatLng.from_degrees(0.0, 0.0).to_point()
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        loop = s2rst.Loop(pts)
        assert loop.contains_point(center)
        far = s2rst.LatLng.from_degrees(89.0, 0.0).to_point()
        assert not loop.contains_point(far)

    def test_contains_origin(self):
        loop = s2rst.Loop.full()
        assert loop.contains_origin()

    def test_contains_loop(self):
        big_pts = make_regular_points(0.0, 0.0, 20.0, 100)
        small_pts = make_regular_points(0.0, 0.0, 5.0, 100)
        big = s2rst.Loop(big_pts)
        small = s2rst.Loop(small_pts)
        assert big.contains_loop(small)
        assert not small.contains_loop(big)

    def test_intersects_loop(self):
        a_pts = make_regular_points(0.0, 0.0, 20.0, 100)
        b_pts = make_regular_points(10.0, 0.0, 20.0, 100)
        a = s2rst.Loop(a_pts)
        b = s2rst.Loop(b_pts)
        assert a.intersects_loop(b)

    def test_normalize(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        loop.normalize()
        assert loop.is_normalized()

    def test_invert(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        area_before = loop.area()
        loop.invert()
        area_after = loop.area()
        assert area_before + area_after == pytest.approx(4 * math.pi)

    def test_validate(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        assert loop.validate() is None

    def test_equal(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        a = s2rst.Loop(pts)
        b = s2rst.Loop(pts)
        assert a.equal(b)

    def test_bounds(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        assert isinstance(loop.cap_bound(), s2rst.Cap)
        assert isinstance(loop.rect_bound(), s2rst.Rect)
        assert len(loop.cell_union_bound()) >= 1

    def test_len_getitem(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        assert len(loop) == 4
        v = loop[0]
        assert isinstance(v, s2rst.S2Point)
        with pytest.raises(IndexError):
            loop[100]

    def test_repr(self):
        loop = s2rst.Loop.empty()
        assert "empty" in repr(loop)
        loop = s2rst.Loop.full()
        assert "full" in repr(loop)
        pts = make_regular_points(0.0, 0.0, 10.0, 4)
        loop = s2rst.Loop(pts)
        assert "4 vertices" in repr(loop)


class TestPolygon:
    def test_empty(self):
        p = s2rst.Polygon.empty()
        assert p.is_empty_polygon()
        assert p.num_loops() == 0

    def test_full(self):
        p = s2rst.Polygon.full()
        assert p.is_full_polygon()
        assert p.num_loops() == 1

    def test_from_loops(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        loop = s2rst.Loop(pts)
        poly = s2rst.Polygon([loop])
        assert poly.num_loops() == 1
        assert poly.num_vertices() == 100

    def test_from_cell(self):
        cell = s2rst.Cell(s2rst.CellId.from_face(0))
        poly = s2rst.Polygon.from_cell(cell)
        assert poly.num_loops() == 1

    def test_loop_(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        loop = s2rst.Loop(pts)
        poly = s2rst.Polygon([loop])
        loop = poly.loop_(0)
        assert isinstance(loop, s2rst.Loop)
        assert loop.num_vertices() == 100

    def test_loops(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        loop = s2rst.Loop(pts)
        poly = s2rst.Polygon([loop])
        ls = poly.loops()
        assert len(ls) == 1

    def test_has_holes(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        assert not poly.has_holes()

    def test_area(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        assert poly.area() > 0

    def test_centroid(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        c = poly.centroid()
        assert isinstance(c, s2rst.S2Point)

    def test_contains_point(self):
        center = s2rst.LatLng.from_degrees(0.0, 0.0).to_point()
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        assert poly.contains_point(center)

    def test_contains_polygon(self):
        big_pts = make_regular_points(0.0, 0.0, 20.0, 100)
        small_pts = make_regular_points(0.0, 0.0, 5.0, 100)
        big = s2rst.Polygon([s2rst.Loop(big_pts)])
        small = s2rst.Polygon([s2rst.Loop(small_pts)])
        assert big.contains_polygon(small)

    def test_intersects_polygon(self):
        a_pts = make_regular_points(0.0, 0.0, 20.0, 100)
        b_pts = make_regular_points(10.0, 0.0, 20.0, 100)
        a = s2rst.Polygon([s2rst.Loop(a_pts)])
        b = s2rst.Polygon([s2rst.Loop(b_pts)])
        assert a.intersects_polygon(b)

    def test_complement(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        comp = poly.complement()
        assert comp.num_loops() >= 1
        assert comp.area() + poly.area() == pytest.approx(4 * math.pi, abs=1e-6)

    def test_union(self):
        a_pts = make_regular_points(0.0, 0.0, 10.0, 100)
        b_pts = make_regular_points(5.0, 0.0, 10.0, 100)
        a = s2rst.Polygon([s2rst.Loop(a_pts)])
        b = s2rst.Polygon([s2rst.Loop(b_pts)])
        u = a.union(b)
        assert u.area() > a.area()

    def test_intersection(self):
        a_pts = make_regular_points(0.0, 0.0, 10.0, 100)
        b_pts = make_regular_points(5.0, 0.0, 10.0, 100)
        a = s2rst.Polygon([s2rst.Loop(a_pts)])
        b = s2rst.Polygon([s2rst.Loop(b_pts)])
        i = a.intersection(b)
        assert i.area() < a.area()
        assert i.area() > 0

    def test_difference(self):
        a_pts = make_regular_points(0.0, 0.0, 10.0, 100)
        b_pts = make_regular_points(5.0, 0.0, 10.0, 100)
        a = s2rst.Polygon([s2rst.Loop(a_pts)])
        b = s2rst.Polygon([s2rst.Loop(b_pts)])
        d = a.difference(b)
        assert d.area() < a.area()

    def test_invert(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        area_before = poly.area()
        poly.invert()
        assert poly.area() + area_before == pytest.approx(4 * math.pi, abs=1e-6)

    def test_validate(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        assert poly.validate() is None

    def test_equals(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        a = s2rst.Polygon([s2rst.Loop(pts)])
        b = s2rst.Polygon([s2rst.Loop(pts)])
        assert a.equals(b)

    def test_bounds(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        assert isinstance(poly.cap_bound(), s2rst.Cap)
        assert isinstance(poly.rect_bound(), s2rst.Rect)

    def test_len_getitem(self):
        pts = make_regular_points(0.0, 0.0, 10.0, 100)
        poly = s2rst.Polygon([s2rst.Loop(pts)])
        assert len(poly) == 1
        loop = poly[0]
        assert isinstance(loop, s2rst.Loop)
        with pytest.raises(IndexError):
            poly[5]

    def test_repr(self):
        poly = s2rst.Polygon.empty()
        assert "empty" in repr(poly)
        poly = s2rst.Polygon.full()
        assert "full" in repr(poly)

    def test_destructive_union(self):
        a_pts = make_regular_points(0.0, 0.0, 10.0, 100)
        b_pts = make_regular_points(30.0, 0.0, 10.0, 100)
        a = s2rst.Polygon([s2rst.Loop(a_pts)])
        b = s2rst.Polygon([s2rst.Loop(b_pts)])
        u = s2rst.Polygon.destructive_union([a, b])
        assert u.num_loops() == 2
