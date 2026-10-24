# `verify/deductive/`: the Verus track of spec 003

Restated copies of the library's variable-length codecs (the varint, the fixed-width
helpers, the integer vectors, the cell-id vector and the string vector), proved with Verus
for **every input and every length**, and the tests that tie each copy to the library
function it mirrors. Design: `specs/003-formal-verify-decoders/` (research R11, R12;
`contracts/deductive-conventions.md`). Results: `verify/decoding/PROPERTIES.md` (the V
column) and `verify/decoding/EVALUATION.md`, "Kani vs Verus on shared properties".

## What each module proves

| Module | Restates | Proved |
|--------|----------|--------|
| `varint.rs` | `write_uvarint`, `read_uvarint` | the encoder writes the shortest form; the decoder is the spec decoder on every input (D-003's tenth-byte rule included); round trip with exact consumption for every `u64` |
| `fixed_width.rs` | `encode_`/`get_`/`decode_uint_with_length`, `byte_length_u32`/`_u64` | the little-endian value and bytes; the minimal width |
| `uint_vector.rs` | `encode_`/`decode_uint_vector_u32`/`_u64` | the canonical encoding; the spec decoder on every input; round trip for every vector up to the count limit |
| `cellid_vector.rs` | `encode_s2cell_id_vector`, `encode_`/`decode_base_shift`, `decode_s2cell_id_vector` | the decoder is the spec decoder; decoding the encoder's output returns the ids, for every sequence, with upstream's base and shift choice |
| `string_vector.rs` | `encode_`/`decode_string_vector` | the spec decoder, FR-020's rule for invalid ranges; round trip for every string sequence within the limits |

No `assume`, `admit`, `external_body` or `external` in the restated code or its proofs.
vstd's own specifications of standard-library functions (`Vec::push`, slice helpers) are
trusted.

## Running

From the repository root, with Verus 0.2026.09.20.aef82ed unpacked somewhere and Rust 1.98.1
installed (`rustup toolchain install 1.98.1`):

```sh
make verify-deductive VERUS=<unpacked dir>/verus            # proofs, then the link tests
make verify-deductive VERUS=<unpacked dir>/verus MUTANTS=1  # the V-NNN mutants instead
```

`run-verus.sh` caps Verus's memory and verifies one module at a time. The whole run takes
about 20 seconds on a workstation. Never run `verus` on this crate uncapped.

`cargo test` (stable Rust) builds the same sources with the ghost code erased and runs the
link tests.

## The rules for restated code

A restated function is the library function statement by statement, with only the
differences that `mirrors.toml` lists for it: a slice and a cursor for the reader, a
`Vec<u8>` for the writer, `None` for an `io::Error`, and elementary operations for
intrinsics vstd does not specify. Every restated decoder ensures that it returns exactly
the spec decoder's answer; every restated encoder ensures that it appends exactly the spec
encoding.

## Links to the library (FR-011)

- **Drift guard** (`tests/drift.rs`, `mirrors.toml`): fails when a mirrored library
  function's tokens change, naming the function and its restated copy. After reviewing the
  restated copy against the change, run `make verify-deductive-bless` to record the new
  fingerprints.
- **Tested agreement** (`tests/agree.rs`): library and restated code agree on random inputs
  (byte strings up to 4 KiB, vectors up to 10,000 elements) and on every row of the C++
  vectors. It is the string vector decoder's only link.
- **Bounded agreement** (`verify/src/cd_agree.rs`, Kani): the same comparisons on every input
  within the R5 bounds.

## Mutants

`mutants/V-NNN-*.patch` are one-defect diffs of the restated files, at least one per
theorem; `MUTANTS=1` applies each in a throwaway copy and expects Verus to report a
verification error. A mutant that fails some other way (it does not compile, or Verus stops
before its result line) is reported as `ERROR` and fails the run, like a survivor. The
library-side mutants `verify/mutants/M-25N-*.patch` change a mirrored library function; the
bounded agreement and the drift guard must both catch each of them.
