# bothy — where to start

You are in the repository of `bothy`, a command-line tool written in
Rust. It keeps a project's decisions and parks as one TOML file per
entry, checks them, and shows a session what it needs before it touches
code. This repository is also its own first consumer: the tool's own
rulings are entries in `bothy/decisions/` and its deferred work in
`bothy/parks/`.

## Read first

1. `README.md`, for what the tool is and the commands that exist.
2. Run `cargo build && ./target/debug/bothy check`. The output is the
   state of the book.
3. Run `./target/debug/bothy pack --path <the file you will change>`.
   It prints the rulings and parks for that file's area, the
   cross-cutting rulings, and where each is held in code, as one
   document. `--area <name>` does the same for a whole area;
   `bothy list decisions` and `bothy show B3` are the smaller views.
   The areas and the paths each one governs are declared in
   `bothy.toml`.
4. `docs/design-brief.md` is history: the design as it stood when the
   project began, edited in place where rulings changed it. Where it and
   an entry disagree, the entry wins.

## How to work here

- **Before you change code, read the rulings for its area.** The id of a
  ruling appears in a doc comment wherever code holds it. If you change
  what the code does, change or add the ruling too.
- **To add a ruling**, copy an existing file in `bothy/decisions/`, give
  it the next free id (one more than the largest present) and name the
  file by it with the number padded to three digits, the way `B7` lives
  in `B007.toml`. Fill in the fields and run `check`. Until a `new`
  command exists, this is the way. The same goes for a park in
  `bothy/parks/`, where `K7` lives in `K007.toml`.
- **A ruling is never deleted.** To replace one, add the new ruling,
  then set the old one's `status = "superseded"` and
  `superseded_by = "<new id>"`. To stop a park, set `fired = <date>` and
  `fired_by = "<the ruling that fired it>"`.
- **Write plainly.** Help text, error messages, comments and entries are
  read by people and by models smaller than you, who have never seen
  this project. Say what is wrong and what to do.
- **Keep the gate green.** `tools/gate.sh` builds, tests, lints, formats
  and runs `check`. Run it before you commit.
- **Commit messages carry the story.** What changed and why, so `git
  log` reads as the project's history. Each concern in its own commit.

## What exists and what does not

`check`, `list`, `show`, `pack` and `import` exist. `render`, `new`,
`amend` and `park fire` are described in the brief and do not exist
yet; the uncited rulings `check` reports are the ones waiting for them.

`import` reads acquisition's registry from the owner's clone at
`~/Development/GitHub/gerwaric/acquisition/spikes/rust-playground` and
writes to a directory you name, never into that clone. Acquisition adopts
bothy only after its search slice closes (the owner, 2026-10-10); until
then it is test data, and `tests/import_acquisition.rs` is the test.
