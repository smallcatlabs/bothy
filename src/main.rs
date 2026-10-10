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
use bothy::id::Id;
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
    #[value(alias = "park")]
    Parks,
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
            let want_decisions = args.kind == KindArg::Decisions;
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
                .filter(|l| matches!(l.entry, Entry::Decision(_)) == want_decisions)
                .filter(|l| args.area.as_deref().is_none_or(|a| l.entry.area() == a))
                .filter(|l| match (args.all, args.retired) {
                    (true, _) => true,
                    (_, true) => l.entry.is_retired(),
                    _ => !l.entry.is_retired(),
                })
                .filter(|l| uncited.as_ref().is_none_or(|u| u.contains(&l.entry.id())))
                .collect();
            if cli.json {
                let items: Vec<&Entry> = rows.iter().map(|l| &l.entry).collect();
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
                    path: &'a std::path::Path,
                    entry: &'a Entry,
                    names: Vec<Id>,
                    cited_by: Vec<cite::Citation>,
                }
                let shown = Shown {
                    path: &loaded.path,
                    entry: &loaded.entry,
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
    }
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
            println!("Trigger: {}", p.trigger.trim_end());
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
