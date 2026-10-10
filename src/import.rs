//! `import`: the one-time migration of acquisition's markdown registry
//! into entries (the brief, §4 and §7). It is the grammar audit grown
//! up: the same parsers, writing files instead of counting, and it
//! must find the same counts and exceptions on `bfa59416`.
//!
//! The grammar it reads, as acquisition writes it at the tip:
//!
//! - a decision is one bullet in `CONTEXT.md` (cross-cutting) or
//!   `decisions/<area>.md`: `- **C<n> — <title>** <body>`, the body
//!   a ruling followed by any of `*Why:*`, `*Details:*`, `*Pinned:*`,
//!   `*Evidence:*` in any order, often ending in a dating sentence
//!   ("Ruled 2026-09-02.") that this keeps whole as `history`;
//! - a park is one bullet under `## Parked`: `<item> → <home>. Trigger:
//!   <trigger>`, the trigger sometimes absent;
//! - an invariant is `<n>. **<lead>** <rest>` under `## Invariants`;
//! - a principle is `- **P<n>.** <text>` under `## Working style`.
//!
//! Everything is kept verbatim: titles with their terminators, rulings
//! on their one line, marker texts as written (B18). Dates the prose
//! lacks come from git and say so (B13). Nothing under the consumer's
//! tree is written (B4); the output goes where the caller says.
//!
//! This file is the playground's grammar. A second source has its own
//! module and shares nothing but the writer (B26): [`master`] reads the
//! claims and findings on acquisition's master branch.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use regex::Regex;

use crate::emit;
use crate::id::Id;
use crate::schema::{Date, Decision, Park};

pub mod master;

/// What one area's file told us, for the config and the renderer.
#[derive(Debug, Default)]
pub struct AreaMeta {
    pub title: String,
    pub index_title: Option<String>,
    pub intro: Option<String>,
    pub parked_intro: Option<String>,
    /// The files the intro says the area governs, each found on disk.
    pub paths: Vec<String>,
    /// Files the intro names that were not found where it points.
    pub paths_not_found: Vec<String>,
    /// The file's sections in order, each with its decision ids in order.
    pub sections: Vec<(String, Vec<Id>)>,
    /// The file's parks in order, by item text until ids are assigned.
    pub park_items: Vec<String>,
    pub parks: Vec<Id>,
}

#[derive(Debug, Default)]
pub struct Imported {
    pub decisions: Vec<Decision>,
    pub invariants: Vec<Decision>,
    pub principles: Vec<Decision>,
    pub parks: Vec<Park>,
    /// Areas in the order CONTEXT's index table lists them, then any the
    /// table misses.
    pub areas: IndexMap<String, AreaMeta>,
    /// Lines that looked like an entry and did not parse, with why.
    pub exceptions: Vec<String>,
    /// Letters named inside entries that no namespace will define.
    pub undeclared: BTreeMap<char, usize>,
    /// The paragraph under `## Orientation`, which becomes the book's
    /// `orientation` and one tag in the template (B27).
    pub orientation: Option<String>,
    /// `CONTEXT.md` with its generated blocks replaced by template tags.
    pub context_template: String,
}

const MARKERS: [&str; 4] = ["Why", "Details", "Pinned", "Evidence"];

/// Reads the playground at `root` (the directory holding `CONTEXT.md`).
pub fn parse_playground(root: &Path) -> Result<Imported> {
    let mut out = Imported::default();
    let context = read(root, "CONTEXT.md")?;

    // The index table fixes the areas' order and their index titles.
    let row_re = Regex::new(r"^\| `decisions/(\w+)\.md` \| (.*?) \| (.*?) \|$").expect("fixed");
    for line in context.lines() {
        if let Some(c) = row_re.captures(line) {
            out.areas.insert(
                c[1].to_string(),
                AreaMeta {
                    index_title: Some(c[2].to_string()),
                    ..Default::default()
                },
            );
        }
    }

    parse_context(&context, &mut out, root)?;

    let dir = root.join("decisions");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    for path in files {
        let area = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let text = std::fs::read_to_string(&path)?;
        parse_area_file(&area, &text, &mut out, root)?;
        if let Some(meta) = out.areas.get_mut(&area)
            && let Some(intro) = meta.intro.clone()
        {
            (meta.paths, meta.paths_not_found) = governed_paths(&intro, root);
        }
    }

    fill_dates_from_git(root, &mut out);
    count_undeclared(&mut out);
    Ok(out)
}

fn read(root: &Path, rel: &str) -> Result<String> {
    std::fs::read_to_string(root.join(rel))
        .with_context(|| format!("cannot read {}", root.join(rel).display()))
}

