//! `pack`: what a session reads before one task, in one call (B15).
//!
//! The order is fixed and goes from what binds everything to what binds
//! least: the project's orientation (B27), invariants, principles,
//! cross-cutting rulings, the areas' rulings, their parks, the
//! cross-cutting parks, the documents to read also, then the citation
//! map that says where each ruling is held. Retired entries are omitted
//! (B3). A budget trims whole blocks from the tail and the footer names
//! each one trimmed; nothing leaves silently.
//!
//! The text is written for a reader who has never seen the project and
//! is the smallest model in use (B17).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use serde::Serialize;

use crate::book::{Book, Loaded};
use crate::cite::{self, Citation};
use crate::config::Kind;
use crate::id::Id;

/// What the caller asked for.
#[derive(Debug, Default)]
pub struct Request {
    /// Areas to pack, by name.
    pub areas: Vec<String>,
    /// Files about to be touched; each resolves to the areas whose
    /// `paths` cover it (B19).
    pub paths: Vec<String>,
    /// One entry: itself, what it names, what names it, where it is held.
    pub for_id: Option<Id>,
    /// Token budget; see [`ESTIMATOR`].
    pub budget: Option<usize>,
}

/// The name of the token estimate, printed beside every number it
/// produces. The exact tokenizer is parked (K2).
pub const ESTIMATOR: &str = "chars/4 estimate";

/// Tokens, estimated as the text's characters divided by four.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

#[derive(Serialize, Debug)]
pub struct Pack {
    pub text: String,
    pub bytes: usize,
    pub tokens: usize,
    pub estimator: &'static str,
    /// The commit the pack was built at, with `+` when the tree has
    /// uncommitted changes; `None` outside git.
    pub commit: Option<String>,
    pub areas: Vec<String>,
    pub entries: Vec<Id>,
    /// Labels of the blocks a budget removed, in the order removed.
    pub trimmed: Vec<String>,
}

/// One unit the budget may remove, from the tail first.
struct Block {
    label: String,
    text: String,
}

pub fn build(book: &Book, req: &Request) -> Result<Pack> {
    let mut areas: Vec<String> = req.areas.clone();
    for p in &req.paths {
        let rel = Path::new(p);
        let mut hit: Vec<String> = book
            .config
            .area
            .iter()
            .filter(|(_, a)| cite::covered(&a.paths, rel))
            .map(|(name, _)| name.clone())
            .collect();
        if hit.is_empty() {
            let declared: Vec<String> = book
                .config
                .area
                .iter()
                .filter(|(_, a)| !a.paths.is_empty())
                .map(|(n, a)| format!("{n} ({})", a.paths.join(", ")))
                .collect();
            bail!(
                "no area's paths cover {p}; the areas with paths are: {}. Pass --area <name>, or add the file to an area's `paths` in bothy.toml",
                if declared.is_empty() {
                    "none".to_string()
                } else {
                    declared.join("; ")
                }
            );
        }
        areas.append(&mut hit);
    }
    areas.sort();
    areas.dedup();
    for a in &areas {
        if !book.config.area.contains_key(a) {
            let known: Vec<&str> = book.config.area.keys().map(String::as_str).collect();
            bail!(
                "area \"{a}\" is not declared in bothy.toml; the areas are: {}",
                known.join(", ")
            );
        }
    }
    if areas.is_empty() && req.for_id.is_none() {
        bail!("say what the pack is for: --area <name>, --path <file>, or --for <id>");
    }

    let citations = cite::scan(book)?;
    let mut blocks: Vec<Block> = Vec::new();
    let mut included: Vec<Id> = Vec::new();

    if let Some(id) = req.for_id {
        pack_for(book, id, &citations, &mut blocks, &mut included)?;
    }
    if !areas.is_empty() {
        pack_areas(book, &areas, &citations, &mut blocks, &mut included);
    }

    let commit = commit_at(&book.root);
    let mut trimmed = Vec::new();
    let text = loop {
        let body: String = blocks.iter().map(|b| b.text.as_str()).collect();
        let footer = footer(&body, included.len(), commit.as_deref(), &trimmed);
        let full = format!("{body}{footer}");
        match req.budget {
            Some(budget) if estimate_tokens(&full) > budget && blocks.len() > 1 => {
                let gone = blocks.pop().expect("more than one block");
                trimmed.push(gone.label);
            }
            _ => break full,
        }
    };

    Ok(Pack {
        bytes: text.len(),
        tokens: estimate_tokens(&text),
        estimator: ESTIMATOR,
        commit,
        areas,
        entries: included,
        trimmed,
        text,
    })
}

