//! The `bothy` command line. Every verb reads the whole book, answers,
//! and exits (B8); every verb takes `--json` so a script or an agent
//! gets the same answer a reader does. Text written here is written for
//! a reader who has never seen the project (B17).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use bothy::book::Book;
use bothy::check;
use bothy::cite;
use bothy::config::Kind;
use bothy::id::Id;
use bothy::import;
use bothy::pack;
use bothy::render;
use bothy::schema::{Date, Entry};
use bothy::write;

/// bothy — the book the next session reads.
///
/// Keeps a project's decisions and parks as one TOML file per entry in
/// git, checks that every id is unique and every citation resolves, and
/// shows a session what it needs before it touches code.
#[derive(Parser)]
#[command(name = "bothy", version, about, long_about = None, max_term_width = 80)]
struct Cli {
    /// Path to the consumer's bothy.toml. Default: the nearest one at or
    /// above the working directory.
    #[arg(long, global = true, value_name = "PATH")]
    book: Option<PathBuf>,

    /// Print the answer as JSON instead of text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check the book: ids, fields, citations. Exit 1 on any failure.
    ///
    /// Refuses an id out of step with its file name, a duplicate id, an
    /// undeclared area, a value outside a closed enum, and a citation that
    /// no namespace defines. Reports, without failing, the decisions that
    /// nothing in the scan set cites.
    Check,

    /// List entries, one line each.
    List(ListArgs),

    /// Show one entry: its fields, what it names, and what cites it.
    Show {
        /// An id, like B3 or K1.
        id: String,
    },

    /// Assemble what a session reads before one task, as one document.
    ///
    /// Say what the task touches: an area, a file, or an entry. The pack
    /// holds, in this order, the invariants, the principles, the
    /// cross-cutting rulings, the areas' rulings and parks, the documents
    /// to read also, and where each entry is held in code. Its footer
    /// gives bytes, a token estimate and the commit it was built at.
    Pack(PackArgs),

    /// Migrate a markdown registry into entries, once. Writes nothing
    /// under the source tree.
    Import(ImportArgs),

    /// Write the markdown files the book projects, or check them.
    ///
    /// Without flags, every projection in bothy.toml is rendered and
    /// written. With --check nothing is written and the exit is 1 if any
    /// file on disk differs from its render. --diff shows the difference.
    Render {
        /// Compare with the files on disk instead of writing.
        #[arg(long)]
        check: bool,
        /// Print a unified diff for each file that differs; implies --check.
        #[arg(long)]
        diff: bool,
    },

    /// Add an entry: the next free id, a new file, the fields you give.
    ///
    /// Give every field as a flag and the entry is complete in one call.
    /// A required field you leave out is written blank, under a comment
    /// that says what belongs there, and `bothy check` refuses the entry
    /// until you fill it in. Nothing is asked and no editor is opened.
    ///
    /// A decision, invariant or principle takes --title, --ruling and
    /// --why. A park takes --item, --home, --trigger and --why.
    New(NewArgs),

    /// Record that a ruling changed today: add the date to `amended`.
    ///
    /// Change the ruling's text in its file first; this dates the change
    /// and touches no other line. Run twice in one day, it adds the date
    /// once. A park or a ruling no longer in force is refused.
    Amend {
        /// The ruling, like C54.
        id: String,

        /// The owner's words, quoted exactly, when the change was their
        /// call. Refused if the entry already has a verdict.
        #[arg(long, value_name = "TEXT")]
        verdict: Option<String>,

        /// The date to record, like 2026-10-09. Default: today, by this
        /// machine's clock and time zone.
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
    },

    /// Work with parks. `fire` stops one whose trigger has fired.
    ///
    /// To add a park, use `bothy new park`. To see them, `bothy list
    /// parks`; add --retired for the ones that fired.
    Park {
        #[command(subcommand)]
        command: ParkCommand,
    },
}

#[derive(Subcommand)]
enum ParkCommand {
    /// Stop a park whose trigger fired, naming the ruling that fired it.
    ///
    /// The park's file gains today's date as `fired` and the ruling as
    /// `fired_by`, and the ruling's `evidence` gains the park's id, so
    /// each names the other. The park keeps its file and leaves the
    /// lists, the packs and the rendered markdown. Write the ruling
    /// first, with `bothy new decision`.
    Fire {
        /// The park, like K1.
        id: String,

        /// The ruling that fired the trigger, like C110.
        #[arg(long, value_name = "ID")]
        by: String,

        /// The date to record, like 2026-10-09. Default: today, by this
        /// machine's clock and time zone.
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
    },
}

