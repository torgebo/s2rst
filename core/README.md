# s2rst

A Rust port of [Google's S2 Geometry library](https://s2geometry.io/) — robust,
efficient spherical geometry on the unit sphere (S²).

S2 represents points, regions, and shapes on the surface of a sphere using
unit-length 3D vectors rather than latitude/longitude pairs. This avoids
singularities at the poles and discontinuities at the antimeridian, and makes
geodesic-edge operations behave consistently everywhere on the sphere. It also
decomposes the sphere into a hierarchy of cells ordered along a Hilbert
space-filling curve, enabling fast spatial indexing and queries.

## Installation

```toml
[dependencies]
s2rst = "0.1"
```

## Quick example

```rust
use s2rst::s1::Angle;
use s2rst::s2::{Cap, LatLng, Region};
use s2rst::s2::region_coverer::RegionCoverer;

// Define a spherical cap (disc) centered on Paris.
let center = LatLng::from_degrees(48.8566, 2.3522).to_point();
let cap = Cap::from_center_angle(center, Angle::from_degrees(0.5));

// Approximate the cap with a covering of S2 cells.
let coverer = RegionCoverer::new().max_level(14).max_cells(8);
let covering = coverer.covering(&cap);
assert!(!covering.is_empty());

// Every point inside the cap is contained by the covering.
assert!(covering.contains_point(center));
```

## Modules

The crate is organized into modules corresponding to the mathematical spaces
they operate in. Most users will work primarily with the types in `s2`.

| Module | Space | Key types |
|--------|-------|-----------|
| `r1`   | Real line (ℝ¹)        | `Interval` |
| `r2`   | Euclidean plane (ℝ²)  | `Point`, `Rect` |
| `r3`   | Euclidean 3-space (ℝ³)| `Vector`, `PreciseVector`, `ExactFloat` |
| `s1`   | Unit circle (S¹)      | `Angle`, `ChordAngle`, `Interval` |
| `s2`   | Unit sphere (S²)      | `Point`, `CellId`, `Cell`, `Loop`, `Polygon`, and many more |

## Feature flags

- `serde` — `Serialize`/`Deserialize` implementations for the core types.
- `geo-types` — conversions to and from the
  [`geo-types`](https://crates.io/crates/geo-types) ecosystem.

## License

Licensed under the [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0),
the same license as the upstream S2 geometry library this project is derived
from. See the bundled `LICENSE`, `NOTICE`, and `AUTHORS` files for the full text
and attribution.

s2rst is an independent port and is not affiliated with, endorsed by, or
sponsored by Google; S2 is a trademark of Google LLC.
