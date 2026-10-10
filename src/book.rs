//! The book: one consumer's `bothy.toml` and every entry under it,
//! loaded whole into memory on every run (B8). Nothing is cached and
//! nothing is written here; the book is read, asked, and dropped.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use regex::Regex;

use crate::check::{Diagnostic, Level};
use crate::config::{Config, Kind};
use crate::id::Id;
use crate::schema::{Decision, Entry, Park};

/// One entry as loaded: where it came from and what it says.
#[derive(Debug)]
pub struct Loaded {
    /// Relative to the root.
    pub path: PathBuf,
    /// The key of the namespace that owns it.
    pub namespace: String,
    /// That namespace's kind.
    pub kind: Kind,
    pub entry: Entry,
    /// The file's text, kept so a check can point at a line.
    pub source: String,
}

impl Loaded {
    /// The 1-based line on which `key =` first appears, for a message.
    pub fn line_of(&self, key: &str) -> Option<usize> {
        self.source
            .lines()
            .position(|l| l.starts_with(key) && l[key.len()..].trim_start().starts_with('='))
            .map(|i| i + 1)
    }
}

#[derive(Debug)]
pub struct Book {
    pub config: Config,
    pub config_path: PathBuf,
    /// Absolute.
    pub root: PathBuf,
    pub entries: Vec<Loaded>,
    /// Ids an external namespace defines (B10), by namespace key.
    pub external: BTreeMap<String, BTreeSet<Id>>,
    /// What went wrong while loading; `check` reports these first.
    pub load_failures: Vec<Diagnostic>,
}

impl Book {
    /// Opens the book whose `bothy.toml` is at `config_path`.
    pub fn open(config_path: &Path) -> Result<Book> {
        let config_path = config_path
            .canonicalize()
            .with_context(|| format!("cannot find {}", config_path.display()))?;
        let config = Config::load(&config_path)?;
        let config_dir = config_path
            .parent()
            .ok_or_else(|| anyhow!("{} has no parent directory", config_path.display()))?;
        let root = config_dir
            .join(&config.root)
            .canonicalize()
            .with_context(|| format!("root {} does not exist", config.root.display()))?;

        let mut book = Book {
            config,
            config_path,
            root,
            entries: Vec::new(),
            external: BTreeMap::new(),
            load_failures: Vec::new(),
        };
        book.load_entries()?;
        book.load_external()?;
        Ok(book)
    }

    /// Finds `bothy.toml` from `start` upward and opens it.
    pub fn find_and_open(start: &Path) -> Result<Book> {
        let path = Config::find(start).ok_or_else(|| {
            anyhow!(
                "no bothy.toml in {} or any directory above it; run from inside a consumer, or pass --book <path/to/bothy.toml>",
                start.display()
            )
        })?;
        Book::open(&path)
    }

