// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Sanity tests for the union model (`s2rst_verify::union_spec`) against the committed C++
//! oracle vectors (spec 002, research R13; data-model §10). The Kani proofs show that the
//! implementation agrees with the model within their bounds; these tests show, by sampling
//! larger unions, that the model agrees with upstream C++. Together: C++ oracle ⇔ model
//! (sampled) ⇔ implementation (proved within bounds).
//!
//! Covered sets are compared as merged leaf-index intervals, computed here from the cell
//! model only.

use s2rst_verify::spec::{self, Decomposed};
use s2rst_verify::union_spec;

const CSV: &str = include_str!("../../core/tests/data/cpp_cellunion_vectors.csv");

/// Lines of a named `# SECTION` (stops at the next `#` line or a blank line).
fn section_lines(name: &str) -> Vec<&'static str> {
    let header = format!("# {name}");
    let mut in_section = false;
    let mut lines = Vec::new();
    for line in CSV.lines() {
        if line == header {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with('#') || line.is_empty() {
                break;
            }
            lines.push(line);
        }
    }
    assert!(!lines.is_empty(), "section {name} is empty or missing");
    lines
}

fn fields(line: &str) -> Vec<&str> {
    line.split(',').collect()
}

fn hex64(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or_else(|e| panic!("bad id {s:?}: {e}"))
}

/// A `;`-separated id list; the empty field is the empty list.
fn ids(s: &str) -> Vec<u64> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(';').map(hex64).collect()
    }
}

fn bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn flag(s: &str) -> bool {
    match s {
        "0" => false,
        "1" => true,
        _ => panic!("bad flag {s:?}"),
    }
}

fn decomposed(ids: &[u64]) -> Vec<Decomposed> {
    ids.iter()
        .map(|&x| spec::decompose(x).unwrap_or_else(|| panic!("{x:#x} is not spec::valid")))
        .collect()
}

/// The covered set of valid ids as sorted, merged, inclusive leaf-index intervals.
fn intervals(ids: &[u64]) -> Vec<(u64, u64)> {
    let mut r: Vec<(u64, u64)> = decomposed(ids).into_iter().map(spec::range).collect();
    r.sort_unstable();
    merge(r)
}

fn merge(sorted: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    let mut out: Vec<(u64, u64)> = Vec::new();
    for (lo, hi) in sorted {
        match out.last_mut() {
            Some(last) if lo <= last.1 + 1 => last.1 = last.1.max(hi),
            _ => out.push((lo, hi)),
        }
    }
    out
}

fn set_union(a: &[(u64, u64)], b: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut all = [a, b].concat();
    all.sort_unstable();
    merge(all)
}

fn set_intersection(a: &[(u64, u64)], b: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for &(alo, ahi) in a {
        for &(blo, bhi) in b {
            let (lo, hi) = (alo.max(blo), ahi.min(bhi));
            if lo <= hi {
                out.push((lo, hi));
            }
        }
    }
    out.sort_unstable();
    merge(out)
}

fn set_difference(a: &[(u64, u64)], b: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for &(alo, ahi) in a {
        let mut lo = alo;
        for &(blo, bhi) in b {
            if bhi < lo || blo > ahi {
                continue;
            }
            if blo > lo {
                out.push((lo, blo - 1));
            }
            lo = bhi.saturating_add(1);
            if lo > ahi {
                break;
            }
        }
        if lo <= ahi {
            out.push((lo, ahi));
        }
    }
    merge(out)
}

/// The leaf index of a leaf cell, or `6·4^30` for `End(30)`.
fn leaf_index_or_end(id: u64) -> u64 {
    if id == spec::end(30) {
        6 * spec::pow4(30)
    } else {
        let d = spec::decompose(id).unwrap();
        assert_eq!(d.level, 30, "{id:#x} is not a leaf");
        spec::leaf_index(d)
    }
}

