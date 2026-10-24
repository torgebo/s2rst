// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Input-domain generators for the harnesses (data-model §2; research R4, R6).
//!
//! Every cell domain is built from the reference model (`spec::compose`), never by assuming
//! the implementation's own `is_valid`, so a harness's domain does not depend on the code
//! under test.

use s2rst::s2::{CellId, Face, Level};

use crate::spec::{self, Decomposed};

/// `Any`: every `u64` bit pattern.
pub fn any_u64() -> CellId {
    CellId(kani::any())
}

/// `Level`: `0..=30`, as a value of the level type.
pub fn any_level() -> Level {
    let l = Level::try_new(kani::any());
    kani::assume(l.is_some());
    l.unwrap()
}

/// `Face`: `0..=5`, as a raw face number.
pub fn any_face() -> u8 {
    let f: u8 = kani::any();
    kani::assume(f <= 5);
    f
}

/// `Face`: `0..=5`, as a value of the face type.
pub fn any_face_t() -> Face {
    Face::from_u8(any_face())
}

/// A well-formed decomposition: `face ≤ 5`, `level ≤ 30`, `path < 4^level`.
pub fn any_decomposed() -> Decomposed {
    let d = Decomposed {
        face: kani::any(),
        level: kani::any(),
        path: kani::any(),
    };
    kani::assume(d.is_well_formed());
    d
}

/// `Valid`: every valid cell, with its decomposition.
pub fn any_valid() -> (CellId, Decomposed) {
    let d = any_decomposed();
    (CellId(spec::compose(d)), d)
}

/// Every valid cell at a fixed `level`.
pub fn any_valid_at(level: u8) -> (CellId, Decomposed) {
    let d = Decomposed {
        face: any_face(),
        level,
        path: kani::any(),
    };
    kani::assume(d.path < spec::pow4(u32::from(level)));
    (CellId(spec::compose(d)), d)
}

/// Every valid cell on a fixed `face` (the `__f<K>` splits).
#[cfg_attr(
    not(feature = "slow"),
    allow(dead_code, reason = "only the slow-tier harnesses split by face")
)]
pub fn any_valid_on_face(face: u8) -> (CellId, Decomposed) {
    let d = Decomposed {
        face,
        level: kani::any(),
        path: kani::any(),
    };
    kani::assume(d.is_well_formed());
    (CellId(spec::compose(d)), d)
}

/// `ValidOrEnd`: a valid cell, `end(l)` for any level `l`, or `sentinel`.
pub fn any_valid_or_end() -> CellId {
    let k: u8 = kani::any();
    kani::assume(k < 3);
    match k {
        0 => any_valid().0,
        1 => {
            let l: u8 = kani::any();
            kani::assume(l <= spec::MAX_LEVEL);
            CellId(spec::end(l))
        }
        _ => CellId(spec::SENTINEL),
    }
}

/// `Text(L)`: calls `f` with every byte string of exactly `L` bytes, as a `&str`.
///
/// This is a superset of every `&str` of `L` bytes. The three text parsers read only bytes
/// (`len`, `as_bytes`, byte comparisons, `from_str_radix`, `bytes().all(..)`) and never
/// decode `char`s, so their result on a `&str` is a function of its bytes, and a proof over
/// all byte strings covers every `&str` (research R6). Kani does not model UTF-8 validity (a
/// safety invariant of `str`, not a validity invariant), so the invalid members of the
/// superset can only add counterexamples, never hide one; a counterexample counts as a defect
/// only if its bytes are valid UTF-8.
#[allow(
    unsafe_code,
    reason = "byte-superset text domain for the parsers, research R6; the only unsafe in verify/"
)]
pub fn with_text<const L: usize>(f: impl FnOnce(&str)) {
    let bytes: [u8; L] = kani::any();
    // SAFETY: deliberately not upheld for every input; see the doc comment. The parsers under
    // test never depend on UTF-8 validity, and Kani does not treat it as undefined behaviour.
    let s = unsafe { std::str::from_utf8_unchecked(&bytes) };
    f(s);
}
