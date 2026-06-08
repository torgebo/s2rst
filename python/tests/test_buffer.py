# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for buffer operations (expand/contract geometry by a radius)."""

import pytest

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _opts(degrees, **kwargs):
    return s2rst.BufferOptions(s2rst.Angle.from_degrees(degrees), **kwargs)


def _regular_loop(radius=5.0, n=16):
    return s2rst.Loop.make_regular(_point(0, 0), s2rst.Angle.from_degrees(radius), n)


def test_buffer_point_makes_disc():
    center = _point(0, 0)
    poly = s2rst.buffer_point(center, _opts(1.0))
    assert poly.num_loops() == 1
    assert poly.area() > 0
    assert poly.contains_point(center)


def test_buffer_point_radius_scales_area():
    c = _point(0, 0)
    small = s2rst.buffer_point(c, _opts(1.0))
    big = s2rst.buffer_point(c, _opts(2.0))
    assert big.area() > small.area()


def test_buffer_negative_radius_removes_point():
    # A 0-dimensional input buffered by a negative radius vanishes.
    poly = s2rst.buffer_point(_point(0, 0), _opts(-1.0))
    assert poly.is_empty_polygon()


def test_buffer_loop_expand_and_contract():
    # A large triangle both expands and erodes to a non-empty smaller region
    # (mirrors core's loop-contract coverage). Expansion of the regular polygon
    # is covered separately by ``test_buffer_polygon_expands``.
    loop = s2rst.Loop([_point(-30, -30), _point(-30, 30), _point(30, 0)])
    base = loop.area()
    expanded = s2rst.buffer_loop(loop, _opts(1.0))
    contracted = s2rst.buffer_loop(loop, _opts(-1.0))
    assert expanded.area() > base
    assert 0 < contracted.area() < base


def test_buffer_polyline_non_empty():
    pl = s2rst.Polyline([_point(0, 0), _point(0, 5)])
    poly = s2rst.buffer_polyline(pl, _opts(1.0))
    assert not poly.is_empty_polygon()
    assert poly.area() > 0


def test_buffer_polyline_flat_end_cap():
    pl = s2rst.Polyline([_point(0, 0), _point(0, 5)])
    rounded = s2rst.buffer_polyline(pl, _opts(1.0))
    flat = s2rst.buffer_polyline(pl, _opts(1.0, end_cap_style=s2rst.EndCapStyle.FLAT))
    # Flat caps cover strictly less area than round caps.
    assert 0 < flat.area() < rounded.area()


def test_buffer_polyline_one_sided():
    pl = s2rst.Polyline([_point(0, 0), _point(0, 5)])
    poly = s2rst.buffer_polyline(pl, _opts(1.0, polyline_side=s2rst.PolylineSide.LEFT))
    assert poly.area() > 0


def test_buffer_polygon_expands():
    poly = s2rst.Polygon([_regular_loop()])
    expanded = s2rst.buffer_polygon(poly, _opts(1.0))
    assert expanded.area() > poly.area()


def test_buffer_circle_segments_controls_detail():
    c = _point(0, 0)
    coarse = s2rst.buffer_point(c, _opts(1.0, circle_segments=8))
    fine = s2rst.buffer_point(c, _opts(1.0, circle_segments=64))
    assert fine.num_vertices() > coarse.num_vertices()


def test_buffer_snap_function():
    poly = s2rst.buffer_point(
        _point(0, 0), _opts(1.0, snap_function=s2rst.S2CellIdSnapFunction(15))
    )
    assert poly.area() > 0


def test_buffer_options_accessors_and_repr():
    o = s2rst.BufferOptions(
        s2rst.Angle.from_degrees(2.0),
        end_cap_style=s2rst.EndCapStyle.FLAT,
        polyline_side=s2rst.PolylineSide.LEFT,
    )
    assert o.radius.degrees == pytest.approx(2.0)
    assert o.end_cap_style == s2rst.EndCapStyle.FLAT
    assert o.polyline_side == s2rst.PolylineSide.LEFT
    assert "BufferOptions" in repr(o)


def test_buffer_bad_snap_function_type():
    with pytest.raises(TypeError):
        s2rst.buffer_point(_point(0, 0), _opts(1.0, snap_function="nope"))


def test_buffer_enums_distinct_and_hashable():
    assert s2rst.EndCapStyle.ROUND != s2rst.EndCapStyle.FLAT
    sides = {s2rst.PolylineSide.LEFT, s2rst.PolylineSide.RIGHT, s2rst.PolylineSide.BOTH}
    assert len(sides) == 3
