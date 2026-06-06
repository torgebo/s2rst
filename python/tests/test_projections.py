# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for map projections (PlateCarreeProjection, MercatorProjection)."""

import math

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


# --- Strategies ---

# Stay strictly inside the valid ranges so the round-trip is well-defined
# (the Mercator y-axis blows up at the poles, and lng wraps at ±180).
round_trip_lats = st.floats(
    min_value=-85.0, max_value=85.0, allow_nan=False, allow_infinity=False
)
# Exclude the antimeridian: +180 and -180 are the same meridian, so a
# round-trip there legitimately flips the sign.
round_trip_lngs = st.floats(
    min_value=-179.9, max_value=179.9, allow_nan=False, allow_infinity=False
)


class TestPlateCarree:
    def test_from_lat_lng_degrees(self):
        # In degrees mode, lng maps to x and lat maps to y, both unscaled.
        proj = s2rst.PlateCarreeProjection(180)
        p = proj.from_lat_lng(s2rst.LatLng.from_degrees(0, 90))
        assert p.x == pytest.approx(90.0, abs=1e-9)
        assert p.y == pytest.approx(0.0, abs=1e-9)

    def test_from_lat_lng_lat_to_y(self):
        proj = s2rst.PlateCarreeProjection(180)
        p = proj.from_lat_lng(s2rst.LatLng.from_degrees(45, 0))
        assert p.x == pytest.approx(0.0, abs=1e-9)
        assert p.y == pytest.approx(45.0, abs=1e-9)

    def test_origin(self):
        proj = s2rst.PlateCarreeProjection(180)
        p = proj.from_lat_lng(s2rst.LatLng.from_degrees(0, 0))
        assert p.x == pytest.approx(0.0, abs=1e-9)
        assert p.y == pytest.approx(0.0, abs=1e-9)

    def test_to_lat_lng_inverse(self):
        proj = s2rst.PlateCarreeProjection(180)
        ll = proj.to_lat_lng(s2rst.R2Point(90.0, 45.0))
        assert ll.lat.degrees == pytest.approx(45.0, abs=1e-9)
        assert ll.lng.degrees == pytest.approx(90.0, abs=1e-9)

    def test_wrap_distance(self):
        proj = s2rst.PlateCarreeProjection(180)
        wrap = proj.wrap_distance()
        # x wraps every 360 degrees; y does not wrap.
        assert wrap.x == pytest.approx(360.0, abs=1e-9)
        assert wrap.y == pytest.approx(0.0, abs=1e-9)

    def test_interpolate_midpoint(self):
        proj = s2rst.PlateCarreeProjection(180)
        a = s2rst.R2Point(0.0, 0.0)
        b = s2rst.R2Point(10.0, 20.0)
        mid = proj.interpolate(0.5, a, b)
        assert mid.x == pytest.approx(5.0, abs=1e-9)
        assert mid.y == pytest.approx(10.0, abs=1e-9)

    def test_project_unproject_roundtrip(self):
        proj = s2rst.PlateCarreeProjection(180)
        ll = s2rst.LatLng.from_degrees(30, 60)
        point = ll.to_point()
        projected = proj.project(point)
        recovered = proj.unproject(projected)
        back = s2rst.LatLng.from_point(recovered)
        assert back.lat.degrees == pytest.approx(30.0, abs=1e-6)
        assert back.lng.degrees == pytest.approx(60.0, abs=1e-6)


class TestMercator:
    def test_origin(self):
        # The equator/prime-meridian maps to the origin.
        proj = s2rst.MercatorProjection(180)
        p = proj.from_lat_lng(s2rst.LatLng.from_degrees(0, 0))
        assert p.x == pytest.approx(0.0, abs=1e-9)
        assert p.y == pytest.approx(0.0, abs=1e-9)

    def test_lng_linear(self):
        # Longitude maps linearly to x just like Plate Carree.
        proj = s2rst.MercatorProjection(180)
        p = proj.from_lat_lng(s2rst.LatLng.from_degrees(0, 90))
        assert p.x == pytest.approx(90.0, abs=1e-9)
        assert p.y == pytest.approx(0.0, abs=1e-9)

    def test_lat_nonlinear_positive(self):
        # Positive latitude gives positive (and stretched) y.
        proj = s2rst.MercatorProjection(180)
        p = proj.from_lat_lng(s2rst.LatLng.from_degrees(45, 0))
        assert p.x == pytest.approx(0.0, abs=1e-9)
        assert p.y > 45.0

    def test_wrap_distance(self):
        proj = s2rst.MercatorProjection(180)
        wrap = proj.wrap_distance()
        assert wrap.x == pytest.approx(360.0, abs=1e-9)
        assert wrap.y == pytest.approx(0.0, abs=1e-9)


@given(lat=round_trip_lats, lng=round_trip_lngs)
def test_plate_carree_latlng_roundtrip(lat, lng):
    proj = s2rst.PlateCarreeProjection(180)
    ll = s2rst.LatLng.from_degrees(lat, lng)
    back = proj.to_lat_lng(proj.from_lat_lng(ll))
    assert back.lat.degrees == pytest.approx(lat, abs=1e-6)
    assert back.lng.degrees == pytest.approx(lng, abs=1e-6)


@given(lat=round_trip_lats, lng=round_trip_lngs)
def test_mercator_latlng_roundtrip(lat, lng):
    proj = s2rst.MercatorProjection(180)
    ll = s2rst.LatLng.from_degrees(lat, lng)
    back = proj.to_lat_lng(proj.from_lat_lng(ll))
    assert back.lat.degrees == pytest.approx(lat, abs=1e-6)
    assert back.lng.degrees == pytest.approx(lng, abs=1e-6)


@given(lat=round_trip_lats, lng=round_trip_lngs)
def test_plate_carree_project_unproject_roundtrip(lat, lng):
    proj = s2rst.PlateCarreeProjection(180)
    point = s2rst.LatLng.from_degrees(lat, lng).to_point()
    back = s2rst.LatLng.from_point(proj.unproject(proj.project(point)))
    assert back.lat.degrees == pytest.approx(lat, abs=1e-6)
    # Longitude is undefined at the poles; everywhere else it round-trips.
    if abs(abs(lat) - 90.0) > 1e-9:
        assert math.cos(math.radians(back.lng.degrees - lng)) == pytest.approx(
            1.0, abs=1e-9
        )
