//! Projections (B2, B9): the markdown files rendered from the book.
//! Each `[[projection]]` in `bothy.toml` names an output, a template and
//! optionally one output per area. Templates are minijinja; two are
//! built in, `builtin:area` (acquisition's area-file form) and
//! `builtin:book` (every area in one file). Strings render as stored,
//! never reflowed (B18). A rendered file begins with a line naming its
//! source unless the projection turns the header off.
//!
//! `check` compares each projection with the file on disk and reports
//! the ones that differ (B20); `render` writes them.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use minijinja::syntax::SyntaxConfig;
use minijinja::value::Serde;
use minijinja::{AutoEscape, Environment, Value, context};
use serde::Serialize;

use crate::book::{Book, Loaded};
use crate::check::Diagnostic;
use crate::config::Kind;
use crate::id::Id;
use crate::schema::Entry;

/// Markers in the order they render, whatever order they were written in.
pub const MARKER_ORDER: [&str; 4] = ["Why", "Details", "Pinned", "Evidence"];

/// One entry as the line acquisition's registry writes it. A decision is
/// `- **C<n> — <title>** <ruling> *Why:* … <history>`; the ruling joins
/// the title with a space unless it opens with punctuation; an invariant
/// is `<n>. **<title>** <ruling>`; a principle `- **P<n>.** <ruling>`; a
/// park `- <item> → <home>. Trigger: <trigger>`.
pub fn entry_line(l: &Loaded) -> String {
    let mut s = String::new();
    match (&l.entry, l.kind) {
        (Entry::Decision(d), Kind::Invariant) => {
            let _ = write!(
                s,
                "{}. **{}** {}",
                d.id.number,
                d.title.trim(),
                d.ruling.trim()
            );
        }
        (Entry::Decision(d), Kind::Principle) => {
            let _ = write!(s, "- **{}.** {}", d.id, d.ruling.trim());
        }
        (Entry::Decision(d), _) => {
            let ruling = d.ruling.trim();
            let join = if ruling.is_empty() || ruling.starts_with([',', ';', ':']) {
                ""
            } else {
                " "
            };
            let _ = write!(s, "- **{} — {}**{join}{ruling}", d.id, d.title.trim());
            let markers: [(&str, Vec<&String>); 4] = [
                ("Why", d.why.iter().collect()),
                ("Details", d.details.iter().collect()),
                ("Pinned", d.pinned.iter().collect()),
                ("Evidence", d.evidence.iter().collect()),
            ];
            for (name, texts) in markers {
                if !texts.is_empty() {
                    let joined: Vec<&str> = texts.iter().map(|t| t.trim()).collect();
                    let _ = write!(s, " *{name}:* {}", joined.join("; "));
                }
            }
            if let Some(v) = &d.verdict {
                let _ = write!(s, " Owner, verbatim: \"{}\"", v.trim());
            }
            if let Some(h) = &d.history {
                let _ = write!(s, " {}", h.trim());
            } else if let (Some(date), None) = (&d.decided, &d.decided_from) {
                let _ = write!(s, " Decided {date}");
                if !d.amended.is_empty() {
                    let dates: Vec<String> = d.amended.iter().map(ToString::to_string).collect();
                    let _ = write!(s, "; amended {}", dates.join(", "));
                }
                s.push('.');
            }
        }
        (Entry::Park(p), _) => {
            let _ = write!(s, "- {} → {}", p.item.trim(), p.home.trim());
            if let Some(t) = &p.trigger {
                let _ = write!(s, ". Trigger: {}", t.trim());
            }
            if let Some(why) = &p.why {
                let _ = write!(s, " ({})", why.trim());
            }
            if let Some(v) = &p.verdict {
                let _ = write!(s, " Owner, verbatim: \"{}\"", v.trim());
            }
            if let Some(m) = &p.measured {
                let _ = write!(s, " Measured: {}", m.trim());
            }
        }
    }
    s
}

/// What a template sees for one entry.
#[derive(Serialize, Debug)]
pub struct EntryView {
    pub id: String,
    pub kind: &'static str,
    pub line: String,
}

