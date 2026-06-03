# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for CellId, Cell, CellUnion."""

import pytest
import s2rst


class TestCellId:
    def test_new(self):
        cid = s2rst.CellId(0)
        assert cid.id == 0

    def test_none_sentinel(self):
        n = s2rst.CellId.none()
        assert n.id == 0
        s = s2rst.CellId.sentinel()
        assert s.id != 0

    def test_from_face(self):
        cid = s2rst.CellId.from_face(0)
        assert cid.face() == 0
        assert cid.level() == 0

    def test_from_face_pos_level(self):
        cid = s2rst.CellId.from_face_pos_level(3, 0, 0)
        assert cid.face() == 3
        assert cid.level() == 0

    def test_from_point(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        cid = s2rst.CellId.from_point(p)
        assert cid.is_valid()
        assert cid.is_leaf()

    def test_from_lat_lng(self):
        ll = s2rst.LatLng.from_degrees(0.0, 0.0)
        cid = s2rst.CellId.from_lat_lng(ll)
        assert cid.is_valid()

    def test_from_token(self):
        cid = s2rst.CellId.from_face(0)
        token = cid.to_token()
        cid2 = s2rst.CellId.from_token(token)
        assert cid == cid2

    def test_from_debug_string(self):
        cid = s2rst.CellId.from_debug_string("3/012")
        assert cid is not None
        assert cid.face() == 3
        assert cid.level() == 3

    def test_properties(self):
        cid = s2rst.CellId.from_face(0)
        assert cid.is_valid()
        assert cid.is_face()
        assert not cid.is_leaf()

    def test_hierarchy(self):
        cid = s2rst.CellId.from_face(0)
        children = cid.children()
        assert len(children) == 4
        for child in children:
            assert child.level() == 1
            p = child.parent()
            assert p == cid

    def test_parent_at_level(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        leaf = s2rst.CellId.from_point(p)
        face = leaf.parent_at_level(0)
        assert face.level() == 0
        assert face.is_face()

    def test_child_begin_end(self):
        cid = s2rst.CellId.from_face(0)
        begin = cid.child_begin()
        end = cid.child_end()
        assert begin < end

    def test_range_min_max(self):
        cid = s2rst.CellId.from_face(0)
        rmin = cid.range_min()
        rmax = cid.range_max()
        assert rmin.is_leaf()
        assert rmax.is_leaf()
        assert rmin <= rmax

    def test_contains_intersects(self):
        parent = s2rst.CellId.from_face(0)
        child = parent.children()[0]
        assert parent.contains(child)
        assert parent.intersects(child)
        assert not child.contains(parent)

    def test_next_prev(self):
        cid = s2rst.CellId.from_face(0)
        n = cid.next()
        p = n.prev()
        assert p == cid

    def test_advance(self):
        cid = s2rst.CellId.from_face(0)
        a = cid.advance(1)
        b = a.advance(-1)
        assert b == cid

    def test_to_point_to_lat_lng(self):
        cid = s2rst.CellId.from_face(0)
        p = cid.to_point()
        assert isinstance(p, s2rst.S2Point)
        ll = cid.to_lat_lng()
        assert isinstance(ll, s2rst.LatLng)

    def test_edge_neighbors(self):
        cid = s2rst.CellId.from_face(0)
        neighbors = cid.edge_neighbors()
        assert len(neighbors) == 4

    def test_vertex_neighbors(self):
        cid = s2rst.CellId.from_face(0)
        neighbors = cid.vertex_neighbors(0)
        assert len(neighbors) >= 1

    def test_common_ancestor_level(self):
        parent = s2rst.CellId.from_face(0)
        child = parent.children()[0]
        level = parent.common_ancestor_level(child)
        assert level == 0

    def test_hash(self):
        cid = s2rst.CellId.from_face(0)
        assert hash(cid) == cid.id

    def test_int(self):
        cid = s2rst.CellId.from_face(0)
        assert int(cid) == cid.id

    def test_comparisons(self):
        a = s2rst.CellId.from_face(0)
        b = s2rst.CellId.from_face(1)
        assert a != b
        assert (a < b) or (a > b)

    def test_repr(self):
        cid = s2rst.CellId.from_face(0)
        assert "CellId" in repr(cid)

    def test_debug_string_roundtrip(self):
        cid = s2rst.CellId.from_face(3)
        ds = cid.to_debug_string()
        cid2 = s2rst.CellId.from_debug_string(ds)
        assert cid2 is not None
        assert cid == cid2


class TestCell:
    def test_new(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        assert cell.face() == 0
        assert cell.level() == 0

    def test_from_point(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        cell = s2rst.Cell.from_point(p)
        assert cell.is_leaf()
        assert cell.level() == 30

    def test_from_lat_lng(self):
        ll = s2rst.LatLng.from_degrees(45.0, 90.0)
        cell = s2rst.Cell.from_lat_lng(ll)
        assert cell.is_leaf()

    def test_id(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        assert cell.id() == cid

    def test_vertex(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        for k in range(4):
            v = cell.vertex(k)
            assert isinstance(v, s2rst.S2Point)

    def test_edge(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        for k in range(4):
            e = cell.edge(k)
            assert isinstance(e, s2rst.S2Point)

    def test_center(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        c = cell.center()
        assert c.is_unit()

    def test_children(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        children = cell.children()
        assert children is not None
        assert len(children) == 4

    def test_leaf_no_children(self):
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        cell = s2rst.Cell.from_point(p)
        assert cell.children() is None

    def test_area(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        assert cell.average_area() > 0
        assert cell.approx_area() > 0

    def test_contains_point(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        center = cell.center()
        assert cell.contains_point(center)

    def test_bounds(self):
        cid = s2rst.CellId.from_face(0)
        cell = s2rst.Cell(cid)
        cap = cell.cap_bound()
        assert isinstance(cap, s2rst.Cap)
        rect = cell.rect_bound()
        assert isinstance(rect, s2rst.Rect)
        cub = cell.cell_union_bound()
        assert len(cub) >= 1


class TestCellUnion:
    def test_empty(self):
        cu = s2rst.CellUnion()
        assert cu.num_cells() == 0
        assert len(cu) == 0

    def test_from_cell_ids(self):
        ids = [s2rst.CellId.from_face(i) for i in range(6)]
        cu = s2rst.CellUnion.from_cell_ids(ids)
        assert cu.num_cells() == 6

    def test_cell_ids(self):
        ids = [s2rst.CellId.from_face(0)]
        cu = s2rst.CellUnion.from_cell_ids(ids)
        result = cu.cell_ids()
        assert len(result) == 1
        assert result[0] == ids[0]

    def test_contains_cell_id(self):
        face0 = s2rst.CellId.from_face(0)
        cu = s2rst.CellUnion.from_cell_ids([face0])
        child = face0.children()[0]
        assert cu.contains_cell_id(child)
        assert not cu.contains_cell_id(s2rst.CellId.from_face(1))

    def test_contains_point(self):
        face0 = s2rst.CellId.from_face(0)
        cu = s2rst.CellUnion.from_cell_ids([face0])
        p = s2rst.S2Point(1.0, 0.0, 0.0)
        assert cu.contains_point(p)

    def test_intersects_cell_id(self):
        face0 = s2rst.CellId.from_face(0)
        cu = s2rst.CellUnion.from_cell_ids([face0])
        child = face0.children()[0]
        assert cu.intersects_cell_id(child)

    def test_contains_union(self):
        all_faces = s2rst.CellUnion.from_cell_ids(
            [s2rst.CellId.from_face(i) for i in range(6)]
        )
        one_face = s2rst.CellUnion.from_cell_ids([s2rst.CellId.from_face(0)])
        assert all_faces.contains_union(one_face)
        assert not one_face.contains_union(all_faces)

    def test_normalize(self):
        face0 = s2rst.CellId.from_face(0)
        children = face0.children()
        cu = s2rst.CellUnion.from_cell_ids(list(children))
        assert cu.is_normalized()

    def test_getitem(self):
        ids = [s2rst.CellId.from_face(0), s2rst.CellId.from_face(1)]
        cu = s2rst.CellUnion.from_cell_ids(ids)
        assert cu[0] == ids[0]
        with pytest.raises(IndexError):
            cu[10]

    def test_contains_operator(self):
        face0 = s2rst.CellId.from_face(0)
        cu = s2rst.CellUnion.from_cell_ids([face0])
        child = face0.children()[0]
        assert child in cu

    def test_iter(self):
        ids = [s2rst.CellId.from_face(i) for i in range(3)]
        cu = s2rst.CellUnion.from_cell_ids(ids)
        collected = list(cu)
        assert len(collected) == 3

    def test_eq(self):
        ids = [s2rst.CellId.from_face(0)]
        a = s2rst.CellUnion.from_cell_ids(ids)
        b = s2rst.CellUnion.from_cell_ids(ids)
        assert a == b

    def test_repr(self):
        cu = s2rst.CellUnion.from_cell_ids([s2rst.CellId.from_face(0)])
        assert "CellUnion" in repr(cu)