#[derive(Args)]
struct NewArgs {
    /// Which kind of entry to add.
    #[arg(value_enum)]
    kind: NewKind,

    /// The area the entry belongs to, as declared in bothy.toml.
    #[arg(long, value_name = "NAME")]
    area: String,

    /// The rule in one line: the bold lead a reader skims.
    #[arg(long, value_name = "TEXT")]
    title: Option<String>,

    /// What was decided, in full sentences. Give `-` to read the text
    /// from standard input, which spares quoting backticks for the shell.
    #[arg(long, value_name = "TEXT")]
    ruling: Option<String>,

    /// The reason, in a sentence. For a park: why it waits, in a clause.
    #[arg(long, value_name = "TEXT")]
    why: Option<String>,

    /// Park: what is deferred, in one line.
    #[arg(long, value_name = "TEXT")]
    item: Option<String>,

    /// Park: where it lands when it is built.
    #[arg(long, value_name = "TEXT")]
    home: Option<String>,

    /// Park: what reopens it. `never` is legal and means what it says.
    #[arg(long, value_name = "TEXT")]
    trigger: Option<String>,

    /// The namespace letter, needed only when the book has two
    /// namespaces of this kind.
    #[arg(long, value_name = "LETTER")]
    letter: Option<char>,

    /// The date to record, like 2026-10-09. Default: today, by this
    /// machine's clock and time zone.
    #[arg(long, value_name = "DATE")]
    date: Option<String>,
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
enum NewKind {
    Decision,
    Invariant,
    Principle,
    Park,
}

impl NewKind {
    fn kind(self) -> Kind {
        match self {
            NewKind::Decision => Kind::Decision,
            NewKind::Invariant => Kind::Invariant,
            NewKind::Principle => Kind::Principle,
            NewKind::Park => Kind::Park,
        }
    }
}

#[derive(Args)]
struct ImportArgs {
    /// The grammar to read. Only `acquisition` exists.
    #[arg(value_enum)]
    grammar: Grammar,

    /// The source: the directory holding CONTEXT.md and decisions/.
    source: PathBuf,

    /// Where to write bothy.toml and bothy/. Outside the source tree the
    /// config points at the source by absolute path; inside, by relative.
    #[arg(long, value_name = "DIR")]
    out: PathBuf,

    /// Parse and report, writing nothing.
    #[arg(long)]
    dry_run: bool,
}

#[derive(ValueEnum, Clone, Copy)]
enum Grammar {
    Acquisition,
}

#[derive(Args)]
struct PackArgs {
    /// An area to pack; may be repeated.
    #[arg(long, value_name = "NAME")]
    area: Vec<String>,

    /// A file about to be touched; its area is found through the paths
    /// declared in bothy.toml. May be repeated.
    #[arg(long, value_name = "FILE")]
    path: Vec<String>,

    /// One entry: itself, what it names, what names it, where it is held.
    #[arg(long = "for", value_name = "ID")]
    for_id: Option<String>,

    /// Token budget. Blocks are removed from the tail until the pack fits,
    /// and the footer names each one removed.
    #[arg(long, value_name = "TOKENS")]
    budget: Option<usize>,
}

#[derive(Args)]
struct ListArgs {
    /// Which kind to list.
    #[arg(value_enum)]
    kind: KindArg,

    /// Only entries in this area.
    #[arg(long)]
    area: Option<String>,

    /// Only decisions nothing cites.
    #[arg(long)]
    uncited: bool,

    /// Include retired decisions and fired parks, which lists omit by default.
    #[arg(long)]
    all: bool,

    /// Only retired decisions or fired parks.
    #[arg(long, conflicts_with = "all")]
    retired: bool,
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
enum KindArg {
    #[value(alias = "decision")]
    Decisions,
    #[value(alias = "invariant")]
    Invariants,
    #[value(alias = "principle")]
    Principles,
    #[value(alias = "park")]
    Parks,
}

impl KindArg {
    fn kind(self) -> Kind {
        match self {
            KindArg::Decisions => Kind::Decision,
            KindArg::Invariants => Kind::Invariant,
            KindArg::Principles => Kind::Principle,
            KindArg::Parks => Kind::Park,
        }
    }
}

/// One listed or shown entry, with the kind its namespace gives it.
#[derive(Serialize)]
struct Row<'a> {
    kind: &'static str,
    path: &'a std::path::Path,
    #[serde(flatten)]
    entry: &'a Entry,
}

use render::kind_name;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("bothy: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    if let Command::Import(args) = &cli.command {
        return run_import(args, cli.json);
    }
    let book = match &cli.book {
        Some(path) => Book::open(path)?,
        None => Book::find_and_open(&std::env::current_dir()?)?,
    };

