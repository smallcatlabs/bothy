//! The brief's done-criterion 4: the registers on acquisition's master
//! branch import as claims and findings under a second `bothy.toml`,
//! and the verbs that read a book read that one with no branch for it
//! (B25, B26).
//!
//! The first tests build a small master in a temporary directory, so
//! they run anywhere. The last reads the owner's clone, whose `master`
//! branch is not the one checked out there: it is read with `git
//! archive` into a temporary directory, and the clone is never written
//! (B4). `$BOTHY_ACQUISITION_MASTER` names a checkout of master to read
//! instead. Its counts are floors, because the branch moves, and it is
//! skipped with a note when neither is present.

use std::path::{Path, PathBuf};
use std::process::Command;

use bothy::import::master;
use bothy::{Book, Level, check, pack};

const NETWORK: &str = r#"# Network Ground Truth

## Claims ledger

### Layer 1 — Cloudflare

**N1. A protection layer sits in front of the API; its rules
are opaque.** [INCIDENT + TOM — Confirmed]
Discovered via the incident in N2.

**N2. The one known block was triggered by a burst.** [INCIDENT +
GGG-EMAIL — Confirmed]
The report, verbatim:

> "It made over a thousand requests in a minute."

The origin of the F5 standing constraint.

### First capture (July 18, 2026)

Source for all claims in this group: the first instrumented session
[OBS — Confirmed for this session; one account], July 18, 2026.

**N3. Observed policy topology.**
Five policies, confirming N1.

| Policy | Rules |
|---|---|
| `stash-request-limit` | Account |

---

## Open questions

- **Q1. Unresolved.** Nothing here is a claim.
"#;

const TRADE: &str = r#"# Trade Ground Truth

## Claims ledger

### The trade site

**T1. Categories are grouped.** [SITE — Confirmed for 15 of 22 groups;
September 8, 2026]
A quoted `[item post="1" index="1"]` in a body is not a tag.

**T2. What worked once.** [CODE — what worked once; read September 3–4, 2026]
"#;

const FINDINGS: &str = r#"# Findings Register

## Open findings

### F22. Dual persistence paths in `BuyoutManager` — Confirmed; won't fix unless touched

Buyouts persist two ways. Related: F5.

### F74. The cookie is not host-scoped — Likely

A gap needing a runtime check.

---

## Standing constraints and lessons

- **F5 — one HEAD at a time.** At most one HEAD may ever be in
  flight (N2).

---

## Resolved ledger

| F | Finding | Resolution |
|----|---------|------------|
| F5 | `SetupEndpoint` nested event loop | Standing constraint (above) |
| F61 | Keys purely on the selection (`m_update_all || m_tabs.count(id)`) | Fixed, PR #161 |
"#;

/// A small master under a fresh temporary directory: the three
/// registers and one source file that cites them.
fn small_master(name: &str, source: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("bothy-master-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (rel, text) in [
        (master::NETWORK, NETWORK),
        (master::TRADE, TRADE),
        (master::FINDINGS, FINDINGS),
        ("src/limiter.cpp", source),
    ] {
        let path = root.join("tree").join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }
    root
}

fn failures(report: &bothy::Report) -> Vec<String> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.level == Level::Fail)
        .map(ToString::to_string)
        .collect()
}

