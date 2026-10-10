//! Bothy is its own first consumer (B16): the book under `bothy/` in
//! this repository must load and pass `check` with no failure.

use std::path::Path;

use bothy::{Book, Level, check};

fn own_book() -> Book {
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("bothy.toml");
    Book::open(&config).expect("the repository's own bothy.toml opens")
}

#[test]
fn own_book_passes_check() {
    let book = own_book();
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
        report.entries >= 30,
        "the hand-written book has at least twenty rulings and ten parks"
    );
}

#[test]
fn own_book_has_both_kinds_and_a_next_id() {
    let book = own_book();
    assert!(book.get(bothy::Id::parse("B1").unwrap()).is_some());
    assert!(book.get(bothy::Id::parse("K1").unwrap()).is_some());
    let next = book.next_id('B');
    assert!(
        next.number > 20,
        "the next B id follows the ones present (B3)"
    );
    assert_eq!(book.next_id('Z').to_string(), "Z1"); // a letter no namespace uses
}

#[test]
fn a_decision_names_its_park() {
    // B8 names K7 (the SQLite park) and the check must resolve it.
    let book = own_book();
    let b8 = book
        .get(bothy::Id::parse("B8").unwrap())
        .expect("B8 exists");
    let names = bothy::cite::cross_references(&book, &b8.entry.prose());
    assert!(names.contains(&bothy::Id::parse("K7").unwrap()));
}
