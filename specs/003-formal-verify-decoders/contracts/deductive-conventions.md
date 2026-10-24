# Contract: Deductive Track Conventions (new)

The rules for the restated copies, their proofs and their links to the library (FR-010 to
FR-012; research R11, R12). They apply only to `verify/deductive/`.

## Crate

| Item | Contract |
|------|----------|
| Location | `verify/deductive/`: a standalone Cargo package `s2rst-deductive` with its own `[workspace]`, `publish = false`, outside the root workspace, like `fuzz/` and `verify/` |
| Dependencies | `vstd`, `verus_builtin` and `verus_builtin_macros`, pinned with `=` to the versions that match `VERUS_VERSION` (research R11). All three are MIT. No other normal dependency |
| Dev-dependencies | `s2rst` by path (the library under comparison), `quickcheck` (tested agreement), `syn` and `quote` (drift guard). No dependency of any published crate changes (FR-012) |
| Toolchains | Verus verifies the crate with its own pinned compiler (1.98.1). `cargo test` builds the same source with the project's stable toolchain, where the `verus!` macro erases all ghost code. Kani builds it with Kani's nightly for the bounded agreement harnesses |
| Lints | `unsafe_code = "forbid"`. Every `#[allow]` carries a `reason` |

## Modules

| Module (`verify/deductive/src/`) | Restates |
|----------------------------------|----------|
| `varint.rs` | `encoding::write_uvarint`, `encoding::read_uvarint` |
| `fixed_width.rs` | `encoded_uint_vector::encode_uint_with_length`, `decode_uint_with_length`, `byte_length_u32`, `byte_length_u64` |
| `uint_vector.rs` | `encode_uint_vector_u32`, `encode_uint_vector_u64`, `decode_uint_vector_u32`, `decode_uint_vector_u64` |
| `cellid_vector.rs` | `encode_s2cell_id_vector`, `encode_base_shift`, `decode_s2cell_id_vector`, `decode_base_shift` |
| `string_vector.rs` | `encode_string_vector`, `decode_string_vector` |
| `lib.rs` | The module list, and nothing else |

Each module has one `verus! { … }` block. It holds the spec functions (the format's
meaning), the restated exec functions, and the lemmas.

## Restatement rules

1. **Statement by statement.** A restated function keeps the library function's control
   flow, its arithmetic, its limits and its order of checks. A reader can put the two side
   by side and match every line.
2. **Permitted differences**, each one listed for its function in `mirrors.toml`:
   - the reader: `(buf: &[u8], pos: usize)` with the new position returned, instead of
     `&mut dyn Read`. A short read becomes `None` at the same point where the library
     returns `UnexpectedEof`;
   - the writer: `&mut Vec<u8>` instead of `&mut dyn Write`;
   - errors: `None` instead of an `io::Error`. Error kinds and messages are not compared;
   - preallocation: capacity is not modelled, because it is not observable in the result
     (FR-019 has its own evidence);
   - an intrinsic that Verus does not specify (for example `leading_zeros` or `div_ceil`, if
     vstd lacks them) may be restated in elementary operations. The difference is listed,
     and the agreement checks cover it.
3. **No escape hatches** in the restated code or its proofs: no `assume`, no `admit`, no
   `#[verifier::external_body]`, no `#[verifier::external]` and no
   `exec_allows_no_decreases_clause`. vstd's own specifications of standard-library
   functions (for example `Vec::push`) are trusted and listed in the trusted base.
4. **Exec functions carry their facts.** Every restated decoder `ensures` that it returns
   exactly what the module's spec decoder returns. Every restated encoder `ensures` that
   it appends exactly the module's spec encoding. Termination and the absence of
   overflow and indexing errors follow from Verus's own obligations, for every input.

## Theorems

Each module ends with its theorem, stated against its spec functions, for every length and
every surrounding bytes:

- `lemma_<codec>_round_trip(prefix, v, rest)`: decoding `prefix + encode(v) + rest` at
  `prefix.len()` returns `v` and stops at `prefix.len() + encode(v).len()`. The domain is
  the codec's encodable set, for example vectors up to the library's count limit.
- **Sound acceptance** is the restated decoder's `ensures`: for **every** byte sequence and
  position, it returns exactly what the spec decoder (the format's meaning) returns. None of
  these formats has a validity condition beyond its own syntax. Upstream and the library both
  accept non-canonical encodings (an over-long varint, decreasing string offsets), so
  sound acceptance does not mean "re-encoding gives the same bytes". Where a format is
  non-canonical, the module states the relation it does satisfy, for example "re-encoding
  gives a canonical encoding of the same value".

## Links to the library (FR-011)

| Link | Where | Rule |
|------|-------|------|
| Bounded agreement | `verify/src/cd_agree.rs` (Kani) | For every input within the FR-008 bounds: the library's decoder on `&mut &bytes[..]` and the restated decoder on `(&bytes, 0)` agree on acceptance, on the value and on the bytes consumed. The library's encoder and the restated encoder write identical bytes for every value sequence within the bounds |
| Tested agreement | `verify/deductive/tests/agree.rs` (`cargo test`) | The same comparisons on quickcheck inputs: byte strings up to 4 KiB, vectors up to 10,000 elements, strings up to 1 KiB. Also every row of `cpp_coding_vectors.csv` |
| Drift guard | `verify/deductive/tests/drift.rs` and `mirrors.toml` | For every mirrored library function, the test parses its source file, takes the function's tokens without doc comments, and compares their FNV-1a 64-bit hash with the recorded fingerprint. A mismatch fails and names the function and its restated copy. `make verify-deductive-bless` updates the fingerprints after review |

`mirrors.toml` has one entry per mirrored function:

```toml
[[mirror]]
restated = "uint_vector::decode_uint_vector_u64"
library = "core/src/s2/encoded_uint_vector.rs::decode_uint_vector_u64"
fingerprint = "fnv1a64:…"
differences = ["slice and cursor reader", "None for io::Error", "no preallocation"]
```

A library function inside an `impl` block is named `file.rs::<impl header>::<fn>`, for
example `encoding.rs::impl S2Decode for CellId::decode`.

## Mutants

- **`V-NNN-<slug>.patch`** in `verify/deductive/mutants/`: a one-defect diff of a restated
  file. Verus must fail on it. At least one per theorem (FR-009).
- **`M-25N-<slug>.patch`** in `verify/mutants/`: a one-defect diff of a mirrored library
  function. Both the bounded agreement harness for that codec and the drift guard must
  fail on it (FR-009, SC-005).
- `make verify-deductive` runs the `V-` patches only with `MUTANTS=1`, each in a throwaway
  copy of the crate. The weekly `mutants` CI job runs them.