/// `CONTEXT.md`: invariants, cross-cutting decisions and parks, principles.
fn parse_context(text: &str, out: &mut Imported, _root: &Path) -> Result<()> {
    let inv_re = Regex::new(r"^(\d+)\. \*\*(.*?)\*\* (.*)$").expect("fixed");
    let prin_re = Regex::new(r"^- \*\*P(\d+)\.\*\* (.*)$").expect("fixed");
    let row_re = Regex::new(r"^\| `decisions/(\w+)\.md` \|").expect("fixed");
    let mut section = String::new();
    let mut cross_ids: Vec<Id> = Vec::new();
    let mut cross_parks: Vec<String> = Vec::new();
    // Each generated block becomes one template tag: a run of lines of one
    // kind is replaced once, and the tag carries the lines' own newlines.
    let mut template = String::new();
    let mut last_block: Option<&str> = None;
    let mut orientation_until = 0;
    for (i, line) in text.lines().enumerate() {
        let lineno = i + 1;
        // The orientation is the first paragraph under its heading; it
        // moves to bothy.toml and the template names it (B27).
        if section.starts_with("Orientation")
            && out.orientation.is_none()
            && !line.trim().is_empty()
            && !line.starts_with('#')
        {
            let end = text
                .lines()
                .skip(i)
                .position(|l| l.trim().is_empty())
                .map_or(text.lines().count(), |n| i + n);
            let para: Vec<&str> = text.lines().skip(i).take(end - i).collect();
            out.orientation = Some(para.join("\n"));
            orientation_until = end;
            template.push_str("{{ orientation }}\n");
            last_block = None;
            continue;
        }
        if i < orientation_until {
            continue;
        }
        let block = if section.starts_with("Invariants") && inv_re.is_match(line) {
            Some((
                "invariants",
                "{% for e in invariants %}{{ e.line }}\n{% endfor %}",
            ))
        } else if section.starts_with("Working style") && prin_re.is_match(line) {
            Some((
                "principles",
                "{% for e in principles %}{{ e.line }}\n{% endfor %}",
            ))
        } else if section.starts_with("Decisions") && row_re.is_match(line) {
            Some((
                "index",
                "{% for r in index %}| `decisions/{{ r.file }}.md` | {{ r.title }} | {{ r.ids }} |\n{% endfor %}",
            ))
        } else if section.starts_with("Decisions") && line.starts_with("- **C") {
            Some(("cross", "{% for e in cross %}{{ e.line }}\n{% endfor %}"))
        } else if section.starts_with("Parked") && line.starts_with("- ") {
            Some((
                "cross_parks",
                "{% for e in cross_parks %}{{ e.line }}\n{% endfor %}",
            ))
        } else {
            None
        };
        match block {
            Some((kind, tag)) => {
                if last_block != Some(kind) {
                    template.push_str(tag);
                    last_block = Some(kind);
                }
            }
            None => {
                last_block = None;
                template.push_str(line);
                template.push('\n');
            }
        }
        if let Some(h) = line.strip_prefix("## ") {
            section = h.to_string();
            continue;
        }
        if line.starts_with("### ") {
            continue;
        }
        if section.starts_with("Invariants")
            && let Some(c) = inv_re.captures(line)
        {
            out.invariants.push(Decision {
                id: Id {
                    letter: 'I',
                    number: c[1].parse().unwrap_or(0),
                },
                area: "cross-cutting".into(),
                title: c[2].to_string(),
                ruling: c[3].to_string(),
                ..blank_decision()
            });
            continue;
        }
        if section.starts_with("Working style")
            && let Some(c) = prin_re.captures(line)
        {
            out.principles.push(Decision {
                id: Id {
                    letter: 'P',
                    number: c[1].parse().unwrap_or(0),
                },
                area: "cross-cutting".into(),
                ruling: c[2].to_string(),
                ..blank_decision()
            });
            continue;
        }
        if section.starts_with("Decisions") && line.starts_with("- **C") {
            match parse_decision(line, "cross-cutting") {
                Ok(d) => {
                    cross_ids.push(d.id);
                    out.decisions.push(d);
                }
                Err(e) => out.exceptions.push(format!("CONTEXT.md:{lineno}: {e}")),
            }
            continue;
        }
        if section.starts_with("Parked") && line.starts_with("- ") {
            match parse_park(&line[2..], "cross-cutting") {
                Ok(p) => {
                    cross_parks.push(p.item.clone());
                    out.parks.push(p);
                }
                Err(e) => out.exceptions.push(format!("CONTEXT.md:{lineno}: {e}")),
            }
        }
    }
    out.context_template = template;
    let cross = out.areas.entry("cross-cutting".to_string()).or_default();
    cross.title = "What every area must know".into();
    cross.sections = vec![("Cross-cutting".to_string(), cross_ids)];
    cross.park_items = cross_parks;
    Ok(())
}

