# Survey probe (spec 011), 2026-10-28

The same cases on upstream C++ (`v0.14.0-40-g37a4501`, Release, x86-64, the existing build in
`/home/t/sources/s2geometry/build`) and on the library (release build, at `7910a44`). Both
programs draw their inputs from the same 64-bit LCG and print one line per case
(`<section> <index> <values as hex bits, counts or 0/1>`); `compare.py` diffs them section by
section. The comparison is committed in `compare.txt`, with each program's stderr summary
(`*_stderr.txt`); the full outputs (7.2 MB each) are not, since they regenerate exactly.

```sh
# C++ (installs the existing build; any prefix will do)
cmake --install /home/t/sources/s2geometry/build --prefix <s2-install>
cmake -S cpp -B <build> -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=<s2-install>
cmake --build <build>
LD_LIBRARY_PATH=<s2-install>/lib <build>/survey > cpp/survey_cpp.txt 2> cpp/survey_cpp_stderr.txt

# Rust
cargo run --release -q --manifest-path rust/Cargo.toml > rust/survey_rust.txt 2> rust/survey_rust_stderr.txt

python3 -I compare.py rust/survey_rust.txt cpp/survey_cpp.txt > compare.txt
```

| Section | Cases | Differ | What |
|---------|-------|--------|------|
| PC, PR | 20,000 each | 0 | `S2PointRegion`'s `GetCapBound()` and `GetRectBound()` over random unit points, bit for bit |
| PSC, PSR | 11 each | 0 | the same for special vectors: the axes, (−1, −0, 0), the zero vector, (3, 4, 0), 10⁻³⁰⁰, NaN and ∞ coordinates (upstream's constructor `DCHECK`s unit length; the Release build does not check) |
| PU, PUC | 2,000 each | 0 | `GetCellUnionBound()`, id for id, and the union of the bound contains the point (all 2,000) |
| PX | 5,000 | 0 | `Contains(cell)` (always false) and `MayIntersect(cell)`, for the point's own cell at a random level and a random cell |
| PV | 20,000 | 0 | the vertex law for `S2PointRegion`: `S2Cell::Contains` accepts each of a cell's four vertices, so the region of a vertex intersects the cell. 0 of 80,000 rejected, in both |
| UC, UR, UU | 10,000 each | 0, **3**, 0 | `S2CellUnion`'s `GetCapBound()`, `GetRectBound()` and `GetCellUnionBound()` over random unions (1 to 8 cells, scattered or clustered). The 3 `UR` rows are unions with a face cell 2 or 5 (K1) |
| UA | 10,000 | 0 | `AverageBasedArea()`, `ApproxArea()`, `ExactArea()` |
| US | 6 | **1** | special unions: empty, the whole sphere, one leaf, opposite faces. The empty union's `ApproxArea()` and `ExactArea()` are −0 in the library, +0 upstream (K2) |
| UG | 10,000 | 0 | the bounds contain every contained point: each cell's centre, vertices and 4 random points. **0 of 262,154 contained points outside the cap bound or the rectangle bound**, in both |
| UUC | 10,000 | 0 | every cell of the union is contained by its covering bound: 0 misses |
| CF | 6 | **2** | `S2Cell::GetRectBound()` of the six face cells: faces 2 and 5 differ by 2 ulps in the latitude bound (K1) |
| UV | 5,000 | 0 | the vertex law for `S2CellUnion`: of 39,636 edge neighbours the union does not intersect, the union contains 20,573 of their vertices, in both. The union's point test is by leaf cell (half-open), its cell test by id, so the law does not hold for it (by design, as for `Cell`) |
| UE | 3,000 | 0 | `Expand(min_radius, max_level_diff)` over small clustered unions, special radii (0, −1, 2, 4, 10⁻³⁰⁰, ∞, the face widths) and `max_level_diff` 40 |

Findings:

- **K1**: `Cell::rect_bound`'s polar faces use `POLE_MIN_LAT = 0.615479708670387`, Go's 15-digit
  literal, which rounds to `0x3fe3b2028082e8d1`. Upstream computes `asin(sqrt(1./3))`,
  `0x3fe3b2028082e8d3` (`0.6154797086703873`). The library's bound for face cells 2 and 5 is 2
  ulps wider (conservative, but not upstream's), and every `CellUnion::rect_bound` and covering
  that unions it inherits the difference.
- **K2**: `CellUnion::approx_area` and `exact_area` sum with `Iterator::sum`, whose neutral
  element for `f64` is −0, so the empty union's area is −0; upstream starts its sum at +0.

## Planning probe R0, R0b, R0c (research R0)

- `r0_harnesses.rs` (35 harnesses): run inside the verification crate through a temporary
  `#[cfg(all(kani, feature = "values"))] #[path = "../../specs/011-formal-verify-point-region-cell-union/probe/r0_harnesses.rs"] mod r0;`
  at the end of `verify/src/lib.rs` (not committed), with
  `VERIFY_PLAYBACK=0 VERIFY_RUN_DIR=target/run-r011 ./run-verify.sh --harness r0::r0_` (4 GiB,
  12 minutes, 4 jobs; `r0b_`, `r0c_` for the later rounds). Results: `r0_results.txt`.
- `rust/src/bin/leaf.rs`: does `CellId::from_point` return a valid leaf for every triple? In
  release builds, all 1,001,331 tried (the 1,331 special triples and 10⁶ random ones); in debug
  builds 443 of the specials panic, 112 of them without a NaN or a zero vector: every one has
  two or more infinite coordinates (K4). `rust/src/bin/inf.rs`: the backtrace of one.

## After the fixes (2026-10-28, after FR-017 and FR-018)

`compare.after.txt`, `rust/survey_rust_stderr.after.txt`: 0 differences in every section
(spec SC-007): `UR`, `US` and `CF` now agree. `rust/src/bin/twin.rs` is the covering law's
counterexample for `Cell` (research R6), `rust/src/bin/vertex.rs` the vertex law's for
`CellUnion`, and `rust/src/bin/panics.rs` the check of K4's conditions.
