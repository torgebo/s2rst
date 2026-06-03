// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! WebAssembly bindings for the s2rst spherical geometry library.
//!
//! This crate provides a JavaScript-friendly API surface via `wasm-bindgen`,
//! wrapping the core `s2rst` library types as opaque JS classes with methods.

use wasm_bindgen::prelude::*;

mod angle;
mod cap;
mod cell;
mod cell_id;
mod cell_union;
mod convex_hull;
mod earth;
mod error;
mod latlng;
mod point;
mod polygon;
mod polyline;
mod rect;
mod region_coverer;
mod s2loop;
mod shape_index;
mod text_format;

// Re-export all public types so they are registered by wasm-bindgen.
pub use angle::*;
pub use cap::*;
pub use cell::*;
pub use cell_id::*;
pub use cell_union::*;
pub use convex_hull::*;
pub use earth::*;
pub use latlng::*;
pub use point::*;
pub use polygon::*;
pub use polyline::*;
pub use rect::*;
pub use region_coverer::*;
pub use s2loop::*;
pub use shape_index::*;
pub use text_format::*;

/// Library version.
#[wasm_bindgen]
pub fn version() -> String {
    "0.1.0".to_string()
}
