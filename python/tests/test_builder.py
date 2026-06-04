# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for S2Builder."""

import s2rst


def _pt(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def test_len_tracks_inputs():
    builder = s2rst.S2Builder()
    assert len(builder) == 0
    builder.add_edge(_pt(0, 0), _pt(1, 1))
    builder.add_loop_from_points([_pt(0, 0), _pt(0, 5), _pt(5, 0)])
    assert len(builder) == 2


def test_build_polygon_from_loop():
    builder = s2rst.S2Builder()
    builder.add_loop_from_points([_pt(0, 0), _pt(0, 5), _pt(5, 5), _pt(5, 0)])
    poly = builder.build_polygon()
    assert poly.num_loops() == 1
    assert poly.area() > 0


def test_build_polyline_from_edges():
    builder = s2rst.S2Builder()
    builder.add_edge(_pt(0, 0), _pt(1, 1))
    builder.add_edge(_pt(1, 1), _pt(2, 2))
    pl = builder.build_polyline()
    assert len(pl) == 3


def test_build_polyline_from_points():
    builder = s2rst.S2Builder()
    builder.add_polyline_from_points([_pt(0, 0), _pt(1, 0), _pt(2, 0)])
    assert len(builder.build_polyline()) == 3


def test_snap_level_produces_valid_polygon():
    builder = s2rst.S2Builder(snap_level=12)
    builder.add_loop_from_points([_pt(0, 0), _pt(0, 5), _pt(5, 5), _pt(5, 0)])
    poly = builder.build_polygon()
    assert poly.num_loops() == 1
    assert poly.area() > 0
