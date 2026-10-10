//! The brief's done-criterion 1: `import` over acquisition's playground
//! yields its decisions, parks, principles and invariants with no
//! exception, and `check` passes on the result with the findings
//! `docs-check.sh` reports today (C16 uncited; three parks without a
//! trigger). The playground is read, never written (B4).
//!
//! The clone is the owner's, at the path below or `$BOTHY_ACQUISITION`,
//! and its tip moves; so the counts are floors, not exact numbers, and
//! the test is skipped with a note when the clone is absent.

use std::path::PathBuf;

use bothy::{Book, Level, check, import, pack};

fn playground() -> Option<PathBuf> {
    let path = std::env::var("BOTHY_ACQUISITION")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home)
                .join("Development/GitHub/gerwaric/acquisition/spikes/rust-playground")
        });
    path.join("CONTEXT.md").is_file().then_some(path)
}

#[test]
fn acquisition_imports_whole_and_passes_check() {
    let Some(root) = playground() else {
        eprintln!("skipped: no acquisition playground at the default path or $BOTHY_ACQUISITION");
        return;
    };
    let imported = import::parse_playground(&root).expect("parses");
    assert!(
        imported.exceptions.is_empty(),
        "exceptions:\n{}",
        imported.exceptions.join("\n")
    );
    assert!(
        imported.decisions.len() >= 108,
        "{} decisions",
        imported.decisions.len()
    );
    // A park is deleted there when its trigger fires, so the park count
    // falls as well as rises (53 at `9e934746`, 52 at `075e4d2c`) and
    // is no floor; the count the summary reports is checked below.
    assert!(!imported.parks.is_empty());
    assert_eq!(imported.principles.len(), 6);
    // The orientation leaves the template for bothy.toml, so a pack and
    // the rendered CONTEXT.md print one source (B27).
    let orientation = imported.orientation.as_deref().expect("an orientation");
    assert!(orientation.starts_with("Acquisition is "));
    assert!(imported.context_template.contains("{{ orientation }}"));
    assert!(!imported.context_template.contains(orientation));
    assert_eq!(imported.invariants.len(), 5);
    assert_eq!(
        imported.areas.len(),
        8,
        "seven area files and cross-cutting"
    );
    assert!(
        imported
            .decisions
            .iter()
            .all(|d| d.history.is_some() || d.decided.is_some() || d.ruling.contains("20"))
    );

    let out = std::env::temp_dir().join(format!("bothy-import-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let summary = import::write(&imported, &out, &root).expect("writes");
    assert_eq!(
        summary.written,
        imported.decisions.len() + imported.parks.len() + 11
    );

    let book = Book::open(&summary.config).expect("the written book opens");
    let report = check::run(&book).expect("check runs");
    // Until acquisition's normalization commit lands, the only failures
    // are projections that differ from the hand-kept files by marker order
    // and the index table's order; nothing else may fail.
    let failures: Vec<String> = report
        .diagnostics
        .iter()
        .filter(|d| d.level == Level::Fail)
        .map(ToString::to_string)
        .collect();
    let other: Vec<&str> = failures
        .iter()
        .map(String::as_str)
        .filter(|f| !f.contains("differs from what the entries render"))
        .collect();
    assert!(
        other.is_empty(),
        "check failed beyond stale projections:\n{}",
        other.join("\n")
    );
    assert!(
        failures.len() <= 8,
        "at most the eight projections may be stale: {failures:?}"
    );
    assert!(
        report.citations > 3000,
        "{} citations resolved",
        report.citations
    );
    let uncited: Vec<String> = report.uncited.iter().map(ToString::to_string).collect();
    assert!(
        uncited.contains(&"C16".to_string()),
        "docs-check reports C16 uncited today; so must this: {uncited:?}"
    );

    // The store area's intro names the files it governs, so a session
    // can ask by the file it is about to change (B19), and the pack it
    // gets opens with what the project is (B27).
    let request = pack::Request {
        paths: vec!["crates/acquisition-store/src/lib.rs".into()],
        ..Default::default()
    };
    let packed = pack::build(&book, &request).expect("the path resolves to an area");
    assert_eq!(packed.areas, ["store"]);
    let orientation = pack_pos(&packed.text, "Acquisition is ");
    assert!(orientation < pack_pos(&packed.text, "## Invariants"));
    assert!(packed.text.contains("## Rulings for `store`"));
    assert!(packed.text.contains("## Parked across every area"));
    let _ = std::fs::remove_dir_all(&out);
}

fn pack_pos(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("the pack has no {needle:?}"))
}
