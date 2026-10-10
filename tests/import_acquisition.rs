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

use bothy::{Book, Level, check, import};

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
    assert!(imported.parks.len() >= 53, "{} parks", imported.parks.len());
    assert_eq!(imported.principles.len(), 6);
    assert_eq!(imported.invariants.len(), 5);
    assert_eq!(imported.areas.len(), 7);
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
    let failures: Vec<String> = report
        .diagnostics
        .iter()
        .filter(|d| d.level == Level::Fail)
        .map(ToString::to_string)
        .collect();
    assert!(
        failures.is_empty(),
        "check failed:\n{}",
        failures.join("\n")
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
    let _ = std::fs::remove_dir_all(&out);
}
