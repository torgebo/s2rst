# Operation Inventory: the decoding layer (spec 003, FR-002)

Every public encode and decode operation of the decoding layer (spec FR-001): **24 rows**.
Field definitions: `specs/003-formal-verify-decoders/data-model.md` §5, which extends 002's
§3 with two columns, "reservation evidence" and "deductive mirror". Domain classes:
data-model §2. Bounds: research R5, with the string and point vector bounds set by
measurement (spec clarification 2026-10-05).

Each in-scope row has a crash-freedom obligation **C-&lt;entry&gt;**: "the operation completes
without panic, arithmetic overflow, undefined behaviour, abort or non-termination for every
input in *domain* within *bounds*, assuming *preconditions*" (FR-005). A decoder's domain is
every byte string, an encoder's every value or value sequence.

**Summary**: 22 in scope, 1 excluded by design (row 13), 1 tool limit (row 11, T029). Row 14 is
in scope with both branches tool-limited from 2 bytes. Crash results (T040, 2026-10-06): every in-scope row has a
crash-freedom result. 192 per-change harnesses pass in 678.6 s of wall time (4 jobs; 1,326 s
of harness time), plus 4 weekly members; 13 rows are proven over their whole domain, 9
within bounds.

## 1. Status, domain and crash proof

| # | entry | kind | status | exclusion reason | domain | preconditions (source) | crash harness | bounds | crash status | reservation evidence | deductive mirror |
|---|-------|------|--------|------------------|--------|------------------------|---------------|--------|--------------|----------------------|------------------|
| 1 | `encoded_uint_vector::encode_uint_with_length` | helper | in-scope | | any `u64`, `Len` | `len ≤ 8` (upstream `ABSL_DCHECK`, `encoded_uint_vector.h:145`; rustdoc after T039) | `cd_fw::p_fw_01` (P-FW-01) | whole domain (`len ≤ 8`) | proven | n/a | `fixed_width::encode_uint_with_length` |
| 2 | `encoded_uint_vector::get_uint_with_length` | helper | in-scope | | `Bytes(k)`, `k ≤ 8`, `len ≤ k` | `len ≤ 8`, `data.len() ≥ len` (`encoded_uint_vector.h:159`) | `cd_fw::p_fw_02__d0 … __d8` (P-FW-02) | whole domain (`len ≤ k ≤ 8`) | proven | n/a | `fixed_width::get_uint_with_length` |
| 3 | `encoded_uint_vector::decode_uint_with_length` | helper | in-scope | | `Bytes(L)`, `Len` | `len ≤ 8` (as row 1) | `cd_fw::p_fw_03__b0 … __b9` (P-FW-03) | whole domain: it reads at most 8 bytes, so the 9-byte member covers every longer input | proven | n/a | `fixed_width::decode_uint_with_length` |
| 4 | `encoded_uint_vector::encode_uint_vector_u32` | encoder | in-scope | | `Ints32(N)` | | `cd_uv::p_uv_02_enc__n0 … __n2` (P-UV-02) | `n ≤ 2`; longer: the Verus proof for every length, property tests | proven-within-bounds | n/a | `uint_vector::encode_uint_vector_u32` |
| 5 | `encoded_uint_vector::decode_uint_vector_u32` | decoder | in-scope | | `Bytes(L)` | | `cd_uv::p_uv_04__b0 … __b8` (P-UV-04); weekly: `__b9 … __b11` | `L ≤ 11`; longer inputs: `decoder_robustness`, `cpp_coding_diff`, the fuzz targets, the Verus proof | proven-within-bounds | `verify/tests/reservation.rs`; `decoder_robustness::reservation_under_address_space_limit` | `uint_vector::decode_uint_vector_u32` |
| 6 | `encoded_uint_vector::encode_uint_vector_u64` | encoder | in-scope | | `Ints64(N)` | | `cd_uv::p_uv_01_enc__n0 … __n2` (P-UV-01) | `n ≤ 2`; as row 4 | proven-within-bounds | n/a | `uint_vector::encode_uint_vector_u64` |
| 7 | `encoded_uint_vector::decode_uint_vector_u64` | decoder | in-scope | | `Bytes(L)` | | `cd_uv::p_uv_03__b0 … __b8` (P-UV-03); weekly: `__b9 … __b11`, `p_uv_03__h2_b18` | `L ≤ 11`; longer inputs: `decoder_robustness`, `cpp_coding_diff`, the fuzz targets, the Verus proof | proven-within-bounds | as row 5 | `uint_vector::decode_uint_vector_u64` |
| 8 | `encoded_s2cell_id_vector::encode_s2cell_id_vector` | encoder | in-scope | | `Ids(N)` | | `cd_cv::p_cv_03__n0` (P-CV-03); weekly: `cd_crash::c_encode_s2cell_id_vector__n1`, `__n2` and `cd_cv::p_cv_03__n1`, `__n2` | `n ≤ 2`; as row 4 | proven-within-bounds | n/a | `cellid_vector::encode_s2cell_id_vector` |
| 9 | `encoded_s2cell_id_vector::decode_s2cell_id_vector` | decoder | in-scope | | `Bytes(L)` | | `cd_cv::p_cv_02__b0 … __b4` (P-CV-02); weekly: `__b5 … __b11` | `L ≤ 11`; longer inputs: `decoder_robustness`, `cpp_coding_diff`, the fuzz targets, the Verus proof | proven-within-bounds | as row 5 | `cellid_vector::decode_s2cell_id_vector` |
| 10 | `encoded_string_vector::encode_string_vector` | encoder | in-scope | | `Strings(N, M)` | | `cd_sv::p_sv_03__n0 … __n2` (P-SV-03) | `n ≤ 2`, strings of up to 2 bytes; as row 4 | proven-within-bounds | n/a | `string_vector::encode_string_vector` |
| 11 | `encoded_string_vector::decode_string_vector` | decoder | tool-limit | every length from 1 byte runs out of memory at 4 and 8 GiB (EVALUATION, "Bounds measured first"); evidence: the Verus proof for every input, tested agreement, the differential rows, fuzzing | `Bytes(L)` | | `cd_xlarge::c_decode_string_vector__b1 … __b4` (xlarge tier, not yet run) | none within the budgets | not-proven (tool limit) | as row 5 | `string_vector::decode_string_vector` |
| 12 | `encoded_s2point_vector::encode_s2point_vector` (`CodingHint::Fast`) | encoder | in-scope | | `Points(N)` | | `cd_pv::p_pv_01_enc__n0 … __n2` (P-PV-01) | `n ≤ 2`; longer: property tests, `cpp_coding_diff` | proven-within-bounds | n/a | |
| 13 | `encoded_s2point_vector::encode_s2point_vector` (`CodingHint::Compact`) | encoder | excluded | by design: points become cell centres through floating-point projections (FR-002) | | | | | | | |
| 14 | `encoded_s2point_vector::decode_s2point_vector` | decoder | in-scope | both format branches are tool limits from 2 bytes (EVALUATION, "Exclusions and tool limits") | `Bytes(L)`, per format tag | | `cd_crash::c_decode_s2point_vector__b0`, `__b1` (every tag); agreement `cd_pv::p_pv_02__b0`, `__b1` (P-PV-02); xlarge: `cd_xlarge::c_decode_s2point_vector__t0_b2 …` | `L ≤ 1`, and every length for tags 2–7 (rejected after the first byte); longer inputs: `decoder_robustness`, `cpp_coding_diff`, the fuzz targets, the R13 property tests | proven-within-bounds | as row 5 | |
| 15 | `impl S2Encode for CellId` | encoder | in-scope | | any `u64` | | `cd_crash::c_encode_cell_id` | whole domain | proven | n/a | |
| 16 | `impl S2Decode for CellId` | decoder | in-scope | | `Bytes(L)` | | `cd_fx::p_fx_06__b0 … __b9` (P-FX-06) | `L ≤ 9`: whole domain | proven | n/a | |
| 17 | `impl S2Encode for Point` | encoder | in-scope | | any bits | | `cd_crash::c_encode_point` | whole domain | proven | n/a | |
| 18 | `impl S2Decode for Point` | decoder | in-scope | | `Bytes(L)` | | `cd_fx::p_fx_07__b0 … __b25` (P-FX-07) | `L ≤ 25`: whole domain | proven | n/a | |
| 19 | `impl S2Encode for Cap` | encoder | in-scope | | `AnyCap` | | `cd_crash::c_encode_cap` | whole domain (any centre and `length2` bits) | proven | n/a | |
| 20 | `impl S2Decode for Cap` | decoder | in-scope | | `Bytes(L)` | | `cd_fx::p_fx_08__b0 … __b31`, `cd_crash::c_decode_cap__b32`, `__b33`; weekly: `cd_fx::p_fx_08__b32`, `__b33` (P-FX-08) | `L ≤ 33`: whole domain | proven | n/a | |
| 21 | `impl S2Encode for Rect` | encoder | in-scope | | `AnyRect` | | `cd_crash::c_encode_rect` | whole domain | proven | n/a | |
| 22 | `impl S2Decode for Rect` | decoder | in-scope | | `Bytes(L)` | | `cd_fx::p_fx_09__b0 … __b34` (P-FX-09) | `L ≤ 34`: whole domain | proven | n/a | |
| 23 | `impl S2Encode for Cell` | encoder | in-scope | | `ValidId` (a `Cell` exists only for a valid id) | | `cd_crash::c_encode_cell` | whole domain (every valid id) | proven | n/a | |
| 24 | `impl S2Decode for Cell` | decoder | in-scope | | `Bytes(L)` | | `cd_fx::p_fx_10__b0 … __b9` (P-FX-10) | `L ≤ 9`: whole domain | proven | n/a | |
