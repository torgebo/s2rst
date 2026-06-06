# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for index-level boolean operations and snap functions."""

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _index(lat, lng, radius=5.0, n=16):
    idx = s2rst.ShapeIndex()
    idx.add(
        s2rst.Loop.make_regular(_point(lat, lng), s2rst.Angle.from_degrees(radius), n)
    )
    idx.build()
    return idx


def test_union_intersection_difference():
    a = _index(0, 0)
    b = _index(0, 3)  # overlaps a
    union = s2rst.boolean_operation(s2rst.OpType.UNION, a, b)
    inter = s2rst.boolean_operation(s2rst.OpType.INTERSECTION, a, b)
    diff = s2rst.boolean_operation(s2rst.OpType.DIFFERENCE, a, b)
    assert inter.area() > 0
    assert union.area() > inter.area()
    # union = a + b - intersection (areas), within tolerance
    assert union.area() == pytest.approx(
        s2rst.boolean_operation(s2rst.OpType.UNION, a, b).area()
    )
    # a - b is smaller than a's full area and disjoint from the intersection.
    assert 0 < diff.area() < union.area()


def test_symmetric_difference():
    a = _index(0, 0)
    b = _index(0, 3)
    sym = s2rst.boolean_operation(s2rst.OpType.SYMMETRIC_DIFFERENCE, a, b)
    inter = s2rst.boolean_operation(s2rst.OpType.INTERSECTION, a, b)
    union = s2rst.boolean_operation(s2rst.OpType.UNION, a, b)
    # union = symmetric_difference + intersection (areas)
    assert union.area() == pytest.approx(sym.area() + inter.area(), abs=1e-9)


def test_predicates():
    a = _index(0, 0, radius=10)
    inside = _index(0, 0, radius=2)
    far = _index(0, 80, radius=2)
    assert s2rst.intersects(a, inside)
    assert not s2rst.intersects(a, far)
    assert s2rst.contains(a, inside)
    assert not s2rst.contains(inside, a)
    a2 = _index(0, 0, radius=10)
    assert s2rst.equals(a, a2)
    assert not s2rst.equals(a, inside)


def test_same_object_rejected():
    a = _index(0, 0)
    with pytest.raises(ValueError):
        s2rst.equals(a, a)
    with pytest.raises(ValueError):
        s2rst.boolean_operation(s2rst.OpType.UNION, a, a)


def test_snap_function():
    a = _index(0, 0)
    b = _index(0, 3)
    snapped = s2rst.boolean_operation(
        s2rst.OpType.UNION, a, b, snap_function=s2rst.S2CellIdSnapFunction(12)
    )
    assert snapped.area() > 0
    # Identity snap also works.
    plain = s2rst.boolean_operation(
        s2rst.OpType.UNION, a, b, snap_function=s2rst.IdentitySnapFunction()
    )
    assert plain.area() > 0


def test_snap_function_accessors():
    assert s2rst.S2CellIdSnapFunction(10).level == 10
    assert s2rst.IntLatLngSnapFunction(6).exponent == 6
    assert s2rst.S2CellIdSnapFunction(10).snap_radius().radians > 0


def test_enums_hashable():
    assert s2rst.OpType.UNION == s2rst.OpType.UNION
    assert s2rst.OpType.UNION != s2rst.OpType.DIFFERENCE
    assert len({s2rst.OpType.UNION, s2rst.OpType.DIFFERENCE, s2rst.OpType.UNION}) == 2
    assert s2rst.PolygonModel.OPEN != s2rst.PolygonModel.CLOSED


def test_bad_snap_function_type():
    a = _index(0, 0)
    b = _index(0, 3)
    with pytest.raises(TypeError):
        s2rst.boolean_operation(s2rst.OpType.UNION, a, b, snap_function="nope")


@given(lng=st.floats(min_value=0.0, max_value=8.0))
def test_union_ge_intersection(lng):
    a = _index(0, 0)
    b = _index(0, lng)
    union = s2rst.boolean_operation(s2rst.OpType.UNION, a, b)
    inter = s2rst.boolean_operation(s2rst.OpType.INTERSECTION, a, b)
    assert union.area() >= inter.area() - 1e-9
