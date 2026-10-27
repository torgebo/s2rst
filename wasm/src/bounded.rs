// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! The JavaScript side of the core's bounded types: one conversion per kind (spec 009, FR-007).
//!
//! A method that takes a cell level, a cube face, a level modulus or a cell vertex or edge index
//! declares its parameter as `f64`, the only primitive through
//! which a JavaScript `number` arrives unchanged: an integer parameter passes through ToInt32,
//! and wasm-bindgen's `u8` through `as u8`, so 286 became level 30 and `NaN` level 0. The
//! TypeScript type stays `number`. A value that is not an integer, or is out of the kind's
//! range, is returned as an error through the crate's one funnel ([`js_err`]), which JavaScript
//! sees as a thrown exception carrying the core's `OutOfRange` text ("level must be in 0..=30,
//! got 31"). No such value reaches the core, so none can panic there and abort the instance.

use s2rst::s2::{BoundedKind, CellEdge, CellVertex, Face, Level, LevelMod, OutOfRange};
use wasm_bindgen::JsValue;

use crate::error::js_err;

/// `v` as JavaScript prints it (`NaN`, `Infinity`, `-Infinity`, `3.5`, `286`).
fn js_number(v: f64) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v.is_infinite() {
        if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
    } else {
        format!("{v}")
    }
}

/// The integer that `v` holds, or an error naming `kind` and its range if `v` is `NaN`,
/// infinite or not a whole number. Values beyond 2⁵³ are not exact in a `number`, and
/// all of them are outside every kind's range, so they are rejected as out of range.
fn integer_arg(v: f64, kind: BoundedKind) -> Result<i64, JsValue> {
    const EXACT: f64 = 9_007_199_254_740_992.0; // 2^53
    if v.is_finite() && v.fract() == 0.0 && v.abs() <= EXACT {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a whole number of magnitude at most 2^53 is exact in i64"
        )]
        Ok(v as i64)
    } else if v.is_finite() && v.fract() == 0.0 {
        let range = kind.range();
        Err(js_err(format!(
            "{} must be in {}..={}, got {}",
            kind.name(),
            range.start(),
            range.end(),
            js_number(v)
        )))
    } else {
        let range = kind.range();
        Err(js_err(format!(
            "{} must be an integer in {}..={}, got {}",
            kind.name(),
            range.start(),
            range.end(),
            js_number(v)
        )))
    }
}

/// Converts a JavaScript number to the bounded type `T` of `kind`.
fn bounded<T>(v: f64, kind: BoundedKind) -> Result<T, JsValue>
where
    T: TryFrom<i64, Error = OutOfRange>,
{
    T::try_from(integer_arg(v, kind)?).map_err(js_err)
}

/// A cell level argument: an integer in `0..=30`.
pub(crate) fn level_arg(v: f64) -> Result<Level, JsValue> {
    bounded(v, BoundedKind::Level)
}

/// A cube face argument: an integer in `0..=5`.
pub(crate) fn face_arg(v: f64) -> Result<Face, JsValue> {
    bounded(v, BoundedKind::Face)
}

/// A cell vertex index argument: an integer in `0..=3`.
pub(crate) fn cell_vertex_arg(v: f64) -> Result<CellVertex, JsValue> {
    bounded(v, BoundedKind::CellVertex)
}

/// A cell edge index argument: an integer in `0..=3`.
pub(crate) fn cell_edge_arg(v: f64) -> Result<CellEdge, JsValue> {
    bounded(v, BoundedKind::CellEdge)
}

/// A level modulus argument: an integer in `1..=3`.
pub(crate) fn level_mod_arg(v: f64) -> Result<LevelMod, JsValue> {
    bounded(v, BoundedKind::LevelMod)
}

/// The one relation between two levels that their types cannot carry: an error unless
/// `min_level <= max_level`. The core panics on it when the levels are used (spec 009, B12),
/// which would abort the instance, so the bindings check it first.
pub(crate) fn check_level_order(min_level: Level, max_level: Level) -> Result<(), JsValue> {
    if min_level <= max_level {
        Ok(())
    } else {
        Err(js_err(format!(
            "min_level ({min_level}) must not exceed max_level ({max_level})"
        )))
    }
}
