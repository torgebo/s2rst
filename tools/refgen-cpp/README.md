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

## Boolean-op oracle (`generate_boolean_op`)

A second oracle, for the `graph_edge_clipper` boolean-op bug (see `BUG.md` §2).
The 13 near-degenerate regression inputs are lifted verbatim from the Rust tests
and run through the C++ reference:

```sh
# 1. Extract the inputs from the Rust tests (single source of truth).
python3 tools/refgen-cpp/extract_boolean_op_inputs.py
#    -> core/tests/data/boolean_op_inputs.txt

# 2. Build + run the C++ oracle (needs /tmp/s2-install as above).
cmake --build build --target generate_boolean_op
LD_LIBRARY_PATH=/tmp/s2-install/lib \
  ./build/generate_boolean_op ../../core/tests/data/boolean_op_inputs.txt \
  > ../../core/tests/data/cpp_boolean_op_vectors.txt
```

`core/tests/cpp_boolean_op_diff.rs` then replays the inputs through the Rust
pipeline and diffs against the committed vectors (no C++ toolchain in CI).
Upstream returns a non-empty union for every case; the Rust port currently
matches only some — the remaining gap is the bug tracked in `BUG.md` §2.

Pinned oracle version: `google/s2geometry` `v0.14.0-39-g8d5c2a8`.
