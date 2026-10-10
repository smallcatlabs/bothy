//! The write verbs. `new` adds an entry (B22), `amend` dates a change
//! to a ruling (B23) and `fire` stops a park whose trigger fired (B24).
//! Each one reads the book,
//! changes one or two entry files under the directories `bothy.toml`
//! names and nothing else (B4), and says what it wrote. None of them
//! prompts or opens an editor: the caller is as often an agent in a
//! shell as a person at a terminal.
//!
//! Every function here takes the date to record, so a test or a server
//! can supply it; the command line passes [`today`].

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use toml_edit::{Array, DocumentMut, Item, value};

use crate::book::{Book, Loaded, parse_entry};
use crate::check::{self, Diagnostic};
use crate::config::Kind;
use crate::emit;
use crate::id::Id;
use crate::render::kind_name;
use crate::schema::{Date, Entry};

/// The calendar day on this machine's clock, in its own time zone
/// (B22). An entry written at six in the evening in Denver is dated
/// that day, not the next one, which is what UTC would say.
pub fn today() -> Date {
    let day = jiff::Zoned::now().date().to_string();
    Date::parse(&day).expect("a civil date prints as YYYY-MM-DD")
}

/// What `new` was told about the entry. A field left `None` (or blank)
/// is written empty for its author to fill, if the kind requires it,
/// and left out if it does not.
#[derive(Debug, Default, Clone)]
pub struct Draft {
    pub area: String,
    /// The namespace's letter, needed only when the book has two
    /// namespaces of the kind.
    pub letter: Option<char>,
    /// Decision, invariant, principle.
    pub title: Option<String>,
    pub ruling: Option<String>,
    /// Park.
    pub item: Option<String>,
    pub home: Option<String>,
    pub trigger: Option<String>,
    /// Either shape.
    pub why: Option<String>,
}

/// What `new` did.
#[derive(Serialize, Debug)]
pub struct Written {
    pub id: Id,
    pub kind: &'static str,
    pub area: String,
    /// Relative to the root.
    pub path: PathBuf,
    /// Required fields written empty, for the author to fill.
    pub missing: Vec<&'static str>,
    /// What `check` says about the new file, read back from disk.
    pub diagnostics: Vec<Diagnostic>,
}

/// Adds one entry of `kind`: the next free id (B3), a new file named by
/// it (B5), the fields the draft gives, and a commented blank for each
/// required field it does not (B22). Refuses an undeclared area (B19)
/// and never replaces a file that exists.
pub fn new_entry(book: &Book, kind: Kind, draft: &Draft, today: &Date) -> Result<Written> {
    let word = kind_name(kind);
    let is_park = kind == Kind::Park;

    if !book.config.area.contains_key(&draft.area) {
        let known: Vec<&str> = book.config.area.keys().map(String::as_str).collect();
        bail!(
            "area \"{}\" is not declared in bothy.toml; use one of: {}; or declare it under [area.{}] first",
            draft.area,
            known.join(", "),
            draft.area
        );
    }

    let other_shape: Vec<&str> = if is_park {
        [("--title", &draft.title), ("--ruling", &draft.ruling)]
            .iter()
            .filter(|(_, v)| v.is_some())
            .map(|(flag, _)| *flag)
            .collect()
    } else {
        [
            ("--item", &draft.item),
            ("--home", &draft.home),
            ("--trigger", &draft.trigger),
        ]
        .iter()
        .filter(|(_, v)| v.is_some())
        .map(|(flag, _)| *flag)
        .collect()
    };
    if !other_shape.is_empty() {
        bail!(
            "{} {} not apply to a {word}; a {word} takes {}",
            other_shape.join(" and "),
            if other_shape.len() == 1 { "does" } else { "do" },
            if is_park {
                "--item, --home, --trigger and --why"
            } else {
                "--title, --ruling and --why"
            }
        );
    }

    let (letter, dir) = namespace_for(book, kind, draft.letter)?;
    let abs_dir = book.root.join(&dir);
    let id = next_free(book, letter, &abs_dir);
    let rel = dir.join(format!("{}.toml", id.file_stem()));

    let (text, missing) = if is_park {
        park_text(id, draft, today)
    } else {
        decision_text(id, kind, draft, today)
    };
    // The tool never writes a file it could not read back.
    parse_entry(kind, &text).map_err(|e| {
        anyhow!(
            "the entry would not be valid TOML for a {word}: {}",
            e.message()
        )
    })?;

    std::fs::create_dir_all(&abs_dir)
        .with_context(|| format!("cannot create {}", dir.display()))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(book.root.join(&rel))
        .with_context(|| {
            format!(
                "cannot create {}; if it already exists, another session took {id} first: run the command again for the next id",
                rel.display()
            )
        })?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("cannot write {}", rel.display()))?;

    let after = Book::open(&book.config_path)?;
    let diagnostics = check::run(&after)?
        .diagnostics
        .into_iter()
        .filter(|d| d.path.as_deref() == Some(rel.as_path()))
        .collect();

    Ok(Written {
        id,
        kind: word,
        area: draft.area.clone(),
        path: rel,
        missing,
        diagnostics,
    })
}

