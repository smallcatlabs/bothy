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
//! the integrity checks (B20).

pub mod book;
pub mod check;
pub mod cite;
pub mod config;
pub mod id;
pub mod schema;

pub use book::Book;
pub use check::{Diagnostic, Level, Report};
pub use id::Id;
