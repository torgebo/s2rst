# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for Edge, ReferencePoint, Shape, LaxLoop, LaxPolyline, LaxPolygon,
PointVector, EdgeVectorShape."""

import pytest
import s2rst


class TestEdge:
    def test_new(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        e = s2rst.Edge(a, b)
        assert e.v0.approx_eq(a)
        assert e.v1.approx_eq(b)

    def test_reversed(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        e = s2rst.Edge(a, b)
        r = e.reversed()
        assert r.v0.approx_eq(b)
        assert r.v1.approx_eq(a)

    def test_is_degenerate(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        e = s2rst.Edge(a, a)
        assert e.is_degenerate()

    def test_eq(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        e1 = s2rst.Edge(a, b)
        e2 = s2rst.Edge(a, b)
        assert e1 == e2

    def test_repr(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        e = s2rst.Edge(a, b)
        assert "Edge" in repr(e)


class TestLaxLoop:
    def test_new(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        assert ll.num_vertices() == 3

    def test_vertex(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        v = ll.vertex(0)
        assert v.approx_eq(pts[0])

    def test_as_shape(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        s = ll.as_shape()
        assert s.dimension() == 2
        assert s.num_edges() == 3
        assert s.num_chains() == 1
        assert s.has_interior()

    def test_len_getitem(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        ll = s2rst.LaxLoop(pts)
        assert len(ll) == 2
        assert ll[0].approx_eq(pts[0])
        with pytest.raises(IndexError):
            ll[5]


class TestLaxPolyline:
    def test_new(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        lp = s2rst.LaxPolyline(pts)
        assert lp.num_vertices() == 3

    def test_as_shape(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        lp = s2rst.LaxPolyline(pts)
        s = lp.as_shape()
        assert s.dimension() == 1
        assert s.num_edges() == 2
        assert not s.has_interior()


class TestLaxPolygon:
    def test_empty(self):
        lp = s2rst.LaxPolygon.empty()
        assert lp.num_loops() == 0
        assert lp.num_vertices() == 0

    def test_full(self):
        lp = s2rst.LaxPolygon.full()
        assert lp.num_loops() == 1
        s = lp.as_shape()
        assert s.is_full()

    def test_new(self):
        loop = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        lp = s2rst.LaxPolygon([loop])
        assert lp.num_loops() == 1
        assert lp.num_vertices() == 3

    def test_loop_vertex(self):
        loop = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        lp = s2rst.LaxPolygon([loop])
        v = lp.loop_vertex(0, 0)
        assert v.approx_eq(loop[0])

    def test_as_shape(self):
        loop = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        lp = s2rst.LaxPolygon([loop])
        s = lp.as_shape()
        assert s.dimension() == 2
        assert s.num_edges() == 3
        assert s.has_interior()


class TestPointVector:
    def test_new(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pv = s2rst.PointVector(pts)
        assert len(pv) == 2

    def test_point(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pv = s2rst.PointVector(pts)
        assert pv.point(0).approx_eq(pts[0])

    def test_as_shape(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pv = s2rst.PointVector(pts)
        s = pv.as_shape()
        assert s.dimension() == 0
        assert s.num_edges() == 2  # each point is a degenerate edge

    def test_getitem(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
        ]
        pv = s2rst.PointVector(pts)
        assert pv[0].approx_eq(pts[0])
        with pytest.raises(IndexError):
            pv[5]


class TestEdgeVectorShape:
    def test_new_empty(self):
        evs = s2rst.EdgeVectorShape()
        assert len(evs) == 0

    def test_add(self):
        evs = s2rst.EdgeVectorShape()
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        evs.add(a, b)
        assert len(evs) == 1

    def test_from_edge(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        evs = s2rst.EdgeVectorShape.from_edge(a, b)
        assert len(evs) == 1

    def test_from_edges(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        c = s2rst.S2Point(0.0, 0.0, 1.0)
        edges = [s2rst.Edge(a, b), s2rst.Edge(b, c)]
        evs = s2rst.EdgeVectorShape.from_edges(edges)
        assert len(evs) == 2

    def test_as_shape(self):
        a = s2rst.S2Point(1.0, 0.0, 0.0)
        b = s2rst.S2Point(0.0, 1.0, 0.0)
        evs = s2rst.EdgeVectorShape.from_edge(a, b)
        s = evs.as_shape()
        assert s.dimension() == 1
        assert s.num_edges() == 1

    def test_set_dimension(self):
        evs = s2rst.EdgeVectorShape()
        evs.set_dimension(2)
        s = evs.as_shape()
        assert s.dimension() == 2


class TestShape:
    def test_edge(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        s = ll.as_shape()
        e = s.edge(0)
        assert isinstance(e, s2rst.Edge)

    def test_chain(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        s = ll.as_shape()
        start, length = s.chain(0)
        assert start == 0
        assert length == 3

    def test_chain_edge(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        s = ll.as_shape()
        e = s.chain_edge(0, 0)
        assert isinstance(e, s2rst.Edge)

    def test_chain_position(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        s = ll.as_shape()
        chain_id, offset = s.chain_position(1)
        assert chain_id == 0
        assert offset == 1

    def test_reference_point(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        s = ll.as_shape()
        rp = s.reference_point()
        assert isinstance(rp, s2rst.ReferencePoint)
        assert isinstance(rp.point, s2rst.S2Point)
        assert isinstance(rp.contained, bool)

    def test_is_empty_full(self):
        empty = s2rst.LaxPolygon.empty().as_shape()
        assert empty.is_empty()
        assert not empty.is_full()
        full = s2rst.LaxPolygon.full().as_shape()
        assert full.is_full()
        assert not full.is_empty()

    def test_repr(self):
        pts = [
            s2rst.S2Point(1.0, 0.0, 0.0),
            s2rst.S2Point(0.0, 1.0, 0.0),
            s2rst.S2Point(0.0, 0.0, 1.0),
        ]
        ll = s2rst.LaxLoop(pts)
        s = ll.as_shape()
        r = repr(s)
        assert "LaxLoop" in r
        assert "dim=2" in r