/// The letter and directory of the namespace a new entry of `kind`
/// goes in. One namespace of the kind is the usual case and needs no
/// letter; with two, the caller must say which.
fn namespace_for(book: &Book, kind: Kind, letter: Option<char>) -> Result<(char, PathBuf)> {
    let word = kind_name(kind);
    let owned: Vec<(char, &PathBuf)> = book
        .config
        .namespace
        .iter()
        .filter(|(_, ns)| ns.kind == kind)
        .filter_map(|(key, ns)| ns.dir.as_ref().map(|dir| (ns.letter_or(key), dir)))
        .collect();
    let letters = || -> String {
        let all: Vec<String> = owned.iter().map(|(l, _)| l.to_string()).collect();
        all.join(", ")
    };
    match (letter, owned.as_slice()) {
        (_, []) => bail!(
            "this book has no {word} namespace; declare one in bothy.toml, as a [namespace.<LETTER>] block with kind = \"{word}\" and dir = \"<directory>\""
        ),
        (None, [(l, dir)]) => Ok((*l, (*dir).clone())),
        (None, _) => bail!(
            "this book has {} {word} namespaces ({}); say which with --letter",
            owned.len(),
            letters()
        ),
        (Some(want), _) => owned
            .iter()
            .find(|(l, _)| *l == want)
            .map(|(l, dir)| (*l, (*dir).clone()))
            .ok_or_else(|| {
                anyhow!(
                    "no {word} namespace has the letter {want}; this book's {word} letters: {}",
                    letters()
                )
            }),
    }
}

/// One more than the largest id present (B3), counting both the entries
/// that loaded and the files in the directory, so a file that failed to
/// parse still holds its number.
fn next_free(book: &Book, letter: char, abs_dir: &Path) -> Id {
    let on_disk = std::fs::read_dir(abs_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .filter_map(|p| {
            let stem = p.file_stem()?.to_str()?;
            let number: u32 = stem.strip_prefix(letter)?.parse().ok()?;
            Some(number)
        })
        .max()
        .unwrap_or(0);
    let loaded = book.next_id(letter);
    Id {
        letter,
        number: loaded.number.max(on_disk + 1),
    }
}

fn given(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.trim().is_empty())
}

/// Several lines of prose end with a newline, so the closing `'''` sits
/// on its own line the way a hand-written entry has it.
fn block(text: &str) -> String {
    if text.contains('\n') && !text.ends_with('\n') {
        format!("{text}\n")
    } else {
        text.to_string()
    }
}

