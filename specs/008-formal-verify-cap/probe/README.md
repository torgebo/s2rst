# Survey probe (spec 008), 2026-10-24

The same cases on upstream C++ (`v0.14.0-40-g37a4501`, Release, x86-64, the existing build in
`/home/t/sources/s2geometry/build`) and on the library (release build). The outputs are
committed next to each program. `rust/src/main.rs` also transcribes upstream's cell tests
(`S2Cap::Contains(const S2Cell&)`, `MayIntersect`, `Intersects(cell, vertices)`) on top of the
library's `Point`, `Cell` and `ChordAngle`; on K2's grid the transcription agrees with C++ on
every case (38 of 65 true in both).

```sh
# C++ (installs the existing build; any prefix will do)
cmake --install /home/t/sources/s2geometry/build --prefix <s2-install>
cmake -S cpp -B <build> -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=<s2-install>
cmake --build <build> && LD_LIBRARY_PATH=<s2-install>/lib <build>/survey > cpp/survey_cpp.txt

# Rust
(cd rust && cargo run --release -q > survey_rust.txt)
```

| Case | C++ (run) | Library (run) | Finding |
|------|-----------|---------------|---------|
| Covering level of the cap with angle radius 0.11785113019775795 rad | 1 (`kMinWidth.GetLevelForMinValue(r) − 1`, through `ilogb`) | 2 (`floor(log2(d / r)) − 1`: `log2` rounds a quotient just below 2ᵏ up to k); 180 of 1,856 radii near the 29 powers of two differ | K1 |
| `Contains(face cell 0)` for caps centred 0–12° off the face centre, radii 56–64° | true in 38 of 65 | `Region::contains_cell` (`contains(cell.cap_bound())`): true in 30; the first difference at 4° off, radius 58° | K2 |
| `MayIntersect(face cell 0)` of a 2° cap at (0°, 50°), on face 1 | false | `Region::intersects_cell` (`intersects(cell.cap_bound())`): true. Both coverings (8 cells) have no face-0 cell | K3 |
| `Complement()`'s length² for length² 0.1, 0.5, 1, 1.7, 2.9, 3.3 | `4 − r²`: 3.9, 3.5, **3**, 2.3, 1.1, 0.7 | `STRAIGHT − r` squares a square root: **2.99999999999999956** for 1, 2.29999999999999982 for 1.7; 472,249 of 10⁶ random radii differ | K4 |
| `GetRectBound()` of a cap at longitude 10⁻¹⁴°, radius 10⁻¹⁸ rad | lng [1.7353·10⁻¹⁶, 1.7553·10⁻¹⁶], contains the centre | `(x + π).rem_euclid(2π) − π` rounds to lng [0, 0], which misses the centre (1.7453·10⁻¹⁶) | K5 |
| `ApproxEquals` of equal radii, centres 5.02·10⁻¹⁵ rad apart | true (default `max_error` 10⁻¹⁴ for both parts) | `approx_eq` false, `approx_eq_with(1e-14)` false: the centres are compared at a fixed 10⁻¹⁵ | K6 |

## After the fixes (T023, 2026-10-24)

`rust/survey_rust_after.txt`, the same program on the fixed library (FR-017 to FR-020,
FR-022): K1 0 of 1,856 radii differ; K2 38 of 65, as C++; K3 false, as C++; K4's complements
are C++'s (3 for length² 1; the "472,249 of 10⁶" line still measures `STRAIGHT − r` itself, the
route `complement` no longer takes); K5's rectangle is C++'s and contains the centre; K6 true,
as C++.

## Planning probe R0, R0b, R0c (research R0)

- `r0_harnesses.rs` (24), `r0b_harnesses.rs` (18), `r0c_harnesses.rs` (9): the Kani
  harnesses, run from a scratch crate that depends on `s2rst` and includes these files by
  `#[path]`, through a copy of `verify/run-verify.sh` with `-Z stubbing` (4 GiB, 12 minutes per
  harness, 3 jobs). Results in `r0_results.tsv` and `r0bc_results.tsv`.
- R0 asked the questions as first stated; R0b tried the remedies (function contracts with
  `cos`, lemma stubs, the symmetry lemmas); R0c completed the stubs and moved their tables to
  fixed size. Research R0 reads every row.
- The failures that are the checker's, not the code's, were replayed natively: R0's
  `r0_level_library_margin` (CBMC's `log2`, at r = 0.4866111182066655), both `rect_bound`
  harnesses of R0 (CBMC's `cos` gave 0 at 1.1·10⁻³⁰⁸ and at −0), and R0b's library
  `rect_bound` (Kani's `f64 %` in `rem_euclid`).
- Scratch crate layout, to rebuild it: `Cargo.toml` with `s2rst = { path = "<repo>/core" }`
  and `[workspace]`; `src/lib.rs` with three `#[cfg(kani)] #[path = "…"] mod r0;` lines; a copy
  of `verify/run-verify.sh` with empty `known-defects.txt` and `known-timeouts.txt`; run with
  `VERIFY_JOBS=3 ./run-verify.sh -Z stubbing --harness r0c::`.
