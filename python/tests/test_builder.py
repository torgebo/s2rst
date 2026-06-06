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


def _square():
    return [_pt(0, 0), _pt(0, 2), _pt(2, 2), _pt(2, 0)]


def test_build_lax_polygon():
    b = s2rst.S2Builder()
    b.add_loop_from_points(_square())
    assert b.build_lax_polygon().num_loops() == 1


def test_build_points():
    b = s2rst.S2Builder()
    for p in _square():
        b.add_point(p)
    assert len(b) == 4
    pts = b.build_points()
    assert len(pts) == 4
    assert all(isinstance(p, s2rst.S2Point) for p in pts)


def test_snap_function_e6():
    b = s2rst.S2Builder(snap_function=s2rst.IntLatLngSnapFunction(6))
    b.add_loop_from_points(_square())
    poly = b.build_polygon()
    assert poly.num_loops() == 1
    # Every vertex lands on the E6 (micro-degree) grid, within float tolerance
    # of the Point->LatLng round-trip.
    for loop_ in poly:
        for i in range(len(loop_)):
            ll = s2rst.LatLng.from_point(loop_.vertex(i))
            for deg in (ll.lat.degrees, ll.lng.degrees):
                micro = deg * 1e6
                assert abs(micro - round(micro)) < 0.02


def test_snap_level_backcompat():
    b = s2rst.S2Builder(snap_level=20)
    b.add_loop_from_points(_square())
    assert b.build_polygon().num_loops() == 1


def test_cell_id_snap_function():
    b = s2rst.S2Builder(snap_function=s2rst.S2CellIdSnapFunction(15))
    b.add_loop_from_points(_square())
    assert b.build_polygon().num_loops() == 1


def test_option_flags():
    b = s2rst.S2Builder(idempotent=False, simplify_edge_chains=True)
    b.add_loop_from_points(_square())
    assert b.build_polygon().num_loops() == 1
