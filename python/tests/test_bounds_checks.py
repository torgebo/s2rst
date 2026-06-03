# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tier-A correctness tests: bounds checks on indexing methods.

Each test fails (red) on the unfixed code — either with
`pyo3_runtime.PanicException` (Rust panic crossing FFI) or with a silent
fallback (wrong-but-non-erroring return value). The fix in each .rs site
makes the test pass (green).

Tests are grouped by item number (A1, A2, …) so it's
clear which fix each test exercises.
"""

import pytest

import s2rst


# ---------------------------------------------------------------------------
# A1: Cell.vertex bounds check
# ---------------------------------------------------------------------------


def test_a1_cell_vertex_out_of_range_raises_index_error():
    c = s2rst.Cell(s2rst.CellId.from_face(0))
    # In-range still works.
    for k in range(4):
        _ = c.vertex(k)
    # Out-of-range must raise IndexError, not panic.
    with pytest.raises(IndexError):
        c.vertex(4)
    with pytest.raises(IndexError):
        c.vertex(99)


# ---------------------------------------------------------------------------
# A2: Cell.vertex_raw bounds check
# ---------------------------------------------------------------------------


def test_a2_cell_vertex_raw_out_of_range_raises_index_error():
    c = s2rst.Cell(s2rst.CellId.from_face(0))
    for k in range(4):
        _ = c.vertex_raw(k)
    with pytest.raises(IndexError):
        c.vertex_raw(4)
    with pytest.raises(IndexError):
        c.vertex_raw(99)


# ---------------------------------------------------------------------------
# A3: Cell.edge bounds check (kill silent fallback to Bottom edge)
# ---------------------------------------------------------------------------


def test_a3_cell_edge_out_of_range_raises_index_error():
    c = s2rst.Cell(s2rst.CellId.from_face(0))
    for k in range(4):
        _ = c.edge(k)
    with pytest.raises(IndexError):
        c.edge(4)
    with pytest.raises(IndexError):
        c.edge(255)


def test_a3_cell_edge_out_of_range_does_not_alias_edge0():
    # Sanity check the silent-fallback bug specifically: prior to the fix
    # `edge(4)` returned the same point as `edge(0)`. After the fix it
    # raises, so this test is implicitly satisfied by test_a3_…raises.
    c = s2rst.Cell(s2rst.CellId.from_face(0))
    e0 = c.edge(0)
    with pytest.raises(IndexError):
        bad = c.edge(4)
        assert bad != e0  # unreachable; defensive


# ---------------------------------------------------------------------------
# A4: Cell.edge_raw bounds check (kill silent fallback)
# ---------------------------------------------------------------------------


def test_a4_cell_edge_raw_out_of_range_raises_index_error():
    c = s2rst.Cell(s2rst.CellId.from_face(0))
    for k in range(4):
        _ = c.edge_raw(k)
    with pytest.raises(IndexError):
        c.edge_raw(4)
    with pytest.raises(IndexError):
        c.edge_raw(255)


# ---------------------------------------------------------------------------
# A5: Rect.vertex bounds check (kill silent fallback to LowerLeft)
# ---------------------------------------------------------------------------


def test_a5_rect_vertex_out_of_range_raises_index_error():
    r = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(10.0, 20.0))
    for k in range(4):
        _ = r.vertex(k)
    with pytest.raises(IndexError):
        r.vertex(4)
    with pytest.raises(IndexError):
        r.vertex(255)


def test_a5_rect_vertex_does_not_alias_vertex0():
    r = s2rst.Rect.from_lat_lng(s2rst.LatLng.from_degrees(10.0, 20.0))
    v0 = r.vertex(0)
    with pytest.raises(IndexError):
        bad = r.vertex(4)
        assert bad != v0  # unreachable


# ---------------------------------------------------------------------------
# A6: EdgeVectorShape.set_dimension validation
# ---------------------------------------------------------------------------


def test_a6_set_dimension_accepts_valid_values():
    evs = s2rst.EdgeVectorShape()
    for dim in (0, 1, 2):
        evs.set_dimension(dim)


def test_a6_set_dimension_rejects_invalid_values():
    evs = s2rst.EdgeVectorShape()
    for bad in (3, 4, 99, 255):
        with pytest.raises(ValueError):
            evs.set_dimension(bad)


# ---------------------------------------------------------------------------
# A7: Polyline.vertex bounds check
# ---------------------------------------------------------------------------


def test_a7_polyline_vertex_in_range_works():
    p = s2rst.Polyline([s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)])
    assert p.vertex(0) == s2rst.S2Point(1, 0, 0)
    assert p.vertex(1) == s2rst.S2Point(0, 1, 0)


def test_a7_polyline_vertex_out_of_range_raises_index_error():
    p = s2rst.Polyline([s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)])
    with pytest.raises(IndexError):
        p.vertex(2)
    with pytest.raises(IndexError):
        p.vertex(100)


# ---------------------------------------------------------------------------
# A8: Loop construction guard (the i % 0 panic actually happens during
# Loop::new() bound-init, not vertex(); reject Loop([]) at the boundary).
# ---------------------------------------------------------------------------


def test_a8_loop_with_no_vertices_raises_value_error():
    # Constructing a Loop with an empty vertex list panics in the core's
    # bound initializer (i % 0). Reject at the Python boundary.
    with pytest.raises(ValueError):
        s2rst.Loop([])


def test_a8_loop_empty_and_full_remain_valid():
    # The special empty / full loops use a 1-vertex sentinel and must
    # still construct + index correctly.
    e = s2rst.Loop.empty()
    f = s2rst.Loop.full()
    assert e.is_empty_loop()
    assert f.is_full_loop()
    assert e.num_vertices() == 1
    assert f.num_vertices() == 1
    _ = e.vertex(0)
    _ = f.vertex(0)


def test_a8_loop_vertex_wraps_for_non_empty():
    # Loops intentionally support i in [0, 2*num_vertices) by wrapping;
    # confirm that contract is still honoured for non-empty loops.
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1)]
    loop_ = s2rst.Loop(pts)
    assert loop_.vertex(0) == loop_.vertex(3)
    assert loop_.vertex(1) == loop_.vertex(4)
    assert loop_.vertex(2) == loop_.vertex(5)


# ---------------------------------------------------------------------------
# A9: Lax shape vertex/point bounds checks
# ---------------------------------------------------------------------------


def test_a9_lax_loop_vertex_bounds():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)]
    ll = s2rst.LaxLoop(pts)
    assert ll.vertex(0) == pts[0]
    assert ll.vertex(1) == pts[1]
    with pytest.raises(IndexError):
        ll.vertex(2)
    with pytest.raises(IndexError):
        ll.vertex(99)


def test_a9_lax_polyline_vertex_bounds():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)]
    lp = s2rst.LaxPolyline(pts)
    assert lp.vertex(0) == pts[0]
    assert lp.vertex(1) == pts[1]
    with pytest.raises(IndexError):
        lp.vertex(2)


def test_a9_point_vector_point_bounds():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0)]
    pv = s2rst.PointVector(pts)
    assert pv.point(0) == pts[0]
    assert pv.point(1) == pts[1]
    with pytest.raises(IndexError):
        pv.point(2)


def test_a9_lax_polygon_loop_vertex_bounds():
    loops = [
        [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1)],
    ]
    lp = s2rst.LaxPolygon(loops)
    # In-range still works.
    assert lp.num_loop_vertices(0) == 3
    _ = lp.loop_vertex(0, 0)
    _ = lp.loop_vertex(0, 2)
    # Out-of-range loop index must raise.
    with pytest.raises(IndexError):
        lp.num_loop_vertices(99)
    with pytest.raises(IndexError):
        lp.loop_vertex(99, 0)
    # Out-of-range vertex index within a valid loop must raise.
    with pytest.raises(IndexError):
        lp.loop_vertex(0, 99)


# ---------------------------------------------------------------------------
# A10: Shape.edge / chain / chain_edge / chain_position bounds checks
# ---------------------------------------------------------------------------


def _shape_with_edges():
    pts = [s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0), s2rst.S2Point(0, 0, 1)]
    return s2rst.LaxLoop(pts).as_shape()


def test_a10_shape_edge_bounds():
    shape = _shape_with_edges()
    n = shape.num_edges()
    assert n > 0
    _ = shape.edge(0)
    _ = shape.edge(n - 1)
    with pytest.raises(IndexError):
        shape.edge(n)
    with pytest.raises(IndexError):
        shape.edge(99)


def test_a10_shape_chain_bounds():
    shape = _shape_with_edges()
    nc = shape.num_chains()
    assert nc > 0
    _ = shape.chain(0)
    with pytest.raises(IndexError):
        shape.chain(nc)
    with pytest.raises(IndexError):
        shape.chain(99)


def test_a10_shape_chain_edge_bounds():
    shape = _shape_with_edges()
    # chain 0 always exists; offset must be < chain length.
    start, length = shape.chain(0)
    _ = shape.chain_edge(0, 0)
    _ = shape.chain_edge(0, length - 1)
    with pytest.raises(IndexError):
        shape.chain_edge(0, length)
    with pytest.raises(IndexError):
        shape.chain_edge(99, 0)


def test_a10_shape_chain_position_bounds():
    shape = _shape_with_edges()
    n = shape.num_edges()
    _ = shape.chain_position(0)
    _ = shape.chain_position(n - 1)
    with pytest.raises(IndexError):
        shape.chain_position(n)
    with pytest.raises(IndexError):
        shape.chain_position(99)


# ---------------------------------------------------------------------------
# A11: R2Rect.vertex bounds check (the underlying core uses
# `rem_euclid(4)`, silently wrapping any i32 — surprises users the same
# way Rect.vertex did before A5. Tighten for consistency.)
# ---------------------------------------------------------------------------


def test_a11_r2_rect_vertex_in_range_works():
    r = s2rst.R2Rect(s2rst.R1Interval(0, 1), s2rst.R1Interval(0, 2))
    assert r.vertex(0) == s2rst.R2Point(0, 0)
    assert r.vertex(1) == s2rst.R2Point(1, 0)
    assert r.vertex(2) == s2rst.R2Point(1, 2)
    assert r.vertex(3) == s2rst.R2Point(0, 2)


def test_a11_r2_rect_vertex_out_of_range_raises_index_error():
    r = s2rst.R2Rect(s2rst.R1Interval(0, 1), s2rst.R1Interval(0, 2))
    with pytest.raises(IndexError):
        r.vertex(4)
    with pytest.raises(IndexError):
        r.vertex(-1)
    with pytest.raises(IndexError):
        r.vertex(99)
