// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

/// Build a `JsValue` error from any displayable message.
///
/// This is the single funnel for user-input errors in the bindings: every
/// fallible binding returns `Result<_, JsValue>` so the failure surfaces as a
/// catchable JS exception rather than a silent default or an uncatchable
/// `unreachable` trap. See the crate-level error-model contract in `lib.rs`.
pub(crate) fn js_err(msg: impl core::fmt::Display) -> JsValue {
    JsValue::from_str(&msg.to_string())
}

/// Convert an `S2Error` (builder / snap / boolean assembly) into a throwable
/// `JsValue`.
pub(crate) fn s2_error_to_js(e: s2rst::s2::builder::S2Error) -> JsValue {
    js_err(e)
}

/// Convert a validation error string into a `JsValue`.
pub(crate) fn validation_error_to_js(e: String) -> JsValue {
    js_err(e)
}
