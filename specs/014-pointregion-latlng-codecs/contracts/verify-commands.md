# Contract: Verification Commands (changes for spec 014)

The 001–013 contracts still apply.

- `make verify-decoding P=codecs` runs `cd_fx`, now with P-FX-11 … P-FX-14.
- `make verify-mutants M='M-993 M-994 M-995 M-996 M-997 M-998'`.
- `make test` runs `cpp_coding_diff` with the four new kinds, the codecs' unit tests and twins;
  `cd verify && cargo test --test wire_spec_vs_cpp` the reference description on them.