#[derive(Serialize, Debug)]
pub struct SectionView {
    pub title: String,
    pub entries: Vec<EntryView>,
}

#[derive(Serialize, Debug)]
pub struct AreaView {
    pub name: String,
    pub title: String,
    pub index_title: String,
    pub intro: String,
    pub parked_intro: String,
    pub sections: Vec<SectionView>,
    pub parks: Vec<EntryView>,
    /// Every decision id in reading order, for an index row.
    pub ids: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct IndexRow {
    pub file: String,
    pub title: String,
    pub ids: String,
}

fn view(l: &Loaded) -> EntryView {
    EntryView {
        id: l.entry.id().to_string(),
        kind: kind_name(l.kind),
        line: entry_line(l),
    }
}

pub fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Decision => "decision",
        Kind::Invariant => "invariant",
        Kind::Principle => "principle",
        Kind::Park => "park",
        Kind::External => "external",
    }
}

/// An area's live entries placed by its layout: the declared sections
/// with their ids in order, then anything the layout did not place, by
/// id, in the last section (or one named for the area). Notes say what
/// was unplaced, so the layout can be kept current.
pub fn area_view(book: &Book, name: &str, notes: &mut Vec<Diagnostic>) -> AreaView {
    let area = &book.config.area[name];
    let live: Vec<&Loaded> = book
        .entries
        .iter()
        .filter(|l| l.entry.area() == name && !l.entry.is_retired())
        .collect();
    let by_id: BTreeMap<Id, &Loaded> = live.iter().map(|l| (l.entry.id(), *l)).collect();

    let mut placed: Vec<Id> = Vec::new();
    let mut sections: Vec<SectionView> = Vec::new();
    for s in &area.sections {
        let mut entries = Vec::new();
        for id in &s.ids {
            match by_id.get(id) {
                Some(l) if l.kind == Kind::Decision => {
                    entries.push(view(l));
                    placed.push(*id);
                }
                Some(_) => notes.push(Diagnostic::note(
                    None,
                    None,
                    format!("area {name}: section \"{}\" lists {id}, which is a {}, not a decision; it is not rendered there", s.title, by_id[id].kind_name()),
                )),
                None => notes.push(Diagnostic::note(
                    None,
                    None,
                    format!("area {name}: section \"{}\" lists {id}, which is not a live entry of this area; remove it from the layout", s.title),
                )),
            }
        }
        sections.push(SectionView {
            title: s.title.clone(),
            entries,
        });
    }
    let mut unplaced: Vec<&Loaded> = live
        .iter()
        .copied()
        .filter(|l| l.kind == Kind::Decision && !placed.contains(&l.entry.id()))
        .collect();
    unplaced.sort_by_key(|l| l.entry.id());
    if !unplaced.is_empty() {
        if !area.sections.is_empty() {
            let ids: Vec<String> = unplaced.iter().map(|l| l.entry.id().to_string()).collect();
            notes.push(Diagnostic::note(
                None,
                None,
                format!("area {name}: {} not placed by the layout and rendered at the end; add {} to a section in bothy.toml", ids.join(", "), if ids.len() == 1 { "it" } else { "them" }),
            ));
        }
        if sections.is_empty() {
            sections.push(SectionView {
                title: area.title.clone(),
                entries: Vec::new(),
            });
        }
        let last = sections.last_mut().expect("one section");
        last.entries.extend(unplaced.iter().map(|l| view(l)));
    }

    let mut parks: Vec<EntryView> = Vec::new();
    let mut placed_parks: Vec<Id> = Vec::new();
    for id in &area.parks {
        match by_id.get(id) {
            Some(l) if l.kind == Kind::Park => {
                parks.push(view(l));
                placed_parks.push(*id);
            }
            _ => notes.push(Diagnostic::note(
                None,
                None,
                format!("area {name}: the parks layout lists {id}, which is not a live park of this area; remove it"),
            )),
        }
    }
    let mut rest: Vec<&Loaded> = live
        .iter()
        .copied()
        .filter(|l| l.kind == Kind::Park && !placed_parks.contains(&l.entry.id()))
        .collect();
    rest.sort_by_key(|l| l.entry.id());
    if !rest.is_empty() && !area.parks.is_empty() {
        let ids: Vec<String> = rest.iter().map(|l| l.entry.id().to_string()).collect();
        notes.push(Diagnostic::note(
            None,
            None,
            format!("area {name}: park{} {} not placed by the layout and rendered last; add to `parks` in bothy.toml", if ids.len() == 1 { "" } else { "s" }, ids.join(", ")),
        ));
    }
    parks.extend(rest.iter().map(|l| view(l)));

    let ids: Vec<String> = sections
        .iter()
        .flat_map(|s| s.entries.iter().map(|e| e.id.clone()))
        .collect();
    AreaView {
        name: name.to_string(),
        title: area.title.clone(),
        index_title: area
            .index_title
            .clone()
            .unwrap_or_else(|| area.title.clone()),
        intro: area.intro.clone().unwrap_or_default(),
        parked_intro: area.parked_intro.clone().unwrap_or_default(),
        sections,
        parks,
        ids,
    }
}

