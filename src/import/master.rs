//! The second grammar: the registers on acquisition's `master` branch,
//! read into claims and findings (B25, B26; the brief, §4.3 and §4.4).
//!
//! What it reads, as master writes it:
//!
//! - a claim is a paragraph under `## Claims ledger` in
//!   `docs/design/network-ground-truth.md` (`N`) or
//!   `docs/design/trade-ground-truth.md` (`T`): `**N<n>. <statement>**
//!   [<sources> — <confidence>]`, then its body, up to the next claim or
//!   heading. The `###` heading above it is its `layer`. A group of
//!   claims may share one tag, given once in a paragraph that begins
//!   "Source for all";
//! - an open finding is a heading under `## Open findings` in
//!   `docs/cleanup/findings.md`: `### F<n>. <title> — <status>;
//!   <disposition>`, then its body;
//! - a settled finding is a row of the table under `## Resolved ledger`:
//!   `| F<n> | <title> | <resolution> |`;
//! - a rule a settled finding left standing is a bullet under
//!   `## Standing constraints and lessons`: `- **F<n> — <rule>** <text>`,
//!   which becomes that finding's body.
//!
//! The tag is kept whole in `basis` and read into the typed fields
//! beside it (B7): the capital source words become `sources`, the first
//! of Confirmed, High or Provisional becomes `confidence`, and the
//! words after it become `scope`. Statements, bodies and titles are
//! kept verbatim, line breaks included (B18). Nothing under the source
//! tree is written (B4).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

use super::{Summary, pathdiff};
use crate::emit;
use crate::id::Id;
use crate::schema::{Claim, DEFAULT_CONFIDENCE, DEFAULT_FINDING_STATUS, Date, Finding};

/// The source words master's registers use, and the token each becomes.
/// `TOM` and `OWNER` are one person and one token; `OBS` and `OBSERVED`
/// are one kind of evidence.
pub const SOURCE_TAGS: &[(&str, &str)] = &[
    ("GGG-EMAIL", "ggg-email"),
    ("DOC", "doc"),
    ("INCIDENT", "incident"),
    ("OBSERVED", "obs"),
    ("OBS", "obs"),
    ("CODE", "code"),
    ("TOM", "owner"),
    ("OWNER", "owner"),
    ("HYP", "hyp"),
    ("COMMUNITY", "community"),
    ("SPIKE", "spike"),
    ("SITE", "site"),
    ("RUN", "run"),
];

/// Where each register is, relative to master's root.
pub const NETWORK: &str = "docs/design/network-ground-truth.md";
pub const TRADE: &str = "docs/design/trade-ground-truth.md";
pub const FINDINGS: &str = "docs/cleanup/findings.md";

#[derive(Debug, Default)]
pub struct Imported {
    /// `N` claims, in the order the register gives them.
    pub network: Vec<Claim>,
    /// `T` claims, in the order the register gives them.
    pub trade: Vec<Claim>,
    pub findings: Vec<Finding>,
    /// Text that looked like an entry and did not parse, with why.
    pub exceptions: Vec<String>,
    /// What the import decided that a reader should know.
    pub notes: Vec<String>,
}

/// Reads the three registers under `root`, master's checkout.
pub fn parse(root: &Path) -> Result<Imported> {
    let read = |rel: &str| -> Result<String> {
        std::fs::read_to_string(root.join(rel)).with_context(|| {
            format!(
                "cannot read {rel} under {}; the source is the root of a checkout of acquisition's master branch",
                root.display()
            )
        })
    };
    let mut out = Imported::default();
    out.network = parse_claims(&read(NETWORK)?, 'N', NETWORK, &mut out);
    out.trade = parse_claims(&read(TRADE)?, 'T', TRADE, &mut out);
    out.findings = parse_findings(&read(FINDINGS)?, &mut out);
    Ok(out)
}

fn source_word() -> Regex {
    let words: Vec<&str> = SOURCE_TAGS.iter().map(|(w, _)| *w).collect();
    Regex::new(&format!(r"\b({})\b", words.join("|"))).expect("a fixed pattern")
}