#[test]
fn normalize_matches_cpp() {
    let mut normalized_inputs = 0;
    for line in section_lines("CELLUNION_NORMALIZE") {
        let f = fields(line);
        let (input, output) = (ids(f[0]), ids(f[1]));
        assert!(union_spec::normalized_union(&output), "{line}");
        assert_eq!(intervals(&output), intervals(&input), "{line}");
        if union_spec::normalized_union(&input) {
            assert_eq!(input, output, "a normalised input is a fixed point: {line}");
            normalized_inputs += 1;
        }
    }
    assert!(normalized_inputs > 0);
}

#[test]
fn denormalize_matches_cpp() {
    for line in section_lines("CELLUNION_DENORMALIZE") {
        let f = fields(line);
        let ds = decomposed(&ids(f[0]));
        let (min_level, level_mod): (u8, u8) = (f[1].parse().unwrap(), f[2].parse().unwrap());
        assert_eq!(
            union_spec::denormalize(&ds, min_level, level_mod),
            ids(f[3]),
            "{line}"
        );
    }
}

#[test]
fn set_operations_match_cpp() {
    for line in section_lines("CELLUNION_SETOPS") {
        let f = fields(line);
        let (a, b) = (ids(f[0]), ids(f[1]));
        let (ia, ib) = (intervals(&a), intervals(&b));
        assert_eq!(intervals(&ids(f[2])), set_union(&ia, &ib), "union: {line}");
        assert_eq!(
            intervals(&ids(f[3])),
            set_intersection(&ia, &ib),
            "intersection: {line}"
        );
        assert_eq!(
            intervals(&ids(f[4])),
            set_difference(&ia, &ib),
            "difference: {line}"
        );
        let (da, db) = (decomposed(&a), decomposed(&b));
        let contains = db.iter().all(|&x| union_spec::contains_id(&da, x));
        let intersects = db.iter().any(|&x| union_spec::intersects_id(&da, x));
        assert_eq!(contains, flag(f[5]), "contains: {line}");
        assert_eq!(intersects, flag(f[6]), "intersects: {line}");
    }
}

#[test]
fn from_begin_end_covers_exactly_the_range() {
    for line in section_lines("CELLUNION_FROM_BEGIN_END") {
        let f = fields(line);
        let (b, e) = (
            leaf_index_or_end(hex64(f[0])),
            leaf_index_or_end(hex64(f[1])),
        );
        let output = ids(f[2]);
        assert!(union_spec::normalized_union(&output), "{line}");
        let expected = if b == e { vec![] } else { vec![(b, e - 1)] };
        assert_eq!(intervals(&output), expected, "{line}");
    }
}

#[test]
fn leaf_count_matches_cpp() {
    for line in section_lines("CELLUNION_LEAF_CELLS") {
        let f = fields(line);
        let ds = decomposed(&ids(f[0]));
        assert!(union_spec::valid_union_d(&ds), "{line}");
        assert_eq!(
            union_spec::leaf_count(&ds),
            f[1].parse::<u128>().unwrap(),
            "{line}"
        );
    }
}

#[test]
fn encoding_matches_cpp() {
    for line in section_lines("CELLUNION_ENCODE") {
        let f = fields(line);
        assert_eq!(union_spec::encoding(&ids(f[0])), bytes(f[1]), "{line}");
    }
}

#[test]
fn decode_matches_cpp_with_the_fr017_deviation() {
    let (mut accepted, mut rejected_by_both, mut u003) = (0, 0, 0);
    for line in section_lines("CELLUNION_DECODE") {
        let f = fields(line);
        let (ok, valid, decoded) = (flag(f[1]), flag(f[2]), ids(f[3]));
        let expected = (ok && valid).then(|| decoded.clone());
        assert_eq!(union_spec::decode(&bytes(f[0])), expected, "{line}");
        match (ok, valid) {
            (true, true) => accepted += 1,
            (true, false) => u003 += 1,
            (false, _) => rejected_by_both += 1,
        }
    }
    assert!(accepted > 0 && rejected_by_both > 0 && u003 > 0);
}
