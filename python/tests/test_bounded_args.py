# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Red/green tests for spec 009's bounded arguments.

Each test failed (red) on the unfixed code, with `pyo3_runtime.PanicException`
(a Rust panic crossing FFI, which `except Exception` does not catch), with an
`OverflowError` for some values and not others, or with a silently clamped or
invalid result. After the fix, every out-of-range value of a kind raises the
same native exception, with the core's `OutOfRange` text. The tests are
numbered as in `specs/009-bounded-types/contracts/red-green-tests.md`.
"""

import re

import pytest

import s2rst

# ---------------------------------------------------------------------------
# Fixtures shared by the kinds
# ---------------------------------------------------------------------------

FACE0 = s2rst.CellId.from_face(0)
LEAF = FACE0.child_begin_at_level(30)


def _small_polygon():
    center = s2rst.LatLng.from_degrees(10.0, 10.0).to_point()
    return s2rst.Polygon(
        [s2rst.Loop.make_regular(center, s2rst.Angle.from_degrees(1.0), 4)]
    )


def _leaf_union():
    return s2rst.CellUnion.from_cell_ids([LEAF])


def _expand(level):
    cu = _leaf_union()
    cu.expand_at_level(level)
    return cu


def _assert_native(call):
    """PY-10: whatever `call` raises is an `Exception`, never a bare `BaseException`."""
    try:
        call()
    except BaseException as e:  # noqa: BLE001 - the point of the test
        assert isinstance(e, Exception), f"{type(e).__module__}.{type(e).__name__}: {e}"


# ---------------------------------------------------------------------------
# Levels (PY-01 to PY-04, PY-10): every method taking a level
# ---------------------------------------------------------------------------

# (label, call taking the level, levels that are legal for this call). The legal levels
# respect each method's other preconditions (a level no finer than the cell's, and so on),
# which are relations and not part of the level's range.
LEVEL_METHODS = [
    ("CellId.parent_at_level", lambda lv: LEAF.parent_at_level(lv), [0, 30]),
    ("CellId.child_position", lambda lv: LEAF.child_position(lv), [1, 30]),
    ("CellId.child_begin_at_level", lambda lv: FACE0.child_begin_at_level(lv), [0, 30]),
    ("CellId.child_end_at_level", lambda lv: FACE0.child_end_at_level(lv), [0, 30]),
    ("CellId.vertex_neighbors", lambda lv: LEAF.vertex_neighbors(lv), [0, 29]),
    ("CellId.all_neighbors", lambda lv: LEAF.all_neighbors(lv), [30]),
    (
        "CellId.from_face_pos_level",
        lambda lv: s2rst.CellId.from_face_pos_level(0, 0, lv),
        [0, 30],
    ),
    ("CellUnion.expand_at_level", _expand, [0, 30]),
    ("CellUnion.denormalize", lambda lv: _leaf_union().denormalize(lv, 1), [0, 30]),
    (
        "RegionCoverer(min_level=)",
        lambda lv: s2rst.RegionCoverer(min_level=lv, max_level=30),
        [0, 30],
    ),
    (
        "RegionCoverer(max_level=)",
        lambda lv: s2rst.RegionCoverer(min_level=0, max_level=lv),
        [0, 30],
    ),
    (
        "RegionTermIndexer(min_level=)",
        lambda lv: s2rst.RegionTermIndexer(min_level=lv, max_level=30),
        [0, 30],
    ),
    (
        "RegionTermIndexer(max_level=)",
        lambda lv: s2rst.RegionTermIndexer(min_level=0, max_level=lv),
        [0, 30],
    ),
    ("Metric.value", lambda lv: s2rst.Metric.AVG_AREA.value(lv), [0, 30]),
    ("S2CellIdSnapFunction", lambda lv: s2rst.S2CellIdSnapFunction(lv), [0, 30]),
    (
        "Polygon.snapped",
        lambda lv: s2rst.Polygon.snapped(_small_polygon(), lv),
        [10, 30],
    ),
    (
        "Polygon.simplified",
        lambda lv: s2rst.Polygon.simplified(_small_polygon(), lv),
        [10, 30],
    ),
    ("S2Builder(snap_level=)", lambda lv: s2rst.S2Builder(snap_level=lv), [0, 30]),
]
LEVEL_IDS = [m[0] for m in LEVEL_METHODS]
BAD_LEVELS = [31, 255, 256, -1, 2**40, 2**70]
LEVEL_MESSAGE = re.escape("level must be in 0..=30")


@pytest.mark.parametrize(("label", "call", "legal"), LEVEL_METHODS, ids=LEVEL_IDS)
@pytest.mark.parametrize("level", BAD_LEVELS)
def test_py01_level_out_of_range_raises_value_error(label, call, legal, level):
    with pytest.raises(ValueError, match=LEVEL_MESSAGE):
        call(level)


@pytest.mark.parametrize(("label", "call", "legal"), LEVEL_METHODS, ids=LEVEL_IDS)
def test_py02_legal_levels_still_work(label, call, legal):
    for level in legal:
        call(level)


@pytest.mark.parametrize(("label", "call", "legal"), LEVEL_METHODS, ids=LEVEL_IDS)
@pytest.mark.parametrize("value", [3.5, "5"])
def test_py03_non_integers_raise_type_error(label, call, legal, value):
    with pytest.raises(TypeError):
        call(value)


def test_py04_bool_is_an_integer():
    assert LEAF.parent_at_level(True).level() == 1


@pytest.mark.parametrize(("label", "call", "legal"), LEVEL_METHODS, ids=LEVEL_IDS)
def test_py10_level_errors_are_exceptions(label, call, legal):
    for level in BAD_LEVELS:
        _assert_native(lambda: call(level))


# ---------------------------------------------------------------------------
# Faces (PY-05, PY-10): every method taking a face
# ---------------------------------------------------------------------------

FACE_METHODS = [
    ("CellId.from_face", lambda f: s2rst.CellId.from_face(f)),
    ("CellId.from_face_pos_level", lambda f: s2rst.CellId.from_face_pos_level(f, 0, 0)),
    ("face_uv_to_xyz", lambda f: s2rst.face_uv_to_xyz(f, 0.0, 0.0)),
    ("get_u_axis", lambda f: s2rst.get_u_axis(f)),
    ("get_v_axis", lambda f: s2rst.get_v_axis(f)),
]
FACE_IDS = [m[0] for m in FACE_METHODS]
BAD_FACES = [6, 7, 255, -1, 256, 2**70]
FACE_MESSAGE = re.escape("face must be in 0..=5")


@pytest.mark.parametrize(("label", "call"), FACE_METHODS, ids=FACE_IDS)
@pytest.mark.parametrize("face", BAD_FACES)
def test_py05_face_out_of_range_raises_value_error(label, call, face):
    with pytest.raises(ValueError, match=FACE_MESSAGE):
        call(face)


@pytest.mark.parametrize(("label", "call"), FACE_METHODS, ids=FACE_IDS)
def test_py05_every_face_still_works(label, call):
    for face in range(6):
        call(face)


def test_py05_from_face_gives_valid_cells():
    for face in range(6):
        assert s2rst.CellId.from_face(face).is_valid()
        assert s2rst.CellId.from_face(face).face() == face


@pytest.mark.parametrize(("label", "call"), FACE_METHODS, ids=FACE_IDS)
def test_py10_face_errors_are_exceptions(label, call):
    for face in BAD_FACES:
        _assert_native(lambda: call(face))


# ---------------------------------------------------------------------------
# Level moduli and the coverer's levels (PY-06, PY-07, PY-08)
# ---------------------------------------------------------------------------

LEVEL_MOD_MESSAGE = re.escape("level_mod must be in 1..=3")
BAD_LEVEL_MODS = [0, 4, 255, -1, 2**40]


@pytest.mark.parametrize(
    ("label", "make"),
    [
        ("RegionCoverer", lambda m: s2rst.RegionCoverer(level_mod=m)),
        ("RegionTermIndexer", lambda m: s2rst.RegionTermIndexer(level_mod=m)),
        ("CellUnion.denormalize", lambda m: _leaf_union().denormalize(0, m)),
    ],
    ids=["RegionCoverer", "RegionTermIndexer", "CellUnion.denormalize"],
)
@pytest.mark.parametrize("level_mod", BAD_LEVEL_MODS)
def test_py06_py07_level_mod_out_of_range_raises_value_error(label, make, level_mod):
    with pytest.raises(ValueError, match=LEVEL_MOD_MESSAGE):
        make(level_mod)


def test_py06_level_mod_in_range_still_works():
    for m in (1, 2, 3):
        assert s2rst.RegionCoverer(level_mod=m).level_mod == m
        assert s2rst.RegionTermIndexer(level_mod=m).level_mod == m
        _leaf_union().denormalize(0, m)


@pytest.mark.parametrize(
    "make",
    [
        lambda: s2rst.RegionCoverer(min_level=10, max_level=5),
        lambda: s2rst.RegionTermIndexer(min_level=10, max_level=5),
    ],
    ids=["RegionCoverer", "RegionTermIndexer"],
)
def test_py08_min_level_above_max_level_raises_value_error(make):
    with pytest.raises(
        ValueError, match=re.escape("min_level (10) must not exceed max_level (5)")
    ):
        make()


def test_py08_equal_levels_still_work():
    s2rst.RegionCoverer(min_level=7, max_level=7)
    s2rst.RegionTermIndexer(min_level=7, max_level=7)


def test_py10_level_mod_errors_are_exceptions():
    for m in BAD_LEVEL_MODS:
        _assert_native(lambda: s2rst.RegionCoverer(level_mod=m))
        _assert_native(lambda: s2rst.RegionTermIndexer(level_mod=m))


# ---------------------------------------------------------------------------
# Cell vertex and edge indices (PY-09)
# ---------------------------------------------------------------------------


def _cell():
    return s2rst.Cell(s2rst.CellId.from_face(0))


@pytest.mark.parametrize(
    ("label", "call", "message"),
    [
        (
            "Cell.vertex",
            lambda k: _cell().vertex(k),
            "cell vertex index must be in 0..=3",
        ),
        (
            "Cell.vertex_raw",
            lambda k: _cell().vertex_raw(k),
            "cell vertex index must be in 0..=3",
        ),
        ("Cell.edge", lambda k: _cell().edge(k), "cell edge index must be in 0..=3"),
        (
            "Cell.edge_raw",
            lambda k: _cell().edge_raw(k),
            "cell edge index must be in 0..=3",
        ),
    ],
    ids=["vertex", "vertex_raw", "edge", "edge_raw"],
)
@pytest.mark.parametrize("k", [4, -1, 2**70])
def test_py09_index_out_of_range_raises_index_error(label, call, message, k):
    with pytest.raises(IndexError, match=re.escape(message)):
        call(k)


def test_py09_indices_in_range_still_work():
    cell = _cell()
    for k in range(4):
        cell.vertex(k)
        cell.vertex_raw(k)
        cell.edge(k)
        cell.edge_raw(k)
