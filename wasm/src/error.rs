// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

/// Convert an `S2Error` into a `JsValue` for throwing.
#[allow(dead_code)]
pub(crate) fn s2_error_to_js(e: s2rst::s2::builder::S2Error) -> JsValue {
    JsValue::from_str(&format!("{e}"))
}

/// Convert a validation error string into a `JsValue`.
pub(crate) fn validation_error_to_js(e: String) -> JsValue {
    JsValue::from_str(&e)
}