fn pack_areas(
    book: &Book,
    areas: &[String],
    citations: &[Citation],
    blocks: &mut Vec<Block>,
    included: &mut Vec<Id>,
) {
    let titles: Vec<String> = areas
        .iter()
        .map(|a| format!("`{a}` ({})", book.config.area[a].title))
        .collect();
    let mut head = format!(
        "# What to read before touching {}\n\n",
        titles.join(" and ")
    );
    // What the project is comes first: the rulings below name things
    // only that sentence explains (B27).
    if let Some(o) = &book.config.orientation
        && !o.trim().is_empty()
    {
        let _ = write!(head, "{}\n\n", o.trim());
    }
    head.push_str(
        "This pack is the project's book, assembled for one task. Rulings bind: \
         if the code you are about to change holds one, change the ruling first or \
         cite it in the change. Parks are scope deferred on purpose; do not build \
         one unless its trigger has fired.\n\n",
    );
    let governed: Vec<&str> = areas
        .iter()
        .flat_map(|a| book.config.area[a].paths.iter().map(String::as_str))
        .collect();
    if !governed.is_empty() {
        let _ = writeln!(head, "Files these areas govern: {}\n", governed.join(", "));
    }
    blocks.push(Block {
        label: "the heading".into(),
        text: head,
    });

    let live = |l: &&Loaded| !l.entry.is_retired();

    let invariants: Vec<&Loaded> = book
        .entries
        .iter()
        .filter(|l| l.kind == Kind::Invariant)
        .filter(live)
        .collect();
    if !invariants.is_empty() {
        push_section(
            blocks,
            included,
            "## Invariants (never break these)\n\n",
            "invariant",
            &invariants,
        );
    }
    let principles: Vec<&Loaded> = book
        .entries
        .iter()
        .filter(|l| l.kind == Kind::Principle)
        .filter(live)
        .collect();
    if !principles.is_empty() {
        push_section(
            blocks,
            included,
            "## Principles (how this project works)\n\n",
            "principle",
            &principles,
        );
    }
    let cross: Vec<&Loaded> = book
        .entries
        .iter()
        .filter(|l| l.kind == Kind::Decision && l.entry.area() == Some("cross-cutting"))
        .filter(live)
        .collect();
    if !cross.is_empty() && !areas.iter().any(|a| a == "cross-cutting") {
        push_section(
            blocks,
            included,
            "## Cross-cutting rulings (bind every area)\n\n",
            "cross-cutting ruling",
            &cross,
        );
    }
    for area in areas {
        let rulings: Vec<&Loaded> = book
            .entries
            .iter()
            .filter(|l| l.kind == Kind::Decision && l.entry.area() == Some(area))
            .filter(live)
            .collect();
        if !rulings.is_empty() {
            push_section(
                blocks,
                included,
                &format!("## Rulings for `{area}`\n\n"),
                &format!("`{area}` ruling"),
                &rulings,
            );
        }
    }
    for area in areas {
        let parks: Vec<&Loaded> = book
            .entries
            .iter()
            .filter(|l| l.kind == Kind::Park && l.entry.area() == Some(area))
            .filter(live)
            .collect();
        if !parks.is_empty() {
            push_section(
                blocks,
                included,
                &format!(
                    "## Parked in `{area}` (deferred; each with the trigger that reopens it)\n\n"
                ),
                &format!("`{area}` park"),
                &parks,
            );
        }
    }
    // Scope deferred across every area is deferred in this one (B15).
    let cross_parks: Vec<&Loaded> = book
        .entries
        .iter()
        .filter(|l| l.kind == Kind::Park && l.entry.area() == Some("cross-cutting"))
        .filter(live)
        .collect();
    if !cross_parks.is_empty() && !areas.iter().any(|a| a == "cross-cutting") {
        push_section(
            blocks,
            included,
            "## Parked across every area (deferred; each with the trigger that reopens it)\n\n",
            "cross-cutting park",
            &cross_parks,
        );
    }
    let also: Vec<String> = areas
        .iter()
        .flat_map(|a| book.config.area[a].also.iter())
        .map(|doc| {
            let path = doc.path();
            let size = match std::fs::metadata(book.root.join(path)) {
                Ok(m) => format!("{} bytes", m.len()),
                Err(_) => "missing".to_string(),
            };
            match doc.why() {
                Some(why) => format!("- `{path}` ({size}): {why}"),
                None => format!("- `{path}` ({size})"),
            }
        })
        .collect();
    if !also.is_empty() {
        blocks.push(Block {
            label: "the read-also list".into(),
            text: format!(
                "## Read also (not in this pack; open what your task needs)\n\n{}\n\n",
                also.join("\n")
            ),
        });
    }
    push_citation_map(blocks, included, citations);
}

