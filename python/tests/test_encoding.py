# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for binary encode / decode round-trips."""

import pytest

import s2rst


def _loop():
    return s2rst.make_loop("0:0, 0:10, 10:10, 10:0")


def test_polygon_roundtrip():
    poly = s2rst.Polygon([_loop()])
    data = s2rst.encode(poly)
    assert isinstance(data, bytes)
    assert s2rst.encode(s2rst.decode_polygon(data)) == data


def test_polyline_roundtrip():
    pl = s2rst.make_polyline("0:0, 1:1, 2:2")
    data = s2rst.encode(pl)
    assert s2rst.encode(s2rst.decode_polyline(data)) == data


def test_loop_roundtrip():
    data = s2rst.encode(_loop())
    assert s2rst.encode(s2rst.decode_loop(data)) == data


def test_cell_union_roundtrip():
    cap = s2rst.Cap.from_center_angle(
        s2rst.LatLng.from_degrees(0, 0).to_point(), s2rst.Angle.from_degrees(1.0)
    )
    cov = s2rst.RegionCoverer(max_cells=8).covering(cap)
    data = s2rst.encode(cov)
    cov2 = s2rst.decode_cell_union(data)
    assert len(cov2) == len(cov)
    assert s2rst.encode(cov2) == data


def test_encode_rejects_unsupported():
    with pytest.raises(TypeError):
        s2rst.encode(42)


def test_decode_rejects_garbage():
    with pytest.raises(ValueError):
        s2rst.decode_polygon(b"\x00\x01\x02garbage")