/// The comment that ends a file with a blank left in it. A complete
/// entry carries no comments (B22).
fn optional_note(out: &mut String, optional: &[&str]) {
    out.push_str("# Optional fields. Add each as its own line when it applies:\n");
    for line in optional {
        let _ = writeln!(out, "#   {line}");
    }
    out.push_str("# These comment lines may be deleted when the entry is complete.\n");
}

fn decision_text(id: Id, kind: Kind, draft: &Draft, today: &Date) -> (String, Vec<&'static str>) {
    let mut out = String::new();
    let mut missing = Vec::new();
    let _ = writeln!(out, "id = \"{id}\"");
    let _ = writeln!(out, "area = {}", emit::string(&draft.area));
    match given(&draft.title) {
        Some(title) => {
            let _ = writeln!(out, "title = {}", emit::string(title));
        }
        // A principle may go without a title; nothing else may.
        None if kind == Kind::Principle => {}
        None => {
            missing.push("title");
            out.push_str("# title: the rule in one line. It is the bold lead a reader skims.\n");
            out.push_str("title = \"\"\n");
        }
    }
    match given(&draft.ruling) {
        Some(ruling) => {
            let _ = writeln!(out, "ruling = {}", emit::string(&block(ruling)));
        }
        None => {
            missing.push("ruling");
            out.push_str(
                "# ruling: what was decided, in full sentences, between the two ''' lines.\n",
            );
            out.push_str("# Leave it blank only when the title is the whole rule.\n");
            out.push_str("ruling = '''\n'''\n");
        }
    }
    if let Some(why) = given(&draft.why) {
        let _ = writeln!(out, "why = {}", emit::string(why));
    }
    let _ = writeln!(out, "decided = {today}");
    if !missing.is_empty() {
        let mut optional = Vec::new();
        if given(&draft.why).is_none() {
            optional.push("why = \"the reason, in a sentence\"");
        }
        optional.extend([
            "details = [\"where the mechanism lives: a file or a module\"]",
            "pinned = [\"a test that holds the ruling\"]",
            "evidence = [\"an id or a measurement that supports it\"]",
            "verdict = \"the owner's words, quoted exactly\"",
        ]);
        optional_note(&mut out, &optional);
    }
    (out, missing)
}

fn park_text(id: Id, draft: &Draft, today: &Date) -> (String, Vec<&'static str>) {
    let mut out = String::new();
    let mut missing = Vec::new();
    let _ = writeln!(out, "id = \"{id}\"");
    let _ = writeln!(out, "area = {}", emit::string(&draft.area));
    let wanted = [
        (
            "item",
            &draft.item,
            "# item: what is deferred, in one line.\n",
        ),
        (
            "home",
            &draft.home,
            "# home: where it lands when it is built: a crate, a file, a verb.\n",
        ),
        (
            "trigger",
            &draft.trigger,
            "# trigger: what reopens it. \"never\" is legal and means what it says.\n",
        ),
    ];
    for (key, value, comment) in wanted {
        match given(value) {
            Some(v) => {
                let _ = writeln!(out, "{key} = {}", emit::string(v));
            }
            None => {
                missing.push(key);
                out.push_str(comment);
                let _ = writeln!(out, "{key} = \"\"");
            }
        }
    }
    if let Some(why) = given(&draft.why) {
        let _ = writeln!(out, "why = {}", emit::string(why));
    }
    let _ = writeln!(out, "parked = {today}");
    if !missing.is_empty() {
        let mut optional = Vec::new();
        if given(&draft.why).is_none() {
            optional.push("why = \"why it waits, in one clause\"");
        }
        optional.push("measured = \"numbers the park carries, kept out of the lines above\"");
        optional_note(&mut out, &optional);
    }
    (out, missing)
}

/// The entry `id` names, when this book owns its file.
fn owned(book: &Book, id: Id) -> Result<&Loaded> {
    book.get(id).ok_or_else(|| {
        if book.knows(id) {
            anyhow!(
                "{id} is defined in an external namespace; bothy does not own its file and cannot change it"
            )
        } else {
            anyhow!(
                "no entry {id} in this book; `bothy list decisions` and `bothy list parks` show what exists"
            )
        }
    })
}

