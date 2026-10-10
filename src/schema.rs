//! The entry kinds. One TOML file per entry (B5); prose fields carry
//! their author's words verbatim and are never reflowed (B18). Unknown
//! fields are refused, so a misspelled key is a loud failure rather than
//! a silently ignored one (B17).

use std::fmt;

use serde::{Deserialize, Serialize, Serializer};

use crate::id::Id;

/// A calendar date, `2026-10-09`, written as a TOML local date.
#[derive(Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(transparent)]
pub struct Date(toml::value::Datetime);

impl Date {
    /// `2026-10-09`; anything else is `None`.
    pub fn parse(s: &str) -> Option<Date> {
        let dt: toml::value::Datetime = s.parse().ok()?;
        let d = Date(dt);
        d.is_plain_date().then_some(d)
    }

    /// A date carries a day and nothing else; a time or an offset here is
    /// a mistake the check reports.
    pub fn is_plain_date(&self) -> bool {
        self.0.date.is_some() && self.0.time.is_none() && self.0.offset.is_none()
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

/// A ruling: what was decided, why, and where the rest lives.
#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub id: Id,
    pub area: String,
    /// The bold lead, verbatim, with whatever terminator its author gave
    /// it. A principle has none; a decision or invariant must.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    pub ruling: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// Where the mechanism lives: a module doc, a file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<String>,
    /// The tests that hold it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pinned: Vec<String>,
    /// Claim ids, park ids, ledger rows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided: Option<Date>,
    /// `"git"` when the importer took the date from history (B13).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_from: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub amended: Vec<Date>,
    /// The owner's words, when quoted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    /// The dating sentence as its author wrote it, when an imported entry
    /// carries one ("Ruled 2026-09-17 (\"I agree to 1 entirely\")."); it
    /// renders last and is never reflowed (B18). `decided` and `amended`
    /// are read from it when they are not set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<String>,
    /// One of the consumer's `decision_status` values; `ruled` by default.
    #[serde(default = "default_status")]
    pub status: String,
    /// The id that replaced this one; only with `status = "superseded"` (B3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<Id>,
}

fn default_status() -> String {
    "ruled".to_string()
}

/// The built-in decision statuses, used when the consumer declares none.
pub const DEFAULT_DECISION_STATUS: &[&str] = &["ruled", "superseded", "retired"];

/// Scope deferred with the trigger that reopens it. A park has an id
/// like any other entry (B6).
#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Park {
    pub id: Id,
    pub area: String,
    /// What is deferred.
    pub item: String,
    /// Where it lands when built.
    pub home: String,
    /// What reopens it. `"never"` is legal and means what it says. A park
    /// without one is reported by `check`, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    pub parked: Date,
    /// `"git"` when the importer took the date from history (B13).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked_from: Option<String>,
    /// Set when the trigger fired; the file stays (B3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fired: Option<Date>,
    /// The ruling that fired it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fired_by: Option<Id>,
    /// Measured data the park carries, kept out of its one-line fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured: Option<String>,
}

/// A fact about the world outside the project, with what it rests on
/// (B25). It names no area: a claim is cited by number from wherever it
/// bears, and `layer` is the heading it sits under in its register.
#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: Id,
    /// The section of the register the claim sits under, free text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub layer: String,
    /// The claim itself, in its author's words.
    pub statement: String,
    /// What it rests on, each a value of the consumer's `source` enum (B7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// One of the consumer's `confidence` values (B7). A claim without
    /// one is reported by `check`, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    /// The qualifier that rode on the confidence word: "for 15 of 22
    /// groups; September 8, 2026".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The source-and-confidence tag as its author wrote it, when an
    /// imported claim carries one ("TOM + Cloudflare public docs — High").
    /// `sources`, `confidence` and `scope` are the typed reading of it;
    /// this keeps the words the enums cannot (B18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis: Option<String>,
    /// The day the evidence was observed, when the claim states one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed: Option<Date>,
    /// The evidence and its consequences: quotes, tables, reasoning.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
}

/// A problem found in the code, open or settled, in one shape for both
/// states (B25). It names no area: a finding is anchored to symbols.
#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: Id,
    pub title: String,
    /// One of the consumer's `finding_status` values (B7).
    pub status: String,
    /// What is being done about it: "won't fix unless touched".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<String>,
    /// How and where it was settled: "Fixed, Phase 2", "PR #161".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_in: Option<String>,
    /// The mechanism, the evidence, and any rule it left standing.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
}

/// The built-in confidence values, used when the consumer declares none (B7).
pub const DEFAULT_CONFIDENCE: &[&str] = &["confirmed", "high", "provisional", "hypothesis"];

/// The built-in finding statuses, used when the consumer declares none.
pub const DEFAULT_FINDING_STATUS: &[&str] = &[
    "suspected",
    "likely",
    "confirmed",
    "resolved",
    "retired",
    "superseded",
];

