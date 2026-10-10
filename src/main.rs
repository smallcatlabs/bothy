//! The `bothy` command line. Every verb reads the whole book, answers,
//! and exits (B8); every verb takes `--json` so a script or an agent
//! gets the same answer a reader does. Text written here is written for
//! a reader who has never seen the project (B17).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use bothy::book::Book;
use bothy::check;
use bothy::cite;
use bothy::config::Kind;
use bothy::id::Id;
use bothy::import;
use bothy::pack;
use bothy::schema::Entry;

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

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Decision => "decision",
        Kind::Invariant => "invariant",
        Kind::Principle => "principle",
        Kind::Park => "park",
        Kind::External => "external",
    }
}

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
