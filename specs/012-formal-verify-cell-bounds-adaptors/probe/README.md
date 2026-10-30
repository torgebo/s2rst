# Survey probe (spec 012), 2026-10-29

The same cases on upstream C++ (`v0.14.0-40-g37a4501`, Release, x86-64, the existing build in
`/home/t/sources/s2geometry/build`) and on the library (release build, at `67e969e`), as spec
011's probe: same LCG, one line per case, `compare.py` diffs them section by section
(`compare.txt`); the full outputs are not committed (they regenerate exactly).

```sh
cmake --install /home/t/sources/s2geometry/build --prefix <s2-install>
cmake -S cpp -B <build> -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=<s2-install>
cmake --build <build>
LD_LIBRARY_PATH=<s2-install>/lib <build>/survey > cpp/survey_cpp.txt 2> cpp/survey_cpp_stderr.txt
cargo run --release -q --manifest-path rust/Cargo.toml > rust/survey_rust.txt 2> rust/survey_rust_stderr.txt
python3 -I compare.py rust/survey_rust.txt cpp/survey_cpp.txt > compare.txt
```

| Section | Cases | Differ | What |
|---------|-------|--------|------|
| CB | 20,000 cells (levels 0–30) × 60 points | 0 | `S2Cell::GetCapBound()`, `GetRectBound()` bit for bit, and for each point whether `Contains(p)`, the cap and the rectangle hold it. The points: the 4 corners and 4 edge midpoints of the cell's u-v bound, moved outward by 0, 0.5, 1, 1.5, 5/3 (just under and over) and 2 times `DBL_EPSILON` in u-v, then normalised; and 4 random interior points |
| RU, RI | 5,000 each | 4,960, 1,961 | `S2RegionUnion`, `S2RegionIntersection` of 1 to 4 mixed members (cap, rectangle, cell, point region): bounds, covering size, `Contains(cell)`, `MayIntersect(cell)`, `Contains(p)`. Every difference is the cap bound's squared radius alone: the adaptors' cap bound is their rectangle's `GetCapBound()`, which the library widens (spec 010 FR-019, U-016, a documented deviation) |

**U-017, found here (C++ and library alike):** of 1,008,724 points `S2Cell::Contains(p)` accepts,
118,093 lie outside the cell's cap bound and 36,315 outside its rectangle bound. Every one is a
point beyond the cell's u-v bound, within `Contains(p)`'s `(5/3)·DBL_EPSILON` margin (added so
that `S2Cell(S2CellId(p)).Contains(p)` always holds): of the corners and edge midpoints moved
by 0 (160,000) and the random interior points (80,000), every one lies in both bounds. By
offset (code 3 = in both bounds, 2 = cap only, 1 = rectangle only, 0 = neither, x = not
contained):

| Offset (× ε) | 3 | 2 | 1 | 0 | x |
|--------------|---|---|---|---|---|
| 0 | 160,000 | | | | |
| 0.5 | 137,002 | 477 | 21,827 | 694 | |
| 1 | 134,232 | 2,613 | 20,821 | 2,334 | |
| 1.5 | 124,029 | 3,619 | 18,703 | 5,596 | 8,053 |
| 5/3 − 0.001 | 118,350 | 4,026 | 17,445 | 5,799 | 14,380 |
| 5/3 + 0.001 | 112,347 | 3,502 | 16,436 | 5,746 | 21,969 |
| 2 | 9,798 | 636 | 1,419 | 1,273 | 146,874 |
| random interior | 80,000 | | | | |

The `Region` contract says the bounds contain the region, and `Contains(p)` defines it, so the
bounding-cap and bounding-rectangle laws fail for `S2Cell` at its margin. The maintainer chose
(2026-10-30) to keep upstream's bounds and state the laws for the cell's own points, the margin
a documented exception.
