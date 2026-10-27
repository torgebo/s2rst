// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bounded types: values checked once, when they are created.
//!
//! A bounded type holds an integer restricted to a range ([`Level`](super::Level), 0..=30) or
//! one of a small set ([`Face`](super::Face)). A function that takes one needs no check of its
//! own and documents no panic for that argument. A raw integer becomes a bounded type through
//! `TryFrom`, which returns [`OutOfRange`] for a value outside the range, or through the type's
//! `new`, which panics there (and, in a constant context, is rejected when the code is
//! compiled). No bounded type converts from a raw integer through `From`.
//!
//! This module holds the one error of every such conversion. The record of which parameters
//! are bounded, and why, is `BOUNDED_TYPES.md` at the repository root.

use std::fmt;
use std::ops::RangeInclusive;

/// What a bounded conversion was converting: the kind of value, its name and its range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BoundedKind {
    /// A cell level ([`Level`](super::Level)), 0..=30.
    Level,
    /// A cube face ([`Face`](super::Face)), 0..=5.
    Face,
    /// A level modulus, 1..=3.
    LevelMod,
    /// The index of a cell vertex, 0..=3.
    CellVertex,
    /// The index of a cell edge ([`CellEdge`](super::CellEdge)), 0..=3.
    CellEdge,
}

impl BoundedKind {
    /// The kind's name, as error messages use it (for example `"level"`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            BoundedKind::Level => "level",
            BoundedKind::Face => "face",
            BoundedKind::LevelMod => "level_mod",
            BoundedKind::CellVertex => "cell vertex index",
            BoundedKind::CellEdge => "cell edge index",
        }
    }

    /// The values the kind accepts.
    #[must_use]
    pub const fn range(self) -> RangeInclusive<i64> {
        match self {
            BoundedKind::Level => 0..=30,
            BoundedKind::Face => 0..=5,
            BoundedKind::LevelMod => 1..=3,
            BoundedKind::CellVertex | BoundedKind::CellEdge => 0..=3,
        }
    }
}

/// The error of a bounded conversion: the value was outside the kind's range.
///
/// Its `Display` names the kind, the range and the value, for example
/// "level must be in 0..=30, got 31". The Python and WebAssembly bindings raise it as their
/// languages' native errors, with the same text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfRange {
    kind: BoundedKind,
    value: i64,
}

impl OutOfRange {
    /// The error for `value`, which is outside `kind`'s range.
    pub(crate) const fn new(kind: BoundedKind, value: i64) -> Self {
        OutOfRange { kind, value }
    }

    /// What was being converted.
    #[must_use]
    pub const fn kind(self) -> BoundedKind {
        self.kind
    }

    /// The value that was rejected.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.value
    }

    /// The values that would have been accepted.
    #[must_use]
    pub const fn range(self) -> RangeInclusive<i64> {
        self.kind.range()
    }
}

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let range = self.range();
        write!(
            f,
            "{} must be in {}..={}, got {}",
            self.kind.name(),
            range.start(),
            range.end(),
            self.value
        )
    }
}

impl std::error::Error for OutOfRange {}

/// A level modulus in the range `1..=3`: a covering uses only the levels `min_level + k·m`.
///
/// A bounded type, like [`Level`](super::Level). [`RegionCoverer`], the term indexer's options
/// and [`CellUnion::denormalize`](super::CellUnion::denormalize) take one, so none of them can be
/// handed a modulus outside the range: upstream checks it with `ABSL_DCHECK` and then clamps it
/// in its coverer's setter, and divides by it elsewhere (a modulus of 0 divided by zero in the
/// term indexer before spec 009). Build one with [`LevelMod::new`] for a literal (checked at
/// compile time in a constant context) or [`LevelMod::try_from`](TryFrom::try_from) for a
/// value computed at run time.
///
/// [`RegionCoverer`]: super::region_coverer::RegionCoverer
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LevelMod(u8);

impl LevelMod {
    /// The default modulus, 1: every level between the minimum and the maximum.
    pub const DEFAULT: LevelMod = LevelMod(1);

    /// Creates a level modulus.
    ///
    /// # Panics
    ///
    /// Panics if `v` is not 1, 2 or 3. In a constant context that is rejected when the code is
    /// compiled.
    #[track_caller]
    #[must_use]
    pub const fn new(v: u8) -> Self {
        assert!(v >= 1 && v <= 3, "level_mod must be in 1..=3");
        LevelMod(v)
    }

    /// The modulus, 1, 2 or 3.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    /// The number of descendants of a cell, `get()` levels down: 4, 16 or 64.
    #[must_use]
    pub const fn branching_factor(self) -> u32 {
        1 << (2 * self.0 as u32)
    }
}

impl Default for LevelMod {
    fn default() -> Self {
        LevelMod::DEFAULT
    }
}

impl fmt::Display for LevelMod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for LevelMod {
    type Error = OutOfRange;

    /// The modulus `v`, or [`OutOfRange`] outside `1..=3`.
    fn try_from(v: i64) -> Result<Self, OutOfRange> {
        match u8::try_from(v) {
            Ok(m @ 1..=3) => Ok(LevelMod(m)),
            _ => Err(OutOfRange::new(BoundedKind::LevelMod, v)),
        }
    }
}

impl TryFrom<u8> for LevelMod {
    type Error = OutOfRange;

    /// The modulus `v`, or [`OutOfRange`] outside `1..=3`.
    fn try_from(v: u8) -> Result<Self, OutOfRange> {
        LevelMod::try_from(i64::from(v))
    }
}

impl TryFrom<usize> for LevelMod {
    type Error = OutOfRange;

    /// The modulus `v`, or [`OutOfRange`] outside `1..=3`. A value beyond `i64` is reported as
    /// `i64::MAX`.
    fn try_from(v: usize) -> Result<Self, OutOfRange> {
        LevelMod::try_from(i64::try_from(v).unwrap_or(i64::MAX))
    }
}

/// Deserialises the form that `Serialize` writes (a bare number in JSON), and accepts only a
/// modulus in `1..=3`, as `Level`'s does: anything else is an error carrying the
/// [`OutOfRange`] text.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for LevelMod {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        #[derive(serde::Deserialize)]
        #[serde(rename = "LevelMod")]
        struct Raw(u8);

        let Raw(v) = Raw::deserialize(deserializer)?;
        LevelMod::try_from(v).map_err(D::Error::custom)
    }
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod bounded_tests;