/// The files an area file's intro says it governs, as globs under the
/// root, for `pack --path` (B19). The intro names them in backticks
/// before "Read this before": a path inside a crate
/// (`acquisition-store/src/lib.rs`), a bare file name beside the one
/// before it (`snapshot.rs`), or a whole crate (`acquisition-cli`).
/// Only what exists on disk is returned; a file name that was not found
/// where the intro points is returned second, to be reported, and
/// anything else in backticks (a type, a verb) is not a path.
pub fn governed_paths(intro: &str, root: &Path) -> (Vec<String>, Vec<String>) {
    let scope = intro.split("Read this before").next().unwrap_or(intro);
    let tick = Regex::new(r"`([^`]+)`").expect("fixed");
    let mut found: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut dir: Option<String> = None;
    for c in tick.captures_iter(scope) {
        let token = &c[1];
        let candidate = if token.starts_with("acquisition-") && token.contains('/') {
            dir = token.rsplit_once('/').map(|(d, _)| d.to_string());
            format!("crates/{token}")
        } else if token.starts_with("acquisition-") && !token.contains(['.', ' ']) {
            format!("crates/{token}")
        } else if token.ends_with(".rs") && !token.contains(['/', ' ']) {
            match &dir {
                Some(d) => format!("crates/{d}/{token}"),
                None => {
                    missing.push(token.to_string());
                    continue;
                }
            }
        } else {
            continue;
        };
        let on_disk = root.join(&candidate);
        let glob = if on_disk.is_dir() {
            format!("{candidate}/**")
        } else if on_disk.is_file() {
            candidate
        } else {
            missing.push(candidate);
            continue;
        };
        if !found.contains(&glob) {
            found.push(glob);
        }
    }
    (found, missing)
}

/// `decisions/<area>.md`: the title, the intro, sections of decisions,
/// then `## Parked` with its intro and its parks.
fn parse_area_file(area: &str, text: &str, out: &mut Imported, _root: &Path) -> Result<()> {
    let meta = out.areas.entry(area.to_string()).or_default();
    let mut in_parked = false;
    // Prose between a heading and the first bullet is the heading's
    // intro, blank lines included, so a two-paragraph intro survives.
    let mut intro_lines: Vec<&str> = Vec::new();
    let mut collecting: Option<&str> = None;
    fn flush(meta: &mut AreaMeta, collecting: &mut Option<&str>, lines: &mut Vec<&str>) {
        let text = lines.join("\n").trim().to_string();
        match collecting.take() {
            Some("intro") if !text.is_empty() => meta.intro = Some(text),
            Some("parked") if !text.is_empty() => meta.parked_intro = Some(text),
            _ => {}
        }
        lines.clear();
    }
    for (i, line) in text.lines().enumerate() {
        let lineno = i + 1;
        if let Some(h) = line.strip_prefix("# ") {
            meta.title = h.strip_prefix("Decisions — ").unwrap_or(h).to_string();
            collecting = Some("intro");
            continue;
        }
        if let Some(h) = line.strip_prefix("## ") {
            flush(meta, &mut collecting, &mut intro_lines);
            in_parked = h.starts_with("Parked");
            if in_parked {
                collecting = Some("parked");
            } else {
                meta.sections.push((h.to_string(), Vec::new()));
                collecting = Some("section");
            }
            continue;
        }
        if !line.starts_with("- ") {
            if collecting.is_some() {
                intro_lines.push(line);
            }
            continue;
        }
        flush(meta, &mut collecting, &mut intro_lines);
        if in_parked {
            match parse_park(&line[2..], area) {
                Ok(p) => {
                    meta.park_items.push(p.item.clone());
                    out.parks.push(p);
                }
                Err(e) => out
                    .exceptions
                    .push(format!("decisions/{area}.md:{lineno}: {e}")),
            }
        } else if line.starts_with("- **C") {
            match parse_decision(line, area) {
                Ok(d) => {
                    match meta.sections.last_mut() {
                        Some(last) => last.1.push(d.id),
                        None => out.exceptions.push(format!(
                            "decisions/{area}.md:{lineno}: {} sits before any `## ` section",
                            d.id
                        )),
                    }
                    out.decisions.push(d);
                }
                Err(e) => out
                    .exceptions
                    .push(format!("decisions/{area}.md:{lineno}: {e}")),
            }
        }
    }
    flush(meta, &mut collecting, &mut intro_lines);
    if meta.title.is_empty() {
        out.exceptions.push(format!(
            "decisions/{area}.md: no `# Decisions — <title>` heading"
        ));
    }
    Ok(())
}

fn blank_decision() -> Decision {
    Decision {
        id: Id {
            letter: 'C',
            number: 1,
        },
        area: String::new(),
        title: String::new(),
        ruling: String::new(),
        why: None,
        details: Vec::new(),
        pinned: Vec::new(),
        evidence: Vec::new(),
        decided: None,
        decided_from: None,
        amended: Vec::new(),
        verdict: None,
        history: None,
        status: "ruled".into(),
        superseded_by: None,
    }
}

