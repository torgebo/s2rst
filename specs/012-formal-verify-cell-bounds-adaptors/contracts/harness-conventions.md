# Contract: Harness and Artifact Conventions (changes for spec 012)

004's to 011's contracts still apply.

| Module | Content | Gate |
|--------|---------|------|
| `any_region.rs` | the abstract region (data-model §1, research R4) | `cfg(all(kani, feature = "values"))` |
| `ad_crash.rs`, `ad_mean.rs`, `ad_reg.rs` | the adaptors' crash, meaning and preservation harnesses: `<module>::<c_…|n_…|p_ad…>__{ru,ri}__n<k>` | same |

- **Catalogue ids:** `P-ADCON`, `P-ADBND`, `P-ADTST`, `P-ADREG`; `Cell`'s `P-CREG`.
- **Member counts** concrete (`__n<k>`), chosen so a statement's table keys fit in ten slots.
- **Preservation statements** read each member's answer through the method the adaptor calls,
  assume the law of it, then assert the law of the adaptor.
- **Mutants** `M-974` onwards, series 9; at least one patches `any_region.rs`.
- **Records**: `values/INVENTORY.md` §14, `PROPERTIES.md` §13 and §2's `P-CREG` rows, and the
  documents of spec FR-020.
