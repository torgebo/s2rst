// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The deductive track of spec 003: restated copies of the library's variable-length codecs,
//! proved with Verus for every length (FR-010), and linked to the library by bounded
//! agreement (`verify/src/cd_agree.rs`), tested agreement (`tests/agree.rs`) and the drift
//! guard (`tests/drift.rs`, `mirrors.toml`) (FR-011).
//!
//! Verify with `make verify-deductive` from the repository root, never with an uncapped
//! `verus`. The rules for restated code are in
//! `specs/003-formal-verify-decoders/contracts/deductive-conventions.md`.

#![allow(
    unused_variables,
    unused_imports,
    unused_braces,
    reason = "values and imports used only by proofs, and the `global` directive's expansion, are erased outside Verus"
)]
#![allow(
    clippy::assign_op_pattern,
    clippy::question_mark,
    clippy::implicit_saturating_sub,
    clippy::len_zero,
    reason = "the restated code keeps the library's statements and the forms Verus proves most directly (`x = x + 1`, an explicit `match` on `Option`); rewriting them for style would change the proofs"
)]

use vstd::prelude::*;

verus! {
// The library targets 64-bit platforms here: offsets and lengths are `u64`s cast to `usize`.
global size_of usize == 8;
}

pub mod cellid_vector;
pub mod fixed_width;
pub mod string_vector;
pub mod uint_vector;
pub mod varint;
