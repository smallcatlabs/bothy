//! Where ids are named. A citation is an id token in a file the
//! consumer's `[cite]` paths cover: code, tests, the documents a session
//! acts on. The entries' own cross-references are not citations (a
//! registry naming itself is not a test, a doc or a tool naming the
//! decision); they are checked for resolution separately.
//!
//! The scan never writes anything (B4).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::Serialize;

use crate::book::Book;
use crate::id::Id;

/// One place an id is named.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Citation {
    pub id: Id,
    /// Relative to the root.
    pub path: PathBuf,
    /// 1-based.
    pub line: usize,
}

/// Every id token of a known letter in every file the scan set covers.
pub fn scan(book: &Book) -> Result<Vec<Citation>> {
    let letters = book.letters();
    if letters.is_empty() || book.config.cite.paths.is_empty() {
        return Ok(Vec::new());
    }
    let include = globs(&book.config.cite.paths)?;
    let exclude = globs(&book.config.cite.exclude)?;
    let entry_dirs: Vec<PathBuf> = book
        .config
        .namespace
        .values()
        .filter_map(|ns| ns.dir.clone())
        .collect();

    let alphabet: String = letters.iter().collect();
    let re = Regex::new(&format!(r"\b([{alphabet}])([1-9][0-9]*)\b")).expect("a fixed pattern");

    let mut out = Vec::new();
    let walker = ignore::WalkBuilder::new(&book.root)
        .hidden(false)
        .git_ignore(true)
        .build();
    for item in walker {
        let item = item?;
        if !item.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let rel = item.path().strip_prefix(&book.root).unwrap_or(item.path());
        if rel.starts_with(".git") || !include.is_match(rel) || exclude.is_match(rel) {
            continue;
        }
        if entry_dirs.iter().any(|d| rel.starts_with(d)) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(item.path()) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            for cap in re.captures_iter(line) {
                let letter = cap[1].chars().next().expect("one letter");
                let number: u32 = cap[2].parse().unwrap_or(0);
                out.push(Citation {
                    id: Id { letter, number },
                    path: rel.to_path_buf(),
                    line: i + 1,
                });
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Ids named in an entry's own prose fields: its outgoing references.
pub fn cross_references(book: &Book, prose: &[(&str, &str)]) -> BTreeSet<Id> {
    let letters = book.letters();
    if letters.is_empty() {
        return BTreeSet::new();
    }
    let alphabet: String = letters.iter().collect();
    let re = Regex::new(&format!(r"\b([{alphabet}])([1-9][0-9]*)\b")).expect("a fixed pattern");
    let mut out = BTreeSet::new();
    for (_, text) in prose {
        for cap in re.captures_iter(text) {
            let letter = cap[1].chars().next().expect("one letter");
            let number: u32 = cap[2].parse().unwrap_or(0);
            out.insert(Id { letter, number });
        }
    }
    out
}

fn globs(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        b.add(
            GlobBuilder::new(p)
                .literal_separator(true)
                .build()
                .with_context(|| format!("bothy.toml: \"{p}\" is not a valid glob"))?,
        );
    }
    Ok(b.build()?)
}

/// Whether `rel` is covered by any of `patterns` (used by `pack --path`).
pub fn covered(patterns: &[String], rel: &Path) -> bool {
    globs(patterns).is_ok_and(|set| set.is_match(rel))
}
