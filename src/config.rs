//! `bothy.toml`: what the tool needs to know about one consumer. The
//! areas and the paths they govern (B19), the namespaces with their
//! letters and scopes (B10), the files scanned for citations, and the
//! closed enumerations (B7). Everything else lives in the entries.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use serde::Deserialize;

/// The file name the tool looks for, walking up from the working
/// directory the way cargo looks for `Cargo.toml`.
pub const FILE_NAME: &str = "bothy.toml";

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The consumer's root, relative to the directory holding `bothy.toml`.
    /// Every other path in this file is relative to the root.
    #[serde(default = "default_root")]
    pub root: PathBuf,
    /// Declared areas, keyed by name, in the order they render (B19).
    #[serde(default)]
    pub area: IndexMap<String, Area>,
    /// Namespaces, keyed by a name that is usually the letter (B10).
    #[serde(default)]
    pub namespace: BTreeMap<String, Namespace>,
    /// Where citations are looked for.
    #[serde(default)]
    pub cite: Cite,
    /// Closed enumerations (B7).
    #[serde(default, rename = "enum")]
    pub enums: BTreeMap<String, Enum>,
}

fn default_root() -> PathBuf {
    PathBuf::from(".")
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Area {
    pub title: String,
    /// The title as the consumer's index table shows it, when it differs.
    pub index_title: Option<String>,
    /// The paragraph under the area file's heading, for `render`.
    pub intro: Option<String>,
    /// The paragraph under the area file's Parked heading, for `render`.
    pub parked_intro: Option<String>,
    /// Globs, relative to the root, of the files this area governs.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Documents a session reads before touching this area, which a pack
    /// names with their sizes but does not inline (K11).
    #[serde(default)]
    pub also: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Namespace {
    /// The letter, when it is not the first character of the key.
    pub letter: Option<char>,
    pub kind: Kind,
    /// For a namespace bothy owns: the directory of entry files.
    pub dir: Option<PathBuf>,
    /// Globs of the files within which this letter means this register.
    /// Defaults to everything under the root.
    #[serde(default = "default_scope")]
    pub scope: Vec<String>,
    /// Globs carved out of the scope: files where this letter means
    /// something else (an OAuth constant, a hex digit) and is never an id.
    #[serde(default)]
    pub except: Vec<String>,
    /// For an external namespace: the document or directory defining its ids.
    pub defined_in: Option<PathBuf>,
    /// For an external namespace defined in a document: a regex matched
    /// per line whose first non-empty capture is the id or its number.
    pub pattern: Option<String>,
    pub budget: Option<Budget>,
}

fn default_scope() -> Vec<String> {
    vec!["**".to_string()]
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Decision,
    /// A property that must never be broken; the decision shape, read first.
    Invariant,
    /// A rule about how the project works; the decision shape, read second.
    Principle,
    Park,
    /// Ids bothy knows exist but does not own (B10); parsed for existence only.
    External,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub cross_cutting: Option<usize>,
    pub entry_bytes: Option<usize>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Cite {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Enum {
    pub values: Vec<String>,
}

impl Config {
    /// Finds `bothy.toml` in `start` or the nearest ancestor.
    pub fn find(start: &Path) -> Option<PathBuf> {
        start
            .ancestors()
            .map(|dir| dir.join(FILE_NAME))
            .find(|candidate| candidate.is_file())
    }

    /// Reads and validates one `bothy.toml`.
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let config: Config = toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e.message()))?;
        config.validate(path)?;
        Ok(config)
    }

    fn validate(&self, path: &Path) -> Result<()> {
        for (key, ns) in &self.namespace {
            let letter = ns.letter_or(key);
            if !letter.is_ascii_uppercase() {
                bail!(
                    "{}: namespace \"{key}\" has letter {letter:?}; a letter is one capital A to Z — set `letter = \"X\"` when the key is a longer name",
                    path.display()
                );
            }
            match ns.kind {
                Kind::External if ns.defined_in.is_none() => bail!(
                    "{}: namespace \"{key}\" is external but has no `defined_in`; name the document or directory that defines its ids",
                    path.display()
                ),
                Kind::External => {}
                _ if ns.dir.is_none() => bail!(
                    "{}: namespace \"{key}\" is a {:?} namespace but has no `dir`; name the directory its entry files live in",
                    path.display(),
                    ns.kind
                ),
                _ => {}
            }
        }
        Ok(())
    }
}

impl Namespace {
    /// The letter this namespace resolves: `letter` when set, else the
    /// key's first character.
    pub fn letter_or(&self, key: &str) -> char {
        self.letter.or_else(|| key.chars().next()).unwrap_or('?')
    }
}
