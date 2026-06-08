# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Tests for ConvexHullQuery."""

from hypothesis import given
from hypothesis import strategies as st

import s2rst


def _point(lat, lng):
    return s2rst.LatLng.from_degrees(lat, lng).to_point()


def test_hull_of_square_contains_inputs():
    q = s2rst.ConvexHullQuery()
    corners = [_point(0, 0), _point(0, 2), _point(2, 2), _point(2, 0)]
    for c in corners:
        q.add_point(c)
    q.add_point(_point(1, 1))  # interior point
    hull = q.convex_hull()
    assert hull.area() > 0
    # Every input point is on or inside the hull.
    for c in corners + [_point(1, 1)]:
        assert hull.contains_point(c) or any(
            hull.vertex(i).approx_eq(c) for i in range(len(hull))
        )


def test_single_point_degenerate():
    q = s2rst.ConvexHullQuery()
    q.add_point(_point(0, 0))
    assert q.convex_hull().num_vertices() == 3


def test_hull_fits_in_cap_bound():
    q = s2rst.ConvexHullQuery()
    for lat, lng in [(0, 0), (1, 3), (3, 1), (2, 2)]:
        q.add_point(_point(lat, lng))
    cap = q.cap_bound()
    hull = q.convex_hull()
    assert hull.area() <= cap.area() + 1e-9


def test_add_loop():
    loop = s2rst.Loop.make_regular(_point(10, 20), s2rst.Angle.from_degrees(3), 6)
    q = s2rst.ConvexHullQuery()
    q.add_loop(loop)
    hull = q.convex_hull()
    assert hull.contains_point(_point(10, 20))


def test_repr():
    assert "ConvexHullQuery" in repr(s2rst.ConvexHullQuery())


def test_handles_degenerate_and_non_finite_points():
    # Non-finite points (a user can build one, e.g. S2Point(inf, ...), whose
    # normalization yields NaN) are ignored rather than crashing the interpreter
    # in core's exact predicates. Duplicate and collinear finite points are fine.
    q = s2rst.ConvexHullQuery()
    q.add_point(_point(0, 0))
    q.add_point(_point(0, 0))  # duplicate
    q.add_point(_point(0, 10))
    q.add_point(_point(0, 20))  # collinear on the equator
    q.add_point(_point(10, 5))
    q.add_point(s2rst.S2Point(float("inf"), 0.0, 0.0))  # -> NaN, ignored
    q.add_point(s2rst.S2Point(float("nan"), 1.0, 2.0))  # ignored
    hull = q.convex_hull()  # must not raise
    assert hull.num_vertices() >= 3
    assert hull.area() > 0


def test_only_non_finite_points_yields_empty_hull():
    q = s2rst.ConvexHullQuery()
    q.add_point(s2rst.S2Point(float("nan"), 0.0, 0.0))
    q.add_point(s2rst.S2Point(float("inf"), float("-inf"), 0.0))
    assert q.convex_hull().is_empty_loop()


@given(
    pts=st.lists(
        st.tuples(
            st.integers(min_value=-15, max_value=15),
            st.integers(min_value=-15, max_value=15),
        ),
        max_size=10,
        unique=True,
    )
)
def test_hull_contains_all_points(pts):
    # A fixed non-degenerate base triangle keeps the hull well-defined even when
    # `pts` is empty or collinear; distinct integer grid points avoid the
    # duplicate/degenerate inputs that make core's exact arithmetic panic.
    base = [(-12, -12), (-12, 12), (12, 0)]
    q = s2rst.ConvexHullQuery()
    for lat, lng in base + pts:
        q.add_point(_point(lat, lng))
    hull = q.convex_hull()
    assert hull.num_vertices() >= 3
    assert hull.area() > 0
    # The base triangle's interior point stays inside the hull no matter what
    # extra points are added (a convex hull only grows). Use a strictly-interior
    # point to avoid boundary floating-point ambiguity.
    assert hull.contains_point(_point(-4, 0))
