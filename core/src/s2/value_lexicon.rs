// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry

//! Compact mapping from distinct values to sequential integer IDs.
//!
//! [`ValueLexicon`] automatically eliminates duplicates and maps each distinct
//! value to a sequentially increasing `u32` identifier. This is useful for
//! intern-style deduplication where values need compact integer references.
//!
//! Corresponds to C++ `ValueLexicon<T>`.
//!
//! # Example
//!
//! ```
//! use s2rst::s2::value_lexicon::ValueLexicon;
//!
//! let mut lex = ValueLexicon::new();
//! let cat_id = lex.add("cat".to_string());
//! assert_eq!(cat_id, lex.add("cat".to_string()));
//! assert_eq!("cat", lex.value(cat_id));
//! ```

use std::collections::HashMap;
use std::hash::Hash;

/// Maps distinct values to sequential `u32` identifiers with deduplication.
///
/// Each distinct value is assigned a sequential ID starting from 0.
/// Adding a value that already exists returns its existing ID.
#[derive(Clone, Debug)]
pub struct ValueLexicon<T: Eq + Hash + Clone> {
    values: Vec<T>,
    index: HashMap<T, u32>,
}

impl<T: Eq + Hash + Clone> ValueLexicon<T> {
    /// Creates a new empty lexicon.
    pub fn new() -> Self {
        ValueLexicon {
            values: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Adds a value to the lexicon if not already present, returning its ID.
    ///
    /// IDs are assigned sequentially starting from 0.
    #[expect(clippy::cast_possible_truncation, reason = "IDs always fit in u32")]
    pub fn add(&mut self, value: T) -> u32 {
        if let Some(&id) = self.index.get(&value) {
            return id;
        }
        let id = self.values.len() as u32;
        self.index.insert(value.clone(), id);
        self.values.push(value);
        id
    }

    /// Returns the number of distinct values in the lexicon.
    #[expect(clippy::cast_possible_truncation, reason = "IDs always fit in u32")]
    pub fn size(&self) -> u32 {
        self.values.len() as u32
    }

    /// Returns the value associated with the given ID.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of range.
    pub fn value(&self, id: u32) -> &T {
        &self.values[id as usize]
    }

    /// Removes all values from the lexicon.
    pub fn clear(&mut self) {
        self.values.clear();
        self.index.clear();
    }
}

impl<T: Eq + Hash + Clone> Default for ValueLexicon<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── C++ value_lexicon_test.cc ports ─────────────────────────────────

    #[test]
    fn test_duplicate_values() {
        // C++ TEST(ValueLexicon, DuplicateValues)
        let mut lex = ValueLexicon::new();
        assert_eq!(0, lex.add(5_i64));
        assert_eq!(1, lex.add(0_i64));
        assert_eq!(1, lex.add(0_i64));
        assert_eq!(2, lex.add(-3_i64));
        assert_eq!(0, lex.add(5_i64));
        assert_eq!(1, lex.add(0_i64));
        assert_eq!(3, lex.add(0x7fffffffffffffff_i64));
        assert_eq!(4, lex.add(-0x8000000000000000_i64));
        assert_eq!(3, lex.add(0x7fffffffffffffff_i64));
        assert_eq!(4, lex.add(-0x8000000000000000_i64));
        assert_eq!(5, lex.size());
        assert_eq!(&5_i64, lex.value(0));
        assert_eq!(&0_i64, lex.value(1));
        assert_eq!(&-3_i64, lex.value(2));
        assert_eq!(&0x7fffffffffffffff_i64, lex.value(3));
        assert_eq!(&-0x8000000000000000_i64, lex.value(4));
    }

    #[test]
    fn test_clear() {
        // C++ TEST(ValueLexicon, Clear)
        let mut lex = ValueLexicon::new();
        assert_eq!(0, lex.add(1_i64));
        assert_eq!(1, lex.add(2_i64));
        assert_eq!(0, lex.add(1_i64));
        lex.clear();
        assert_eq!(0, lex.add(2_i64));
        assert_eq!(1, lex.add(1_i64));
        assert_eq!(0, lex.add(2_i64));
    }

    #[test]
    fn test_copy_constructor() {
        // C++ TEST(ValueLexicon, CopyConstructor)
        let mut original = ValueLexicon::new();
        assert_eq!(0, original.add(5_i64));
        let mut lex = original.clone();
        drop(original);
        assert_eq!(1, lex.add(10_i64));
        assert_eq!(&5_i64, lex.value(0));
        assert_eq!(&10_i64, lex.value(1));
    }

    #[test]
    fn test_move_constructor() {
        // C++ TEST(ValueLexicon, MoveConstructor)
        let mut original = ValueLexicon::new();
        assert_eq!(0, original.add(5_i64));
        let mut lex = original; // move
        assert_eq!(1, lex.add(10_i64));
        assert_eq!(&5_i64, lex.value(0));
        assert_eq!(&10_i64, lex.value(1));
    }

    #[test]
    fn test_copy_assignment_operator() {
        // C++ TEST(ValueLexicon, CopyAssignmentOperator)
        let mut original = ValueLexicon::new();
        assert_eq!(0, original.add(5_i64));
        let mut lex = ValueLexicon::new();
        assert_eq!(0, lex.add(10_i64));
        assert_eq!(1, lex.add(15_i64));
        lex = original.clone();
        drop(original);
        assert_eq!(1, lex.add(20_i64));
        assert_eq!(&5_i64, lex.value(0));
        assert_eq!(&20_i64, lex.value(1));
    }

    #[test]
    fn test_move_assignment_operator() {
        // C++ TEST(ValueLexicon, MoveAssignmentOperator)
        let mut original = ValueLexicon::new();
        assert_eq!(0, original.add(5_i64));
        let mut lex = ValueLexicon::new();
        assert_eq!(0, lex.add(10_i64));
        assert_eq!(1, lex.add(15_i64));
        lex = original; // move
        assert_eq!(1, lex.add(20_i64));
        assert_eq!(&5_i64, lex.value(0));
        assert_eq!(&20_i64, lex.value(1));
    }

    #[test]
    fn test_string_values() {
        // Additional: test with String type
        let mut lex = ValueLexicon::new();
        let cat_id = lex.add("cat".to_string());
        let dog_id = lex.add("dog".to_string());
        assert_eq!(cat_id, lex.add("cat".to_string()));
        assert_ne!(cat_id, dog_id);
        assert_eq!("cat", lex.value(cat_id).as_str());
        assert_eq!("dog", lex.value(dog_id).as_str());
        assert_eq!(2, lex.size());
    }

    #[test]
    fn test_empty_lexicon() {
        let lex = ValueLexicon::<i64>::new();
        assert_eq!(0, lex.size());
    }

    #[test]
    #[expect(clippy::cast_sign_loss, reason = "test values are non-negative")]
    fn test_sequential_ids() {
        let mut lex = ValueLexicon::new();
        for i in 0..100_i32 {
            assert_eq!(i as u32, lex.add(i));
        }
        assert_eq!(100, lex.size());
        for i in 0..100_i32 {
            assert_eq!(&i, lex.value(i as u32));
        }
    }

    #[test]
    fn test_default() {
        let lex = ValueLexicon::<i32>::default();
        assert_eq!(0, lex.size());
    }
}
