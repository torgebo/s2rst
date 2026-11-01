# Quickstart: validating spec 014 end to end

Every Kani command runs through the Makefile (the memory-guarded `run-verify.sh`), one at a time.

1. `make test`: the codecs' unit tests, the twins, `cpp_coding_diff` on the new rows. Measured:
   passes (2026-11-01).
2. `cd verify && cargo test`: `wire_spec_vs_cpp` on the new rows. Measured: passes.
3. `make verify-decoding P=codecs` and `P=fixed` (the leg was split, research R7): measured 101
   of 101 in 400 s and 96 of 96 in 385 s, P-FX-11 … 14 included.
4. `make verify-mutants M='M-993 … M-998'`: measured 6 of 6 killed.
5. `cd verify && cargo test --test property_kinds`: passes.
