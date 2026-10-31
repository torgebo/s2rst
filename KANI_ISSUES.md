# Kani issues

Bugs met in the verification tools while formally verifying s2rst (specs 001–011 under
`specs/`): Kani and the CBMC it bundles. They are recorded here and not filed with the Kani
project, at the maintainer's choice (2026-10-25). Each entry gives the proofs' workaround. Bugs
in the upstream S2 libraries are in `UPSTREAM.md`.

Versions: Kani 0.68.0 (cargo plugin), CBMC 6.11.0 with CaDiCaL 3.0.0, Kani's toolchain
`nightly-2026-08-21`, x86-64 Linux. Kani is pinned in the `Makefile` and
`.github/workflows/verify.yml`; `verify/TRUSTED_BASE.md` lists what the proofs trust.

"Re-run 2026-10-25" means the reproducer below was run again on that date in a scratch crate and
gave the result shown. "From the record" means it was seen during the spec named and not run
again. The reproducers are harnesses in a library crate with
`[lints.rust] unexpected_cfgs = { level = "warn", check-cfg = ['cfg(kani)'] }` in its
`Cargo.toml`, run with a plain `cargo kani` unless stated.

| Id | What | Kind | Seen | s2rst's workaround | Follow-up |
|----|------|------|------|--------------------|-----------|
| [KI-001](#ki-001--and--on-symbolic-bools) | `<` and `<=` on symbolic `bool`s give wrong answers | wrong result: can prove a false assertion | 004; re-run 2026-10-25 | booleans compared with `==` only | — |
| [KI-002](#ki-002--on-f64) | `%` on `f64` is wrong: `1.0 % 2.0` is 0 | wrong result: can prove a false assertion | 007; re-run 2026-10-25 | nothing proven through float `%` | 007 T062 |
| [KI-003](#ki-003-compiler-panic-on-thread_local-with-a-destructor) | the compiler panics on a `thread_local!` whose value has a destructor | crash (compiler) | 008; re-run 2026-10-25 | `static Mutex` tables | 008 T057 |
| [KI-004](#ki-004---stop-on-fail-crashes-the-output-parser) | `--cbmc-args --stop-on-fail` panics Kani's output parser | crash (driver) | 001; re-run 2026-10-25 | not used | — |
| [KI-005](#ki-005-cbmcs-smt2-backend-aborts) | CBMC's SMT2 backend aborts with `map::at` | crash (CBMC) | 001, from the record | CaDiCaL for every harness | — |
| [KI-006](#ki-006-division-by-an-infinity-reported-as-nan) | the NaN check reports a division by an infinity | false alarm | 006; re-run 2026-10-25 | domains, then native replay | — |
| [KI-007](#ki-007-sqrt-is-not-a-function) | `sqrt` gives two results for one input | false alarm | 006; re-run 2026-10-25 | no harness compares two `sqrt` results | — |
| [KI-008](#ki-008-misaligned-cast-when-dropping-an-ioerror) | "misaligned pointer to reference cast" when some `io::Error`s are dropped | false alarm | 003, from the record | decoding harnesses forget the error | — |
| [KI-009](#ki-009-an-out-of-memory-kill-reads-as-a-failure) | a harness killed for memory reports `FAILED` with "0 of N failed" | misleading report | 001, from the record | `run-verify.sh` re-runs it alone and reports `OOM` | — |
| [KI-010](#ki-010-the-driver-keeps-memory-for-every-harness-it-has-run) | the driver's memory grows with every harness until it runs out | crash (driver) | 003, 008 | `run-verify.sh` runs batches of 32, and splits a batch whose driver crashes | — |
| [KI-011](#ki-011---verbose-panics-the-compiler) | `--verbose` panics `kani-compiler` | crash (compiler) | 008 | not used | — |
| [KI-012](#ki-012---no-overflow-checks-keeps-the-overflow-checks) | `--no-overflow-checks` keeps rustc's overflow checks and drops the NaN check | misleading option | gap 1, 2026-10-26 | the release pass claims "no overflow", and uses the option for the NaN check | — |
| [KI-013](#ki-013-powi-is-not-a-function) | `powi` gives two results for one input | false alarm | 011, 2026-10-28 | no harness compares two `powi` results; `pure_avg_area`; the 31 levels enumerated natively | — |

## KI-001: `<` and `<=` on symbolic bools

```rust
#[kani::proof]
fn bool_le() {
    let a: bool = kani::any();
    let b: bool = kani::any();
    assert_eq!(a <= b, !a || b);
}
```

- **Observed:** FAILED (re-run 2026-10-25, 0.02 s). The counterexamples pass when replayed
  concretely. `bool::partial_cmp`, `==`, `!=` and concrete constants are correct.
- **Expected:** SUCCESSFUL. `false < true`, so `a <= b` is `!a || b`.
- **Effect:** any ordering that involves a `bool` is affected, including a tuple key such as
  `(bool, f64)`. A wrong ordering can fail a true assertion or prove a false one. In spec 004
  the first `arc_spec`, which ordered points by a `(bool, f64)` key, failed 8 property harnesses
  spuriously.
- **Workaround:** harnesses and reference models compare booleans with `==` and `!=` only and
  branch explicitly (`verify/src/arc_spec.rs`, `ccw_le` and `ccw_lt`). Nothing in the 001–003
  harnesses orders booleans (checked in 004).
- **Records:** `verify/values/EVALUATION.md` (spec 004, "Notes from implementation");
  `specs/004-formal-verify-cell-interval/data-model.md`.

## KI-002: `%` on `f64`

```rust
#[kani::proof]
fn frem() {
    let two: f64 = kani::any();
    kani::assume(two == 2.0);
    assert!(1.0 % two == 1.0);
}
```

- **Observed:** FAILED (re-run 2026-10-25, 0.07 s). The covers of spec 007's `r0_frem_which`
  show that Kani computes `1.0 % 2.0` as 0 and `−0.0 % 2π` as `+0`.
- **Expected:** SUCCESSFUL. Rust's `%` on floats is C's `fmod`: 1.0, and −0 for the second.
- **Effect:** a property through float `%` can fail spuriously or be proven falsely. In spec 008
  the library's `rem_euclid` (built on `%`) gave a spurious `rect_bound` counterexample.
- **Workaround:** no harness or reference description uses float `%`. Evenness is tested as
  `(f / 2.0).floor() * 2.0 == f`. Library code through `%` gets a crash proof at most, and its
  values rest on the C++ reference vectors. Finite integer domains are enumerated natively
  instead (`specs/007-formal-verify-chord-angle-angle/probe/rust/src/bin/exhaust.rs`, every
  `i32`, 67.8 s). `round_ties_even`, `floor` and `round` are modelled correctly.
- **Records:** `specs/007-formal-verify-chord-angle-angle/research.md` R0, Finding 1; probe
  `specs/007-formal-verify-chord-angle-angle/probe/r0_harnesses.rs` (`r0_frem_concrete`,
  `r0_frem_which`); `verify/TRUSTED_BASE.md`.
- **Follow-up:** `specs/007-formal-verify-chord-angle-angle/tasks.md` T062.

## KI-003: compiler panic on `thread_local!` with a destructor

```rust
use std::cell::RefCell;

thread_local! {
    static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn double_memo(x: u32) -> u32 {
    SEEN.with(|s| {
        let r = x.wrapping_mul(2);
        s.borrow_mut().push(r);
        r
    })
}

#[cfg(kani)]
#[kani::proof]
fn thread_local_vec() {
    let x: u32 = kani::any();
    assert_eq!(double_memo(x), x.wrapping_mul(2));
}
```

- **Observed** (re-run 2026-10-25): Kani's compiler panics before any check runs:

  ```text
  thread 'rustc' panicked at kani-compiler/src/intrinsics.rs:243:17:
  assertion failed: matches!(output.kind(), TyKind :: RigidTy(RigidTy::Int(IntTy::I32)))
  ```

  The backtrace passes through `Intrinsic::from_instance` and `RustcIntrinsicsPass::transform`.
  Editions 2021 and 2024 both crash, with or without `-Z stubbing`, and with the function called
  directly or through `#[kani::stub]`. The same harness with `RefCell<Option<u32>>`, a value with
  no destructor, verifies. So does the same memo in a `static Mutex<Vec<u32>>`.
- **Expected:** the harness compiles and verifies.
- **Effect:** spec 008's first lemma stubs (research R0b) kept their memo tables in
  `thread_local!` + `RefCell<Vec<…>>`, and every harness that used them failed to compile.
- **Workaround:** the lemma stubs keep their tables in `static Mutex<Table<K>>` with fixed slots
  (`verify/src/stubs.rs`).
- **Records:** `specs/008-formal-verify-cap/research.md` R0b; `verify/TRUSTED_BASE.md`
  ("Tables").
- **Follow-up:** `specs/008-formal-verify-cap/tasks.md` T057.

## KI-004: `--stop-on-fail` crashes the output parser

```rust
#[kani::proof]
fn int_fail() {
    let x: u8 = kani::any();
    assert!(x < 200);
}
```

Run with `cargo kani --harness int_fail -Z unstable-options --cbmc-args --stop-on-fail`.

- **Observed** (re-run 2026-10-25): CBMC finds the failure, then Kani's driver panics instead of
  reporting it:

  ```text
  thread '<unnamed>' panicked at kani-driver/src/cbmc_output_parser.rs:477:21:
  called `Result::unwrap()` on an `Err` value: Error("data did not match any variant of
  untagged enum ParserItem", line: 0, column: 0)
  ```

- **Expected:** a FAILED verdict for the failing check.
- **Workaround:** not used. `run-verify.sh` limits time per harness instead.
- **Records:** `specs/001-formal-verify-base-type/research.md` R12; `verify/EVALUATION.md`
  ("Kani issues met").

## KI-005: CBMC's SMT2 backend aborts

- **Observed** (from the record): in spec 001's solver probe, the `from_token` crash harness at 4
  characters, run with `#[kani::solver(z3)]`, ended with CBMC aborting on an internal error
  (`map::at`, status 6).
- **Expected:** a verdict, or a time-out.
- **Workaround:** every harness uses Kani's default, CaDiCaL, which was never beaten in the probe
  (`grep -rn kani::solver verify/src` is empty).
- **Records:** `specs/001-formal-verify-base-type/research.md` R6; `verify/EVALUATION.md`
  ("Kani issues met").

## KI-006: division by an infinity reported as NaN

```rust
#[kani::proof]
fn div_by_inf() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite());
    let y = x / f64::INFINITY;
    assert!(y == 0.0);
}
```

- **Observed:** FAILED on "NaN on division" (re-run 2026-10-25, 0.03 s). Spec 006 met it in
  `normalize`'s `1 / ∞` for a squared length that overflows, and in `ensure_normalizable`'s
  `2 / −∞`. Both counterexamples replay to zeros.
- **Expected:** SUCCESSFUL. IEEE-754 gives ±0 for a finite number divided by an infinity.
- **Workaround:** `from_coords` and `normalize` range over `NormableTriple` (finite, with a finite
  squared length), and the rest is checked by tests. Every "NaN on division" is replayed under
  IEEE-754 before it is read as a defect.
- **Records:** `specs/006-formal-verify-r2-rect-point/research.md` R0, Finding 1.

## KI-007: `sqrt` is not a function

```rust
#[kani::proof]
fn sqrt_deterministic() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite() && x > 0.0);
    assert!(x.sqrt().to_bits() == x.sqrt().to_bits());
}
```

- **Observed:** FAILED (re-run 2026-10-25, 1.6 s). Kani lowers `f64::sqrt` to CBMC's C-library
  model, which constrains the root without fixing it, so two calls on one input can differ.
- **Expected:** SUCCESSFUL. IEEE-754 requires a correctly rounded square root, which is a
  single value.
- **Effect:** any harness that compares two computations that each take a square root fails
  spuriously, even when they are the same call. In spec 006 this cost P-PTORT-01,
  P-PTVAL-02's normalising clause and P-PTORT-03's `ortho` branch. In spec 008 a direct
  L-CP-06 failed on two different roots of one subnormal argument.
- **Workaround:** harnesses assert properties of one result, never the equality of two. Bit-for-bit
  comparisons move to tests against the C++ vectors.
- **Records:** `specs/006-formal-verify-r2-rect-point/research.md` R0, Finding 4;
  `verify/TRUSTED_BASE.md`.

## KI-008: misaligned cast when dropping an `io::Error`

- **Observed** (from the record): Kani reports "misaligned pointer to reference cast" in std's
  `io::error::repr_bitpacked::decode_repr` (line 273, the `SimpleMessage` arm) when a harness
  drops the error of a short `read_exact` returned through `Point::decode` or `Cap::decode`. In
  spec 003 this failed 56 decoding harnesses, `c_decode_point__b0 … __b23` and
  `c_decode_cap__b0 … __b31`, on that one check each. Probes, each on a 0-byte input:

  | Probe | Result |
  |-------|--------|
  | `Point::decode`, error dropped | FAILED (the same check) |
  | `Point::decode`, error forgotten (`mem::forget`) | SUCCESSFUL |
  | `read_exact` alone, error dropped | SUCCESSFUL |
  | `CellId::decode`, error dropped | SUCCESSFUL |

- **Expected:** SUCCESSFUL. The error points to one static object, and whether that is aligned
  cannot depend on which function returns it.
- **Workaround:** decoding harnesses end with `wire_domain::discard(result)`, which forgets an
  error instead of dropping it.
- **Records:** `verify/decoding/EVALUATION.md`; `verify/TRUSTED_BASE.md`; `verify/README.md`.

## KI-009: an out-of-memory kill reads as a failure

- **Observed** (from the record): when the memory limit kills CBMC, terse output reports
  `VERIFICATION:- FAILED` with "0 of N failed", as for a real failure without a failing check.
- **Expected:** a verdict that says the run did not finish.
- **Workaround:** `verify/run-verify.sh` re-runs such a harness alone and reports `OOM`.
- **Records:** `verify/EVALUATION.md` ("Kani issues met").

## KI-010: the driver keeps memory for every harness it has run

- **Observed** (2026-10-25, `make verify`, 829 harnesses, 7 jobs): `kani-driver`'s resident
  memory grows with each finished harness and is not given back until it exits: 98 MB after 11
  harnesses, 1.5 GB after 52, 2.3 GB after 127, about 18 MB each (sampled from
  `/proc/<pid>/status` every 5 s). The driver runs under the same `ulimit -v` as the harnesses
  (4 GiB), so the run died after 515 harnesses:

  ```text
  memory allocation of 49283072 bytes failed
  thread 'main' panicked at .../kani-verifier-0.68.0/src/lib.rs:159:38:
  No exit code?
  ```

  The 7 harnesses in flight read as errors, and the 307 not yet started were missing from the
  report. Spec 003 met the same crash after about 150 harnesses with 4 jobs.
- **Expected:** a driver whose memory does not depend on how many harnesses it has run.
- **Workaround:** `verify/run-verify.sh` takes the harness list from the build's metadata and
  runs it in batches (`VERIFY_BATCH`, 32 at the 4 GiB ceiling), one driver each. A listed
  harness that no driver reached is reported `NOT-RUN`. With batches of 64, the whole suite
  ran (829 of 829 successful, 59 minutes with 7 jobs), but the largest driver peaked at 3.6 GB
  of address space, hence 32. Each batch compiles the code its harnesses reach: the 13 batches
  took 101 s and 2.6 GB of `verify/target/kani` in all. On 2026-10-26 a decoding batch of 32
  (`cd_fx::p_fx_10__b9` … `cd_uv::p_uv_03__b8`, 7 jobs) took its driver to 4.0 GB of address
  space (3.6 GB resident) and crashed, twice: 2.3 GB came while its first seven harnesses were
  in flight, then about 50 MB per harness. So a batch whose driver crashes runs again as two
  batches, each with its own driver, down to single harnesses (`run_main` in `run-verify.sh`).
- **Records:** `verify/decoding/EVALUATION.md` (spec 003's crash).

## KI-011: `--verbose` panics the compiler

- **Observed** (2026-10-25, the `verify/` crate, not reduced): `cargo kani --only-codegen -Z
  stubbing --verbose` fails with "thread 'rustc' panicked at
  kani-compiler/src/kani_middle/analysis.rs:29:45" and "Kani unexpectedly panicked during
  compilation". The same build without `--verbose` succeeds.
- **Expected:** the same build, with more output.
- **Workaround:** `--verbose` is not used.

## KI-012: `--no-overflow-checks` keeps the overflow checks

- **Observed** (2026-10-26, a scratch crate): with `--no-overflow-checks`, and also with
  `CARGO_PROFILE_DEV_OVERFLOW_CHECKS=false` or `RUSTFLAGS=-Coverflow-checks=off`,

  ```rust
  #[kani::proof]
  fn wraps() {
      let x: u8 = kani::any();
      kani::assume(x == 255);
      let y = x + core::hint::black_box(1u8);
      assert!(y == 0);
  }
  ```

  still fails with "attempt to add with overflow": Kani compiles rustc's overflow checks into
  every build. What the option changes is CBMC's flags: `--nan-check` goes, and
  `--no-div-by-zero-check` comes (compared with `--verbose`'s command line on a harness without
  stubs, KI-011). `CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=false` does turn `debug_assert!` off.
- **Expected:** an option named for overflow checks that turns them off, or a way to verify the
  wrapping arithmetic of a release build.
- **Workaround:** the release pass (`make verify-release`) proves "no panic and no arithmetic
  overflow", stronger than a release build needs; where a release build wraps, the inputs are
  outside the proof (`crash::r_children`'s `child_end`, `cu_crash::r_cell_queries__n2`'s `none`).
  It passes `--no-overflow-checks` for its other effect, so that a NaN, which a release build's
  skipped check often leaves and Rust computes without a panic, is not reported.

## KI-013: `powi` is not a function

```rust
#[kani::proof]
fn powi_deterministic() {
    let n: i32 = kani::any();
    kani::assume((0..=30).contains(&n));
    assert!(4.0_f64.powi(n).to_bits() == 4.0_f64.powi(n).to_bits());
}
```

- **Observed:** FAILED in 0.1 s (2026-10-28, spec 011's planning probe R0b, inside the
  verification crate: `specs/011-formal-verify-point-region-cell-union/probe/r0_harnesses.rs`,
  `r0b_powi_deterministic`). Kani lowers `f64::powi` (an LLVM intrinsic) to a model that does
  not fix one result per input, as KI-007's `sqrt`.
- **Expected:** SUCCESSFUL: `powi` is a function of its arguments in every build Rust makes.
- **Effect:** `Cell::average_area_for_level` is `4π / (6 · 4^level)` through `powi`, so a harness
  that compares `CellUnion::average_based_area` with the same expression written again fails
  spuriously (R0's `r0_cu_avg_area__n1`, `__n3`).
- **Workaround:** the statement stubs `Cell::average_area_for_level` with a pure, uninterpreted
  function (`stubs::pure_avg_area`, P-UAREA-01), and the real function is pinned to upstream's
  `ldexp` by enumerating its 31 levels natively (`verify/tests/area_enumeration.rs`, P-UAREA-04).
- **Records:** `specs/011-formal-verify-point-region-cell-union/research.md` R0;
  `verify/TRUSTED_BASE.md`.

## Limits that are not bugs

Documented behaviour or cost, worked around in the same way:

- **Loose models of transcendental functions.** CBMC over-approximates `sin`, `cos` and `log2`.
  Re-run 2026-10-25: `cos(1.1·10⁻³⁰⁸) == 1.0` fails, as does `cos(x) > 0.5` for
  `|x| ≤ 10⁻³⁰⁰`, and `log2(1.9375) < 1.0` fails, though it is 0.954. A concrete `cos(−0)` alone
  gives 1; inside spec 008's `rect_bound` harness it gave 0. Spec 008 stubs `cos` with a range
  contract that tests check against the real libm (`verify/src/stubs.rs`), and FR-017 removed
  `log2` from the library.
- **`asin`, `atan2`, `tan`, `atan`, `acos` and `hypot` are not supported.** Each is a foreign C
  call that Kani replaces with a failing check: "call to foreign "C" function `asin` is not
  currently supported by Kani" (re-run 2026-10-25; `atan`, `acos`, `hypot` 2026-10-26). Kani
  tracks these in its issue 2423. Spec 008 stubs `asin` and `atan2` with range contracts, and the
  float-geometry proofs of 2026-10-26 `tan` and `atan`; `ln`, `exp`, `powi`, `round`, `floor` and
  `sin_cos` are supported.
- **The loose `sin` and `cos` lose relations a proof can need.** Each is a value in [−1, 1]
  on its own: `sin_cos` can give (0, 0), so `LatLng::to_point` can be the zero vector, which no
  face contains (`Cell::from_lat_lng` failed a debug check, 2026-10-26), and `sin` of a positive
  angle can be negative (`Rect::expanded_by_distance` failed through `asin` of a ratio above 1).
  Both are over-approximations, sound for a crash proof that passes, but they fail ones that
  should pass; contracts for `sin` and `sin_cos` (`verify/src/stubs.rs`) restore the facts.
- **An unsatisfied `kani::cover!` does not fail a run** (Kani exits 0). `run-verify.sh` enforces
  covers itself.
- **`#[kani::should_panic]` is existential.** It passes when one input panics and every failed
  check is a panic; `kani::assert(false)` after the call counts as a panic too (re-run
  2026-10-26). The `n_*` harnesses end in one `kani::cover!` after the call instead, which is
  unreachable exactly when no input returns, and `run-verify.sh` requires it unsatisfied.
- **A symbolic panic condition keeps the code after it alive.** After `assume(raw > 30)`, a
  failed `assert!(raw <= 30)` is not simplified to false, so CBMC explores what follows on paths
  no input takes; loops there timed out (`crash::n_raw_level`, retired by spec 009; 10 minutes). A small unwind bound
  fixes it, with unwinding checks still on. A constant condition is pruned.
- **Terse output gives no counterexample values.** `run-verify.sh` re-runs failures with concrete
  playback.
- **Concrete playback needs more memory than the harness.** On a neighbour harness Kani's driver
  ran out of memory at 4 GiB ("memory allocation of 8388352 bytes failed", then a panic) and
  printed no test. Playback runs alone under `VERIFY_PLAYBACK_MEM` (12 GiB here).
- **Unsupported constructs** in the compiled code: one `caller_location` and one foreign function
  (`verify/TRUSTED_BASE.md`).
- **NaN checks on `∞ − ∞` and `0 × ∞`** are correct: those operations do give NaN. They are part
  of the crash definition, and harness domains exclude them.
- **Cost.** These blow up CBMC's memory or time:
  - symbolic-length collections and strings, and `core::fmt`;
  - `memcmp` on `Vec`s, which needs an unwind of the byte length;
  - a `Vec` in a static;
  - a law that recomputes a float product, which needs a lemma stub;
  - the robust predicates' exact fallback (`num-bigint` multiplication), which every edge
    crossing and closest-point computation reaches on symbolic points: `Cell`'s distances to an
    edge and to a cell, `exact_area` and `Rect::boundary_intersects` ran out of memory at 8 GiB
    or timed out at 60 minutes (2026-10-27);
  - a `Cell` built from a symbolic id, whose u-v bound is the quadratic projection of a symbolic
    (i, j): stubbing `ij_level_to_bound_uv` with any rectangle in [−1, 1]² (`any_uv_bound`) cut
    `Cell::vertex`'s proof from 326 s to 25 s and `rect_bound`'s from 30 minutes to 280 s.

  See `verify/README.md` and research of specs 002, 003 and 008.
