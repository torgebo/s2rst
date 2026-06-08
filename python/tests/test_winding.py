# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for the winding operation (N-way boolean via winding numbers)."""

import pytest

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _square(lat0, lng0, size):
    return [
        _point(lat0, lng0),
        _point(lat0, lng0 + size),
        _point(lat0 + size, lng0 + size),
        _point(lat0 + size, lng0),
    ]


def test_winding_single_square_inside():
    loops = [_square(0, 0, 10)]
    poly = s2rst.winding_operation(loops, _point(5, 5), 1, s2rst.WindingRule.POSITIVE)
    assert poly.num_loops() == 1
    assert poly.area() > 0
    assert poly.contains_point(_point(5, 5))


def test_winding_reference_point_independence():
    # The result is the same whether described from an interior reference point
    # (winding 1) or an exterior one (winding 0): both yield the square.
    loops = [_square(0, 0, 10)]
    inside = s2rst.winding_operation(loops, _point(5, 5), 1, s2rst.WindingRule.POSITIVE)
    outside = s2rst.winding_operation(
        loops, _point(45, 45), 0, s2rst.WindingRule.POSITIVE
    )
    assert outside.area() > 0
    assert inside.area() == pytest.approx(outside.area())


def test_winding_rules():
    loops = [_square(0, 0, 10)]
    ref = _point(5, 5)
    for rule in (
        s2rst.WindingRule.POSITIVE,
        s2rst.WindingRule.NON_ZERO,
        s2rst.WindingRule.ODD,
    ):
        poly = s2rst.winding_operation(loops, ref, 1, rule)
        assert poly.area() > 0
    # The square has positive winding, so the NEGATIVE rule selects nothing.
    neg = s2rst.winding_operation(loops, ref, 1, s2rst.WindingRule.NEGATIVE)
    assert neg.is_empty_polygon()


def test_winding_union_of_overlapping_squares():
    loops = [_square(0, 0, 4), _square(1, 1, 4)]  # overlapping
    poly = s2rst.winding_operation(
        loops,
        _point(0.5, 0.5),
        1,
        s2rst.WindingRule.POSITIVE,
        snap_function=s2rst.IntLatLngSnapFunction(1),
    )
    assert poly.area() > 0
    # The union covers more than either square alone.
    single = s2rst.winding_operation(
        [_square(0, 0, 4)], _point(0.5, 0.5), 1, s2rst.WindingRule.POSITIVE
    )
    assert poly.area() > single.area()


def test_winding_include_degeneracies():
    loops = [_square(0, 0, 10)]
    poly = s2rst.winding_operation(
        loops, _point(5, 5), 1, s2rst.WindingRule.POSITIVE, include_degeneracies=True
    )
    assert poly.area() > 0


def test_winding_bad_snap_function_type():
    with pytest.raises(TypeError):
        s2rst.winding_operation(
            [_square(0, 0, 10)],
            _point(5, 5),
            1,
            s2rst.WindingRule.POSITIVE,
            snap_function="nope",
        )


def test_winding_rule_enum_distinct():
    rules = {
        s2rst.WindingRule.POSITIVE,
        s2rst.WindingRule.NEGATIVE,
        s2rst.WindingRule.NON_ZERO,
        s2rst.WindingRule.ODD,
    }
    assert len(rules) == 4
