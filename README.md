# s2rst

A Rust port of [Google's S2 Geometry library](https://s2geometry.io/), with
optional Python bindings (via [PyO3](https://pyo3.rs/)) and a WebAssembly
target.

The provided software should be considered beta, and should not be used in production.

S2 is a library for spherical geometry — geometry on the surface of a sphere.
It provides exact, efficient representations of points, regions, and shapes
on the sphere, and supports operations like containment tests, set operations,
nearest-neighbour queries, and discrete cell decomposition over a hierarchical
quad-tree (the S2 cell hierarchy).

## Crates

| Path     | Crate            | Description                                    |
|----------|------------------|------------------------------------------------|
| `core/`  | `s2rst`          | The Rust core library.                         |
| `python/`| `s2rst-python`   | Python bindings, exposed as the `s2rst` module.|
| `wasm/`  | `s2rst-wasm`     | WebAssembly bindings.                          |

## Building

The Rust core builds with stable Rust:

```
cargo build --release -p s2rst
cargo test  --release -p s2rst
```

## Python bindings

The Python package is built with [maturin](https://www.maturin.rs/):

```
cd python
uv sync --extra test --group dev
uv run maturin develop --release
uv run pytest
```

### Quick example

```python
import s2rst

# Create a point on the sphere from latitude/longitude (in degrees).
ll = s2rst.LatLng.from_degrees(37.7749, -122.4194)
p = ll.to_point()

# Find the leaf cell containing the point.
cell = s2rst.CellId.from_point(p)
print(cell.to_token())          # hex token, e.g. "808f7c"
print(cell.level())              # 30 (leaf)

# Walk up to a coarser level.
parent = cell.parent_at_level(10)
print(parent.to_debug_string())  # face/childpath form

# Build a circular cap (disc) of 10 km angular radius around the point.
radius = s2rst.Angle.from_radians(10.0 / 6371.0)   # 10 km / earth radius
cap = s2rst.Cap.from_center_angle(p, radius)
print(cap.area())                                  # steradians
```

The Python package ships type stubs (`py.typed` + `_core.pyi`) and is
recognised as a typed package by mypy and pyright.

## License

Licensed under the [Apache License, Version 2.0](LICENSE), the same license
as the upstream S2 geometry library this project is derived from.

s2rst is an independent port and is not affiliated with, endorsed by, or
sponsored by Google; S2 is a trademark of Google LLC.
