# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for R2Point, Vector, Matrix3x3, R2Rect."""

import math
import pytest
import s2rst


class TestR2Point:
    def test_new(self):
        p = s2rst.R2Point(1.0, 2.0)
        assert p.x == 1.0
        assert p.y == 2.0

    def test_dot(self):
        a = s2rst.R2Point(1.0, 0.0)
        b = s2rst.R2Point(0.0, 1.0)
        assert a.dot(b) == pytest.approx(0.0)

    def test_cross(self):
        a = s2rst.R2Point(1.0, 0.0)
        b = s2rst.R2Point(0.0, 1.0)
        assert a.cross(b) == pytest.approx(1.0)

    def test_norm(self):
        p = s2rst.R2Point(3.0, 4.0)
        assert p.norm() == pytest.approx(5.0)
        assert p.norm2() == pytest.approx(25.0)

    def test_normalize(self):
        p = s2rst.R2Point(3.0, 4.0)
        n = p.normalize()
        assert n.x == pytest.approx(0.6)
        assert n.y == pytest.approx(0.8)

    def test_ortho(self):
        p = s2rst.R2Point(1.0, 0.0)
        o = p.ortho()
        assert p.dot(o) == pytest.approx(0.0)

    def test_arithmetic(self):
        a = s2rst.R2Point(1.0, 2.0)
        b = s2rst.R2Point(3.0, 4.0)
        s = a + b
        assert s.x == pytest.approx(4.0)
        assert s.y == pytest.approx(6.0)
        d = b - a
        assert d.x == pytest.approx(2.0)
        m = a * 2.0
        assert m.x == pytest.approx(2.0)
        rm = 2.0 * a
        assert rm.x == pytest.approx(2.0)
        div = b / 2.0
        assert div.x == pytest.approx(1.5)
        neg = -a
        assert neg.x == pytest.approx(-1.0)

    def test_indexing(self):
        p = s2rst.R2Point(1.0, 2.0)
        assert len(p) == 2
        assert p[0] == 1.0
        assert p[1] == 2.0
        with pytest.raises(IndexError):
            p[2]

    def test_eq(self):
        a = s2rst.R2Point(1.0, 2.0)
        b = s2rst.R2Point(1.0, 2.0)
        assert a == b


class TestVector:
    def test_new(self):
        v = s2rst.Vector(1.0, 2.0, 3.0)
        assert v.x == 1.0
        assert v.y == 2.0
        assert v.z == 3.0

    def test_dot(self):
        a = s2rst.Vector(1.0, 0.0, 0.0)
        b = s2rst.Vector(0.0, 1.0, 0.0)
        assert a.dot(b) == pytest.approx(0.0)

    def test_cross(self):
        a = s2rst.Vector(1.0, 0.0, 0.0)
        b = s2rst.Vector(0.0, 1.0, 0.0)
        c = a.cross(b)
        assert c.z == pytest.approx(1.0)

    def test_norm(self):
        v = s2rst.Vector(1.0, 2.0, 2.0)
        assert v.norm() == pytest.approx(3.0)

    def test_normalize(self):
        v = s2rst.Vector(0.0, 0.0, 5.0)
        n = v.normalize()
        assert n.z == pytest.approx(1.0)
        assert n.is_unit()

    def test_ortho(self):
        v = s2rst.Vector(1.0, 0.0, 0.0)
        o = v.ortho()
        assert v.dot(o) == pytest.approx(0.0)

    def test_distance(self):
        a = s2rst.Vector(0.0, 0.0, 0.0)
        b = s2rst.Vector(1.0, 0.0, 0.0)
        assert a.distance(b) == pytest.approx(1.0)

    def test_angle(self):
        a = s2rst.Vector(1.0, 0.0, 0.0)
        b = s2rst.Vector(0.0, 1.0, 0.0)
        assert a.angle(b) == pytest.approx(math.pi / 2)

    def test_components(self):
        v = s2rst.Vector(1.0, 2.0, 3.0)
        assert v.largest_abs_component() == 2  # z
        assert v.smallest_abs_component() == 0  # x

    def test_arithmetic(self):
        a = s2rst.Vector(1.0, 2.0, 3.0)
        b = s2rst.Vector(4.0, 5.0, 6.0)
        s = a + b
        assert s.x == pytest.approx(5.0)
        d = b - a
        assert d.x == pytest.approx(3.0)
        m = a * 2.0
        assert m.x == pytest.approx(2.0)
        rm = 2.0 * a
        assert rm.y == pytest.approx(4.0)

    def test_indexing(self):
        v = s2rst.Vector(1.0, 2.0, 3.0)
        assert len(v) == 3
        assert v[0] == 1.0
        assert v[1] == 2.0
        assert v[2] == 3.0

    def test_approx_eq(self):
        a = s2rst.Vector(1.0, 2.0, 3.0)
        b = s2rst.Vector(1.0, 2.0, 3.0)
        assert a.approx_eq(b)


