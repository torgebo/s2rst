<!--
Thanks for contributing to s2rst! Please describe what this change does and why.
See CONTRIBUTING.md for build/test instructions and conventions.
-->

## Summary

<!-- What does this PR change, and why? -->

## Checklist

- [ ] `make test` passes (or the relevant per-crate target — `make test-core`,
      `make -C python test`, `make -C wasm test` — if only one crate is touched).
- [ ] Added or updated tests for the change (new Rust tests go in a separate
      `<module>_tests.rs` file; see CONTRIBUTING.md).
- [ ] Updated the docs and the Python stub (`_core.pyi`) if the public API
      changed.
- [ ] No new clippy warnings (`make clippy` is clean — warnings are errors).
