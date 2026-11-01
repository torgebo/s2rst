# Contract: Harness and Artifact Conventions (changes for spec 013)

004's to 012's contracts still apply.

| Module | Content | Gate |
|--------|---------|------|
| `cr_reg.rs` | `Cell`'s bounding-rectangle law: `p_creg_02__l0`, `__lat`, `__lng_f<face>` | `cfg(all(kani, feature = "values"))` |
| `stubs.rs` (spec 013 section) | `atan2_order`, `sqrt_order`, `sumsq`, `latitude_lemma`, `quadrant_uv_bound` | `cfg(kani)` |

- **Order contracts** keep a log per proof; every harness that uses one calls `stubs::reset`
  first.
- **Lemma ids** `L-CR-NN`; trusted lemmas say why (the standard) and name their native test.
- **Mutants** `M-988` onwards, series 9.
