# Mutation Measurement: the decoding layer (spec 003, FR-018, SC-012)

What the proofs add over the test suite, measured with tool-generated mutants (research
R14; data-model §8). `cargo-mutants` 27.1.0 runs over the five in-scope source files,
excluding the compact point-vector encoder (FR-002) and `encoding.rs`'s geometry and
`CellUnion` codecs (FR-003). The exact command, filters and test selection are in
[`../run-mutation-measure.sh`](../run-mutation-measure.sh). The raw outcome lists are in
`mutants-before/` and `mutants-after/`.

Each mutant is classified:
- **by the tests** (cargo-mutants): `caught`, `missed`, `timeout` or `unviable`;
- **for `missed` and `timeout`, by the proofs** (T065): `killed by <harness>` when a
  per-change proof over the mutant's file fails, otherwise `undetected`, with a reason. Where
  the reason says "equivalent", the argument is written out by hand (no proof checks it);
  the others name the bound the mutant lies beyond.

## Before (the unmodified library, `core/` at `774b06c`)

Test selection: `--lib --test decoder_robustness --test property_tests`. Run on
2026-10-05/06: 164 minutes with 2 jobs, alongside other work on the workstation.

| Phase | Caught | Missed | Timeout | Unviable | Total |
|-------|--------|--------|---------|----------|-------|
| before | 442 | 27 | 1 | 3 | 473 |

