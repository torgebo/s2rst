# Contract: Verification Commands (changes for spec 013)

The 001–012 contracts still apply.

- `make verify-values P=cellrect` selects the `cr_*` module (`p_creg_02__l0`, `__lat`); `P=…`
  without it runs every part.
- `make verify-slow F=values P='cr_reg::p_creg_02__lng_f'` runs the weekly longitude members.
- `cd verify && cargo test --release --test libm_contracts -- --ignored` runs the order
  contract's ten-million-pair tests.
- `make verify-mutants M='M-988 M-989 M-990 M-991 M-992'`.
- CI: the `verify` matrix gains `values-cellrect`; the slow job a `values-cellrect` leg.