/// `- **C<n> — <title>** <body>`.
pub fn parse_decision(line: &str, area: &str) -> Result<Decision> {
    // The body follows the title's `**` after a space, or with no space
    // when it opens with punctuation ("**...realm**, stamped from..."); the
    // renderer puts the space back by the same rule.
    let head_re = Regex::new(r"^- \*\*C(\d+) — (.*?)\*\*(.*)$").expect("fixed");
    let Some(c) = head_re.captures(line) else {
        bail!(
            "starts like a decision but is not `- **C<n> — <title>** <body>`: {}",
            &line[..line.len().min(80)]
        );
    };
    let number: u32 = c[1].parse()?;
    let title = c[2].to_string();
    let body = match c[3].strip_prefix(' ') {
        Some(rest) => rest.to_string(),
        None if c[3].starts_with([',', ';', ':']) => c[3].to_string(),
        None => bail!(
            "C{number}: the title's closing `**` is followed by neither a space nor punctuation: {}",
            &c[3][..c[3].len().min(40)]
        ),
    };

    // Markers, in the order they appear.
    let mut positions: Vec<(usize, &str)> = Vec::new();
    for mk in MARKERS {
        let needle = format!("*{mk}:*");
        let mut start = 0;
        while let Some(off) = body[start..].find(&needle) {
            positions.push((start + off, mk));
            start += off + needle.len();
        }
    }
    positions.sort();
    let mut seen = std::collections::HashSet::new();
    for (_, mk) in &positions {
        if !seen.insert(*mk) {
            bail!("C{number}: marker *{mk}:* appears twice");
        }
    }

    let ruling_end = positions.first().map(|p| p.0).unwrap_or(body.len());
    let mut segments: Vec<(&str, String)> = vec![("ruling", body[..ruling_end].trim().to_string())];
    for (k, (pos, mk)) in positions.iter().enumerate() {
        let text_start = pos + mk.len() + 3;
        let end = positions.get(k + 1).map(|p| p.0).unwrap_or(body.len());
        segments.push((mk, body[text_start..end].trim().to_string()));
    }

    // A dating sentence at the very end of the last segment is history.
    let mut history = None;
    if let Some(last) = segments.last_mut()
        && let Some(cut) = history_start(&last.1)
    {
        history = Some(last.1[cut..].trim().to_string());
        last.1 = last.1[..cut].trim_end().to_string();
    }

    let mut d = Decision {
        id: Id {
            letter: 'C',
            number,
        },
        area: area.to_string(),
        title,
        history,
        ..blank_decision()
    };
    for (name, text) in segments {
        match name {
            "ruling" => d.ruling = text,
            "Why" => d.why = Some(text),
            "Details" => d.details = vec![text],
            "Pinned" => d.pinned = vec![text],
            "Evidence" => d.evidence = vec![text],
            _ => unreachable!(),
        }
    }
    if d.ruling.is_empty() && d.why.is_none() {
        bail!("C{number}: no ruling text after the title");
    }
    Ok(d)
}

/// The byte where a trailing dating sentence begins, if the text ends in
/// one: the earliest "Decided|Ruled|Amended <date>" that starts a
/// sentence (at the start, or after `.`, `)`, `` ` `` or `*` and a space).
fn history_start(text: &str) -> Option<usize> {
    let re = Regex::new(r"\b(Decided|Ruled|Amended) \d{4}-\d{2}-\d{2}").expect("fixed");
    for m in re.find_iter(text) {
        let before = text[..m.start()].trim_end();
        if before.is_empty() || before.ends_with(['.', ')', '`', '*']) {
            return Some(m.start());
        }
    }
    // A bare date as the last sentence ("… `derive.rs`. 2026-09-17.") is a
    // dating sentence too.
    let bare = Regex::new(r" (\d{4}-\d{2}-\d{2}\.)$").expect("fixed");
    if let Some(m) = bare.find(text)
        && text[..m.start()].ends_with(['.', ')', '`', '*'])
    {
        return Some(m.start() + 1);
    }
    None
}

/// `<item> → <home>. Trigger: <trigger>`.
pub fn parse_park(body: &str, area: &str) -> Result<Park> {
    let Some((item, rest)) = body.split_once(" → ") else {
        bail!("a park without `→`: {}", &body[..body.len().min(80)]);
    };
    let (home, trigger) = match rest.split_once(" Trigger: ") {
        Some((h, t)) => {
            let h = h.trim_end();
            let Some(h) = h.strip_suffix('.') else {
                bail!(
                    "the home before `Trigger:` does not end in a period: ...{}",
                    &h[h.len().saturating_sub(40)..]
                );
            };
            (h.to_string(), Some(t.trim().to_string()))
        }
        None if rest.contains("Trigger:") => bail!(
            "`Trigger:` is present but not as `. Trigger: `: {}",
            &rest[..rest.len().min(80)]
        ),
        None => (rest.trim().to_string(), None),
    };
    Ok(Park {
        id: Id {
            letter: 'K',
            number: 0,
        },
        area: area.to_string(),
        item: item.trim().to_string(),
        home,
        trigger,
        why: None,
        verdict: None,
        parked: Date::parse("1970-01-01").expect("a date"),
        parked_from: None,
        fired: None,
        fired_by: None,
        measured: None,
    })
}

