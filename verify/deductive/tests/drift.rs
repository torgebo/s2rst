// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The drift guard (spec 003 FR-011; contracts/deductive-conventions.md, "Links").
//!
//! For every `[[mirror]]` in `mirrors.toml`, this test parses the library file, takes the named
//! function's tokens with its doc attributes removed, followed by those of every module
//! constant it names (a top-level `const` of the same file, such as a decode limit), and
//! compares their FNV-1a 64-bit hash with the recorded fingerprint. A mismatch means a
//! mirrored library function, or a constant it uses, changed: review
//! its restated copy in `src/` against it, then run `make verify-deductive-bless` (which
//! runs this test with `BLESS=1` and rewrites the fingerprints).
//!
//! Formatting and comments do not change a fingerprint; any change to the code does.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use quote::ToTokens;

const MIRRORS: &str = "mirrors.toml";

struct Mirror {
    restated: String,
    library: String,
    fingerprint: String,
}

/// The `[[mirror]]` entries, from the fixed format `mirrors.toml` uses.
fn mirrors(text: &str) -> Vec<Mirror> {
    let mut out = Vec::new();
    let mut cur: Option<Mirror> = None;
    for line in text.lines() {
        let line = line.trim();
        if line == "[[mirror]]" {
            out.extend(cur.take());
            cur = Some(Mirror {
                restated: String::new(),
                library: String::new(),
                fingerprint: String::new(),
            });
            continue;
        }
        let Some(m) = cur.as_mut() else { continue };
        let value = |key: &str| {
            line.strip_prefix(key)
                .and_then(|r| r.trim_start().strip_prefix('='))
                .map(|r| r.trim().trim_matches('"').to_string())
        };
        if let Some(v) = value("restated") {
            m.restated = v;
        } else if let Some(v) = value("library") {
            m.library = v;
        } else if let Some(v) = value("fingerprint") {
            m.fingerprint = v;
        }
    }
    out.extend(cur);
    out
}

/// FNV-1a, 64 bits.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The fingerprint of `core/src/s2/<file>.rs::<fn>`: a free function, or a method inside an
/// `impl` block named `<file>.rs::<impl header>::<fn>`.
fn fingerprint(library: &str) -> String {
    let (file, path) = library
        .split_once(".rs::")
        .unwrap_or_else(|| panic!("bad library path {library}"));
    let src_path = Path::new("../..").join(format!("{file}.rs"));
    let src = std::fs::read_to_string(&src_path)
        .unwrap_or_else(|e| panic!("{}: {e}", src_path.display()));
    let ast = syn::parse_file(&src).unwrap_or_else(|e| panic!("{}: {e}", src_path.display()));
    let (impl_header, name) = match path.rsplit_once("::") {
        Some((h, n)) => (Some(h), n),
        None => (None, path),
    };
    let mut tokens = None;
    for item in &ast.items {
        match (item, impl_header) {
            (syn::Item::Fn(f), None) if f.sig.ident == name => {
                let mut f = f.clone();
                f.attrs.retain(|a| !a.path().is_ident("doc"));
                tokens = Some(f.to_token_stream().to_string());
            }
            (syn::Item::Impl(i), Some(h)) => {
                let header = format!(
                    "impl {} for {}",
                    i.trait_
                        .as_ref()
                        .map(|t| t.1.to_token_stream().to_string())
                        .unwrap_or_default(),
                    i.self_ty.to_token_stream()
                );
                if header.replace(' ', "") != h.replace(' ', "") {
                    continue;
                }
                for it in &i.items {
                    if let syn::ImplItem::Fn(f) = it
                        && f.sig.ident == name
                    {
                        let mut f = f.clone();
                        f.attrs.retain(|a| !a.path().is_ident("doc"));
                        tokens = Some(f.to_token_stream().to_string());
                    }
                }
            }
            _ => {}
        }
    }
    let mut tokens = tokens.unwrap_or_else(|| panic!("{library}: function not found"));
    // The module constants the function names: changing one, such as a decode limit, changes
    // what the function does as much as a change to its body.
    let words: BTreeSet<&str> = tokens
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .collect();
    let mut consts: Vec<(String, String)> = ast
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Const(c) if words.contains(c.ident.to_string().as_str()) => {
                let mut c = c.clone();
                c.attrs.retain(|a| !a.path().is_ident("doc"));
                Some((c.ident.to_string(), c.to_token_stream().to_string()))
            }
            _ => None,
        })
        .collect();
    consts.sort();
    for (_, c) in consts {
        tokens.push(' ');
        tokens.push_str(&c);
    }
    let mut out = String::from("fnv1a64:");
    let _ = write!(out, "{:016x}", fnv1a64(tokens.as_bytes()));
    out
}

#[test]
fn mirrored_library_functions_are_unchanged() {
    let text = std::fs::read_to_string(MIRRORS).expect("mirrors.toml");
    let list = mirrors(&text);
    assert!(list.len() >= 17, "only {} mirrors in {MIRRORS}", list.len());
    let bless = std::env::var("BLESS").is_ok_and(|v| v == "1");
    let mut updated = text.clone();
    let mut drift = Vec::new();
    for m in &list {
        let now = fingerprint(&m.library);
        if now == m.fingerprint {
            continue;
        }
        if bless {
            let old_line = format!("fingerprint = \"{}\"", m.fingerprint);
            let new_line = format!("fingerprint = \"{now}\"");
            // Replace the fingerprint of this entry only: the first occurrence after its
            // `library` line.
            let at = updated
                .find(&format!("library = \"{}\"", m.library))
                .expect("entry");
            let rel = updated[at..].find(&old_line).expect("fingerprint line");
            updated.replace_range(at + rel..at + rel + old_line.len(), &new_line);
            println!("blessed {} ({} -> {now})", m.library, m.fingerprint);
        } else {
            drift.push(format!(
                "{} changed; review {} against it, then run `make verify-deductive-bless` (fingerprint {now}, recorded {})",
                m.library, m.restated, m.fingerprint
            ));
        }
    }
    if bless {
        std::fs::write(MIRRORS, updated).expect("writing mirrors.toml");
    }
    assert!(drift.is_empty(), "{}", drift.join("\n"));
}