| Mutant (line numbers of `774b06c`) | Tests | Proofs | Reason |
|------------------------------------|-------|--------|--------|
| `core/src/s2/encoded_s2cell_id_vector.rs:57:13: replace > with >= in encode_s2cell_id_vector` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2cell_id_vector.rs:72:88: replace + with * in encode_s2cell_id_vector` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2cell_id_vector.rs:73:24: replace < with <= in encode_s2cell_id_vector` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2cell_id_vector.rs:108:47: replace \| with ^ in encode_base_shift` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:173:14: replace < with <= in reconstruct_uvarint` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:176:15: replace += with *= in reconstruct_uvarint` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:177:18: replace >= with < in reconstruct_uvarint` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:190:57: replace \| with ^ in encode_fast` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:205:14: replace > with >= in decode_uncompressed` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:377:54: replace + with * in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:377:35: replace * with + in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:377:49: replace - with + in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:377:49: replace - with / in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:399:38: replace > with >= in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:407:35: replace > with == in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:407:35: replace > with >= in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:407:20: replace + with * in decode_cell_ids_raw` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:534:26: replace \| with ^ in value_to_point` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:535:26: replace \| with ^ in value_to_point` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_s2point_vector.rs:536:43: replace \| with ^ in value_to_point` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_string_vector.rs:61:18: replace > with >= in decode_string_vector` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_uint_vector.rs:68:59: replace \| with ^ in encode_uint_vector_u32` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_uint_vector.rs:70:41: replace \| with ^ in encode_uint_vector_u32` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_uint_vector.rs:86:41: replace \| with ^ in encode_uint_vector_u64` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_uint_vector.rs:105:14: replace > with >= in decode_uint_vector_u32` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoded_uint_vector.rs:127:14: replace > with >= in decode_uint_vector_u64` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoding.rs:49:31: replace << with >>` | missed | n/a (no proofs yet) | |
| `core/src/s2/encoding.rs:112:13: replace >= with < in write_uvarint` | timeout | n/a (no proofs yet) | |

## After (the final tree)

Test selection: `--lib --test decoder_robustness --test property_tests --test cpp_coding_diff`.
Run on 2026-10-06/07 with 2 jobs: 160 minutes for the first 412 mutants, until the machine shut down,
then 34 minutes for the other 89 with `MUTANTS_ITERATE=1` (cargo-mutants' `--iterate`; the
interrupted run's caught and unviable mutants carry forward). Then each survivor of the tests
ran through the per-change proofs that cover its file (133 minutes for the 24;
`run-mutation-measure.sh`, `group`; 5 minutes per harness, 3 jobs).

| Phase | Caught | Missed | Timeout | Unviable | Total |
|-------|--------|--------|---------|----------|-------|
| after | 452 | 23 | 1 | 3 | 479 |

Of the 24 survivors of the tests, the proofs kill 2 and 22 remain undetected.

| Mutant (line numbers of the final tree) | Tests | Proofs | Reason |
|-----------------------------------------|-------|--------|--------|
| `core/src/s2/encoded_s2cell_id_vector.rs:108:47: replace \| with ^ in encode_base_shift` | missed | undetected | equivalent: `shift_code << 3` has its low three bits clear and `base_len <= 7`. The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_s2cell_id_vector.rs:72:88: replace + with * in encode_s2cell_id_vector` | missed | undetected | equivalent: it subtracts the same `n` from every candidate base length's cost, so the chosen base length is unchanged (`e_bytes` is only compared). The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_s2point_vector.rs:220:14: replace < with <= in reconstruct_uvarint` | missed | undetected | beyond the bound: a continuation byte 0x80 then ends the header, so `[0x80, 0x80]` decodes as zero points instead of failing at the end of input. A 2-byte input with tag 0; the point vector decoder is proven on inputs up to 1 byte (T029), and no test or C++ row has that header. A test gap: add the row. |
| `core/src/s2/encoded_s2point_vector.rs:237:57: replace \| with ^ in encode_fast` | missed | undetected | equivalent: `Format::Uncompressed` is 0. |
| `core/src/s2/encoded_s2point_vector.rs:252:14: replace > with == in decode_uncompressed` | missed | undetected | beyond the bound: observable only on an input holding more than 50,000,000 points (1.2 GB). On every shorter input both versions reject, with a different error kind (`UnexpectedEof` for `InvalidData`); the contract compares acceptance and values, not error kinds. |
| `core/src/s2/encoded_s2point_vector.rs:252:14: replace > with >= in decode_uncompressed` | missed | undetected | beyond the bound: observable only on an input of exactly 50,000,000 points (1.2 GB); every shorter input with that count is rejected either way. |
| `core/src/s2/encoded_s2point_vector.rs:445:38: replace > with >= in decode_cell_ids_raw` | missed | undetected | compact branch, a tool limit from 2 bytes (P-PV-03, T029): with no offset bytes and 16 delta nibbles it rejects a block upstream accepts. |
| `core/src/s2/encoded_s2point_vector.rs:453:20: replace + with * in decode_cell_ids_raw` | missed | undetected | compact branch, a tool limit (P-PV-03): with `ptr == 1` the bound check is one byte short, so an offset one byte past the block panics on the slice (fuzzing can reach it). |
| `core/src/s2/encoded_s2point_vector.rs:453:35: replace > with == in decode_cell_ids_raw` | missed | undetected | compact branch, a tool limit (P-PV-03): an offset past the block's end then panics on the slice; reachable by the `decode_s2point_vector` fuzz target, which is not in the test selection. |
| `core/src/s2/encoded_s2point_vector.rs:453:35: replace > with >= in decode_cell_ids_raw` | missed | undetected | compact branch, a tool limit (P-PV-03): rejects an offset that ends exactly at the block's end. |
| `core/src/s2/encoded_s2point_vector.rs:580:26: replace \| with ^ in value_to_point` | missed | undetected | equivalent: the low bit of `sj << 1` is 0. |
| `core/src/s2/encoded_s2point_vector.rs:581:26: replace \| with ^ in value_to_point` | missed | undetected | equivalent: the low bit of `tj << 1` is 0. |
| `core/src/s2/encoded_s2point_vector.rs:582:43: replace \| with ^ in value_to_point` | missed | undetected | equivalent: `sj` is a `u32`, so `(sj << shift) >> 30` is at most 3 and never shares bit 2 with the second term. |
| `core/src/s2/encoded_string_vector.rs:70:18: replace > with == in decode_string_vector` | missed | undetected | beyond the bound: observable only on an input holding more than 500,000,000 data bytes; every shorter input is rejected either way (a different error kind). The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_string_vector.rs:70:18: replace > with >= in decode_string_vector` | missed | undetected | beyond the bound: observable only on an input of exactly 500,000,000 data bytes. The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_uint_vector.rs:104:41: replace \| with ^ in encode_uint_vector_u64` | missed | undetected | equivalent: `v.len() * 8` has its low three bits clear and `len - 1 <= 7`. The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_uint_vector.rs:131:14: replace > with == in decode_uint_vector_u32` | missed | undetected | beyond the bound: observable only on an input holding more than 50,000,000 elements (50 MB at width 1); every shorter input is rejected either way (a different error kind). The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_uint_vector.rs:131:14: replace > with >= in decode_uint_vector_u32` | missed | undetected | beyond the bound: observable only on an input of exactly 50,000,000 elements (50 MB). The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_uint_vector.rs:158:14: replace > with == in decode_uint_vector_u64` | missed | undetected | beyond the bound: as the `u32` decoder's, at 50,000,000 elements. The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_uint_vector.rs:158:14: replace > with >= in decode_uint_vector_u64` | missed | undetected | beyond the bound: observable only on an input of exactly 50,000,000 elements (50 MB). The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoded_uint_vector.rs:86:59: replace \| with ^ in encode_uint_vector_u32` | missed | killed by `cd_uv::p_uv_02_enc__n2` | the tests miss it: random vectors almost never have an XOR one byte shorter than their OR; two values with the same high byte do. |
| `core/src/s2/encoded_uint_vector.rs:88:41: replace \| with ^ in encode_uint_vector_u32` | missed | undetected | equivalent: `v.len() * 4` has its low two bits clear and `len - 1 <= 3`. The drift guard flags it (a mirrored function changed), but no proof or test fails. |
| `core/src/s2/encoding.rs:49:31: replace << with >>` | missed | undetected | equivalent in behaviour and outside the scope: `MAX_PREALLOC` becomes 0, and only the out-of-scope `CellUnion` and geometry decoders use it, as a capacity hint (they grow on demand). It is measured because the scope filter matches function names, and a constant is in none. |
| `core/src/s2/encoding.rs:112:13: replace >= with < in write_uvarint` | timeout | killed by `cd_uv::p_uv_01_enc__n0` | the tests time out: every value below 128 makes the loop endless. The proofs fail on the unwinding bound. |

## Summary

| Phase | Mutants | Caught by the tests | Survivors of the tests | Killed by the proofs | Undetected |
|-------|---------|---------------------|------------------------|----------------------|------------|
| before (`774b06c`) | 473 (3 unviable) | 442 (94.0 % of 470) | 28 | n/a | 28 |
| after (final tree) | 479 (3 unviable) | 452 (95.0 % of 476) | 24 | 2 | 22 |

The tree grew by six mutants (FR-019's growth paths, D-003's tenth-byte check).

**What the feature's tests added.** Eight survivors of the before phase are caught by the
tests written in Phase 3 (`cpp_coding_diff` and the sibling tests): the cell-id encoder's
base-search boundaries (2), the compact point decoder's arithmetic (4) and
`reconstruct_uvarint`'s loop (2, after D-003's fix).

**What FR-019 took away.** Four `> → ==` mutants of the count limits (`decode_uint_vector_u32`,
`_u64`, `decode_string_vector`, `decode_uncompressed`) survive only after the feature. Before
it, such a mutant let a huge declared count reach the up-front reservation, and the robustness
tests caught the abort. With growth on demand the limit is a second line of defence: past it,
a short input is rejected at its end anyway, with a different error kind.

**What the proofs added.** Two kills among the 24 survivors: the `u32` encoder's width
computed from an XOR (`cd_uv::p_uv_02_enc__n2`, two values with the same high byte), and
`write_uvarint`'s loop condition (`cd_uv::p_uv_01_enc__n0`; the tests only time out). The
22 left, each with its reason above:
- **9 equivalent**: `|` → `^` on disjoint bits (6), the cell-id encoder's cost offset that
  is the same for every candidate (1), and `MAX_PREALLOC`, a capacity hint used only by
  out-of-scope decoders (1, measured because the scope filter matches function names), plus
  `Format::Uncompressed`, which is 0 (1);
- **8 at the count and size limits**, observable only on inputs of 50 MB to 1.2 GB;
- **4 in the compact point-vector branch**, the P-PV-03 tool limit;
- **1 beyond the point vector's 1-byte bound**: `reconstruct_uvarint` accepting 0x80 as a
  final byte. It needs the 2-byte input `80 80`, which no test or C++ row has: a cheap test
  to add.

Counting the proofs, 454 of the 476 viable mutants are detected (95.4 %); leaving out the 9
equivalent ones, 454 of 467 (97.2 %). The 13 detectable survivors are all beyond a bound the
evaluation records: the limits, the compact branch, or the point vector's bound.
