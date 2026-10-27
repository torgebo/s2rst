// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::CellUnion`] defects found by, or before, the formal
//! verification of `CellUnion` (specs/002-formal-verify-cell-union). Written for this crate,
//! not ported from upstream S2.

use super::*;

/// A cell at `level` on face 1 (any cell at that level would do).
fn cell_at(level: u8) -> CellId {
    CellId::from_face(1u8).child_begin_at_level(level)
}

/// D-002: `from_range(begin, end)` must cover exactly the leaves `[begin, end)`, as C++
/// `S2CellUnion::FromBeginEnd` does, including near `end` and across a face boundary.
#[test]
#[expect(
    deprecated,
    reason = "the D-002 regression test of the deprecated alias"
)]
fn from_range_covers_exactly_the_range() {
    // Five leaves: one level-29 cell (four leaves) and one more leaf.
    let b = cell_at(30);
    let e = b.advance(5);
    let cu = CellUnion::from_range(b, e);
    assert_eq!(cu.leaf_cells_covered(), 5);
    assert_eq!(cu, CellUnion::from_begin_end(b, e));

    // Across the face 0 / face 1 boundary: no cell may reach past `end`.
    let b = CellId::from_face(0u8).range_max().advance(-2000);
    let e = CellId::from_face(1u8).range_min().advance(2000);
    let cu = CellUnion::from_range(b, e);
    assert_eq!(cu.cell_ids().first().map(|c| c.range_min()), Some(b));
    assert!(cu.cell_ids().iter().all(|c| c.range_max() < e));
    assert_eq!(cu.leaf_cells_covered(), 4001);
    assert!(cu.is_normalized());

    // Empty and whole ranges.
    assert!(CellUnion::from_range(b, b).is_empty());
    assert_eq!(
        CellUnion::from_range(CellId::begin(30u8), CellId::end(30u8)),
        CellUnion::whole_sphere()
    );
}

/// FR-015: C++ `S2CellUnion::Denormalize` (`s2cell_union.cc:215–223`) rounds the level up in
/// `int` and clamps it to `kMaxLevel`; the Rust port must not panic there.
#[test]
fn denormalize_clamps_to_max_level() {
    // Level 29, min_level 1, level_mod 3: (29 − 1) % 3 = 1, so it rounds up to 31 → 30.
    let c = cell_at(29);
    let cu = CellUnion::from_verbatim(vec![c]).denormalize(Level::new(1), 3);
    assert_eq!(cu.cell_ids(), &c.children()[..]);

    // A leaf with min_level 1 and level_mod 2: (30 − 1) % 2 = 1 rounds up to 31 → 30, so the
    // leaf stays as it is.
    let leaf = cell_at(30);
    let cu = CellUnion::from_verbatim(vec![leaf]).denormalize(Level::new(1), 2);
    assert_eq!(cu.cell_ids(), &[leaf]);

    // Every min_level 0–30, level_mod 1–3 and cell level whose output stays small (a gap of at
    // most 2 below min_level, so at most 16 cells). Smaller levels take the
    // `level < min_level` branch, which does no level arithmetic, and their outputs have
    // 4^(min_level − level) cells.
    for min_level in 0..=30u8 {
        for level_mod in 1..=3u8 {
            for level in min_level.saturating_sub(2)..=30 {
                let c = cell_at(level);
                let cu =
                    CellUnion::from_verbatim(vec![c]).denormalize(Level::new(min_level), level_mod);
                assert!(!cu.is_empty());
                for &d in cu.cell_ids() {
                    let l = d.level().as_u8();
                    assert!(c.contains(d), "{c:?} {min_level} {level_mod}: {d:?}");
                    assert!(l >= min_level, "{c:?} {min_level} {level_mod}: level {l}");
                    assert!(
                        l == 30 || (l - min_level) % level_mod == 0,
                        "{c:?} {min_level} {level_mod}: level {l}"
                    );
                }
            }
        }
    }
}

// FR-017: C++ `S2CellUnion::Decode` (`s2cell_union.cc:544–562`) keeps the ids exactly as
// encoded. The Rust port does the same, but rejects input that is not a valid union (a
// documented deliberate deviation: C++ validates nothing, upstream finding U-003).

