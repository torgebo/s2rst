# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for the extra region types: PointRegion, RegionUnion,
RegionIntersection."""

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _cap(lat, lng, radius_deg):
    return s2rst.Cap.from_center_angle(
        _point(lat, lng), s2rst.Angle.from_degrees(radius_deg)
    )


# ---------------------------------------------------------------------------
# PointRegion
# ---------------------------------------------------------------------------


class TestPointRegion:
    def test_contains_its_own_point(self):
        p = _point(10, 20)
        region = s2rst.PointRegion(p)
        assert region.contains_point(p)

    def test_does_not_contain_other_point(self):
        region = s2rst.PointRegion(_point(10, 20))
        assert not region.contains_point(_point(10, 21))

    def test_point_accessor_roundtrips(self):
        p = _point(-30, 45)
        assert s2rst.PointRegion(p).point().approx_eq(p)

    def test_cap_bound_is_near_zero_radius_and_contains_point(self):
        p = _point(5, 5)
        cap = s2rst.PointRegion(p).cap_bound()
        assert cap.contains_point(p)
        # A single-point cap has (essentially) zero radius / area.
        assert cap.angle_radius().degrees == pytest.approx(0.0, abs=1e-9)
        assert cap.area() == pytest.approx(0.0, abs=1e-12)

    def test_rect_bound_contains_point(self):
        p = _point(12, 34)
        rect = s2rst.PointRegion(p).rect_bound()
        assert rect.contains_point(p)

    def test_cell_union_bound_nonempty(self):
        assert len(s2rst.PointRegion(_point(0, 0)).cell_union_bound()) >= 1

    def test_repr(self):
        assert "PointRegion" in repr(s2rst.PointRegion(_point(0, 0)))

    @given(
        lat=st.floats(min_value=-89.0, max_value=89.0),
        lng=st.floats(min_value=-179.0, max_value=179.0),
    )
    def test_always_contains_own_point(self, lat, lng):
        p = _point(lat, lng)
        assert s2rst.PointRegion(p).contains_point(p)


# ---------------------------------------------------------------------------
# RegionUnion
# ---------------------------------------------------------------------------


class TestRegionUnion:
    def test_empty_union(self):
        u = s2rst.RegionUnion()
        assert len(u) == 0
        assert u.is_empty()
        # An empty union has an empty bounding cap.
        assert u.cap_bound().is_empty()

    def test_add_increases_len(self):
        u = s2rst.RegionUnion()
        u.add(_cap(0, 0, 5))
        assert len(u) == 1
        u.add(_cap(0, 90, 5))
        assert len(u) == 2

    def test_contains_point_in_either_cap(self):
        u = s2rst.RegionUnion()
        u.add(_cap(0, 0, 5))
        u.add(_cap(0, 90, 5))
        # Points near either center are contained.
        assert u.contains_point(_point(0, 0))
        assert u.contains_point(_point(0, 90))
        # A point far from both is not.
        assert not u.contains_point(_point(45, 45))

    def test_accepts_point_region_member(self):
        p = _point(7, 7)
        u = s2rst.RegionUnion()
        u.add(s2rst.PointRegion(p))
        assert u.contains_point(p)

    def test_accepts_rect_member(self):
        rect = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(0, 0)).expanded(
            s2rst.LatLng.from_degrees(5, 5)
        )
        u = s2rst.RegionUnion()
        u.add(rect)
        assert u.contains_point(_point(0, 0))

    def test_accepts_cell_union_member(self):
        cu = s2rst.CellUnion.from_cell_ids([s2rst.CellId.from_point(_point(0, 0))])
        u = s2rst.RegionUnion()
        u.add(cu)
        assert u.contains_point(_point(0, 0))

    def test_add_rejects_non_region(self):
        u = s2rst.RegionUnion()
        with pytest.raises(TypeError):
            u.add(object())

    def test_rect_bound_covers_member_centers(self):
        u = s2rst.RegionUnion()
        u.add(_cap(0, 0, 5))
        u.add(_cap(0, 90, 5))
        rect = u.rect_bound()
        assert rect.contains_point(_point(0, 0))
        assert rect.contains_point(_point(0, 90))

    def test_repr(self):
        u = s2rst.RegionUnion()
        u.add(_cap(0, 0, 5))
        assert "RegionUnion" in repr(u)


# ---------------------------------------------------------------------------
# RegionIntersection
# ---------------------------------------------------------------------------


class TestRegionIntersection:
    def test_empty_intersection_covers_sphere(self):
        # An intersection of no regions covers the entire sphere.
        i = s2rst.RegionIntersection()
        assert len(i) == 0
        assert i.rect_bound().is_full()
        assert i.contains_point(_point(0, 0))
        assert i.contains_point(_point(80, -120))

    def test_two_overlapping_caps(self):
        # Two large overlapping caps centered 45 deg apart; their lens overlaps
        # around the midpoint.
        c1 = _cap(0, 0, 40)
        c2 = _cap(0, 45, 40)
        i = s2rst.RegionIntersection([c1, c2])
        assert len(i) == 2
        # The midpoint lies inside both caps.
        midpoint = _point(0, 22.5)
        assert c1.contains_point(midpoint)
        assert c2.contains_point(midpoint)
        assert i.contains_point(midpoint)

    def test_point_in_only_one_cap_excluded(self):
        c1 = _cap(0, 0, 40)
        c2 = _cap(0, 45, 40)
        i = s2rst.RegionIntersection([c1, c2])
        # A point near c1's center but outside c2.
        only_c1 = _point(0, -30)
        assert c1.contains_point(only_c1)
        assert not c2.contains_point(only_c1)
        assert not i.contains_point(only_c1)

    def test_accepts_mixed_region_types(self):
        # Cap intersected with a PointRegion at the cap center: the point is in
        # both, so it is contained.
        center = _point(0, 0)
        i = s2rst.RegionIntersection([_cap(0, 0, 10), s2rst.PointRegion(center)])
        assert i.contains_point(center)
        # Any other point fails the PointRegion's exact-match test.
        assert not i.contains_point(_point(1, 1))

    def test_constructor_rejects_non_region(self):
        with pytest.raises(TypeError):
            s2rst.RegionIntersection([object()])

    def test_repr(self):
        i = s2rst.RegionIntersection([_cap(0, 0, 5)])
        assert "RegionIntersection" in repr(i)
