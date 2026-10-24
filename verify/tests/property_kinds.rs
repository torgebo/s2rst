// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Keeps `PROPERTIES_BY_KIND.md` and the harness docs in step with the four property
//! catalogues, so that the proofs can be browsed by kind:
//! - every catalogue entry is listed on the page exactly once, under one kind, and the page
//!   lists nothing else;
//! - every kind tag in the code (`P-ID (kind)`, at the start of a harness doc) is the kind
//!   the page gives;
//! - every entry the page says a harness proves carries its tag in the module the page links,
//!   and the harness names it gives are there;
//! - the summary table's counts are the sections' counts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// The kinds: the page's section heading and the tag the code uses.
const KINDS: [(&str, &str); 8] = [
    ("Never crashes", "no crash"),
    ("Correct result", "correct result"),
    ("Round trip", "round trip"),
    ("Exact output", "exact output"),
    ("Exact input", "exact input"),
    ("Validity", "validity"),
    ("Laws", "law"),
    ("Model checks", "model check"),
];

const PAGE: &str = "PROPERTIES_BY_KIND.md";
const CATALOGUES: [&str; 4] = [
    "PROPERTIES.md",
    "cell_union/PROPERTIES.md",
    "decoding/PROPERTIES.md",
    "values/PROPERTIES.md",
];

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The property id (`P-AREA-NN`) that `s` starts with, if any.
fn id_at(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    if !s.starts_with("P-") {
        return None;
    }
    let mut i = 2;
    while i < b.len() && b[i].is_ascii_uppercase() {
        i += 1;
    }
    let digits = |k: usize| b.get(k).is_some_and(u8::is_ascii_digit);
    if i == 2 || b.get(i) != Some(&b'-') || !digits(i + 1) || !digits(i + 2) || digits(i + 3) {
        return None;
    }
    Some(&s[..i + 3])
}

/// The cells of a Markdown table row (none of the rows read here has a `|` inside a cell).
fn cells(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

/// The ids in the first column of a catalogue's tables.
fn catalogue_ids() -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for cat in CATALOGUES {
        for line in read(cat).lines().filter(|l| l.starts_with("| P-")) {
            let first = cells(line)[0];
            assert_eq!(id_at(first), Some(first), "{cat}: a malformed id in {line}");
            assert!(ids.insert(first.to_owned()), "{first} is in two catalogues");
        }
    }
    ids
}

/// One entry of the page: its kind's tag and the "Proved by" cell.
struct Entry {
    tag: &'static str,
    proved_by: String,
}

/// The page's entries, and the summary table's count per tag.
fn page() -> (BTreeMap<String, Entry>, BTreeMap<&'static str, usize>) {
    let mut entries = BTreeMap::new();
    let mut summary = BTreeMap::new();
    let mut kind: Option<&'static str> = None;
    for line in read(PAGE).lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            kind = KINDS.iter().find(|(h, _)| *h == heading).map(|&(_, t)| t);
            continue;
        }
        let Some(row) = line.strip_prefix("| [") else {
            continue;
        };
        let text = &row[..row.find(']').expect("a link")];
        if let Some(id) = id_at(text) {
            let tag = kind.unwrap_or_else(|| panic!("{id} is listed outside the kind sections"));
            let proved_by = cells(line)[2].to_owned();
            let old = entries.insert(id.to_owned(), Entry { tag, proved_by });
            assert!(old.is_none(), "{id} is listed twice");
        } else if let Some(&(_, tag)) = KINDS.iter().find(|(h, _)| *h == text) {
            let last = *cells(line).last().unwrap();
            let n: String = last
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(char::is_ascii_digit)
                .collect();
            summary.insert(tag, n.parse().expect("a count in the summary table"));
        }
    }
    (entries, summary)
}

/// Every `P-ID (kind)` in a source file, as (id, kind).
fn tags(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (pos, _) in src.match_indices("P-") {
        let Some(id) = id_at(&src[pos..]) else {
            continue;
        };
        let Some(rest) = src[pos + id.len()..].strip_prefix(" (") else {
            continue;
        };
        let inner = &rest[..rest.find(')').unwrap_or(0)];
        if KINDS.iter().any(|&(_, t)| t == inner) {
            out.push((id.to_owned(), inner.to_owned()));
        }
    }
    out
}

/// The `(src/<file>)` links of a "Proved by" cell, each with the code spans of its text.
fn links(cell: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut rest = cell;
    while let Some(end) = rest.find("](src/") {
        let start = rest[..end].rfind('[').expect("a link text");
        let text = &rest[start + 1..end];
        let after = &rest[end + 2..];
        let close = after.find(')').expect("a link target");
        let spans = text
            .split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect();
        out.push((after[..close].to_owned(), spans));
        rest = &after[close..];
    }
    out
}

#[test]
fn the_page_lists_every_catalogue_entry_once() {
    let want = catalogue_ids();
    let (entries, _) = page();
    let got: BTreeSet<String> = entries.keys().cloned().collect();
    let missing: Vec<_> = want.difference(&got).collect();
    let extra: Vec<_> = got.difference(&want).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "{PAGE}: missing {missing:?}, not in any catalogue {extra:?}"
    );
}

#[test]
fn the_summary_counts_the_sections() {
    let (entries, summary) = page();
    for (_, tag) in KINDS {
        let n = entries.values().filter(|e| e.tag == tag).count();
        assert_eq!(
            summary.get(tag),
            Some(&n),
            "{PAGE}: the summary's count for {tag}"
        );
    }
}

#[test]
fn the_code_tags_each_property_with_its_kind() {
    let (entries, _) = page();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut by_file: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for path in files {
        let name = format!("src/{}", path.file_name().unwrap().to_str().unwrap());
        for (id, tag) in tags(&fs::read_to_string(&path).unwrap()) {
            let entry = entries
                .get(&id)
                .unwrap_or_else(|| panic!("{name}: {id} ({tag}) is not on {PAGE}"));
            assert_eq!(
                tag, entry.tag,
                "{name}: {id} is tagged ({tag}), {PAGE} lists it as ({})",
                entry.tag
            );
            by_file.entry(name.clone()).or_default().push((id, tag));
        }
    }
    for (id, entry) in &entries {
        for (file, spans) in links(&entry.proved_by) {
            let tagged = by_file
                .get(&file)
                .is_some_and(|t| t.iter().any(|(i, _)| i == id));
            assert!(
                tagged,
                "{PAGE}: {id} links {file}, which has no `{id} ({})`",
                entry.tag
            );
            let src = read(&file);
            for span in spans {
                // `module::p_x_01__n*` names a family: its members start with the part before
                // the first `*`.
                let name = span.rsplit("::").next().unwrap();
                let stem = &name[..name.find('*').unwrap_or(name.len())];
                assert!(
                    src.contains(stem),
                    "{PAGE}: {id} names `{span}`, which {file} lacks"
                );
            }
        }
    }
}
