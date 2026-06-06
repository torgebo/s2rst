# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for ChainInterpolationQuery, EdgeTessellator, and density."""

import pytest

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


# --- ChainInterpolationQuery ---


def _line_shape():
    return s2rst.LaxPolyline([_point(0, 0), _point(0, 1), _point(0, 2)]).as_shape()


def test_chain_interp_length_and_endpoints():
    q = s2rst.ChainInterpolationQuery(_line_shape())
    assert q.get_length().degrees == pytest.approx(2.0, abs=1e-6)
    assert q.at_fraction(0.0).point.approx_eq(_point(0, 0))
    assert q.at_fraction(1.0).point.approx_eq(_point(0, 2))


def test_chain_interp_midpoint():
    q = s2rst.ChainInterpolationQuery(_line_shape())
    mid = q.at_fraction(0.5)
    assert s2rst.LatLng.from_point(mid.point).lng.degrees == pytest.approx(
        1.0, abs=1e-6
    )
    assert mid.distance.degrees == pytest.approx(1.0, abs=1e-6)


def test_chain_interp_at_distance_and_slice():
    q = s2rst.ChainInterpolationQuery(_line_shape())
    r = q.at_distance(s2rst.Angle.from_degrees(0.5))
    assert s2rst.LatLng.from_point(r.point).lng.degrees == pytest.approx(0.5, abs=1e-6)
    assert len(q.slice(0.0, 1.0)) >= 2


def test_chain_interp_empty_shape():
    q = s2rst.ChainInterpolationQuery(s2rst.PointVector([]).as_shape())
    assert q.at_fraction(0.5) is None


# --- EdgeTessellator ---


def test_tessellator_projected():
    t = s2rst.EdgeTessellator(
        s2rst.PlateCarreeProjection(180), s2rst.Angle.from_degrees(0.1)
    )
    pts = t.append_projected(_point(0, 0), _point(20, 20))
    assert len(pts) >= 2
    assert all(isinstance(p, s2rst.R2Point) for p in pts)


def test_tessellator_unprojected_roundtrip():
    proj = s2rst.PlateCarreeProjection(180)
    t = s2rst.EdgeTessellator(proj, s2rst.Angle.from_degrees(0.5))
    a, b = s2rst.R2Point(0, 0), s2rst.R2Point(30, 30)
    pts = t.append_unprojected(a, b)
    assert len(pts) >= 2
    # Endpoints project back near the inputs.
    assert proj.from_lat_lng(s2rst.LatLng.from_point(pts[0])).x == pytest.approx(
        0, abs=1e-6
    )


def test_tessellator_bad_projection():
    with pytest.raises(TypeError):
        s2rst.EdgeTessellator("nope", s2rst.Angle.from_degrees(0.1))


# --- Density ---


def _density_index():
    idx = s2rst.ShapeIndex()
    idx.add(s2rst.PointVector([_point(i * 2.0, i * 2.0) for i in range(10)]))
    idx.build()
    return idx


def test_density_tree_and_clusters():
    tree = s2rst.S2DensityTree()
    assert tree.is_empty()
    tree.init_to_vertex_density(_density_index(), 10000, 20)
    assert not tree.is_empty()
    assert tree.encoded_size() > 0
    coverings = s2rst.DensityClusterQuery(5).coverings(tree)
    assert len(coverings) >= 1
    assert all(isinstance(cu, s2rst.CellUnion) for cu in coverings)
