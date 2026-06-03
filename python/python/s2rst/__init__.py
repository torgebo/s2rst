# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

"""Python bindings for the s2rst spherical geometry library."""

from collections.abc import Sequence

from ._core import (
    __version__,
    Angle,
    Cap,
    Cell,
    CellId,
    CellUnion,
    ChordAngle,
    Edge,
    EdgeVectorShape,
    LatLng,
    LaxLoop,
    LaxPolygon,
    LaxPolyline,
    Loop,
    Matrix3x3,
    PointVector,
    Polygon,
    Polyline,
    R1Interval,
    R2Point,
    R2Rect,
    Rect,
    ReferencePoint,
    S1Interval,
    S2Point,
    Shape,
    Vector,
    s2_ortho,
    s2_rotate,
)

# Register variable-size sequence types as Sequence virtual subclasses so
# `isinstance(x, Sequence)` returns True. Sequence has no structural
# __subclasshook__, so this registration is required even though every type
# below already has __len__, __getitem__, and __iter__.
for _cls in (
    Polyline,
    Loop,
    Polygon,
    LaxLoop,
    LaxPolyline,
    PointVector,
    EdgeVectorShape,
    CellUnion,
):
    Sequence.register(_cls)
del _cls

__all__ = [
    "Angle",
    "Cap",
    "Cell",
    "CellId",
    "CellUnion",
    "ChordAngle",
    "Edge",
    "EdgeVectorShape",
    "LatLng",
    "LaxLoop",
    "LaxPolygon",
    "LaxPolyline",
    "Loop",
    "Matrix3x3",
    "PointVector",
    "Polygon",
    "Polyline",
    "R1Interval",
    "R2Point",
    "R2Rect",
    "Rect",
    "ReferencePoint",
    "S1Interval",
    "S2Point",
    "Shape",
    "Vector",
    "__version__",
    "s2_ortho",
    "s2_rotate",
]