/// Parks have no ids today (B6): date each from the commit that
/// introduced its line (B13), then number them oldest first, so K ids
/// read chronologically the way C ids do. Dates the prose lacks come
/// from git the same way.
fn fill_dates_from_git(root: &Path, out: &mut Imported) {
    let mut undated = Vec::new();
    for (k, p) in out.parks.iter_mut().enumerate() {
        let probe: String = p.item.chars().take(60).collect();
        match first_commit_date(root, &probe) {
            Some(d) => {
                p.parked = d;
                p.parked_from = Some("git".into());
            }
            None => undated.push(k),
        }
    }
    out.parks
        .sort_by(|a, b| a.parked.to_string().cmp(&b.parked.to_string()));
    for (k, p) in out.parks.iter_mut().enumerate() {
        p.id = Id {
            letter: 'K',
            number: k as u32 + 1,
        };
    }
    // The layouts named parks by item text; now they have ids.
    let by_item: BTreeMap<(String, String), Id> = out
        .parks
        .iter()
        .map(|p| ((p.area.clone(), p.item.clone()), p.id))
        .collect();
    for (name, meta) in out.areas.iter_mut() {
        meta.parks = meta
            .park_items
            .iter()
            .filter_map(|item| by_item.get(&(name.clone(), item.clone())).copied())
            .collect();
    }
    for k in undated {
        let p = &out.parks[k];
        out.exceptions.push(format!(
            "{}: git names no commit introducing this park; `parked` left at 1970-01-01",
            p.id
        ));
    }
    for d in out.decisions.iter_mut() {
        if d.history.is_some() || dated_in_prose(&d.ruling) {
            continue;
        }
        let probe = format!("**{} — ", d.id);
        match first_commit_date(root, &probe) {
            Some(date) => {
                d.decided = Some(date);
                d.decided_from = Some("git".into());
            }
            None => out.exceptions.push(format!(
                "{}: no date in the prose and git names no commit introducing it",
                d.id
            )),
        }
    }
}

fn dated_in_prose(text: &str) -> bool {
    Regex::new(r"\b(Decided|Ruled|Amended|confirmed live) \d{4}-\d{2}-\d{2}")
        .expect("fixed")
        .is_match(text)
}