fn pack_for(
    book: &Book,
    id: Id,
    citations: &[Citation],
    blocks: &mut Vec<Block>,
    included: &mut Vec<Id>,
) -> Result<()> {
    let loaded = book
        .get(id)
        .ok_or_else(|| anyhow!("no entry {id} in this book; `bothy list decisions` and `bothy list parks` show what exists"))?;
    let mut head = format!("# {id} and what touches it\n\n");
    head.push_str(&render_entry(loaded));
    head.push('\n');
    included.push(id);
    blocks.push(Block {
        label: format!("{id} itself"),
        text: head,
    });

    let names: Vec<&Loaded> = cite::cross_references(book, &loaded.entry.prose())
        .into_iter()
        .filter(|x| *x != id)
        .filter_map(|x| book.get(x))
        .collect();
    if !names.is_empty() {
        push_section(
            blocks,
            included,
            &format!("## What {id} names\n\n"),
            "named entry",
            &names,
        );
    }
    let named_by: Vec<&Loaded> = book
        .entries
        .iter()
        .filter(|l| l.entry.id() != id && !included.contains(&l.entry.id()))
        .filter(|l| cite::cross_references(book, &l.entry.prose()).contains(&id))
        .collect();
    if !named_by.is_empty() {
        push_section(
            blocks,
            included,
            &format!("## What names {id}\n\n"),
            "naming entry",
            &named_by,
        );
    }
    push_citation_map(blocks, included, citations);
    Ok(())
}

fn push_section(
    blocks: &mut Vec<Block>,
    included: &mut Vec<Id>,
    heading: &str,
    label: &str,
    entries: &[&Loaded],
) {
    blocks.push(Block {
        label: format!("the {label}s heading"),
        text: heading.to_string(),
    });
    for l in entries {
        included.push(l.entry.id());
        blocks.push(Block {
            label: format!("{label} {}", l.entry.id()),
            text: render_entry(l),
        });
    }
    if let Some(last) = blocks.last_mut() {
        last.text.push('\n');
    }
}

/// One entry as a markdown bullet, in the registry's own form (B18),
/// with the id in bold so a reader can find it again.
fn render_entry(l: &Loaded) -> String {
    let line = crate::render::entry_line(l);
    let mut s = match l.kind {
        Kind::Park => line.replacen("- ", &format!("- **{}** ", l.entry.id()), 1),
        // A claim or a finding is paragraphs, not a bullet; a blank line
        // keeps the next entry out of its last paragraph.
        Kind::Claim | Kind::Finding => format!("{line}\n"),
        _ => line,
    };
    s.push('\n');
    s
}

/// Where each included ruling is held: id → file:line, so the reader
/// knows which code and tests carry the mechanism.
fn push_citation_map(blocks: &mut Vec<Block>, included: &[Id], citations: &[Citation]) {
    let mut by_id: BTreeMap<Id, Vec<String>> = BTreeMap::new();
    for c in citations {
        if included.contains(&c.id) {
            let site = format!("{}:{}", c.path.display(), c.line);
            let sites = by_id.entry(c.id).or_default();
            if sites.last() != Some(&site) {
                sites.push(site);
            }
        }
    }
    let mut ids: Vec<Id> = included.to_vec();
    ids.dedup();
    let mut text = String::from("## Where each entry is held (file:line that names it)\n\n");
    for id in &ids {
        match by_id.get(id) {
            Some(sites) => {
                let _ = writeln!(text, "- {id}: {}", sites.join(", "));
            }
            None => {
                let _ = writeln!(text, "- {id}: nothing cites it yet");
            }
        }
    }
    text.push('\n');
    blocks.push(Block {
        label: "the citation map".into(),
        text,
    });
}

fn footer(body: &str, entries: usize, commit: Option<&str>, trimmed: &[String]) -> String {
    let mut f = String::from("---\n");
    let _ = write!(
        f,
        "pack: {entries} entries · {} bytes · ~{} tokens ({ESTIMATOR}) · built at {}",
        body.len(),
        estimate_tokens(body),
        commit.unwrap_or("no git commit")
    );
    if !trimmed.is_empty() {
        let _ = write!(
            f,
            "\ntrimmed to fit the budget, from the tail: {}",
            trimmed.join(", ")
        );
    }
    f.push('\n');
    f
}

/// The short hash of HEAD under `root`, with `+` if the tree is dirty.
pub fn commit_at(root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let mut hash = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let dirty = std::process::Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .is_some_and(|o| o.status.success() && !o.stdout.is_empty());
    if dirty {
        hash.push('+');
    }
    Some(hash)
}