/// Every claim under `## Claims ledger`, in order.
pub fn parse_claims(text: &str, letter: char, rel: &str, out: &mut Imported) -> Vec<Claim> {
    let head = Regex::new(&format!(r"^\*\*{letter}([1-9][0-9]*)\. ")).expect("a fixed pattern");
    let lines: Vec<&str> = text.lines().collect();
    let mut claims: Vec<Claim> = Vec::new();
    let mut in_ledger = false;
    let mut layer = String::new();
    let mut group_tag: Option<String> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(title) = line.strip_prefix("## ") {
            in_ledger = title.trim() == "Claims ledger";
            layer.clear();
            group_tag = None;
            i += 1;
            continue;
        }
        if let Some(title) = line.strip_prefix("### ") {
            layer = title.trim().to_string();
            group_tag = None;
            i += 1;
            continue;
        }
        let Some(caps) = head.captures(line) else {
            if in_ledger && line.starts_with("Source for all") {
                let end = (i..lines.len())
                    .find(|&j| lines[j].trim().is_empty())
                    .unwrap_or(lines.len());
                group_tag = tag_in(&lines[i..end].join("\n"));
            }
            i += 1;
            continue;
        };
        let number: u32 = caps[1].parse().unwrap_or(0);
        let id = Id { letter, number };
        let end = (i + 1..lines.len())
            .find(|&j| {
                head.is_match(lines[j])
                    || lines[j].starts_with("## ")
                    || lines[j].starts_with("### ")
            })
            .unwrap_or(lines.len());
        let block = lines[i..end].join("\n");
        i = end;
        if !in_ledger {
            out.exceptions.push(format!(
                "{rel}: {id} is written like a claim outside the \"Claims ledger\" section; move it into the ledger, or reword the line so it does not open with a bold id"
            ));
            continue;
        }
        let opened = caps[0].len();
        let Some(close) = block[opened..].find("**") else {
            out.exceptions.push(format!(
                "{rel}: {id} opens a bold statement that never closes; end the statement with `**`"
            ));
            continue;
        };
        let statement = block[opened..opened + close].to_string();
        let rest = &block[opened + close + 2..];
        let (own_tag, body) = split_tag(rest);
        let basis = match (own_tag, &group_tag) {
            (Some(t), _) => Some(t),
            (None, Some(g)) => {
                out.notes.push(format!(
                    "{id} has no tag of its own and takes its group's: [{}]",
                    one_line(g)
                ));
                Some(g.clone())
            }
            (None, None) => {
                out.exceptions.push(format!(
                    "{rel}: {id} has no `[<sources> — <confidence>]` tag after its statement and its section gives none; add one"
                ));
                None
            }
        };
        let mut claim = Claim {
            id,
            layer: layer.clone(),
            statement,
            sources: Vec::new(),
            confidence: None,
            scope: None,
            basis: None,
            observed: None,
            body: body.to_string(),
        };
        if let Some(tag) = basis {
            read_tag(&tag, &mut claim);
            if claim.confidence.is_none() {
                out.notes.push(format!(
                    "{id} states no confidence in its tag: [{}]",
                    one_line(&tag)
                ));
            }
            claim.basis = Some(tag);
        }
        if claims.iter().any(|c| c.id == id) {
            out.exceptions.push(format!(
                "{rel}: {id} is defined twice; a claim number is never reused"
            ));
            continue;
        }
        claims.push(claim);
    }
    claims
}

/// The tag that opens `rest`, if one does, and the body after it. A
/// bracket is a tag only when it names a source word, so a quoted
/// `[item post="1"]` in a body is left where it is.
fn split_tag(rest: &str) -> (Option<String>, &str) {
    let trimmed = rest.trim_start();
    if let Some(inner) = trimmed.strip_prefix('[')
        && let Some(close) = inner.find(']')
        && source_word().is_match(&inner[..close])
    {
        let body = inner[close + 1..].trim_start_matches([' ', '\t']);
        let body = body.strip_prefix('\n').unwrap_or(body);
        return (Some(inner[..close].to_string()), trim_block(body));
    }
    (None, trim_block(rest))
}

