# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tier B1: __bool__ on collection types with semantic emptiness.

`Loop`, `Polygon`, and `LaxPolygon` distinguish "empty" (no interior) from
"full" (covers the sphere) independently of vertex/loop count, so they define
__bool__ explicitly. `CellUnion` and `EdgeVectorShape` deliberately rely on
the implicit __len__ fallback (len 0 -> falsey), which is already correct for
them; the last two tests pin that behaviour.
"""

import s2rst


def _triangle():
    return [
        s2rst.S2Point(1, 0, 0),
        s2rst.S2Point(0, 1, 0),
        s2rst.S2Point(0, 0, 1),
    ]


def test_loop_bool():
    assert bool(s2rst.Loop.empty()) is False
    assert bool(s2rst.Loop.full()) is True
    assert bool(s2rst.Loop(_triangle())) is True


def test_polygon_bool():
    assert bool(s2rst.Polygon.empty()) is False
    assert bool(s2rst.Polygon.full()) is True
    assert bool(s2rst.Polygon([s2rst.Loop(_triangle())])) is True


def test_lax_polygon_bool():
    assert bool(s2rst.LaxPolygon.empty()) is False
    assert bool(s2rst.LaxPolygon.full()) is True
    assert bool(s2rst.LaxPolygon([_triangle()])) is True


def test_cell_union_bool_uses_len_fallback():
    assert bool(s2rst.CellUnion()) is False
    nonempty = s2rst.CellUnion.from_cell_ids([s2rst.CellId.from_face(0)])
    assert bool(nonempty) is True


def test_edge_vector_shape_bool_uses_len_fallback():
    evs = s2rst.EdgeVectorShape()
    assert bool(evs) is False
    evs.add(s2rst.S2Point(1, 0, 0), s2rst.S2Point(0, 1, 0))
    assert bool(evs) is True