class TestMatrix3x3:
    def test_identity(self):
        m = s2rst.Matrix3x3.identity()
        assert m.get(0, 0) == 1.0
        assert m.get(0, 1) == 0.0
        assert m.get(1, 1) == 1.0

    def test_from_cols(self):
        c0 = s2rst.Vector(1.0, 0.0, 0.0)
        c1 = s2rst.Vector(0.0, 1.0, 0.0)
        c2 = s2rst.Vector(0.0, 0.0, 1.0)
        m = s2rst.Matrix3x3.from_cols(c0, c1, c2)
        assert m.get(0, 0) == 1.0

    def test_col_row(self):
        m = s2rst.Matrix3x3.identity()
        c = m.col(0)
        assert c.x == 1.0
        r = m.row(0)
        assert r.x == 1.0

    def test_transpose(self):
        m = s2rst.Matrix3x3(1, 2, 3, 4, 5, 6, 7, 8, 9)
        t = m.transpose()
        assert t.get(0, 1) == 4.0
        assert t.get(1, 0) == 2.0

    def test_mul_vec(self):
        m = s2rst.Matrix3x3.identity()
        v = s2rst.Vector(1.0, 2.0, 3.0)
        result = m.mul_vec(v)
        assert result.x == pytest.approx(1.0)
        assert result.y == pytest.approx(2.0)

    def test_matmul(self):
        m = s2rst.Matrix3x3.identity()
        v = s2rst.Vector(1.0, 2.0, 3.0)
        result = m @ v
        assert result.x == pytest.approx(1.0)


class TestR2Rect:
    def test_empty(self):
        r = s2rst.R2Rect.empty()
        assert r.is_empty()

    def test_from_points(self):
        lo = s2rst.R2Point(1.0, 2.0)
        hi = s2rst.R2Point(3.0, 4.0)
        r = s2rst.R2Rect.from_points(lo, hi)
        assert r.lo().x == 1.0
        assert r.hi().y == 4.0

    def test_center_size(self):
        center = s2rst.R2Point(2.0, 3.0)
        size = s2rst.R2Point(2.0, 4.0)
        r = s2rst.R2Rect.from_center_size(center, size)
        assert r.center().x == pytest.approx(2.0)
        assert r.size().y == pytest.approx(4.0)

    def test_contains_point(self):
        lo = s2rst.R2Point(0.0, 0.0)
        hi = s2rst.R2Point(2.0, 2.0)
        r = s2rst.R2Rect.from_points(lo, hi)
        assert r.contains_point(s2rst.R2Point(1.0, 1.0))
        assert not r.contains_point(s2rst.R2Point(3.0, 3.0))

    def test_vertices(self):
        lo = s2rst.R2Point(0.0, 0.0)
        hi = s2rst.R2Point(1.0, 1.0)
        r = s2rst.R2Rect.from_points(lo, hi)
        vs = r.vertices()
        assert len(vs) == 4

    def test_union_intersection(self):
        r1 = s2rst.R2Rect.from_points(s2rst.R2Point(0.0, 0.0), s2rst.R2Point(2.0, 2.0))
        r2 = s2rst.R2Rect.from_points(s2rst.R2Point(1.0, 1.0), s2rst.R2Point(3.0, 3.0))
        u = r1.union(r2)
        assert u.lo().x == pytest.approx(0.0)
        assert u.hi().x == pytest.approx(3.0)
        i = r1.intersection(r2)
        assert i.lo().x == pytest.approx(1.0)
        assert i.hi().x == pytest.approx(2.0)

    def test_expanded(self):
        r = s2rst.R2Rect.from_points(s2rst.R2Point(1.0, 1.0), s2rst.R2Point(2.0, 2.0))
        e = r.expanded(s2rst.R2Point(0.5, 0.5))
        assert e.lo().x == pytest.approx(0.5)

    def test_approx_eq(self):
        r = s2rst.R2Rect.from_points(s2rst.R2Point(0.0, 0.0), s2rst.R2Point(1.0, 1.0))
        assert r.approx_eq(r, max_error=1e-15)
