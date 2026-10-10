//! `pack` on bothy's own book (B15): the fixed order, the omission of
//! other areas, the budget that trims from the tail and says so.

use std::path::Path;

use bothy::pack::{self, Request};
use bothy::{Book, Id};

fn own_book() -> Book {
    Book::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("bothy.toml")).unwrap()
}

fn pos(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("pack lacks {needle:?}:\n{text}"))
}

#[test]
fn area_pack_has_the_fixed_order_and_only_its_area() {
    let book = own_book();
    let pack = pack::build(
        &book,
        &Request {
            areas: vec!["check".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let t = &pack.text;
    // What the project is comes before anything that binds (B27).
    let orientation = pos(t, "bothy is a command-line tool");
    assert!(
        orientation < pos(t, "This pack is the project's book"),
        "the orientation opens the pack"
    );
    let cross = pos(t, "## Cross-cutting rulings");
    let rulings = pos(t, "## Rulings for `check`");
    let parks = pos(t, "## Parked in `check`");
    let map = pos(t, "## Where each entry is held");
    let footer = pos(t, "\n---\npack: ");
    assert!(
        cross < rulings && rulings < parks && parks < map && map < footer,
        "order is fixed"
    );
    assert!(
        t.contains("**B1 — "),
        "cross-cutting rulings are in every pack"
    );
    assert!(t.contains("**B20 — "), "the area's ruling is in");
    assert!(t.contains("**K3**"), "the area's park is in");
    let across = pos(t, "## Parked across every area");
    assert!(parks < across && across < map, "order is fixed");
    assert!(
        t.contains("**K7**"),
        "a cross-cutting park is in every pack, as a cross-cutting ruling is"
    );
    assert!(!t.contains("**B5 — "), "another area's ruling is out");
    assert!(
        t.contains(pack::ESTIMATOR),
        "the footer names the estimator"
    );
    assert!(
        t.contains("- B20: src/check.rs:"),
        "the map says where B20 is held"
    );
    assert_eq!(pack.bytes, pack.text.len());
    assert!(pack.trimmed.is_empty());
}

#[test]
fn a_document_to_read_also_is_named_with_its_size_and_why() {
    let book = own_book();
    let pack = pack::build(
        &book,
        &Request {
            areas: vec!["pack".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let t = &pack.text;
    let also = pos(t, "## Read also");
    assert!(pos(t, "## Parked across every area") < also);
    assert!(also < pos(t, "## Where each entry is held"));
    let line = t[also..]
        .lines()
        .find(|l| l.starts_with("- `AGENTS.md` ("))
        .expect("the document is named");
    assert!(line.contains(" bytes): how to work in this repository"));
    assert!(
        !t.contains("# bothy — where to start"),
        "the document is named, never inlined (K11)"
    );
}

#[test]
fn path_resolves_to_its_area() {
    let book = own_book();
    let pack = pack::build(
        &book,
        &Request {
            paths: vec!["src/cite.rs".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(pack.areas, vec!["check".to_string()]);
    let err = pack::build(
        &book,
        &Request {
            paths: vec!["Cargo.lock".into()],
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("no area's paths cover Cargo.lock"),
        "{err}"
    );
}

#[test]
fn budget_trims_from_the_tail_and_says_so() {
    let book = own_book();
    let full = pack::build(
        &book,
        &Request {
            areas: vec!["schema".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let small = pack::build(
        &book,
        &Request {
            areas: vec!["schema".into()],
            budget: Some(full.tokens / 2),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        small.tokens <= full.tokens / 2,
        "{} > {}",
        small.tokens,
        full.tokens / 2
    );
    assert!(!small.trimmed.is_empty());
    assert_eq!(
        small.trimmed[0], "the citation map",
        "the map goes first, being last"
    );
    assert!(
        small.trimmed.iter().all(|l| l.len() < 40),
        "labels stay short: {:?}",
        small.trimmed
    );
    assert!(
        small
            .text
            .contains("trimmed to fit the budget, from the tail: the citation map")
    );
    assert!(
        small.text.contains("**B1 — "),
        "what binds most survives, being first"
    );
}

#[test]
fn for_one_entry_gives_its_neighbours() {
    let book = own_book();
    let pack = pack::build(
        &book,
        &Request {
            for_id: Some(Id::parse("B8").unwrap()),
            ..Default::default()
        },
    )
    .unwrap();
    let t = &pack.text;
    pos(t, "# B8 and what touches it");
    pos(t, "## What B8 names");
    assert!(t.contains("**K7**"), "B8 names the SQLite park");
    assert!(
        !t.contains("## What names B8"),
        "K7 names B8 and was already listed; it is not listed twice"
    );
    assert_eq!(t.matches("**K7**").count(), 1);
    assert!(t.contains("- B8: src/"), "B8 is held in code");
}

#[test]
fn asking_for_nothing_is_refused_plainly() {
    let book = own_book();
    let err = pack::build(&book, &Request::default()).unwrap_err();
    assert!(
        err.to_string()
            .contains("--area <name>, --path <file>, or --for <id>")
    );
}
