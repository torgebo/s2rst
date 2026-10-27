# Planning probe: literal inference through `impl Into<Level>`

`into_inference.rs` is a stand-alone copy of `Level`'s shape, used for research P3. Compile it
with `rustc --edition 2024 --cfg <case> into_inference.rs`. The results below are for
rustc 1.99.1 on 2026-10-27.

| Case | `From` impls for `Level` | `parent_at_level(5)` through `impl Into<Level>` |
|------|--------------------------|-------------------------------------------------|
| `case_a` | `From<u8>` | compiles: the literal is inferred as `u8`, the only candidate |
| `case_b` | `From<u8>` and `From<u16>` | error E0277, `Level: From<i32>` not satisfied: with two candidates the literal falls back to `i32` |
| `case_c` | none | error E0277, `Level: From<{integer}>` not satisfied |
| `case_const` | any | `const BAD: Level = Level::new(31)` is error E0080 ("evaluation panicked"); `const { Level::new(5) }` compiles |
