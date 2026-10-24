---

description: "Task list for the formal verification of ChordAngle and Angle (spec 007)"
---

# Tasks: Formal Verification of `ChordAngle` and `Angle`

**Input**: Design documents from `specs/007-formal-verify-chord-angle-angle/`:

- plan.md, spec.md (amended at planning with R0's results);
- research.md (R0–R14; R0 ran on 2026-10-16);
- data-model.md;
- contracts/ (verify-commands, harness-conventions);
- quickstart.md;
- the probes in `probe/`:
  - the survey (`cpp/survey.cc`, `rust/src/main.rs`, with their outputs);
  - the Kani probe `r0_harnesses.rs`, with `r0_results.tsv` and `r0_counterexamples.txt`;
  - the native replay (`rust/src/bin/replay.rs`) and the enumeration
    (`rust/src/bin/exhaust.rs`).

  R0's harnesses are the first drafts of several tasks below. Exceptions: those using `%`
  (`r0_ag_normalized_draft_*`, `r0_round_ties_even_model`), whose failures were the checker's
  (fact 2).

The feature extends the `values` suite of 004–006 in `verify/`. 006's task list
(`specs/006-formal-verify-r2-rect-point/tasks.md`) shows the patterns. Its files are the
style reference:

- `verify/src/point_spec.rs`, `pt_*.rs`;
- `verify/src/value_domain.rs` and `laws.rs` (`laws::eq`, `laws::ord`);
- `verify/tests/uv_projection.rs` (an exhaustive test, 004 R8);
- `tools/refgen-cpp/generate_values.cc`;
- `core/tests/cpp_values_diff.rs`;
- `verify/tests/values_spec_vs_cpp.rs`;
- the hand-written `Deserialize` of `r2::Rect` (`core/src/r2/rect.rs`).

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **For T005 only**: the C++ reference install at `/tmp/s2-install` (present on 2026-10-16),
  built from `/home/t/sources/s2geometry` at `v0.14.0-40-g37a4501`. If it is gone, rebuild it
  with `cmake --install` from `/home/t/sources/s2geometry/build`
  (`tools/refgen-cpp/README.md`). The generator runs with `LD_LIBRARY_PATH=/tmp/s2-install/lib`.
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run (61 GB on 2026-10-16;
  `verify/target` is 56 GB, see memory "disk space and the Kani cache").

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- the regression tests of FR-017, FR-018, FR-019's debug checks, FR-020, FR-021 and FR-022,
  each failing without its change (Constitution III, SC-007);
- the exact-remainder quickcheck property for FR-021;
- the differential tests against the new C++ vectors (FR-024);
- the descriptions' unit tests and their sanity tests against the same vectors (FR-007);
- the enumeration tests (FR-008), which are proofs and are listed under US3.

The Kani harnesses are the verification deliverable itself, so they are implementation
tasks.

**Organization**: Tasks are grouped by user story (spec.md):

| Story | Priority | Phase |
|-------|----------|-------|
| US1: proven crash freedom | P1 | 4 |
| US2: a chord angle that holds only canonical values | P1 | 5 |
| US3: proven meaning against independent descriptions | P2 | 6 |
| US4: CI regression protection | P3 | 7 |
| US5: evaluation and what next | P3 | 8 |

The library changes (FR-017 to FR-025) come first, in Phase 3, because the proofs' domains
and statements rest on the documented and fixed behaviour (plan, Complexity Tracking). US2's
behaviour lands there; its proof is Phase 5.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks). For
  Kani **runs**, "parallel" applies only to writing the code; the runs themselves go one
  after another (Memory safety, below).
- **[Story]**: the user story the task belongs to (US1–US5).
- Paths are relative to the repository root.

## Memory safety (read first)

- **Every** Kani run goes through a Makefile target or `verify/run-verify.sh`, which apply
  the memory guard:
  - `make verify-values [P=chord|angle]`;
  - `make verify-harness H=…`;
  - `make verify-mutants M=…`;
  - `make verify-decoding P=codecs` (T015's re-run).

  For a throwaway measurement, use R0's set-up: the scratch crate
  `$SCRATCHPAD/r07kani` (lost on reboot; recreate it from `probe/r0_harnesses.rs` as research
  R0 describes), with a copy of `run-verify.sh`, `VERIFY_HARNESS_TIMEOUT=12m
  VERIFY_MEM_PER_HARNESS=4G`.
- **No run is queued** (2026-10-16). Before starting one, check that no other is going with
  `pgrep -fa '[c]bmc'`. Each run starts with a codegen of the whole crate and ends with a
  concrete playback that compiles it again, so `verify/` must **compile at every save** while
  one is going.
- **Kani runs go one at a time**, never beside another Kani run. Writing code, the ordinary
  tests, the enumeration test and `cargo kani --only-codegen` (no CBMC) can go on meanwhile.

## Upstream findings rule (FR-025, read first)

If a harness, a C++ vector or reading upstream code shows **undefined or wrong behaviour in
the upstream reference**, do these **before** marking the affected proof complete:

1. Append a task to Phase 10 with the next free task ID and the next free `U-NNN`:
   - U-014 (K5, K9: C++'s E-forms of `S1ChordAngle::Infinity()` and of out-of-range angles;
     K8: its tie rounding depends on the processor) is this feature's, with task T061;
   - the next free number is **U-015**.

   Give the upstream `file:line`, the inputs, the observed behaviour, and what the library
   does.
2. Add the same entry under "Upstream findings" in the spec-007 part of
   `verify/values/EVALUATION.md`, and to `UPSTREAM.md`.
3. Cite the `U-NNN` in the affected harness's doc comment.

Never change upstream code.

## Known-defect rule (FR-012, read first)

A proof that fails on a **real library defect** gets a `D-NNN`:

- D-001 to D-007 are taken; the next free one is **D-008**.
- K1–K10 are the survey's findings, fixed or documented by FR-017 to FR-023, and need no
  `D-NNN`, as 004–006's survey findings did not. That includes K7's input
  −2.462748394469102·10¹⁶, which R0 found and the spec's K7 records.
- Record a new one under "Defects found" in the spec-007 part of
  `verify/values/EVALUATION.md`.
- If its harness is written before the fix lands, list it in `verify/known-defects.txt`.
- The fix is a **separate, tested change**.
- **Ask the user before fixing** a defect that is not in the spec's survey or this plan.

**A failure is replayed before it is believed** (research R0, R14): run the concrete
playback's input in an ordinary test.

## Facts these tasks rely on

1. **R0's results** (`probe/r0_results.tsv`; 4 GiB, 12 minutes):
   - **the chord angle**: the draft constructor canonical 0.0 s; the order laws 1.0 s;
     `successor` 1.0 s; `predecessor` 0.5 s; the neighbours' inverse 0.3 s; `Add`'s range
     4.5 s; `Sub`'s range 15.4 s; `plus_error` canonical 0.7 s; `from_angle`'s range 0.2 s;
     `tan` crash 1.6 s;
   - **the angle**: the E-forms' crash 0.0 s; `sin_cos` crash 0.0 s; `round_ties_even`
     against its `floor` definition 6.5 s;
   - **failed as expected**: the library's `from_length2(NaN)` (K3); the library's
     `normalized` at −2.462748394469102·10¹⁶, which returns 4.0 (K7);
   - **tool limits**: `asin` (`to_angle`, `e7` of a chord angle) and `tan` of an `Angle` fail
     on "call to foreign "C" function … is not currently supported by Kani";
   - **timeouts at 12 minutes**: the E7 round trip with either rounding, over all `i32` and
     over `0 ≤ n < 2²⁴`; upstream's `E6(n) == E7(10 n)`.
2. **Kani's `%` on `f64` is wrong** (research R0, Finding 1; memory "Kani float % is
   wrong"). `1.0 % 2.0` is 0 under Kani, and `−0.0 % 2π` is `+0`. **No harness or description
   may use `%` on floats.** Evenness is tested with `(f / 2.0).floor() * 2.0 == f`.
   `normalized` after FR-021 gets a crash proof only.
3. **Enumeration** (`probe/rust/src/bin/exhaust.rs`, 2026-10-16, release mode, 67.8 s). Every
   `i32` holds for:
   - `from_e5(n).e5()`, `from_e6(n).e6()` and `from_e7(n).e7()` with ties to even;
   - `from_e7(n).e7()` with the library's present rounding;
   - `E6(n) == E7(10 n)`;
   - `Degrees(n) == E6(10⁶ n)` and `Degrees(n) == E7(10⁷ n)` wherever the product fits.
4. **`ChordAngle`** (`core/src/s1/chord_angle.rs`, unchanged since the initial commit):
   - `pub struct ChordAngle(f64)`, private field; `#[derive(Clone, Copy, Debug, PartialEq,
     PartialOrd)]` and a derived serde pair (lines 51–53);
   - the constants (lines 57–72): `ZERO` 0, `RIGHT` 2, `STRAIGHT` `MAX_LENGTH2` (4),
     `INFINITY` +∞, `NEGATIVE` −1;
   - `from_length2` (79–85): `length2 > 4 → STRAIGHT`, else as is (so NaN and negatives
     pass);
   - `from_angle` (89–98): negative radians → `NEGATIVE`; infinite → `INFINITY`; else
     `(2 sin(0.5 · min(r, π)))²` with Rust's NaN-ignoring `min`;
   - `to_angle` (141–149): `asin` and `sqrt`; `radians`, `degrees`, `e5`, `e6`, `e7`
     (153–179) go through it;
   - `is_negative` `< 0`, `is_infinity` `is_infinite()` (197–199), `is_special`, `is_valid`
     (209–211) as upstream's;
   - `successor` (221–233), `predecessor` (237–247), through `f64_next_after` (line ~392);
   - `plus_error` (252–257) with `clamp(0, 4)`, which propagates NaN;
   - `sin` (`sin2().sqrt()`), `sin2`, `cos`, `tan` (277–301) unchecked;
   - `Add` (328–356) and `Sub` (358–382) with `debug_assert!(!x.is_special())`;
   - `Display` (385) through `to_angle`;
   - inline `mod tests` (line 415) and `mod quickcheck_tests` (line 974). New tests go to a
     sibling `chord_angle_tests.rs`.
5. **`Angle`** (`core/src/s1/angle.rs`, unchanged since the initial commit):
   - `pub struct Angle(f64)`, `#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]`
     and a derived serde pair (lines 60–62);
   - the module-level `#![expect(clippy::cast_possible_truncation, …)]` and
     `cast_possible_wrap` (lines 26–33), whose reasons say "values clamped to valid range"
     (K9: nothing clamps);
   - `e5`, `e6`, `e7` (132–147): `(self.degrees() * 1eK).round() as i32`;
   - `normalized` (157–160): `x − round(x / 2π) · 2π`, then `≤ −π → π`;
   - `tan` (176) is a C library call under Kani;
   - inline `mod tests` (line 264). New tests go to a sibling `angle_tests.rs`.
6. **The `Cap` decoder** (`core/src/s2/encoding.rs:219–250`, `impl S2Decode for Cap` at line
   230): it reads four `f64`s, builds `ChordAngle::from_length2(radius)` and rejects unless
   `cap.is_valid()`. A NaN radius is rejected today only because `from_length2(NaN)` is NaN
   (003 D-004). Its tests are in `core/src/s2/encoding_tests.rs`
   (`cap_with_nan_length2_is_rejected`, line 71).
7. **003's model of it** (`verify/src/wire_spec.rs:534–600`): `ref_cap` (upstream,
   `std::min(4.0, l)`), `lib_cap` (the library: clamp above 4, reject unless `cap_valid`),
   used by `cd_fx::p_fx_08__b0 … __b33` (`verify/src/cd_fx.rs:235…`). P-FX-08's row is in
   `verify/decoding/PROPERTIES.md` (line 55).
