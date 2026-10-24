// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::CellId;

// The three `CellId` text parsers (`from_token`, `from_debug_string` and `FromStr`) take
// untrusted text (Constitution II; spec 001, FR-017). Beyond never panicking/aborting/hanging:
// a decoded token re-encodes and decodes back to the same id, an accepted debug string is
// the canonical spelling of its cell, and `FromStr` agrees with `from_debug_string`.
fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let t = CellId::from_token(s);
    assert_eq!(CellId::from_token(&t.to_token()), t, "token round trip for {s:?}");
    let d = CellId::from_debug_string(s);
    if let Some(c) = d {
        assert_eq!(c.to_debug_string(), s, "debug string of {s:?} is not canonical");
    }
    assert_eq!(s.parse::<CellId>(), d.ok_or(()), "FromStr disagrees for {s:?}");
});
