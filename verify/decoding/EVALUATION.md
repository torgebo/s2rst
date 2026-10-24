# Evaluation: Formal Verification of the Decoding Layer (spec 003, FR-017)

Spec: `specs/003-formal-verify-decoders/`. Records: [INVENTORY.md](INVENTORY.md),
[PROPERTIES.md](PROPERTIES.md), [MUTATION.md](MUTATION.md), [../MUTANTS.md](../MUTANTS.md),
[../TRUSTED_BASE.md](../TRUSTED_BASE.md). The 001 and 002 evaluations are
[../EVALUATION.md](../EVALUATION.md) and [../cell_union/EVALUATION.md](../cell_union/EVALUATION.md).

## Summary

**What was verified.** The binary decoding layer that reads untrusted bytes: the integer,
cell-id, string and point vector codecs, the fixed-width helpers, and the `CellId`, `Point`,
`Cap`, `Rect` and `Cell` codecs (22 of 24 operations in scope). Two tracks:
- **Kani (bounded, on the library's own code):** no panic, overflow or undefined behaviour,
  and one contract per codec (round trip, exact consumption, sound acceptance, rejection)
  stated against a reference description of upstream's formats, which matches C++ on 4,714
  rows. Whole domains for the fixed-width helpers and the five fixed-size codecs (`Cap`'s full
length weekly); every input
  of up to 8 bytes per change and 11 weekly for the integer and cell-id vector decoders. Two
  tool limits: the string vector decoder, and the point vector decoder beyond 1 byte.
- **Verus (every length, on restated copies):** the five variable-length codecs, 132
  obligations in 17 s, tied to the library by bounded agreement, tested agreement and a drift
  guard that fails on any change to the 17 mirrored functions.

**What it found.** A crash (K2: a 5-byte input made the decoders reserve up to 1.2 GB and
abort), fixed by FR-019; a varint that dropped bits above 63 (D-003), fixed; a NaN cap length
that the library rejects and C++ accepts (D-004), kept as a documented deviation; and three
upstream findings (U-004, undefined behaviour in C++ `S2Cell::Decode` from 8 zero bytes;
U-005, minor; U-006, C++ `S2Cap::Decode` accepting invalid caps, or aborting in debug
builds). The survey, the C++ differential tests and reading upstream found all of them, not
the proofs:
the proofs then confirmed the fixed behaviour on their domains. Among 24 mutants that survive
the tests, the proofs kill 2.

**Continue?** Yes, for this kind of code. The decoders are the library's attack surface, and
the work removed a real crash and now guarantees crash freedom and the contract within the
stated bounds, checked on every change. Do not expect bounded proofs to find the bugs: here,
as in 002, the differential tests did.

**With which methods?** All three, each where it is strong: the C++ differential tests and
fuzzing to find defects; Kani for the library's own code, the fixed-size codecs (whole domains,
float validity) and short inputs; Verus for the variable-length codecs at every length.

**Is the deductive track worth its cost?** Yes. About a day for five codecs, 17 s per run, and
2.8 lines of specification and proof per executable line. It is the only evidence beyond
11 bytes, and the only proof at all for the string vector decoder. Its running cost is review:
every change to a mirrored function stops CI until someone checks the restated copy and
blesses the fingerprint.

**What next?** In order: run the xlarge tier on a 32 GB machine (the cell-id encoder's K-track
properties wait on it); exercise CI end to end (T064); add the 2-byte test `80 80` that the
mutation measurement found missing; then the ranked follow-ups under "Recommendations",
starting with type-level invariants for the decoded types.

## Effort log

