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

#[test]
fn amend_adds_one_line_and_changes_no_other() {
    let dir = fixture("amend");
    let amended = write::amend(&open(&dir), id("D1"), None, &day("2026-10-10")).unwrap();
    assert!(amended.date_added && !amended.verdict_set);
    let text = std::fs::read_to_string(dir.join("book/decisions/D001.toml")).unwrap();
    assert_eq!(
        text,
        D1.replace(
            "decided = 2026-09-02\n",
            "decided = 2026-09-02\namended = [2026-10-10]\n"
        ),
        "the comments, the ''' block and the key order are the author's"
    );

    // The same day again adds nothing; a later day joins the list.
    let again = write::amend(&open(&dir), id("D1"), None, &day("2026-10-10")).unwrap();
    assert!(!again.date_added);
    assert_eq!(
        std::fs::read_to_string(dir.join("book/decisions/D001.toml")).unwrap(),
        text
    );
    write::amend(&open(&dir), id("D1"), None, &day("2026-10-12")).unwrap();
    let text = std::fs::read_to_string(dir.join("book/decisions/D001.toml")).unwrap();
    assert!(
        text.contains("amended = [2026-10-10, 2026-10-12]\n"),
        "{text}"
    );
    assert!(failures(&open(&dir)).is_empty());
}

#[test]
fn amend_sets_a_verdict_once_and_never_replaces_it() {
    let dir = fixture("verdict");
    let words = "I agree to 1 entirely";
    let amended = write::amend(&open(&dir), id("D1"), Some(words), &day("2026-10-10")).unwrap();
    assert!(amended.verdict_set);
    let text = std::fs::read_to_string(dir.join("book/decisions/D001.toml")).unwrap();
    let verdict = text.find("verdict = \"I agree to 1 entirely\"\n").unwrap();
    assert!(
        text.find("ruling = ").unwrap() < verdict && verdict < text.find("decided = ").unwrap(),
        "the new field sits where a written entry has it:\n{text}"
    );

    let err = write::amend(
        &open(&dir),
        id("D1"),
        Some("something else"),
        &day("2026-10-11"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("already has a verdict"), "{err}");
    assert_eq!(
        std::fs::read_to_string(dir.join("book/decisions/D001.toml")).unwrap(),
        text,
        "a refusal writes nothing, not even the date"
    );
}

#[test]
fn amend_refuses_a_park_a_retired_ruling_and_an_unknown_id() {
    let dir = fixture("amend-refusals");
    std::fs::write(
        dir.join("book/decisions/D002.toml"),
        "id = \"D2\"\narea = \"store\"\ntitle = \"Old\"\nruling = \"\"\nstatus = \"superseded\"\nsuperseded_by = \"D1\"\n",
    )
    .unwrap();
    let book = open(&dir);
    let today = day("2026-10-10");
    let refusal = |s: &str| {
        write::amend(&book, id(s), None, &today)
            .unwrap_err()
            .to_string()
    };
    assert!(refusal("P1").contains("is a park"), "{}", refusal("P1"));
    assert!(
        refusal("D2").contains("superseded by D1; amend D1 instead"),
        "{}",
        refusal("D2")
    );
    assert!(refusal("D9").contains("no entry D9"), "{}", refusal("D9"));
}

#[test]
fn fire_marks_the_park_and_the_ruling_names_it() {
    let dir = fixture("fire");
    let fired = write::fire(&open(&dir), id("P1"), id("D1"), &day("2026-10-10")).unwrap();
    assert_eq!(fired.path, Path::new("book/parks/P001.toml"));
    assert_eq!(
        fired.evidence_added_to.as_deref(),
        Some(Path::new("book/decisions/D001.toml"))
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("book/parks/P001.toml")).unwrap(),
        format!("{P1}fired = 2026-10-10\nfired_by = \"D1\"\n"),
        "the park gains two lines"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("book/decisions/D001.toml")).unwrap(),
        D1.replace("'''\ndecided", "'''\nevidence = [\"P1\"]\ndecided"),
        "the ruling gains one"
    );

    let book = open(&dir);
    assert!(failures(&book).is_empty());
    assert!(
        book.get(id("P1")).unwrap().entry.is_retired(),
        "a fired park keeps its file and leaves the lists and packs"
    );

    let err = write::fire(&book, id("P1"), id("D1"), &day("2026-10-11"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("already fired on 2026-10-10 by D1"), "{err}");
}

#[test]
fn fire_leaves_evidence_that_already_names_the_park() {
    let dir = fixture("fire-named");
    let with_evidence = D1.replace(
        "decided = ",
        "evidence = [\"takes up P1, measured twice\"]\ndecided = ",
    );
    std::fs::write(dir.join("book/decisions/D001.toml"), &with_evidence).unwrap();
    let fired = write::fire(&open(&dir), id("P1"), id("D1"), &day("2026-10-10")).unwrap();
    assert!(fired.evidence_added_to.is_none());
    assert_eq!(
        std::fs::read_to_string(dir.join("book/decisions/D001.toml")).unwrap(),
        with_evidence
    );
}

#[test]
fn fire_refuses_what_is_not_a_park_or_not_a_ruling_and_writes_nothing() {
    let dir = fixture("fire-refusals");
    let book = open(&dir);
    let today = day("2026-10-10");
    let refusal = |park: &str, by: &str| {
        write::fire(&book, id(park), id(by), &today)
            .unwrap_err()
            .to_string()
    };
    assert!(refusal("D1", "D1").contains("not a park"));
    assert!(refusal("P1", "P1").contains("P1 is a park"));
    let unknown = refusal("P1", "D9");
    assert!(
        unknown.contains("no entry D9") && unknown.contains("bothy new decision"),
        "{unknown}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("book/parks/P001.toml")).unwrap(),
        P1
    );
}
