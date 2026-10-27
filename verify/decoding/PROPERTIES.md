# Property Catalogue: the decoding layer (spec 003, FR-006)

In plain language and grouped by kind of guarantee, with the other two catalogues:
[`PROPERTIES_BY_KIND.md`](../PROPERTIES_BY_KIND.md).

The functional properties of the decoding layer, each stated against the reference
description of the wire formats, `verify/src/wire_spec.rs`. The description is written from
upstream C++ at `v0.14.0-40` and is checked against the C++ vectors by
`verify/tests/wire_spec_vs_cpp.rs` (P-PAR-01). Field definitions:
`specs/003-formal-verify-decoders/data-model.md` §4, which adds **Track** to 002's fields:
`K` (bounded checker), `V` (deductive, restated code, linked by FR-011), or both, with a
status per track.

**Status values**: `draft` | `proven` (the whole domain) | `proven-within-bounds` (a
byte-length, element-count or loop bound leaves part of the domain uncovered; `Bound` says
which, and which tests exercise it) | `proven-all-sizes-restated` (V track) |
`not-proven (<reason>)`.

**The contract, by composition** (research R4). Part 1, the round trip, is a codec's encoder
agreement, its decoder agreement and the model round trip together, so a round-trip entry
names three harnesses.

**SC-003 denominator**: every entry except P-PAR-01, which is a test (FR-006).

