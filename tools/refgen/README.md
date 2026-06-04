# refgen — reference-vector generator for differential tests

Generates the CSV in [`core/tests/data/go_test_vectors.csv`](../../core/tests/data/go_test_vectors.csv)
that [`core/tests/go_cross_validation.rs`](../../core/tests/go_cross_validation.rs)
diffs the Rust implementation against. It uses the Go S2 port
[`github.com/golang/geo`](https://github.com/golang/geo) as an **independent
oracle**, so the tests catch *wrong answers* a same-implementation unit test
can't (a flipped predicate, a drifted result).

The committed CSV is the durable artifact — CI runs the differential test with no
Go toolchain involved. You only need this generator to **extend or regenerate**
the vectors.

## Requirements

- Go (1.23+).
- A local checkout of `github.com/golang/geo`. `go.mod` points at it with a
  relative `replace` (`../../../../geo`, i.e. `~/sources/geo` next to this repo's
  grandparent). Adjust that path for your layout, or drop the `replace` to fetch
  the published module over the network.

## Regenerate

```sh
cd tools/refgen
go run . > ../../core/tests/data/go_test_vectors.csv
```

Output is deterministic (fixed RNG seed), so regenerating without source changes
produces a byte-identical file. After changing the generator, regenerate and run
`cargo test -p s2rst --test go_cross_validation`.

## Sections currently emitted

| Section         | Rust function(s) checked                                      | Oracle (golang/geo)                          | Comparison |
|-----------------|--------------------------------------------------------------|----------------------------------------------|------------|
| `ROBUST_SIGN`   | `s2::predicates::robust_sign`                                | `s2.RobustSign`                              | exact      |
| `EDGE_CROSSING` | `s2::edge_crossings::crossing_sign` + `edge_or_vertex_crossing` | `s2.CrossingSign` + `s2.EdgeOrVertexCrossing` | exact   |
| `POINT_AREA`    | `s2::point_measures::point_area`                             | `s2.PointArea`                               | tolerance  |
| `EDGE_DISTANCE` | `s2::edge_distances::distance_from_segment`                  | `s2.DistanceFromSegment`                     | tolerance  |
| `CLOSEST_EDGE`  | `s2::closest_edge_query::ClosestEdgeQuery` over a `ShapeIndex` | `s2.ClosestEdgeQuery`                      | tolerance  |
| `POLYLINE_LENGTH` | `s2::polyline::Polyline::length`                           | `s2.Polyline.Length`                         | tolerance  |
| `LOOP_AREA`     | `s2::Loop::area`                                             | `s2.Loop.Area`                               | tolerance  |

Points are written at full f64 precision and reconstructed in the test with the
non-normalizing `Point::new`, so exact predicates match bit-for-bit; the numeric
measures (areas, distances) are compared within a small tolerance, since Go and
Rust use different libm. Add new sections by appending to `main.go` and a matching
`#[test]` in `go_cross_validation.rs`.