/// The finding statuses that mean the finding is settled. A settled
/// finding keeps its file and leaves lists and packs (B3, B25).
pub const SETTLED_FINDING_STATUS: &[&str] = &["resolved", "retired", "superseded"];

/// Any entry shape bothy owns. The kind is the namespace's: an invariant
/// and a principle use the [`Decision`] shape, so one struct serves
/// three kinds and the kind decides where the entry renders and where
/// it sits in a pack (B15).
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Entry {
    Decision(Decision),
    Park(Park),
    Claim(Claim),
    Finding(Finding),
}

impl Entry {
    pub fn id(&self) -> Id {
        match self {
            Entry::Decision(d) => d.id,
            Entry::Park(p) => p.id,
            Entry::Claim(c) => c.id,
            Entry::Finding(f) => f.id,
        }
    }

    /// The area the entry names; `None` for a claim or a finding, which
    /// name none (B25).
    pub fn area(&self) -> Option<&str> {
        match self {
            Entry::Decision(d) => Some(&d.area),
            Entry::Park(p) => Some(&p.area),
            Entry::Claim(_) | Entry::Finding(_) => None,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Entry::Decision(_) => "decision",
            Entry::Park(_) => "park",
            Entry::Claim(_) => "claim",
            Entry::Finding(_) => "finding",
        }
    }

    /// The one line a list shows: a decision's title, a park's item, a
    /// claim's statement, a finding's title.
    pub fn headline(&self) -> &str {
        match self {
            Entry::Decision(d) => &d.title,
            Entry::Park(p) => &p.item,
            Entry::Claim(c) => &c.statement,
            Entry::Finding(f) => &f.title,
        }
    }

    /// A retired entry keeps its file and leaves every projection and
    /// pack (B3): a decision that is not `ruled`, a park that has fired,
    /// a finding that is settled. A claim is never retired; one that
    /// falls is rewritten in place and says so.
    pub fn is_retired(&self) -> bool {
        match self {
            Entry::Decision(d) => d.status != "ruled",
            Entry::Park(p) => p.fired.is_some(),
            Entry::Claim(_) => false,
            Entry::Finding(f) => SETTLED_FINDING_STATUS.contains(&f.status.as_str()),
        }
    }

    /// Every prose field, named, for cross-reference extraction.
    pub fn prose(&self) -> Vec<(&'static str, &str)> {
        let mut out = Vec::new();
        match self {
            Entry::Decision(d) => {
                out.push(("title", d.title.as_str()));
                out.push(("ruling", d.ruling.as_str()));
                if let Some(w) = &d.why {
                    out.push(("why", w));
                }
                for s in &d.details {
                    out.push(("details", s));
                }
                for s in &d.pinned {
                    out.push(("pinned", s));
                }
                for s in &d.evidence {
                    out.push(("evidence", s));
                }
                if let Some(v) = &d.verdict {
                    out.push(("verdict", v));
                }
                if let Some(h) = &d.history {
                    out.push(("history", h));
                }
            }
            Entry::Park(p) => {
                out.push(("item", p.item.as_str()));
                out.push(("home", p.home.as_str()));
                if let Some(t) = &p.trigger {
                    out.push(("trigger", t));
                }
                if let Some(w) = &p.why {
                    out.push(("why", w));
                }
                if let Some(v) = &p.verdict {
                    out.push(("verdict", v));
                }
                if let Some(m) = &p.measured {
                    out.push(("measured", m));
                }
            }
            Entry::Claim(c) => {
                out.push(("statement", c.statement.as_str()));
                if let Some(x) = &c.scope {
                    out.push(("scope", x));
                }
                if let Some(x) = &c.basis {
                    out.push(("basis", x));
                }
                out.push(("body", c.body.as_str()));
            }
            Entry::Finding(f) => {
                out.push(("title", f.title.as_str()));
                if let Some(x) = &f.disposition {
                    out.push(("disposition", x));
                }
                if let Some(x) = &f.resolved_in {
                    out.push(("resolved_in", x));
                }
                out.push(("body", f.body.as_str()));
            }
        }
        out
    }

    /// Every date the entry carries, named, so the check can refuse one
    /// that is not a plain date.
    pub fn dates(&self) -> Vec<(&'static str, &Date)> {
        let mut out = Vec::new();
        match self {
            Entry::Decision(d) => {
                if let Some(x) = &d.decided {
                    out.push(("decided", x));
                }
                for x in &d.amended {
                    out.push(("amended", x));
                }
            }
            Entry::Park(p) => {
                out.push(("parked", &p.parked));
                if let Some(x) = &p.fired {
                    out.push(("fired", x));
                }
            }
            Entry::Claim(c) => {
                if let Some(x) = &c.observed {
                    out.push(("observed", x));
                }
            }
            Entry::Finding(_) => {}
        }
        out
    }
}