/// The first bracket in `text` that names a source word.
fn tag_in(text: &str) -> Option<String> {
    let word = source_word();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let inner = &rest[open + 1..];
        let close = inner.find(']')?;
        if word.is_match(&inner[..close]) {
            return Some(inner[..close].to_string());
        }
        rest = &inner[close + 1..];
    }
    None
}

/// Reads a tag into the claim's typed fields (B7). The tag itself is
/// kept by the caller, so nothing here has to be reversible.
pub fn read_tag(tag: &str, claim: &mut Claim) {
    let flat = one_line(tag);
    for caps in source_word().captures_iter(&flat) {
        let token = SOURCE_TAGS
            .iter()
            .find(|(w, _)| *w == &caps[1])
            .map(|(_, t)| t.to_string())
            .expect("the pattern is built from the table");
        if !claim.sources.contains(&token) {
            claim.sources.push(token);
        }
    }
    let said = match flat.split_once(" — ") {
        Some((_, right)) => right,
        None => "",
    };
    // `HYP` on the confidence side is the hypothesis source, already
    // read above; the confidence is the word that follows it.
    let said = said.strip_prefix("HYP, ").unwrap_or(said);
    let word = Regex::new(r"\b(Confirmed|High|Provisional)\b").expect("a fixed pattern");
    if let Some(m) = word.find(said) {
        claim.confidence = Some(m.as_str().to_lowercase());
        let scope = if m.start() == 0 {
            said[m.end()..].trim_start_matches([',', ';', ' '])
        } else {
            said
        };
        if !scope.is_empty() {
            claim.scope = Some(scope.to_string());
        }
    } else if !said.is_empty() {
        claim.scope = Some(said.to_string());
    }
    let dates = full_dates(&flat);
    if let [one] = dates.as_slice() {
        claim.observed = Some(one.clone());
    }
}

/// Every "September 8, 2026" in `text`, as a date. A month without a
/// day, or a range of days, is not one day and is left in the words.
fn full_dates(text: &str) -> Vec<Date> {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let re = Regex::new(&format!(
        r"\b({}) ([0-9]{{1,2}}), ([0-9]{{4}})\b",
        MONTHS.join("|")
    ))
    .expect("a fixed pattern");
    re.captures_iter(text)
        .filter_map(|c| {
            let month = MONTHS.iter().position(|m| *m == &c[1])? + 1;
            let day: u32 = c[2].parse().ok()?;
            Date::parse(&format!("{}-{month:02}-{day:02}", &c[3]))
        })
        .collect()
}

