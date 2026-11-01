# Quickstart: validating spec 014 end to end

Every Kani command runs through the Makefile (the memory-guarded `run-verify.sh`), one at a time.

1. `make test`: the codecs' unit tests, the twins, `cpp_coding_diff` on the new rows. Expected:
   passes.
2. `cd verify && cargo test`: `wire_spec_vs_cpp` on the new rows. Expected: passes.
3. `make verify-decoding P=codecs`: every `cd_fx` harness `SUCCESSFUL`, P-FX-11 … 14 included.
4. `make verify-mutants M='M-993 … M-998'`: every mutant killed.
5. `cd verify && cargo test --test property_kinds`: passes.