8. **Python** (`python/src/angle.rs`): `PyChordAngle::from_length2` (line 238),
   `successor`/`predecessor` (304–308), `__add__` (328); `PyAngle`'s `e5`–`e7` (79–89) and
   `normalized` (99). Tests in `python/tests/test_angle.py` use non-negative lengths and
   `NEGATIVE` only.
9. **Records and numbering.**
   - `verify/values/INVENTORY.md` has §1–6, and `PROPERTIES.md` has §1–7. This feature adds
     INVENTORY §7–8 and PROPERTIES §8–9.
   - Mutants are numbered from `M-601`. The CI mutant matrix's `series` is
     `['0', …, '5']` (`.github/workflows/verify.yml:331`).
   - `make verify-values` maps parts through `values_part` (`Makefile:233`), which passes
     each `verify/src/<prefix>_*.rs` module as `--harness <module>::`. Its recipe runs
     `cd verify && cargo test`, then the harnesses (`Makefile:237–239`).
10. **The C++ vectors.**
    - `tools/refgen-cpp/generate_values.cc`'s `main()` (line 1038) calls `PointSpecial()`
      last (line 1055). New sections called after it continue the one `std::mt19937_64`
      sequence and leave every earlier line byte-identical.
    - `core/tests/data/cpp_values_vectors.csv` is 8.9 MB (22,292 lines) and excluded from the
      published crate (`core/Cargo.toml:14`). Keep the new sections to about 1,000 rows each,
      so the file stays under 10 MB.
11. **The pinned upstream** (`/home/t/sources/s2geometry`):
    - `s1chord_angle.h:361–446` (`is_valid`, `is_negative` with its TODO, `is_infinity`);
    - `s1chord_angle.cc:92–117` (`+`, `−` debug checks), `:132`, `:147` (`sin2`, `cos`);
    - `s1angle.h:44–72` (the identities), `:127`, `:248–264` (the E-forms and their checks);
    - `s1angle.cc` (`Normalize`); `util/math/mathutil.h:71–79`, `:172–192` (`Round`,
      `FastIntRound`).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the new modules, the Makefile parts, and the record sections (research R2;
contracts).

- [X] T001 Add the new modules to `verify/src/lib.rs` and update `verify/Cargo.toml`'s comment (research R2; contracts/harness-conventions.md "Modules and names").
  - **New files**, each starting with the two SPDX lines and
    `// Written for this crate (not ported from upstream S2).`, with a one-line `//!` naming
    its data-model section or property area:
    - `verify/src/chord_spec.rs` and `verify/src/chord_spec_tests.rs` (data-model §1.1);
    - `verify/src/angle_spec.rs` and `verify/src/angle_spec_tests.rs` (§1.2);
    - `verify/src/ca_{crash,val,cls,ord,nbr,err,ari}.rs`;
    - `verify/src/ag_{crash,val,cnv,eform,ari}.rs` (no `ag_nrm`: P-AGNRM-01 has no harness, and a module without one would make `P=angle` pass a filter that matches nothing);
    - `verify/tests/angle_enumeration.rs` (an empty test module for now).
  - **`verify/src/lib.rs`:**
    - `pub mod chord_spec;` and `pub mod angle_spec;`, unconditional, next to
      `pub mod point_spec;`;
    - the `#[cfg(test)] #[path = "…_tests.rs"]` links in each spec module;
    - the `ca_*` and `ag_*` modules behind `#[cfg(all(kani, feature = "values"))]`;
    - extend the crate doc's values paragraph: spec 007, `chord_spec`, `angle_spec`, the two
      prefixes, and the enumeration test.
  - **`verify/Cargo.toml`:** the `values` comment names spec 007, `ca_*` (`ChordAngle`) and
    `ag_*` (`Angle`). No dependency changes.
  - **Checks:** `cd verify && cargo build && cargo test`; `cargo build --no-default-features`;
    `cargo kani --only-codegen`.
- [X] T002 [P] Extend `make verify-values` with `P=chord` and `P=angle`, and run the enumeration, in `Makefile` (contracts/verify-commands.md; research R12; fact 9).
  - `VALUES_HARNESSES` (line 234) maps `chord` → `$(call values_part,ca)` and `angle` →
    `$(call values_part,ag)`. Without `P`, all eight parts.
  - The recipe (lines 237–239): after `cd verify && cargo test`, run
    `cd verify && cargo test --release --test angle_enumeration -- --ignored` when `P` is
    empty or `angle`.
  - The help text: `[P=<cell|interval|real|rect|uvrect|point|chord|angle|module::prefix ...>]`,
    specs 004–007.
  - **Check:** `make -n verify-values P=chord`; `make -n verify-values P=angle` shows the
    enumeration; `make help | grep verify-values`.
- [X] T003 [P] Add the record sections in `verify/values/` (data-model §4, §5, §8; fact 9).
  - `INVENTORY.md`: section 7 "`s1::ChordAngle` (36 entries)" and section 8 "`s1::Angle`
    (31 entries)", with 004's columns and no rows yet.
  - `PROPERTIES.md`: section 8 "`s1::ChordAngle`" and section 9 "`s1::Angle`", with 004's
    columns and no rows yet. Add to the opening paragraph:
    - spec 007;
    - the new status **proven by enumeration** (data-model §8): the property is checked for
      every input of a finite integer domain, natively, in release mode, by a named test.
  - `EVALUATION.md`: a part "Spec 007: `ChordAngle` and `Angle`", with the headings of 006's
    part, "Composition" and "Reach into the point's geometry" replaced by "The canonical-value
    invariant (option B)" and "Reach into the trigonometric conversions", plus "`LatLng` and
    `Cap`".
  - **Check:** `cd verify && cargo test --test property_kinds` passes.

