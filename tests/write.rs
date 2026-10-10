//! The write verbs on a small book made for the test: what `new`
//! writes, what it refuses, and that what it wrote is what `check`
//! reads. The fixture's letters are D and P so that nothing here reads
//! as a citation of this repository's own book.

use std::path::{Path, PathBuf};

use bothy::config::Kind;
use bothy::schema::{Date, Entry};
use bothy::write::{self, Draft};
use bothy::{Book, Id, Level, check};

const CONFIG: &str = r#"
[area.cross-cutting]
title = "Everywhere"

[area.store]
title = "The store"

[namespace.D]
kind = "decision"
dir = "book/decisions"

[namespace.P]
kind = "park"
dir = "book/parks"
"#;

const D1: &str = r#"# The first ruling, written by hand.
id = "D1"
area = "store"
title = "A listing owns membership"
ruling = '''
A fetch owns contents and nothing more.
'''
decided = 2026-09-02
status = "ruled"   # in force
"#;

const P1: &str = r#"id = "P1"
area = "store"
item = "A second home for annotations"
home = "the store crate"
trigger = "the first user-scoped kind actually written"
parked = 2026-09-03
"#;

/// A fresh copy of the fixture, in a directory named for the test.
fn fixture(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("write-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("book/decisions")).unwrap();
    std::fs::create_dir_all(dir.join("book/parks")).unwrap();
    std::fs::write(dir.join("bothy.toml"), CONFIG).unwrap();
    std::fs::write(dir.join("book/decisions/D001.toml"), D1).unwrap();
    std::fs::write(dir.join("book/parks/P001.toml"), P1).unwrap();
    dir
}

fn open(dir: &Path) -> Book {
    Book::open(&dir.join("bothy.toml")).unwrap()
}

fn day(s: &str) -> Date {
    Date::parse(s).unwrap()
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

fn failures(book: &Book) -> Vec<String> {
    check::run(book)
        .unwrap()
        .diagnostics
        .iter()
        .filter(|d| d.level == Level::Fail)
        .map(ToString::to_string)
        .collect()
}

#[test]
fn new_writes_a_complete_decision_in_one_call() {
    let dir = fixture("complete");
    let draft = Draft {
        area: "store".into(),
        title: Some("A fetch never revives a retired row".into()),
        ruling: Some("Only a listing does.\nSee `fetched_at`.".into()),
        why: Some("Five review rounds found it in five places.".into()),
        ..Default::default()
    };
    let written =
        write::new_entry(&open(&dir), Kind::Decision, &draft, &day("2026-10-10")).unwrap();
    assert_eq!(written.id, id("D2"), "one more than the largest present");
    assert_eq!(written.path, Path::new("book/decisions/D002.toml"));
    assert!(written.missing.is_empty());
    assert!(written.diagnostics.is_empty(), "{:?}", written.diagnostics);

    let text = std::fs::read_to_string(dir.join(&written.path)).unwrap();
    assert_eq!(
        text,
        "id = \"D2\"\n\
         area = \"store\"\n\
         title = \"A fetch never revives a retired row\"\n\
         ruling = '''\nOnly a listing does.\nSee `fetched_at`.\n'''\n\
         why = \"Five review rounds found it in five places.\"\n\
         decided = 2026-10-10\n",
        "a complete entry carries no comments and no empty fields"
    );
    assert!(failures(&open(&dir)).is_empty());
}

#[test]
fn new_leaves_a_blank_for_each_required_field_and_check_refuses_it() {
    let dir = fixture("template");
    let draft = Draft {
        area: "store".into(),
        ..Default::default()
    };
    let written =
        write::new_entry(&open(&dir), Kind::Decision, &draft, &day("2026-10-10")).unwrap();
    assert_eq!(written.missing, ["title", "ruling"]);
    let said: Vec<String> = written
        .diagnostics
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(
        said.iter().any(|d| d.contains("no title")),
        "the verb reports what check says about its own file: {said:?}"
    );

    let text = std::fs::read_to_string(dir.join(&written.path)).unwrap();
    assert!(
        text.contains("# title: "),
        "each blank says what belongs there:\n{text}"
    );
    assert!(text.contains("# ruling: "));
    assert!(
        text.contains("#   why = "),
        "the optional fields are named:\n{text}"
    );
    let said = failures(&open(&dir));
    assert_eq!(
        said.len(),
        1,
        "the gate is red until the title is filled: {said:?}"
    );

    // A title filled in by hand is enough: it may be the whole rule.
    let filled = text.replace("title = \"\"", "title = \"Filled\"");
    std::fs::write(dir.join(&written.path), filled).unwrap();
    assert!(failures(&open(&dir)).is_empty());
}

#[test]
fn a_park_with_a_blank_item_or_home_is_refused() {
    let dir = fixture("blank-park");
    let draft = Draft {
        area: "store".into(),
        ..Default::default()
    };
    let written = write::new_entry(&open(&dir), Kind::Park, &draft, &day("2026-10-10")).unwrap();
    assert_eq!(written.missing, ["item", "home", "trigger"]);
    let said = failures(&open(&dir));
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0].contains("`item` is blank"), "{said:?}");
    assert!(said[1].contains("`home` is blank"), "{said:?}");
}

