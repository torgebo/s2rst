# s2rst — Python bindings

Python bindings for [s2rst](https://github.com/torgebo/s2rst), a Rust port of
[Google's S2 Geometry library](https://s2geometry.io/).

S2 is a library for spherical geometry — geometry on the surface of a sphere.
This package exposes the core S2 types (points, cells, caps, polygons, ...) to
Python through a thin [PyO3](https://pyo3.rs/) wrapper.

## Install

The package is built with [maturin](https://www.maturin.rs/):

```
pip install maturin
maturin develop --release
```

(or, for a release wheel: `maturin build --release`).

## Quick example

```python
import s2rst

# Create a point on the sphere from latitude/longitude (in degrees).
ll = s2rst.LatLng.from_degrees(37.7749, -122.4194)
p = ll.to_point()

# Find the leaf cell containing the point.
cell = s2rst.CellId.from_point(p)
print(cell.to_token())           # e.g. "808f7c"
print(cell.level())              # 30 (leaf)

# Walk up to a coarser level.
parent = cell.parent_at_level(10)
print(parent.to_debug_string())  # face/childpath form

# Build a circular cap (disc) of 10 km angular radius around the point.
radius = s2rst.Angle.from_radians(10.0 / 6371.0)   # 10 km / earth radius
cap = s2rst.Cap.from_center_angle(p, radius)
print(cap.area())                                   # steradians
```

## Type checking

The package ships PEP 561 type stubs (`py.typed` + `_core.pyi`) and is
recognised as a typed package by mypy and pyright.

## License

Licensed under the [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).

s2rst is an independent port and is not affiliated with, endorsed by, or
sponsored by Google.