    fn load_entries(&mut self) -> Result<()> {
        let owned: Vec<(String, Kind, PathBuf)> = self
            .config
            .namespace
            .iter()
            .filter_map(|(key, ns)| {
                ns.dir
                    .as_ref()
                    .filter(|_| ns.kind != Kind::External)
                    .map(|dir| (key.clone(), ns.kind, dir.clone()))
            })
            .collect();

        for (key, kind, dir) in owned {
            let abs = self.root.join(&dir);
            let mut paths: Vec<PathBuf> = match std::fs::read_dir(&abs) {
                Ok(rd) => rd
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                    .collect(),
                Err(_) => {
                    self.load_failures.push(Diagnostic::fail(
                        Some(dir.clone()),
                        None,
                        format!("namespace \"{key}\" names this directory and it does not exist"),
                        Some("create it, or fix `dir` in bothy.toml".into()),
                    ));
                    continue;
                }
            };
            paths.sort();
            for abs_path in paths {
                let rel = abs_path
                    .strip_prefix(&self.root)
                    .unwrap_or(&abs_path)
                    .to_path_buf();
                let source = std::fs::read_to_string(&abs_path)
                    .with_context(|| format!("cannot read {}", rel.display()))?;
                match parse_entry(kind, &source) {
                    Ok(entry) => self.entries.push(Loaded {
                        path: rel,
                        namespace: key.clone(),
                        kind,
                        entry,
                        source,
                    }),
                    Err(e) => {
                        let line = e.span().map(|s| line_at(&source, s.start));
                        self.load_failures.push(Diagnostic::fail(
                            Some(rel),
                            line,
                            e.message().to_string(),
                            Some("fix the file; where a field is unknown, the message lists every field this kind takes".into()),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Reads each external namespace's definitions for existence only (B10).
    fn load_external(&mut self) -> Result<()> {
        let externals: Vec<(String, char, PathBuf, Option<String>)> = self
            .config
            .namespace
            .iter()
            .filter(|(_, ns)| ns.kind == Kind::External)
            .filter_map(|(key, ns)| {
                ns.defined_in.as_ref().map(|d| {
                    (
                        key.clone(),
                        ns.letter_or(key),
                        d.clone(),
                        ns.pattern.clone(),
                    )
                })
            })
            .collect();

        for (key, letter, defined_in, pattern) in externals {
            let abs = self.root.join(&defined_in);
            let mut ids = BTreeSet::new();
            if abs.is_dir() {
                for entry in std::fs::read_dir(&abs)
                    .with_context(|| format!("cannot read {}", abs.display()))?
                {
                    let p = entry?.path();
                    if p.extension().is_some_and(|x| x == "toml")
                        && let Some(stem) = p.file_stem().and_then(|s| s.to_str())
                        && let Some(id) = id_from_stem(stem)
                    {
                        ids.insert(id);
                    }
                }
            } else if abs.is_file() {
                let Some(pattern) = pattern else {
                    self.load_failures.push(Diagnostic::fail(
                        None,
                        None,
                        format!("external namespace \"{key}\" is defined in a document but has no `pattern`"),
                        Some("add a regex whose first capture is the id or its number".into()),
                    ));
                    continue;
                };
                let re = Regex::new(&pattern).with_context(|| {
                    format!("namespace \"{key}\": pattern is not a valid regex")
                })?;
                let text = std::fs::read_to_string(&abs)
                    .with_context(|| format!("cannot read {}", abs.display()))?;
                for line in text.lines() {
                    for cap in re.captures_iter(line) {
                        let Some(m) = cap.iter().skip(1).flatten().next() else {
                            continue;
                        };
                        let s = m.as_str();
                        let parsed = if s.starts_with(|c: char| c.is_ascii_digit()) {
                            s.parse::<u32>().ok().map(|n| Id { letter, number: n })
                        } else {
                            Id::parse(s).ok()
                        };
                        if let Some(id) = parsed {
                            ids.insert(id);
                        }
                    }
                }
            } else {
                self.load_failures.push(Diagnostic::fail(
                    Some(defined_in.clone()),
                    None,
                    format!("external namespace \"{key}\" is defined here and it does not exist"),
                    Some("fix `defined_in` in bothy.toml, or remove the namespace".into()),
                ));
                continue;
            }
            self.external.insert(key, ids);
        }
        Ok(())
    }

    /// Every letter any namespace resolves.
    pub fn letters(&self) -> BTreeSet<char> {
        self.config
            .namespace
            .iter()
            .map(|(k, ns)| ns.letter_or(k))
            .collect()
    }

    pub fn get(&self, id: Id) -> Option<&Loaded> {
        self.entries.iter().find(|l| l.entry.id() == id)
    }

    /// Whether some namespace, owned or external, defines `id`.
    pub fn knows(&self, id: Id) -> bool {
        self.get(id).is_some() || self.external.values().any(|set| set.contains(&id))
    }

    /// The namespaces that resolve `letter` in the file at `rel`
    /// (relative to the root), by scope (B10).
    pub fn namespaces_for(&self, letter: char, rel: &Path) -> Vec<&str> {
        self.config
            .namespace
            .iter()
            .filter(|(k, ns)| ns.letter_or(k) == letter)
            .filter(|(_, ns)| scope_matches(&ns.scope, rel) && !scope_matches(&ns.except, rel))
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Whether `id`, cited from `rel`, resolves in a namespace whose
    /// scope covers that file.
    pub fn resolves(&self, id: Id, rel: &Path) -> Resolution {
        let keys = self.namespaces_for(id.letter, rel);
        if keys.is_empty() {
            return Resolution::NoNamespaceHere;
        }
        for key in keys {
            let ns = &self.config.namespace[key];
            let found = if ns.kind == Kind::External {
                self.external.get(key).is_some_and(|set| set.contains(&id))
            } else {
                self.get(id).is_some_and(|l| l.namespace == key)
            };
            if found {
                return Resolution::Found(key.to_string());
            }
        }
        Resolution::Unknown
    }

    /// The next free id for `letter`: one more than the largest present (B3).
    pub fn next_id(&self, letter: char) -> Id {
        let max = self
            .entries
            .iter()
            .map(|l| l.entry.id())
            .filter(|id| id.letter == letter)
            .map(|id| id.number)
            .max()
            .unwrap_or(0);
        Id {
            letter,
            number: max + 1,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    Found(String),
    /// No namespace of that letter covers the citing file.
    NoNamespaceHere,
    /// A namespace covers the file, but none defines the id.
    Unknown,
}

fn parse_entry(kind: Kind, source: &str) -> Result<Entry, toml::de::Error> {
    Ok(match kind {
        Kind::Decision | Kind::Invariant | Kind::Principle => {
            Entry::Decision(toml::from_str::<Decision>(source)?)
        }
        Kind::Park => Entry::Park(toml::from_str::<Park>(source)?),
        Kind::External => unreachable!("external namespaces have no entry files"),
    })
}

/// `C054` → `C54`.
fn id_from_stem(stem: &str) -> Option<Id> {
    let letter = stem.chars().next().filter(char::is_ascii_uppercase)?;
    let number: u32 = stem[1..].parse().ok()?;
    (number > 0).then_some(Id { letter, number })
}

fn scope_matches(scope: &[String], rel: &Path) -> bool {
    if scope.is_empty() {
        return false;
    }
    let mut builder = globset::GlobSetBuilder::new();
    for g in scope {
        if let Ok(glob) = globset::GlobBuilder::new(g)
            .literal_separator(false)
            .build()
        {
            builder.add(glob);
        }
    }
    builder.build().is_ok_and(|set| set.is_match(rel))
}

/// 1-based line of byte offset `at`.
pub fn line_at(text: &str, at: usize) -> usize {
    text[..at.min(text.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

impl Diagnostic {
    pub fn fail(
        path: Option<PathBuf>,
        line: Option<usize>,
        message: String,
        fix: Option<String>,
    ) -> Self {
        Diagnostic {
            level: Level::Fail,
            path,
            line,
            message,
            fix,
        }
    }

    pub fn note(path: Option<PathBuf>, line: Option<usize>, message: String) -> Self {
        Diagnostic {
            level: Level::Note,
            path,
            line,
            message,
            fix: None,
        }
    }
}
