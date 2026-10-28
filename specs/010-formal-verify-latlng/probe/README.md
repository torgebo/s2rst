# Survey probe (spec 010), 2026-10-27

The same cases on upstream C++ (`v0.14.0-40-g37a4501`, Release, x86-64, the existing build in
`/home/t/sources/s2geometry/build`) and on the library (release build, at `7495cc6`). Both
programs draw their inputs from the same 64-bit LCG and print one line per case
(`<section> <index> <values as hex bits or 0/1>`); `compare.py` diffs them section by section.
The comparison is committed in `compare.txt`, with each program's stderr summary
(`*_stderr.txt`); the full outputs (3.8 MB each) are not, since they regenerate exactly.

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
| N | 12,014 | 8,378 | `Normalized()` of latitude 0.3 and 14 special plus 6 × 2,000 random longitudes (scales 4 to 10³⁰⁰). Upstream takes `remainder(lng, 2π)`; the library `lng − round(lng / 2π) · 2π`. Identical at scale 4 except ±π; 372 of 2,000 differ at scale 100, all beyond. **293 library results lie outside [−π, π]** (10¹⁶ gives −4.0, −2.462748394469102·10¹⁶ gives 4.0), so `normalized` can return an invalid `LatLng`. ±π come back as ∓π, and −0 and −2π lose the sign of zero (K1) |
| NL | 6 | 0 | latitude clamping, NaN and ±∞ (both give `Invalid()`) |
| P, PS | 20,004 | 0 | `ToPoint()`, bit for bit |
| L, LS | 20,009 | 0 | `S2LatLng(p)`, bit for bit, signed zeros and NaN included |
| D | 20,000 | 0 | `GetDistance()` of valid pairs |
| DS | 6 | 1 | `GetDistance()` with out-of-range latitudes: one point written twice, (3.124793493547795, 0) and (0.0167991600419979, π), gives NaN upstream and π in the library (K3) |
| S | 5 | 1 | `ToStringInDegrees()` of (−0, −0): upstream `-0.000000,-0.000000`, the library `-0.000000,0.000000` (K1's sign of zero) |
| R | 20,000 | 0 | `S2LatLngRect` as a region: `Contains(cell)` and `MayIntersect(cell)`, random rectangles × cells of levels 0–8 |
| G | 20,000 | 0 | the bounding cap contains every contained point: 7 points on and inside each rectangle's boundary. **Both miss 114 of 129,595 contained points** (U-016), every one when `GetCapBound()` returns the pole cap and the point lies on the far latitude bound, by up to 18.5ε of the cap's length² (`rust/survey_rust_stderr.txt`) |
| V | 20,000 | 0 | no vertex of a cell that `MayIntersect` rejects is contained: 0 violations in both |

Also measured (stderr): `S2LatLng(p)` is valid for 200,000 of 200,000 random finite vectors
whose coordinates span 600 decades (library); upstream's exact `Intersects(const S2Cell&)`,
which the library does not have, differs from `MayIntersect` on 158 of 20,000 random pairs
(C++); and outside the probe, `GetDistance(a, b)` and `GetDistance(b, a)` differ bit for bit on
15,613 of 200,000 random valid pairs, since the formula multiplies the two cosines in argument
order (the same formula in both).