/// Every finding: the open ones with their bodies, the settled ones
/// from the ledger, and the standing rules laid on the findings they
/// came from.
pub fn parse_findings(text: &str, out: &mut Imported) -> Vec<Finding> {
    let rel = FINDINGS;
    let heading = Regex::new(r"^### F([1-9][0-9]*)\. (.*)$").expect("a fixed pattern");
    let row = Regex::new(r"^\| *F([1-9][0-9]*) *\|").expect("a fixed pattern");
    let rule = Regex::new(r"^- \*\*F([1-9][0-9]*) — ").expect("a fixed pattern");
    let lines: Vec<&str> = text.lines().collect();
    let mut findings: BTreeMap<u32, Finding> = BTreeMap::new();
    let mut standing: Vec<(u32, String)> = Vec::new();
    let mut section = String::new();
    let mut add = |f: Finding, out: &mut Imported| {
        let n = f.id.number;
        match findings.entry(n) {
            Entry::Occupied(_) => out.exceptions.push(format!(
                "{rel}: F{n} is defined twice (an open heading and a ledger row, or two of either); keep one"
            )),
            Entry::Vacant(slot) => {
                slot.insert(f);
            }
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(title) = line.strip_prefix("## ") {
            section = title.trim().to_string();
            i += 1;
            continue;
        }
        if let Some(caps) = heading.captures(line) {
            let number: u32 = caps[1].parse().unwrap_or(0);
            let end = (i + 1..lines.len())
                .find(|&j| lines[j].starts_with("## ") || lines[j].starts_with("### "))
                .unwrap_or(lines.len());
            let body = trim_block(&lines[i + 1..end].join("\n")).to_string();
            i = end;
            let Some((title, said)) = caps[2].rsplit_once(" — ") else {
                out.exceptions.push(format!(
                    "{rel}: F{number}'s heading has no ` — <status>` after its title; add one"
                ));
                continue;
            };
            let word: String = said.chars().take_while(char::is_ascii_alphabetic).collect();
            let status = word.to_lowercase();
            if !DEFAULT_FINDING_STATUS.contains(&status.as_str()) {
                out.exceptions.push(format!(
                    "{rel}: F{number}'s status opens with \"{word}\", which is not one of: {}",
                    DEFAULT_FINDING_STATUS.join(", ")
                ));
                continue;
            }
            let disposition = said[word.len()..].trim_start_matches([';', ',', ' ']);
            add(
                Finding {
                    id: Id {
                        letter: 'F',
                        number,
                    },
                    title: title.to_string(),
                    status,
                    disposition: (!disposition.is_empty()).then(|| disposition.to_string()),
                    resolved_in: None,
                    body,
                },
                out,
            );
            continue;
        }
        if let Some(caps) = rule.captures(line) {
            let number: u32 = caps[1].parse().unwrap_or(0);
            let end = (i + 1..lines.len())
                .find(|&j| !lines[j].starts_with("  ") || lines[j].trim().is_empty())
                .unwrap_or(lines.len());
            let mut body = line[2..].to_string();
            for l in &lines[i + 1..end] {
                body.push('\n');
                body.push_str(l.strip_prefix("  ").unwrap_or(l));
            }
            standing.push((number, body));
            i = end;
            continue;
        }
        if let Some(caps) = row.captures(line) {
            let number: u32 = caps[1].parse().unwrap_or(0);
            let cells = table_cells(line);
            if section != "Resolved ledger" {
                out.exceptions.push(format!(
                    "{rel}: F{number} has a table row outside the \"Resolved ledger\" section; move it there"
                ));
            } else if let [_, title, resolution] = cells.as_slice() {
                add(
                    Finding {
                        id: Id {
                            letter: 'F',
                            number,
                        },
                        title: title.clone(),
                        status: "resolved".into(),
                        disposition: None,
                        resolved_in: Some(resolution.clone()),
                        body: String::new(),
                    },
                    out,
                );
            } else {
                out.exceptions.push(format!(
                    "{rel}: F{number}'s ledger row has {} cells where three are expected (id, finding, resolution); a `|` outside a code span splits a cell, so write it as `\\|`",
                    cells.len()
                ));
            }
        }
        i += 1;
    }
    for (number, body) in standing {
        match findings.get_mut(&number) {
            Some(f) => {
                f.disposition
                    .get_or_insert_with(|| "standing constraint".to_string());
                if f.body.is_empty() {
                    f.body = body;
                } else {
                    f.body = format!("{}\n\n{body}", f.body);
                }
            }
            None => out.exceptions.push(format!(
                "{rel}: a standing constraint names F{number}, which has neither an open heading nor a ledger row"
            )),
        }
    }
    let highest = findings.keys().next_back().copied().unwrap_or(0);
    let missing: Vec<String> = (1..=highest)
        .filter(|n| !findings.contains_key(n))
        .map(|n| format!("F{n}"))
        .collect();
    if !missing.is_empty() {
        out.notes.push(format!(
            "finding numbers with neither an open heading nor a ledger row: {}",
            missing.join(" ")
        ));
    }
    findings.into_values().collect()
}

/// The cells of one markdown table row. A `|` inside a backticked code
/// span is part of its cell, and so is an escaped `\|`.
pub fn table_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut in_code = false;
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => {
                in_code = !in_code;
                cell.push(c);
            }
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '|' if !in_code => {
                cells.push(cell.trim().to_string());
                cell.clear();
            }
            c => cell.push(c),
        }
    }
    if !cell.trim().is_empty() {
        cells.push(cell.trim().to_string());
    }
    // The row opens with a `|`, which leaves one empty cell before the first.
    if cells.first().is_some_and(String::is_empty) {
        cells.remove(0);
    }
    cells
}

