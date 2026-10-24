---

description: "Task list for the formal verification of the decoding layer (spec 003)"
---

# Tasks: Formal Verification of the Decoding Layer

**Input**: Design documents from `specs/003-formal-verify-decoders/`: plan.md, spec.md,
research.md (R0–R18), data-model.md, contracts/ (verify-commands, harness-conventions,
deductive-conventions), quickstart.md, and the Verus probe `probe/uint_vector.rs`. The
feature extends 001/002's `verify/` crate and adds a new crate, `verify/deductive/`. 002's
task list (`specs/002-formal-verify-cell-union/tasks.md`) shows the established patterns.
The existing files in `verify/` are the style reference.

**Prerequisites**:

- **Kani 0.68.0**: `make verify-kani-check` passes.
- **Verus `0.2026.09.20.aef82ed`** at `~/Downloads/verus-x86-linux/verus`, with
  `rustup toolchain list` showing `1.98.1-x86_64-unknown-linux-gnu` (research R11).
- **For T005–T007 only**: a C++ toolchain (`cmake`, `g++`), the upstream checkout at
  `/home/t/sources/s2geometry` (`v0.14.0-40`, the pinned oracle plus one example commit),
  and its install at `/tmp/s2-install`. **The reboot of 2026-10-05 wiped `/tmp`**, so T006
  rebuilds the install.
- **For T004 and T066 only**: `cargo-mutants`, installed by T004.
- **Disk**: `df -h /` shows at least 20 GB free before any Kani run. `verify/target/kani`
  grows by several GB (memory note "Disk space and the Kani cache").

**Tests**: The spec asks for these ordinary tests, so they appear as tasks:

- the FR-019 regression test, which must fail without the fix (Constitution III), and a
  Python test of the visible behaviour;
- differential tests against the new C++ reference vectors (FR-024; Principle I);
- the reference description's sanity tests against the same vectors (FR-007);
- quickcheck properties and 1,000,000-element cases for sizes the proofs do not reach (R13);
- display-text tests for the R6 messages (FR-013);
- the reservation measurement (SC-008);
- the deductive track's tested agreement and drift guard (FR-011).

The Kani harnesses and the Verus proofs are the verification deliverable itself, so they are
implementation tasks.

**Organization**: Tasks are grouped by user story.

- Phase 2 holds what every story needs: the reference description, generators, the codec
  contract's bodies, the C++ vectors and the records.
- Phase 3 belongs to no story. It holds the FR-019 fix, the R6 change, the property and
  differential tests, the fuzz targets and the D-003 check. It lands before the proofs,
  because R0 shows that the vector decoders are out of the checker's reach until FR-019,
  and `Rect` and the point vector until R6.
- The FR-020 to FR-022 documentation lands in US1 (T039), once the crash proofs have fixed
  the list of panics.
- Phase 10 holds follow-ups (FR-025, FR-014). They are **not** implemented in this feature.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on unfinished tasks). For
  Kani or Verus **runs**, "parallel" applies to writing the code only. The runs themselves
  go one after another (Memory safety, below).
- **[Story]**: The user story the task belongs to (US1–US5).
- Paths are relative to the repository root.

## Memory safety while implementing (read first)

R0 saw CBMC pass 4 GiB within 20 seconds on a 2-byte input. An uncapped run can take down
the session.

- **Every** Kani run goes through a Makefile target: `make verify-decoding`,
  `make verify-harness H=…`, `make verify-mutants M=…`, `make verify-slow F=decoding` or
  `make verify`. They all apply the memory guard of `verify/run-verify.sh`.
- **Every** Verus run goes through `verify/deductive/run-verus.sh`, via
  `make verify-deductive`. It caps memory at `VERUS_MEM` (default 4G).
- For a throwaway measurement outside the Makefile, cap the run by hand. From the crate
  directory: `timeout -k 10 600 systemd-run --user --scope --unit=cd-probe-<n> -p MemoryMax=4G -p MemorySwapMax=0 cargo kani --exact --harness <h>`.
  Use a unique unit name, and run `systemctl --user reset-failed 'cd-probe-*'` before
  reusing names.
- Run separate verification jobs **one after another**: full runs, mutant campaigns, the
  mutation measurement and Verus runs.
- **Never** let a harness reach a `Vec::with_capacity` sized from decoded data, and never
  run the library's encoder and decoder in the same harness (facts 1 and 2).

## Upstream findings rule (FR-025, read first)

If a harness, a C++ vector, or reading upstream code shows **undefined or wrong behaviour in
the upstream reference** (C++ canonical, Go, Java, or the generated vectors), do these
**before** marking the affected proof complete:

1. Append a task to Phase 10 with the next free task ID. Give it the next free `U-NNN` id
   (U-004 and U-005 are reserved for the candidates in T074 and T075), the upstream
   `file:line`, the inputs, the observed behaviour, and the behaviour the Rust port
   documents (data-model §9).
2. Add the same entry under "Upstream findings" in `verify/decoding/EVALUATION.md`.
3. Cite the `U-NNN` in the affected harness's doc comment.

Never change upstream code.

## Known-defect rule (FR-014, read first)

A proof that fails on a **real library defect** gets a `D-NNN` (the next free one is D-003,
reserved for the varint tenth byte, T028). Record it under "Defects found" in
`verify/decoding/EVALUATION.md`, and list the harness in `verify/known-defects.txt` so that
runs report `KNOWN-DEFECT`. The fix is a **separate, tested change**. **Ask the user before
fixing** a defect that was not in the spec's survey, as 002 did for D-002.

## Facts these tasks rely on

1. **Reservations sized from decoded data are the verification wall** (R0 finding 2).
   `Vec::with_capacity(count)` with `count` read from the input runs out of memory at
   4 GiB even for 2-byte inputs. `count.min(65_536)` does not help. `Vec::new()` + push
   verified the same decoders at 11 bytes in 20–82 s. FR-019's fix is therefore "grow on
   demand" (R7).
2. **The library's encoder and decoder together in one harness do not fit**, even for one
   element and with a fixed-capacity writer (R0 finding 5). Part 1 of the contract is proven
   by composition (R4): encoder agreement, decoder agreement, and the model's own round trip.