fn document(loaded: &Loaded) -> Result<DocumentMut> {
    loaded
        .source
        .parse()
        .with_context(|| format!("cannot read {} as TOML", loaded.path.display()))
}

fn toml_date(date: &Date) -> toml_edit::Datetime {
    date.to_string()
        .parse()
        .expect("a plain date is a TOML datetime")
}

/// Adds `key` to an entry file on the line where `emit` would have
/// written it, and leaves every other line as its author had it (B14).
/// `order` is the kind's field order; a key the order does not know
/// goes last.
fn insert_in_order(doc: &mut DocumentMut, key: &str, item: Item, order: &[&str]) {
    let rank = |k: &str| order.iter().position(|o| *o == k);
    let mut keys: Vec<String> = doc.iter().map(|(k, _)| k.to_string()).collect();
    let at = rank(key)
        .and_then(|mine| keys.iter().position(|k| rank(k).is_some_and(|r| r > mine)))
        .unwrap_or(keys.len());
    keys.insert(at, key.to_string());
    doc.insert(key, item);
    let place = |k: &str| keys.iter().position(|x| x == k);
    doc.as_table_mut()
        .sort_values_by(|a, _, b, _| place(a.get()).cmp(&place(b.get())));
}

/// Writes an edited entry back, having first read the new text the way
/// the book will: an edit that would leave the file unreadable is
/// refused and nothing is written.
fn save(book: &Book, loaded: &Loaded, doc: &DocumentMut) -> Result<()> {
    let text = doc.to_string();
    parse_entry(loaded.kind, &text).map_err(|e| {
        anyhow!(
            "the change would leave {} unreadable ({}); nothing was written",
            loaded.path.display(),
            e.message()
        )
    })?;
    std::fs::write(book.root.join(&loaded.path), text)
        .with_context(|| format!("cannot write {}", loaded.path.display()))
}

/// What `amend` did.
#[derive(Serialize, Debug)]
pub struct Amended {
    pub id: Id,
    /// Relative to the root.
    pub path: PathBuf,
    pub date: Date,
    /// False when the entry was already amended on this date.
    pub date_added: bool,
    pub verdict_set: bool,
}

/// Records that a ruling changed today (B23): adds the date to
/// `amended`, once per day, and the owner's words as `verdict` when
/// given. The author changes the ruling's text; this dates the change.
/// Refuses a park, a ruling no longer in force, and a verdict where
/// one already stands.
pub fn amend(book: &Book, id: Id, verdict: Option<&str>, today: &Date) -> Result<Amended> {
    let loaded = owned(book, id)?;
    let Entry::Decision(d) = &loaded.entry else {
        bail!(
            "{id} is a park, and a park is not amended: edit its file, or stop it with `bothy park fire {id} --by <the ruling that fired it>`"
        );
    };
    if d.status != "ruled" {
        let by = match d.superseded_by {
            Some(by) => format!(" by {by}; amend {by} instead"),
            None => String::new(),
        };
        bail!(
            "{id} is {}{by}; only a ruling in force is amended",
            d.status
        );
    }
    if let (Some(_), Some(old)) = (verdict, &d.verdict) {
        bail!(
            "{id} already has a verdict, \"{}\", and amend does not replace the owner's words; edit {} to keep both, then run amend without --verdict",
            old.trim(),
            loaded.path.display()
        );
    }

    let mut doc = document(loaded)?;
    let date_added = !d.amended.contains(today);
    if date_added {
        match doc.get_mut("amended").and_then(Item::as_array_mut) {
            Some(dates) => dates.push(toml_date(today)),
            None => {
                let mut dates = Array::new();
                dates.push(toml_date(today));
                insert_in_order(&mut doc, "amended", value(dates), emit::DECISION_KEYS);
            }
        }
    }
    if let Some(words) = verdict {
        insert_in_order(&mut doc, "verdict", value(words), emit::DECISION_KEYS);
    }
    if date_added || verdict.is_some() {
        save(book, loaded, &doc)?;
    }

    Ok(Amended {
        id,
        path: loaded.path.clone(),
        date: today.clone(),
        date_added,
        verdict_set: verdict.is_some(),
    })
}