const BUILTIN_AREA: &str = "# Decisions — {{ area.title }}

{{ area.intro }}
{% for s in area.sections %}
## {{ s.title }}

{% for e in s.entries %}{{ e.line }}
{% endfor %}{% endfor %}
## Parked (do not build yet; each with its trigger)

{{ area.parked_intro }}

{% for e in area.parks %}{{ e.line }}
{% endfor %}";

const BUILTIN_BOOK: &str = "# The book

Every ruling and every park, by area, rendered from the entries.
{% for a in areas %}
## {{ a.title }}
{% for s in a.sections %}{% if a.sections|length > 1 %}
### {{ s.title }}
{% endif %}
{% for e in s.entries %}{{ e.line }}
{% endfor %}{% endfor %}{% if a.parks %}
Parked, each with the trigger that reopens it:

{% for e in a.parks %}{{ e.line }}
{% endfor %}{% endif %}{% endfor %}";

#[derive(Debug)]
pub struct Rendered {
    /// Relative to the root.
    pub path: PathBuf,
    pub text: String,
}

/// Every projection, rendered.
pub fn all(book: &Book, notes: &mut Vec<Diagnostic>) -> Result<Vec<Rendered>> {
    let mut env = Environment::new();
    env.set_syntax(
        SyntaxConfig::builder()
            .keep_trailing_newline(true)
            .build()?,
    );
    env.set_auto_escape_callback(|_| AutoEscape::None);
    env.add_template("builtin:area", BUILTIN_AREA)?;
    env.add_template("builtin:book", BUILTIN_BOOK)?;

    let area_names: Vec<&str> = book.config.area.keys().map(String::as_str).collect();
    let views: Vec<AreaView> = area_names
        .iter()
        .map(|n| area_view(book, n, notes))
        .collect();
    let kind_lines = |kind: Kind, area: Option<&str>| -> Vec<EntryView> {
        let mut v: Vec<&Loaded> = book
            .entries
            .iter()
            .filter(|l| l.kind == kind && !l.entry.is_retired())
            .filter(|l| area.is_none_or(|a| l.entry.area() == a))
            .collect();
        v.sort_by_key(|l| l.entry.id());
        v.iter().map(|l| view(l)).collect()
    };
    let index: Vec<IndexRow> = views
        .iter()
        .filter(|v| v.name != "cross-cutting")
        .map(|v| IndexRow {
            file: v.name.clone(),
            title: v.index_title.clone(),
            ids: v.ids.join(", "),
        })
        .collect();
    let cross = views.iter().find(|v| v.name == "cross-cutting");
    let cross_entries: Vec<&EntryView> = cross
        .map(|v| v.sections.iter().flat_map(|s| s.entries.iter()).collect())
        .unwrap_or_default();
    let cross_parks: Vec<&EntryView> = cross.map(|v| v.parks.iter().collect()).unwrap_or_default();

    let mut out = Vec::new();
    for (k, proj) in book.config.projection.iter().enumerate() {
        let template_name = match &proj.template {
            Some(t) if t.starts_with("builtin:") => t.clone(),
            Some(file) => {
                let path = book.root.join(file);
                let source = std::fs::read_to_string(&path).with_context(|| {
                    format!("projection {}: cannot read template {}", proj.out, file)
                })?;
                let name = format!("file:{k}:{file}");
                env.add_template_owned(name.clone(), source)?;
                name
            }
            None if proj.for_each.is_some() => "builtin:area".to_string(),
            None => "builtin:book".to_string(),
        };
        let tmpl = env.get_template(&template_name)?;
        let header = |source: &str| -> String {
            if proj.header {
                format!(
                    "<!-- Rendered by bothy from {source}; edit the entries, not this file. -->\n"
                )
            } else {
                String::new()
            }
        };
        match proj.for_each.as_deref() {
            Some("area") => {
                for v in views.iter().filter(|v| !proj.except.contains(&v.name)) {
                    let text = tmpl
                        .render(context! { area => Value::from(Serde(v)) })
                        .with_context(|| format!("projection {} for area {}", proj.out, v.name))?;
                    out.push(Rendered {
                        path: PathBuf::from(proj.out.replace("{area}", &v.name)),
                        text: format!("{}{}", header(&format!("the `{}` entries", v.name)), text),
                    });
                }
            }
            Some(other) => bail!(
                "projection {}: for_each = {other:?} is not understood; only \"area\" is",
                proj.out
            ),
            None => {
                let text = tmpl
                    .render(context! {
                        areas => Value::from(Serde(&views)),
                        index => Value::from(Serde(&index)),
                        invariants => Value::from(Serde(kind_lines(Kind::Invariant, None))),
                        principles => Value::from(Serde(kind_lines(Kind::Principle, None))),
                        cross => Value::from(Serde(&cross_entries)),
                        cross_parks => Value::from(Serde(&cross_parks)),
                    })
                    .with_context(|| format!("projection {}", proj.out))?;
                out.push(Rendered {
                    path: PathBuf::from(&proj.out),
                    text: format!("{}{}", header("the book"), text),
                });
            }
        }
    }
    Ok(out)
}

