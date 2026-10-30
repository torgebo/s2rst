# Quickstart: validating spec 012 end to end

Every Kani command runs through the Makefile (the memory-guarded `run-verify.sh`), one at a time.

1. `make test`: the gate, with `cpp_cell_bounds_diff`, `cell_tests::bounds_miss_the_margin`, the
   `region(i)` panic test and the twins. Expected: passes.
2. The probe again (as `probe/README.md`): `CB` matches C++, `RU`/`RI` differ only by spec 010's
   cap radii.
3. `make verify-values P=adaptors` (expected: every `ad_*` harness `SUCCESSFUL`, under 10
   minutes), the weekly members (`make verify-slow F=values P=…`), `make verify-release`.
4. `make verify-mutants M='M-974 …'`: every mutant killed.
5. `cd verify && cargo test --test property_kinds`: passes; the evaluation's spec-012 part answers
   SC-013.
