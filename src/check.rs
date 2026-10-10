//! Integrity (B20): one pass over the book and the citation sites, a
//! list of failures and notes, and nothing written. A failure is one
//! line naming the file, the line, what is wrong and what to do (B17).
//! A note is reported and does not fail the run.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use anyhow::Result;
use serde::Serialize;

use crate::book::{Book, Resolution};
use crate::cite::{self, Citation};
use crate::config::Kind;
use crate::id::Id;
use crate::schema::{DEFAULT_DECISION_STATUS, Entry};

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Fail,
    Note,
}

#[derive(Serialize, Debug, Clone)]
pub struct Diagnostic {
    pub level: Level,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
    pub message: String,
    pub fix: Option<String>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self.level {
            Level::Fail => "FAIL",
            Level::Note => "note",
        };
        write!(f, "{tag:<5} ")?;
        match (&self.path, self.line) {
            (Some(p), Some(l)) => write!(f, "{}:{l}  ", p.display())?,
            (Some(p), None) => write!(f, "{}  ", p.display())?,
            _ => {}
        }
        write!(f, "{}", self.message)?;
        if let Some(fix) = &self.fix {
            write!(f, ". {fix}")?;
        }
        Ok(())
    }
}

#[derive(Serialize, Debug)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub entries: usize,
    pub citations: usize,
    /// Decisions nothing in the scan set names.
    pub uncited: Vec<Id>,
}

impl Report {
    pub fn failed(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == Level::Fail)
    }

    pub fn failures(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == Level::Fail)
            .count()
    }
}

/// Runs every check.
pub fn run(book: &Book) -> Result<Report> {
    let mut out: Vec<Diagnostic> = book.load_failures.clone();
    let citations = cite::scan(book)?;

    check_ids(book, &mut out);
    check_fields(book, &mut out);
    check_cross_references(book, &mut out);
    check_citations(book, &citations, &mut out);
    let uncited = report_uncited(book, &citations, &mut out);
    if !book.config.projection.is_empty() {
        let mut notes = Vec::new();
        out.extend(crate::render::check(book, &mut notes)?);
        out.extend(notes);
    } else {
        let mut notes = Vec::new();
        for name in book.config.area.keys() {
            crate::render::area_view(book, name, &mut notes);
        }
        out.extend(notes);
    }

    Ok(Report {
        diagnostics: out,
        entries: book.entries.len(),
        citations: citations.len(),
        uncited,
    })
}

/// Each file is named by its id (B5); each id appears once (B3); each
/// id's letter is its namespace's.
fn check_ids(book: &Book, out: &mut Vec<Diagnostic>) {
    let mut seen: BTreeMap<Id, PathBuf> = BTreeMap::new();
    for loaded in &book.entries {
        let id = loaded.entry.id();
        let stem = loaded
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if stem != id.file_stem() {
            out.push(Diagnostic::fail(
                Some(loaded.path.clone()),
                loaded.line_of("id"),
                format!("id \"{id}\" does not match the file name"),
                Some(format!(
                    "rename the file to {}.toml, or fix the id",
                    id.file_stem()
                )),
            ));
        }
        let ns = &book.config.namespace[&loaded.namespace];
        let letter = ns.letter_or(&loaded.namespace);
        if id.letter != letter {
            out.push(Diagnostic::fail(
                Some(loaded.path.clone()),
                loaded.line_of("id"),
                format!(
                    "id \"{id}\" is in namespace \"{}\", whose letter is {letter}",
                    loaded.namespace
                ),
                Some("move the file to the directory of its letter, or fix the id".into()),
            ));
        }
        if let Some(first) = seen.insert(id, loaded.path.clone()) {
            out.push(Diagnostic::fail(
                Some(loaded.path.clone()),
                loaded.line_of("id"),
                format!("id \"{id}\" is also used by {}", first.display()),
                Some("an id is never reused; give the newer entry the next free id".into()),
            ));
        }
    }
}

