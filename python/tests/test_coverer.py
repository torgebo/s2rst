# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for RegionCoverer."""

import pytest

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _cap(lat, lng, deg):
    return s2rst.Cap.from_center_angle(_point(lat, lng), s2rst.Angle.from_degrees(deg))


def _triangle():
    return [_point(0, 0), _point(0, 2), _point(2, 0)]


def test_defaults():
    c = s2rst.RegionCoverer()
    assert (c.min_level, c.max_level, c.level_mod, c.max_cells) == (0, 30, 1, 8)


def test_settings_and_repr():
    c = s2rst.RegionCoverer(min_level=2, max_level=14, level_mod=2, max_cells=16)
    assert (c.min_level, c.max_level, c.level_mod, c.max_cells) == (2, 14, 2, 16)
    assert (
        repr(c) == "RegionCoverer(min_level=2, max_level=14, level_mod=2, max_cells=16)"
    )


def test_settings_are_keyword_only():
    with pytest.raises(TypeError):
        s2rst.RegionCoverer(0, 14)  # positional not allowed


def test_covering_respects_max_cells():
    c = s2rst.RegionCoverer(max_cells=8)
    cov = c.covering(_cap(48.8566, 2.3522, 0.5))
    assert 0 < len(cov) <= 8


def test_covering_accepts_all_region_types():
    c = s2rst.RegionCoverer(max_cells=12)
    regions = [
        _cap(0, 0, 1.0),
        s2rst.Rect.from_center_size(
            s2rst.LatLng.from_degrees(0, 0), s2rst.LatLng.from_degrees(2, 2)
        ),
        s2rst.Loop(_triangle()),
        s2rst.Polygon([s2rst.Loop(_triangle())]),
    ]
    for region in regions:
        cov = c.covering(region)
        assert len(cov) > 0


def test_interior_covering_runs():
    c = s2rst.RegionCoverer(max_cells=20)
    cap = _cap(10, 20, 5.0)
    interior = c.interior_covering(cap)
    # An interior covering never has more cells than the (outer) covering.
    assert len(interior) <= len(c.covering(cap)) + 1


def test_covering_rejects_non_region():
    c = s2rst.RegionCoverer()
    for bad in (42, "not a region", _point(0, 0)):
        with pytest.raises(TypeError):
            c.covering(bad)
