// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Compact mapping from distinct sequences of values to sequential integer IDs.
//!
//! [`SequenceLexicon`] automatically eliminates duplicate sequences and maps
//! each distinct sequence to a sequentially increasing `u32` identifier. This
//! is useful for compactly representing tuples or ordered collections.
//!
//! Corresponds to C++ `SequenceLexicon<T>`.
//!
//! # Example
//!
//! ```
//! use s2rst::s2::sequence_lexicon::SequenceLexicon;
//!
//! let mut lex = SequenceLexicon::new();
//! let pets = vec!["cat", "dog", "parrot"];
//! let pets_id = lex.add(&pets);
//! assert_eq!(pets_id, lex.add(&pets));
//! let seq: Vec<_> = lex.sequence(pets_id).collect();
//! assert_eq!(seq, vec![&"cat", &"dog", &"parrot"]);
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Maps distinct sequences of values to sequential `u32` identifiers.
///
/// Each distinct sequence is assigned a sequential ID starting from 0.
/// Adding a sequence that already exists returns its existing ID.
/// Sequences are compared element-by-element; order matters.
#[derive(Clone, Debug)]
pub struct SequenceLexicon<T: Eq + Hash + Clone> {
    /// All values, concatenated. Each sequence is a contiguous slice.
    values: Vec<T>,
    /// `begins[i]` is the start index in `values` for sequence `i`.
    /// `begins[i+1]` (or `values.len()` for the last) is the end.
    begins: Vec<usize>,
    /// Maps sequence content hash to its ID for deduplication.
    index: HashMap<SequenceKey, u32>,
}

/// A hashable key for a sequence, storing the actual elements.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SequenceKey(Vec<u8>);

impl Hash for SequenceKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: Eq + Hash + Clone> SequenceLexicon<T> {
    /// Creates a new empty lexicon.
    pub fn new() -> Self {
        SequenceLexicon {
            values: Vec::new(),
            begins: vec![0],
            index: HashMap::new(),
        }
    }

    /// Computes a byte key for the sequence at the given range in `values`.
    fn make_key(values: &[T]) -> SequenceKey {
        // Hash each element to create a canonical byte representation.
        use std::collections::hash_map::DefaultHasher;
        let mut bytes = Vec::with_capacity(values.len() * 8 + 4);
        // Include length to distinguish e.g. [] from [[]]
        bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
        for v in values {
            let mut hasher = DefaultHasher::new();
            v.hash(&mut hasher);
            bytes.extend_from_slice(&hasher.finish().to_le_bytes());
        }
        SequenceKey(bytes)
    }

    /// Adds a sequence to the lexicon if not already present, returning its ID.
    ///
    /// IDs are assigned sequentially starting from 0.
    #[expect(clippy::cast_possible_truncation, reason = "IDs always fit in u32")]
    pub fn add(&mut self, sequence: &[T]) -> u32 {
        let key = Self::make_key(sequence);

        // Check if we already have this sequence.
        if let Some(&existing_id) = self.index.get(&key) {
            // Verify it's actually equal (hash collisions).
            let existing = self.sequence_slice(existing_id);
            if existing.len() == sequence.len()
                && existing.iter().zip(sequence.iter()).all(|(a, b)| a == b)
            {
                return existing_id;
            }
        }

        // Append values.
        self.values.extend_from_slice(sequence);
        self.begins.push(self.values.len());
        let id = (self.begins.len() - 2) as u32;
        self.index.insert(key, id);
        id
    }

    /// Returns the number of distinct sequences in the lexicon.
    #[expect(clippy::cast_possible_truncation, reason = "IDs always fit in u32")]
    pub fn size(&self) -> u32 {
        (self.begins.len() - 1) as u32
    }

    /// Returns an iterator over the values in the sequence with the given ID.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of range.
    pub fn sequence(&self, id: u32) -> impl Iterator<Item = &T> {
        let slice = self.sequence_slice(id);
        slice.iter()
    }

    /// Returns the sequence as a slice.
    fn sequence_slice(&self, id: u32) -> &[T] {
        let begin = self.begins[id as usize];
        let end = self.begins[id as usize + 1];
        &self.values[begin..end]
    }

    /// Removes all sequences from the lexicon.
    pub fn clear(&mut self) {
        self.values.clear();
        self.begins.clear();
        self.begins.push(0);
        self.index.clear();
    }
}

