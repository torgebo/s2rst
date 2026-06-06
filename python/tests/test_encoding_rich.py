# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
"""Tests for richer S2 encoding: vector codecs and lexicons."""

import pytest

from hypothesis import given
from hypothesis import strategies as st

import s2rst


# ---------------------------------------------------------------------------
# S2Point vector codec
# ---------------------------------------------------------------------------


def _cell_center_points():
    # Cell centers round-trip exactly through the compact (CELL_IDS) format.
    return [
        s2rst.CellId.from_face(0).to_point(),
        s2rst.CellId.from_face_pos_level(1, 0, 5).to_point(),
        s2rst.CellId.from_face_pos_level(3, 1234, 20).to_point(),
    ]


def test_point_vector_roundtrip():
    pts = _cell_center_points()
    data = s2rst.encode_s2point_vector(pts)
    assert isinstance(data, bytes)
    assert s2rst.decode_s2point_vector(data) == pts


def test_point_vector_empty():
    assert s2rst.decode_s2point_vector(s2rst.encode_s2point_vector([])) == []


def test_point_vector_arbitrary_points_roundtrip():
    # Non-cell-center points are stored verbatim (exact f64 bytes).
    pts = [
        s2rst.S2Point(1.0, 2.0, 3.0),
        s2rst.S2Point(-1.0, 0.5, 0.25),
        s2rst.S2Point(0.0, 0.0, 1.0),
    ]
    assert s2rst.decode_s2point_vector(s2rst.encode_s2point_vector(pts)) == pts


def test_point_vector_decode_rejects_garbage():
    with pytest.raises(ValueError):
        s2rst.decode_s2point_vector(b"\xff\xff\xffgarbage")


# ---------------------------------------------------------------------------
# CellId vector codec
# ---------------------------------------------------------------------------


def test_cell_id_vector_roundtrip():
    ids = [s2rst.CellId.from_face(i) for i in range(6)]
    data = s2rst.encode_s2cell_id_vector(ids)
    assert isinstance(data, bytes)
    assert s2rst.decode_s2cell_id_vector(data) == ids


def test_cell_id_vector_empty():
    assert s2rst.decode_s2cell_id_vector(s2rst.encode_s2cell_id_vector([])) == []


@given(
    faces=st.lists(st.integers(min_value=0, max_value=5), min_size=0, max_size=20),
)
def test_cell_id_vector_roundtrip_property(faces):
    ids = [s2rst.CellId.from_face(f) for f in faces]
    assert s2rst.decode_s2cell_id_vector(s2rst.encode_s2cell_id_vector(ids)) == ids


@given(
    raw=st.lists(
        st.integers(min_value=0, max_value=(1 << 64) - 1), min_size=0, max_size=30
    ),
)
def test_cell_id_vector_roundtrip_random_ids(raw):
    ids = [s2rst.CellId(v) for v in raw]
    assert s2rst.decode_s2cell_id_vector(s2rst.encode_s2cell_id_vector(ids)) == ids


# ---------------------------------------------------------------------------
# SequenceLexicon
# ---------------------------------------------------------------------------


def test_sequence_lexicon_add_and_dedup():
    c1 = s2rst.CellId.from_face(1)
    c2 = s2rst.CellId.from_face(2)
    lex = s2rst.SequenceLexicon()
    i = lex.add([c1, c2])
    assert lex.sequence(i) == [c1, c2]
    # Re-adding an equal sequence returns the same id (dedup).
    assert lex.add([c1, c2]) == i
    assert lex.size() == 1


def test_sequence_lexicon_distinct_and_len():
    c1 = s2rst.CellId.from_face(1)
    c2 = s2rst.CellId.from_face(2)
    lex = s2rst.SequenceLexicon()
    i0 = lex.add([c1, c2])
    i1 = lex.add([c2, c1])  # order matters → distinct
    assert i0 != i1
    assert len(lex) == lex.size() == 2
    assert lex[0] == [c1, c2]
    assert lex[1] == [c2, c1]


def test_sequence_lexicon_empty_sequence():
    lex = s2rst.SequenceLexicon()
    i = lex.add([])
    assert i == 0
    assert lex.sequence(i) == []
    assert lex[0] == []


def test_sequence_lexicon_negative_index():
    c1 = s2rst.CellId.from_face(1)
    lex = s2rst.SequenceLexicon()
    lex.add([c1])
    assert lex[-1] == [c1]


def test_sequence_lexicon_index_error():
    lex = s2rst.SequenceLexicon()
    with pytest.raises(IndexError):
        lex[0]
    with pytest.raises(IndexError):
        lex.sequence(5)


def test_sequence_lexicon_repr():
    lex = s2rst.SequenceLexicon()
    lex.add([s2rst.CellId.from_face(0)])
    assert "SequenceLexicon" in repr(lex)


# ---------------------------------------------------------------------------
# ValueLexicon
# ---------------------------------------------------------------------------


def test_value_lexicon_add_and_dedup():
    c1 = s2rst.CellId.from_face(1)
    c2 = s2rst.CellId.from_face(2)
    lex = s2rst.ValueLexicon()
    i = lex.add(c1)
    assert lex.value(i) == c1
    assert lex.add(c1) == i  # dedup
    j = lex.add(c2)
    assert j != i
    assert len(lex) == lex.size() == 2
    assert lex[0] == c1
    assert lex[1] == c2


def test_value_lexicon_negative_index():
    c1 = s2rst.CellId.from_face(3)
    lex = s2rst.ValueLexicon()
    lex.add(c1)
    assert lex[-1] == c1


def test_value_lexicon_index_error():
    lex = s2rst.ValueLexicon()
    with pytest.raises(IndexError):
        lex[0]
    with pytest.raises(IndexError):
        lex.value(5)


@given(faces=st.lists(st.integers(min_value=0, max_value=5), min_size=1, max_size=20))
def test_value_lexicon_dedup_property(faces):
    lex = s2rst.ValueLexicon()
    seen = {}
    for f in faces:
        cid = s2rst.CellId.from_face(f)
        idx = lex.add(cid)
        if f in seen:
            assert idx == seen[f]
        else:
            seen[f] = idx
            assert lex.value(idx) == cid
    assert lex.size() == len(seen)


# ---------------------------------------------------------------------------
# EncodedS2ShapeIndex
# ---------------------------------------------------------------------------


def test_encoded_shape_index_rejects_garbage():
    with pytest.raises(ValueError):
        s2rst.EncodedS2ShapeIndex.from_bytes(b"\x00\x01\x02garbage")
