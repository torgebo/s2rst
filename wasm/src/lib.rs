// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! WebAssembly bindings for the s2rst spherical geometry library.
//!
//! This crate provides a JavaScript-friendly API surface via `wasm-bindgen`,
//! wrapping the core `s2rst` library types as opaque JS classes with methods.
//!
//! # Error model
//!
//! Failure is split into two kinds:
//!
//! - **Expected failures** — malformed user input, out-of-range indices, invalid
//!   geometry — are surfaced as JS exceptions: the binding returns
//!   `Result<_, JsValue>` and throws. JS callers can `try`/`catch` them. Parsers
//!   are strict (they throw rather than silently skip/default), a deliberate
//!   divergence from the lenient core parsers chosen so a typed JS API never
//!   returns silently-wrong geometry.
//! - **Bugs** — invariant violations — `panic!`. Because WebAssembly cannot
//!   unwind, a panic aborts and poisons the instance, so panics are reserved for
//!   genuine defects. [`start`] installs [`console_error_panic_hook`] so any
//!   panic produces a readable message and stack instead of a bare
//!   `RuntimeError: unreachable`.

use wasm_bindgen::prelude::*;

mod angle;
mod buffer;
mod builder;
mod cap;
mod cell;
mod cell_id;
mod cell_union;
mod convex_hull;
mod earth;
mod error;
mod indexes;
mod latlng;
mod lax;
mod point;
mod polygon;
mod polyline;
mod rect;
mod region_coverer;
mod s2loop;
mod shape_index;
mod snap;
mod term_indexer;
mod tessellate;
mod text_format;

// Re-export all public types so they are registered by wasm-bindgen.
pub use angle::*;
pub use buffer::*;
pub use builder::*;
pub use cap::*;
pub use cell::*;
pub use cell_id::*;
pub use cell_union::*;
pub use convex_hull::*;
pub use earth::*;
pub use indexes::*;
pub use latlng::*;
pub use lax::*;
pub use point::*;
pub use polygon::*;
pub use polyline::*;
pub use rect::*;
pub use region_coverer::*;
pub use s2loop::*;
pub use shape_index::*;
pub use snap::*;
pub use term_indexer::*;
pub use tessellate::*;
pub use text_format::*;

/// Module init: installs a panic hook so Rust panics surface as readable JS
/// errors with a stack trace. Runs automatically when the module is loaded.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Library version (sourced from `Cargo.toml`, so it cannot drift).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
