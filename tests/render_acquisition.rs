//! The brief's done-criterion 2: the imported acquisition book renders
//! `CONTEXT.md` and the seven `decisions/*.md` so that what differs from
//! the files on disk is only the normalization the brief predicts:
//! marker order, a dating sentence moved to the end of its entry, the
//! index table in file order and naming every decision (the C86 gap).
//! Every line of every original file must appear in the render once
//! those are allowed for, and nothing else may.
//!
//! Skipped with a note when the owner's clone is absent; the clone is
//! read, never written (B4).

use std::path::PathBuf;

use bothy::{Book, import, render};

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

/// The same words in the same multiset: a line reordered (markers in
/// canonical order, the dating sentence moved last) passes; a line that
/// lost or gained a word does not. This does not share the renderer's
/// logic, so a bug in both cannot hide from it.
fn same_tokens(a: &str, b: &str) -> bool {
    let mut x: Vec<&str> = a.split_whitespace().collect();
    let mut y: Vec<&str> = b.split_whitespace().collect();
    x.sort_unstable();
    y.sort_unstable();
    x == y
}

#[test]
fn acquisition_renders_back_to_its_files_up_to_normalization() {
    let Some(root) = playground() else {
        eprintln!("skipped: no acquisition playground at the default path or $BOTHY_ACQUISITION");
        return;
    };
    let imported = import::parse_playground(&root).expect("parses");
    let out = std::env::temp_dir().join(format!("bothy-render-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let summary = import::write(&imported, &out, &root).expect("writes");
    let book = Book::open(&summary.config).expect("opens");
    let mut notes = Vec::new();
    let rendered = render::all(&book, &mut notes).expect("renders");
    assert_eq!(rendered.len(), 8, "CONTEXT.md and seven area files");
    assert!(
        notes.is_empty(),
        "the generated layout places everything:\n{}",
        notes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );

    let mut report = String::new();
    let mut identical = 0;
    let mut reordered = 0;
    for r in &rendered {
        let original = std::fs::read_to_string(root.join(&r.path)).expect("original exists");
        if original == r.text {
            identical += 1;
            continue;
        }
        let want: Vec<&str> = original.lines().collect();
        let got: Vec<&str> = r.text.lines().collect();
        if want.len() != got.len() {
            report.push_str(&format!(
                "\n== {}: {} lines on disk, {} rendered\n",
                r.path.display(),
                want.len(),
                got.len()
            ));
            continue;
        }
        for (w, g) in want.iter().zip(&got) {
            if w == g {
                continue;
            }
            let is_entry = w.starts_with("- **C") && g.starts_with("- **C");
            let is_index_row = w.starts_with("| `decisions/") && g.starts_with("| `decisions/");
            if is_entry && same_tokens(w, g) {
                reordered += 1;
            } else if is_index_row {
                // File order and every decision named (the C86 gap): the
                // rendered row must contain every id the original names.
                let ids = |l: &str| {
                    l.rsplit('|')
                        .nth(1)
                        .unwrap_or("")
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                };
                let (wi, gi) = (ids(w), ids(g));
                if !wi.iter().all(|i| gi.contains(i)) {
                    report.push_str(&format!(
                        "\n== {}: an index row lost an id\n  disk:     {w}\n  rendered: {g}\n",
                        r.path.display()
                    ));
                }
            } else {
                report.push_str(&format!(
                    "\n== {}: a line changed beyond reordering\n  disk:     {}\n  rendered: {}\n",
                    r.path.display(),
                    &w[..w.len().min(200)],
                    &g[..g.len().min(200)]
                ));
            }
        }
    }
    eprintln!(
        "{identical} of {} files identical; {reordered} entry lines reordered within the allowed normalization",
        rendered.len()
    );
    assert!(
        identical >= 4,
        "at least the four files with canonical marker order render byte for byte; got {identical}"
    );
    let _ = std::fs::remove_dir_all(&out);
    assert!(
        report.is_empty(),
        "the render differs beyond the allowed normalization:{report}"
    );
}
