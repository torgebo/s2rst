// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original hand-made unit tests for the union model ([`super`]). Written for this crate,
//! not ported from upstream S2. The model's agreement with C++ is checked separately, against
//! the committed vectors, in `tests/union_spec_vs_cpp.rs`.

use super::*;

fn face(f: u8) -> Decomposed {
    Decomposed {
        face: f,
        level: 0,
        path: 0,
    }
}

fn children(d: Decomposed) -> [Decomposed; 4] {
    [0, 1, 2, 3].map(|k| spec::child(d, k))
}

#[test]
fn four_children_in_order_are_a_sibling_group() {
    let [a, b, c, d] = children(face(0));
    assert!(sibling_group(a, b, c, d));
    assert!(!sibling_group(b, a, c, d));
    assert!(!sibling_group(a, b, c, spec::child(d, 0)));
    // Faces have no parent, so no four faces form a group.
    assert!(!sibling_group(face(0), face(1), face(2), face(3)));
}

#[test]
fn normalized_needs_validity_and_no_sibling_group() {
    let kids = children(face(2));
    assert!(valid_union_d(&kids));
    assert!(!normalized_union_d(&kids));
    assert!(normalized_union_d(&kids[..3]));
    // A cell next to one of its descendants is not valid.
    assert!(!valid_union_d(&[face(1), spec::child(face(1), 3)]));
    // Unsorted cells are not valid.
    assert!(!valid_union_d(&[face(3), face(0)]));
}

#[test]
fn coverage_and_queries() {
    let u = [spec::child(face(0), 1), face(4)];
    let (lo, hi) = spec::range(u[0]);
    assert!(covers(&u, lo) && covers(&u, hi));
    assert!(!covers(&u, lo - 1) && !covers(&u, hi + 1));
    assert!(contains_id(&u, spec::child(u[0], 2)));
    assert!(!contains_id(&u, face(0)));
    assert!(intersects_id(&u, face(0)));
    assert!(!intersects_id(&u, face(5)));
    assert_eq!(leaf_count(&u), u128::from(spec::span(1) + spec::span(0)));
    assert_eq!(leaf_count(&[face(0); 6]), 6 * u128::from(spec::pow4(30)));
}

#[test]
fn denorm_level_matches_the_cpp_formula() {
    assert_eq!(denorm_level(29, 1, 3), 30);
    assert_eq!(denorm_level(30, 1, 2), 30);
    assert_eq!(denorm_level(3, 5, 1), 5);
    assert_eq!(denorm_level(6, 2, 3), 8);
    assert_eq!(denorm_level(8, 2, 3), 8);
    assert_eq!(denorm_level(0, 0, 2), 0);
}

#[test]
fn denormalize_replaces_a_cell_by_its_descendants() {
    let leaf_parent = Decomposed {
        face: 1,
        level: 29,
        path: 7,
    };
    let out = denormalize(&[leaf_parent], 1, 3);
    let expected: Vec<u64> = children(leaf_parent).map(spec::compose).to_vec();
    assert_eq!(out, expected);
    assert_eq!(denormalize(&[face(0)], 0, 1), vec![spec::compose(face(0))]);
    assert_eq!(denormalize(&[face(0)], 2, 1).len(), 16);
}

#[test]
fn decode_keeps_valid_unions_verbatim_and_rejects_the_rest() {
    let f0 = spec::compose(face(0));
    let f3 = spec::compose(face(3));
    let bytes = encoding(&[f0, f3]);
    assert_eq!(bytes.len(), 25);
    assert_eq!(decode(&bytes), Some(vec![f0, f3]));
    assert_eq!(decode(&encoding(&[f3, f0])), None);
    assert_eq!(decode(&encoding(&[f0, f0])), None);
    assert_eq!(decode(&encoding(&[0])), None);
    assert_eq!(decode(&encoding(&[])), Some(vec![]));

    // Four siblings stay four siblings.
    let kids: Vec<u64> = children(face(5)).map(spec::compose).to_vec();
    assert_eq!(decode(&encoding(&kids)), Some(kids.clone()));

    // Trailing bytes are ignored; truncation, version 2 and an oversized count are not.
    let mut trailing = bytes.clone();
    trailing.push(0xff);
    assert_eq!(decode(&trailing), Some(vec![f0, f3]));
    assert_eq!(decode(&bytes[..24]), None);
    assert_eq!(decode(&[]), None);
    let mut v2 = encoding(&[]);
    v2[0] = 2;
    assert_eq!(decode(&v2), None);
    let mut too_many = vec![1u8];
    too_many.extend_from_slice(&(MAX_DECODE_CELLS + 1).to_le_bytes());
    assert_eq!(decode(&too_many), None);
}
