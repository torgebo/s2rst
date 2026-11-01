# Quickstart: validating spec 013 end to end

Every Kani command runs through the Makefile (the memory-guarded `run-verify.sh`), one at a time.

1. `cd verify && cargo test --release --test libm_contracts -- --ignored` and `cargo test`: the
   order contract on ten million pairs per rule, the anchors, the lemma, the quadrant premise
   (`uv_projection`), the drift guard. Expected: passes.
2. `make verify-values P=cellrect`: `p_creg_02__l0` and `__lat` (expected: under 10 minutes).
3. `make verify-slow F=values P='cr_reg::p_creg_02__lng_f'`: the six longitude members.
4. `make verify-mutants M='M-988 M-989 M-990 M-991 M-992'`: every mutant killed.
5. `make test`: the gate with the new twin; `cd verify && cargo test --test property_kinds`.
