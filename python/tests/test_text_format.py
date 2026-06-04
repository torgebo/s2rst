# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for text_format parsing and formatting."""

import s2rst


def test_parse_points_roundtrip():
    pts = s2rst.parse_points("0:0, 0:10, 10:0")
    assert len(pts) == 3
    assert s2rst.points_to_string(pts) == "0:0, 0:10, 10:0"


def test_parse_point():
    p = s2rst.parse_point("10:20")
    assert s2rst.point_to_string(p) == "10:20"


def test_parse_latlngs():
    lls = s2rst.parse_latlngs("1:2, 3:4")
    assert len(lls) == 2
    assert s2rst.latlng_to_string(lls[0]) == "1:2"


def test_make_loop():
    loop = s2rst.make_loop("0:0, 0:10, 10:0")
    assert len(loop) == 3
    assert s2rst.loop_to_string(loop) == "0:0, 0:10, 10:0"


def test_make_polygon():
    poly = s2rst.make_polygon("0:0, 0:10, 10:10, 10:0")
    assert s2rst.polygon_to_string(poly)  # non-empty round-trip text


def test_make_polyline():
    pl = s2rst.make_polyline("0:0, 1:1, 2:2")
    assert len(pl) == 3
    assert s2rst.polyline_to_string(pl) == "0:0, 1:1, 2:2"


def test_malformed_input_is_lenient():
    # The S2 text parser is lenient: unparseable coordinates default to 0.
    assert s2rst.point_to_string(s2rst.parse_point("abc:def")) == "0:0"
