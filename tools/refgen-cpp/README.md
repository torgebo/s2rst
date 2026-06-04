# refgen-cpp — C++ reference-vector generator for CellId differential tests

Generates [`core/tests/data/cpp_cellid_vectors.csv`](../../core/tests/data/cpp_cellid_vectors.csv),
which [`core/tests/cpp_cellid_diff.rs`](../../core/tests/cpp_cellid_diff.rs) diffs
the Rust `CellId` implementation against. It uses the **C++ reference**
[google/s2geometry](https://github.com/google/s2geometry) as the oracle, so the
tests catch *wrong answers* (a mis-stepped Hilbert curve, a bad neighbour) by
comparison against the canonical implementation.

The committed CSV is the durable artifact — CI runs the differential test with no
C++ toolchain involved. You only need this generator to extend or regenerate it.

## Build the C++ reference (once)

s2geometry needs abseil; the easiest path fetches and builds it:

```sh
cd /path/to/s2geometry
cmake -B build -DCMAKE_BUILD_TYPE=Release -DFETCH_ABSEIL=ON \
      -DBUILD_TESTS=OFF -DBUILD_EXAMPLES=OFF -DWITH_PYTHON=OFF \
      -DS2_ENABLE_INSTALL=ON -DCMAKE_INSTALL_PREFIX=/tmp/s2-install
cmake --build build -j"$(nproc)"
cmake --install build
```

## Build and run the generator

```sh
cd tools/refgen-cpp
cmake -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install
cmake --build build
./build/generate_cellid > ../../core/tests/data/cpp_cellid_vectors.csv
```

Output is deterministic (fixed RNG seed). After changing the generator, regenerate
and run `cargo test -p s2rst --test cpp_cellid_diff`.

## Sections emitted

| Section                  | Rust method(s) checked              | Oracle (S2CellId)       |
|--------------------------|-------------------------------------|-------------------------|
| `CELLID_EDGE_NEIGHBORS`  | `edge_neighbors`                    | `GetEdgeNeighbors`      |
| `CELLID_RANGE`           | `range_min`, `range_max`            | `range_min`, `range_max`|
| `CELLID_CONTAINS`        | `contains`, `intersects`            | `contains`, `intersects`|
| `CELLID_CHILDREN`        | `children`                          | `child`                 |
| `CELLID_FACE_IJ`         | `to_face_ij_orientation`            | `ToFaceIJOrientation`   |

All values are exact `u64` cell ids, compared exactly.
