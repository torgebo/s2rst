// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for the singleton set-id helpers
//! ([`IdSetLexicon::add_singleton`] / [`IdSetLexicon::singleton_value`]) and
//! the encoding convention they guard. This port encodes singleton sets as
//! `-(value + 1)`, while upstream C++ encodes them as the raw value itself —
//! so ported code must never push a raw id as a set id (see BUG.md §2,
//! Phase 5: the `GraphEdgeClipper` emission did exactly that, silently losing
//! all input-edge attribution). Written for this crate, not ported from
//! upstream S2.

use super::{EMPTY_SET_ID, IdSetLexicon};
use quickcheck_macros::quickcheck;

/// `singleton_value` is the inverse of `add_singleton`.
#[quickcheck]
fn prop_singleton_roundtrip(v: u16) -> bool {
    let v = i32::from(v);
    IdSetLexicon::singleton_value(IdSetLexicon::add_singleton(v)) == v
}

/// A singleton id decodes to its one-element set on a *fresh* lexicon — no
/// instance state is required (singletons are encoded inline, never stored).
#[quickcheck]
fn prop_singleton_decodes_without_instance_state(v: u16) -> bool {
    let v = i32::from(v);
    let lex = IdSetLexicon::new();
    lex.id_set(IdSetLexicon::add_singleton(v)) == vec![v]
}

/// The instance encoder agrees with the instance-free helper for singletons.
#[quickcheck]
fn prop_add_set_singleton_matches_add_singleton(v: u16) -> bool {
    let v = i32::from(v);
    let mut lex = IdSetLexicon::new();
    lex.add_set(&[v]) == IdSetLexicon::add_singleton(v)
}

/// Singleton ids occupy their own id space: always negative (stored
/// multi-element sets use non-negative ids) and never the empty-set id.
#[quickcheck]
fn prop_singleton_ids_disjoint_from_stored_and_empty(v: u16) -> bool {
    let id = IdSetLexicon::add_singleton(i32::from(v));
    id < 0 && id != EMPTY_SET_ID
}

/// The C++-convention trap, as a test: pushing a *raw* non-negative id as a
/// set id does NOT decode to the singleton of that value in this port. Any
/// ported code relying on the C++ "singleton == raw value" encoding must go
/// through `add_singleton` instead.
#[quickcheck]
fn prop_raw_ids_are_not_valid_singleton_encodings(v: u16) -> bool {
    let v = i32::from(v);
    let lex = IdSetLexicon::new();
    lex.id_set(v) != vec![v]
}