impl<T: Eq + Hash + Clone> Default for SequenceLexicon<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: verify that a sequence in the lexicon matches expected values.
    fn expect_sequence<T: Eq + Hash + Clone + std::fmt::Debug>(
        expected: &[T],
        lex: &SequenceLexicon<T>,
        id: u32,
    ) {
        let actual: Vec<&T> = lex.sequence(id).collect();
        assert_eq!(
            expected.len(),
            actual.len(),
            "sequence {id} length mismatch"
        );
        for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
            assert_eq!(e, *a, "sequence {id} element {i} mismatch");
        }
    }

    // ─── C++ sequence_lexicon_test.cc ports ──────────────────────────────

    #[test]
    fn test_int64() {
        // C++ TEST(SequenceLexicon, int64_t)
        let mut lex = SequenceLexicon::new();
        assert_eq!(0, lex.add(&[]));
        assert_eq!(1, lex.add(&[5_i64]));
        assert_eq!(0, lex.add(&[]));
        assert_eq!(2, lex.add(&[5_i64, 5]));
        assert_eq!(3, lex.add(&[5_i64, 0, -3]));
        assert_eq!(1, lex.add(&[5_i64]));
        assert_eq!(4, lex.add(&[0x7fffffffffffffff_i64]));
        assert_eq!(3, lex.add(&[5_i64, 0, -3]));
        assert_eq!(0, lex.add(&[]));
        assert_eq!(5, lex.size());
        expect_sequence::<i64>(&[], &lex, 0);
        expect_sequence(&[5_i64], &lex, 1);
        expect_sequence(&[5_i64, 5], &lex, 2);
        expect_sequence(&[5_i64, 0, -3], &lex, 3);
        expect_sequence(&[0x7fffffffffffffff_i64], &lex, 4);
    }

    #[test]
    fn test_clear() {
        // C++ TEST(SequenceLexicon, Clear)
        let mut lex = SequenceLexicon::new();
        assert_eq!(0, lex.add(&[1_i64]));
        assert_eq!(1, lex.add(&[2_i64]));
        lex.clear();
        assert_eq!(0, lex.add(&[2_i64]));
        assert_eq!(1, lex.add(&[1_i64]));
    }

    #[test]
    fn test_copy_constructor() {
        // C++ TEST(SequenceLexicon, CopyConstructor)
        let mut original = SequenceLexicon::new();
        assert_eq!(0, original.add(&[1_i64, 2]));
        let mut lex = original.clone();
        drop(original);
        assert_eq!(1, lex.add(&[3_i64, 4]));
        expect_sequence(&[1_i64, 2], &lex, 0);
        expect_sequence(&[3_i64, 4], &lex, 1);
    }

    #[test]
    fn test_move_constructor() {
        // C++ TEST(SequenceLexicon, MoveConstructor)
        let mut original = SequenceLexicon::new();
        assert_eq!(0, original.add(&[1_i64, 2]));
        let mut lex = original; // move
        assert_eq!(1, lex.add(&[3_i64, 4]));
        expect_sequence(&[1_i64, 2], &lex, 0);
        expect_sequence(&[3_i64, 4], &lex, 1);
    }

    #[test]
    fn test_copy_assignment_operator() {
        // C++ TEST(SequenceLexicon, CopyAssignmentOperator)
        let mut original = SequenceLexicon::new();
        assert_eq!(0, original.add(&[1_i64, 2]));
        let mut lex = SequenceLexicon::new();
        assert_eq!(0, lex.add(&[3_i64, 4]));
        assert_eq!(1, lex.add(&[5_i64, 6]));
        lex = original.clone();
        drop(original);
        assert_eq!(1, lex.add(&[7_i64, 8]));
        expect_sequence(&[1_i64, 2], &lex, 0);
        expect_sequence(&[7_i64, 8], &lex, 1);
    }

    #[test]
    fn test_move_assignment_operator() {
        // C++ TEST(SequenceLexicon, MoveAssignmentOperator)
        let mut original = SequenceLexicon::new();
        assert_eq!(0, original.add(&[1_i64, 2]));
        let mut lex = SequenceLexicon::new();
        assert_eq!(0, lex.add(&[3_i64, 4]));
        assert_eq!(1, lex.add(&[5_i64, 6]));
        lex = original; // move
        assert_eq!(1, lex.add(&[7_i64, 8]));
        expect_sequence(&[1_i64, 2], &lex, 0);
        expect_sequence(&[7_i64, 8], &lex, 1);
    }

    // ─── Additional Rust-specific tests ─────────────────────────────────

    #[test]
    fn test_string_sequences() {
        let mut lex = SequenceLexicon::new();
        let id0 = lex.add(&["cat".to_string(), "dog".to_string()]);
        let id1 = lex.add(&["parrot".to_string()]);
        assert_eq!(id0, lex.add(&["cat".to_string(), "dog".to_string()]));
        assert_ne!(id0, id1);
        assert_eq!(2, lex.size());

        let seq0: Vec<_> = lex.sequence(id0).collect();
        assert_eq!(seq0, vec!["cat", "dog"]);
        let seq1: Vec<_> = lex.sequence(id1).collect();
        assert_eq!(seq1, vec!["parrot"]);
    }

    #[test]
    fn test_empty_sequence() {
        let mut lex = SequenceLexicon::<i32>::new();
        let id = lex.add(&[]);
        assert_eq!(0, id);
        assert_eq!(0, lex.sequence(id).count());
        assert_eq!(1, lex.size());
    }

    #[test]
    fn test_order_matters() {
        let mut lex = SequenceLexicon::new();
        let id_ab = lex.add(&[1_i32, 2]);
        let id_ba = lex.add(&[2_i32, 1]);
        assert_ne!(id_ab, id_ba);
    }

    #[test]
    fn test_single_element_sequences() {
        let mut lex = SequenceLexicon::new();
        let id0 = lex.add(&[42_i32]);
        let id1 = lex.add(&[43_i32]);
        assert_eq!(id0, lex.add(&[42_i32]));
        assert_ne!(id0, id1);
    }

    #[test]
    #[expect(clippy::cast_sign_loss, reason = "test values are non-negative")]
    fn test_many_sequences() {
        let mut lex = SequenceLexicon::new();
        for i in 0..1000_i32 {
            let id = lex.add(&[i, i + 1]);
            assert_eq!(i as u32, id);
        }
        assert_eq!(1000, lex.size());
        // Re-add all and verify dedup.
        for i in 0..1000_i32 {
            assert_eq!(i as u32, lex.add(&[i, i + 1]));
        }
        assert_eq!(1000, lex.size());
    }

    #[test]
    fn test_default() {
        let lex = SequenceLexicon::<i32>::default();
        assert_eq!(0, lex.size());
    }

    #[test]
    fn test_prefix_not_equal() {
        // [1, 2] and [1, 2, 3] are different sequences.
        let mut lex = SequenceLexicon::new();
        let id_short = lex.add(&[1_i32, 2]);
        let id_long = lex.add(&[1_i32, 2, 3]);
        assert_ne!(id_short, id_long);
    }
}
