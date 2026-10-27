# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for Angle and ChordAngle."""

import math
import pytest
import s2rst


class TestAngle:
    def test_from_radians(self):
        a = s2rst.Angle.from_radians(math.pi)
        assert a.radians == pytest.approx(math.pi)
        assert a.degrees == pytest.approx(180.0)

    def test_from_degrees(self):
        a = s2rst.Angle.from_degrees(90.0)
        assert a.radians == pytest.approx(math.pi / 2)
        assert a.degrees == pytest.approx(90.0)

    def test_from_e5(self):
        a = s2rst.Angle.from_e5(9000000)
        assert a.degrees == pytest.approx(90.0)

    def test_from_e6(self):
        a = s2rst.Angle.from_e6(90000000)
        assert a.degrees == pytest.approx(90.0)

    def test_from_e7(self):
        a = s2rst.Angle.from_e7(900000000)
        assert a.degrees == pytest.approx(90.0)

    def test_e5_e6_e7_roundtrip(self):
        a = s2rst.Angle.from_degrees(45.123)
        assert a.e5() == 4512300
        assert a.e6() == 45123000
        assert a.e7() == 451230000

    def test_constants(self):
        assert s2rst.Angle.ZERO.radians == 0.0
        assert s2rst.Angle.INFINITY.is_infinite()

    def test_abs(self):
        a = s2rst.Angle.from_degrees(-45.0)
        assert a.abs().degrees == pytest.approx(45.0)

    def test_normalized(self):
        a = s2rst.Angle.from_degrees(270.0)
        n = a.normalized()
        assert n.degrees == pytest.approx(-90.0)

    def test_trig(self):
        a = s2rst.Angle.from_degrees(90.0)
        assert a.sin() == pytest.approx(1.0)
        assert a.cos() == pytest.approx(0.0, abs=1e-15)

    def test_approx_eq(self):
        a = s2rst.Angle.from_degrees(45.0)
        b = s2rst.Angle.from_degrees(45.0)
        assert a.approx_eq(b)

    def test_arithmetic(self):
        a = s2rst.Angle.from_degrees(30.0)
        b = s2rst.Angle.from_degrees(60.0)
        assert (a + b).degrees == pytest.approx(90.0)
        assert (b - a).degrees == pytest.approx(30.0)
        assert (a * 2.0).degrees == pytest.approx(60.0)
        assert (2.0 * a).degrees == pytest.approx(60.0)
        assert (-a).degrees == pytest.approx(-30.0)

    def test_division_by_scalar(self):
        a = s2rst.Angle.from_degrees(90.0)
        result = a / 2.0
        assert isinstance(result, s2rst.Angle)
        assert result.degrees == pytest.approx(45.0)

    def test_division_by_angle(self):
        a = s2rst.Angle.from_degrees(90.0)
        b = s2rst.Angle.from_degrees(30.0)
        result = a / b
        assert isinstance(result, float)
        assert result == pytest.approx(3.0)

    def test_comparisons(self):
        a = s2rst.Angle.from_degrees(30.0)
        b = s2rst.Angle.from_degrees(60.0)
        assert a < b
        assert a <= b
        assert b > a
        assert b >= a
        assert a == a
        assert not (a == b)

    def test_repr(self):
        a = s2rst.Angle.from_degrees(45.0)
        assert "45" in repr(a)

    def test_float(self):
        a = s2rst.Angle.from_radians(1.5)
        assert float(a) == pytest.approx(1.5)


class TestChordAngle:
    def test_constants(self):
        assert s2rst.ChordAngle.ZERO.is_zero()
        assert s2rst.ChordAngle.INFINITY.is_infinity()
        assert s2rst.ChordAngle.NEGATIVE.is_negative()

    def test_from_length2(self):
        ca = s2rst.ChordAngle.from_length2(2.0)
        assert ca.length2 == pytest.approx(2.0)
        assert ca.degrees == pytest.approx(90.0)

    def test_from_length2_canonical(self):
        # Spec 007 FR-017/FR-018: every negative length is NEGATIVE; NaN and +inf are STRAIGHT.
        for x in (-5.0, -0.001, float("-inf")):
            assert s2rst.ChordAngle.from_length2(x).length2 == -1.0
        for x in (float("nan"), float("inf"), 5.0):
            assert s2rst.ChordAngle.from_length2(x).length2 == 4.0

    def test_from_angle(self):
        a = s2rst.Angle.from_degrees(90.0)
        ca = s2rst.ChordAngle.from_angle(a)
        assert ca.degrees == pytest.approx(90.0)

    def test_from_radians(self):
        ca = s2rst.ChordAngle.from_radians(math.pi / 2)
        assert ca.degrees == pytest.approx(90.0)

    def test_from_degrees(self):
        ca = s2rst.ChordAngle.from_degrees(60.0)
        assert ca.degrees == pytest.approx(60.0)

    def test_to_angle(self):
        ca = s2rst.ChordAngle.from_degrees(90.0)
        a = ca.to_angle()
        assert isinstance(a, s2rst.Angle)
        assert a.degrees == pytest.approx(90.0)

    def test_successor_predecessor(self):
        ca = s2rst.ChordAngle.from_length2(1.0)
        s = ca.successor()
        p = s.predecessor()
        assert p.length2 == pytest.approx(ca.length2)

    def test_trig(self):
        ca = s2rst.ChordAngle.from_degrees(90.0)
        assert ca.sin() == pytest.approx(1.0)
        assert ca.cos() == pytest.approx(0.0, abs=1e-15)

    def test_arithmetic(self):
        a = s2rst.ChordAngle.from_degrees(30.0)
        b = s2rst.ChordAngle.from_degrees(30.0)
        result = a + b
        # ChordAngle + adds the angles (not length2 values)
        assert result.degrees == pytest.approx(60.0, abs=1e-10)

    def test_comparisons(self):
        a = s2rst.ChordAngle.from_degrees(30.0)
        b = s2rst.ChordAngle.from_degrees(60.0)
        assert a < b
        assert b > a
        assert a == a

    def test_is_valid(self):
        assert s2rst.ChordAngle.ZERO.is_valid()
        assert s2rst.ChordAngle.RIGHT.is_valid()
        assert s2rst.ChordAngle.STRAIGHT.is_valid()
        assert s2rst.ChordAngle.INFINITY.is_valid()

    def test_is_special(self):
        assert s2rst.ChordAngle.INFINITY.is_special()
        assert s2rst.ChordAngle.NEGATIVE.is_special()
        assert not s2rst.ChordAngle.ZERO.is_special()

    def test_repr(self):
        ca = s2rst.ChordAngle.from_degrees(45.0)
        assert "45" in repr(ca)


@pytest.mark.parametrize("special", ["INFINITY", "NEGATIVE"])
def test_chord_angle_special_trig_and_arithmetic_raise_value_error(special):
    # The core checks these only in debug builds: a PanicException there, a meaningless
    # number in release builds.
    ca = getattr(s2rst.ChordAngle, special)
    right = s2rst.ChordAngle.RIGHT
    for op in (ca.sin, ca.sin2, ca.cos, ca.tan):
        with pytest.raises(ValueError):
            op()
    for a, b in ((ca, right), (right, ca)):
        with pytest.raises(ValueError):
            a + b
        with pytest.raises(ValueError):
            a - b
    assert right.sin() == pytest.approx(1.0)
    assert (right + right).length2 == 4.0
