# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Verify that immutable s2rst value types satisfy collections.abc.Hashable.

Requirements per the Python data model:
- An object with value-based __eq__ should provide __hash__ that agrees with it
  (a == b implies hash(a) == hash(b)).
- The object should be usable as a dict key and in a set.
- isinstance(obj, Hashable) should return True (this is automatic when
  __hash__ is non-None — collections.abc.Hashable has a __subclasshook__).
"""

from collections.abc import Hashable

import pytest

import s2rst


def _angle_pair():
    return s2rst.Angle.from_degrees(45.0), s2rst.Angle.from_degrees(45.0)


def _chord_angle_pair():
    a = s2rst.Angle.from_degrees(60.0)
    return s2rst.ChordAngle.from_angle(a), s2rst.ChordAngle.from_angle(a)


def _r1_interval_pair():
    return s2rst.R1Interval(1.0, 2.0), s2rst.R1Interval(1.0, 2.0)


def _s1_interval_pair():
    return s2rst.S1Interval(0.1, 0.2), s2rst.S1Interval(0.1, 0.2)


def _r2_point_pair():
    return s2rst.R2Point(1.0, 2.0), s2rst.R2Point(1.0, 2.0)


def _vector_pair():
    return s2rst.Vector(1.0, 2.0, 3.0), s2rst.Vector(1.0, 2.0, 3.0)


def _matrix_pair():
    return (
        s2rst.Matrix3x3(1, 0, 0, 0, 1, 0, 0, 0, 1),
        s2rst.Matrix3x3(1, 0, 0, 0, 1, 0, 0, 0, 1),
    )


def _r2_rect_pair():
    a = s2rst.R2Rect(s2rst.R1Interval(0, 1), s2rst.R1Interval(0, 1))
    b = s2rst.R2Rect(s2rst.R1Interval(0, 1), s2rst.R1Interval(0, 1))
    return a, b


def _s2_point_pair():
    return s2rst.S2Point(1, 0, 0), s2rst.S2Point(1, 0, 0)


def _lat_lng_pair():
    return (
        s2rst.LatLng.from_degrees(37.0, -122.0),
        s2rst.LatLng.from_degrees(37.0, -122.0),
    )


def _cell_id_pair():
    return s2rst.CellId.from_face(0), s2rst.CellId.from_face(0)


def _cell_pair():
    return s2rst.Cell(s2rst.CellId.from_face(0)), s2rst.Cell(s2rst.CellId.from_face(0))


def _cap_pair():
    p = s2rst.S2Point(1, 0, 0)
    r = s2rst.ChordAngle.from_radians(0.1)
    return s2rst.Cap(p, r), s2rst.Cap(p, r)


def _rect_pair():
    return (
        s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(0, 0)),
        s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(0, 0)),
    )


def _edge_pair():
    p = s2rst.S2Point(1, 0, 0)
    q = s2rst.S2Point(0, 1, 0)
    return s2rst.Edge(p, q), s2rst.Edge(p, q)


# (factory, label) parametrization keeps test ids readable.
_PAIRS = [
    (_angle_pair, "Angle"),
    (_chord_angle_pair, "ChordAngle"),
    (_r1_interval_pair, "R1Interval"),
    (_s1_interval_pair, "S1Interval"),
    (_r2_point_pair, "R2Point"),
    (_vector_pair, "Vector"),
    (_matrix_pair, "Matrix3x3"),
    (_r2_rect_pair, "R2Rect"),
    (_s2_point_pair, "S2Point"),
    (_lat_lng_pair, "LatLng"),
    (_cell_id_pair, "CellId"),
    (_cell_pair, "Cell"),
    (_cap_pair, "Cap"),
    (_rect_pair, "Rect"),
    (_edge_pair, "Edge"),
]


@pytest.mark.parametrize(("factory", "label"), _PAIRS, ids=[lbl for _, lbl in _PAIRS])
def test_isinstance_hashable(factory, label):
    a, _ = factory()
    assert isinstance(a, Hashable), f"{label} must satisfy collections.abc.Hashable"


@pytest.mark.parametrize(("factory", "label"), _PAIRS, ids=[lbl for _, lbl in _PAIRS])
def test_hash_consistent_with_eq(factory, label):
    a, b = factory()
    assert a == b, f"{label}: equality precondition for the test failed"
    assert hash(a) == hash(b), f"{label}: equal objects must share a hash"


@pytest.mark.parametrize(("factory", "label"), _PAIRS, ids=[lbl for _, lbl in _PAIRS])
def test_hash_stable(factory, label):
    a, _ = factory()
    h1 = hash(a)
    h2 = hash(a)
    assert h1 == h2, f"{label}: hash must be stable across calls"


@pytest.mark.parametrize(("factory", "label"), _PAIRS, ids=[lbl for _, lbl in _PAIRS])
def test_usable_as_set_member(factory, label):
    a, b = factory()
    s = {a}
    assert b in s, f"{label}: equal object must be found in a set containing it"


@pytest.mark.parametrize(("factory", "label"), _PAIRS, ids=[lbl for _, lbl in _PAIRS])
def test_usable_as_dict_key(factory, label):
    a, b = factory()
    d = {a: 1}
    assert d[b] == 1, f"{label}: equal object must look up the same dict entry"


def test_distinct_values_likely_distinct_hashes():
    # Not a guarantee (collisions are legal) but a sanity check on the helper:
    # hashing different bit patterns should usually give different hashes.
    a = s2rst.Angle.from_degrees(45.0)
    b = s2rst.Angle.from_degrees(46.0)
    assert hash(a) != hash(b)


def test_referencepoint_eq_hash():
    # ReferencePoint isn't directly constructible from Python — it's returned
    # by Shape.reference_point(). Build one and verify equal-and-hashable.
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1)]
    shape_a = s2rst.LaxLoop(pts).as_shape()
    shape_b = s2rst.LaxLoop(pts).as_shape()
    rp_a = shape_a.reference_point()
    rp_b = shape_b.reference_point()
    assert rp_a == rp_b
    assert hash(rp_a) == hash(rp_b)
    assert isinstance(rp_a, Hashable)
