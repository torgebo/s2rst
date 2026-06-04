# Contributing to s2rst

Thanks for your interest in s2rst, a Rust port of
[Google's S2 Geometry library](https://s2geometry.io/) with optional Python
(PyO3) and WebAssembly bindings.

The software is **beta and not intended for production** (see the
[README](README.md)), so the bar for changes is correctness and parity with
upstream S2, kept honest by the test suite.

This is a small project. Before starting non-trivial work, please open an issue
to discuss it so we don't duplicate effort or diverge on approach.

## Workspace layout

| Path      | Crate          | Description                                      |
|-----------|----------------|--------------------------------------------------|
| `core/`   | `s2rst`        | The Rust core library (the published crate).     |
| `python/` | `s2rst-python` | Python bindings, exposed as the `s2rst` module.  |
| `wasm/`   | `s2rst-wasm`   | WebAssembly bindings.                            |

## Building and testing

Targets are driven from the [`Makefile`](Makefile); run `make help` to list them.

The verification gate for the **whole workspace** is:

```
make test
```

It runs, in order: `cargo check` (all features), `cargo fmt --check`,
`cargo clippy` (warnings as errors), the core Rust tests, the API docs build
(warnings as errors), the Python suite, the wasm suite, and `cargo-deny`.
**`make test` must pass before a pull request is ready for review.**

Useful narrower targets while iterating:

| Target              | What it does                                              |
|---------------------|----------------------------------------------------------|
| `make check`        | Type-check the workspace (all features).                 |
| `make fmt-check`    | `cargo fmt --check` (formatting only).                   |
| `make clippy`       | Lint the workspace, warnings as errors.                  |
| `make test-core`    | Run the core (`s2rst`) Rust test suite.                  |
| `make doc`          | Build the `s2rst` API docs, warnings as errors.          |
| `make deny`         | Audit dependencies with `cargo-deny`.                    |
| `make -C python test` | The full Python suite (see below).                     |
| `make -C wasm test` | The wasm-bindgen tests in Node (`wasm-pack test --node`).|

The Python and wasm targets need extra tooling
([`uv`](https://docs.astral.sh/uv/) / [maturin](https://www.maturin.rs/) for
Python; [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and a Node runtime
for wasm). If you only touch the core crate, `make test-core` plus
`make fmt-check`, `make clippy`, and `make doc` cover most of it — but the full
`make test` is what gates a PR.

The benchmark targets (`bench-iai`, `bench-wall`, `pgo`, …) are for performance
work and are **not** part of `make test`; don't run them as part of a normal
change.

## Tests

Add tests for new behaviour and for any bug you fix.

### Rust: separate `<module>_tests.rs` files

New Rust unit tests for a module go in a **separate sibling file** named
`<module>_tests.rs`, wired from the source file with a `#[path = ...]`
declaration — not appended to an inline `#[cfg(test)] mod tests`:

```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod foo_tests;
```

Because the test module is still a child of the module under test, `use super::*`
(and `use super::Item`) reaches `pub(crate)` and private items. Leave any
pre-existing inline tests where they are.

Integration-style tests live under `core/tests/` (e.g.
`core/tests/property_tests.rs`); property-based tests use `quickcheck`.

### Python and wasm

Python tests run under `pytest` (`python/tests/`); wasm tests are
`wasm-bindgen` tests run in Node via `wasm-pack`.

## Code style and lints

- **Rust formatting:** `cargo fmt`. CI checks it with `make fmt-check`.
- **Rust lints:** clippy is configured in pedantic mode in
  [`core/Cargo.toml`](core/Cargo.toml) (`[lints.clippy]`, plus a strict
  `[lints.rust]` set including `unsafe_code = "forbid"`). `make clippy` treats
  **all warnings as errors**, so a change must not introduce new warnings. If a
  lint genuinely doesn't fit, prefer adjusting the shared config over scattering
  `#[allow]`s — and note that `allow_attributes_without_reason` is on, so any
  `#[allow]` needs a `reason = "…"`.
- **Docs:** public items are documented (`missing_docs` is a warning and
  `make doc` denies warnings).
- **Python:** [`ruff`](https://docs.astral.sh/ruff/) for lint and format
  (`ruff check` / `ruff format --check`).
- **Python stub:** the package ships type stubs
  (`python/python/s2rst/_core.pyi` + `py.typed`). They are checked with `mypy`
  and `mypy.stubtest` (`make -C python typecheck`). If you change the bindings'
  public API, **update the stub to match** — stubtest will fail otherwise.

## License and SPDX headers

s2rst is licensed under the [Apache License, Version 2.0](LICENSE), the same
license as upstream S2. By contributing you agree your contribution is licensed
under those terms.

Every source file carries an SPDX header. Match the surrounding files:

- **Ported / derivative source files** (most of `core/src/`) carry
  `SPDX-License-Identifier: Apache-2.0`, an `SPDX-FileCopyrightText` line, and a
  short note that the file is part of a Rust port of Google's S2 (a derivative
  work of the upstream Apache-2.0 implementations). See
  [`core/src/lib.rs`](core/src/lib.rs) for the exact style.
- **Original test files** (`<module>_tests.rs`, files under `core/tests/`) carry
  only the two-line header

  ```rust
  // SPDX-License-Identifier: Apache-2.0
  // SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
  ```

  plus a one-line note that they were written for this crate and not ported from
  upstream S2. Do **not** add the Google/derivative-work attribution lines to
  original test files.

## Pull requests

Before opening a PR:

1. `make test` passes (or the relevant per-crate target if you only touched one
   crate — but the full suite is the gate).
2. Tests are added or updated for the change.
3. Docs and the Python stub are updated if the public API changed.
4. No new clippy warnings.

The [pull request template](.github/PULL_REQUEST_TEMPLATE.md) restates this as a
checklist. Keep PRs focused and the description clear about what changed and why.