/// Areas declared (B19), enum values legal (B7), dates plain, no entry
/// that says nothing (B20), the superseded pair consistent (B3), fired
/// pairs consistent.
fn check_fields(book: &Book, out: &mut Vec<Diagnostic>) {
    let statuses: Vec<String> = book
        .config
        .enums
        .get("decision_status")
        .map(|e| e.values.clone())
        .unwrap_or_else(|| {
            DEFAULT_DECISION_STATUS
                .iter()
                .map(|s| s.to_string())
                .collect()
        });

    for loaded in &book.entries {
        let path = Some(loaded.path.clone());
        let area = loaded.entry.area();
        if !book.config.area.contains_key(area) {
            let known: Vec<&str> = book.config.area.keys().map(String::as_str).collect();
            out.push(Diagnostic::fail(
                path.clone(),
                loaded.line_of("area"),
                format!("area \"{area}\" is not declared in bothy.toml"),
                Some(format!(
                    "use one of: {}; or declare it under [area.{area}]",
                    known.join(", ")
                )),
            ));
        }
        for (field, date) in loaded.entry.dates() {
            if !date.is_plain_date() {
                out.push(Diagnostic::fail(
                    path.clone(),
                    loaded.line_of(field),
                    format!("`{field}` is {date}, which carries a time or offset"),
                    Some("write a plain date, like 2026-10-09".into()),
                ));
            }
        }
        match &loaded.entry {
            Entry::Decision(d) => {
                if d.title.trim().is_empty() && loaded.kind != Kind::Principle {
                    out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("ruling"),
                        "the entry has no title".into(),
                        Some("add `title = \"...\"`, the bold lead a reader skims; only a principle may go without".into()),
                    ));
                }
                // A title may be the whole rule (nine of acquisition's are),
                // so a blank ruling is refused only where no title stands in.
                if loaded.kind == Kind::Principle
                    && d.title.trim().is_empty()
                    && d.ruling.trim().is_empty()
                {
                    out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("ruling"),
                        "the principle has no title and a blank `ruling`, so it says nothing"
                            .into(),
                        Some("write the principle in `ruling`".into()),
                    ));
                }
                if !statuses.iter().any(|s| s == &d.status) {
                    out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("status"),
                        format!("status \"{}\" is not in the decision_status enum", d.status),
                        Some(format!("use one of: {}", statuses.join(", "))),
                    ));
                }
                match (&d.status[..], &d.superseded_by) {
                    ("superseded", None) => out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("status"),
                        "status is superseded but `superseded_by` names no id".into(),
                        Some(
                            "add `superseded_by = \"<id>\"` naming the ruling that replaced it"
                                .into(),
                        ),
                    )),
                    (s, Some(by)) if s != "superseded" => out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("superseded_by"),
                        format!("`superseded_by = \"{by}\"` is set but status is \"{s}\""),
                        Some("set `status = \"superseded\"`, or remove `superseded_by`".into()),
                    )),
                    _ => {}
                }
                if let Some(by) = d.superseded_by
                    && !book.knows(by)
                {
                    out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("superseded_by"),
                        format!("`superseded_by` names {by}, which no namespace defines"),
                        Some("name an existing id".into()),
                    ));
                }
            }
            Entry::Park(p) => {
                for (field, value, fix) in [
                    ("item", &p.item, "say what is deferred, in one line"),
                    ("home", &p.home, "say where it lands when it is built"),
                ] {
                    if value.trim().is_empty() {
                        out.push(Diagnostic::fail(
                            path.clone(),
                            loaded.line_of(field),
                            format!("`{field}` is blank"),
                            Some(fix.into()),
                        ));
                    }
                }
                if p.trigger.as_deref().is_none_or(|t| t.trim().is_empty()) {
                    out.push(Diagnostic::note(
                        path.clone(),
                        loaded.line_of("home"),
                        format!("park {} states no trigger; deferral with no trigger is re-argued every time it is read", p.id),
                    ));
                }
                match (&p.fired, &p.fired_by) {
                    (Some(_), None) => out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("fired"),
                        "`fired` is set but `fired_by` names no id".into(),
                        Some(
                            "add `fired_by = \"<id>\"` naming the ruling that fired the trigger"
                                .into(),
                        ),
                    )),
                    (None, Some(by)) => out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("fired_by"),
                        format!("`fired_by = \"{by}\"` is set but `fired` has no date"),
                        Some("add `fired = <date>`".into()),
                    )),
                    _ => {}
                }
                if let Some(by) = p.fired_by
                    && !book.knows(by)
                {
                    out.push(Diagnostic::fail(
                        path.clone(),
                        loaded.line_of("fired_by"),
                        format!("`fired_by` names {by}, which no namespace defines"),
                        Some("name an existing id".into()),
                    ));
                }
            }
        }
    }
}

/// An id named inside an entry's prose must exist somewhere the entry's
/// own file can see (B10).
fn check_cross_references(book: &Book, out: &mut Vec<Diagnostic>) {
    for loaded in &book.entries {
        let refs = cite::cross_references(book, &loaded.entry.prose());
        for id in refs {
            if id == loaded.entry.id() {
                continue;
            }
            match book.resolves(id, &loaded.path) {
                Resolution::Found(_) => {}
                Resolution::NoNamespaceHere => {}
                Resolution::Unknown => out.push(Diagnostic::fail(
                    Some(loaded.path.clone()),
                    None,
                    format!("names {id}, which no namespace defines"),
                    Some("fix the id, or add the entry it means".into()),
                )),
            }
        }
    }
}

/// Every citation resolves in its scope (B10, B20).
fn check_citations(book: &Book, citations: &[Citation], out: &mut Vec<Diagnostic>) {
    let mut reported: BTreeSet<(Id, PathBuf)> = BTreeSet::new();
    for c in citations {
        match book.resolves(c.id, &c.path) {
            Resolution::Found(_) => {}
            Resolution::NoNamespaceHere => {
                // The letter is a namespace elsewhere; in this file it means nothing bothy knows.
            }
            Resolution::Unknown => {
                if reported.insert((c.id, c.path.clone())) {
                    out.push(Diagnostic::fail(
                        Some(c.path.clone()),
                        Some(c.line),
                        format!(
                            "cites {}, which no namespace covering this file defines",
                            c.id
                        ),
                        Some(
                            "fix the id; if the entry was retired its file should still exist (B3)"
                                .into(),
                        ),
                    ));
                }
            }
        }
    }
}

/// A decision nothing cites is a note, not a failure: enforced by nothing
/// is a lint, a test, or a smell, and the report is how the smell is seen.
fn report_uncited(book: &Book, citations: &[Citation], out: &mut Vec<Diagnostic>) -> Vec<Id> {
    let cited: BTreeSet<Id> = citations.iter().map(|c| c.id).collect();
    let mut uncited: Vec<Id> = book
        .entries
        .iter()
        .filter(|l| book.config.namespace[&l.namespace].kind == Kind::Decision)
        .filter(|l| !l.entry.is_retired())
        .map(|l| l.entry.id())
        .filter(|id| !cited.contains(id))
        .collect();
    uncited.sort();
    uncited.dedup();
    if !uncited.is_empty() {
        let list: Vec<String> = uncited.iter().map(ToString::to_string).collect();
        out.push(Diagnostic::note(
            None,
            None,
            format!(
                "{} uncited decision{} (no code, test or scanned document names {}): {}",
                uncited.len(),
                if uncited.len() == 1 { "" } else { "s" },
                if uncited.len() == 1 { "it" } else { "them" },
                list.join(" ")
            ),
        ));
    }
    uncited
}
