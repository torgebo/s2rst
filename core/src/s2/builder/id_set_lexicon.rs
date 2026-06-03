// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

#![expect(
    clippy::cast_sign_loss,
    reason = "i32 sequence IDs used as Vec indices"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "sequence ID (i32) <-> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for sequence IDs — always in range"
)]
// IdSetLexicon: compact storage for sets of integer IDs.
//
// Maps IdSetId → set of i32 values, with deduplication so identical sets
// share the same ID. Single-element sets are encoded inline as negative IDs.

use std::collections::HashMap;

/// Compact storage for sets of integer IDs with deduplication.
///
/// Single-element sets are stored inline: the ID is `-(value + 1)`.
/// Multi-element sets are stored in a vector and referenced by non-negative ID.
/// The empty set has ID `i32::MIN`.
#[derive(Clone, Debug, PartialEq)]
pub struct IdSetLexicon {
    /// Maps sorted set contents to its index in `sets`.
    index: HashMap<Vec<i32>, i32>,
    /// Stored multi-element sets.
    sets: Vec<Vec<i32>>,
}

/// The ID representing the empty set.
pub const EMPTY_SET_ID: i32 = i32::MIN;

impl IdSetLexicon {
    /// Creates a new empty lexicon.
    pub fn new() -> Self {
        IdSetLexicon {
            index: HashMap::new(),
            sets: Vec::new(),
        }
    }

    /// Adds a set and returns its ID. Identical sets return the same ID.
    pub fn add_set(&mut self, values: &[i32]) -> i32 {
        if values.is_empty() {
            return EMPTY_SET_ID;
        }

        // Single-element optimization: encode as -(value + 1).
        if values.len() == 1 {
            return -(values[0] + 1);
        }

        // Sort and dedup for canonical form.
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();

        // After dedup, check if it collapsed to a single element.
        if sorted.len() == 1 {
            return -(sorted[0] + 1);
        }

        if let Some(&id) = self.index.get(&sorted) {
            return id;
        }

        let id = self.sets.len() as i32;
        self.index.insert(sorted.clone(), id);
        self.sets.push(sorted);
        id
    }

    /// Retrieves the set for a given ID.
    pub fn id_set(&self, id: i32) -> Vec<i32> {
        if id == EMPTY_SET_ID {
            return Vec::new();
        }
        if id < 0 {
            // Single-element: decode from -(value + 1).
            return vec![-(id + 1)];
        }
        self.sets.get(id as usize).cloned().unwrap_or_default()
    }
}

impl Default for IdSetLexicon {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    fn test_empty_set() {
        let mut lex = IdSetLexicon::new();
        let id = lex.add_set(&[]);
        assert_eq!(id, EMPTY_SET_ID);
        assert_eq!(lex.id_set(id), Vec::<i32>::new());
    }

    #[test]
    fn test_single_element() {
        let mut lex = IdSetLexicon::new();
        let id = lex.add_set(&[42]);
        assert!(id < 0);
        assert_eq!(lex.id_set(id), vec![42]);
    }

    #[test]
    fn test_single_element_zero() {
        let mut lex = IdSetLexicon::new();
        let id = lex.add_set(&[0]);
        assert_eq!(id, -1); // -(0 + 1) = -1
        assert_eq!(lex.id_set(id), vec![0]);
    }

    #[test]
    fn test_multi_element() {
        let mut lex = IdSetLexicon::new();
        let id = lex.add_set(&[3, 1, 2]);
        assert!(id >= 0);
        assert_eq!(lex.id_set(id), vec![1, 2, 3]); // sorted
    }

    #[test]
    fn test_dedup() {
        let mut lex = IdSetLexicon::new();
        let id1 = lex.add_set(&[1, 2, 3]);
        let id2 = lex.add_set(&[3, 1, 2]); // same set, different order
        let id3 = lex.add_set(&[1, 2, 3, 2]); // with duplicates
        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_different_sets_get_different_ids() {
        let mut lex = IdSetLexicon::new();
        let id1 = lex.add_set(&[1, 2]);
        let id2 = lex.add_set(&[1, 3]);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_single_element_dedup_after_sort() {
        let mut lex = IdSetLexicon::new();
        // Set with duplicates that collapses to single element.
        let id = lex.add_set(&[5, 5, 5]);
        assert!(id < 0);
        assert_eq!(lex.id_set(id), vec![5]);
    }

    // ─── Property tests ─────────────────────────────────────────────────

    // Note: IdSetLexicon's single-element encoding -(value+1) only works
    // for non-negative values. This matches the intended use — stored values
    // are always edge IDs or labels, which are non-negative. All property
    // tests below use u16 to stay in the valid range.

    /// Roundtrip: `add_set` followed by `id_set` returns the sorted, deduped
    /// version of the input.
    #[quickcheck]
    fn prop_roundtrip(values: Vec<u16>) -> bool {
        let mut lex = IdSetLexicon::new();
        let vals: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let id = lex.add_set(&vals);
        let result = lex.id_set(id);

        let mut expected = vals;
        expected.sort_unstable();
        expected.dedup();
        result == expected
    }

    /// Single-element sets are always encoded as negative IDs.
    #[quickcheck]
    fn prop_single_element_negative_id(v: u16) -> bool {
        let mut lex = IdSetLexicon::new();
        let id = lex.add_set(&[i32::from(v)]);
        id < 0 && lex.id_set(id) == vec![i32::from(v)]
    }

    /// Single-element encoding is -(value + 1).
    #[quickcheck]
    fn prop_single_element_encoding(v: u16) -> bool {
        let mut lex = IdSetLexicon::new();
        let val = i32::from(v);
        let id = lex.add_set(&[val]);
        id == -(val + 1)
    }

    /// Adding the same set twice returns the same ID (dedup).
    #[quickcheck]
    fn prop_dedup_same_id(values: Vec<u16>) -> bool {
        if values.is_empty() {
            return true; // empty set always returns EMPTY_SET_ID
        }
        let mut lex = IdSetLexicon::new();
        let vals: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let id1 = lex.add_set(&vals);
        let id2 = lex.add_set(&vals);
        id1 == id2
    }

    /// Adding a permutation of the same set returns the same ID.
    #[quickcheck]
    fn prop_dedup_permutation(mut values: Vec<u16>) -> bool {
        if values.len() < 2 {
            return true;
        }
        let mut lex = IdSetLexicon::new();
        let vals1: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let id1 = lex.add_set(&vals1);

        // Reverse the values and add again.
        values.reverse();
        let vals2: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let id2 = lex.add_set(&vals2);

        id1 == id2
    }

    /// Different sets with distinct sorted contents get different IDs.
    #[quickcheck]
    fn prop_different_sets_different_ids(a: u16, b: u16) -> bool {
        if a == b {
            return true;
        }
        let mut lex = IdSetLexicon::new();
        let id1 = lex.add_set(&[i32::from(a)]);
        let id2 = lex.add_set(&[i32::from(b)]);
        id1 != id2
    }

    /// Empty set ID is always `EMPTY_SET_ID` regardless of lexicon state.
    #[quickcheck]
    fn prop_empty_set_stable(values: Vec<u16>) -> bool {
        let mut lex = IdSetLexicon::new();
        // Add some noise first.
        for &v in &values {
            lex.add_set(&[i32::from(v)]);
        }
        let id = lex.add_set(&[]);
        id == EMPTY_SET_ID && lex.id_set(id).is_empty()
    }
}
