# Quickstart: validating spec 012 end to end

Every Kani command runs through the Makefile (the memory-guarded `run-verify.sh`), one at a time.

1. `make test`: the gate, with `cpp_cell_bounds_diff`, `cell_tests::bounds_miss_the_margin`, the
   `region(i)` panic test and the twins. Measured: passes (2026-10-31).
2. The probe again (as `probe/README.md`): `CB` matches C++, `RU`/`RI` differ only by spec 010's
   cap radii.
3. `make verify-values P=adaptors` (measured: 32 of 32 in 5 min 29 s; the weekly 8 of 8; the release pass 46 of 46), the weekly members (`make verify-slow F=values P=…`), `make verify-release`.
4. `make verify-mutants M='M-974 …'`: measured: 14 of 14 killed.
5. `cd verify && cargo test --test property_kinds`: passes; the evaluation's spec-012 part answers
   SC-013.