| Phase | Dates | Notes |
|-------|-------|-------|
| Specification and planning (R0 probe: 5 Kani runs, 2 Verus files, 1 agreement harness) | 2026-10-04 – 2026-10-05 | A reboot wiped `/tmp` mid-probe; runs 4–5 were repeated |
| Setup and foundations (T001–T013) | 2026-10-05 | The before mutation measurement (T004) ran on the unmodified library |
| The library fixes, tests and fuzz targets (T014–T028) | 2026-10-05 – 2026-10-06 | FR-019 (K2), R6, D-003 fixed; the C++ coding vectors (4,714 rows) |
| US3, the Verus track (T052–T057, T059 V) | 2026-10-05 – 2026-10-06 | 132 obligations; 10 V mutants killed |
| US1, crash proofs (T029–T040) | 2026-10-06 | Two tool limits measured; a spurious `io::Error` report worked around |
| xlarge tier (the user's request) | 2026-10-06 | `make verify-xlarge`, not run on this 30 GB workstation |
| US2, the contract per codec (T041–T051, T058) | 2026-10-06 | The reference description reworked for the checker; three tiers |
| US4, CI (T061–T063) | 2026-10-06 | T064 needs a push, left to the user |
| US5 and polish: mutation measurement, evaluation, quickstart (T064–T073) | 2026-10-06 – 2026-10-07 | The measurement took about 3.5 hours of tests and 2 of proofs; a machine shutdown interrupted it once |

## Exclusions and tool limits (FR-002)

- Row 13, the compact point-vector encoder: excluded by design (floating-point projections).
- **Row 11, the string vector decoder: tool limit** (T029). No length `K ≥ 3` fits even the
  weekly budget; only the empty input does. Every member from 1 byte up runs out of solver
  memory in under a minute, at 4 GiB and at 8 GiB (table below). The cost is the data read,
  `Read::take` + `read_to_end` over a symbolic `total_len`: CBMC unrolls its growth loop for
  every offset vector the earlier bytes could declare, reachable or not. Evidence instead:
  the Verus proof for every input (`string_vector::decode_string_vector`, T055), tested
  agreement with the library, which is its only link (FR-011's fallback;
  `verify/deductive/tests/agree.rs`), the C++ differential rows, the robustness tests and
  the fuzz target.
- **Row 14, the point vector decoder: both branches are tool limits from 2 bytes** (T029).
  The compact branch (`__t1`) fits at no length `K ≥ 2`. The uncompressed branch (`__t0`)
  fits at 1 byte only. `decode_s2point_vector` dispatches on the format tag inside one
  function, and with 2 or more bytes CBMC expands the compact branch in every member,
  including its embedded string vector and `read_to_end`. The format tag stays symbolic
  whether it is assumed or built into the byte. At 2 bytes that is 1.06 million symex steps
  and 12,286 verification conditions, and the solver runs out of memory at 4 GiB. What stays
  in scope: the empty input, every 1-byte input, and the unknown tags 2–7, which are
  rejected after the first byte. Evidence beyond that: the R13 property tests, the C++
  differential rows, the robustness tests and the fuzz target.
- Both tool limits have members in the xlarge tier (`cd_xlarge`, `make verify-xlarge`), for a
  machine with 32 GB of memory. They are not yet run.

## Bounds reached

| Codec | Per change (4 GiB, 5 min) | Weekly (8 GiB, 30 min) | Every length (Verus, restated copy) |
|-------|---------------------------|------------------------|-------------------------------------|
| fixed-width helpers | whole domain | | yes |
| integer vectors, decoders | inputs up to 8 bytes | up to 11 bytes; a two-byte header (18 bytes) | yes |
| integer vectors, encoders | up to 2 values; 16 zeros (header 128) | | yes |
| cell-id vector, decoder | inputs up to 4 bytes | up to 11 bytes | yes |
| cell-id vector, encoder | 0 ids (agreement); crash at 0 | crash at 1–2 ids; model round trip at 1 | yes (xlarge: agreement at 1–2) |
| string vector, decoder | none (tool limit) | none | yes |
| string vector, encoder | up to 2 strings of up to 2 bytes | | yes |
| point vector, decoder | inputs up to 1 byte; tags 2–7 every length | | no (not restated) |
| point vector, FAST encoder | up to 2 points | | no |
| `CellId`, `Point`, `Rect`, `Cell` codecs | whole domain | | no |
| `Cap` codec | whole domain up to 31 bytes; full-length crash | agreement and round trip at full length (R18) | no |

The `bounds` column of [INVENTORY.md](INVENTORY.md) gives each entry's uncovered part and
the tests that exercise it.

## Upstream findings (FR-025, SC-014)

| Id | Upstream | Inputs | Observed upstream | Rust | Follow-up |
|----|----------|--------|-------------------|------|-----------|
| U-004 (confirmed) | C++ `S2Cell::Decode` (`s2cell.cc:326–332`) with `S2CellId::Decode` (`s2cell_id.cc:261–265`) | 8 zero bytes; an id with face 6 or 7 | `S2Cell(S2CellId(0))` calls `level()` (U-001's `ABSL_ASSUME`, undefined behaviour from 8 bytes). An id with face 6 or 7 decodes; later uses index `kFaceUVWAxes[face]` out of bounds (`s2coords.h:481`) | rejected (FR-022) | tasks.md T074 |
| U-006 (found while writing `UPSTREAM.md`) | C++ `S2Cap::Decode` (`s2cap.cc:340–353`) | a NaN center coordinate; a non-unit center; a NaN `length2` | no validity check in release builds: all three decode (the `CAP_DECODE` rows), the NaN radius as a full cap. Under `FLAGS_s2debug`, on by default in debug builds, an `ABSL_CHECK` aborts instead. `S2LatLngRect::Decode` returns `false` | rejected (FR-022, D-004) | tasks.md T077 |
| U-005 (confirmed, minor) | C++ `GetUintWithLength<T>` (`encoded_uint_vector.h:159`) | `length > sizeof(T)` | guarded by `ABSL_DCHECK` only; a release build returns a silently wrong value (a length of 16 gives 0). It reads at most `sizeof(T)` bytes, so no undefined behaviour, and upstream's decoders never pass such a length | panics (documented by T039) | tasks.md T075 |

## `# Panics` documentation added (SC-010)

T039 (FR-020 – FR-022). Documentation only; no behaviour changed (SC-007).

| Item | Added |
|------|-------|
| `encode_uint_with_length` | `# Panics`: `length > 8` (upstream: debug-only `ABSL_DCHECK`, `encoded_uint_vector.h:145`) |
| `get_uint_with_length` | `# Panics`: `length > 8` or `data.len() < length` (upstream checks the length in debug builds only and cannot check the input's size) |
| `decode_uint_with_length` | `# Panics`: `length > 8`; `# Errors`: `UnexpectedEof` when fewer than `length` bytes are left |
| `decode_uint_vector_u32`, `_u64` | `# Errors`: truncation; a header varint whose tenth byte is above 1; the 50,000,000-element limit, as a deliberate deviation. Elements are not validated |
| `decode_s2cell_id_vector` | `# Errors`: truncation, an extended shift code above 56, the delta vector's errors and its limit. Identifiers are not validated, as upstream |
| `decode_string_vector` | FR-020's rule (out-of-order or out-of-range entries decode as empty, citing upstream `95b1e15`); `# Errors`: the offsets' errors, the 500,000,000-byte limit as a deliberate deviation, truncated data |
| `decode_s2point_vector` | `# Errors`: truncation, unknown format tags, the 50,000,000-point limit as a deliberate deviation, the cell-id format's level, block and range errors. Points are not validated, as upstream |
| `impl S2Decode for CellId` | `# Errors` and the deviation note: invalid identifiers are rejected (`s2cell_id.cc:261–265` accepts any 64 bits) |
| `impl S2Decode for Cap` | `# Errors` and the deviation note: invalid caps are rejected, including a NaN length (D-004; `s2cap.cc:340–353`) |
| `impl S2Decode for Point`, `Rect`, `Cell` | `# Errors`: every rejection condition. `Cell` notes the U-004 candidate |

The crash proofs assume only these documented preconditions: `len ≤ 8` for the three
helpers, and `len ≤ data.len()` for `get_uint_with_length`. No other panic was found.

## Defects found

| Id | Where | Inputs | Upstream | Library | Status |
|----|-------|--------|----------|---------|--------|
| K2 (survey) | the vector decoders | 5-byte inputs declaring a huge count | rejects without reserving (checks `avail()`) | reserved 0.2–1.2 GB up front, and aborted under an address-space limit | fixed by FR-019 (T016) |
| D-003 | `encoding::read_uvarint` and `encoded_s2point_vector::reconstruct_uvarint` | a ten-byte varint whose tenth byte is 2–127, e.g. `ff ff ff ff ff ff ff ff ff 02` | rejects ("too long to be a varint64"), on both parse paths, as the `VARINT_DECODE` rows show | accepted it and dropped the bits above 63 | **fixed** in `a0b7487` (the user chose to fix it in the feature, 2026-10-06). The in-scope decoders rejected such inputs anyway (every 10-byte varint there is a count above their limits); 19 out-of-scope call sites now reject them too. Regression tests: `read_uvarint_rejects_a_tenth_byte_above_one`, `reconstruct_uvarint_rejects_a_tenth_byte_above_one` |
| D-004 | `impl S2Decode for Cap`, through `ChordAngle::from_length2` | a cap whose `length2` is NaN | `S1ChordAngle::FromLength2` is `std::min(4.0, length2)`, so NaN decodes as a full cap | keeps NaN, so the cap is invalid and rejected | **kept, as a documented deviation** (the user's decision, 2026-10-06; FR-022, rustdoc in T039). C++'s own encoder never writes NaN, so only corrupt or hostile bytes contain it. The `CAP_DECODE` row is marked `deviation:reject`; test `cap_with_nan_length2_is_rejected` |

## Measurements

### FR-019 benchmark

`cargo bench -p s2rst --bench encoded_uint_vector` (iai-callgrind, instructions; no
`target-cpu=native`). Before: `core/` at `774b06c` (unchanged through `d900eb1`).

| Benchmark | Before FR-019 | After FR-019 | Change |
|-----------|---------------|--------------|--------|
| `encode_decode_u32_1024` | 83,961 | 81,603 | −2.8 % |
| `encode_decode_u64_1024` | 73,185 | 74,934 | +2.4 % |
| `encode_u32_1024` | 35,276 | 35,276 | 0 |
| `decode_u32_1024` | 83,961 | 81,603 | −2.8 % |
| `encode_decode_u32_65536` | 5,620,751 | 5,492,900 | −2.3 % |

After: `eb0fc84` (FR-019). Every change is within ±3 %. The u32 decode paths got slightly
cheaper (no up-front `with_capacity`), the u64 round trip slightly dearer (the growth
reallocations); none crosses T017's 5 % threshold, so no follow-up is needed
(Constitution V).

### Bounds measured first

T029, 2026-10-05 – 2026-10-06. These are the members that research R5 left open, run before
their families were written, each through `run-verify.sh --exact`:
- run 1 at 4 GiB and 5 minutes per harness, the per-change budget;
- run 2 with the failing members alone, at 8 GiB and 30 minutes, the weekly budget;
- runs 3–5 with the uncompressed point branch at smaller unwinds and lengths.

Unwind is the byte length plus one unless stated.

| Member | Per change (4 GiB, 5 min) | Weekly (8 GiB, 30 min) |
|--------|---------------------------|------------------------|
| `c_decode_rect__b33` | SUCCESSFUL, 3.2 s | |
| `c_decode_s2cell_id_vector__b11` | SUCCESSFUL, 78.6 s | |
| `c_decode_string_vector__b0` | SUCCESSFUL, 1.7 s | |
| `c_decode_string_vector__b1` | out of memory, 28.8 s | out of memory, 39.1 s |
| `c_decode_string_vector__b2` | out of memory, 34.8 s | out of memory, 46.6 s |
| `c_decode_string_vector__b3` | out of memory, 42.8 s | out of memory, 51.2 s |
| `c_decode_string_vector__b4` | out of memory, 52.6 s | not re-run (b3 failed) |
| `c_decode_s2point_vector__t0_b1` | SUCCESSFUL, 2.8 s | |
| `c_decode_s2point_vector__t0_b2` | out of memory (run 5, with the tag built into the byte) | |
| `c_decode_s2point_vector__t0_b11` | timeout (runs 3 and 5) | |
| `c_decode_s2point_vector__t0_b25` | timeout | timeout at 30 min, 6.0 GB resident |
| `c_decode_s2point_vector__t0_b25`, unwind 11 | timeout (run 3) | |
| `c_decode_s2point_vector__t1_b2` | out of memory | out of memory (CBMC status 6) |
| `c_decode_s2point_vector__t1_b11` | out of memory | not re-run (b2 failed) |
| `c_decode_s2point_vector__tx_b1` | SUCCESSFUL, 3.0 s | |

Every out-of-memory failure was "Solver ran out of memory during propositional reduction",
with no failed check ("0 of 1615 failed"). The formula is too large; no defect was found.
Unwind 11 is the smallest that can pass at 25 bytes: at most 10 varint bytes and 2 point
iterations. It did not help, which points at the formula's size rather than the unrolling.
`--verbose` on `t0_b2` showed why: the compact branch's loops and the string vector
decoder are unwound even though the tag is 0 (above, row 14).

**Outcomes under T029's rules:** the string vector decoder (row 11) is a tool limit. The point
vector decoder (row 14) stays in scope at `__b0`, `__t0_b1` and `__tx_b1`, with both branches
tool limits from 2 bytes. SC-002: 22 of 24 entries in scope (row 13 by design, row 11 a tool
limit), above the threshold of 20.

### A spurious report in `io::Error`'s destructor (T040)

The first run of the full leg failed 56 members: every truncated `Point` and `Cap` input,
`c_decode_point__b0 … __b23` and `c_decode_cap__b0 … __b31`. Each failed on one check only,
"misaligned pointer to reference cast", inside std's `io::error::repr_bitpacked::decode_repr`
(line 273, the `SimpleMessage` arm) while the harness dropped the error. Five probes, each a
0-byte input, isolate it:

| Probe | Result |
|-------|--------|
| `Point::decode`, error dropped | FAILED (the same check) |
| `Point::decode`, no cover, error dropped | FAILED |
| `Point::decode`, error forgotten (`mem::forget`) | SUCCESSFUL |
| `read_exact` alone, error dropped | SUCCESSFUL |
| `CellId::decode`, error dropped | SUCCESSFUL |

The error is the same static `SimpleMessage` in every case. Only the function that returns
it differs, and a real misalignment of one static cannot depend on that. So the report is
the checker's model, not the code. The harnesses now end with `wire_domain::discard`, which
forgets an error and drops a value normally, and std's destructor for `io::Error` is in the
trusted base. With it the probe and the affected members pass (`c_decode_cap__b31` 0.8 s,
`__b32` 4.0 s, `c_decode_point__b23` 0.5 s).

### US2: the contract harnesses, their cost and their tiers (T041–T048)

The first runs of the property modules failed or ran out of memory for reasons in the
harnesses and the reference description, not in the library. Each was fixed without changing
what is proven:

- **The reference description allocated like a naive program.** Its encoders built a
  temporary `Vec` per field and extended them by symbolic lengths, and the string model
  copied entries with `to_vec` of symbolic slices: R0's memory wall. The encoders now append
  whole fields to one buffer whose capacity comes from concrete counts (`put_varint`,
  `put_fixed`, `put_uint_vector`, `put_string_vector`), and string entries are built byte
  by byte. `ref_fixed` is straight-line code: as a loop it set every agreement harness's
  unwind to 9, and so unrolled the library's own loops that far on 2-byte inputs. The C++
  vector tests (`wire_spec_vs_cpp.rs`) pass unchanged.
- **The model's round trip decoded a `Vec` of symbolic length**, so none of the decoder's
  loops had a concrete bound. It now decodes `enc(v)` copied into an `E`-byte buffer whose
  other bytes are arbitrary, which is R4's statement, `ref_decode(enc(v) + rest)`.
- **Harness mistakes:** `==` on arrays (it unrolls `memcmp`); `spec::valid` needs unwind 32
  (it searches 31 levels); a comparison bound smaller than a string's length; a capacity from
  a decoded length in `cd_agree` (`ids_of`).
- **`Cap`** uses research R18's fallback (acceptance relative to `Cap::is_valid`, in the
  trusted base). Even so, its agreement at 32–33 bytes and its model round trip fit only the
  weekly budget: 1,172 s, 1,440 s and 1,553 s at 8 GiB. The direct round trip timed out, so
  P-FX-03 is proven by composition, like the vector codecs (`p_fx_03_enc`, `p_fx_08__b33`,
  `p_fx_03_mrt`). Two crash members keep the full-length `Cap` decoder in the per-change run
  (`c_decode_cap__b32`, `__b33`).
- **The Kani driver ran out of address space** in one run of about 150 harnesses with 4
  jobs ("memory allocation of 2097088 bytes failed", then "No exit code?"). It runs under the
  same `ulimit -v` as the harnesses. Large runs now use 3 jobs.
- **The disk filled up** (`verify/target/kani` reached 32 GB over builds of three feature
  sets). Deleting the cache freed it; the memory note on this was updated.

**Tiers.** Per change (4 GiB, 5 minutes per harness), the integer vector agreements reach
8 bytes, the cell-id vector agreement 4 bytes, the `cd_agree` integer decoders 8 bytes and its
cell-id decoder 3 bytes. Their longer members, up to 11 bytes, run weekly (8 GiB, 30 minutes;
the longest, `cd_agree::a_cv_dec__b11`, takes 1,284 s). Also weekly: the two-byte header
decode (`p_uv_03__h2_b18`), the `Cap` members above, the cell-id model round trip at one id,
the string model round trip at two strings, and the restated `u64`/`u32` encoders' agreement
at one value. The restated encoders append byte by byte, as their Verus proofs need, which is
heavier than the library's writes.

Out of memory even at 8 GiB, and so in the xlarge tier (`cd_xlarge`, not yet run): the
cell-id vector's encoder agreement at one and two ids (`p_cv_03__n1`, `__n2`) and its model
round trip at two ids (`p_cv_mrt__n2`), and the restated encoders' agreement at two values
(`a_uv64_enc__n2`, `a_uv32_enc__n2`, `a_sv_enc__n2`). So the K track's cell-id encoder
property P-CV-03 holds for zero ids only, and P-CV-01's round trip by composition likewise;
the V track proves both for every sequence. The first two-byte varint header the encoder
writes (128) is checked on one concrete vector of sixteen zeros (`p_uv_01_enc__n16_zeros`,
18 s): with a symbolic value among the sixteen, the member ran out of memory at 8 GiB.

**Before tiering**, the per-change leg took 2,715 s of wall time with 3 jobs (268 harnesses,
4,431 s of harness time), well over T040's 10 minutes. The tiers above moved about 1,800 s of
harness time to the weekly run. **The final per-change suite**: 241 harnesses, all
successful, every cover satisfied, in 1,264 s of wall time with 3 jobs (2,191 s of harness
time, 53 min of CPU). To meet SC-006, CI splits it into three parallel legs, `make verify-decoding
P=vectors|links|codecs`. Kani spends about 5 s setting up each harness beyond its
"Verification Time", which dominates the 118 small fixed-size members, so the parts are
balanced on harness time plus that set-up. Measured on the workstation at 3 jobs, build and
sanity tests included (T072): **vectors 7.1 min** (56 harnesses), **links 7.5 min** (43),
**codecs 8.2 min** (142), all successful with every cover satisfied. A hosted 4-vCPU runner is
slower than this workstation, so the 10-minute limit is likely tight for the codecs leg; only
a CI run (T064) can tell.

**The whole suite with its weekly members** (`make verify-slow F=decoding`, one harness at a
time at 8 GiB and 60 minutes, on 2026-10-09): 278 harnesses (241 per change, 37 weekly), all
successful, every cover satisfied (33 harnesses have none), in 3.8 hours of wall time (213
minutes of harness time). The slowest were `cd_fx::p_fx_03_mrt` 1,550 s,
`cd_agree::a_cv_dec__b11` 1,208 s and `cd_cv::p_cv_02__b11` 1,042 s, all within the
30-minute weekly budget.

### The mutant campaign (T050, T059)

`make verify-mutants M="M-200 … M-256"`, 2026-10-06, alone on the workstation: **34 killed,
0 survived, 0 errors**, in 3,222 s (54 minutes). Each mutant ran in a throwaway worktree with a
fresh build. `M-200 … M-228` are one per proven K-track property; `M-250 … M-256` change a
mirrored library function, and both their `cd_agree` harness and the drift guard fail on them
(the drift guard checked by hand, each patch's `Defect` line). `M-222` (the cell-id encoder's
base search) has its only target in the xlarge tier and was skipped (`VERIFY_XLARGE=1` runs
it). The ten Verus mutants `V-001 … V-010` are all killed, each on a proof obligation.

## Comparison with 001 and 002

| | 001 `CellId` | 002 `CellUnion` | 003 decoding layer |
|---|---|---|---|
| Inventory in scope | 48 of 64 (75 %) | 27 of 42 (64 %) | 22 of 24 (92 %) |
| Crash proofs over the whole domain | 45 of 48 | 2 of 27; 25 within size bounds | 13 of 22; 9 within byte or element bounds |
| Properties proven (entries whose operations are in scope) | 42 of 43 | 23 of 23 | 28 of 29 (K and V tracks together) |
| Every-length results | none | none | 5 codecs, 132 Verus obligations |
| Harnesses | 196 per change, plus the weekly tier | 120 per change, plus 45 weekly | 241 per change, 37 weekly, 16 xlarge |
| Per-change time (workstation) | 28 min CPU | 21 min (4 CPUs) | 21 min wall, 53 min CPU (3 jobs) |
| Mutants | 38 | 25, all killed | 35 `M-2NN`: 34 killed, 1 xlarge-only not run; 10 `V-NNN`, all killed |
| New defects | D-001 | D-002 (fixed) | D-003 (fixed), D-004 (kept as a deviation) |
| Known defects fixed in the feature | 2 | 3 | 1 (K2, FR-019) |
| Upstream findings | 2 (U-001, U-002) | 1 (U-003) | 3 (U-004, U-005, U-006) |
| `# Panics` sections added | 22 | 23 | 3, plus `# Errors` on 10 decoders |
| Elapsed time, specification to completion | about 4 days | about 3 days | about 3 days |

What changed in kind:
- **Two tracks.** For the five variable-length codecs, the bounded checker stops at 11 bytes
  (and at 1 byte for the string and point vectors), while Verus proves the restated copies
  for every length in seconds. The price is the link: the restated code is a second copy,
  kept honest by bounded agreement, tested agreement and the drift guard.
- **The reference description is code the checker must afford.** It had to be written
  for Kani: no capacity from data, no temporary per field, no loop where straight-line code
  will do. Its first form was correct (it matched C++ on every row) and still ran out of
  memory.
- **The defects came from the C++ vectors, not the proofs.** D-003 and D-004 were found by
  the differential rows written in Phase 3. The proofs found no defect in the library; they
  confirmed the documented behaviour on their domains, and K2's fix.

## Kani vs Verus on shared properties

T060, `make verify-deductive` alone (Verus 0.2026.09.20.aef82ed, Rust 1.98.1, Z3 4.16.0), on
this workstation:

| Module | Obligations | Time | Peak memory |
|--------|-------------|------|-------------|
| `varint` | 19 | 2.1 s | 346 MiB |
| `fixed_width` | 16 | 1.8 s | 346 MiB |
| `uint_vector` | 36 | 2.7 s | 352 MiB |
| `cellid_vector` | 46 | 8.1 s | 358 MiB |
| `string_vector` | 15 | 2.2 s | 352 MiB |
| **total** | **132** | **17.0 s** | |

The whole target, with the link tests (tested agreement, the drift guard), took 43 s.
SC-005's ten-minute limit for the deductive job holds with a wide margin.

Per codec. Kani: the bound and the per-change harness time of the final run (properties plus
bounded agreement; every harness runs under the 4 GiB ceiling, and the runner records no peak
memory). Verus: obligations, time and peak memory from the table above. Lines: non-blank,
non-comment lines of each restated module, executable against specification and proof
(`requires`/`ensures`/invariants, assertions, `proof` blocks, spec and proof functions).

| Codec | Kani bound (per change; weekly) | Kani time per change | Verus | Executable : spec+proof lines | Drift risk |
|-------|---------------------------------|----------------------|-------|-------------------------------|------------|
| varint | inside the integer vector inputs up to 8 bytes; 11, and a two-byte header, weekly | within `cd_uv` | 19 obligations, 2.1 s, 346 MiB, every `u64` | 27 : 113 | low: both functions mirrored; `M-250` fails `cd_agree` and the guard |
| fixed-width helpers | whole domain | 38 s (21 harnesses) + 19 s agreement | 16, 1.8 s, 346 MiB | 38 : 95 | low: `M-251` |
| integer vectors | inputs up to 8 bytes; 11 weekly; encoders at 2 values | 782 s (35) + 467 s agreement (20) | 36, 2.7 s, 352 MiB, every length | 77 : 324 | low: `M-252`, `M-253`; every callee is mirrored |
| cell-id vector | decoder inputs up to 4 bytes, 11 weekly; encoder at 0 ids (1–2 xlarge) | 91 s (7) + 176 s agreement (5) | 46, 8.1 s, 358 MiB, every sequence | 176 : 312 | low: `M-254`, `M-255` |
| string vector | decoder: none (tool limit); encoder at 2 strings | 193 s (5) + 74 s agreement (2) | 15, 2.2 s, 352 MiB, every length | 54 : 194 | medium: the decoder's only link is the tested agreement (no bounded agreement reaches it); the encoder's divergence `M-256` fails `cd_agree` and the guard |

In all, 372 executable lines carry 1,038 lines of specification and proof (2.8 to 1), and
Verus checks them in 17 s, against about 31 minutes of Kani time for bounded results on the
same codecs. The drift guard fails on any token change to the 17 mirrored functions, so the
risk is not a silent divergence but review cost: a change that keeps the behaviour still needs
a reviewer to check the copy and bless the fingerprint.

What the two tracks prove about the same codecs:
- **Integer vectors:** Kani proves agreement with the reference description on every input
  of up to 11 bytes, at a cost of up to 100 s per member at 4 GiB. Verus proves the restated
  decoder equal to the spec decoder on every input of every length, and the round trip for
  every vector up to the count limit, in 2.7 s for both widths.
- **The cell-id vector:** Kani's agreement reaches 11 bytes only weekly (843 s at 11 bytes),
  and its model round trip fits 8 GiB at one id but not at two. Verus proves the round trip
  for every sequence, including upstream's base and shift search, in 8 s.
- **The string vector:** out of Kani's reach from 1 byte (T029). Verus proves the decoder,
  FR-020's rule and the round trip for every length, in 2.2 s.
- **What only Kani gives:** the library's own code. Verus proves restated copies, linked to
  the library by `cd_agree` (bounded), `tests/agree.rs` (tested) and the drift guard. Kani
  also covers the fixed-size codecs and their float validity, which the Verus track does not
  model.


## Mutation measurement

T004 (before) and T066 (after), cargo-mutants 27.1.0 over the five in-scope files; details,
every survivor and its reason in [MUTATION.md](MUTATION.md).

| Phase | Mutants | Caught by the tests | Killed by the proofs | Undetected |
|-------|---------|---------------------|----------------------|------------|
| before | 473 (3 unviable) | 442 (94.0 %) | n/a | 28 |
| after | 479 (3 unviable) | 452 (95.0 %) | 2 | 22 |

- The feature's tests catch eight earlier survivors (the cell-id encoder's base search, the
  compact point decoder's arithmetic, `reconstruct_uvarint`).
- FR-019 made four `> → ==` mutants of the count limits invisible: with growth on demand, the
  limit no longer guards a reservation, so passing it changes only the error kind.
- The proofs kill two of the 24 survivors of the tests: the `u32` encoder's width from an XOR
  (`cd_uv::p_uv_02_enc__n2`) and `write_uvarint`'s loop condition (`cd_uv::p_uv_01_enc__n0`).
- Of the 22 left, 9 are equivalent (argued by hand), 8 are at the count and size limits (inputs
  of 50 MB to 1.2 GB), 4 are in the compact point-vector branch (the P-PV-03 tool limit), and 1
  is beyond the point vector's 1-byte bound (`80 80`, a test worth adding).
- With the proofs, 454 of 476 viable mutants are detected (95.4 %); without the equivalent ones,
  454 of 467 (97.2 %).

So the proofs add little to this layer's mutation score: the C++ differential tests already
catch almost everything the tests can reach. What the proofs add is the guarantee within the
bounds, which a score does not measure.

## The other branch's findings

Re-checked natively on the final tree (2026-10-06), from
`claude/formal-methods-type-invariance-ept6u0`'s `docs/formal-methods-and-type-invariance.md`:

| Finding | Status | Bearing on the decoding layer |
|---------|--------|-------------------------------|
| `s1::Interval::length` returns `-1.0` for the valid, non-empty one-ulp interval `(π, nextafter(−π, 0))`, as upstream's `GetLength` does | still present | none: the `Rect` decoder checks `is_valid`, which does not use `length` |
| `ChordAngle::is_valid` accepts every negative `f64` | still present: `from_length2(-5.0)` is valid | the `Cap` decoder accepts any negative `length2` as an empty cap, bit for bit, as upstream (`std::min(4.0, x)` keeps it); P-FX-03 and P-FX-08 prove that behaviour |
| `EdgeVectorShape::set_dimension` does not check the edges | still present: `Dimension::Point` with a non-degenerate edge is accepted | none (not a decoded type) |

None was changed in this feature, which is limited to the decoding layer (SC-007).

## Limitations

- **Small byte bounds on the K track.** The vector decoders are proven on every input of up
  to 11 bytes (from 9 or 5 bytes only weekly), the point vector decoder up to 1 byte, the
  string vector decoder not at all. Longer inputs rest on the Verus proofs of the restated
  copies (for every codec but the point vector), the tested agreement, the C++ vectors, the
  property tests and the fuzz targets.
- **The restated copies are a second implementation.** Verus proves them, not the library.
  The drift guard catches any change to a mirrored function, but it cannot tell whether a
  changed function still matches its copy; a reviewer must.
- **The reference description is trusted** to state upstream's formats. The C++ vectors
  check it on 4,714 rows; it is not proven against upstream.
- **R18's fallback** puts `Cap::is_valid` in the trusted base, and the `Cap` agreement at its
  full length runs only weekly.
- **The xlarge tier has not been run.** Its 16 members are written and build, but this
  workstation has 30 GB of memory. Until they run, the cell-id encoder's K-track properties
  (P-CV-01, P-CV-03) hold for zero ids only.
- **The per-change suite takes 21 minutes** on the workstation. CI runs it as three parallel
  legs of 7.1–8.2 minutes each here; on a slower CI runner the codecs leg may pass 10 minutes
  (SC-006), which T064 would show.
- **T064 is open:** the CI path has not been exercised end to end, because that needs a
  push.

## Recommendations

- **Keep both tracks for codecs.** For variable-length formats the bounded checker cannot
  reach realistic sizes; Verus can, cheaply, once the copy is written (the five codecs took
  about a day, the proofs run in 17 s). Kani remains the only track on the library's own
  code, and the one that covers floats.
- **Go for deductive proof of `CellUnion::normalize`** only with the same three links, and
  only if its loop invariants can be stated against 002's model: the payoff (every size,
  where 002 reached three cells) is the same kind as here.
- **Ranked follow-ups**, from the declined methods:
  1. type-level invariants for the decoded types (a private `ChordAngle` field with a
     validating constructor would close the other branch's finding and make the `Cap`
     decoder's validity a type property);
  2. `Shape`/`Region` law harnesses (the other branch's `set_dimension` finding shows the gap);
  3. float value types with certified error bounds, for `Cap` and `Rect` validity;
  4. the out-of-scope decoders (`Polygon`, `Loop`, `Polyline`, the shape index), starting
     with the ones built on the proven vectors.
- **Run the xlarge tier** on a 32 GB machine and record its results; promote what passes.

## Notes from implementation

- Most failures were in the harnesses, not the library: array `==` (it unrolls `memcmp`),
  capacities sized from decoded data, a too-small comparison bound, a loop in the reference
  that raised every unwind. The checker reports these as out-of-memory or unwinding
  failures, never as "this harness is badly written".
- Two Kani artefacts cost hours: the spurious `io::Error` destructor report, and the driver
  itself running out of address space under the per-harness `ulimit`. The disk filled twice
  (`verify/target/kani` at 32 GB).
- A machine shutdown stopped the mutation measurement at mutant 412 of 479 and wiped `/tmp`.
  `run-mutation-measure.sh` now resumes (`MUTANTS_ITERATE=1`, cargo-mutants' `--iterate`,
  merging the interrupted run's lists), so only the 89 untested mutants ran again.
- Verus was the easier tool for the codecs: the proofs needed bit-vector assertions and
  explicit sequence lemmas, but each module verified in seconds, and its errors pointed at
  the failing obligation.

## Acceptance (SC-001 … SC-014)

T073, on the final tree (2026-10-06).

| Criterion | Result | Evidence |
|-----------|--------|----------|
| SC-001 | **met, with one gap on the K track** | Whole domains for the fixed-width helpers and the five fixed-size codecs; every input up to 11 bytes for the integer and cell-id vector decoders (the longer members weekly); the string and point vector decoders as measured tool limits (the clarification of 2026-10-05); the vector encoders up to 2 values. The round trip by composition holds for at least one element for the integer vectors, but for the cell-id vector only at zero ids: its encoder agreement at one id runs out of memory at 8 GiB (xlarge, not run). The V track proves it for every sequence |
| SC-002 | met | 22 of 24 in scope (row 13 by design, row 11 a tool limit) |
| SC-003 | met | 28 of 29 entries proven by at least one track (96.6 %); P-PV-03 is the tool limit |
| SC-004 | met, except the xlarge member | 34 of 35 `M-2NN` mutants killed; `M-222` targets an xlarge harness and was not run. All 10 `V-NNN` killed. Every crash and agreement harness's covers are satisfied (the final leg and the weekly runs) |
| SC-005 | met locally; CI unverified | Five codecs proven for every size (132 obligations); the agreement checks pass; `M-250 … M-256` fail both `cd_agree` and the drift guard. The deductive target takes 43 s here; the CI job is unexercised (T064) |
| SC-006 | **not verified on CI** | The per-change suite runs as three CI legs, measured on the workstation at 3 jobs: 7.1, 7.5 and 8.2 minutes. The hosted runner is slower, so the codecs leg may exceed 10 minutes; no CI run yet (T064). Every weekly member finishes within 30 minutes (the longest 1,553 s). The `CellId` and `CellUnion` legs are unchanged |
| SC-007 | met | From spec 003's base, `core/`, `python/` and `wasm/` change only in Phase 3's commits (FR-019, R6, the tests, D-003's fix, which the user approved) and T039; no published `Cargo.toml` changes. `make test` passes |
| SC-008 | met | `verify/tests/reservation.rs` (at most 2 MiB for each K2 input); `decoder_robustness::reservation_under_address_space_limit` (an error, not an abort); the 1,000,000-element property tests (T022–T025); the regression tests failed before FR-019 (T015) |
| SC-009 | met | FR-020's rule documented (T039), proven for every length on the V track (`string_vector::decode_string_vector`), and checked by the C++ rows, the robustness tests and the fuzz target. The K track cannot reach the decoder (a tool limit) |
| SC-010 | met | "`# Panics` documentation added" lists every addition |
| SC-011 | met | Ten fuzz targets in the nightly matrix (three added: `decode_cell_id`, `decode_point`, `decode_rect`); the C++ vectors cover every codec; `cpp_coding_diff` passes |
| SC-012 | met | Every one of the 479 mutants is classified (MUTATION.md): 452 caught, 3 unviable, and of the 24 survivors 2 killed by the proofs and 22 undetected, each with a written reason. The before and after counts are in "Mutation measurement" |
| SC-013 | met | "Summary" answers "continue?" (yes, for code that reads untrusted bytes), "with which methods?" (differential tests and fuzzing to find defects, Kani for the library's code and the fixed-size codecs, Verus for the variable-length codecs), "is the deductive track worth its cost?" (yes: a day's work and 17 s per run, for the only every-length evidence) and "what next?" (the xlarge tier, T064, the missing `80 80` test, then the ranked follow-ups) |
| SC-014 | met | U-004 and U-005 have tasks T074 and T075 (created during planning, confirmed in T067), and U-006 has T077 (found after the proofs, while writing `UPSTREAM.md`); all three have rows under "Upstream findings"; D-003 and D-004 are under "Defects found" |