/// What `fire` did.
#[derive(Serialize, Debug)]
pub struct Fired {
    pub park: Id,
    pub by: Id,
    pub date: Date,
    /// The park's file, relative to the root.
    pub path: PathBuf,
    /// The firing ruling's file, when this book owns it and its
    /// `evidence` gained the park's id.
    pub evidence_added_to: Option<PathBuf>,
}

/// Whether `text` names `id` as a whole word: `C5` is in "takes up C5"
/// and not in "C54".
fn names(text: &str, id: Id) -> bool {
    let id = id.to_string();
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| word == id)
}

/// Stops a park whose trigger fired (B24): the park gains `fired` and
/// `fired_by`, and the ruling that fired it gains the park's id in its
/// `evidence`, so the two name each other (B6). The park keeps its
/// file (B3). Both files are prepared before either is written.
pub fn fire(book: &Book, park: Id, by: Id, today: &Date) -> Result<Fired> {
    let loaded = owned(book, park)?;
    let Entry::Park(p) = &loaded.entry else {
        bail!(
            "{park} is a {}, not a park; only a park is fired. To replace a ruling, write the new one and set the old one's `status = \"superseded\"` and `superseded_by`",
            loaded.kind_name()
        );
    };
    if let Some(date) = &p.fired {
        let by_whom = match p.fired_by {
            Some(by) => format!(" by {by}"),
            None => String::new(),
        };
        bail!("{park} already fired on {date}{by_whom}; a park fires once");
    }

    // The ruling that fired it: one this book owns gains the evidence;
    // one an external namespace defines is named and left alone.
    let ruling = match book.get(by) {
        Some(l) => match &l.entry {
            Entry::Decision(d) => Some((l, d)),
            other => bail!(
                "{by} is a {}; a park is fired by the ruling that takes up its work. Write that ruling with `bothy new decision`, then fire {park} by it",
                other.kind_name()
            ),
        },
        None if book.knows(by) => None,
        None => bail!(
            "no entry {by} in this book; write the ruling that fired the trigger with `bothy new decision`, then fire {park} by it"
        ),
    };

    let mut park_doc = document(loaded)?;
    insert_in_order(
        &mut park_doc,
        "fired",
        value(toml_date(today)),
        emit::PARK_KEYS,
    );
    insert_in_order(
        &mut park_doc,
        "fired_by",
        value(by.to_string()),
        emit::PARK_KEYS,
    );

    let ruling_edit = match ruling {
        Some((l, d)) if !d.evidence.iter().any(|e| names(e, park)) => {
            let mut doc = document(l)?;
            match doc.get_mut("evidence").and_then(Item::as_array_mut) {
                Some(list) => list.push(park.to_string()),
                None => {
                    let mut list = Array::new();
                    list.push(park.to_string());
                    insert_in_order(&mut doc, "evidence", value(list), emit::DECISION_KEYS);
                }
            }
            // Read it back now, so a ruling that would not save stops
            // the verb before the park is touched.
            parse_entry(l.kind, &doc.to_string()).map_err(|e| {
                anyhow!(
                    "the change would leave {} unreadable ({}); nothing was written",
                    l.path.display(),
                    e.message()
                )
            })?;
            Some((l, doc))
        }
        _ => None,
    };

    save(book, loaded, &park_doc)?;
    if let Some((l, doc)) = &ruling_edit {
        save(book, l, doc)?;
    }

    Ok(Fired {
        park,
        by,
        date: today.clone(),
        path: loaded.path.clone(),
        evidence_added_to: ruling_edit.map(|(l, _)| l.path.clone()),
    })
}
