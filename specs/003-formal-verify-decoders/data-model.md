# Data Model: Formal Verification of the Decoding Layer

This feature adds no runtime data. The entities below are the **verification artifacts**
(spec Key Entities), the **reference description** of the wire formats, and the
**restated copies** of the deductive track. The schemas extend 001's and 002's
(`specs/001-formal-verify-base-type/data-model.md`,
`specs/002-formal-verify-cell-union/data-model.md`). Where a field is unchanged, this
document says so.

## 1. Reference description of the wire formats (`verify/src/wire_spec.rs`)

The description is written from upstream's format documentation and code (research R3) and
never calls `s2rst`'s coding modules (FR-007). Like `spec.rs` and `union_spec.rs`, it is
plain Rust, compiled outside Kani, and sanity-tested against C++ vectors (research R10).

Each format has a **reference decoder** `ref_<fmt>(b: &[u8]) -> Option<(V, usize)>`. It
returns the value and the number of bytes consumed, or `None` where upstream rejects the
input. Each format also has the **canonical encoder** `enc_<fmt>(v) -> Vec<u8>`, the bytes
upstream's encoder writes.

| Format | Reference decoder: accepts and returns | Canonical encoder | Upstream source |
|--------|----------------------------------------|-------------------|-----------------|
| varint | 1–10 bytes, little-endian 7-bit groups, high bit = continuation; a tenth byte must be 0 or 1. Over-long forms are accepted | shortest form | `util/coding/varint.h`, `varint.cc:229–262` |
| fixed width `len` | `len` ≤ 8 bytes, little-endian | low `len` bytes | `encoded_uint_vector.h:139–170` |
| integer vector `T` (4 or 8 bytes) | a varint `h`, then `h / sizeof(T)` elements of `(h mod sizeof(T)) + 1` bytes each | width = minimal bytes of `1 | OR(v)`; header `n·sizeof(T) | (width − 1)` | `encoded_uint_vector.h` header comment and `Init` |
| cell-identifier vector | a byte `shift_code << 3 | base_len`; if `shift_code = 31`, a byte `s` with `29 + s ≤ 56`; `base_len` bytes of the base's top bytes; an integer vector of 64-bit deltas. Value `i` = `base + (delta_i << shift)`, wrapping | upstream's base and shift choice (the minimal-bytes search) | `encoded_s2cell_id_vector.cc` |
| string vector | a 64-bit integer vector of end offsets, then `offsets.last()` data bytes. Entry `i` covers `[offsets[i−1], offsets[i])`, or is empty when that range is out of order or outside the data | offsets of the cumulative lengths | `encoded_string_vector.cc:57–84` (v0.14, lazy check) |
| point vector, uncompressed | a varint `h` with format tag `h & 7 = 0`, then `h >> 3` points of 24 bytes | the FAST encoding | `encoded_s2point_vector.cc` |
| `CellId` | 8 bytes little-endian | the same | `s2cell_id.cc:261` |
| `Point` | 24 bytes, three `f64` | the same | `s2point` coder |
| `Cap` | 32 bytes: centre and `length²` | the same | `s2cap.cc:330–353` |
| `Rect` | a version byte ≤ 1, then 32 bytes; rejected unless valid | version 1 | `s2latlng_rect.cc:360–392` |
| `Cell` | the `CellId` format | the same | `s2cell.cc:326` |

**Library deviations** are named predicates, kept separate from the upstream meaning and
applied in properties as "upstream's answer, minus the documented deviations" (FR-022):