/// `text` without the blank lines and the `---` rule around it.
fn trim_block(text: &str) -> &str {
    let mut t = text.trim_matches(['\n', '\r']);
    loop {
        let before = t.len();
        t = t.trim_end();
        t = t.strip_suffix("\n---").unwrap_or(t);
        if t.len() == before {
            return t;
        }
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Writes the entries and a `bothy.toml` under `out_dir`. `root` is
/// master's root; when `out_dir` is inside it the config's paths are
/// relative, otherwise absolute (the scratch case).
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
    for c in &imported.network {
        write_entry("claims/network", c.id.file_stem(), emit::claim(c))?;
    }
    for c in &imported.trade {
        write_entry("claims/trade", c.id.file_stem(), emit::claim(c))?;
    }
    for f in &imported.findings {
        write_entry("findings", f.id.file_stem(), emit::finding(f))?;
    }
    let config = config_toml(imported, &root, &out_dir, inside);
    std::fs::write(out_dir.join("bothy.toml"), config)?;
    Ok(Summary {
        written,
        config: out_dir.join("bothy.toml"),
    })
}

/// Master's `bothy.toml`: three namespaces, the scan set, and the
/// enumerations with exactly the source tokens the claims use (B7).
fn config_toml(imported: &Imported, root: &Path, out_dir: &Path, inside: bool) -> String {
    let mut s = String::from(
        "# bothy.toml — the book of acquisition's master branch, generated by `bothy import`\n\
         # and then kept by hand. It holds the ground-truth claims (N: the network, T: trade)\n\
         # and the findings register (F). It declares no area: a claim or a finding names none.\n\n",
    );
    let root_value = if inside {
        pathdiff(out_dir, root)
    } else {
        root.display().to_string()
    };
    let _ = writeln!(s, "root = {}\n", emit::string(&root_value));
    let dir = |sub: &str| -> String {
        if inside {
            format!("bothy/{sub}")
        } else {
            out_dir.join("bothy").join(sub).display().to_string()
        }
    };
    for (letter, kind, sub, more) in [
        ('N', "claim", "claims/network", ""),
        (
            'T',
            "claim",
            "claims/trade",
            "# \"Tab T1\" in this test is a stash tab's name, not a claim (B10).\nexcept = [\"tests/tst_workerupdate.cpp\"]\n",
        ),
        ('F', "finding", "findings", ""),
    ] {
        let _ = writeln!(
            s,
            "[namespace.{letter}]\nkind = \"{kind}\"\ndir = {}\n{more}",
            emit::string(&dir(sub))
        );
    }
    s.push_str(
        "# Where ids are named: the application, its tests, and the documents. The three\n\
         # registers are scanned too, so a claim that names a finding that does not exist\n\
         # is reported.\n\
         [cite]\n\
         paths = [\"src/**\", \"tests/**\", \"tools/**\", \"docs/**/*.md\", \"*.md\"]\n\n\
         # Closed enumerations (B7). A value outside its list is a check failure; the\n\
         # words that used to ride on a confidence are in each claim's `scope`.\n",
    );
    let quoted = |values: &[&str]| -> String {
        values
            .iter()
            .map(|v| format!("\"{v}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let _ = writeln!(
        s,
        "[enum.confidence]\nvalues = [{}]\n",
        quoted(DEFAULT_CONFIDENCE)
    );
    let mut used: Vec<&str> = Vec::new();
    for (_, token) in SOURCE_TAGS {
        let named = imported
            .network
            .iter()
            .chain(&imported.trade)
            .any(|c| c.sources.iter().any(|s| s == token));
        if named && !used.contains(token) {
            used.push(token);
        }
    }
    let _ = writeln!(s, "[enum.source]\nvalues = [{}]\n", quoted(&used));
    let _ = writeln!(
        s,
        "[enum.finding_status]\nvalues = [{}]",
        quoted(DEFAULT_FINDING_STATUS)
    );
    s
}

/// The report `import` prints: counts, what each confidence and source
/// came to, notes and exceptions.
pub fn report(imported: &Imported) -> String {
    let mut r = String::new();
    let claims: Vec<&Claim> = imported.network.iter().chain(&imported.trade).collect();
    let open = imported
        .findings
        .iter()
        .filter(|f| f.resolved_in.is_none())
        .count();
    let _ = writeln!(
        r,
        "claims {} (N {}, T {})  findings {} ({} from headings, {} from the resolved ledger)",
        claims.len(),
        imported.network.len(),
        imported.trade.len(),
        imported.findings.len(),
        open,
        imported.findings.len() - open
    );
    let mut tally = |label: &str, values: Vec<&str>| {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for v in values {
            *counts.entry(v).or_default() += 1;
        }
        let parts: Vec<String> = counts.iter().map(|(k, n)| format!("{k} {n}")).collect();
        let _ = writeln!(r, "{label}: {}", parts.join(", "));
    };
    tally(
        "confidence",
        claims
            .iter()
            .map(|c| c.confidence.as_deref().unwrap_or("(none stated)"))
            .collect(),
    );
    tally(
        "sources",
        claims
            .iter()
            .flat_map(|c| c.sources.iter().map(String::as_str))
            .collect(),
    );
    tally(
        "finding status",
        imported
            .findings
            .iter()
            .map(|f| f.status.as_str())
            .collect(),
    );
    for n in &imported.notes {
        let _ = writeln!(r, "note  {n}");
    }
    for e in &imported.exceptions {
        let _ = writeln!(r, "FAIL  {e}");
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim_from(tag: &str) -> Claim {
        let mut c = Claim {
            id: Id {
                letter: 'N',
                number: 1,
            },
            layer: String::new(),
            statement: "s".into(),
            sources: Vec::new(),
            confidence: None,
            scope: None,
            basis: None,
            observed: None,
            body: String::new(),
        };
        read_tag(tag, &mut c);
        c
    }

    #[test]
    fn a_tag_becomes_sources_a_confidence_and_a_scope() {
        let c = claim_from("SITE — Confirmed for 15 of 22 groups; September 8, 2026");
        assert_eq!(c.sources, ["site"]);
        assert_eq!(c.confidence.as_deref(), Some("confirmed"));
        assert_eq!(
            c.scope.as_deref(),
            Some("for 15 of 22 groups; September 8, 2026")
        );
        assert_eq!(
            c.observed.map(|d| d.to_string()).as_deref(),
            Some("2026-09-08")
        );
    }

    #[test]
    fn tom_and_owner_are_one_source_and_a_wrapped_tag_reads_the_same() {
        let c = claim_from("INCIDENT +\nGGG-EMAIL — Confirmed");
        assert_eq!(c.sources, ["incident", "ggg-email"]);
        assert_eq!(c.scope, None);
        let c = claim_from("TOM + Cloudflare public docs — High");
        assert_eq!(c.sources, ["owner"]);
        assert_eq!(c.confidence.as_deref(), Some("high"));
        let c = claim_from("OWNER — Confirmed; September 7, 2026");
        assert_eq!(c.sources, ["owner"]);
    }

    #[test]
    fn a_hypothesis_is_a_source_and_its_confidence_is_the_word_after() {
        let c = claim_from("TOM reasoning — HYP, Provisional");
        assert_eq!(c.sources, ["owner", "hyp"]);
        assert_eq!(c.confidence.as_deref(), Some("provisional"));
        assert_eq!(c.scope, None);
    }

    #[test]
    fn a_tag_with_no_confidence_word_keeps_its_words_as_scope() {
        let c = claim_from("CODE — what worked once; read September 3–4, 2026");
        assert_eq!(c.confidence, None);
        assert_eq!(
            c.scope.as_deref(),
            Some("what worked once; read September 3–4, 2026")
        );
        assert_eq!(c.observed, None, "a range of days is not one day");
    }

    #[test]
    fn a_pipe_in_a_code_span_stays_in_its_cell() {
        let cells = table_cells("| F61 | keys on `a || b` only | Fixed, PR #1 |");
        assert_eq!(cells, ["F61", "keys on `a || b` only", "Fixed, PR #1"]);
    }
}