3. **`format!` from decoded values** stalls CBMC in `core::fmt` (002's R0 finding 5). Fixed
   messages took `Rect` from a timeout to 3.3 s. R6 formats lazily, with the same text.
4. **Never compare collections with `==`.** It unrolls `memcmp` and failed an unwinding
   assertion in R0. Compare element-wise over a concrete maximum.
5. **The smallest unwind that passes.** Decoders: byte length + 1. Encoders: 11 (the varint
   loop). Restated code with an element loop of up to 8 bytes (`get_le`): at least 9. R0's
   agreement harness used 10.
6. **The Kani driver panic** "data did not match any variant of untagged enum
   `ParserItem`" means a run near its memory limit. Treat it as `OOM`, not as a result.
7. **`--harness cd_` selects only the new suite.** No existing harness path in `verify/src`
   contains `cd_` (checked 2026-10-05).
8. **The Verus crates on crates.io that match the release** are `vstd = "=0.0.0-2026-09-20-0158"`,
   `verus_builtin = "=0.0.0-2026-09-16-0054"` and
   `verus_builtin_macros = "=0.0.0-2026-09-20-0158"`. All are MIT. A `verus!` file builds
   with stable `rustc` and with Kani's nightly through them (R0).
9. **The pinned upstream is `/home/t/sources/s2geometry`** (`v0.14.0-40`). The checkouts in
   `~/contrib` and `~/exp` are `v0.11.1`. Their `EncodedStringVector::Init` differs, which
   caused the K3 error (R1).
10. **Count limits** (`core/`): `MAX_DECODE_COUNT = 50_000_000` (integer vectors),
    `MAX_DECODE_POINTS = 50_000_000` (point vector), string data at most `500_000_000`
    bytes. They stay, and are documented as deviations (FR-022).
11. **`read_uvarint` and `write_uvarint` are `pub(crate)`** in `core/src/s2/encoding.rs`.
    `verify/` reaches them only through the public decoders. Tests inside `core/` can call
    them directly.
12. **`io::Error::new(kind, e)` with a custom error type** boxes `e` and formats nothing until
    displayed. `to_string()` prints `e`'s `Display`, so displayed messages are unchanged
    (002's fact 6). Python's `io_err` (`python/src/encoding_rich.rs:28`) uses `Display`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: The `decoding` feature and empty modules, the deductive crate's skeleton, the
Makefile entry points, and the "before" mutation measurement on the unmodified tree
(research R2, R11, R14; contracts/verify-commands.md).

- [X] T001 Add the `decoding` feature and the empty decoding modules to `verify/Cargo.toml` and `verify/src/lib.rs` (research R2; contracts/harness-conventions.md).
  - **`verify/Cargo.toml`:** `default = ["cellunion", "decoding"]` and `decoding = []`, with
    a comment: the decoding-layer proofs (spec 003); `--harness cd_` selects them. Update the
    header comment to name all three specs.
  - **New files**, each starting with the two SPDX lines and
    `// Written for this crate (not ported from upstream S2).`, as in
    `verify/src/union_domain.rs`. Each gets a one-line `//!` doc naming its data-model
    section or property area:
    - `verify/src/wire_spec.rs` and `verify/src/wire_spec_tests.rs`;
    - `verify/src/codec.rs` and `verify/src/wire_domain.rs`;
    - `verify/src/cd_crash.rs`, `verify/src/cd_{fw,uv,cv,sv,pv,fx}.rs` and
      `verify/src/cd_agree.rs`.
  - **`verify/src/lib.rs`:**
    - `pub mod wire_spec;`, unconditional, next to `pub mod union_spec;`, with
      `#[cfg(test)] #[path = "wire_spec_tests.rs"] mod wire_spec_tests;` in `wire_spec.rs`;
    - the other modules behind `#[cfg(all(kani, feature = "decoding"))]`;
    - extend the crate doc to name spec 003, and link `decoding/INVENTORY.md` and
      `decoding/PROPERTIES.md`.
  - **Checks:** `cd verify && cargo build && cargo test`;
    `cargo build --no-default-features`; `cargo kani --only-codegen`. Codegen needs no cap.
- [X] T002 [P] Create the deductive crate skeleton in `verify/deductive/` (research R11; contracts/deductive-conventions.md).
  - **`verify/deductive/Cargo.toml`:**
    - package `s2rst-deductive`, `version = "0.0.0"`, `edition = "2024"`, `publish = false`;
    - its own empty `[workspace]`;
    - `[dependencies]`: the three Verus crates pinned with `=` (fact 8);
    - `[dev-dependencies]`: `s2rst = { path = "../../core" }`, `quickcheck` (the version
      `core` uses), and `syn` (features `full`, `visit`) and `quote`;
    - `[lints.rust]`: `unsafe_code = "forbid"`, plus
      `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(verus_keep_ghost)', 'cfg(verus_keep_ghost_body)', 'cfg(kani)'] }`.
      If the `verus!` expansion emits other cfgs, add them here with a comment.
  - **`verify/deductive/src/lib.rs`:** `//!` docs (purpose, how to run, a link to the
    contract), and `pub mod varint; pub mod fixed_width; pub mod uint_vector; pub mod
    cellid_vector; pub mod string_vector;`.
  - **The five module files:** SPDX header, `use vstd::prelude::*;`, and an empty
    `verus! { }` block.
  - **`verify/deductive/run-verus.sh`** (executable, SPDX header):
    - locate Verus from `$VERUS`, else `verus` on `PATH`;
    - check that `verus --version` reports `$VERUS_VERSION` (default `0.2026.09.20.aef82ed`),
      and fail with an install hint otherwise;
    - run `verus --crate-type=lib src/lib.rs --rlimit 60 --triggers-mode silent` under
      `ulimit -v` of `${VERUS_MEM:-4G}`, inside `systemd-run --user --scope -p
      MemoryMax=… -p MemorySwapMax=0` when available (copy the detection from
      `verify/run-verify.sh`);
    - print the contract's output line, and exit non-zero on any error;
    - with `MUTANTS=1`, run the `V-NNN` mode (T059).
  - **`verify/deductive/mirrors.toml`:** a header comment with the schema (data-model §6),
    and no entries yet.
  - **`verify/deductive/tests/agree.rs` and `tests/drift.rs`:** each with one passing
    placeholder test and a `//!` saying which task fills it (T057, T056).
  - **Checks:** `cd verify/deductive && cargo +stable test` passes, and
    `VERUS=~/Downloads/verus-x86-linux/verus ./run-verus.sh` reports 0 errors.
- [X] T003 Add the decoding targets to `Makefile` (contracts/verify-commands.md). Needs T001, T002.
  - **Variables:** `VERUS_VERSION := 0.2026.09.20.aef82ed`, and `VERUS ?= verus`.
  - **`verify-decoding`:** `cd verify && cargo test`, then
    `cd verify/deductive && cargo test`, then `cd verify && ./run-verify.sh --harness cd_`.
    Help: "Run the decoding-layer proofs only (spec 003)".
  - **`verify-verus-check`:** check `$(VERUS) --version` and the 1.98.1 toolchain, and print
    install hints on failure.
  - **`verify-deductive`:** depends on `verify-verus-check`. It runs
    `cd verify/deductive && VERUS=$(VERUS) VERUS_VERSION=$(VERUS_VERSION) ./run-verus.sh`,
    then `cargo test`, and passes `MUTANTS` through.
  - **`verify-deductive-bless`:** `cd verify/deductive && BLESS=1 cargo test --test drift`.
  - **`verify-mutation-measure`:** runs `verify/run-mutation-measure.sh $(PHASE)` (T004).
  - **`verify`:** also runs `cd verify/deductive && cargo test`. Update its help to name
    three suites.
  - Update the section's header comment to name spec 003.
  - **Check:** `make help` lists the new targets. `make verify-decoding` runs and finds no
    `cd_` harnesses yet. `make verify-deductive VERUS=~/Downloads/verus-x86-linux/verus`
    passes.
- [X] T004 Install cargo-mutants and record the **before** mutation measurement on the unmodified library (FR-018; research R14). Run it alone; it takes hours.
  - Install the latest release: `cargo install --locked cargo-mutants`. Record the version
    it reports.
  - **Write `verify/run-mutation-measure.sh`** (SPDX header, executable). Argument:
    `before` or `after`. It runs:
    ```text
    cargo mutants -p s2rst --jobs 2 \
      --file core/src/s2/encoded_uint_vector.rs --file core/src/s2/encoded_s2cell_id_vector.rs \
      --file core/src/s2/encoded_string_vector.rs --file core/src/s2/encoded_s2point_vector.rs \
      --file core/src/s2/encoding.rs \
      --re '<in-scope functions in encoding.rs>' --exclude-re '<compact point-vector encoder>' \
      -- --lib --test decoder_robustness --test property_tests [--test cpp_coding_diff, after]
    ```
    - Build the `--re` list from the inventory's `encoding.rs` entries: the `CellId`,
      `Point`, `Cap`, `Rect` and `Cell` `encode`/`decode`, the `read_*`/`write_*` helpers,
      `read_uvarint` and `write_uvarint`. Restrict the other files only by
      `--exclude-re` for the compact encoder (`encode_compact` and the functions only it
      calls). Write the final patterns into the script with a comment.
    - It copies `mutants.out/outcomes.json` to `verify/decoding/mutants-<phase>.json`.
  - **`verify/decoding/MUTATION.md`:** create it with data-model §8's schema and a **before**
    table from the JSON: counts of caught, missed, timeout and unviable, then one row per
    missed or timed-out mutant. The `proofs` column reads `n/a (no proofs yet)`.
  - **Check:** the table's total equals the number of mutants cargo-mutants reports. Note
    the wall time in the file.

**Checkpoint**: `make verify-decoding` and `make verify-deductive` run (both empty).
`make test` is unchanged. MUTATION.md has its before table.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The C++ reference vectors, the reference description and its sanity tests, the
generators, the contract's generic bodies, and the records.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [X] T005 Rebuild the s2geometry install at `/tmp/s2-install`, wiped by the 2026-10-05 reboot. Follow `tools/refgen-cpp/README.md` (lines 15–31), with the checkout `/home/t/sources/s2geometry`.
  - **Check:** `/tmp/s2-install/include/s2/encoded_string_vector.h` exists, and
    `tools/refgen-cpp`'s existing generators build (`cmake -B build -DCMAKE_PREFIX_PATH=/tmp/s2-install && cmake --build build`).
- [X] T006 Write `tools/refgen-cpp/generate_coding.cc` and register it (FR-024; research R10; data-model §10). Needs T005.
  - Follow `generate_cellunion.cc`: its header comment, output `kind,input,expected,note`
    with hex-encoded bytes, `reject` for a rejection, and `note = deviation` where the
    library documents a different answer (data-model §1, "Library deviations").
  - **Sections and inputs:**
    - **`VARINT_DECODE`**: canonical forms of `0`, `1`, `127`, `128`, `2^32`, `2^63`,
      `u64::MAX`; over-long forms (`80 00`, `ff 80 00`); tenth byte `00`, `01`, `02`,
      `7f` (via `Decoder::get_varint64`); truncations of each.
    - **`UINT32_ENCODE` / `UINT64_ENCODE`**: value lists for every width 1–4 / 1–8, empty,
      all-zero, the maximum value, and lengths 1, 2, 3, 17.
    - **`UINT32_DECODE` / `UINT64_DECODE`**: every encoding above and every truncation of
      it; the K2 headers (`varint(50_000_000·sizeof(T))` with no data); a count
      `50_000_001` with all its data present (a `deviation` row); a header whose
      `size_ > max / sizeof(T)`.
    - **`CELLID_VECTOR_ENCODE` / `_DECODE`**: id lists that hit every `base_len` 0–7 and
      every shift code 0–31, including the extended code with its second byte, plus
      invalid ids (the format carries any bits); malformed headers (shift above 56, a
      truncated base).
    - **`STRING_VECTOR_ENCODE` / `_DECODE`**: string lists (empty, empty strings,
      `["abcde", "", "cdef"]`); offsets `[5, 2, 6]` over `abcdef` (decreasing, K3);
      offsets beyond the data; truncated data.
    - **`POINT_VECTOR_ENCODE` / `_DECODE`**: FAST-encoded lists (0, 1 and 3 points,
      including NaN and non-unit bit patterns); format tags 2–7; the K2 header
      (`varint(50_000_000 << 3)` with no data).
    - **`FIXED_ENCODE` / `FIXED_DECODE`**: `CellId` (valid, `0`, face 6), `Point` (NaN,
      infinities), `Cap` (valid, empty, full, NaN centre, negative radius), `Rect` (valid,
      versions 0, 1 and 2, invalid bounds), `Cell` (valid id, `0`). Mark the `CellId` and
      `Cap` rows that C++ accepts but the library rejects `deviation` (K4).
  - **Register** `add_executable(generate_coding …)` in `tools/refgen-cpp/CMakeLists.txt`,
    and add its section table to `tools/refgen-cpp/README.md`.
  - **Upstream findings rule:** if a C++ decode crashes or trips a sanitizer (for example
    `S2Cell::Decode` on id `0`, candidate U-004), skip that input in the generator, comment
    why, and confirm or dismiss T074.
- [X] T007 Generate `core/tests/data/cpp_coding_vectors.csv`. Needs T006.
  - Run `LD_LIBRARY_PATH=/tmp/s2-install/lib ./build/generate_coding > ../../core/tests/data/cpp_coding_vectors.csv`.
  - Commit the CSV. Check its size (under 1 MB) and that every section has rows:
    `cut -d, -f1 … | sort | uniq -c`.
- [X] T008 Implement the reference description in `verify/src/wire_spec.rs` (FR-007; data-model §1; research R3).
  - **Independence:** use only plain Rust and `crate::spec` (for `spec::is_valid`). Never
    call any `s2rst` coding module. Write each function from the upstream source named in
    data-model §1, and cite it (file and line at `v0.14.0-40`) in its doc comment.
  - **Functions** (each reference decoder returns `Option<(V, usize)>`):
    - `ref_varint`, `enc_varint`;
    - `ref_fixed(b, len)`, `enc_fixed(v, len)`;
    - `ref_uint_vector::<4|8>`, `enc_uint_vector`;
    - `ref_cellid_vector`, `enc_cellid_vector` (upstream's base and shift search, restated
      from `encoded_s2cell_id_vector.cc`, not from `core/`);
    - `ref_string_vector`, `enc_string_vector` (v0.14's lazy rule: an invalid entry is
      empty, and decoding continues from its end);
    - `ref_point_vector_fast`, `enc_point_vector_fast`;
    - `ref_cell_id`, `ref_point`, `ref_cap`, `ref_rect`, `ref_cell` and their encoders;
    - the deviation predicates `DEV_COUNT`, `DEV_STRING_TOTAL`, `DEV_VALID_ID`,
      `DEV_VALID_CAP`, `DEV_VARINT_TENTH`, as `fn`s with the data-model's names in
      lower case;
    - `lib_<fmt>` wrappers that apply the deviations on top of `ref_<fmt>`. This is the
      "upstream minus deviations" answer the agreement properties use.
  - **Loops** over slices are indexed (`for i in 0..len`), never iterator adapters (memory
    note "Kani collection-harness lessons").
- [X] T009 [P] Write hand-made unit tests of the description in `verify/src/wire_spec_tests.rs` (Constitution III). Needs T008.
  - Cover at least: a varint of each length 1–10; the tenth-byte rule; width minimality; a
    cell-id vector with `base_len` 0 and 7 and the extended shift; the string rule on
    `[5, 2, 6]`; the point-vector tag; `Rect` versions 0, 1 and 2; and every deviation
    predicate on both sides of its limit.
- [X] T010 Write `verify/tests/wire_spec_vs_cpp.rs`. For every row of `cpp_coding_vectors.csv`, the reference decoder or encoder gives C++'s answer, and for `deviation` rows the `lib_` wrapper gives the library's documented answer (FR-006 P-PAR-01). Needs T007, T008.
  - Parse the CSV as `verify/tests/union_spec_vs_cpp.rs` does. Report row numbers on
    failure.
  - **Check:** `cd verify && cargo test --test wire_spec_vs_cpp` passes.
- [X] T011 [P] Implement the generators in `verify/src/wire_domain.rs` (contracts/harness-conventions.md, "Generators"). Re-export 002's `with_bytes::<L>` rather than copying it.
- [X] T012 Implement the contract's generic bodies in `verify/src/codec.rs` (FR-004; research R4; data-model §3).
  - The `Codec` trait exactly as in data-model §3.
  - **`check_encoder::<C, const N: usize>`**: for a value sequence from the generator, the
    library's bytes equal `enc_<fmt>`'s, compared byte by byte over a concrete maximum.
  - **`check_agreement::<C, const L: usize>`**: on `with_bytes::<L>`, the library's
    `decode(&mut &b[..])` and `C::reference_decode(&b)` agree on acceptance, on `same`, and
    on bytes consumed (`L − remaining.len()`).
  - **`check_model_round_trip::<C, const N: usize>`**: `reference_decode(enc(v) + rest)`
    returns `(v, |enc(v)|)` for a symbolic one-byte `rest`.
  - Element-wise comparisons only (fact 4). No codec implementations yet (T041).
- [X] T013 Create the records in `verify/decoding/` (FR-002, FR-006, FR-017).
  - **`INVENTORY.md`:** 24 rows (data-model §5), with 002's columns plus "Reservation
    evidence" and "Deductive mirror". Entry 13 (the compact encoder) is
    `excluded (by design)`. The others are `in-scope`, with the bound planned in research
    R5.
  - **`PROPERTIES.md`:** the 32 entries of data-model §4.1, status `draft`, with the
    statement, the upstream source (`file:line`), the track(s) and the planned harness or
    lemma.
  - **`EVALUATION.md`:** 002's section headings (Summary, Effort log, Exclusions and tool
    limits, Bounds reached, Upstream findings, `# Panics` documentation added, Defects
    found, Measurements, Comparison with 001 and 002, Limitations, Recommendations, Notes
    from implementation, Acceptance), plus:
    - "Kani vs Verus on shared properties";
    - "Mutation measurement";
    - "The other branch's findings";
    - "FR-019 benchmark".

    Seed "Defects found" with the D-003 candidate, and "Upstream findings" with the
    U-004/U-005 candidates (research R1, R16).

**Checkpoint**: `cd verify && cargo test` passes with the description's sanity tests. The
records exist.

---

## Phase 3: Behaviour Fix, Deferred Formatting, Tests and Fuzzing (FR-019, R6, FR-023, FR-024, R13), no story

**Purpose**: Change `core/` so the decoders become checkable and K2 is fixed, and add the
ordinary tests the spec requires. Each item is its own commit.

- [X] T014 Record the iai-callgrind baseline of `core/benches/encoded_uint_vector.rs` before FR-019 (Constitution V; plan "Constitution Check").
  - Run `cargo bench -p s2rst --bench encoded_uint_vector`, with no `target-cpu=native`.
  - Copy the instruction counts per benchmark into "FR-019 benchmark" of
    `verify/decoding/EVALUATION.md`, with the commit hash.
- [X] T015 Write the FR-019 regression test in `core/tests/decoder_robustness.rs`, and see it **fail** on the unfixed library (FR-019; SC-008; research R7).
  - **Child test**, `#[ignore]`d and `#[cfg(target_os = "linux")]`: `reservation_child`.
    It decodes each K2 input and asserts `is_err()`:
    - `decode_uint_vector_u64(varint(50_000_000 * 8))`;
    - `decode_uint_vector_u32(varint(50_000_000 * 4))`;
    - `decode_s2cell_id_vector`: the byte `0x00` (shift 0, base length 0) followed by the
      u64 K2 header;
    - `decode_string_vector`: `encode_uint_vector_u64(&[499_999_999])` with no data;
    - `decode_s2point_vector(varint(50_000_000 << 3))`.

    Each input is at most 16 bytes. Write a local `varint` helper in the test file.
  - **Parent test**, `reservation_under_address_space_limit`, Linux only. It runs
    `std::process::Command::new("sh").args(["-c", "ulimit -v 262144 && exec \"$0\" --exact reservation_child --ignored --test-threads=1", <current_exe>])`
    and asserts the child exited successfully. On failure it prints the child's stderr and
    signal.
  - **Check:** `cargo test -p s2rst --test decoder_robustness reservation` **fails** now. The
    child dies by SIGABRT with "memory allocation of … bytes failed". Note it in the commit
    message of T016.
- [X] T016 Fix FR-019: grow on demand (research R7). Needs T015.
  - `core/src/s2/encoded_uint_vector.rs`: in both `decode_uint_vector_u32` and `_u64`,
    `Vec::with_capacity(count)` → `Vec::new()`.
  - `core/src/s2/encoded_s2cell_id_vector.rs`: `Vec::with_capacity(deltas.len())` →
    `Vec::new()`.
  - `core/src/s2/encoded_string_vector.rs`:
    - replace `vec![0u8; total_len as usize]` + `read_exact` with
      `let mut data = Vec::new(); let n = r.take(total_len).read_to_end(&mut data)?;`;
    - if `n as u64 != total_len`, return `io::ErrorKind::UnexpectedEof`, with a fixed
      message;
    - `Vec::with_capacity(offsets.len())` → `Vec::new()`.
  - `core/src/s2/encoded_s2point_vector.rs`: `decode_uncompressed`'s
    `Vec::with_capacity(count)` → `Vec::new()`. Also audit `decode_cell_ids_raw` and
    every other `with_capacity` reached from `decode_s2point_vector`, and apply the same
    change to any whose size comes from decoded data.
  - Keep the count limits (fact 10), and keep every message text.
  - **Checks:** T015 passes. `cargo test -p s2rst` passes. Run the fuzz targets
    `decode_uint_vector_u64`, `decode_string_vector` and `decode_s2point_vector` for 60 s
    each.
  - **Commit message:** the K2 inputs, the before and after results of T015, and "Python
    inherits the fix".
- [X] T017 Record the iai-callgrind numbers after FR-019, next to T014's (Constitution V). Needs T016.
  - Same command as T014. Write both columns and the relative change into "FR-019
    benchmark".
  - If any decode benchmark grows by more than 5%, say so in the evaluation's
    "Limitations", with the reallocation count as the explanation. Do not change the fix
    without asking the user.
- [X] T018 [P] Measure the reservation per K2 input in `verify/tests/reservation.rs`, with `stats_alloc = "0.1.10"` as a dev-dependency of `verify/` (SC-008; research R7). Needs T016.
  - `#[global_allocator] static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;` (no
    `unsafe` in our code).
  - For each of T015's five inputs: `Region::new(GLOBAL)`, decode, then assert
    `change().bytes_allocated <= 2 * 1024 * 1024`.
  - **Check:** `cd verify && cargo test --test reservation` passes, and `grep -n unsafe verify/tests/reservation.rs` prints nothing.
- [X] T019 [P] Add a Python test of the visible K2 behaviour in `python/tests/test_encoding_rich.py` (Constitution IV; plan "Testing").
  - `test_k2_inputs_raise_under_address_space_limit`, skipped off Linux. It runs a
    `subprocess` with `python -c` that calls `resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))`
    and then `s2rst.decode_s2cell_id_vector(...)` and `s2rst.decode_s2point_vector(...)` on
    T015's inputs, each in `try/except ValueError`. The test asserts exit code 0.
  - **Check:** `cd python && make test` passes. On the commit before T016 the subprocess
    aborts (verify once and note it in the commit message).
- [X] T020 Format the `Rect` and point-vector decoders' error messages lazily (FR-013; research R6).
  - **`core/src/s2/encoding.rs`, `impl S2Decode for Rect`:** a private
    `enum RectDecodeError { UnsupportedVersion(u8) }` with `Display` printing exactly
    `unsupported encoding version {version}`, and `io::Error::new(InvalidData, …)`.
  - **`core/src/s2/encoded_s2point_vector.rs`:** a private `PointVectorDecodeError` with
    `UnknownFormat(u8)` and `InvalidLevel(i32)`, printing exactly
    `unknown S2PointVector format {format}` and `invalid level {level}`.
  - Follow `CellUnionDecodeError` in `core/src/s2/encoding.rs`: its derives, its
    `std::error::Error` impl and its doc comments.
  - **Tests** in new sibling files, `core/src/s2/encoding_tests.rs` (wired with
    `#[cfg(test)] #[path = "encoding_tests.rs"] mod encoding_tests;`) and
    `core/src/s2/encoded_s2point_vector_tests.rs` (T026 creates the wiring; coordinate):
    each message's `to_string()` equals today's text for a sample value.
  - **Check:** `cargo test -p s2rst` passes. `rg 'format!' core/src/s2/encoding.rs` no
    longer matches inside `impl S2Decode for Rect`.
- [X] T021 Write the differential tests `core/tests/cpp_coding_diff.rs` against every row of `cpp_coding_vectors.csv` (FR-024; Principle I). Needs T007, T016, T020.
  - SPDX header and "original test" note (Constitution, Licensing). Parse as
    `core/tests/cpp_cellunion_diff.rs` does.
  - **ENCODE rows:** the library's bytes equal C++'s.
  - **DECODE rows:** acceptance and value equal C++'s. For `deviation` rows, the library's
    answer equals the documented deviation (reject). Point values compare bit for bit.
  - **VARINT_DECODE rows:** checked through `decode_uint_vector_u64` headers (fact 11) in
    this external test. The tenth-byte rows are `#[ignore]`d with a pointer to D-003 (T028)
    until D-003 is decided.
  - **Check:** `cargo test -p s2rst --test cpp_coding_diff` passes.
- [X] T022 [P] Add quickcheck properties and a 1,000,000-element case in `core/src/s2/encoded_uint_vector_tests.rs`, wired from `encoded_uint_vector.rs` (R13; SC-008).
  - Round trip of arbitrary `Vec<u32>` and `Vec<u64>`, with nothing left unread.
  - Width minimality: the header's width field matches the largest element's minimal byte
    length.
  - A deterministic 1,000,000-element `u64` vector (values `i * 0x9e37_79b9_7f4a_7c15`)
    round-trips.
  - The existing inline `mod tests` stays.
- [X] T023 [P] Do the same in `core/src/s2/encoded_s2cell_id_vector_tests.rs`: round trip of arbitrary `Vec<CellId>` of any bits, and a 1,000,000-id case (R13).
- [X] T024 [P] Do the same in `core/src/s2/encoded_string_vector_tests.rs` (R13, FR-020).
  - Round trip of arbitrary `Vec<Vec<u8>>`.
  - The FR-020 rule as a property: for arbitrary offsets and data, each decoded entry is
    `data[prev..end]` when `prev ≤ end ≤ data.len()`, else empty, with `prev` the previous
    offset (0 first).
  - A 1,000,000-string case.
- [X] T025 [P] Do the same in `core/src/s2/encoded_s2point_vector_tests.rs`: FAST round trip of arbitrary point bit patterns (`f64::from_bits`), and a 1,000,000-point case (R13). Coordinate the file's creation with T020.
- [X] T026 [P] Add the fuzz targets `fuzz/fuzz_targets/decode_cell_id.rs`, `decode_point.rs` and `decode_rect.rs`, their `[[bin]]` entries in `fuzz/Cargo.toml`, and the three names in `.github/workflows/fuzz.yml`'s matrix (FR-023; research R9).
  - Follow `fuzz/fuzz_targets/decode_cap.rs`: decode, and on success encode, decode again
    and re-encode. Assert bit-identical bytes (for `Point`, compare the bytes, not the
    floats).
  - **Check:** `cargo +nightly fuzz build`, then run each target for 60 s with
    `-max_total_time=60`.
- [X] T027 [P] Extend `core/tests/decoder_robustness.rs`'s seed sets with the new C++ malformed inputs for the vector decoders, and keep `no_decoder_panics_on_any_input` passing (FR-023).
- [X] T028 Confirm the D-003 candidate and ask the user how to proceed (FR-014; research R1).
  - Add a test in `core/src/s2/encoding_tests.rs` calling `read_uvarint` on
    `ff ff ff ff ff ff ff ff ff 02`. Today it returns `Ok` (bits dropped), where
    upstream's `Parse64WithLimit` rejects (`util/coding/varint.cc:260–262`). Mark the
    test `#[ignore = "D-003"]` and have it assert upstream's answer, so it fails until a
    fix lands.
  - Record D-003 under "Defects found" in `verify/decoding/EVALUATION.md`:
    - the inputs;
    - the in-scope impact: none, because every such value is a count above the limits;
    - the out-of-scope callers: `rg -n 'read_uvarint' core/src`.
  - **Outcome (2026-10-06):** the user chose "fix now" for D-003 (`a0b7487`) and "document as a
    deviation" for D-004, a NaN cap `length2` found by T006's rows (rustdoc in T039).
  - **Ask the user:** fix in this feature (as 002's D-002), or defer to T076?
    - **If fix:** reject a tenth byte above 1 in `read_uvarint` and in
      `encoded_s2point_vector.rs`'s `reconstruct_uvarint`, with the same message as the
      other overflow. Un-ignore the test and T021's tenth-byte rows. Then run
      `cargo test -p s2rst` and the out-of-scope decoders' robustness tests.
    - **If defer:** leave the ignores, and keep T076.

**Checkpoint**: `make test` passes. The vector decoders grow on demand, and `Rect` and the
point vector format lazily. The C++ vectors, differential tests, sibling property tests and
fuzz targets exist.

---

## Phase 4: User Story 1 — Proven crash-freedom on untrusted bytes (Priority: P1) 🎯 MVP

**Goal**: Every in-scope operation is proven free of panics, overflow, undefined behaviour,
abort and non-termination within its stated bounds (SC-001). The decoders are proven over
every byte string up to their bound. Every proven panic is documented (FR-021, FR-022).

**Independent Test**:

- `make verify-decoding` reports every crash harness named in
  `verify/decoding/INVENTORY.md` as `SUCCESSFUL`, with all covers satisfied, and exits 0.
- With `M-200` applied, `cd_crash::c_get_uint_with_length` fails and prints a
  concrete-playback test.

Every harness follows contracts/harness-conventions.md:
- a `#[kani::proof]` whose doc comment's first line is the row's `C-<entry>` statement;
- `kani::assume` of exactly the inventory preconditions;
- the smallest passing unwind (fact 5);
- covers for each distinct outcome (FR-009);
- families from a `macro_rules!`, as in `verify/src/cu_enc.rs`.

Develop one member with `make verify-harness H=cd_crash::<name>`, then the family with
`make verify-decoding`.

- [X] T029 [US1] Measure the bounds that research R5 leaves open, before writing those families, and record them (clarification 2026-10-05). Needs Phase 3.
  - Write these as the first real members, in `verify/src/cd_crash.rs`, and run each alone
    with `make verify-harness`:
    - `c_decode_string_vector__b0 … __b4`;
    - `c_decode_s2point_vector__t0_b1`, `__t0_b25`, `__t1_b2`, `__t1_b11`, `__tx_b1`;
    - `c_decode_rect__b33`;
    - `c_decode_s2cell_id_vector__b11`.
  - **The string vector and the point vector:** for any member that runs out of memory at
    4 GiB or exceeds 5 minutes, re-run it with `VERIFY_MEM_PER_HARNESS=8G
    VERIFY_HARNESS_TIMEOUT=30m`, alone. Its family's bound becomes the largest length that
    fits:
    - **per change**: 4 GiB, 5 minutes;
    - **weekly** (`#[cfg(feature = "slow")]`): 8 GiB, 30 minutes.
  - **Tool-limit rule:**
    - If no string-vector length `K ≥ 3` fits even weekly, the string-vector decoder becomes
      `tool-limit` in the inventory, with these measurements.
    - If no compact-branch length `K ≥ 2` fits, the compact branch becomes a `tool-limit`
      part of entry 14, and `__t1` members are not written.
    - Each exclusion goes in "Exclusions and tool limits" with its measurements.
    - **If SC-002 would fall below 20 of 24**, stop and report to the user.
  - Record every measurement in "Bounds measured first" of
    `verify/decoding/EVALUATION.md`, and the final bounds in INVENTORY.md.
- [X] T030 [P] [US1] Write the fixed-width crash harnesses in `verify/src/cd_crash.rs`: `c_encode_uint_with_length` (any value, any `len ≤ 8`); `c_get_uint_with_length__d<k>` (a data slice of exactly `k ≤ 8` bytes, any `len ≤ k`); `c_decode_uint_with_length__b0 … __b9` (any `len ≤ 8`). Precondition `len ≤ 8`, from upstream's DCHECK (FR-005). Covers: `len = 0` and `len = 8`; and for the stream reader, both success and `UnexpectedEof`.
- [X] T031 [P] [US1] Write the fixed-size decoder families in `verify/src/cd_crash.rs`: `c_decode_cell_id__b0 … __b9`, `c_decode_cell__b0 … __b9`, `c_decode_point__b0 … __b25`, `c_decode_cap__b0 … __b33`, `c_decode_rect__b0 … __b34`.
  - Covers per family: success, truncation, and each rejection branch. That means an
    invalid id for `CellId`/`Cell`, an invalid cap, and for `Rect` version 2 and invalid
    bounds.
  - At the encoded length plus one, also cover "one byte left unread".
  - Apply the upstream findings rule to `Cell` (candidate U-004, T074).
- [X] T032 [P] [US1] Write the fixed-size encoder crash harnesses in `verify/src/cd_crash.rs`:
  - `c_encode_cell_id`: any `u64`;
  - `c_encode_point`: any bit patterns;
  - `c_encode_cap`: `Cap::from_center_chord_angle` of any bits;
  - `c_encode_rect`: any four `f64` in the public fields;
  - `c_encode_cell`: from `any_valid_cell_id()`, since a `Cell` exists only for a valid id.

  The writer is `Vec::with_capacity(40)`.
- [X] T033 [P] [US1] Write the integer vector decoder families in `verify/src/cd_crash.rs`: `c_decode_uint_vector_u32__b0 … __b11` and `c_decode_uint_vector_u64__b0 … __b11`, unwind `L + 1`. Covers: an empty vector, the longest vector the length allows, a count over the limit (with a 4-byte header), a varint overflow (at `L ≥ 10`), and truncation.
- [X] T034 [P] [US1] Write `c_decode_s2cell_id_vector__b0 … __b11` in `verify/src/cd_crash.rs`. Covers: base length 0 and 7, the extended shift code, a shift above 56 rejected, and a decoded id that is not a valid cell. Move members over 5 minutes to the weekly tier.
- [X] T035 [P] [US1] Write the string vector decoder family at T029's bound, or record its tool limit, in `verify/src/cd_crash.rs`. Covers: zero strings, an empty string, an out-of-order entry decoded as empty (FR-020), and data truncated.
- [X] T036 [P] [US1] Write the point vector decoder families per tag at T029's bounds in `verify/src/cd_crash.rs` (`__t0`, `__t1` unless excluded, `__tx`). Covers: a decoded point (if the bound allows), an unknown tag rejected, a count over the limit, truncation.
- [X] T037 [P] [US1] Write the vector encoder crash families `c_encode_uint_vector_u32__n0 … __n2`, `_u64__n0 … __n2`, `c_encode_s2cell_id_vector__n0 … __n2`, `c_encode_string_vector__n0 … __n2` (strings of up to 2 bytes) and `c_encode_s2point_vector_fast__n0 … __n2` in `verify/src/cd_crash.rs`. Use a writer with a concrete capacity, unwind 11, and cover every width class reached.
- [X] T038 [US1] Add `verify/mutants/M-200-get-uint-reads-past-input.patch` (US1 scenario 4; contracts/harness-conventions.md).
  - The defect: `get_uint_with_length` copies `data[..length + 1]`. Header lines follow
    `verify/mutants/M-100-leaf-count-overflow.patch`: `Mutant`, `Targets` (the C- entry),
    `Expect-Fail` (`cd_crash::c_get_uint_with_length__d8`), and `Defect`.
  - **Check:** `make verify-mutants M=M-200` reports `killed`, with a concrete playback.
- [X] T039 [US1] Write the documentation commit for FR-020, FR-021 and FR-022 (research R8; SC-010). Needs T030–T037, so the list of panics is final.
  - **`# Panics`** on `encode_uint_with_length`, `get_uint_with_length` and
    `decode_uint_with_length`, with the exact conditions. Add any other panic a crash proof
    had to assume.
  - **Deviation notes** in the rustdoc of `impl S2Decode for CellId` and `impl S2Decode for
    Cap`, citing `s2cell_id.cc:261–265` and `s2cap.cc:340–353`, with the reason.
  - **`# Errors`** on the ten decoders named in FR-023: every rejection condition, the
    count limits marked as deliberate deviations (FR-022, fact 10), and what each does not
    validate (cell-id vector ids; point-vector unit length).
  - **The FR-020 rule** in `decode_string_vector`'s rustdoc, citing upstream commit
    `95b1e15`.
  - **Check:** `make doc` builds with warnings denied. List every addition in "`# Panics`
    documentation added" of the evaluation.
- [X] T040 [US1] Run `make verify-decoding` alone. Fill the crash columns of `verify/decoding/INVENTORY.md`: harness or family, bound, uncovered part, and the tests that exercise it. Record the leg's wall time and CPU time in "Measurements". If it exceeds 10 minutes, move the slowest members to the weekly tier (T063) and re-run.

**Checkpoint**: US1 is complete and independently shippable. Every in-scope operation has a
crash-freedom result within its bound, and every panic is documented.

---

## Phase 5: User Story 2 — One proven contract for every codec (Priority: P2)

**Goal**: The decoder contract holds for every in-scope codec, and the codec-specific
catalogue properties are proven against the reference description (FR-004, FR-006).

**Independent Test**: `make verify-decoding` reports every `cd_{fw,uv,cv,sv,pv,fx}` harness
`SUCCESSFUL`. Every `proven*` K-track row of `PROPERTIES.md` names a harness in the output
and a killed mutant in `MUTANTS.md`.

- [X] T041 [US2] Implement `Codec` for the ten codecs of data-model §3 in `verify/src/codec.rs`, with `encodable` exactly as in the table. `same` is bit-exact for floats (`to_bits`).
  - `U32Vec`, `U64Vec`, `IdVec`, `PtVec`: `len ≤ 50 000 000`.
  - `StrVec`: total ≤ 500 000 000.
  - `CellIdC`, `CellC`: valid (`spec::is_valid`).
  - `PointC`: every bit pattern.
  - `CapC`: `ValidCap` (`|x²+y²+z² − 1| ≤ 5ε` in the library's operation order, and
    `length² ≤ 4`).
  - `RectC`: `ValidRect`.
- [X] T042 [P] [US2] Write the model round trips `p_<area>_mrt__n0 … __n2` (one per format) in the area modules. Each calls `check_model_round_trip`, the third leg of part 1 (research R4).
- [X] T043 [P] [US2] Write P-FW-01 … P-FW-04 in `verify/src/cd_fw.rs` (data-model §4.1). These are whole-domain and symbolic in `len ≤ 8`. They discharge T030's crash obligations; remove T030's harnesses that the new ones subsume, and update the inventory's crash column.
- [X] T044 [P] [US2] Write P-UV-01 … P-UV-06 and P-VAR-01 in `verify/src/cd_uv.rs`.
  - **P-UV-01/02, part 1** (composition): `check_encoder` `__n0 … __n2`, plus
    `check_agreement` `__b0 … __b11`, plus T042.
  - **P-UV-03/04:** the agreement families. They replace T033's crash families, which
    are deleted. Update the crash column of the inventory.
  - **P-UV-05:** width minimality, on the encoder's output for `__n1`/`__n2`.
  - **P-UV-06:** the encoded length.
  - **P-VAR-01:** checked inside the agreement (the header is read as `ref_varint`, with
    `DEV_VARINT_TENTH` while D-003 is open). If it fails on the tenth byte, apply the
    known-defect rule and list the harness in `verify/known-defects.txt`.
- [X] T045 [P] [US2] Write P-CV-01 … P-CV-03 in `verify/src/cd_cv.rs`. P-CV-02's agreement family replaces T034's crash family. P-CV-03 checks the encoder against `enc_cellid_vector` (upstream's search) for `__n0 … __n2`.
- [X] T046 [P] [US2] Write P-SV-01 … P-SV-03 in `verify/src/cd_sv.rs`, at T029's bound. If the string-vector decoder is a tool limit, P-SV-01/02 are `Not proven (tool limit)` on the K track: write only P-SV-03 (encoder side) and the model round trip, and say so in PROPERTIES.md.
- [X] T047 [P] [US2] Write P-PV-01 … P-PV-03 in `verify/src/cd_pv.rs`. P-PV-01 is the FAST encoder agreement plus the `__t0` agreement. P-PV-02 is `__t0`/`__tx` agreement. P-PV-03 is the `__t1` crash family from T036, renamed, unless it is excluded.
- [X] T048 [P] [US2] Write P-FX-01 … P-FX-10 in `verify/src/cd_fx.rs`.
  - **P-FX-01 … 05:** a direct round trip (encode a valid value, decode, `same`). It fits
    for fixed sizes (R0).
  - **P-FX-06 … 10:** `check_agreement` over `__b0 … __b<K+1>`. These replace T031's
    decoder families.
  - **`Cap` and `Rect`:** agreement asserts on `cap_valid` / `ValidRect`. If the `CapC`
    agreement exceeds the budget, use R18's fallback: state acceptance relative to
    `Cap::is_valid`, add it to the trusted base, and note it in INVENTORY.md.
  - Apply the upstream findings rule to `Cell`.
- [X] T049 [US2] Write one mutant per proven K-track property in `verify/mutants/` (data-model §7). At least these:
  - `M-201`: width off by one (`byte_length_u64` uses `div_ceil(7)`);
  - `M-202`: the `u64` decoder divides by 4;
  - `M-203`: the header uses `len` instead of `len − 1` (quickstart §4);
  - `M-204`: the `u64` width mask is `& 3`;
  - `M-205`: the extended shift code is `28 + b` instead of `29 + b`;
  - `M-206`: the base shift is `56 − 8·len` instead of `64 − 8·len`;
  - `M-207`: the string decoder does not advance `start` past an invalid entry;
  - `M-208`: the point-vector tag mask is 2 bits;
  - `M-209`: the `Rect` version check is `>=`;
  - `M-210`: `Cap` validation is skipped;
  - `M-211`: `CellId` validation is skipped;
  - `M-212`: the `Point` coordinates y and z are swapped;
  - `M-213`: `Cell` decodes `id.parent()`;
  - `M-214`: `decode_uint_with_length` reads `len − 1` bytes;
  - `M-215`: `write_uvarint` uses `> 0x80`;
  - `M-216`: `read_uvarint` shifts by 8.

  Add more, from `M-217`, until every proven K-track property has one. Each header names
  its `Targets` and `Expect-Fail` harnesses.
- [X] T050 [US2] Run `make verify-mutants M="M-200 M-201 …"` (every `M-2NN` below `M-250`) alone. Every mutant must be `killed`; fix a harness that lets one survive. Regenerate `verify/MUTANTS.md` with `run-mutants.sh`'s table. Record the campaign's wall time.
- [X] T051 [US2] Set every K-track status in `verify/decoding/PROPERTIES.md` (`proven`, `proven-within-bounds` with the uncovered part, or `not-proven (reason)`), with its harnesses and mutant. Check SC-003: at least 90% of the entries whose operations are all in scope are proven by at least one track (P-PAR-01 excluded). Check SC-004.

**Checkpoint**: US2 is complete. The contract holds per codec within bounds, every proven
property has a killed mutant, and the crash proofs are shared where the domains coincide.

---

## Phase 6: User Story 3 — Unbounded proofs for the variable-length formats (Priority: P3)

**Goal**: Round trip, exact consumption, sound acceptance and crash-freedom are proven for
every length, for the varint, both integer vectors, the cell-id vector and the string
vector. Each is linked to the library by bounded agreement, tested agreement and the drift
guard (FR-010, FR-011, SC-005).

**Independent Test**:

- `make verify-deductive` reports 0 errors for all five modules, and both tests pass.
- `make verify-deductive MUTANTS=1` kills every `V-NNN`.
- Adding `let _ = 0;` to `decode_uint_vector_u64` makes `cargo test --test drift` fail and
  name the function (quickstart §3).

Follow contracts/deductive-conventions.md: restate statement by statement, use no escape
hatches, and list every difference in `mirrors.toml`. Run Verus only through
`run-verus.sh`.

- [X] T052 [P] [US3] Move the probe into `verify/deductive/src/varint.rs` and `verify/deductive/src/fixed_width.rs`, splitting `specs/003-formal-verify-decoders/probe/uint_vector.rs`.
  - **`varint.rs`:** `varint`, `write_uvarint`, `dec`, `read_uvarint`, `lemma_dec_varint`
    and `lemma_round_trip`. The restated functions' names mirror the library.
  - **`fixed_width.rs`:** `byte_at`, `le`, `le_value`, `fits` and their lemmas; `put_le`
    and `get_le`, renamed to mirror `encode_uint_with_length` and
    `decode_uint_with_length`; `byte_length` as `byte_length_u64`, plus a new
    `byte_length_u32`. Keep `put_le`/`get_le` as `pub(crate)` aliases only if the vector
    module needs them.
  - Add the varint's tenth-byte relation (contracts/deductive-conventions.md, "Theorems") as
    a lemma.
  - **Check:** `make verify-deductive` passes, with obligations at least the probe's 51 for
    these parts.
- [X] T053 [US3] Complete `verify/deductive/src/uint_vector.rs`: the probe's `u64` encoder and decoder, plus the `u32` restatement (header `n·4 | (len − 1)`, mask `& 3`, width ≤ 4, `as u32` truncation), each with its `lemma_*_full_round_trip`. Needs T052.
- [X] T054 [US3] Write `verify/deductive/src/cellid_vector.rs` (FR-010).
  - **Restate** `encode_s2cell_id_vector`, `encode_base_shift`, `decode_s2cell_id_vector`
    and `decode_base_shift` statement by statement. That includes the `v_or`/`v_and`/
    `v_min`/`v_max` fold, the 8-candidate base search, the shift adjustment and the
    `wrapping_add`/`wrapping_shl` reconstruction.
  - **Spec:** `idvec(ids, base, base_len, shift)` = the header bytes, the base's top
    `base_len` bytes, and `uvec(deltas)`.
  - **Lemma:** decoding `prefix + idvec + rest` returns the ids and the exact length, for
    every id sequence up to the count limit.
  - **Helper lemmas** for `(base + (delta << shift)) mod 2^64 == id`, given the encoder's
    choice of base and shift, are expected to need `by (bit_vector)` goals.
  - Measure the time; it is the largest of the five (plan "Spec criteria at risk").
- [X] T055 [US3] Write `verify/deductive/src/string_vector.rs` (FR-010, FR-020).
  - **Restate** `encode_string_vector` and `decode_string_vector`. The data read is a slice
    copy of `total_len` bytes or `None`: the permitted reader difference for
    `take` + `read_to_end`.
  - **Spec:** v0.14's lazy rule as a spec decoder.
  - **Lemmas:** the round trip for every string sequence whose total is at most the limit,
    and sound acceptance, meaning the exec `ensures` equals the spec decoder for every input.
- [X] T056 [US3] Fill `verify/deductive/mirrors.toml` (one `[[mirror]]` per restated function, with its `differences`) and implement `verify/deductive/tests/drift.rs` (FR-011; research R12).
  - **Locating a function:** parse the library file with `syn::parse_file`. Find the `fn`
    by name at module level, or inside the `impl` block whose header matches the entry
    (`impl S2Decode for X`).
  - **Fingerprint:** strip `#[doc]` attributes, take `quote::ToTokens::to_token_stream`
    as a string, and hash it with FNV-1a 64, written inline (no dependency).
  - **On a mismatch:** fail with "`<library>` changed; review `<restated>` against it, then
    run `make verify-deductive-bless`". With `BLESS=1`, rewrite the fingerprints and print
    each change.
  - **Check:** quickstart §3.
- [X] T057 [US3] Implement tested agreement in `verify/deductive/tests/agree.rs` (FR-011; contracts/deductive-conventions.md).
  - For each restated codec, use quickcheck to generate:
    - byte strings up to 4 KiB, decoded by the library on `&mut &b[..]` and by the
      restated code on `(&b, 0)`, comparing acceptance, values and bytes consumed;
    - value vectors up to 10,000 elements (strings up to 1 KiB), encoded by both and
      compared byte for byte.
  - Also run every row of `core/tests/data/cpp_coding_vectors.csv` through both.
- [X] T058 [US3] Write the bounded agreement harnesses `verify/src/cd_agree.rs` (FR-011; research R12). Add `s2rst-deductive = { path = "deductive" }` to `verify/Cargo.toml`.
  - **Decoders:** `a_<codec>_dec__b0 … __b<L>` at each codec's R5/T029 bound.
  - **Encoders:** `a_<codec>_enc__n0 … __n2`.
  - Compare element-wise over the concrete maximum (fact 4), with unwind ≥ 9 for restated
    element loops (fact 5).
  - **String vector:** if its decoder is a tool limit, write no decoder harness. Record in
    `verify/TRUSTED_BASE.md` that its link is test-only (FR-011's fallback).
  - **Check:** `make verify-decoding` includes them, all `SUCCESSFUL`.
- [X] T059 [US3] Write the deductive mutants (FR-009; contracts/deductive-conventions.md).
  - **`V-NNN`:** `verify/deductive/mutants/V-001 …`, one or more per theorem, as one-defect
    diffs of the restated files. Examples: a varint continuation threshold `>= 0x7f`, a
    header width `len`, a `+ 28` shift code, a string `start` not advanced.
  - **`MUTANTS=1` in `run-verus.sh`:** apply each patch in a temporary copy of the crate,
    and expect Verus to fail. Print `killed` or `SURVIVED`.
  - **`M-25N`:** `verify/mutants/M-250 …`, one library-side divergence per mirrored codec.
    `Expect-Fail` names the `cd_agree` harness. The drift guard must also fail; check that
    by hand once per mutant and record it in the patch header's `Defect` line.
  - **Check:** `make verify-deductive MUTANTS=1` kills every `V-NNN`, and
    `make verify-mutants M="M-250 …"` kills every `M-25N`.
- [X] T060 [US3] Run `make verify-deductive` alone, and record each module's obligations, time and peak memory in "Kani vs Verus on shared properties" of the evaluation. Set every V-track status in PROPERTIES.md, naming its lemma. Check SC-005: all four FR-010 properties for all five codecs, and a deductive job under 10 minutes.

**Checkpoint**: US3 is complete. Every-length results exist for the five variable-length
codecs, with three links to the library.

---

## Phase 7: User Story 4 — Regression protection for future changes (Priority: P3)

**Goal**: Both proof tracks run in CI for relevant changes, and stay out of the default gate
(FR-015, FR-012; contracts/verify-commands.md).

**Independent Test**: a change that breaks a proven property fails the `verify (decoding)`
leg or the `deductive` job, naming the property. A change elsewhere does not trigger them.

- [X] T061 [US4] Extend `.github/workflows/verify.yml` (contracts/verify-commands.md, "CI").
  - **Paths:** add the in-scope files and their dependencies to both `pull_request` and
    `push` path lists:
    - `core/src/s2/encoded_{uint_vector,s2cell_id_vector,string_vector,s2point_vector}.rs`;
    - `core/src/s2/{cap,rect,point}.rs`, `core/src/s1/{chord_angle,interval}.rs` and
      `core/src/r1/interval.rs`;
    - `core/tests/data/cpp_coding_vectors.csv`.
  - **Matrix:** `suite: [cellid, union, decoding]`. Update the header comment.
- [X] T062 [US4] Add the `deductive` job to `.github/workflows/verify.yml`.
  - Same triggers, plus the weekly schedule.
  - Cache key `verus-${VERUS_VERSION}`. On a cache miss, download the release archive for
    tag `release/0.2026.09.20.aef82ed` from the Verus GitHub releases, and unpack it to
    `~/.verus/`. Check the archive name against the release page when writing this task.
  - `rustup toolchain install 1.98.1 --profile minimal`.
  - Run `make verify-deductive VERUS=$HOME/.verus/verus-x86-linux/verus`.
  - 20-minute limit; upload `run-verus` logs on failure.
  - The weekly schedule also runs `MUTANTS=1`.
- [X] T063 [US4] If T040 or T029 put members in the weekly tier, add the `decoding` part to `make verify-slow` in `Makefile` (`F=decoding` → `--features slow --harness cd_`) and to the `slow` job's matrix in `.github/workflows/verify.yml`. Set that file's `mutants` job time limit from T050's wall time plus 50%.
- [ ] T064 [US4] Prove the CI path works end to end, if a GitHub remote is available.
  - Open a draft PR with `M-203` applied: `verify (decoding)` fails and names P-UV.
    Revert it, and the leg passes. Record the run links in `verify/decoding/EVALUATION.md`.
  - The repo's remotes may be local clones only (as in 002's T046). If so, record that,
    leave the task open, and tell the user.
  - **2026-10-07:** GitHub remotes exist (`origin`, `github`, `priv`), but this needs a push
    and a draft PR, which the user has not approved. Left open; the workstation's timings of
    the three decoding legs (7.1, 7.5 and 8.2 minutes at 3 jobs) are in EVALUATION.md.

**Checkpoint**: CI runs both tracks on relevant changes. `make test` is unchanged.

---

## Phase 8: User Story 5 — Evaluation of the methods (Priority: P3)

**Goal**: A self-contained evaluation, including the mutation measurement and the Kani-vs-
Verus comparison (FR-017, FR-018, SC-012, SC-013).

**Independent Test**: a maintainer who did not do the work can answer "continue?", "with
which method or methods?", "is the deductive track worth its cost?" and "what next?" from
`verify/decoding/EVALUATION.md` alone.

- [X] T065 [US5] Extend `verify/run-mutation-measure.sh` for the **after** phase (FR-018; research R14).
  - Export every `missed` or `timeout` mutant's diff (`cargo mutants --list --diff`, or
    `mutants.out/diff/`) as a patch in `verify/target/mutation-after/` with 001's headers.
    `Expect-Fail` is `cd_` (any).
  - Run each through `run-mutants.sh`. A failing proof gives `killed by <harness>`.
  - For a survivor:
    - write a temporary Kani harness asserting that the mutated function still satisfies
      its catalogue properties; if it verifies, the result is `equivalent (<harness>)`;
    - otherwise `undetected`, with a reason.
- [X] T066 [US5] Run `make verify-mutation-measure PHASE=after` alone, on the final tree. It uses the same files, plus `--test cpp_coding_diff`. Write the **after** table and the before/after summary in `verify/decoding/MUTATION.md` (SC-012).
- [X] T067 [US5] Write `verify/decoding/EVALUATION.md` in full (FR-017; data-model §9).
  - **Summary.**
  - **Effort log**, by phase.
  - **Exclusions and tool limits**, with measurements.
  - **Bounds reached.**
  - **Upstream findings** (U-NNN).
  - **`# Panics` documentation added.**
  - **Defects found:** D-003 and anything new.
  - **Measurements:** the leg time, the deductive job, the FR-019 benchmark.
  - **Comparison with 001 and 002:** a table like 002's.
  - **Kani vs Verus**, per shared property: Kani's bound, time and memory; Verus's
    obligations, time, memory, and lines of specification and proof against executable
    code; drift risk.
  - **Mutation measurement summary.**
  - **The other branch's findings:** the `s1::Interval::length` seam, `ChordAngle`
    accepting every negative value, and `EdgeVectorShape::set_dimension`, each re-checked
    on the final tree with its status.
  - **Limitations.**
  - **Recommendations:**
    - go/no-go for each track, including deductive proof for `CellUnion::normalize`;
    - a ranked follow-up list weighing the declined methods: type-level invariants,
      float value types, `Shape`/`Region` laws, certified error bounds;
    - the out-of-scope decoders.
  - **Notes from implementation.**

**Checkpoint**: the evaluation answers SC-013's four questions.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [X] T068 [P] Add a "Decoding layer (spec 003)" section to `verify/TRUSTED_BASE.md` (FR-016; research R17).
  - Reliance on 001: P-VAL-03, and `Cell::from` as executed.
  - The standard-library pieces executed: `Read for &[u8]`, `Write for Vec<u8>`, `take`,
    `read_to_end`.
  - vstd's `Vec` and slice specifications.
  - Verus, Z3 4.16.0 and Rust 1.98.1.
  - The restated copies and their links, including any test-only link from T058.
  - The float operations inside `cap_valid` and `ValidRect`, and R18's fallback if used.
  - Every bound, with its uncovered part.
- [X] T069 [P] Update `verify/README.md`: three suites, the new targets, the `decoding` feature, the `cd_*` modules, `decoding/` and the new facts 1–5. Add `verify/deductive/README.md`: purpose, running, the restatement rules, blessing the drift guard, and the mutants.
- [X] T070 [P] Lint and check headers.
  - `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`, in `verify/` and
    in `verify/deductive/`.
  - `grep -rn unsafe verify/src verify/tests verify/deductive` shows only
    `domain::with_text`.
  - Every new `.rs`, `.sh`, `.toml` and `.cc` file has an SPDX header (002 T051's `find`
    command, extended to `verify/deductive`, `fuzz/fuzz_targets`, the new `core/` files and
    `tools/refgen-cpp/generate_coding.cc`).
  - `make test` passes.
- [X] T071 [P] Add a short "Formal verification" paragraph to `CONTRIBUTING.md`, so it does not contradict the constitution (Governance). It names `make verify-decoding`, `make verify-deductive` and the drift-guard rule: a change to a mirrored function needs a review of its restated copy and `make verify-deductive-bless`.
- [X] T072 Run `specs/003-formal-verify-decoders/quickstart.md` end to end (§1–§9), one verification job at a time. Fix any step that does not match reality, in quickstart.md or in the code.
- [X] T073 Cross-check SC-001 … SC-014 and record the results in "Acceptance" at the end of `verify/decoding/EVALUATION.md`.
  - **SC-001:** each bound meets the amended criterion.
  - **SC-002:** at least 20 of 24 in scope.
  - **SC-003 and SC-004:** from T051 and T060.
  - **SC-005:** from T060.
  - **SC-006:** from T040 and T063.
  - **SC-007:** `git diff master --stat -- core python wasm` shows only Phase 3's commits
    and T039, and no change to any published `Cargo.toml`.
  - **SC-008:** from T015, T018 and T022–T025.
  - **SC-009:** from T021 and T024.
  - **SC-010:** from T039.
  - **SC-011:** from T026 and T021.
  - **SC-012:** from T066.
  - **SC-013:** from T067.
  - **SC-014:** every `U-NNN` has a Phase 10 task and an evaluation row.

---

## Phase 10: Follow-ups (FR-025, FR-014) — recorded, not implemented in this feature

**Purpose**: Each task here records undefined or wrong upstream behaviour, or a library
defect deferred by the user. Resolve each in a **separate change** after this feature.
`/speckit-implement` must not execute these tasks. It only appends new ones, following the
rules above, and confirms or dismisses the candidates in T074 and T075. The next free task
ID is T078.

- [ ] T074 U-004 (candidate; confirm in T006/T031/T048): C++ `S2Cell::Decode` builds a cell from an unvalidated id (research R16).
  - **Upstream:** `s2cell.cc:326–332`, with `S2CellId::Decode` (`s2cell_id.cc:261–265`)
    validating nothing.
  - **Inputs:** 8 zero bytes; an id with face 6 or 7.
  - **Observed (expected):** `S2Cell(S2CellId(0))` calls `level()`, which is U-001's
    `ABSL_ASSUME(id_ != 0)` undefined behaviour, reachable from untrusted bytes. Faces 6–7
    index the face tables out of bounds.
  - **Rust:** the `Cell` decoder rejects invalid ids (FR-022).
  - **Decision needed:** whether to report it upstream. Dismiss this task if T006's
    generator shows C++ rejecting these inputs.
  - **Confirmed (2026-10-06):** at `v0.14.0-40`, `S2CellId::Decode` (`s2cell_id.cc:261–265`)
    accepts any 8 bytes and `S2Cell::Decode` (`s2cell.cc:326–332`) constructs `S2Cell(id)`,
    whose constructor calls `id.level()` (`s2cell.cc:69`); `level()` is
    `ABSL_ASSUME(id_ != 0)` (`s2cell_id.h:599`). So 8 zero bytes reach undefined behaviour.
    The generator leaves these inputs out for that reason (`generate_coding.cc:25`). An id
    with face 6 or 7 decodes; the undefined behaviour comes when the cell is used:
    `GetUVWAxis` indexes `kFaceUVWAxes[face]` out of bounds (`s2coords.h:481`), while
    `FaceUVtoXYZ` treats the face as 5 (its `default`, upstream TODO b/465912778).
- [ ] T075 U-005 (candidate; confirm in T030): `GetUintWithLength<T>` with `length > sizeof(T)` is guarded only by `ABSL_DCHECK` (`encoded_uint_vector.h:159`). In release builds it reads beyond the value. The library panics instead, documented in T039. **Decision needed:** whether to report it upstream.
  - **Confirmed, corrected (2026-10-06):** the precondition is checked in debug builds only,
    but a release build does not read past the value: the `length & sizeof(T)` branch loads
    exactly one `T`, the others at most `length mod sizeof(T)` bytes. It returns a silently
    wrong value instead (a length of 16 with `uint64_t` gives 0). Upstream's own decoders
    cannot pass such a length (the widths come from 3-bit fields). A minor finding: wrong
    results on a violated precondition, not undefined behaviour.
- [ ] T077 U-006 (found 2026-10-07, writing `UPSTREAM.md`): C++ `S2Cap::Decode` does not validate the cap in release builds, and aborts in debug builds.
  - **Upstream:** `s2cap.cc:340–353`. It reads four doubles, sets `radius_` through
    `S1ChordAngle::FromLength2` (`s1chord_angle.h:365–367`, `std::min(4.0, length2)`), and
    checks `is_valid()` only under `FLAGS_s2debug`, with `ABSL_CHECK`. The flag defaults to
    `S2_DEBUG_MODE` (`s2debug.cc:23`), which is true without `NDEBUG`
    (`base/log_severity.h:20–24`). `S2LatLngRect::Decode` returns `false` instead
    (`s2latlng_rect.cc:386–390`).
  - **Inputs:** a center with a NaN coordinate; a center of length 13 (`3, 4, 12`); a NaN
    `length2`.
  - **Observed:** in the Release-built generator, all three decode (`CAP_DECODE` rows marked
    `deviation:reject`), the NaN `length2` as 4.0, a full cap. In a debug build the same bytes
    would fail the `ABSL_CHECK` and abort (read from the code, not run).
  - **Rust:** rejected with `InvalidData`, a documented deviation (FR-022; D-004 for the NaN
    length).
  - **Decision needed:** whether to report it upstream, together with U-004.
- [X] T076 D-003 fix, only if the user chose "defer" in T028. **Not needed:** the user chose "fix now"; fixed in `a0b7487`.
  - Reject a varint tenth byte above 1 in `read_uvarint` (`core/src/s2/encoding.rs`) and
    `reconstruct_uvarint` (`core/src/s2/encoded_s2point_vector.rs`), as upstream
    `Parse64WithLimit` does.
  - Un-ignore T028's test and T021's tenth-byte rows.
  - Re-run the out-of-scope decoders' robustness tests.
  - Remove the `known-defects.txt` entry.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: no dependencies. T004 (the before measurement) must run on the
  unmodified library, before any Phase 3 commit.
- **Phase 2 (Foundational)**: needs Phase 1. T006–T007 need T005. T008 precedes
  T009–T012. T010 needs T007.
- **Phase 3**: needs Phase 2. Order inside:
  T014 → T015 → T016 → T017, then T018/T019 [P]. T020 is independent of T015–T017. T021
  needs T016 and T020. T022–T027 are [P] after T016. T028 can run any time after T020
  creates `encoding_tests.rs`.
- **Phase 4 (US1)**: needs Phases 2 and 3. T029 first, then T030–T037 [P], then T038, T039
  and T040.
- **Phase 5 (US2)**: needs Phase 4's families (it replaces them) and T041.
- **Phase 6 (US3)**: T052–T057 need only Phase 1's T002 and Phase 3's fixed library. T058
  needs T029's bounds and T041. T059 and T060 come last.
- **Phase 7 (US4)**: needs the targets it calls: US1 for the decoding leg, US3 for the
  deductive job.
- **Phase 8 (US5)**: needs US1–US4.
- **Phase 9 (Polish)**: needs everything above.

### User Story Dependencies

- **US1 (P1)**: independent after Phase 3. It is the MVP.
- **US2 (P2)**: builds on US1's families, and replaces many of them with agreement
  families.
- **US3 (P3)**: the deductive proofs (T052–T057) can proceed in parallel with US1/US2.
  Only the bounded agreement (T058) needs US1's bounds.
- **US4 (P3)**: needs the suites it runs.
- **US5 (P3)**: needs everything it evaluates.

### Parallel Opportunities

- T001 and T002; T009, T011 and T012 (after T008).
- Phase 3's T018, T019 and T022–T027.
- US1's T030–T037: different harness groups in one file. Write each group as its own
  section, or in a scratch file merged at the end.
- US2's T042–T048: different modules.
- US3's T052–T055: different modules, but they share Verus runs, so run Verus one module at
  a time while editing.
- Polish's T068–T071.

## Parallel Example: User Story 1

```text
# After T029 has fixed the bounds, write these groups side by side:
T030 fixed-width crash harnesses          (cd_crash.rs, section "fixed width")
T031 fixed-size decoder families          (cd_crash.rs, section "fixed-size decoders")
T033 integer vector decoder families      (cd_crash.rs, section "integer vectors")
T037 vector encoder families              (cd_crash.rs, section "encoders")
# Then run them one family at a time: make verify-harness H=cd_crash::<member>
```

## Parallel Example: User Story 2 and User Story 3

```text
# Different crates and files, so they can proceed together:
T044 cd_uv.rs agreement families          (verify/, Kani)
T052 varint.rs + fixed_width.rs from the probe   (verify/deductive/, Verus)
T054 cellid_vector.rs                     (verify/deductive/, Verus)
# But never run a Kani job and a Verus job at the same time.
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1, then Phase 2.
2. Phase 3: the FR-019 fix with its regression test, R6, the differential and property
   tests, and the fuzz targets. This alone fixes K2, a constitution violation, and is worth
   shipping.
3. Phase 4: crash-freedom proofs and the documentation commit.
4. **Stop and validate:** quickstart §1 (crash part), §5, §6 and §7.

### Incremental Delivery

1. Setup + Foundational + Phase 3: the decoders are fixed, tested and fuzzed.
2. US1: crash-freedom proven within bounds (MVP).
3. US2: one contract per codec, with mutants.
4. US3: every-length proofs, linked to the library.
5. US4: CI.
6. US5 + Polish: measurement, evaluation, acceptance.

## Notes

- Commit after each task or each logical group, with the attribution lines from the
  session's instructions. Behaviour changes (T016, T020, D-003 if approved) are their own
  commits.
- Never run Kani or Verus uncapped. Never run two verification jobs at once.
- Re-read the pinned upstream (`/home/t/sources/s2geometry`) before stating any upstream
  behaviour (fact 9).
- Ask the user before:
  - fixing D-003 (T028);
  - lowering a bound below the amended SC-001;
  - letting SC-002 drop below 20 of 24 (T029).
