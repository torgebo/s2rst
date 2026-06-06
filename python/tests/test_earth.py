# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for the Earth conversion helpers."""

import math

import s2rst


def test_radius_constants():
    assert s2rst.Earth.RADIUS_METERS > 6_000_000
    assert math.isclose(s2rst.Earth.RADIUS_KM, s2rst.Earth.RADIUS_METERS / 1000)


def test_meters_angle_roundtrip():
    angle = s2rst.Earth.meters_to_angle(100_000.0)
    assert math.isclose(s2rst.Earth.to_meters(angle), 100_000.0, rel_tol=1e-9)
    assert math.isclose(
        s2rst.Earth.radians_to_meters(angle.radians), 100_000.0, rel_tol=1e-9
    )


def test_km_angle_roundtrip():
    angle = s2rst.Earth.km_to_angle(100.0)
    assert math.isclose(s2rst.Earth.to_km(angle), 100.0, rel_tol=1e-9)


def test_area_roundtrip():
    sr = s2rst.Earth.square_km_to_steradians(1000.0)
    assert math.isclose(s2rst.Earth.steradians_to_square_km(sr), 1000.0, rel_tol=1e-9)


def test_distance():
    paris = s2rst.LatLng.from_degrees(48.8566, 2.3522)
    london = s2rst.LatLng.from_degrees(51.5074, -0.1278)
    d_km = s2rst.Earth.distance_km(paris.to_point(), london.to_point())
    assert 320 < d_km < 360  # ~344 km
    d_m = s2rst.Earth.distance_meters(paris.to_point(), london.to_point())
    assert math.isclose(d_m, d_km * 1000, rel_tol=1e-9)
    assert math.isclose(
        s2rst.Earth.distance_km_latlng(paris, london), d_km, rel_tol=1e-9
    )


def test_haversine():
    assert s2rst.Earth.haversine(0.0) == 0.0
    assert math.isclose(s2rst.Earth.haversine(math.pi), 1.0)
    assert math.isclose(s2rst.Earth.haversine(math.pi / 2), 0.5)


def test_initial_bearing():
    origin = s2rst.LatLng.from_degrees(0, 0)
    north = s2rst.LatLng.from_degrees(1, 0)
    east = s2rst.LatLng.from_degrees(0, 1)
    assert math.isclose(
        s2rst.Earth.get_initial_bearing(origin, north).degrees, 0.0, abs_tol=1e-6
    )
    assert math.isclose(
        s2rst.Earth.get_initial_bearing(origin, east).degrees, 90.0, abs_tol=1e-6
    )


def test_chord_angle_distance_round_trip():
    for m in (1.0, 100.0, 1e5, 1e6):
        ca = s2rst.Earth.meters_to_chord_angle(m)
        assert math.isclose(s2rst.Earth.chord_angle_to_meters(ca), m, rel_tol=1e-6)
    for km in (1.0, 50.0, 5000.0):
        ca = s2rst.Earth.km_to_chord_angle(km)
        assert math.isclose(s2rst.Earth.chord_angle_to_km(ca), km, rel_tol=1e-6)


def test_longitude_radians_at_equator():
    m = 100_000.0
    assert math.isclose(
        s2rst.Earth.meters_to_longitude_radians(m, 0.0),
        s2rst.Earth.meters_to_radians(m),
        rel_tol=1e-9,
    )
    # Away from the equator the same distance spans more longitude.
    assert s2rst.Earth.meters_to_longitude_radians(
        m, math.radians(60)
    ) > s2rst.Earth.meters_to_longitude_radians(m, 0.0)