#[test]
fn new_park_takes_the_park_fields() {
    let dir = fixture("park");
    let draft = Draft {
        area: "store".into(),
        item: Some("Retention of old rows".into()),
        home: Some("the store crate".into()),
        ..Default::default()
    };
    let written = write::new_entry(&open(&dir), Kind::Park, &draft, &day("2026-10-10")).unwrap();
    assert_eq!(written.id, id("P2"));
    assert_eq!(written.missing, ["trigger"]);
    let book = open(&dir);
    assert!(
        failures(&book).is_empty(),
        "a park with no trigger is reported, not refused"
    );
    let Entry::Park(p) = &book.get(id("P2")).unwrap().entry else {
        panic!("P2 is a park");
    };
    assert_eq!(p.item, "Retention of old rows");
    assert_eq!(p.parked, day("2026-10-10"));
}

#[test]
fn new_refuses_an_undeclared_area_and_the_other_kinds_fields() {
    let dir = fixture("refusals");
    let book = open(&dir);
    let today = day("2026-10-10");

    let err = write::new_entry(
        &book,
        Kind::Decision,
        &Draft {
            area: "stor".into(),
            ..Default::default()
        },
        &today,
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("\"stor\" is not declared") && err.contains("store"),
        "{err}"
    );

    let err = write::new_entry(
        &book,
        Kind::Park,
        &Draft {
            area: "store".into(),
            title: Some("A title".into()),
            ..Default::default()
        },
        &today,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("--title does not apply to a park"), "{err}");

    let err = write::new_entry(
        &book,
        Kind::Invariant,
        &Draft {
            area: "store".into(),
            ..Default::default()
        },
        &today,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("no invariant namespace"), "{err}");

    assert_eq!(open(&dir).entries.len(), 2, "a refusal writes nothing");
}

#[test]
fn new_counts_a_file_that_does_not_parse_and_never_replaces_one() {
    let dir = fixture("high-water");
    // D7 exists on disk but is not valid TOML, so it does not load.
    std::fs::write(dir.join("book/decisions/D007.toml"), "id = \n").unwrap();
    let draft = Draft {
        area: "store".into(),
        title: Some("Next".into()),
        ruling: Some("After the broken one.".into()),
        ..Default::default()
    };
    let written =
        write::new_entry(&open(&dir), Kind::Decision, &draft, &day("2026-10-10")).unwrap();
    assert_eq!(
        written.id,
        id("D8"),
        "the broken file still holds its number"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("book/decisions/D007.toml")).unwrap(),
        "id = \n"
    );
}
