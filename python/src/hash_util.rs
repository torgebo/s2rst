// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Stable hash of an f64 sequence via the underlying bit representation.
///
/// Hashing the bits (rather than `f64` directly, which doesn't implement
/// `Hash`) gives deterministic results consistent with bit-equal `==`. NaN
/// values hash to whatever their bit pattern dictates; the caller is
/// responsible for the standard caveat that two NaNs are never `==`.
pub(crate) fn hash_f64s(vals: &[f64]) -> u64 {
    let mut h = DefaultHasher::new();
    for v in vals {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}
