# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for the cell-size Metric and its constants."""

import math

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst

# Every named metric constant, for parametrized sanity checks.
ALL_METRICS = [
    s2rst.Metric.MIN_ANGLE_SPAN,
    s2rst.Metric.AVG_ANGLE_SPAN,
    s2rst.Metric.MAX_ANGLE_SPAN,
    s2rst.Metric.MIN_WIDTH,
    s2rst.Metric.AVG_WIDTH,
    s2rst.Metric.MAX_WIDTH,
    s2rst.Metric.MIN_EDGE,
    s2rst.Metric.AVG_EDGE,
    s2rst.Metric.MAX_EDGE,
    s2rst.Metric.MIN_DIAG,
    s2rst.Metric.AVG_DIAG,
    s2rst.Metric.MAX_DIAG,
    s2rst.Metric.MIN_AREA,
    s2rst.Metric.AVG_AREA,
    s2rst.Metric.MAX_AREA,
]

LENGTH_METRICS = [
    s2rst.Metric.MIN_ANGLE_SPAN,
    s2rst.Metric.AVG_ANGLE_SPAN,
    s2rst.Metric.MAX_ANGLE_SPAN,
    s2rst.Metric.MIN_WIDTH,
    s2rst.Metric.AVG_WIDTH,
    s2rst.Metric.MAX_WIDTH,
    s2rst.Metric.MIN_EDGE,
    s2rst.Metric.AVG_EDGE,
    s2rst.Metric.MAX_EDGE,
    s2rst.Metric.MIN_DIAG,
    s2rst.Metric.AVG_DIAG,
    s2rst.Metric.MAX_DIAG,
]

AREA_METRICS = [
    s2rst.Metric.MIN_AREA,
    s2rst.Metric.AVG_AREA,
    s2rst.Metric.MAX_AREA,
]


def test_avg_area_at_level_zero():
    # The six face cells tile the whole sphere (area 4*pi), so the average
    # area of a face cell is 4*pi/6.
    assert math.isclose(s2rst.Metric.AVG_AREA.value(0), 4.0 * math.pi / 6.0)


def test_value_at_level_zero_equals_deriv():
    # value(0) == deriv * 2**0 == deriv.
    for m in ALL_METRICS:
        assert math.isclose(m.value(0), m.deriv)


def test_value_shrinks_with_level():
    # A finer level always has a strictly smaller value.
    assert s2rst.Metric.MIN_WIDTH.value(0) > s2rst.Metric.MIN_WIDTH.value(1)
    for m in ALL_METRICS:
        for level in range(0, 30):
            assert m.value(level) > m.value(level + 1)


def test_length_value_halves_per_level():
    # A length metric (dim == 1) halves with each level.
    for m in LENGTH_METRICS:
        assert math.isclose(m.value(1), m.value(0) / 2.0)


def test_area_value_quarters_per_level():
    # An area metric (dim == 2) quarters with each level.
    for m in AREA_METRICS:
        assert math.isclose(m.value(1), m.value(0) / 4.0)


def test_dimensions():
    assert s2rst.Metric.AVG_AREA.dim == 2
    assert s2rst.Metric.MIN_AREA.dim == 2
    assert s2rst.Metric.MAX_AREA.dim == 2
    assert s2rst.Metric.AVG_EDGE.dim == 1
    assert s2rst.Metric.MIN_WIDTH.dim == 1
    for m in LENGTH_METRICS:
        assert m.dim == 1
    for m in AREA_METRICS:
        assert m.dim == 2


def test_deriv_positive():
    for m in ALL_METRICS:
        assert m.deriv > 0.0


def test_min_avg_max_area_ordering():
    for level in range(0, 31):
        lo = s2rst.Metric.MIN_AREA.value(level)
        mid = s2rst.Metric.AVG_AREA.value(level)
        hi = s2rst.Metric.MAX_AREA.value(level)
        assert lo <= mid <= hi


def test_min_avg_max_ordering_all_families():
    families = [
        (
            s2rst.Metric.MIN_ANGLE_SPAN,
            s2rst.Metric.AVG_ANGLE_SPAN,
            s2rst.Metric.MAX_ANGLE_SPAN,
        ),
        (s2rst.Metric.MIN_WIDTH, s2rst.Metric.AVG_WIDTH, s2rst.Metric.MAX_WIDTH),
        (s2rst.Metric.MIN_EDGE, s2rst.Metric.AVG_EDGE, s2rst.Metric.MAX_EDGE),
        (s2rst.Metric.MIN_DIAG, s2rst.Metric.AVG_DIAG, s2rst.Metric.MAX_DIAG),
        (s2rst.Metric.MIN_AREA, s2rst.Metric.AVG_AREA, s2rst.Metric.MAX_AREA),
    ]
    for lo_m, mid_m, hi_m in families:
        for level in range(0, 31):
            assert lo_m.value(level) <= mid_m.value(level) <= hi_m.value(level)