/// `git log --reverse -S<probe>` over the registry files, first date.
fn first_commit_date(root: &Path, probe: &str) -> Option<Date> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--reverse", "--format=%as", "-S"])
        .arg(probe)
        .args(["--", "CONTEXT.md", "decisions"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text.lines().next()?.trim();
    Date::parse(first)
}

fn count_undeclared(out: &mut Imported) {
    let re = Regex::new(r"\b([A-Z])([1-9][0-9]*)\b").expect("fixed");
    let known = ['C', 'K', 'P', 'I', 'N', 'T', 'S', 'F'];
    let mut texts: Vec<&str> = Vec::new();
    for d in out
        .decisions
        .iter()
        .chain(&out.invariants)
        .chain(&out.principles)
    {
        texts.push(&d.ruling);
        texts.extend(d.why.as_deref());
        texts.extend(d.details.iter().map(String::as_str));
        texts.extend(d.pinned.iter().map(String::as_str));
        texts.extend(d.evidence.iter().map(String::as_str));
        texts.extend(d.history.as_deref());
    }
    for p in &out.parks {
        texts.push(&p.item);
        texts.push(&p.home);
        texts.extend(p.trigger.as_deref());
    }
    for t in texts {
        for c in re.captures_iter(t) {
            let letter = c[1].chars().next().expect("one");
            if !known.contains(&letter) {
                *out.undeclared.entry(letter).or_default() += 1;
            }
        }
    }
}

/// Writes the entries and a `bothy.toml` under `out_dir`. `root` is the
/// consumer's root the config will point at; when `out_dir` is inside it
/// the paths are relative, otherwise absolute (the scratch case).
pub fn write(imported: &Imported, out_dir: &Path, root: &Path) -> Result<Summary> {
    let root = root.canonicalize()?;
    std::fs::create_dir_all(out_dir)?;
    let out_dir = out_dir.canonicalize()?;
    let inside = out_dir.starts_with(&root);

    let mut written = 0;
    let mut write_entry = |sub: &str, stem: String, text: String| -> Result<()> {
        let dir = out_dir.join("bothy").join(sub);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{stem}.toml")), text)?;
        written += 1;
        Ok(())
    };
    for d in &imported.decisions {
        write_entry("decisions", d.id.file_stem(), emit::decision(d))?;
    }
    for d in &imported.invariants {
        write_entry("invariants", d.id.file_stem(), emit::decision(d))?;
    }
    for d in &imported.principles {
        write_entry("principles", d.id.file_stem(), emit::decision(d))?;
    }
    for p in &imported.parks {
        write_entry("parks", p.id.file_stem(), emit::park(p))?;
    }

    let templates = out_dir.join("templates");
    std::fs::create_dir_all(&templates)?;
    std::fs::write(templates.join("context.md"), &imported.context_template)?;
    let config = config_toml(imported, &root, &out_dir, inside);
    std::fs::write(out_dir.join("bothy.toml"), config)?;
    Ok(Summary {
        written,
        config: out_dir.join("bothy.toml"),
    })
}

pub struct Summary {
    pub written: usize,
    pub config: PathBuf,
}

/// The consumer's `bothy.toml`: areas from the files, namespaces for
/// what the registry owns and what it only cites, docs-check's scan set.
fn config_toml(imported: &Imported, root: &Path, out_dir: &Path, inside: bool) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# bothy.toml — acquisition's book, generated by `bothy import` and then kept by hand."
    );
    let _ = writeln!(
        s,
        "# Areas come from decisions/*.md, each with the curated order of its file; namespaces"
    );
    let _ = writeln!(
        s,
        "# are what the registry owns (C, K, P, I) and what it only cites (N, T, S, F); the"
    );
    let _ = writeln!(s, "# scan set is tools/docs-check.sh's.\n");
    if inside {
        let rel = pathdiff(out_dir, root);
        let _ = writeln!(s, "root = {}\n", emit::string(&rel));
    } else {
        let _ = writeln!(s, "root = {}\n", emit::string(&root.display().to_string()));
    }
    if let Some(o) = &imported.orientation {
        let _ = writeln!(
            s,
            "# What the project is. Every pack opens with it and CONTEXT.md prints it (B27).\norientation = {}\n",
            emit::string(o)
        );
    }
    for (name, meta) in &imported.areas {
        let _ = writeln!(s, "[area.{name}]");
        let _ = writeln!(s, "title = {}", emit::string(&meta.title));
        if let Some(t) = &meta.index_title
            && t != &meta.title
        {
            let _ = writeln!(s, "index_title = {}", emit::string(t));
        }
        if let Some(t) = &meta.intro {
            let _ = writeln!(s, "intro = {}", emit::string(t));
        }
        if let Some(t) = &meta.parked_intro {
            let _ = writeln!(s, "parked_intro = {}", emit::string(t));
        }
        if meta.paths.is_empty() {
            let _ = writeln!(
                s,
                "paths = []  # the crates and files this area governs; fill in by hand"
            );
        } else {
            let globs: Vec<String> = meta.paths.iter().map(|p| emit::string(p)).collect();
            let _ = writeln!(
                s,
                "# Read from the files this area's intro names; check it and add what the intro left out.\npaths = [{}]",
                globs.join(", ")
            );
        }
        let _ = writeln!(
            s,
            "also = []  # documents a session reads before touching this area, each `{{ path = \"...\", why = \"...\" }}`; fill in by hand"
        );
        if !meta.parks.is_empty() {
            let ids: Vec<String> = meta.parks.iter().map(|i| format!("\"{i}\"")).collect();
            let _ = writeln!(s, "parks = [{}]", ids.join(", "));
        }
        for (title, ids) in &meta.sections {
            let ids: Vec<String> = ids.iter().map(|i| format!("\"{i}\"")).collect();
            let _ = writeln!(
                s,
                "[[area.{name}.sections]]\ntitle = {}\nids = [{}]",
                emit::string(title),
                ids.join(", ")
            );
        }
        s.push('\n');
    }
    let template = if inside {
        "templates/context.md".to_string()
    } else {
        out_dir.join("templates/context.md").display().to_string()
    };
    let _ = writeln!(
        s,
        "# The rendered files. `header = false` keeps them byte-identical to the hand-kept\n# originals; turn it on at adoption so each file's first line names its source.\n[[projection]]\nout = \"CONTEXT.md\"\ntemplate = {}\nheader = false\n\n[[projection]]\nout = \"decisions/{{area}}.md\"\nfor_each = \"area\"\nexcept = [\"cross-cutting\"]\nheader = false\n",
        emit::string(&template)
    );
    let dir = |sub: &str| -> String {
        if inside {
            format!("bothy/{sub}")
        } else {
            out_dir.join("bothy").join(sub).display().to_string()
        }
    };
    for (letter, kind, sub) in [
        ('C', "decision", "decisions"),
        ('K', "park", "parks"),
        ('P', "principle", "principles"),
        ('I', "invariant", "invariants"),
    ] {
        let _ = writeln!(
            s,
            "[namespace.{letter}]\nkind = \"{kind}\"\ndir = {}\n",
            emit::string(&dir(sub))
        );
    }
    s.push_str(
        r#"# Ground-truth claims are authored master-side and only cited here.
[namespace.N]
kind = "external"
defined_in = "../../docs/design/network-ground-truth.md"
pattern = '^\*\*(N\d+)\.'

[namespace.T]
kind = "external"
defined_in = "../../docs/design/trade-ground-truth.md"
pattern = '^\*\*(T\d+)\.'

# The search slice's acceptance set: one row per S id in the digest.
[namespace.S]
kind = "external"
defined_in = "search/DIGEST.md"
pattern = '^(S\d+) ·'
# S256 in the daemon is the PKCE code-challenge method, not an id.
except = ["crates/acquisition-daemon/**"]

# F is a finding on master and a fault in the search slice (B10).
[namespace.F]
kind = "external"
defined_in = "../../docs/cleanup/findings.md"
pattern = '^### (F\d+)\.|^\| (F\d+) \|'

[namespace.F_fault]
letter = "F"
kind = "external"
scope = ["search/**", "SEARCH-SLICE.md", "crates/acquisition-search/**"]
defined_in = "crates/acquisition-search/tests/seat_faults.rs"
pattern = '\b(F\d+)\b'

# docs-check.sh's "anywhere": code, tools, skills, the control documents
# and the closed slice records; never the brainstorming notes.
[cite]
paths = [
  "crates/**/*.rs", "tools/*", "README.md", "AGENTS.md", "LIVE-TESTING.md",
  "RUN-LEDGER.md", "TESTING-NOTES.md", "SURFACES.md", "REFRESH-SLICE.md",
  "PRICING-SLICE.md", "SEARCH-SLICE.md", "DAEMON-SPLIT-SLICE.md",
  "NETWORK-CLEANUP.md", "search/DIGEST.md", "search/MEASUREMENTS.md",
  "search/LEDGER.md", ".claude/**/*.md",
]
exclude = ["brainstorming-notes/**", "crates/**/target/**"]

[enum.decision_status]
values = ["ruled", "superseded", "retired"]
"#,
    );
    s
}