#[test]
fn the_registers_become_claims_and_findings() {
    let root = small_master("parse", "// one HEAD at a time (N2, F5)\n");
    let imported = master::parse(&root.join("tree")).expect("parses");

    assert_eq!(imported.exceptions, Vec::<String>::new());
    assert_eq!(imported.network.len(), 3);
    assert_eq!(imported.trade.len(), 2);
    assert_eq!(imported.findings.len(), 4);

    let n2 = &imported.network[1];
    assert_eq!(n2.layer, "Layer 1 — Cloudflare");
    assert_eq!(
        n2.statement,
        "The one known block was triggered by a burst."
    );
    assert_eq!(n2.sources, ["incident", "ggg-email"]);
    assert_eq!(n2.confidence.as_deref(), Some("confirmed"));
    assert_eq!(
        n2.basis.as_deref(),
        Some("INCIDENT +\nGGG-EMAIL — Confirmed")
    );
    assert!(n2.body.starts_with("The report, verbatim:"));
    assert!(
        n2.body
            .ends_with("The origin of the F5 standing constraint.")
    );

    // A claim with no tag of its own takes its group's, and says so.
    let n3 = &imported.network[2];
    assert_eq!(n3.sources, ["obs"]);
    assert_eq!(n3.scope.as_deref(), Some("for this session; one account"));
    assert!(n3.body.ends_with("| `stash-request-limit` | Account |"));
    assert!(
        imported
            .notes
            .iter()
            .any(|n| n.starts_with("N3 has no tag"))
    );

    let t1 = &imported.trade[0];
    assert_eq!(
        t1.scope.as_deref(),
        Some("for 15 of 22 groups; September 8, 2026")
    );
    assert_eq!(
        t1.observed.as_ref().map(ToString::to_string).as_deref(),
        Some("2026-09-08")
    );
    assert!(t1.body.contains("[item post="));
    assert_eq!(imported.trade[1].confidence, None);

    let by_id = |n: u32| {
        imported
            .findings
            .iter()
            .find(|f| f.id.number == n)
            .expect("the finding")
    };
    let f22 = by_id(22);
    assert_eq!(f22.title, "Dual persistence paths in `BuyoutManager`");
    assert_eq!(f22.status, "confirmed");
    assert_eq!(f22.disposition.as_deref(), Some("won't fix unless touched"));
    assert_eq!(f22.body, "Buyouts persist two ways. Related: F5.");
    assert_eq!(by_id(74).status, "likely");
    assert_eq!(by_id(74).disposition, None);

    // A settled finding is the ledger's row; the rule it left standing
    // is its body.
    let f5 = by_id(5);
    assert_eq!(f5.status, "resolved");
    assert_eq!(
        f5.resolved_in.as_deref(),
        Some("Standing constraint (above)")
    );
    assert_eq!(f5.disposition.as_deref(), Some("standing constraint"));
    assert!(f5.body.starts_with("**F5 — one HEAD at a time.**"));
    assert!(f5.body.ends_with("flight (N2)."));
    assert_eq!(by_id(61).resolved_in.as_deref(), Some("Fixed, PR #161"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn what_looks_like_an_entry_and_is_not_one_is_an_exception() {
    let mut out = master::Imported::default();
    let stray = "**N9. A bold id above the ledger.**\n\n## Claims ledger\n\n**N1. No tag follows.**\nBody.\n";
    let claims = master::parse_claims(stray, 'N', master::NETWORK, &mut out);
    // The untagged claim is kept, so the count is right; the exception
    // is what makes the import exit non-zero.
    assert_eq!(claims.len(), 1);
    assert_eq!(out.exceptions.len(), 2, "{:?}", out.exceptions);
    assert!(out.exceptions[0].contains("N9 is written like a claim outside"));
    assert!(out.exceptions[1].contains("N1 has no `[<sources> — <confidence>]` tag"));

    let mut out = master::Imported::default();
    let register = "## Open findings\n\n### F1. No status\n\n### F2. Odd — Maybe; later\n\n## Resolved ledger\n\n| F3 | a | b | c |\n| F3 | a | b |\n| F3 | a | b |\n";
    let findings = master::parse_findings(register, &mut out);
    assert_eq!(findings.len(), 1);
    for expected in [
        "F1's heading has no ` — <status>`",
        "F2's status opens with \"Maybe\"",
        "F3's ledger row has 4 cells",
        "F3 is defined twice",
    ] {
        assert!(
            out.exceptions.iter().any(|e| e.contains(expected)),
            "no exception says {expected:?} in {:?}",
            out.exceptions
        );
    }
}

#[test]
fn the_imported_book_opens_checks_lists_and_packs() {
    let root = small_master("book", "// one HEAD at a time (N2, F5)\n");
    let tree = root.join("tree");
    let imported = master::parse(&tree).expect("parses");
    let summary = master::write(&imported, &root.join("book"), &tree).expect("writes");
    assert_eq!(summary.written, 9);

    let book = Book::open(&summary.config).expect("the written book opens");
    assert!(
        book.config.area.is_empty(),
        "master's book declares no area"
    );
    let report = check::run(&book).expect("check runs");
    assert_eq!(failures(&report), Vec::<String>::new());
    assert_eq!(report.entries, 9);
    // The claim that states no confidence is reported, not refused.
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.level == Level::Note && d.message.contains("T2 states no confidence"))
    );

    // A settled finding keeps its file and leaves the live set.
    let live: Vec<String> = book
        .entries
        .iter()
        .filter(|l| l.kind_name() == "finding" && !l.entry.is_retired())
        .map(|l| l.entry.id().to_string())
        .collect();
    assert_eq!(live, ["F22", "F74"]);

    // A pack for one finding carries what names it and where it is held.
    let request = pack::Request {
        for_id: Some(bothy::Id::parse("F5").expect("an id")),
        ..Default::default()
    };
    let packed = pack::build(&book, &request).expect("packs");
    assert!(
        packed
            .text
            .contains("**F5. `SetupEndpoint` nested event loop** — resolved")
    );
    assert!(packed.text.contains("## What names F5"));
    assert!(packed.text.contains("**N2. The one known block"));
    assert!(packed.text.contains("src/limiter.cpp:1"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn check_refuses_a_citation_of_no_entry_and_a_value_outside_its_enum() {
    let root = small_master("refuse", "// see N7 and F5\n");
    let tree = root.join("tree");
    let imported = master::parse(&tree).expect("parses");
    let out = root.join("book");
    master::write(&imported, &out, &tree).expect("writes");

    // The phrasing the enum replaced, written by hand into one claim,
    // and a source and a status no list holds.
    let n1 = out.join("bothy/claims/network/N001.toml");
    let text = std::fs::read_to_string(&n1).expect("reads");
    let text = text
        .replace(
            "confidence = \"confirmed\"",
            "confidence = \"Confirmed for one account\"",
        )
        .replace("\"owner\"", "\"folklore\"");
    std::fs::write(&n1, text).expect("writes");
    let f74 = out.join("bothy/findings/F074.toml");
    let text = std::fs::read_to_string(&f74).expect("reads");
    std::fs::write(&f74, text.replace("\"likely\"", "\"probably\"")).expect("writes");

    let book = Book::open(&out.join("bothy.toml")).expect("opens");
    let report = check::run(&book).expect("check runs");
    let failed = failures(&report);
    for expected in [
        "src/limiter.cpp:1  cites N7, which no namespace covering this file defines",
        "confidence \"Confirmed for one account\" is not in the confidence enum",
        "goes in `scope`",
        "source \"folklore\" is not in the source enum",
        "status \"probably\" is not in the finding_status enum",
    ] {
        assert!(
            failed.iter().any(|f| f.contains(expected)),
            "no failure says {expected:?} in:\n{}",
            failed.join("\n")
        );
    }
    assert_eq!(failed.len(), 4, "{}", failed.join("\n"));

    let _ = std::fs::remove_dir_all(&root);
}

/// A checkout of master to read: `$BOTHY_ACQUISITION_MASTER`, or the
/// `master` branch of the owner's clone, archived into `into`.
fn real_master(into: &Path) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("BOTHY_ACQUISITION_MASTER") {
        let path = PathBuf::from(path);
        return path.join(master::NETWORK).is_file().then_some(path);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let clone = PathBuf::from(home).join("Development/GitHub/gerwaric/acquisition");
    if !clone.join(".git").exists() {
        return None;
    }
    let tree = into.join("tree");
    std::fs::create_dir_all(&tree).ok()?;
    let archive = into.join("master.tar");
    let archived = Command::new("git")
        .arg("-C")
        .arg(&clone)
        .args(["archive", "--output"])
        .arg(&archive)
        .args(["master", "docs", "src", "tests", "tools"])
        .status()
        .ok()?;
    let unpacked = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&tree)
        .status()
        .ok()?;
    (archived.success() && unpacked.success() && tree.join(master::NETWORK).is_file())
        .then_some(tree)
}

#[test]
fn acquisition_master_imports_whole_and_passes_check() {
    let scratch = std::env::temp_dir().join(format!("bothy-master-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let Some(tree) = real_master(&scratch) else {
        eprintln!(
            "skipped: no acquisition clone with a master branch at the default path, and no $BOTHY_ACQUISITION_MASTER"
        );
        return;
    };
    let imported = master::parse(&tree).expect("parses");
    assert!(
        imported.exceptions.is_empty(),
        "exceptions:\n{}",
        imported.exceptions.join("\n")
    );
    assert!(imported.network.len() >= 46, "{} N", imported.network.len());
    assert!(imported.trade.len() >= 34, "{} T", imported.trade.len());
    assert!(
        imported.findings.len() >= 78,
        "{} F",
        imported.findings.len()
    );
    // Every claim's tag is kept whole and read into at least one source.
    for claim in imported.network.iter().chain(&imported.trade) {
        assert!(claim.basis.is_some(), "{} has no basis", claim.id);
        assert!(!claim.sources.is_empty(), "{} has no source", claim.id);
    }

    let summary = master::write(&imported, &scratch.join("book"), &tree).expect("writes");
    let book = Book::open(&summary.config).expect("the written book opens");
    let report = check::run(&book).expect("check runs");
    assert_eq!(failures(&report), Vec::<String>::new());
    assert!(report.citations >= 800, "{} citations", report.citations);

    let _ = std::fs::remove_dir_all(&scratch);
}