    match cli.command {
        Command::Check => {
            let report = check::run(&book)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for d in &report.diagnostics {
                    println!("{d}");
                }
                let failures = report.failures();
                if failures == 0 {
                    println!(
                        "ok    {} entries, {} citations, {} uncited",
                        report.entries,
                        report.citations,
                        report.uncited.len()
                    );
                } else {
                    println!("{failures} failure{}", if failures == 1 { "" } else { "s" });
                }
            }
            Ok(if report.failed() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }

        Command::List(args) => {
            let want = args.kind.kind();
            let uncited: Option<std::collections::BTreeSet<Id>> = if args.uncited {
                let citations = cite::scan(&book)?;
                let cited: std::collections::BTreeSet<Id> =
                    citations.iter().map(|c| c.id).collect();
                Some(
                    book.entries
                        .iter()
                        .map(|l| l.entry.id())
                        .filter(|id| !cited.contains(id))
                        .collect(),
                )
            } else {
                None
            };
            let rows: Vec<&bothy::book::Loaded> = book
                .entries
                .iter()
                .filter(|l| l.kind == want)
                .filter(|l| args.area.as_deref().is_none_or(|a| l.entry.area() == a))
                .filter(|l| match (args.all, args.retired) {
                    (true, _) => true,
                    (_, true) => l.entry.is_retired(),
                    _ => !l.entry.is_retired(),
                })
                .filter(|l| uncited.as_ref().is_none_or(|u| u.contains(&l.entry.id())))
                .collect();
            if cli.json {
                let items: Vec<Row> = rows
                    .iter()
                    .map(|l| Row {
                        kind: kind_name(l.kind),
                        path: &l.path,
                        entry: &l.entry,
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                let area_width = rows.iter().map(|l| l.entry.area().len()).max().unwrap_or(4);
                for l in rows {
                    let id = l.entry.id();
                    let mark = if l.entry.is_retired() {
                        " (retired)"
                    } else {
                        ""
                    };
                    println!(
                        "{:<5} {:<area_width$}  {}{mark}",
                        id.to_string(),
                        l.entry.area(),
                        l.entry.headline().trim(),
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Show { id } => {
            let id = Id::parse(&id).map_err(|e| anyhow!("{e}"))?;
            let loaded = book.get(id).ok_or_else(|| {
                if book.knows(id) {
                    anyhow!("{id} is defined in an external namespace; bothy knows it exists and holds nothing else about it")
                } else {
                    anyhow!("no entry {id} in this book; `bothy list decisions` and `bothy list parks` show what exists")
                }
            })?;
            let citations: Vec<cite::Citation> = cite::scan(&book)?
                .into_iter()
                .filter(|c| c.id == id)
                .collect();
            let names: Vec<Id> = cite::cross_references(&book, &loaded.entry.prose())
                .into_iter()
                .filter(|x| *x != id)
                .collect();
            if cli.json {
                #[derive(Serialize)]
                struct Shown<'a> {
                    #[serde(flatten)]
                    row: Row<'a>,
                    names: Vec<Id>,
                    cited_by: Vec<cite::Citation>,
                }
                let row = Row {
                    kind: kind_name(loaded.kind),
                    path: &loaded.path,
                    entry: &loaded.entry,
                };
                let shown = Shown {
                    row,
                    names,
                    cited_by: citations,
                };
                println!("{}", serde_json::to_string_pretty(&shown)?);
            } else {
                print_entry(&loaded.entry, &loaded.path);
                if !names.is_empty() {
                    let list: Vec<String> = names.iter().map(ToString::to_string).collect();
                    println!("\nNames: {}", list.join(", "));
                }
                println!("\nCited by:");
                if citations.is_empty() {
                    println!("  nothing in the scan set");
                } else {
                    for c in citations {
                        println!("  {}:{}", c.path.display(), c.line);
                    }
                }
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Import(_) => unreachable!("handled before the book is opened"),

        Command::Render { check, diff } => {
            if book.config.projection.is_empty() {
                bail!(
                    "this book declares no [[projection]] in bothy.toml, so there is nothing to render"
                );
            }
            let mut notes = Vec::new();
            if check || diff {
                let rendered = render::all(&book, &mut notes)?;
                let mut differing = Vec::new();
                for r in &rendered {
                    let on_disk =
                        std::fs::read_to_string(book.root.join(&r.path)).unwrap_or_default();
                    if on_disk != r.text {
                        differing.push(r.path.clone());
                        if diff && !cli.json {
                            print!("{}", render::unified(&r.path, &on_disk, &r.text));
                        }
                    }
                }
                if cli.json {
                    #[derive(Serialize)]
                    struct Out<'a> {
                        rendered: Vec<&'a std::path::Path>,
                        differing: &'a [PathBuf],
                    }
                    let out = Out {
                        rendered: rendered.iter().map(|r| r.path.as_path()).collect(),
                        differing: &differing,
                    };
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else {
                    for n in &notes {
                        println!("{n}");
                    }
                    if differing.is_empty() {
                        println!(
                            "ok    {} projection{} match the files on disk",
                            rendered.len(),
                            if rendered.len() == 1 { "" } else { "s" }
                        );
                    } else {
                        for p in &differing {
                            let on_disk =
                                std::fs::read_to_string(book.root.join(p)).unwrap_or_default();
                            let r = rendered.iter().find(|r| r.path == *p).expect("rendered");
                            let (added, removed) = render::line_delta(&on_disk, &r.text);
                            println!(
                                "DIFFERS  {}  {removed} lines on disk would change, {added} new",
                                p.display()
                            );
                        }
                    }
                }
                return Ok(if differing.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                });
            }
            let written = render::write(&book, &mut notes)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&written)?);
            } else {
                for n in &notes {
                    println!("{n}");
                }
                for p in &written {
                    println!("wrote {}", p.display());
                }
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::New(args) => {
            let today = date_or_today(args.date.as_deref())?;
            let ruling = match args.ruling.as_deref() {
                Some("-") => {
                    let mut text = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)?;
                    Some(text)
                }
                _ => args.ruling,
            };
            let draft = write::Draft {
                area: args.area,
                letter: args.letter,
                title: args.title,
                ruling,
                item: args.item,
                home: args.home,
                trigger: args.trigger,
                why: args.why,
            };
            let written = write::new_entry(&book, args.kind.kind(), &draft, &today)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&written)?);
            } else {
                println!(
                    "wrote {}  ({}, a {} in area {})",
                    written.path.display(),
                    written.id,
                    written.kind,
                    written.area
                );
                for d in &written.diagnostics {
                    println!("{d}");
                }
                let render = if book.config.projection.is_empty() {
                    ""
                } else {
                    " and `bothy render`"
                };
                if written.missing.is_empty() {
                    println!("next: run `bothy check`{render}");
                } else {
                    let fields: Vec<String> =
                        written.missing.iter().map(|f| format!("`{f}`")).collect();
                    println!(
                        "next: open the file and fill in {}, then run `bothy check`{render}",
                        fields.join(", ")
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Amend { id, verdict, date } => {
            let id = Id::parse(&id).map_err(|e| anyhow!("{e}"))?;
            let today = date_or_today(date.as_deref())?;
            let amended = write::amend(&book, id, verdict.as_deref(), &today)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&amended)?);
            } else {
                let path = amended.path.display();
                let date = &amended.date;
                match (amended.date_added, amended.verdict_set) {
                    (true, true) => {
                        println!("amended {id}: {path} now carries {date} and the verdict")
                    }
                    (true, false) => println!("amended {id}: {path} now carries {date}"),
                    (false, true) => println!(
                        "{id} was already amended on {date}; {path} now carries the verdict"
                    ),
                    (false, false) => {
                        println!("{id} was already amended on {date}; nothing was written")
                    }
                }
                if !book.config.projection.is_empty() && (amended.date_added || amended.verdict_set)
                {
                    println!("next: run `bothy render`");
                }
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Park {
            command: ParkCommand::Fire { id, by, date },
        } => {
            let id = Id::parse(&id).map_err(|e| anyhow!("{e}"))?;
            let by = Id::parse(&by).map_err(|e| anyhow!("{e}"))?;
            let today = date_or_today(date.as_deref())?;
            let fired = write::fire(&book, id, by, &today)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&fired)?);
            } else {
                println!(
                    "fired {id} by {by} on {}: {} keeps its file and leaves the lists and packs",
                    fired.date,
                    fired.path.display()
                );
                match &fired.evidence_added_to {
                    Some(path) => {
                        println!("added {id} to the evidence of {by}: {}", path.display())
                    }
                    None if book.get(by).is_some() => {
                        println!("{by} already names {id} in its evidence")
                    }
                    None => println!(
                        "{by} is defined outside this book, so its evidence was not changed"
                    ),
                }
                if !book.config.projection.is_empty() {
                    println!("next: run `bothy render`");
                }
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Pack(args) => {
            let for_id = match &args.for_id {
                Some(s) => Some(Id::parse(s).map_err(|e| anyhow!("{e}"))?),
                None => None,
            };
            let req = pack::Request {
                areas: args.area,
                paths: args.path,
                for_id,
                budget: args.budget,
            };
            let pack = pack::build(&book, &req)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&pack)?);
            } else {
                print!("{}", pack.text);
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// The date a write verb records: `--date` when given, else today here.
fn date_or_today(given: Option<&str>) -> Result<Date> {
    match given {
        None => Ok(write::today()),
        Some(s) => {
            Date::parse(s).ok_or_else(|| anyhow!("\"{s}\" is not a date; write it like 2026-10-09"))
        }
    }
}

fn run_import(args: &ImportArgs, json: bool) -> Result<ExitCode> {
    let Grammar::Acquisition = args.grammar;
    let imported = import::parse_playground(&args.source)?;
    if !args.dry_run {
        let summary = import::write(&imported, &args.out, &args.source)?;
        if !json {
            println!(
                "wrote {} entries; the book is {}",
                summary.written,
                summary.config.display()
            );
        }
    }
    if json {
        #[derive(Serialize)]
        struct Out<'a> {
            decisions: usize,
            invariants: usize,
            principles: usize,
            parks: usize,
            areas: Vec<&'a String>,
            exceptions: &'a [String],
            undeclared: &'a std::collections::BTreeMap<char, usize>,
        }
        let out = Out {
            decisions: imported.decisions.len(),
            invariants: imported.invariants.len(),
            principles: imported.principles.len(),
            parks: imported.parks.len(),
            areas: imported.areas.keys().collect(),
            exceptions: &imported.exceptions,
            undeclared: &imported.undeclared,
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        print!("{}", import::report(&imported));
    }
    Ok(if imported.exceptions.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn print_entry(entry: &Entry, path: &std::path::Path) {
    match entry {
        Entry::Decision(d) => {
            println!("{} — {}", d.id, d.title);
            let mut line = format!("{} · area: {} · {}", path.display(), d.area, d.status);
            if let Some(by) = d.superseded_by {
                line.push_str(&format!(" by {by}"));
            }
            if let Some(date) = &d.decided {
                line.push_str(&format!(" · decided {date}"));
                if let Some(from) = &d.decided_from {
                    line.push_str(&format!(" (from {from})"));
                }
            }
            if !d.amended.is_empty() {
                let dates: Vec<String> = d.amended.iter().map(ToString::to_string).collect();
                line.push_str(&format!(" · amended {}", dates.join(", ")));
            }
            println!("{line}");
            println!("\n{}", d.ruling.trim_end());
            if let Some(why) = &d.why {
                println!("\nWhy: {}", why.trim_end());
            }
            if let Some(v) = &d.verdict {
                println!("\nVerdict: \"{}\"", v.trim_end());
            }
            if let Some(h) = &d.history {
                println!("\n{}", h.trim_end());
            }
            print_list("Details", &d.details);
            print_list("Pinned", &d.pinned);
            print_list("Evidence", &d.evidence);
        }
        Entry::Park(p) => {
            println!("{} — {}", p.id, p.item);
            let mut line = format!(
                "{} · area: {} · parked {}",
                path.display(),
                p.area,
                p.parked
            );
            if let (Some(date), Some(by)) = (&p.fired, p.fired_by) {
                line.push_str(&format!(" · fired {date} by {by}"));
            }
            println!("{line}");
            println!("\nHome: {}", p.home.trim_end());
            match &p.trigger {
                Some(t) => println!("Trigger: {}", t.trim_end()),
                None => println!("Trigger: none stated"),
            }
            if let Some(why) = &p.why {
                println!("Why: {}", why.trim_end());
            }
            if let Some(v) = &p.verdict {
                println!("Verdict: \"{}\"", v.trim_end());
            }
            if let Some(m) = &p.measured {
                println!("\nMeasured:\n{}", m.trim_end());
            }
        }
    }
}

fn print_list(label: &str, items: &[String]) {
    if !items.is_empty() {
        println!("{label}: {}", items.join("; "));
    }
}
