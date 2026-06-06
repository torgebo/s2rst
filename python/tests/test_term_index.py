# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for RegionTermIndexer and RegionSharder."""

import pytest
from hypothesis import given
from hypothesis import strategies as st

import s2rst

latitudes = st.floats(min_value=-80.0, max_value=80.0, allow_nan=False)
longitudes = st.floats(min_value=-179.0, max_value=179.0, allow_nan=False)


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def _cap(lat, lng, deg):
    return s2rst.Cap.from_center_angle(_point(lat, lng), s2rst.Angle.from_degrees(deg))


# ---------------------------------------------------------------------------
# RegionTermIndexer
# ---------------------------------------------------------------------------


def test_defaults():
    ix = s2rst.RegionTermIndexer()
    assert ix.max_cells == 8
    assert ix.min_level == 4
    assert ix.max_level == 16
    assert ix.level_mod == 1
    assert ix.index_contains_points_only is False
    assert ix.optimize_for_space is False
    assert ix.marker_character == "$"


def test_options_keyword_only():
    with pytest.raises(TypeError):
        s2rst.RegionTermIndexer(8, 4)  # positional not allowed


def test_custom_options():
    ix = s2rst.RegionTermIndexer(
        max_cells=16,
        min_level=2,
        max_level=20,
        level_mod=2,
        index_contains_points_only=True,
        optimize_for_space=True,
        marker_character="#",
    )
    assert ix.max_cells == 16
    assert ix.min_level == 2
    assert ix.max_level == 20
    assert ix.level_mod == 2
    assert ix.index_contains_points_only is True
    assert ix.optimize_for_space is True
    assert ix.marker_character == "#"


def test_invalid_level_raises():
    with pytest.raises(ValueError):
        s2rst.RegionTermIndexer(max_level=31)


def test_index_terms_for_point_are_strings():
    ix = s2rst.RegionTermIndexer()
    terms = ix.get_index_terms_for_point(_point(0, 0))
    assert len(terms) > 0
    assert all(isinstance(t, str) for t in terms)


def test_query_terms_for_point_are_strings():
    ix = s2rst.RegionTermIndexer()
    terms = ix.get_query_terms_for_point(_point(10, 20))
    assert len(terms) > 0
    assert all(isinstance(t, str) for t in terms)


def test_point_in_cap_recall():
    # Index a point, then query a small cap that contains it. The index terms
    # and query terms must overlap (so the point would be found by the query).
    ix = s2rst.RegionTermIndexer()
    p = _point(10, 20)
    index_terms = ix.get_index_terms_for_point(p)
    query_terms = ix.get_query_terms(_cap(10, 20, 1.0))
    assert set(index_terms) & set(query_terms)


def test_region_index_and_query_overlap():
    ix = s2rst.RegionTermIndexer()
    cap = _cap(0, 0, 1.0)
    index_terms = ix.get_index_terms(cap)
    query_terms = ix.get_query_terms(cap)
    assert len(index_terms) > 0
    assert len(query_terms) > 0
    assert all(isinstance(t, str) for t in index_terms)
    assert all(isinstance(t, str) for t in query_terms)
    assert set(index_terms) & set(query_terms)


def test_points_only_has_no_covering_terms():
    ix = s2rst.RegionTermIndexer(index_contains_points_only=True)
    terms = ix.get_index_terms_for_point(_point(0, 0))
    assert not any(t.startswith("$") for t in terms)


def test_custom_marker_used():
    ix = s2rst.RegionTermIndexer(marker_character="#")
    terms = ix.get_index_terms_for_point(_point(45, 45))
    assert any(t.startswith("#") for t in terms)
    assert not any(t.startswith("$") for t in terms)


def test_index_terms_accept_region_types():
    ix = s2rst.RegionTermIndexer()
    rect = s2rst.Rect.from_center_size(
        s2rst.LatLng.from_degrees(0, 0), s2rst.LatLng.from_degrees(2, 2)
    )
    cap = _cap(0, 0, 1.0)
    cov = s2rst.RegionCoverer(max_cells=8).covering(cap)
    for region in (cap, rect, cov):
        assert len(ix.get_index_terms(region)) > 0
        assert len(ix.get_query_terms(region)) > 0


def test_terms_reject_non_region():
    ix = s2rst.RegionTermIndexer()
    for bad in (42, "not a region", _point(0, 0)):
        with pytest.raises(TypeError):
            ix.get_index_terms(bad)
        with pytest.raises(TypeError):
            ix.get_query_terms(bad)


@given(lat=latitudes, lng=longitudes)
def test_recall_property(lat, lng):
    # For any point, querying a cap that contains it must recall it.
    ix = s2rst.RegionTermIndexer()
    p = _point(lat, lng)
    index_terms = ix.get_index_terms_for_point(p)
    query_terms = ix.get_query_terms(_cap(lat, lng, 1.0))
    assert set(index_terms) & set(query_terms)


# ---------------------------------------------------------------------------
# RegionSharder
# ---------------------------------------------------------------------------


def _face_shard(face):
    return s2rst.CellUnion.from_cell_ids([s2rst.CellId.from_face(face)])


def test_sharder_disjoint_single_face():
    # Two disjoint single-face shards; a region inside face 0 maps to shard 0.
    sharder = s2rst.RegionSharder([_face_shard(0), _face_shard(5)])
    region = _cap_at_face_center(0)
    assert sharder.get_intersecting_shards(region) == [0]
    assert sharder.get_most_intersecting_shard(region, -1) == 0


def test_sharder_most_intersecting_default():
    # A region inside face 2 overlaps neither shard 0 nor shard 5.
    sharder = s2rst.RegionSharder([_face_shard(0), _face_shard(5)])
    region = _cap_at_face_center(2)
    assert sharder.get_intersecting_shards(region) == []
    assert sharder.get_most_intersecting_shard(region, -1) == -1


def test_sharder_accepts_cell_union_region():
    sharder = s2rst.RegionSharder([_face_shard(0), _face_shard(5)])
    region = _face_shard(0)
    assert sharder.get_intersecting_shards(region) == [0]


def test_sharder_rejects_non_region():
    sharder = s2rst.RegionSharder([_face_shard(0)])
    for bad in (42, "not a region"):
        with pytest.raises(TypeError):
            sharder.get_intersecting_shards(bad)
        with pytest.raises(TypeError):
            sharder.get_most_intersecting_shard(bad, -1)


def _cap_at_face_center(face):
    center = s2rst.CellId.from_face(face).to_point()
    return s2rst.Cap.from_center_angle(center, s2rst.Angle.from_degrees(1.0))
