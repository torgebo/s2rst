# Kani issues

Bugs met in the verification tools while formally verifying s2rst (specs 001–008 under
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

## Limits that are not bugs

Documented behaviour or cost, worked around in the same way:

- **Loose models of transcendental functions.** CBMC over-approximates `sin`, `cos` and `log2`.
  Re-run 2026-10-25: `cos(1.1·10⁻³⁰⁸) == 1.0` fails, as does `cos(x) > 0.5` for
  `|x| ≤ 10⁻³⁰⁰`, and `log2(1.9375) < 1.0` fails, though it is 0.954. A concrete `cos(−0)` alone
  gives 1; inside spec 008's `rect_bound` harness it gave 0. Spec 008 stubs `cos` with a range
  contract that tests check against the real libm (`verify/src/stubs.rs`), and FR-017 removed
  `log2` from the library.
- **`asin`, `atan2` and `tan` are not supported.** Each is a foreign C call that Kani replaces
  with a failing check: "call to foreign "C" function `asin` is not currently supported by
  Kani" (re-run 2026-10-25). Kani tracks these in its issue 2423. Spec 008 stubs `asin` and
  `atan2` with range contracts.
- **An unsatisfied `kani::cover!` does not fail a run** (Kani exits 0). `run-verify.sh` enforces
  covers itself.
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
  - a law that recomputes a float product, which needs a lemma stub.

  See `verify/README.md` and research of specs 002, 003 and 008.
