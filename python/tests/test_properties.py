# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Property-based tests using hypothesis."""

import math
import pytest

from hypothesis import given, assume
from hypothesis import strategies as st

import s2rst


# --- Strategies ---

angles_deg = st.floats(
    min_value=-360.0, max_value=360.0, allow_nan=False, allow_infinity=False
)
latitudes = st.floats(
    min_value=-90.0, max_value=90.0, allow_nan=False, allow_infinity=False
)
longitudes = st.floats(
    min_value=-180.0, max_value=180.0, allow_nan=False, allow_infinity=False
)
unit_components = st.floats(
    min_value=-1.0, max_value=1.0, allow_nan=False, allow_infinity=False
)
levels = st.integers(min_value=0, max_value=30)
faces = st.integers(min_value=0, max_value=5)


class TestAngleProperties:
    @given(deg=angles_deg)
    def test_radians_degrees_roundtrip(self, deg):
        a = s2rst.Angle.from_degrees(deg)
        assert a.degrees == pytest.approx(deg, abs=1e-10)

    @given(deg=angles_deg)
    def test_negation(self, deg):
        a = s2rst.Angle.from_degrees(deg)
        assert (-a).degrees == pytest.approx(-deg, abs=1e-10)

    @given(a=angles_deg, b=angles_deg)
    def test_addition_commutative(self, a, b):
        x = s2rst.Angle.from_degrees(a)
        y = s2rst.Angle.from_degrees(b)
        assert (x + y).radians == pytest.approx((y + x).radians, abs=1e-10)


class TestLatLngProperties:
    @given(lat=latitudes, lng=longitudes)
    def test_roundtrip(self, lat, lng):
        ll = s2rst.LatLng.from_degrees(lat, lng)
        p = ll.to_point()
        ll2 = s2rst.LatLng.from_point(p)
        assert ll2.lat.degrees == pytest.approx(lat, abs=1e-8)
        assert ll2.lng.degrees == pytest.approx(lng, abs=1e-8)

    @given(lat=latitudes, lng=longitudes)
    def test_valid(self, lat, lng):
        ll = s2rst.LatLng.from_degrees(lat, lng)
        assert ll.is_valid()


class TestCellIdProperties:
    @given(face=faces)
    def test_face_roundtrip(self, face):
        cid = s2rst.CellId.from_face(face)
        assert cid.face() == face
        assert cid.level() == 0
        assert cid.is_valid()

    @given(lat=latitudes, lng=longitudes)
    def test_from_latlng_valid(self, lat, lng):
        ll = s2rst.LatLng.from_degrees(lat, lng)
        cid = s2rst.CellId.from_lat_lng(ll)
        assert cid.is_valid()
        assert cid.is_leaf()

    @given(lat=latitudes, lng=longitudes, level=levels)
    def test_parent_contains_child(self, lat, lng, level):
        ll = s2rst.LatLng.from_degrees(lat, lng)
        leaf = s2rst.CellId.from_lat_lng(ll)
        parent = leaf.parent_at_level(level)
        assert parent.contains(leaf)

    @given(face=faces)
    def test_token_roundtrip(self, face):
        cid = s2rst.CellId.from_face(face)
        token = cid.to_token()
        cid2 = s2rst.CellId.from_token(token)
        assert cid == cid2

    @given(face=faces)
    def test_next_prev_inverse(self, face):
        cid = s2rst.CellId.from_face(face)
        assert cid.next().prev() == cid


class TestCapProperties:
    @given(
        lat=latitudes, lng=longitudes, radius=st.floats(min_value=0.0, max_value=180.0)
    )
    def test_contains_center(self, lat, lng, radius):
        assume(not math.isnan(radius))
        center = s2rst.LatLng.from_degrees(lat, lng).to_point()
        angle = s2rst.Angle.from_degrees(radius)
        cap = s2rst.Cap.from_center_angle(center, angle)
        assert cap.contains_point(center)