def test_avg_area_tiles_sphere():
    # 6 face cells of average area cover the full 4*pi sphere.
    total = s2rst.Metric.AVG_AREA.value(0) * 6.0
    assert math.isclose(total, 4.0 * math.pi)


def test_aspect_constants_are_floats():
    assert isinstance(s2rst.Metric.MAX_EDGE_ASPECT, float)
    assert isinstance(s2rst.Metric.MAX_DIAG_ASPECT, float)
    assert s2rst.Metric.MAX_EDGE_ASPECT >= 1.0
    # MAX_DIAG_ASPECT is sqrt(3) for all projections.
    assert math.isclose(s2rst.Metric.MAX_DIAG_ASPECT, math.sqrt(3.0))


def test_aspect_within_global_bounds():
    edge_ratio = s2rst.Metric.MAX_EDGE.deriv / s2rst.Metric.MIN_EDGE.deriv
    diag_ratio = s2rst.Metric.MAX_DIAG.deriv / s2rst.Metric.MIN_DIAG.deriv
    assert s2rst.Metric.MAX_EDGE_ASPECT <= edge_ratio
    assert s2rst.Metric.MAX_DIAG_ASPECT <= diag_ratio


def test_min_level_extremes():
    # Very large value -> coarsest level (0).
    assert s2rst.Metric.MIN_WIDTH.min_level(1e10) == 0
    # Very small value -> finest level (30).
    assert s2rst.Metric.MIN_WIDTH.min_level(1e-30) == 30
    # Non-positive value -> finest level (30).
    assert s2rst.Metric.MIN_WIDTH.min_level(0.0) == 30
    assert s2rst.Metric.MIN_WIDTH.min_level(-1.0) == 30


def test_max_level_extremes():
    assert s2rst.Metric.MIN_WIDTH.max_level(1e10) == 0
    assert s2rst.Metric.MIN_WIDTH.max_level(1e-30) == 30
    assert s2rst.Metric.MIN_WIDTH.max_level(math.inf) == 0


def test_closest_level_matches_value():
    level = s2rst.Metric.AVG_EDGE.closest_level(0.1)
    assert 0 <= level <= 30
    v = s2rst.Metric.AVG_EDGE.value(level)
    assert 0.05 < v < 0.3


def test_min_level_value_is_within_bound():
    # value(min_level(val)) must be <= val (it is the finest satisfying level).
    for val in (1.0, 0.5, 0.1, 0.01, 1e-3):
        level = s2rst.Metric.MIN_DIAG.min_level(val)
        assert s2rst.Metric.MIN_DIAG.value(level) <= val


def test_max_level_value_is_within_bound():
    # value(max_level(val)) must be >= val (it is the coarsest satisfying level).
    for val in (1.0, 0.5, 0.1, 0.01, 1e-3):
        level = s2rst.Metric.MIN_DIAG.max_level(val)
        assert s2rst.Metric.MIN_DIAG.value(level) >= val


def test_repr():
    text = repr(s2rst.Metric.AVG_AREA)
    assert text.startswith("Metric(")
    assert "dim=2" in text


def test_metric_has_no_constructor():
    # Metric is construction-free: only classattrs and methods are exposed.
    with pytest.raises(TypeError):
        s2rst.Metric()


@given(level=st.integers(min_value=0, max_value=30))
def test_min_level_round_trip(level):
    # The minimum level for the exact metric value at `level` is `level`.
    val = s2rst.Metric.MIN_DIAG.value(level)
    assert s2rst.Metric.MIN_DIAG.min_level(val) == level


@given(level=st.integers(min_value=0, max_value=30))
def test_max_level_round_trip(level):
    val = s2rst.Metric.MIN_DIAG.value(level)
    assert s2rst.Metric.MIN_DIAG.max_level(val) == level


@given(level=st.integers(min_value=0, max_value=30))
def test_closest_level_round_trip(level):
    val = s2rst.Metric.MIN_DIAG.value(level)
    assert s2rst.Metric.MIN_DIAG.closest_level(val) == level


@given(
    level=st.integers(min_value=0, max_value=30),
    metric_index=st.integers(min_value=0, max_value=len(ALL_METRICS) - 1),
)
def test_value_formula(level, metric_index):
    # value(level) == deriv * 2**(-dim * level).
    m = ALL_METRICS[metric_index]
    expected = m.deriv * 2.0 ** (-m.dim * level)
    assert math.isclose(m.value(level), expected, rel_tol=1e-12, abs_tol=1e-300)