/// Each projection against the file on disk: a failure per difference.
pub fn check(book: &Book, notes: &mut Vec<Diagnostic>) -> Result<Vec<Diagnostic>> {
    let mut out = Vec::new();
    for r in all(book, notes)? {
        let on_disk = std::fs::read_to_string(book.root.join(&r.path)).ok();
        match on_disk {
            None => out.push(Diagnostic::fail(
                Some(r.path.clone()),
                None,
                "this projection has not been rendered".into(),
                Some("run `bothy render`".into()),
            )),
            Some(text) if text != r.text => {
                let (added, removed) = line_delta(&text, &r.text);
                out.push(Diagnostic::fail(
                    Some(r.path.clone()),
                    None,
                    format!("differs from what the entries render ({removed} lines would change, {added} new)"),
                    Some("run `bothy render` to rewrite it, or `bothy render --diff` to see the difference; the entries are the source, not this file".into()),
                ));
            }
            Some(_) => {}
        }
    }
    Ok(out)
}

/// Lines removed from `old` and added in `new`.
pub fn line_delta(old: &str, new: &str) -> (usize, usize) {
    let diff = similar::TextDiff::from_lines(old, new);
    let mut added = 0;
    let mut removed = 0;
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Insert => added += 1,
            similar::ChangeTag::Delete => removed += 1,
            similar::ChangeTag::Equal => {}
        }
    }
    (added, removed)
}

/// A unified diff of a projection against the disk, for `render --diff`.
pub fn unified(path: &Path, old: &str, new: &str) -> String {
    let diff = similar::TextDiff::from_lines(old, new);
    diff.unified_diff()
        .context_radius(1)
        .header(
            &format!("{} (on disk)", path.display()),
            &format!("{} (rendered)", path.display()),
        )
        .to_string()
}

/// Writes every projection under the root (B4: declared projections are
/// the one thing outside its own tree the tool writes).
pub fn write(book: &Book, notes: &mut Vec<Diagnostic>) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for r in all(book, notes)? {
        let abs = book.root.join(&r.path);
        if let Some(dir) = abs.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&abs, &r.text)
            .with_context(|| format!("cannot write {}", r.path.display()))?;
        written.push(r.path);
    }
    Ok(written)
}
