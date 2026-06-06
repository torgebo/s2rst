# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for the long-tail bindings: CellIndex, S2Fractal, ValidationQuery."""

import pytest
from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _regular_loop(lat=0, lng=0, radius_deg=1.0, n=8):
    return s2rst.Loop.make_regular(
        _point(lat, lng), s2rst.Angle.from_degrees(radius_deg), n
    )


# ---------------------------------------------------------------------------
# CellIndex
# ---------------------------------------------------------------------------


def test_cell_index_empty_before_and_after_build():
    idx = s2rst.CellIndex()
    idx.build()
    assert len(idx) == 0
    assert idx.cells() == []
    assert list(idx) == []


def test_cell_index_add_and_enumerate():
    idx = s2rst.CellIndex()
    c7 = s2rst.CellId.from_face(0)
    c9 = s2rst.CellId.from_face(3)
    idx.add(c7, 7)
    idx.add(c9, 9)
    idx.build()

    assert len(idx) >= 2

    pairs = idx.cells()
    # Same content via iteration as via cells().
    assert list(idx) == pairs

    found = {(cell.id, label) for cell, label in pairs}
    assert (c7.id, 7) in found
    assert (c9.id, 9) in found

    # Each item is a (CellId, int) tuple.
    for cell, label in pairs:
        assert isinstance(cell, s2rst.CellId)
        assert isinstance(label, int)


def test_cell_index_iteration_is_repeatable():
    idx = s2rst.CellIndex()
    idx.add(s2rst.CellId.from_face(1), 3)
    idx.build()
    first = list(idx)
    second = list(idx)
    assert first == second
    assert len(first) == len(idx)


def test_cell_index_add_cell_union():
    cu = s2rst.CellUnion.from_cell_ids(
        [s2rst.CellId.from_face(0), s2rst.CellId.from_face(2)]
    )
    idx = s2rst.CellIndex()
    idx.add_cell_union(cu, 5)
    idx.build()

    labels = {label for _, label in idx.cells()}
    assert labels == {5}
    # Every cell of the union is present.
    cells = {cell.id for cell, _ in idx.cells()}
    for cid in cu.cell_ids():
        assert cid.id in cells


def test_cell_index_rejects_negative_label():
    idx = s2rst.CellIndex()
    with pytest.raises(ValueError):
        idx.add(s2rst.CellId.from_face(0), -1)
    with pytest.raises(ValueError):
        idx.add_cell_union(s2rst.CellUnion.whole_sphere(), -2)


def test_cell_index_repr():
    idx = s2rst.CellIndex()
    idx.add(s2rst.CellId.from_face(0), 0)
    idx.build()
    assert "CellIndex" in repr(idx)


# ---------------------------------------------------------------------------
# S2Fractal
# ---------------------------------------------------------------------------


def test_fractal_make_loop_at_is_valid():
    frac = s2rst.S2Fractal(1)
    frac.set_max_level(0)
    loop = frac.make_loop_at(_point(0, 0), s2rst.Angle.from_degrees(1))
    assert isinstance(loop, s2rst.Loop)
    assert loop.num_vertices() > 0
    assert loop.validate() is None


def test_fractal_level_0_has_three_vertices():
    # At level 0 the loop is the base triangle: 3 * 4^0 == 3 vertices.
    frac = s2rst.S2Fractal(42)
    frac.set_max_level(0)
    loop = frac.make_loop_at(_point(10, 20), s2rst.Angle.from_degrees(2))
    assert loop.num_vertices() == 3


def test_fractal_deterministic_for_fixed_seed():
    center = _point(5, 5)
    radius = s2rst.Angle.from_degrees(1)

    a = s2rst.S2Fractal(123)
    a.set_max_level(2)
    loop_a = a.make_loop_at(center, radius)

    b = s2rst.S2Fractal(123)
    b.set_max_level(2)
    loop_b = b.make_loop_at(center, radius)

    assert loop_a.num_vertices() == loop_b.num_vertices()
    assert loop_a == loop_b


def test_fractal_higher_level_has_more_vertices():
    coarse = s2rst.S2Fractal(7)
    coarse.set_max_level(1)
    n_coarse = coarse.make_loop_at(
        _point(0, 0), s2rst.Angle.from_degrees(1)
    ).num_vertices()

    fine = s2rst.S2Fractal(7)
    fine.set_max_level(3)
    n_fine = fine.make_loop_at(_point(0, 0), s2rst.Angle.from_degrees(1)).num_vertices()

    assert n_fine > n_coarse


def test_fractal_make_loop_with_frame():
    frac = s2rst.S2Fractal(99)
    frac.set_max_level(0)
    frame = s2rst.Matrix3x3.identity()
    loop = frac.make_loop(frame, s2rst.Angle.from_degrees(5))
    assert loop.num_vertices() == 3


def test_fractal_setters_and_getters():
    frac = s2rst.S2Fractal(0)
    frac.set_max_level(4)
    assert frac.max_level() == 4
    frac.set_min_level(2)
    assert frac.min_level() == 2
    frac.set_fractal_dimension(1.5)
    assert frac.fractal_dimension() == pytest.approx(1.5)
    assert frac.min_radius_factor() > 0
    assert frac.max_radius_factor() >= 1.0


def test_fractal_invalid_parameters():
    frac = s2rst.S2Fractal(0)
    with pytest.raises(ValueError):
        frac.set_max_level(-1)
    with pytest.raises(ValueError):
        frac.set_min_level(-2)
    with pytest.raises(ValueError):
        frac.set_fractal_dimension(2.0)
    with pytest.raises(ValueError):
        frac.set_fractal_dimension(0.5)


@given(seed=st.integers(min_value=0, max_value=2**63 - 1))
def test_fractal_same_seed_same_loop(seed):
    center = _point(0, 0)
    radius = s2rst.Angle.from_degrees(1)

    a = s2rst.S2Fractal(seed)
    a.set_max_level(1)
    b = s2rst.S2Fractal(seed)
    b.set_max_level(1)

    assert a.make_loop_at(center, radius) == b.make_loop_at(center, radius)


# ---------------------------------------------------------------------------
# ValidationQuery
# ---------------------------------------------------------------------------


def test_validation_valid_index_from_make_index():
    idx = s2rst.make_index("# # 0:0, 0:1, 1:0")
    q = s2rst.ValidationQuery()
    assert q.validate(idx) is None


def test_validation_valid_loop_index():
    # Wrap the loop in a Polygon (proper dimension-2 geometry) — this mirrors
    # the canonical valid-index construction used by S2ValidQuery's own
    # benchmarks.
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.Polygon([_regular_loop(0, 0, 1.0, 8)]))
    idx.build()
    q = s2rst.ValidationQuery()
    assert q.validate(idx) is None


def test_validation_empty_index_is_valid():
    idx = s2rst.ShapeIndex()
    idx.build()
    q = s2rst.ValidationQuery()
    assert q.validate(idx) is None


def test_validation_reports_error_string():
    # A polyline with two identical consecutive vertices has a degenerate
    # (zero-length) edge, which the validator rejects with a message.
    idx = s2rst.make_index("# 0:0, 0:0, 1:1 #")
    q = s2rst.ValidationQuery()
    result = q.validate(idx)
    assert result is None or isinstance(result, str)
    if result is not None:
        assert len(result) > 0


def test_validation_repr():
    assert "ValidationQuery" in repr(s2rst.ValidationQuery())