pub(crate) fn pathdiff(from: &Path, to: &Path) -> String {
    let rel = from
        .strip_prefix(to)
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let ups = rel.split('/').filter(|c| !c.is_empty()).count();
    if ups == 0 {
        ".".to_string()
    } else {
        vec![".."; ups].join("/")
    }
}

/// The report `import` prints: counts, exceptions, undeclared letters.
pub fn report(imported: &Imported) -> String {
    let mut r = String::new();
    let _ = writeln!(
        r,
        "decisions {}  invariants {}  principles {}  parks {} ({} without a trigger)  areas {}",
        imported.decisions.len(),
        imported.invariants.len(),
        imported.principles.len(),
        imported.parks.len(),
        imported
            .parks
            .iter()
            .filter(|p| p.trigger.is_none())
            .count(),
        imported.areas.len()
    );
    let with_history = imported
        .decisions
        .iter()
        .filter(|d| d.history.is_some())
        .count();
    let from_git = imported
        .decisions
        .iter()
        .filter(|d| d.decided_from.is_some())
        .count();
    let _ = writeln!(
        r,
        "decisions dated in their prose {with_history}, dated from git {from_git}"
    );
    for (name, meta) in &imported.areas {
        if let Some(t) = &meta.index_title
            && t != &meta.title
        {
            let _ = writeln!(
                r,
                "note  area {name}: the index table says {t:?} and the file says {:?}",
                meta.title
            );
        }
        if !meta.paths_not_found.is_empty() {
            let _ = writeln!(
                r,
                "note  area {name}: its intro names files that are not where it points, so they are not in `paths`: {}",
                meta.paths_not_found.join(", ")
            );
        }
        if meta.paths.is_empty() && name != "cross-cutting" {
            let _ = writeln!(
                r,
                "note  area {name}: its intro names no file found on disk, so `paths` is empty and `pack --path` cannot reach it; fill it in by hand"
            );
        }
    }
    if !imported.undeclared.is_empty() {
        let list: Vec<String> = imported
            .undeclared
            .iter()
            .map(|(l, n)| format!("{l} ({n})"))
            .collect();
        let _ = writeln!(
            r,
            "note  letters named inside entries that no namespace defines: {}",
            list.join(", ")
        );
    }
    for e in &imported.exceptions {
        let _ = writeln!(r, "EXCEPTION  {e}");
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_intro_names_the_files_its_area_governs() {
        let root = std::env::temp_dir().join(format!("bothy-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for file in [
            "crates/acquisition-store/src/lib.rs",
            "crates/acquisition-store/src/snapshot.rs",
            "crates/acquisition-protocol/src/realm.rs",
            "crates/acquisition-cli/src/main.rs",
        ] {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(path, "").expect("write");
        }
        let intro = "The rulings that bind ingest and `PricePlan` (`acquisition-store/src/lib.rs`, \
                     `snapshot.rs`, `gone.rs`, `acquisition-protocol/src/realm.rs`, `acquisition-cli`). \
                     Read this before touching that code; the rest is in `acquisition-mcp`.";
        let (found, missing) = governed_paths(intro, &root);
        assert_eq!(
            found,
            [
                "crates/acquisition-store/src/lib.rs",
                "crates/acquisition-store/src/snapshot.rs",
                "crates/acquisition-protocol/src/realm.rs",
                "crates/acquisition-cli/**",
            ],
            "a path, a file beside the one before it, and a whole crate; a type is not a path, and nothing after \"Read this before\" is read"
        );
        assert_eq!(missing, ["crates/acquisition-store/src/gone.rs"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_plain_decision_splits_into_its_fields() {
        let line = "- **C54 — Authority is the coarser observation's.** A location is its full coordinate. *Why:* five rounds found it. *Details:* `lib.rs` doc, C54.";
        let d = parse_decision(line, "store").unwrap();
        assert_eq!(d.id.to_string(), "C54");
        assert_eq!(d.title, "Authority is the coarser observation's.");
        assert_eq!(d.ruling, "A location is its full coordinate.");
        assert_eq!(d.why.as_deref(), Some("five rounds found it."));
        assert_eq!(d.details, vec!["`lib.rs` doc, C54.".to_string()]);
        assert!(d.history.is_none());
    }

    #[test]
    fn a_title_joined_by_punctuation_keeps_the_punctuation_in_the_ruling() {
        let line = "- **C60 — The store's realm is the request's realm**, stamped from the params. *Evidence:* N42. Ruled 2026-09-02.";
        let d = parse_decision(line, "store").unwrap();
        assert_eq!(d.title, "The store's realm is the request's realm");
        assert_eq!(d.ruling, ", stamped from the params.");
        assert_eq!(d.evidence, vec!["N42.".to_string()]);
        assert_eq!(d.history.as_deref(), Some("Ruled 2026-09-02."));
    }

    #[test]
    fn the_dating_sentence_at_the_end_becomes_history_wherever_the_last_marker_is() {
        let line = "- **C1 — Cargo workspace.** Every crate's edges. *Why:* once. *Pinned:* `tools/docs-check.sh`. Amended 2026-09-09, 2026-09-17.";
        let d = parse_decision(line, "cross-cutting").unwrap();
        assert_eq!(d.pinned, vec!["`tools/docs-check.sh`.".to_string()]);
        assert_eq!(
            d.history.as_deref(),
            Some("Amended 2026-09-09, 2026-09-17.")
        );

        let line = "- **C57 — A skill is a property of its host.** Left in the host's json. *Details:* store doc, C57. Ruled 2026-09-02 (owner: (a), until GGG changes it).";
        let d = parse_decision(line, "store").unwrap();
        assert_eq!(
            d.history.as_deref(),
            Some("Ruled 2026-09-02 (owner: (a), until GGG changes it).")
        );
    }

    #[test]
    fn a_date_inside_the_ruling_stays_there() {
        let line = "- **C38 — Plans are binding.** Reviewed work stays exact. Decided 2026-08-31; **confirmed live 2026-09-01** with no friction. *Details:* `acquisition-plan` doc, C38.";
        let d = parse_decision(line, "cross-cutting").unwrap();
        assert!(d.ruling.contains("Decided 2026-08-31"));
        assert!(
            d.history.is_none(),
            "the date is not at the end, so it is not history"
        );
        assert!(dated_in_prose(&d.ruling));
    }

    #[test]
    fn a_repeated_marker_is_refused() {
        let line = "- **C9 — Two whys.** r. *Why:* a. *Why:* b.";
        let err = parse_decision(line, "x").unwrap_err();
        assert!(err.to_string().contains("appears twice"));
    }

    #[test]
    fn parks_split_on_the_arrow_and_the_trigger() {
        let p = parse_park(
            "Queue UI (drag-to-reorder) → a rendering problem by construction. Trigger: the GUI.",
            "frontends",
        )
        .unwrap();
        assert_eq!(p.item, "Queue UI (drag-to-reorder)");
        assert_eq!(p.home, "a rendering problem by construction");
        assert_eq!(p.trigger.as_deref(), Some("the GUI."));

        let p = parse_park(
            "Fact-path migration to uuid naming → opportunistic, or never; facts are refetchable.",
            "store",
        )
        .unwrap();
        assert_eq!(p.home, "opportunistic, or never; facts are refetchable.");
        assert!(p.trigger.is_none());

        assert!(parse_park("no arrow here. Trigger: x", "store").is_err());
        assert!(
            parse_park("a → b; Trigger: c", "store")
                .unwrap_err()
                .to_string()
                .contains("does not end in a period")
        );
    }

    #[test]
    fn history_starts_only_at_a_sentence() {
        assert_eq!(history_start("x. Ruled 2026-09-02."), Some(3));
        assert_eq!(history_start("Decided 2026-08-24."), Some(0));
        assert_eq!(
            history_start("`tests/realm_wire.rs`. Ruled 2026-09-02."),
            Some(23)
        );
        assert_eq!(
            history_start("the rule Decided 2026-08-24 applies"),
            None,
            "mid-sentence is prose"
        );
        assert_eq!(
            history_start("`tests/derive.rs`. 2026-09-17."),
            Some(19),
            "a bare trailing date is history"
        );
        assert_eq!(
            history_start("measured on 2026-09-17."),
            None,
            "a date inside a sentence is prose"
        );
    }
}