fn decode(bytes: &[u8]) -> std::io::Result<CellUnion> {
    <CellUnion as crate::s2::encoding::S2Decode>::decode(&mut &bytes[..])
}

/// An encoding built by hand: version 1, the count, then the raw ids.
fn raw_encoding(ids: &[u64]) -> Vec<u8> {
    let mut b = vec![1u8];
    b.extend_from_slice(&(ids.len() as u64).to_le_bytes());
    for id in ids {
        b.extend_from_slice(&id.to_le_bytes());
    }
    b
}

fn assert_rejected(bytes: &[u8], what: &str) {
    let err = decode(bytes).expect_err(what);
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{what}: {err}");
}

#[test]
fn decode_keeps_four_siblings() {
    let kids = cell_at(12).children();
    let mut bytes = Vec::new();
    crate::s2::encoding::S2Encode::encode(&CellUnion::from_verbatim(kids.to_vec()), &mut bytes)
        .unwrap();
    let cu = decode(&bytes).unwrap();
    assert_eq!(cu.cell_ids(), &kids[..]);
}

#[test]
fn decode_rejects_unsorted() {
    let (f0, f1) = (CellId::from_face(0u8).0, CellId::from_face(1u8).0);
    assert_rejected(&raw_encoding(&[f1, f0]), "[face 1, face 0]");
}

#[test]
fn decode_rejects_duplicate() {
    let c = cell_at(7).0;
    assert_rejected(&raw_encoding(&[c, c]), "a duplicate");
}

#[test]
fn decode_rejects_overlap() {
    let p = cell_at(7);
    assert_rejected(&raw_encoding(&[p.0, p.children()[3].0]), "[parent, child]");
    assert_rejected(&raw_encoding(&[p.children()[0].0, p.0]), "[child, parent]");
}

#[test]
fn decode_rejects_invalid_id() {
    assert_rejected(&raw_encoding(&[0]), "id 0");
}

#[test]
fn decode_error_messages_unchanged() {
    let mut v2 = raw_encoding(&[]);
    v2[0] = 2;
    assert_eq!(
        decode(&v2).unwrap_err().to_string(),
        "unsupported encoding version 2"
    );
    let mut too_many = vec![1u8];
    too_many.extend_from_slice(&1_000_001u64.to_le_bytes());
    assert_eq!(
        decode(&too_many).unwrap_err().to_string(),
        "too many cells (1000001; max is 1000000)"
    );
    let (f0, f1) = (CellId::from_face(0u8).0, CellId::from_face(1u8).0);
    assert_eq!(
        decode(&raw_encoding(&[f1, f0])).unwrap_err().to_string(),
        "cell ids are not a valid cell union: unsorted or overlapping"
    );
}

/// Code review of v0.4.0..v0.5.0, finding 5: the guards that keep `from_begin_end` from
/// looping forever and `denormalize` from expanding into billions of cells held only in debug
/// builds (and in the Python and WASM bindings). They now hold in every build: run with
/// `--release` too.
#[test]
#[should_panic(expected = "from_begin_end: begin is neither a leaf cell")]
fn from_begin_end_rejects_id_zero_in_every_build() {
    drop(CellUnion::from_begin_end(
        CellId(0),
        CellId::from_face(1u8).child_begin_at_level(30),
    ));
}

#[test]
#[should_panic(expected = "from_begin_end: begin is after end")]
fn from_begin_end_rejects_reversed_range_in_every_build() {
    let a = CellId::from_face(1u8).child_begin_at_level(30);
    drop(CellUnion::from_begin_end(a.next(), a));
}

#[test]
#[should_panic(expected = "from_min_max: max_id is not a valid leaf cell")]
fn from_min_max_rejects_invalid_max_in_every_build() {
    let a = CellId::from_face(1u8).child_begin_at_level(30);
    drop(CellUnion::from_min_max(a, CellId(0)));
}

#[test]
#[should_panic(expected = "denormalize: level_mod must be 1, 2 or 3")]
fn denormalize_rejects_large_level_mod_in_every_build() {
    let u = CellUnion::from_cell_ids(vec![CellId::from_face(0u8)]);
    drop(u.denormalize(Level::new(0), 200));
}