**Checkpoint**: `make verify-values P=chord` and `P=angle` run, with zero harnesses, and
pass. `cargo test` in `verify/` passes.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the C++ reference vectors, the two reference descriptions with their tests, and
the generators.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T004 Extend `tools/refgen-cpp/generate_values.cc` with two sections (research R9; fact 10).
  - **Header comment:** name spec 007 and document the new columns as the existing ones are
    documented.
  - **`# CHORD_ANGLE`:** about 1,000 rows.
    - **Columns:** `l,m,e,results,note`. `l` and `m` are lengths fed to `FromLength2`; `e` is
      an error for `PlusError`.
    - **Inputs:**
      - random `l`, `m` in [0, 4] (uniform, and squared chord lengths of random unit points);
      - the special cases for `l`: −1, −5, −0.001, −∞, −0, +0, the smallest subnormal, 2, 4,
        4 + 2⁻⁵⁰, 5, +∞, NaN;
      - `e` from {0, ±ε, ±1e-15, ±1, NaN} and random small values.
    - **`results`**, `;`-separated, for `a = FromLength2(l)`, `b = FromLength2(m)`:
      - 0 `a.length2()`;
      - 1–5 `is_zero`, `is_negative`, `is_infinity`, `is_special`, `is_valid`;
      - 6 `Successor().length2()`, 7 `Predecessor().length2()`;
      - 8 `PlusError(e).length2()`;
      - 9 `GetS2PointConstructorMaxError()`, 10 `GetS1AngleConstructorMaxError()`;
      - 11–12 `sin2(a)`, `cos(a)`, 13 `sin(a)`, 14 `tan(a)` (`-` for special `a`);
      - 15 `ToAngle().radians()`, 16–18 `e5`, `e6`, `e7`;
      - 19 `(a + b).length2()`, 20 `(a − b).length2()` (`-` if either is special);
      - 21 `a < b`, 22 `a == b`;
      - 23 `S1ChordAngle(S1Angle::Radians(l)).length2()` (`from_angle`, with `l` as radians).
    - **`note`:** `deviation:negative-canonical` where `l` is negative and not −1 (FR-017),
      `deviation:eform-saturate` where results 16–18 are out of the `int32` range (`l = +∞`).
    - **Never** call `+`, `−`, `sin2`, `cos` on a special value (their `ABSL_DCHECK`s).
  - **`# ANGLE`:** about 1,000 rows.
    - **Columns:** `r,s,n,results,note`. `r` and `s` are radians, `n` an integer.
    - **Inputs:**
      - random radians at magnitudes 10⁻³⁰⁰ to 10³⁰⁰, both signs;
      - multiples of π (±π, ±3π, 5π, 2π), 7, 10⁶, 10¹⁶, 10²⁰, 10³⁰⁰, −10²⁰ and R0's
        −2.462748394469102·10¹⁶;
      - the probe's exact ties (`probe/cpp/survey_cpp.txt`: the radians whose degrees × 10⁶
        or × 10⁷ is a half integer);
      - ±0, ±∞, NaN;
      - `n` from the integer extremes (`INT32_MIN`, `INT32_MAX`, 0, ±1), `uint32` values above
        `INT32_MAX` (as `int32_t` bit patterns), and random integers.
    - **`results`**, `;`-separated:
      - 0 `Degrees(r).radians()`, 1 `Radians(r).degrees()`;
      - 2–4 `e5()`, `e6()`, `e7()` of `Radians(r)` (`-` where C++'s value is out of range, so
        the library's saturation is checked against the documented value);
      - 5 `Normalized().radians()`, 6 `IsNormalized()`;
      - 7 `abs`, 8 `sin`, 9 `cos`, 10 `tan`;
      - 11 `r + s`, 12 `r − s`, 13 `r * s`, 14 `r / s`, 15 `Radians(r) / Radians(s)`;
      - 16–18 `E5(n)`, `E6(n)`, `E7(n)` radians; 19–20 `UnsignedE6`, `UnsignedE7` of `n`'s
        bits.
    - **`note`:** `deviation:eform-saturate` on rows where C++'s E-form is out of range.
  - **`main()`:** call the two new functions **after** `PointSpecial()` (fact 10).
- [X] T005 Build and run the generator, append the sections to `core/tests/data/cpp_values_vectors.csv`, and document them (depends on T004).
  - **Build:**
    `cmake -B tools/refgen-cpp/build -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/tmp/s2-install`;
    `cmake --build tools/refgen-cpp/build --target generate_values`.
  - **Run:**
    `LD_LIBRARY_PATH=/tmp/s2-install/lib tools/refgen-cpp/build/generate_values > core/tests/data/cpp_values_vectors.csv`.
  - **Checks:**
    - run it twice and `cmp` the outputs;
    - every earlier line is unchanged
      (`git diff core/tests/data/cpp_values_vectors.csv | grep '^-[^-]'` is empty);
    - the CSV stays under 10 MB (fact 10).
  - **The probe's values:** the rows reproduce `probe/cpp/survey_cpp.txt`. For example:
    - `e6` at the tie 2.5 is 2;
    - `Infinity().e5()` is −2147483648;
    - `FromLength2(NaN)` is 4;
    - `Normalized(1e20)` is 1.8955899103166374.

    If C++ disagrees, stop and revisit the finding with the user.
  - **`tools/refgen-cpp/README.md`:** add the two sections to the "Value types" table, with
    their columns and the Rust methods they check.
- [X] T006 [P] Write the chord angle's description in `verify/src/chord_spec.rs` (data-model §1.1; research R3).
  - **The meaning:** `canonical`, `special`, `less`, `next`, `prev` and the constants, exactly
    as data-model §1.1 defines them:
    - `canonical(l)`: "`l == −1`, or `0 ≤ l ≤ 4` (either zero), or `l == +∞`";
    - `next(l)`: "`−1 → +0`; a length below 4 → the next double up (from the bit pattern;
      `−0` and `+0` → the smallest subnormal); `4 → +∞`; `+∞ → +∞`";
    - `prev(l)`: "`+∞ → 4`; a length above the smallest subnormal → the next double down; the
      smallest subnormal → `+0`; `±0 → −1`; `−1 → −1`".
  - **Neighbours from bit patterns**, never `f64::next_up` (research R3) and never the
    library's `f64_next_after`.
  - **The restatements of upstream's arithmetic**, in a separate section with its own doc
    (FR-007): `upstream_from_length2` (with the option-B deviation stated), `upstream_plus_error`
    (C++'s `std::min`/`std::max` on NaN: `std::min(4.0, NaN)` is 4.0, `std::max(0.0, x)` is x),
    `upstream_sin2`, `upstream_cos`, `upstream_max_point_error`, `upstream_max_angle_error`.
  - Read only `length2()`; never call the operations described.
  - **No `%` on floats** (fact 2). Booleans with `&&`, `||` and `==` only.
- [X] T007 [P] Write the hand-made unit tests in `verify/src/chord_spec_tests.rs`.
  - Cover:
    - `canonical` at −1, −0.5, −0, 0, 4, 4 + ulp, +∞, −∞ and NaN;
    - `next` and `prev` at every boundary of data-model §1.1;
    - `next(prev(l)) == l` over a few hundred lengths;
    - `upstream_plus_error` with a NaN sum (4);
    - `upstream_from_length2` of −5 (−1), +∞ (4) and NaN (4).
- [X] T008 [P] Write the angle's description in `verify/src/angle_spec.rs` (data-model §1.2; research R3, R7).
  - `upstream_from_degrees`, `upstream_degrees`, `upstream_from_e(k, n)` (the product order
    of `s1angle.h:363–378`), and `upstream_e(k, r)`: `10ᵏ · upstream_degrees(r)` rounded to
    nearest, ties to even, built **from `floor`**, never `round_ties_even` (the library's
    choice) or `%` (fact 2):
    - `f = v.floor()`, `d = v − f`;
    - `f` is even when `(f / 2.0).floor() * 2.0 == f`;
    - the result is `f` when `d < 0.5`, or `d == 0.5` and `f` is even; otherwise `f + 1`.
  - Saturation outside the `i32` range (`i32::MAX`/`i32::MIN` by sign), and 0 for NaN, as
    FR-023 documents.
  - The doc says `remainder_2pi` lives in `core/src/s1/angle_tests.rs`, and why
    (data-model §1.2).
- [X] T009 [P] Write the hand-made unit tests in `verify/src/angle_spec_tests.rs`.
  - Cover:
    - `upstream_e` at the probe's ties (0.5 → 0, 1.5 → 2, 2.5 → 2, −2.5 → −2);
    - just below and above a tie;
    - ±∞, NaN, ±2³¹;
    - `upstream_from_e(6, n) == upstream_from_e(7, 10 n)` for a few `n`;
    - `upstream_from_degrees(180) == π`.
- [X] T010 Add the description checks for the new sections to `verify/tests/values_spec_vs_cpp.rs` (depends on T005, T006, T008; research R9).
  - **`chord_description_matches_cpp`**: for every `CHORD_ANGLE` row:
    - `upstream_from_length2(l)` equals result 0, except `deviation:negative-canonical` rows,
      where C++ keeps `l` and the description gives −1;
    - the classifications 1–5 equal the description's on canonical values;
    - `next`/`prev` equal results 6–7 where `l` is canonical;
    - `upstream_plus_error`, the error bounds, `upstream_sin2` and `upstream_cos` equal
      8–12, bit for bit.
  - **`angle_description_matches_cpp`**: for every `ANGLE` row:
    - `upstream_from_degrees`, `upstream_degrees` (0–1);
    - `upstream_e` (2–4) where C++'s value is in range;
    - `upstream_from_e` (16–18), bit for bit.
  - Report the row counts per section, as the existing checks do.
  - **Check:** `cd verify && cargo test --test values_spec_vs_cpp`.
- [X] T011 Add the generators to `verify/src/value_domain.rs` (data-model §2; research R4; depends on T006).
  - `any_chord() -> ChordAngle`: a `kani::any()` selector between `ChordAngle::INFINITY` and
    `ChordAngle::from_length2(x)` with `x == −1 || (0 ≤ x ≤ 4)` assumed through
    `chord_spec::canonical`. This is R0's `any_canonical`; it builds the same values before
    and after FR-017.
  - `any_chord_length()`: `from_length2(x)` with `0 ≤ x ≤ 4`.
  - `any_angle() -> Angle`: `Angle::from_radians(kani::any())`. 006's `any_finite_angle`
    stays.
  - Each doc names its domain class (data-model §2).
  - **Check:** `cd verify && cargo kani --only-codegen`.

**Checkpoint**: The descriptions pass their unit tests and the C++ rows. The generators
compile under Kani.

---

## Phase 3: Library Changes and Tests (FR-017 to FR-025), no story

**Purpose**: the canonical chord angle (US2's behaviour), the documented domains and the
fixed behaviour the proofs assume (research R5–R8). Each change is its own commit.

**Confirm every documented behaviour with a concrete probe** in debug and release builds
before committing, and record the probe's output in the commit message.

- [X] T012 FR-017: write the canonical-value regression tests in a new `core/src/s1/chord_angle_tests.rs` (K1, K2; research R5; data-model §6; fact 4).
  - Link it with `#[cfg(test)] #[path = "chord_angle_tests.rs"] mod chord_angle_tests;` at the
    end of `chord_angle.rs`.
  - **The cases:**
    - `from_length2(x)` is `NEGATIVE`, bit for bit, for x in −5, −0.001, −1, −∞ and the
      negative subnormal;
    - `predecessor(from_length2(−5))` is `NEGATIVE`;
    - `from_length2(−∞).is_infinity()` is false;
    - `from_length2(−0.0)` is a zero (not `NEGATIVE`), and its `predecessor` is `NEGATIVE`;
    - every length in [0, 4] and above 4 is unchanged from today.
  - **Check** that the negative cases **fail** before T013: `cargo test -p s2rst --lib chord_angle_tests`.
- [X] T013 FR-017: map negatives to `NEGATIVE` and test +∞ only in `is_infinity`, in `core/src/s1/chord_angle.rs` (research R5; depends on T012).
  - `from_length2(x)`: `x < 0.0` → `NEGATIVE`; `x ≤ MAX_LENGTH2` → `ChordAngle(x)`; else
    `STRAIGHT` (above 4, +∞ and NaN: this also lands FR-018's NaN half; T016 tests it).
  - `is_infinity`: `self.0 == f64::INFINITY`.
  - **The type's rustdoc:**
    - the canonical set;
    - the deviation from upstream: every negative becomes `NEGATIVE`, upstream keeps it;
    - the reason (one value per meaning, so the order and neighbour laws hold for every
      value);
    - the consequence for a `Cap` decoded from C++'s bytes.
  - `from_length2`'s rustdoc lists the four cases.
  - **Check:** T012 passes; `cargo test -p s2rst` passes. List any existing test that pinned a
    negative value other than −1, and its update, in the commit message.
- [X] T014 FR-017: write the `Cap` decoder's tests in `core/src/s2/encoding_tests.rs` (fact 6; data-model §6; depends on T013).
  - C++'s bytes for a cap with a unit centre and radius −5 decode to radius `NEGATIVE`, and the
    cap is empty.
  - `cap_with_nan_length2_is_rejected` (line 71) stays and still passes. Add the same for a
    NaN with a non-default payload.
  - **Check** that the NaN test **fails** after T013 and before T015: since T013,
    `from_length2(NaN)` is `STRAIGHT`, a valid full cap. That is the reason T015 exists.
- [X] T015 FR-017/FR-018: keep the `Cap` decoder rejecting NaN and document the deviation in `core/src/s2/encoding.rs`, and update 003's model and records (research R5; facts 6, 7; depends on T014).
  - **The decoder:** reject a NaN radius **before** `ChordAngle::from_length2` (003 D-004).
    Its rustdoc gains option B: a negative radius other than −1 decodes as `NEGATIVE`, so
    those bytes do not round-trip, where C++ keeps them.
  - **`verify/src/wire_spec.rs`:** `lib_cap` maps a negative `length2` to −1 (bits of −1.0),
    with the reason in its rustdoc. `ref_cap` is unchanged.
  - **Records, same commit** (constitution v1.2.0):
    - `verify/decoding/PROPERTIES.md` P-FX-08's statement names the option-B deviation;
    - P-FX-03 is unchanged (valid caps now have canonical radii, research R5).
  - **Re-run** `make verify-decoding P=codecs`, after the open runs, one at a time. Record
    the `p_fx_08__*` results in the commit message.
  - **Check:** T014 passes; `cargo test -p s2rst`; `cd verify && cargo test`.
  - *After implementation (2026-10-19, T060's analysis, I1):* this task missed
    `verify/src/codec.rs` `CapC::reference_decode`, which inherited D-004's rejection from
    `from_length2(NaN)` being NaN. The per-change re-run cannot reach it; the weekly
    `p_fx_08__b32` and `__b33` failed with a NaN radius. The reference decoder now states D-004
    itself, and both members are re-proven (711.9 s, 743.0 s).
- [X] T016 FR-018: write the NaN regression tests in `core/src/s1/chord_angle_tests.rs` (K3; data-model §6; depends on T013).
  - `from_length2(NaN)` is `STRAIGHT`.
  - `plus_error` with a NaN sum (a NaN length can no longer be built, so through the error):
    `ZERO.plus_error(f64::NAN)` and `RIGHT.plus_error(f64::INFINITY)` give `STRAIGHT`;
    `RIGHT.plus_error(f64::NEG_INFINITY)` gives `ZERO`.
  - **Check** that the NaN case **fails** before T017 (`clamp` already gives 4 and 0 for the
    infinities, so those cases pin behaviour that stays).
- [X] T017 FR-018: make `plus_error` upstream's clamp in `core/src/s1/chord_angle.rs` (research R5; depends on T016).
  - `ChordAngle(0.0_f64.max(MAX_LENGTH2.min(self.0 + error)))` for non-special values: Rust's
    `min`/`max` ignore NaN as C++'s `std::min(4.0, x)` does here. Replace `clamp`, which
    propagates NaN.
  - The rustdoc says a NaN sum gives `STRAIGHT`, as upstream.
  - **Check:** T016 passes.
- [X] T018 FR-019: document the special operands and add upstream's debug checks in `core/src/s1/chord_angle.rs`, with tests in `chord_angle_tests.rs` (K4; research R8; fact 4).
  - **`# Panics` on `Add`, `Sub`, `sin`, `sin2`, `cos`, `tan`:** in debug builds, when an
    operand is special. The rustdoc also says what release builds return: probe each in
    release, for example `NEGATIVE.sin()` is NaN and `INFINITY.cos()` is −∞.
  - **`sin2` and `cos`:** add `debug_assert!(!self.is_special())`, as upstream
    (`s1chord_angle.cc:132`, `:147`); `sin` and `tan` inherit them.
  - **Tests:** `#[cfg(debug_assertions)] #[should_panic]` for `NEGATIVE.cos()` and
    `INFINITY.sin2()`. Check that they **fail** before the checks are added.
  - Check whether `python/src/angle.rs`'s `PyChordAngle` exposes `sin`/`cos` of a special
    value, and that its tests still pass (`make -C python test`).
- [X] T019 FR-020: write the deserialisation regression tests in `core/src/s1/chord_angle_tests.rs` (K6; data-model §6; depends on T013).
  - **Rejected, each error naming its rule:** 5.0, −5.0, −0.5 and NaN. NaN goes through a
    serde format that carries it, or a hand-built value, as 006's T015.
  - **Accepted unchanged, bit for bit:** −1, ±0, the smallest subnormal, 2, 4, +∞.
  - **Run** `cargo test -p s2rst --features serde chord_angle_tests`, and check that the
    rejections **fail** before T020.
- [X] T020 FR-020: implement the hand-written `Deserialize` for `ChordAngle` in `core/src/s1/chord_angle.rs` (depends on T019).
  - Replace the derived `Deserialize` (fact 4) with a hand-written one that reads the `f64`,
    keeps the bare-number format and rejects unless canonical, naming the rule. Keep the
    derived `Serialize`.
  - `Cap`'s derived `Deserialize` inherits it. Check that `Cap`'s serde tests still pass.
  - **Check:** T019 passes; `make test` passes.
- [X] T021 FR-021: write the reduction's regression tests and the exact-remainder property in a new `core/src/s1/angle_tests.rs` (K7; research R3, R6; data-model §1.2, §6; fact 5).
  - Link it with `#[cfg(test)] #[path = "angle_tests.rs"] mod angle_tests;` in `angle.rs`.
  - **`remainder_2pi(x)`**, a test helper (data-model §1.2):
    - decompose `x` and `y = 2.0 * PI` into integer mantissas and exponents (`num_bigint::BigInt`);
    - compute `n`, the integer nearest `x / y` with ties to even, exactly;
    - compute `r = x − n·y` exactly and convert it to `f64` (exact: the remainder is
      representable);
    - map `−π` to `π`; return NaN for ±∞ and NaN.
  - **The regression cases**, bit for bit against C++ (`probe/cpp/survey_cpp.txt`):
    - 10⁶ gives −0.35756416704675331;
    - 10¹⁶ gives 2.637242432414304;
    - 10²⁰ gives 1.8955899103166374;
    - 10³⁰⁰ gives −0.72342670052702118;
    - −10²⁰ gives −1.8955899103166374;
    - ±3π, 5π give π;
    - R0's −2.462748394469102·10¹⁶ lies in (−π, π] and equals `remainder_2pi`.
  - **The property** (`quickcheck`): for `x = m · 10^k`, `m` in [−1, 1], `k` in −300..300,
    `normalized(x)` equals `remainder_2pi(x)`, bit for bit, and lies in (−π, π]; and
    `normalized` is idempotent.
  - **Check** that the regression cases and the property **fail** before T022.
- [X] T022 FR-021: make `normalized` the exact remainder in `core/src/s1/angle.rs` (research R6; depends on T021).
  - `let y = 2.0 * PI; let r = self.0 % y;` then `r − y` if `r > PI`, `r + y` if `r <= -PI`,
    else `r` (research R6's argument for exactness in the rustdoc's implementation note).
  - The rustdoc: the result is upstream's `remainder` then `−π → π`, in (−π, π] for every
    finite angle, NaN for ±∞ and NaN.
  - **Check:** T021 passes; `make test` passes.
- [X] T023 FR-022: write the tie regression tests in `core/src/s1/angle_tests.rs` (K8; research R7; depends on T021).
  - From the probe's radians (`probe/rust/survey_rust.txt`, bit patterns):
    - `e6` where degrees × 10⁶ is exactly 0.5, 2.5, 4.5 and 6.5 gives 0, 2, 4 and 6;
    - `e7` at −0.5 and −2.5 gives 0 and −2.
  - Away from ties, against the C++ rows (T027).
  - **Check** that they **fail** before T024.
- [X] T024 FR-022: round ties to even in `e5`, `e6`, `e7` in `core/src/s1/angle.rs` (research R7; depends on T023).
  - `(self.degrees() * 1eK).round_ties_even() as i32`.
  - Check that `ChordAngle`'s E-forms follow through `to_angle`, and that
    `python/tests/test_angle.py`'s E-form tests still pass.
  - **Check:** T023 passes; `make test` passes.
- [X] T025 FR-023: document the E-forms' domain and saturation in `core/src/s1/angle.rs` and `core/src/s1/chord_angle.rs` (K5, K9, K10; research R8; depends on T024).
  - **On `e5`, `e6`, `e7`:**
    - the domain upstream documents ((−180°, 180°], `s1angle.h:127`);
    - the rounding (ties to even, as C++ on x86-64; C++ elsewhere rounds ties away from
      zero);
    - outside the `i32` range: `i32::MAX` above, `i32::MIN` below, 0 for NaN, a documented
      deviation from C++'s −2147483648 (U-014).
  - **On `from_unsigned_e6`, `from_unsigned_e7`:** a value above `i32::MAX` wraps to a
    negative angle, as upstream's `static_cast`.
  - **On `ChordAngle`'s E-forms:** `INFINITY`'s are `i32::MAX`; `NEGATIVE`'s are those of
    −1 radian.
  - **The module's `#![expect]` reasons** (fact 5): "saturating `as` cast; the out-of-range
    results are documented on `e5`".
- [X] T026 [P] FR-025: record U-014 in `UPSTREAM.md` (K5, K8, K9; research R1).
  - **U-014**, in U-013's format: `s1angle.h:248–264`; `MathUtil::Round<int32_t>` is
    `FastIntRound` (`mathutil.h:172–192`).
    - `S1ChordAngle::Infinity().e5()` (a valid value) fails the debug check and returns
      −2147483648 in release builds (observed).
    - So does every out-of-range `S1Angle` E-form; +∞ and large positive angles become
      negative.
    - Off x86 the generic `Round` (`mathutil.h:71–79`) casts out of range, undefined
      behaviour, and rounds ties away from zero, so the result depends on the processor.
    - The library: saturation (FR-023), ties to even (FR-022).
    - Mark it "candidate, not reported".
  - A row in the summary table, and option B under "Upstream behaviours s2rst does not
    follow".
- [X] T027 Extend `core/tests/cpp_values_diff.rs` to the new sections (depends on T005, T013–T025; research R9).
  - **`chord_angle_matches_cpp`:** every `CHORD_ANGLE` row, bit for bit, `-` entries skipped.
    - `deviation:negative-canonical` rows: the library gives `NEGATIVE` and its
      classifications, where C++ keeps `l`.
    - `deviation:eform-saturate` rows: `i32::MAX`.
    - `sin`, `tan` and `ToAngle` bit for bit, or within 1 ulp where the platform's `sqrt`
      and `asin` differ, with the reason in a comment.
  - **`angle_matches_cpp`:** every `ANGLE` row, bit for bit.
    - `normalized` included, with no tolerance (SC-008).
    - The E-forms where C++ is in range; `deviation:eform-saturate` rows give the documented
      saturation.
    - `sin`, `cos`, `tan` within 1 ulp where the platforms' libm differ, with the reason in a
      comment.
  - Report the counts, including the deviation rows.
  - **Check:** `cargo test -p s2rst --test cpp_values_diff`.
- [X] T028 Check the bindings against the changed behaviour (Constitution IV; plan "Option B's reach").
  - Run `make -C python test` and `make -C wasm test`.
  - Add a Python test that `ChordAngle.from_length2(-5.0)` equals `ChordAngle.NEGATIVE` and
    `from_length2(float('nan'))` equals `STRAIGHT`, in `python/tests/test_angle.py`.
  - Update `python/python/s2rst/_core.pyi`'s docstrings for `from_length2`, `e5`–`e7` and
    `normalized` if they state the old behaviour.

**Checkpoint**: `make test` passes. Every chord angle is canonical, NaN gives `STRAIGHT`,
`normalized` is exact, ties go to even, the `Cap` decoder still rejects NaN, and the
documentation states every domain the proofs assume.

---

## Phase 4: User Story 1 — Proven crash freedom (Priority: P1) 🎯 MVP

**Goal**: Every in-scope operation of both types is proven free of panics, overflow,
undefined behaviour and firing debug checks over its documented domain (FR-004), with covers
showing the domains non-empty and the outcomes reachable (FR-009).

**Independent Test**: `make verify-values P=chord` and `P=angle` report every crash harness
`SUCCESSFUL`, with all covers satisfied. `verify/values/INVENTORY.md` §7–8 names a harness or
a written reason for every row.

- [X] T029 [P] [US1] Write the chord angle's crash harnesses in `verify/src/ca_crash.rs` (data-model §5; research R4; facts 1, 4).
  - **The 28 entries with crash proofs**, grouped as 006's `pt_crash`. The domains:
    - `AnyF64` for `from_length2`, `from_radians`, `from_degrees`, and `plus_error`'s error;
    - every `i32` for `from_e5`–`from_e7`;
    - `AnyAngle` for `from_angle` and `From<Angle>`;
    - `Chord` for `length2`, the five classifications, `successor`, `predecessor`,
      `plus_error`, `PartialEq`, `PartialOrd`;
    - `ChordLength` for `max_point_error`, `max_angle_error`, `sin`, `sin2`, `cos`, `tan`;
    - `ChordLength²` for `Add` and `Sub`, proving their debug checks (and T018's) never fire.
  - **The 7 entries through `asin`** get no harness. Their inventory rows say "tool limit
    (`asin` unsupported, research R0)", with R0's 0.8 s failing run.
  - Each harness's doc starts with `C-s1::ChordAngle::<op>:` and states its domain.
  - **Covers:** each constructor's branches (negative, in range, above 4, NaN); `successor` at
    `STRAIGHT` and `NEGATIVE`; `predecessor` at `ZERO` and `INFINITY`; `Add` reaching
    `STRAIGHT`; `Sub` reaching `ZERO`.
- [X] T030 [P] [US1] Write the angle's crash harnesses in `verify/src/ag_crash.rs` (data-model §5; research R4, R6; facts 1, 2, 5).
  - **The 29 entries with crash proofs**, over `AnyAngle`, `AnyF64`, every `i32` and `u32`,
    NaN and the infinities included. Where an operation's arithmetic can meet a new NaN
    (`Add` of opposite infinities, `Mul` of 0 and ∞, `Div` of 0 by 0 or ∞ by ∞), narrow the
    domain exactly there, as 006's T025 did. Replay each NaN failure first (fact 2 of 006:
    division by ±∞).
  - **`normalized`:** a crash harness over `AnyAngle`. Its doc states research R6's argument:
    every operation after the `%` is a comparison, an addition or a subtraction of finite
    values, so no value of the checker's (wrong) `%` can make it panic. No cover through the
    branches (fact 2).
  - **`tan`** gets no harness: "tool limit (`tan` unsupported, research R0)".
  - Each harness's doc starts with `C-s1::Angle::<op>:` and states its domain.
  - **Covers:** the E-forms saturating at both ends and at NaN; the unsigned forms wrapping.
- [X] T031 [US1] Run `make verify-values P=chord` and `P=angle`, and fill in the crash columns of `verify/values/INVENTORY.md` §7–8 (depends on T029, T030).
  - **All 67 rows:** name, scope, reason, domain, crash result, crash harness.
  - **The 10 excluded or limited rows:**
    - `Display` for both types (tool limit);
    - the 7 `asin` rows and `Angle::tan`, each with R0's measurement and a pointer to T027's
      vector checks.
  - **`normalized`:** the crash proof, with the note that its values are not provable by
    Kani (P-AGNRM-01).
  - Record each harness's time and peak memory from the run's `results.tsv`.
  - **Records, same change:** `verify/PROPERTIES_BY_KIND.md`'s "Never crashes" table gains
    rows for both types; re-render `verify/PROPERTIES_BY_KIND.html`
    (`pandoc -f markdown -t html`).

**Checkpoint**: US1 is complete and shippable. SC-001 and SC-002 hold.

---

## Phase 5: User Story 2 — A chord angle that holds only canonical values (Priority: P1)

**Goal**: The canonical-value invariant, implemented by T012–T020, is proven for every input
(P-CAVAL-02), with the constructors' and the decoder's behaviour stated (P-CAVAL-01, -03,
-04).

**Independent Test**: build a chord angle from every constructor, the deserialiser and the
`Cap` decoder with a negative input other than −1, and with −∞. Each yields `NEGATIVE`
(T012, T014, T019, T028). `make verify-values P=chord` proves P-CAVAL-02.

- [X] T032 [US2] Write P-CAVAL-01 to P-CAVAL-04 in `verify/src/ca_val.rs` (data-model §3, §4.1; depends on T011, T013, T017).
  - **P-CAVAL-01:** the constants' bits are upstream's; `Default` is `ZERO`; `length2`
    returns what `from_length2` stored, for every length in [0, 4].
  - **P-CAVAL-02** (validity), one member per producer if one is slow (`__ctor`, `__angle`,
    `__nbr`, `__err`, `__ari`):
    - `chord_spec::canonical(from_length2(x).length2())` for every `AnyF64`;
    - the same for `from_angle` over `AnyAngle`, the E-form constructors over every `i32`,
      `from_radians`/`from_degrees` over `AnyF64`;
    - `successor`, `predecessor` and `plus_error` over `Chord` (and `AnyF64` errors);
    - `Add` and `Sub` over `ChordLength²`.

    Drafts: `r0_ca_from_length2_draft_canonical`, `r0_ca_plus_error_canonical`,
    `r0_ca_add_range`, `r0_ca_sub_range`.
  - **P-CAVAL-03:** `from_length2(x)` is `chord_spec::upstream_from_length2(x)`, bit for
    bit, for every `AnyF64`. The doc names the option-B deviation.
  - **P-CAVAL-04:** `from_angle(a)` is `NEGATIVE` for `a.radians() < 0`, `INFINITY` at +∞, and
    in [0, 4] otherwise, NaN giving 4. `from_radians(r)`, `from_degrees(d)`, `from_eK(n)` and
    `From<Angle>` equal `from_angle` of `Angle::from_radians(r)`, etc. (one `sin` call per
    side: compare `length2` of one call each only where both go through the same `sin`
    argument, else state the range alone; never compare two `sin` results computed
    separately, as for `sqrt`). Draft: `r0_ca_from_angle_range`.
- [X] T033 [US2] Run `make verify-values P=chord` for `ca_val`, and record the results in `verify/values/PROPERTIES.md` §8 and the spec-007 "Measurements" (depends on T029, T032).
  - Move a member over 6 minutes to the weekly tier, and record why (none expected: R0's
    longest was 15.4 s).
  - **Records, same change:** `PROPERTIES_BY_KIND.md` lists P-CAVAL-01 to -04, and its HTML is
    re-rendered.

**Checkpoint**: US2 is complete. The chord angle's invariant is a proven type property
(SC-007's proven half).

---

## Phase 6: User Story 3 — Proven meaning against independent descriptions (Priority: P2)

**Goal**: The remaining catalogue entries (data-model §4) are proven against the descriptions,
by Kani or by enumeration, or recorded with a reason. Each proven one has a mutant that
breaks it.

**Independent Test**:

- `make verify-values P=chord` and `P=angle` report every property harness `SUCCESSFUL`, and
  the enumeration passes.
- `make verify-mutants M='M-6*'` reports every `M-6NN` killed.
- `cargo test --test property_kinds` passes.

### Chord-angle properties

- [X] T034 [P] [US3] Write P-CACLS-01 in `verify/src/ca_cls.rs` (data-model §4.1).
  - Over `Chord`: `is_zero` is `l == 0`; `is_negative` is `l < 0`; `is_infinity` is
    `l == +∞`; `is_special` is `chord_spec::special`; `is_valid` holds. Compare booleans with
    `==` only.
- [X] T035 [P] [US3] Write P-CAORD-01 in `verify/src/ca_ord.rs` (data-model §4.1).
  - `laws::ord::all` with `|a, b| a.partial_cmp(b).unwrap()` and `laws::eq::all` over
    `any_chord()³`; plus `partial_cmp(a, b)` is `chord_spec::less`'s order. Draft:
    `r0_ca_order_laws` (1.0 s).
- [X] T036 [P] [US3] Write P-CANBR-01 to P-CANBR-03 in `verify/src/ca_nbr.rs` (data-model §4.1).
  - **P-CANBR-01:** `successor(c).length2()` is `chord_spec::next(c.length2())`, as values
    (signed zeros may differ, so compare with `==`, and bits where neither is zero), over
    `Chord`. Draft: `r0_ca_successor_next`.
  - **P-CANBR-02:** the same for `predecessor` and `prev`. Draft:
    `r0_ca_predecessor_previous`.
  - **P-CANBR-03:** `predecessor(successor(c)) == c` below `INFINITY`, and
    `successor(predecessor(c)) == c` above `NEGATIVE`. Draft: `r0_ca_neighbours_inverse`.
  - Split by `__neg`, `__len`, `__inf` (contracts "Split families") only if needed.
- [X] T037 [P] [US3] Write P-CAERR-01 and P-CAERR-02 in `verify/src/ca_err.rs` (data-model §4.1).
  - **P-CAERR-01:** over `Chord × AnyF64`: a special value is returned unchanged; otherwise
    `plus_error(e).length2()` is `chord_spec::upstream_plus_error`, bit for bit, NaN sums
    giving 4.
  - **P-CAERR-02:** over `ChordLength`: `max_point_error` and `max_angle_error` are the
    restatements, bit for bit.
- [X] T038 [P] [US3] Write P-CAARI-01 to P-CAARI-03 in `verify/src/ca_ari.rs` (data-model §4.1; contracts "Never compare two square roots").
  - **P-CAARI-01:** over `ChordLength²`: `(a + b).length2()` is in [0, 4]; `a + ZERO` is `a`
    (bits); when `a.length2() + b.length2() ≥ 4` the sum is `STRAIGHT`.
  - **P-CAARI-02:** `(a − b).length2()` in [0, 4]; `a − ZERO` is `a`; `a ≤ b` gives `ZERO`.
  - **P-CAARI-03:** `sin2` and `cos` are the restatements, bit for bit; `sin(a)` is in
    [0, 1].
- [X] T039 [US3] Run `make verify-values P=chord`, and record times, memory and results in the spec-007 "Measurements" and the status column of `verify/values/PROPERTIES.md` §8 (depends on T029, T032–T038).

### Angle properties

- [X] T040 [P] [US3] Write P-AGVAL-01 and P-AGVAL-02 in `verify/src/ag_val.rs` (data-model §4.2).
  - **P-AGVAL-01:** over `AnyF64`: `from_radians(x).radians()` keeps the bits; `ZERO`,
    `INFINITY`, `Default` are upstream's; `abs` clears the sign bit; `Neg` flips it, and
    twice is the identity; `is_infinite` is IEEE's.
  - **P-AGVAL-02:** `laws::eq::all` and `laws::ord::all` (with `partial_cmp().unwrap()`) over
    three angles without NaN.
- [X] T041 [P] [US3] Write P-AGCNV-01 in `verify/src/ag_cnv.rs` (data-model §4.2).
  - Over `AnyF64`: `from_degrees` and `degrees` are `angle_spec`'s, bit for bit. Over every
    `i32`: `from_e5`–`from_e7` are `upstream_from_e`, bit for bit. Over every `u32`:
    `from_unsigned_eK(u)` is `from_eK(u as i32)`.
  - P-AGCNV-02's constant identities (`Degrees(180 / n) == Radians(π / n)` for n = 1..8,
    `Degrees(45 k).degrees() == 45 k` for k = 0..8): a concrete Kani harness here too.
- [X] T042 [P] [US3] Write P-AGEF-01 in `verify/src/ag_eform.rs` (data-model §4.2; research R7).
  - Over `AnyAngle`, NaN and the infinities included: `e5`, `e6`, `e7` are
    `angle_spec::upstream_e(k, r)`, bit for bit. Split `__k5`, `__k6`, `__k7` only if one
    statement is slow.
  - This is the harness that kills a mutant rounding away from zero (data-model §7).
- [X] T043 [US3] Write the enumeration tests in `verify/tests/angle_enumeration.rs` (P-AGCNV-02's integer identities, P-AGEF-02; research R12; fact 3; contracts "Modules and names").
  - Follow `verify/tests/uv_projection.rs`'s pattern: a module doc saying why enumeration is
    complete here (the domain is the integers, and Kani timed out, R0); count failures per
    statement and assert all are zero, printing the first failing `n`.
  - **One `#[test] #[ignore]` per statement**, named after its id:
    - `p_agef_02_e5`, `p_agef_02_e6`, `p_agef_02_e7`: `from_eK(n).eK() == n` for every `i32`;
    - `p_agcnv_02_e6_e7`: `from_e6(n) == from_e7(10 n)` for every `n` with `10 n` in range;
    - `p_agcnv_02_degrees`: `from_degrees(n) == from_e6(10⁶ n)` and `== from_e7(10⁷ n)`
      wherever the product fits.
  - **Check:** `cd verify && cargo test --release --test angle_enumeration -- --ignored`, about
    70 s (fact 3). Record the time.
- [X] T044 [US3] Record P-AGNRM-01 as not provable by Kani, with its evidence (data-model §4.2; research R0 Finding 1, R6).
  - Its row in `verify/values/PROPERTIES.md` §9: "not proven (tool limit: Kani's `%` on `f64`
    is wrong, research R0)".
  - The row points at:
    - the C++ rows (T027);
    - the exact-remainder property and the regression tests (T021);
    - the crash proof (T030).
  - No harness.
- [X] T045 [P] [US3] Write P-AGARI-01 in `verify/src/ag_ari.rs` (data-model §4.2).
  - Over `AnyAngle²` and `AnyF64`: `Add`, `Sub`, both `Mul`s, `Div<f64>` and `Div<Angle>` are
    the IEEE operations on radians, bit for bit, over the domain where no new NaN appears
    (T030's narrowing).
- [X] T046 [US3] Run `make verify-values P=angle` (Kani and the enumeration), and record the results in `verify/values/PROPERTIES.md` §9 and the "Measurements" (depends on T030, T040–T045).

### Non-vacuity, catalogue and index

- [X] T047 [US3] Write the `M-6NN` mutant patches in `verify/mutants/` and run them (data-model §7; research R10; contracts "Mutant patches").
  - **The patches:**
    - in 004's format (`Mutant`, `Targets`, `Expect-Fail`, `Defect`, optional `Tier: slow`),
      one defect each, from `M-601`, as data-model §7 lists;
    - at least one per proven property (SC-004), including the two description mutants;
    - generated with a script, so that the context lines match the formatted source.
  - **Not killed by Kani, by design:**
    - the enumeration's mutants name the test that kills them in a `Test:` header, which
      `run-mutants.sh` runs in the mutant's worktree (2026-10-19: `M-644`, `from_e7` with a
      perturbed scale, survived `p_agef_02_e7` and is aimed at `p_agcnv_01_e_forms`; `M-648`,
      `e7` truncating, is P-AGEF-02's);
    - the `normalized` mutants are killed by T021's and T027's tests, through `Test:` headers
      the same way.
  - **Run** `make verify-mutants M='M-6*'`, one campaign, after T039 and T046. Add a row per
    patch to `verify/MUTANTS.md`.
  - A mutant that survives means the harness is vacuous or too weak. Strengthen the harness;
    never delete the mutant.
- [X] T048 [US3] Complete `verify/values/PROPERTIES.md` §8–9, `verify/PROPERTIES_BY_KIND.md` and its HTML render, and tag the harness docs (FR-010, SC-011).
  - **`PROPERTIES.md`:** all 22 rows, with 004's columns. Statuses come from T033, T039, T043,
    T044 and T046; mutants from T047.
  - **`PROPERTIES_BY_KIND.md`:** every entry under its kind; the summary counts; re-render
    `PROPERTIES_BY_KIND.html` with `pandoc -f markdown -t html`.
  - **Harness and test docs:** each starts with `P-ID (kind): <plain statement>`.
  - **Check:** `cd verify && cargo test --test property_kinds` passes. If it does not accept
    the enumeration test as a home for an entry, extend it to read
    `verify/tests/angle_enumeration.rs` too.
- [X] T049 [P] [US3] Extend `verify/TRUSTED_BASE.md` with a spec-007 section (research R11; FR-015).
  - **The checker's model:**
    - `round_ties_even` and `floor` (checked by R0);
    - `sin` over-approximated;
    - `sqrt` under-determined;
    - `asin` and `tan` unsupported;
    - **`%` wrong** (fact 2), so `normalized`'s crash proof rests on the stated argument.
  - **The enumeration:** the library's release build on x86-64 IEEE-754 (no FMA in these
    functions), with the test names and times.
  - **`chord_spec` and `angle_spec`'s restatements** and their check against the C++ vectors.
  - **003's `lib_cap`** now models option B (T015).

**Checkpoint**: US3 is complete. SC-003, SC-004 and SC-011 hold.

---

## Phase 7: User Story 4 — Regression protection for future changes (Priority: P3)

**Goal**: The proofs run in CI on relevant changes, and stay out of the default gate (FR-014,
SC-005).

**Independent Test**: a change that breaks a proven property fails the matching `verify` leg,
naming the property. The revert passes. An unrelated change does not start the legs.

- [X] T050 [US4] Wire the new parts into `.github/workflows/verify.yml` (contracts/verify-commands.md "CI"; fact 9).
  - **The `verify` matrix** (line 132): add `values-chord` and `values-angle` to `suite`. The
    `values-*` case maps them to `make verify-values P=chord|angle`; the angle leg includes
    the enumeration (T002).
  - **The mutant matrix** (line 331): add `'6'` to `series`.
  - **The `paths` lists** already hold `core/src/s1/angle.rs`, `core/src/s1/chord_angle.rs`,
    `core/src/s2/cap.rs` and `core/src/s2/encoding.rs`. Check them.
  - **The header comment:** name spec 007 and the two new legs.
  - **`slow`:** no new leg unless T039/T046 moved a member weekly. If one did, add a
    `values-chord-angle` leg with its filters.
- [X] T051 [US4] Measure both legs against SC-005, record them in `verify/values/EVALUATION.md`, and move members to the weekly tier until each leg is under 10 minutes on the reference runner (depends on T050; research R12).
  - Measure locally with `VERIFY_JOBS=4 make verify-values P=chord`, and the same for
    `P=angle`. On CI, measure with `workflow_dispatch`, which needs the user's push approval.
- [X] T052 [US4] Check US4's acceptance scenarios with a deliberate break in `core/src/s1/chord_angle.rs` (spec US4), reverted afterwards.
  - On a scratch change, make `successor` of `STRAIGHT` step by one ulp.
  - `make verify-values P=chord` fails, naming `ca_nbr::p_canbr_01` with a concrete input.
    Revert it.
  - On CI (needs push approval), the `values-chord` leg fails, and a change touching none of
    the paths does not start the leg.
- [X] T053 [P] [US4] Document the new parts in `verify/README.md` and `CONTRIBUTING.md`.
  - **`verify/README.md`:**
    - the values section and the Layout table: `P=chord` and `P=angle`, spec 007, the
      enumeration test;
    - the tiers, with "proven by enumeration" as a method.
  - **`CONTRIBUTING.md`, "Formal verification" (line 89):** name `ChordAngle` and `Angle`, and
    add `core/src/s1/angle.rs` and `core/src/s1/chord_angle.rs` to the files before whose
    change `make verify-values` runs.

**Checkpoint**: CI runs the new legs on relevant changes. `make test` is unchanged.

---

## Phase 8: User Story 5 — Evaluation and what next (Priority: P3)

**Goal**: A maintainer who did not take part can answer SC-013's questions from the
evaluation alone (FR-016).

**Independent Test**: read the spec-007 part of `verify/values/EVALUATION.md` cold, and answer
"did enforcing the invariant pay off, and should the other value types follow?" and "what
next: `LatLng`, `Cap`, or something else?".

- [X] T054 [US5] Write the spec-007 part of `verify/values/EVALUATION.md` (research R13; FR-016; SC-013).
  - **The canonical-value invariant (option B):**
    - which laws became whole-type properties (P-CAORD-01, P-CANBR-01 to -03);
    - what it cost (T012–T015's diff, the `Cap` deviation, the bindings);
    - whether `s1::Interval`, `r1::Interval`, the rectangles and `Point` should enforce their
      invariants in the type, as a ranked recommendation.
  - **Reach into the trigonometric conversions:** what `sin` and `sqrt` allowed, what `asin`,
    `tan` and `%` stopped, and the enumeration as a second method (where it applies: finite
    integer domains).
  - **The findings:** K1–K10, U-014, Kani's `%` (with its probe), and any `D-NNN`.
  - **`LatLng` and `Cap`:** a go or no-go for each, with what they would reuse (`Angle` and
    `angle_spec` for `LatLng`; `point_spec` and `chord_spec` for `Cap`) and what blocks them
    (`atan2` for `LatLng`'s conversions).
  - **A ranked list of follow-ups:** U-014's report (T061), the next types, reporting Kani's
    `%` defect upstream to Kani (with the user's approval).
  - **Also:** effort, run times, limitations, the comparison with 004–006.
- [X] T055 [P] [US5] Update `VERIFIED_TYPES.md` (FR-026).
  - Move `ChordAngle` out of §2.7 ("reached only through other proofs") into a new section
    "2.7 `ChordAngle` and `Angle` (spec 007)", with the suite, the per-change part, the
    enumeration, the tool limits and links to the records.
  - Renumber the "reached only" section to 2.8.
  - Update §1's counts and the header's date line.
  - Update the Step 3 status, and §4's value-type row.
- [X] T056 [P] [US5] Mark spec 006's deferral note (its FR-025) as resolved by this feature: a one-line note under "Deferred: `ChordAngle`" in `specs/006-formal-verify-r2-rect-point/spec.md`, and in the `ChordAngle` section of 006's part of `verify/values/EVALUATION.md` (FR-026).

**Checkpoint**: US5 is complete. SC-013 holds.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [X] T057 Run the whole gate and the whole values suite once, one after another, and record the run in the spec-007 part of `verify/values/EVALUATION.md` ("Final run").
  - First `make test`, then `make verify-values` over all eight parts, the enumeration
    included.
- [X] T058 [P] Walk through `specs/007-formal-verify-chord-angle-angle/quickstart.md` §1–§6, and correct the quickstart where reality differs (command names, counts, times).
- [X] T059 [P] Check the spec's success criteria SC-001 to SC-013 one by one against the records, and list the result per criterion in the spec-007 part of `verify/values/EVALUATION.md` ("Success criteria").
- [X] T060 Re-run `/speckit-analyze` on `specs/007-formal-verify-chord-angle-angle/`, resolve any CRITICAL or HIGH finding, then commit the work in the order of "Commit plan" below. Push only with the user's approval.

---

## Phase 10: Follow-ups (FR-025) — recorded, not implemented in this feature

These tasks record upstream findings so that they are not lost. They need the user's decision.

- [ ] T061 U-014: report C++'s E-form behaviour upstream (K5, K8, K9; research R1).
  - Upstream: `s1angle.h:248–264` (`e5`–`e7` and their debug checks), with
    `S1ChordAngle::e5`–`e7` reaching them for `Infinity()`. `MathUtil::Round<int32_t>`
    (`mathutil.h:71–79`, `:172–192`).
  - Inputs:
    - `S1ChordAngle::Infinity().e5()`: −2147483648 in a Release build (observed;
      `probe/cpp/survey_cpp.txt`);
    - `S1Angle::Radians(1e10).e7()`: the same;
    - the tie `e6` of an angle with degrees × 10⁶ = 2.5: 2 on x86-64, 3 with the generic
      `Round`.
  - The library: saturation (FR-023) and ties to even (FR-022).
- [ ] T062 Kani's `%` on `f64`: report the mis-model to the Kani project (research R0, Finding 1), with `probe/r0_harnesses.rs`'s `r0_frem_concrete` and `r0_frem_which` as the reproducer. Needs the user's approval: it publishes the probe.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: no dependencies. T002 and T003 are [P] with T001.
- **Phase 2 (Foundational)**: needs Phase 1.
  - T005 needs T004. T010 needs T005, T006 and T008.
  - T006–T009 are [P].
  - T011 needs T006.
- **Phase 3 (library changes)**: needs Phase 1. It can run alongside Phase 2.
  - `chord_angle.rs` and `chord_angle_tests.rs`: T012 → T013 → T016 → T017 → T018 → T019 →
    T020.
  - The `Cap` decoder: T014 (needs T013) → T015.
  - `angle.rs` and `angle_tests.rs`: T021 → T022 → T023 → T024 → T025 (T025 also touches
    `chord_angle.rs`, after T020).
  - T026 is [P].
  - T027 needs T005 and T013–T025. T028 needs T013–T025.
- **Phase 4 (US1)**: needs T011 and Phase 3's T013, T017, T018, T020, T022 and T024 (the
  fixed behaviour).
- **Phase 5 (US2)**: needs Phase 3 and T011. T033 follows T031 (one Kani run at a time).
- **Phase 6 (US3)**: needs Phases 2 and 3. The runs T039 and T046 follow T033. The mutant run
  T047 needs every property harness and the enumeration.
- **Phase 7 (US4)**: needs the harnesses it runs (US1–US3).
- **Phase 8 (US5)**: needs US1–US4's measurements.
- **Phase 9 (Polish)**: needs everything above.
- **Phase 10**: T061 and T062 can be taken up at any time, with the user.

### User Story Dependencies

- **US1 (P1)**: independent after Phases 2–3. It is the MVP.
- **US2 (P1)**: its behaviour is Phase 3's. Its proof needs only the generators and
  `chord_spec`.
- **US3 (P2)**: independent of US1 and US2 except for the shared generators. Each property
  harness also discharges its operations' crash obligations.
- **US4 (P3)**: needs the suites it runs.
- **US5 (P3)**: needs everything it evaluates.

### Parallel Opportunities

- Phase 1: T002 and T003.
- Phase 2: T006, T007, T008 and T009.
- Phase 3: the chord-angle chain and the angle chain (until T025), and T026.
- US1: T029 and T030.
- US3: T034–T038 and T040–T042, T045 (different modules); T043 (a test, no Kani) beside them;
  T049. The Kani runs go one after another.
- US5: T055 and T056.
- Polish: T058 and T059.

---

## Parallel Example: User Story 3

```text
# After Phase 3, write these side by side; run them one at a time:
Task: "T035 [US3] P-CAORD-01 in verify/src/ca_ord.rs"
Task: "T036 [US3] P-CANBR-01..03 in verify/src/ca_nbr.rs"
Task: "T038 [US3] P-CAARI-01..03 in verify/src/ca_ari.rs"
Task: "T041 [US3] P-AGCNV-01 in verify/src/ag_cnv.rs"
Task: "T042 [US3] P-AGEF-01 in verify/src/ag_eform.rs"
Task: "T043 [US3] the enumeration in verify/tests/angle_enumeration.rs"
```

---

## Implementation Strategy

### MVP first (User Stories 1 and 2)

1. Phase 1, then Phases 2 and 3 (they can overlap).
2. Phase 4 (US1): every in-scope operation proven crash-free, with the tool limits recorded.
3. Phase 5 (US2): the canonical-value invariant proven.
4. **Stop and validate:** `make verify-values P=chord` and `P=angle` pass, the inventory is
   complete, and `make test` passes with the option-B behaviour.

### Incremental delivery

1. Setup, foundation and library changes: the vectors, the descriptions, option B, the NaN
   parity, the exact reduction, ties to even, the documentation.
2. US1: crash freedom.
3. US2: the invariant.
4. US3: meaning, the enumeration, the mutants.
5. US4: CI.
6. US5: the evaluation.

Each step leaves `make test` and `make verify` green, and every change to `verify/` carries
its records (constitution v1.2.0).

### Commit plan

Separate commits, in this order:

1. the spec 007 documents (`specs/007-formal-verify-chord-angle-angle/`);
2. T012 with T013 (FR-017, the type);
3. T014 with T015 (FR-017, the `Cap` decoder, with 003's model and records);
4. T016 with T017 (FR-018);
5. T018 (FR-019);
6. T019 with T020 (FR-020);
7. T021 with T022 (FR-021);
8. T023 with T024 (FR-022);
9. T025 (FR-023) and T026 (`UPSTREAM.md`);
10. the vectors (T004, T005, T027) and the bindings check (T028);
11. the `verify/` suite (Phases 1–2, US1, US2, US3), each commit with its records;
12. the CI wiring and documentation (T050, T053);
13. the evaluation and the remaining records (T054–T056, T057–T059).

Push only with the user's approval.
