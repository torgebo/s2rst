# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for R1Interval and S1Interval."""

import math
import pytest
import s2rst


class TestR1Interval:
    def test_new(self):
        i = s2rst.R1Interval(1.0, 3.0)
        assert i.lo == 1.0
        assert i.hi == 3.0

    def test_empty(self):
        i = s2rst.R1Interval.empty()
        assert i.is_empty()

    def test_from_point(self):
        i = s2rst.R1Interval.from_point(2.0)
        assert i.lo == 2.0
        assert i.hi == 2.0

    def test_from_point_pair(self):
        i = s2rst.R1Interval.from_point_pair(5.0, 1.0)
        assert i.lo == 1.0
        assert i.hi == 5.0

    def test_center_length(self):
        i = s2rst.R1Interval(1.0, 3.0)
        assert i.center() == pytest.approx(2.0)
        assert i.length() == pytest.approx(2.0)

    def test_contains(self):
        i = s2rst.R1Interval(1.0, 3.0)
        assert i.contains(2.0)
        assert not i.contains(4.0)

    def test_interior_contains(self):
        i = s2rst.R1Interval(1.0, 3.0)
        assert i.interior_contains(2.0)
        assert not i.interior_contains(1.0)  # boundary

    def test_contains_interval(self):
        i = s2rst.R1Interval(1.0, 5.0)
        j = s2rst.R1Interval(2.0, 4.0)
        assert i.contains_interval(j)
        assert not j.contains_interval(i)

    def test_intersects(self):
        i = s2rst.R1Interval(1.0, 3.0)
        j = s2rst.R1Interval(2.0, 4.0)
        assert i.intersects(j)

    def test_union(self):
        i = s2rst.R1Interval(1.0, 3.0)
        j = s2rst.R1Interval(5.0, 7.0)
        u = i.union(j)
        assert u.lo == 1.0
        assert u.hi == 7.0

    def test_intersection(self):
        i = s2rst.R1Interval(1.0, 5.0)
        j = s2rst.R1Interval(3.0, 7.0)
        x = i.intersection(j)
        assert x.lo == pytest.approx(3.0)
        assert x.hi == pytest.approx(5.0)

    def test_add_point(self):
        i = s2rst.R1Interval(1.0, 3.0)
        i2 = i.add_point(5.0)
        assert i2.hi == 5.0

    def test_expanded(self):
        i = s2rst.R1Interval(1.0, 3.0)
        i2 = i.expanded(1.0)
        assert i2.lo == pytest.approx(0.0)
        assert i2.hi == pytest.approx(4.0)

    def test_project(self):
        i = s2rst.R1Interval(1.0, 3.0)
        assert i.project(0.0) == pytest.approx(1.0)
        assert i.project(2.0) == pytest.approx(2.0)
        assert i.project(5.0) == pytest.approx(3.0)

    def test_approx_eq(self):
        i = s2rst.R1Interval(1.0, 3.0)
        j = s2rst.R1Interval(1.0, 3.0)
        assert i.approx_eq(j, max_error=1e-15)

    def test_eq(self):
        i = s2rst.R1Interval(1.0, 3.0)
        j = s2rst.R1Interval(1.0, 3.0)
        assert i == j

    def test_len_getitem(self):
        i = s2rst.R1Interval(1.0, 3.0)
        assert len(i) == 2
        assert i[0] == 1.0
        assert i[1] == 3.0
        with pytest.raises(IndexError):
            i[2]


class TestS1Interval:
    def test_new(self):
        i = s2rst.S1Interval(0.0, math.pi)
        assert i.lo == 0.0
        assert i.hi == pytest.approx(math.pi)

    def test_empty_full(self):
        e = s2rst.S1Interval.empty()
        assert e.is_empty()
        f = s2rst.S1Interval.full()
        assert f.is_full()

    def test_from_point(self):
        i = s2rst.S1Interval.from_point(1.0)
        assert i.lo == 1.0
        assert i.hi == 1.0

    def test_from_point_pair(self):
        i = s2rst.S1Interval.from_point_pair(1.0, -1.0)
        assert i.contains(0.0)

    def test_center_length(self):
        i = s2rst.S1Interval(0.0, math.pi)
        assert i.center() == pytest.approx(math.pi / 2)
        assert i.length() == pytest.approx(math.pi)

    def test_is_inverted(self):
        i = s2rst.S1Interval(math.pi / 2, -math.pi / 2)
        assert i.is_inverted()

    def test_complement(self):
        i = s2rst.S1Interval(0.0, math.pi)
        c = i.complement()
        assert c.lo == pytest.approx(math.pi)
        assert c.hi == pytest.approx(0.0)

    def test_contains(self):
        i = s2rst.S1Interval(0.0, math.pi)
        assert i.contains(1.0)
        assert not i.contains(-1.0)

    def test_contains_interval(self):
        i = s2rst.S1Interval(0.0, math.pi)
        j = s2rst.S1Interval(0.5, 1.5)
        assert i.contains_interval(j)

    def test_intersects(self):
        i = s2rst.S1Interval(0.0, 1.0)
        j = s2rst.S1Interval(0.5, 2.0)
        assert i.intersects(j)

    def test_union(self):
        i = s2rst.S1Interval(0.0, 1.0)
        j = s2rst.S1Interval(2.0, 3.0)
        u = i.union(j)
        assert u.contains(1.5)

    def test_intersection(self):
        i = s2rst.S1Interval(0.0, 2.0)
        j = s2rst.S1Interval(1.0, 3.0)
        x = i.intersection(j)
        assert x.lo == pytest.approx(1.0)
        assert x.hi == pytest.approx(2.0)

    def test_expanded(self):
        i = s2rst.S1Interval(0.0, 1.0)
        e = i.expanded(0.5)
        assert e.lo == pytest.approx(-0.5)
        assert e.hi == pytest.approx(1.5)

    def test_approx_eq(self):
        i = s2rst.S1Interval(0.0, 1.0)
        j = s2rst.S1Interval(0.0, 1.0)
        assert i.approx_eq(j, max_error=1e-15)

    def test_len_getitem(self):
        i = s2rst.S1Interval(0.0, 1.0)
        assert len(i) == 2
        assert i[0] == 0.0
        assert i[1] == 1.0