| Deviation | Applies to | Predicate |
|-----------|------------|-----------|
| `DEV_COUNT` | integer vectors (`count > 50 000 000` rejected), point vector (`> 50 000 000` points) | count above the limit ⇒ reject |
| `DEV_STRING_TOTAL` | string vector | `offsets.last() > 500 000 000` ⇒ reject |
| `DEV_VALID_ID` | `CellId`, `Cell` | `!spec::is_valid(id)` (001's model) ⇒ reject |
| `DEV_VALID_CAP` | `Cap` | not `cap_valid` (§2) ⇒ reject |
| `DEV_VARINT_TENTH` (D-003, while open) | varint | a tenth byte 2–127 is accepted and truncated. Listed in `known-defects.txt` until fixed |

**Validation rules**: `wire_spec_tests.rs` holds hand-made unit tests (Constitution III).
`tests/wire_spec_vs_cpp.rs` checks every reference decoder and canonical encoder against
every row of `cpp_coding_vectors.csv`.

## 2. Input domain classes

The cell domains are 001's. This feature adds:

| Class | Meaning |
|-------|---------|
| `Bytes(L)` | every byte string of exactly `L` bytes. A family `__b0 … __bL` covers "up to `L`" |
| `Len` | `0 ≤ len ≤ 8` (the fixed-width helpers' documented precondition, FR-021) |
| `Fits(len)` | values whose bits above `8·len` are zero |
| `Ints64(N)`, `Ints32(N)` | exactly `N` values of any bit pattern |
| `Ids(N)` | exactly `N` 64-bit identifiers, valid or not (the format carries any bits) |
| `Strings(N, M)` | exactly `N` byte strings, each of symbolic length ≤ `M` |
| `Points(N)` | exactly `N` points of any `f64` bit patterns, NaN and infinities included |
| `ValidId` | 001's `Valid` |
| `AnyCap`, `ValidCap` | any centre and `length²` bit patterns; `ValidCap` adds `cap_valid`: `|x²+y²+z² − 1| ≤ 5ε` (in the library's operation order) and `length² ≤ 4` |
| `AnyRect`, `ValidRect` | any four `f64` bounds; `ValidRect` adds the documented `Rect::is_valid` conditions (research R18) |

## 3. Codec contract (`verify/src/codec.rs`)

```text
trait Codec {
    type Value;
    fn encode(v: &Value) -> Vec<u8>;                      // the library's encoder
    fn decode(r: &mut &[u8]) -> io::Result<Value>;         // the library's decoder
    fn reference_decode(b: &[u8]) -> Option<(Value, usize)>; // §1, with deviations
    fn encodable(v: &Value) -> bool;                       // round-trip domain
    fn same(a: &Value, b: &Value) -> bool;                 // bit-exact equality
}
```

| Codec | `Value` | `encodable` | Part 1 round trip | Parts 2–4 agreement | Part 5 reservation |
|-------|---------|-------------|-------------------|---------------------|--------------------|
| `U32Vec` | `Vec<u32>` | `len ≤ 50 000 000` | `Ints32(N)` | `Bytes(L)` | test (research R7) |
| `U64Vec` | `Vec<u64>` | the same | `Ints64(N)` | `Bytes(L)` | test |
| `IdVec` | `Vec<CellId>` | the same | `Ids(N)` | `Bytes(L)` | inherits `U64Vec` |
| `StrVec` | `Vec<Vec<u8>>` | total ≤ 500 000 000 | `Strings(N, M)` | `Bytes(L)` | test |
| `PtVec` | `Vec<Point>` | `len ≤ 50 000 000` | `Points(N)`, FAST | `Bytes(L)`, uncompressed branch; the compact branch is crash-only | test |
| `CellIdC` | `CellId` | valid | `ValidId` | `Bytes(0…9)` | n/a (fixed size) |
| `PointC` | `Point` | every bit pattern | `Points(1)` | `Bytes(0…25)` | n/a |
| `CapC` | `Cap` | `ValidCap` | `ValidCap` | `Bytes(0…33)` | n/a |
| `RectC` | `Rect` | `ValidRect` | `ValidRect` | `Bytes(0…34)` | n/a |
| `CellC` | `Cell` | valid id | `ValidId` | `Bytes(0…9)` | n/a |

The bounds `N` and `L` are in research R5. For `StrVec` and `PtVec`, `L` is set by
measurement during implementation, and may end as a tool limit (spec clarification
2026-10-05). Part 1 is proven by composition: encoder agreement, decoder agreement and the
model's own round trip (research R4). The fixed-width helpers are not codecs (they take a
length argument). They have their own properties (§4).

## 4. Property (`verify/decoding/PROPERTIES.md`)

The fields are 002's, plus **Track**: `K` (bounded checker), `V` (deductive, restated code)
or `K+V`, with a status per track (spec FR-006). Ids: `P-<AREA>-<nn>`.

### 4.1 Initial catalogue (32 entries)

| Id | Statement (against §1) | Track | Operations |
|----|------------------------|-------|------------|
| P-FW-01 | `encode_uint_with_length(v, len)` writes exactly `len` bytes, the low `len` bytes of `v` little-endian | K+V | `encode_uint_with_length` |
| P-FW-02 | `get_uint_with_length(d, len)` is the little-endian value of `d[..len]`, for `d.len() ≥ len` | K+V | `get_uint_with_length` |
| P-FW-03 | `decode_uint_with_length` returns what `get_` returns and consumes exactly `len` bytes, or reports `UnexpectedEof` on a shorter input | K+V | `decode_uint_with_length` |
| P-FW-04 | Writing then reading returns `v`, for every `len` ≤ 8 and every `v` in `Fits(len)` | K+V | all three |
| P-VAR-01 | Every vector decoder reads its header varint as upstream does: the same value and length, or rejection (D-003 listed until fixed) | K | the four vector decoders |
| P-VAR-02 | Varint round trip with exact consumption, for every `u64`, whatever surrounds it | V | (internal) |
| P-UV-01 | `U64Vec` round trip (contract part 1) | K+V | `encode_`/`decode_uint_vector_u64` |
| P-UV-02 | `U32Vec` round trip | K+V | `encode_`/`decode_uint_vector_u32` |
| P-UV-03 | `U64Vec` agreement (contract parts 2–4) | K+V | `decode_uint_vector_u64` |
| P-UV-04 | `U32Vec` agreement | K+V | `decode_uint_vector_u32` |
| P-UV-05 | The element width is the minimum number of bytes that holds `1 | OR(v)` | K+V | both encoders |
| P-UV-06 | The encoded length is `|varint(header)| + n · width` | K+V | both encoders |
| P-CV-01 | `IdVec` round trip, for identifiers of any bits | K+V | `encode_`/`decode_s2cell_id_vector` |
| P-CV-02 | `IdVec` agreement, including the extended shift code and shifts above 56 | K+V | `decode_s2cell_id_vector` |
| P-CV-03 | The encoder writes upstream's encoding: base length, shift and delta width as upstream's search chooses them | K | `encode_s2cell_id_vector` |
| P-SV-01 | `StrVec` round trip | K+V | `encode_`/`decode_string_vector` |
| P-SV-02 | `StrVec` agreement: an entry with an invalid range is empty, and decoding continues from its end (FR-020) | K+V | `decode_string_vector` |
| P-SV-03 | The encoded length is the offsets' integer vector plus the total data length | K+V | `encode_string_vector` |
| P-PV-01 | `PtVec` round trip in the FAST format, for every bit pattern | K | `encode_s2point_vector` (fast), `decode_s2point_vector` |
| P-PV-02 | `PtVec` agreement on the uncompressed branch; tags 2–7 rejected | K | `decode_s2point_vector` |
| P-PV-03 | The compact branch returns a value or an error on every input up to its bound (crash-only; the values come from float projections) | K | `decode_s2point_vector` |
| P-FX-01 … 05 | Round trip for `CellIdC`, `PointC`, `CapC`, `RectC`, `CellC` | K | the five encoders and decoders |
| P-FX-06 … 10 | Agreement for the same five codecs, over every input up to the encoded length plus one byte | K | the five decoders |
| P-PAR-01 | The reference description agrees with every row of `cpp_coding_vectors.csv` (a test entry, not a proof; SC-003 does not count it) | test | — |

Every in-scope inventory entry has at least one property (SC-003). The compact
point-vector encoder is excluded by design, and has none.

## 5. Operation inventory entry (`verify/decoding/INVENTORY.md`)

002's fields, with two additions:

- **Reservation evidence**: the test that shows part 5, or `n/a` for a fixed-size decoder.
- **Deductive mirror**: the restated function, if any.

The 24 entries:

| # | Entry | Initial status |
|---|-------|----------------|
| 1–3 | `encode_uint_with_length`, `get_uint_with_length`, `decode_uint_with_length` | in scope |
| 4–7 | `encode_`/`decode_uint_vector_u32`, `encode_`/`decode_uint_vector_u64` | in scope |
| 8–9 | `encode_`/`decode_s2cell_id_vector` | in scope |
| 10–11 | `encode_`/`decode_string_vector` | in scope |
| 12 | `encode_s2point_vector` with `CodingHint::Fast` | in scope |
| 13 | `encode_s2point_vector` with `CodingHint::Compact` | excluded by design (float projections, FR-002) |
| 14 | `decode_s2point_vector` | in scope (compact branch crash-only) |
| 15–24 | `impl S2Encode`/`S2Decode for CellId, Point, Cap, Rect, Cell` | in scope |

## 6. Restated copy and mirror (`verify/deductive/`)

| Field | Meaning |
|-------|---------|
| `restated` | `module::fn` in `verify/deductive/src/` |
| `library` | `core/src/s2/<file>.rs::<fn>`, or `<file>.rs::<impl header>::<fn>` |
| `fingerprint` | `fnv1a64:<16 hex>` of the library function's tokens without doc attributes |
| `differences` | the permitted differences used (contracts/deductive-conventions.md, rule 2) |

The theorems per module are listed in the contract. Each catalogue entry with track `V`
names its lemma.

## 7. Mutants

- **`M-2NN-<slug>.patch`** (`verify/mutants/`): 001's format against the in-scope files.
  About 25 are planned:
  - `M-200` is the crash demonstration (US1 scenario 4);
  - `M-201` … `M-24N` cover one property each, for example a width off by one, the header
    using `len` instead of `len − 1`, a decoder dividing by 4 for `u64`, a 3-bit width mask
    for `u64`, extended shift code `+ 28`, string start not advanced past an invalid entry,
    a format-tag mask of 2 bits, `Rect` version `>=`, `Cap` validity skipped, and `Point`
    coordinates swapped;
  - `M-25N` are library-side divergences that the bounded agreement harnesses and the
    drift guard must catch (SC-005).
- **`V-NNN-<slug>.patch`** (`verify/deductive/mutants/`): one-defect diffs of restated
  files that Verus must reject. At least one per theorem.

## 8. Mutation measurement (`verify/decoding/MUTATION.md`)

| Field | Meaning |
|-------|---------|
| `phase` | `before` or `after` |
| `tool` | cargo-mutants version, and the exact command |
| `mutant` | the tool's mutant name (file, line, replacement) |
| `tests` | `caught`, `missed`, `timeout` or `unviable` |
| `proofs` | for `missed` and `timeout`: `killed by <harness>`, `equivalent (<harness or argument>)` or `undetected` |
| `reason` | required for `undetected` |

The summary table gives the counts per phase and the difference (SC-012).

## 9. Known defect, upstream finding, evaluation

These are unchanged from 002: `known-defects.txt` lines, `U-NNN` rows, the evaluation
sections. The evaluation (`verify/decoding/EVALUATION.md`) adds:

- **Kani vs Verus on shared properties**: for each `K+V` entry, Kani's bound, time and
  memory, and Verus's obligations, time, memory and lines of specification and proof;
- **Mutation measurement summary** (§8);
- **The other branch's findings**, each with its status (spec FR-017).

Candidates recorded at planning time: D-003 (varint tenth byte), U-004 (`S2Cell::Decode`
on unvalidated ids reaching U-001), U-005 (`GetUintWithLength` beyond `sizeof(T)` in
release builds). See research R1 and R16.

## 10. C++ reference vectors (`core/tests/data/cpp_coding_vectors.csv`)

002's CSV layout (`kind,input,expected`, hex-encoded bytes, `reject` for rejection), plus a
fourth column `note`, which is `deviation` where the library documents a different answer.
The sections are in research R10.