| ID | Statement (against `wire_spec`) | Upstream source (`v0.14.0-40`) | Domain | Track | Harness / lemma | Status K | Status V | Bound | Mutants |
|----|--------------------------------|--------------------------------|--------|-------|-----------------|----------|----------|-------|---------|
| P-FW-01 | `encode_uint_with_length(v, len)` writes exactly `len` bytes, `enc_fixed(v, len)` | `EncodeUintWithLength`, `encoded_uint_vector.h:139–153` | any `u64`, `Len` | K+V | `cd_fw::p_fw_01`; V: `fixed_width::encode_uint_with_length` | proven | proven-all-sizes-restated | whole domain (`len ≤ 8`) | M-217; V-003 |
| P-FW-02 | `get_uint_with_length(d, len)` is `ref_fixed(d, len)` for `d.len() ≥ len` | `GetUintWithLength`, `encoded_uint_vector.h:155–178` | `Bytes(8)`, `Len` | K+V | `cd_fw::p_fw_02__d0 … __d8`; V: `fixed_width::get_uint_with_length` | proven | proven-all-sizes-restated | whole domain | M-200, M-218 |
| P-FW-03 | `decode_uint_with_length` returns `ref_fixed` and consumes exactly `len` bytes, or `UnexpectedEof` on a shorter input | `DecodeUintWithLength`, `encoded_uint_vector.h:180–200` | `Bytes(L)`, `Len` | K+V | `cd_fw::p_fw_03__b0 … __b9`; V: `fixed_width::decode_uint_with_length` | proven | proven-all-sizes-restated | whole domain: at most 8 bytes are read, so `__b9` covers every longer input | M-214, M-251 |
| P-FW-04 | writing then reading returns `v` for every `len ≤ 8` and every `v` in `Fits(len)` | the three above | any `u64`, `Len` | K+V | `cd_fw::p_fw_04`; V: `fixed_width::lemma_le_round_trip` | proven | proven-all-sizes-restated | whole domain | M-214; V-003 |
| P-VAR-01 | every vector decoder reads its header as `lib_varint` does: upstream's `get_varint64`, plus D-003 while it is open | `Varint::Parse64WithLimit`, `varint.cc:229–262` | `Bytes(L)` | K | inside `cd_uv::p_uv_03__*`, `p_uv_04__*` (headers in every input up to 11 bytes; 9–11 weekly); `p_uv_03__h2_b18` (weekly; a two-byte header that decodes); `p_uv_01_enc__n16_zeros` (the header 128 written, on one concrete vector) | proven-within-bounds |  | inputs up to 11 bytes; the two-byte header on narrowed domains (18 bytes of width-1 elements; sixteen zeros). Longer headers: the V track (P-VAR-02), `encoding_tests.rs`, the `VARINT_DECODE` rows | M-215, M-216, M-250 |
| P-VAR-02 | varint round trip with exact consumption, for every `u64`, whatever surrounds it; the decoder terminates and never fails a check on any input | `put_varint64`, `get_varint64` | every input | V | V: `varint::lemma_round_trip`, `read_uvarint`'s ensures (= `dec`, the D-003 rule) |  | proven-all-sizes-restated | every `u64`, every input | V-001, V-002 |
| P-UV-01 | `U64Vec` round trip (contract part 1) | `EncodeUintVector<uint64>`, `EncodedUintVector<uint64>::Init`/`Decode` | `Ints64(N)` | K+V | `cd_uv::p_uv_01_enc__n0 … __n2`, `p_uv_03__*`, `p_uv_mrt_u64__n0 … __n2`; V: `uint_vector::lemma_uvec_full_round_trip` | proven-within-bounds | proven-all-sizes-restated | every `n ≤ 2`: the decoder agreement on inputs of at most 11 bytes (`n ≤ 1`, and `n = 2` with widths up to 5) and on every 18-byte input declaring two elements (`p_uv_03__n2_b18`, widths 6–8, added 2026-10-26; code review of v0.4.0..74b9bc1, finding 10); larger: V track, property tests | M-201, M-203, M-221; V-004 |
| P-UV-02 | `U32Vec` round trip | as P-UV-01 for `uint32` | `Ints32(N)` | K+V | `cd_uv::p_uv_02_enc__n0 … __n2`, `p_uv_04__*`, `p_uv_mrt_u32__n0 … __n2`; V: as P-UV-01 for `u32` | proven-within-bounds | proven-all-sizes-restated | `n ≤ 2`; larger: V track, property tests | M-220, M-252 |
| P-UV-03 | `U64Vec` agreement (contract parts 2–4): `decode_uint_vector_u64` gives `lib_uint_vector(·, 8)`'s answer and consumption | `EncodedUintVector<T>::Init`, `encoded_uint_vector.h:229–240` | `Bytes(L)` | K+V | `cd_uv::p_uv_03__b0 … __b8`, `p_uv_03__n2_b18`; weekly: `__b9 … __b11`, `p_uv_03__h2_b18`; V: `uint_vector::decode_uint_vector_u64`'s ensures | proven-within-bounds | proven-all-sizes-restated | inputs up to 11 bytes (9–11 weekly), and every 18-byte input whose header declares two elements; longer: V track, `decoder_robustness`, fuzzing | M-202, M-204, M-253; V-008 |
| P-UV-04 | `U32Vec` agreement | as P-UV-03 | `Bytes(L)` | K+V | `cd_uv::p_uv_04__b0 … __b8`; weekly: `__b9 … __b11`; V: `uint_vector::decode_uint_vector_u32`'s ensures | proven-within-bounds | proven-all-sizes-restated | inputs up to 11 bytes (9–11 weekly); longer: as P-UV-03 | M-219 |
| P-UV-05 | the element width is the minimum number of bytes that holds `1 | OR(v)` | `EncodeUintVector`, `encoded_uint_vector.h:213–216` | `Ints64(N)`, `Ints32(N)` | K+V | `cd_uv::p_uv_05_u64__n1`, `__n2`, `p_uv_05_u32__n1`, `__n2`; V: `fixed_width::byte_length_u64`/`_u32`'s minimality | proven-within-bounds | proven-all-sizes-restated | `n ≤ 2` | M-201; V-007 |
| P-UV-06 | the encoded length is `|varint(header)| + n · width` | `EncodeUintVector`, `encoded_uint_vector.h:220–225` | as P-UV-05 | K+V | `cd_uv::p_uv_05_*` (the same harnesses); V: the encoders' ensures (`uvec`) | proven-within-bounds | proven-all-sizes-restated | `n ≤ 2` | M-221; V-004 |
| P-CV-01 | `IdVec` round trip, for identifiers of any bits | `EncodeS2CellIdVector`, `EncodedS2CellIdVector::Decode` | `Ids(N)` | K+V | `cd_cv::p_cv_03__n0`, `p_cv_02__*`, `p_cv_mrt__n0`, `__n1` (weekly); xlarge: `cd_xlarge::p_cv_03__n1`, `__n2`, `p_cv_mrt__n2`; V: `cellid_vector::lemma_idvec_round_trip`, `encode_s2cell_id_vector`'s ensures | proven-within-bounds | proven-all-sizes-restated | `n = 0`: the encoder agreement at one id or more runs out of memory at 8 GiB and is in the xlarge tier, not yet run (the model's leg holds weekly at `n = 1`, the decoder agreement on every input up to 11 bytes); every `n`: the V track | M-205, M-206; V-005, V-009 |
| P-CV-02 | `IdVec` agreement, including the extended shift code and shifts above 56 | `EncodedS2CellIdVector::Init`, `encoded_s2cell_id_vector.cc:153–179` | `Bytes(L)` | K+V | `cd_cv::p_cv_02__b0 … __b4`; weekly: `__b5 … __b11`; V: `cellid_vector::decode_s2cell_id_vector`'s ensures | proven-within-bounds | proven-all-sizes-restated | inputs up to 11 bytes (5–11 weekly); longer: V track, fuzzing | M-205, M-206, M-254; V-005 |
| P-CV-03 | the encoder writes upstream's encoding: base length, shift and delta width as upstream's search chooses them | `EncodeS2CellIdVector`, `encoded_s2cell_id_vector.cc:62–151` | `Ids(N)` | K | `cd_cv::p_cv_03__n0`; xlarge: `cd_xlarge::p_cv_03__n1`, `__n2` | proven-within-bounds |  | `n = 0` (one or two ids run out of memory at 8 GiB; the xlarge members are not yet run). The cell-id encoder's crash proof reaches two ids weekly (`cd_crash`), and `deductive/tests/agree.rs` compares it with the restated encoder, whose round trip Verus proves | M-222 (xlarge), M-255 |
| P-SV-01 | `StrVec` round trip | `StringVectorEncoder::Encode`, `EncodedStringVector::Decode` | `Strings(N, M)` | K+V | model leg `cd_sv::p_sv_mrt__n0 … __n2` (n2 weekly) and P-SV-03; V: `string_vector::lemma_svec_round_trip` | not-proven (tool limit: the decoder, T029) | proven-all-sizes-restated | K: the decoder is out of reach from 1 byte | V-006, V-010 |
| P-SV-02 | `StrVec` agreement: an entry whose range is out of order or out of the data is empty, and decoding continues from its end (FR-020) | `EncodedStringVector::Init`/`Decode`, `encoded_string_vector.cc:57–84` | `Bytes(L)` | K+V | V: `string_vector::decode_string_vector`'s ensures (= `dec_svec`, FR-020) | not-proven (tool limit, T029) | proven-all-sizes-restated | K: the decoder is out of reach from 1 byte; tested agreement is the link (FR-011's fallback) | V-006 |
| P-SV-03 | the encoded length is the offsets' integer vector plus the total data | `StringVectorEncoder::Encode` | `Strings(N, M)` | K+V | `cd_sv::p_sv_03__n0 … __n2`; V: `string_vector::encode_string_vector`'s ensures | proven-within-bounds | proven-all-sizes-restated | `n ≤ 2`, strings of up to 2 bytes | M-223, M-256; V-010 |
| P-PV-01 | `PtVec` round trip in the FAST format, for every bit pattern | `EncodeS2PointVector(…, FAST)`, `InitUncompressedFormat` | `Points(N)` | K | `cd_pv::p_pv_01_enc__n0 … __n2`, `p_pv_02__b0`, `__b1`, `p_pv_02__n1_b26`, `__n2_b50`, `p_pv_mrt__n0 … __n2` | proven-within-bounds |  | the round trip by composition holds for `n ≤ 2`: the decoder's leg on every input whose header declares one or two points, so on their encodings (`p_pv_02__n1_b26`, `__n2_b50`, added 2026-10-26; code review of v0.4.0..74b9bc1, finding 10: until then `n = 0` only). On all inputs the decoder is a tool limit from 2 bytes (T029). `n ≥ 3`: property tests, `cpp_coding_diff` | M-224, M-257 |
| P-PV-02 | `PtVec` agreement on the uncompressed branch; tags 2–7 rejected | `EncodedS2PointVector::Init`, `encoded_s2point_vector.cc:121–258` | `Bytes(L)`, tags ≠ 1 | K | `cd_pv::p_pv_02__b0`, `__b1`, `p_pv_02__n1_b26`, `__n2_b50` | proven-within-bounds |  | inputs up to 1 byte, and every input whose header byte declares one or two uncompressed points (26 and 50 bytes, any point bits); tags 2–7 for every length (rejected after the first byte); longer: the xlarge members, property tests, fuzzing | M-208, M-257 |
| P-PV-03 | the compact branch returns a value or an error on every input up to its bound (crash only) | `InitCellIdsFormat` | `Bytes(L)`, tag 1 | K | — (the compact branch is a tool limit, T029; `cd_crash::c_decode_s2point_vector__b1` covers its 1-byte inputs) | not-proven (tool limit, T029) |  |  |  |
| P-FX-01 | `CellIdC` round trip | `S2CellId::Encode`/`Decode` | `ValidId` | K | `cd_fx::p_fx_01` | proven |  | whole domain (`ValidId`) | M-225 |
| P-FX-02 | `PointC` round trip, bit-exact for every bit pattern | three `putdouble`s | any bits | K | `cd_fx::p_fx_02` | proven |  | whole domain | M-212 |
| P-FX-03 | `CapC` round trip | `S2Cap::Encode`/`Decode` | `ValidCap` | K | by composition: `cd_fx::p_fx_03_enc`, `p_fx_08__b33` (weekly), `p_fx_03_mrt` (weekly); R18's fallback (`Cap::is_valid` trusted); the round trip under `Cap`'s own equality is spec 008's P-CPCOD-01 (`values/PROPERTIES.md` §10) | proven |  | whole domain (`ValidCap`, relative to `Cap::is_valid`) | M-226 |
| P-FX-04 | `RectC` round trip | `S2LatLngRect::Encode`/`Decode` | `ValidRect` | K | `cd_fx::p_fx_04` | proven |  | whole domain (`ValidRect`) | M-209, M-227 |
| P-FX-05 | `CellC` round trip | `S2Cell::Encode`/`Decode` | `ValidId` | K | `cd_fx::p_fx_05` | proven |  | whole domain | M-213, M-228 |
| P-FX-06 | `CellIdC` agreement, every input up to 9 bytes (FR-022 deviation: invalid ids rejected) | `S2CellId::Decode`, `s2cell_id.cc:261–265` | `Bytes(L)` | K | `cd_fx::p_fx_06__b0 … __b9` | proven |  | every input up to 9 bytes: the whole domain | M-211 |
| P-FX-07 | `PointC` agreement, every input up to 25 bytes | as P-FX-02 | `Bytes(L)` | K | `cd_fx::p_fx_07__b0 … __b25` | proven |  | the whole domain | M-212 |
| P-FX-08 | `CapC` agreement, every input up to 33 bytes (FR-022 deviation: invalid caps rejected; D-004 open; spec 007 FR-017 deviation: a negative length² other than −1 decodes as −1, `wire_spec::lib_cap`) | `S2Cap::Decode`, `s2cap.cc:340–353`; `FromLength2`, `s1chord_angle.h:365–367` | `Bytes(L)` | K | `cd_fx::p_fx_08__b0 … __b33` (`__b32`, `__b33` weekly); R18's fallback | proven |  | the whole domain, acceptance relative to `Cap::is_valid`; `__b32` and `__b33` re-proven 2026-10-19 (711.9 s, 743.0 s) after `CapC`'s reference decoder took over D-004's NaN rejection (spec 007 FR-018) | M-210 |
| P-FX-09 | `RectC` agreement, every input up to 34 bytes | `S2LatLngRect::Decode`, `s2latlng_rect.cc:372–392` | `Bytes(L)` | K | `cd_fx::p_fx_09__b0 … __b34` | proven |  | the whole domain | M-209 |
| P-FX-10 | `CellC` agreement, every input up to 9 bytes (invalid ids rejected; upstream undefined, U-004) | `S2Cell::Decode`, `s2cell.cc:326–332` | `Bytes(L)` | K | `cd_fx::p_fx_10__b0 … __b9` | proven |  | the whole domain (invalid ids rejected; U-004) | M-213 |
| P-PAR-01 | the reference description agrees with every row of `cpp_coding_vectors.csv`, and the library description gives the documented answer on every row except the counted D-003/D-004 rows | the generator `tools/refgen-cpp/generate_coding.cc` | 4,714 rows | test | `verify/tests/wire_spec_vs_cpp.rs` | passing | | every row | |

## Coverage (SC-003, SC-004)

**SC-003.** 31 entries count (P-PAR-01 is a test). Two have an operation outside the scope:
P-SV-01 and P-SV-02 use `decode_string_vector`, a tool limit (INVENTORY row 11). Of the other
29, 28 are proven by at least one track: 96.6 %, against the target of 90 %. The exception is
P-PV-03, the point vector's compact branch, a tool limit from 2 bytes (T029). Both string
vector entries are proven on the V track for every length.

**SC-004.** Every proven K-track entry names its mutants in the last column. `make
verify-mutants` must report each of them killed (T050; `verify/MUTANTS.md`). Every V-track
entry names the `V-NNN` mutant that `make verify-deductive MUTANTS=1` kills. The covers of
every crash and agreement harness are satisfied in the runs that EVALUATION.md records.

**Tiers.** "Weekly" members run under `make verify-slow F=decoding` (8 GiB, 30 minutes per
harness); everything else runs per change, in `make verify-decoding`. A bound that names a
weekly member is reached only by the weekly run.

