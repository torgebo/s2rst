# Survey probe (spec 007), 2026-10-16

The same cases on upstream C++ (`v0.14.0-40`, Release, x86-64) and on the library (release
build). The outputs are committed next to each program.

```sh
# C++ (needs the s2 install of tools/refgen-cpp/README.md)
cmake -S cpp -B <build> -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install
cmake --build <build> && LD_LIBRARY_PATH=/tmp/s2-install/lib <build>/survey > cpp/survey_cpp.txt

# Rust
(cd rust && cargo run --release -q > survey_rust.txt)
```

| Case | C++ | Library | Finding |
|------|-----|---------|---------|
| `e6` where degrees × 10⁶ is exactly 0.5, 2.5, 4.5, 6.5 | 0, 2, 4, 6 (ties to even) | 1, 3, 5, 7 (away from zero) | K8 |
| `e7` where degrees × 10⁷ is exactly −0.5, −2.5 | 0, −2 | −1, −3 | K8 |
| `e7` of 10¹⁰ rad, `e5` of `Infinity`, `e5` of NaN | −2147483648 each | 2147483647, 2147483647, 0 | K9, U-014 |
| `ChordAngle::INFINITY.e5()` / `.e7()` | −2147483648 | 2147483647 | K5, U-014 |
| `normalized` of 10⁶ | −0.35756416704675331 | −0.3575641670031473 | K7 |
| `normalized` of 10¹⁶, 10²⁰, 10³⁰⁰ | 2.637…, 1.895…, −0.723… | π, 0, 0 | K7 |
| `normalized` of −2.462748394469102·10¹⁶ (found by R0) | not run | 4.0, outside (−π, π] | K7 |
| `ChordAngle::from_length2(NaN)` | 4 | NaN | K3 |
| `plus_error(0)` on a NaN length | 4 | NaN | K3 |
| `from_length2(−5)`: value, `is_valid`, `predecessor` | −5, valid, −1 | −5, valid, −1 | K1 |
| `from_length2(−∞)`: `is_negative`, `is_infinity` | true, false (read from `s1chord_angle.h:435–442`, not run) | true, true | K2 |

## Planning probe R0 (research R0)

- `r0_harnesses.rs`: the Kani harnesses, run from a scratch crate that depends on `s2rst`,
  through a copy of `verify/run-verify.sh` (4 GiB, 12 minutes per harness). Results in
  `r0_results.tsv`, counterexamples in `r0_counterexamples.txt`.
- `rust/src/bin/replay.rs`: the counterexamples replayed natively
  (`cargo run --release -q --bin replay`).
- `rust/src/bin/exhaust.rs`: the E-form round trips and upstream's `E6(n) == E7(10 n)` over
  every `i32`, and `Degrees(n) == E6(10⁶ n)`, `Degrees(n) == E7(10⁷ n)` wherever the product
  fits (`cargo run --release -q --bin exhaust`): no failure, 69.2 s on 2026-10-16 (67.8 s
  with the two added identities).
