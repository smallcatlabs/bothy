//! bothy — the book the next session reads.
//!
//! A library and a command-line tool that keep the typed, numbered facts
//! a project rules by (decisions, parks, and in time claims, findings and
//! ledger rows) as one TOML file per entry in git, check their integrity,
//! render the markdown people read, and assemble the context an agent
//! needs for one task.
//!
//! The crate is its own first consumer (B16): its rulings are the `B`
//! entries under `bothy/decisions/` and its parks the `K` entries under
//! `bothy/parks/`. Doc comments cite them by id where a rule is held.
//!
//! Module map: [`id`] names things; [`config`] reads `bothy.toml`;
//! [`schema`] is the entry kinds; [`book`] loads a consumer's whole book
//! into memory (B8); [`cite`] finds where ids are named; [`check`] runs
//! the integrity checks (B20); [`pack`] assembles what a session reads
//! for one task (B15); [`emit`] writes an entry as TOML; [`import`] is
//! the one-time migration of a markdown registry, one grammar per
//! source (B26); [`render`] writes the markdown projections (B9);
//! [`mod@write`] is the verbs that change the book (B22).

pub mod book;
pub mod check;
pub mod cite;
pub mod config;
pub mod emit;
pub mod id;
pub mod import;
pub mod pack;
pub mod render;
pub mod schema;
pub mod write;

pub use book::Book;
pub use check::{Diagnostic, Level, Report};
pub use id::Id;
