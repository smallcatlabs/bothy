<!-- Rendered by bothy from the book; edit the entries, not this file. -->
# The book

Every ruling and every park, by area, rendered from the entries.

bothy is a command-line tool, written in Rust, that keeps a project's decisions and parks as one TOML file per entry, checks them, and assembles what a session reads before it changes code. This repository is the tool and its own first consumer: the rulings here are the tool's own, and the code under src/ is what they govern.

## What every area must know

- **B1 — The source is structured text in git, one entry per file, that a person can edit with nothing but a text editor** No binary, no server, no lock, no tool required to add or change an
entry. A hand-written entry is as valid as one the tool wrote; this
book's first twenty decisions and ten parks were written by hand before
the tool existed. *Why:* The next session may arrive with nothing but a shell, and the owner with nothing but an editor. Decided 2026-10-09.
- **B2 — Every derived artifact is reproducible from the source and is never the source** The index is rebuilt on every run. A rendered markdown file is a
projection: its first line names the source it was rendered from, and
`check` reports a projection on disk that differs from what the source
renders now. *Why:* Two copies of a fact rot apart; one is the source and the rest are its shadows. Decided 2026-10-09.
- **B3 — An id is never reused, and a retired entry keeps its file** A superseded decision stays as a file with `status = "superseded"` and
the id that replaced it. A park whose trigger fired stays with the date
it fired and the id that fired it. The next id is one more than the
largest id present. Projections and packs omit retired entries, so a
retired file costs no reader anything. *Why:* Deleting the file and reading git for the high-water mark puts git in the hot path and makes 'what fired, and by what' archaeology; a kept file makes it a list. Alternative considered: the brief's deletion rule, which acquisition uses because a bullet in an always-loaded file taxes every session; a TOML file nobody loads taxes no one. Decided 2026-10-09.
- **B4 — The tool never writes outside its own tree and its declared projections** It scans a consumer's code and documents for citations; it never
inserts, edits or reflows anything there. The only files it writes are
entries under the directories `bothy.toml` names and the projections it
declares. *Why:* A tool that edits code needs a review a registry tool should never need. Decided 2026-10-09.
- **B8 — The index is in memory, rebuilt every run; there is no database** Every verb reads the whole book and the citation sites, builds its maps,
answers and exits. Every verb takes `--json`, so a script or an agent
gets the same answer a reader does. Alternative considered: SQLite,
rebuilt per run and gitignored (the brief's D4), parked as K7. *Why:* The first consumer has about 170 entries and a few thousand citation sites, which two hash maps answer instantly; a bundled database is a C compile, a dependency and a gitignored file for a question nobody has asked yet. Decided 2026-10-09.
- **B17 — Plain register: everything bothy writes for a reader is written for someone who has never seen the project and is the smallest model in use** Help text, pack headings, check messages and the comments in a template
`new` writes. A check message names the file and line and says what to
do. The legibility seat, when it runs, reads with the smallest model in
use: if a Sonnet-class model acts correctly from the pack, every model
does. *Why:* A compressed register taxes every fresh session and taxes a smaller model most (acquisition note 35 §6.1); the owner asked that Opus and Sonnet-class models be consumers, not only readers. Owner, verbatim: "also consider Opus and Sonnet-class models as consumers and users" Decided 2026-10-09.

Parked, each with the trigger that reopens it:

- A SQLite index, rebuilt per run and gitignored (the brief's D4) → behind the same query surface the in-memory maps serve (B8). Trigger: a query that is awkward over in-memory maps, or a consumer with more than five thousand entries

## The source: entry kinds, files, ids, namespaces, areas

- **B5 — TOML, one file per entry, named by its zero-padded id, under a bothy/ directory beside the documents it renders** `bothy/decisions/C054.toml`, `bothy/parks/K012.toml`. Prose fields are
multiline strings carrying the author's words verbatim. Alternative
considered: one array per area, rejected because two sessions adding an
entry would edit the same file and a duplicate id inside an array is
silent. Markdown with front matter was weighed and set aside: entries
are short typed records, and the audit's lesson is that prose
conventions drift while TOML has no grammar to drift. *Why:* Two sessions adding entries touch two files, or collide loudly on one, never silently. Decided 2026-10-09.
- **B6 — Parks have ids, in a letter the consumer chooses** Acquisition's parks and bothy's own both use `K`. A park's id is named
by the ruling that fired it (in `evidence`) and by the park's own
`fired_by` field, so the two point at each other. *Why:* fire, list and citation all need a name; acquisition's parks have none today. Decided 2026-10-09.
- **B7 — Enumerations are closed per project and declared in bothy.toml** Confidence (`confirmed`, `high`, `provisional`, `hypothesis`), source
tokens, finding status and decision status are lists in the consumer's
`bothy.toml`. A value outside its list is a check failure, and the
qualifier that used to ride on the word goes in `scope`: "Confirmed for
15 of 22 groups" becomes `confidence = "confirmed"`, `scope = "15 of 22
groups"`. A ledger kind declares its columns the same way, because the
two ledgers in use today have different columns. *Why:* Twenty-eight phrasings of confidence exist today for four meanings; the enum keeps the meaning and `scope` keeps the words. Decided 2026-10-09.
- **B10 — A namespace is a letter and a scope: the path globs within which that letter means this register** A citation resolves by its letter and by the file it occurs in. A
namespace bothy owns has a `dir`; an external one has the document that
defines its ids and a pattern for a definition, and is parsed for
existence only. The same letter may name two registers in two scopes. A
citation outside every scope of its letter is unresolved and `check`
says so. A scope may carve files out with `except`, for a file where the
letter and a number mean something that is not an id (amended
2026-10-10: `S256` in the daemon's OAuth code is the PKCE code-challenge
method, found by the first import of acquisition). *Why:* Measured on acquisition at bfa59416: `F` is a finding in docs/cleanup/findings.md (F1 to F78) and a fault in the search slice (F1 to F10, cited from search/LEDGER.md); a per-letter table cannot hold both. Decided 2026-10-09; amended 2026-10-10.
- **B13 — A date the prose lacks is filled from git at import and marked as such** `decided_from = "git"` sits beside a `decided` the importer took from
the commit that introduced the entry, so a reader can tell a measured
date from a recalled one. *Why:* Acquisition applies the same rule to sizes: a number stated from memory is marked, a number measured is not. Decided 2026-10-09.
- **B19 — Areas are declared in bothy.toml with a title and the paths they govern** An entry naming an undeclared area is refused. `pack --path` resolves
an area through the declared paths. `cross-cutting` is declared like any
other area and is the one every pack includes. *Why:* A misspelled area would otherwise become a new area, silently. Decided 2026-10-09.
- **B25 — A claim and a finding are entry kinds with their own shapes, and neither names an area** A namespace of `kind = "claim"` holds claims: a `statement`, the
`sources` it rests on, a `confidence`, the `scope` that qualifies the
confidence, the day it was `observed`, a `body`, and the `layer` it
sits under in its register. A namespace of `kind = "finding"` holds
findings: a `title`, a `status`, a `disposition`, where it was
`resolved_in`, and a `body`, one shape whether the finding is open or
settled. Neither has an `area`: a claim is cited by number from
wherever it bears and a finding is anchored to symbols, so a book of
only claims and findings declares no area at all, and B19 binds the
kinds that name one. `confidence` and `finding_status` have built-in
lists a consumer may replace in `bothy.toml` (B7). `source` has none,
because source words are each project's own: a book whose claims name
sources declares `[enum.source]` or `check` refuses it. A claim with no
confidence is reported, not refused. An imported claim keeps its tag as
its author wrote it in `basis`, beside the typed reading, the way a
decision keeps `history` (B18). A finding whose status is `resolved`,
`retired` or `superseded` is retired in B3's sense: it keeps its file
and leaves lists and packs. A claim is never retired; one that falls is
rewritten in place. *Why:* Master's 80 claims and 78 findings import under these shapes with no exception (2026-10-10, at acquisition master 946a4f51). An area on either would have been one the importer invented, and a closed list of sources built into the tool would have been acquisition's words forced on the next consumer. Decided 2026-10-10.

Parked, each with the trigger that reopens it:

- Jujutsu-aware blame (change ids instead of commit hashes) → import's date filling (B13). Trigger: the jj trial in acquisition surviving its first slice
- Ledger rows as an entry kind: one file per run, with the columns declared in bothy.toml (the brief, section 4.5; B7's last sentence) → a fifth shape in src/schema.rs and a `kind = "ledger"` namespace, with a table projection. Trigger: a consumer adopting bothy whose run ledger a session has to cite or check; acquisition's two ledgers are the test data (done-criterion 4 names claims and findings only, and no reader of a book has asked for a ledger row yet)

## Integrity: what check refuses and what it reports

- **B20 — check is one command, exits non-zero on any failure, and names each offender** It refuses: an id out of step with its file name, a duplicate id, a
citation that does not resolve in its scope, an enum value outside its
list, an undeclared area, a projection that differs from its render, and
an entry that says nothing: a decision with no title, a principle with
neither title nor ruling, a park with a blank item or home (amended
2026-10-10, when `new` began to write blanks for the author to fill). A
blank ruling under a title is legal, because a title may be the whole
rule. It reports, without failing: a decision nothing cites. A failure
is one line: file, line, what is wrong, what to do. The consumer's own
extra checks run last. *Why:* This replaces three of docs-check.sh's seven checks and must be at least as loud. Decided 2026-10-09; amended 2026-10-10.

Parked, each with the trigger that reopens it:

- A generic 'backticked identifiers resolve' check (acquisition's docs-check §3) with configurable roots → check, as a built-in with roots in bothy.toml. Trigger: a second consumer wanting it; until then it stays in acquisition's extra checks

## Projections: templates and the markdown they write

- **B9 — Templates are minijinja files in the consumer, with built-in defaults that reproduce acquisition's current form** A consumer that writes no template gets the built-in shape. A consumer
that wants another shape writes one and names it in a `[[projection]]`. *Why:* v0's golden render passes without the first consumer writing a template. Decided 2026-10-09.
- **B18 — The tool never reflows prose** A string renders as stored, newlines included. Import stores today's
rulings as the single lines they are. A ruling wrapped by hand in its
source renders wrapped, and CommonMark's lazy continuation keeps it
inside its bullet. *Why:* The golden render passes by construction, and prose diffs stay line-shaped. Decided 2026-10-09.
- **B21 — An area's reading order is projection data: the layout in bothy.toml, never a field on the entry** An area block declares its sections, each with its decision ids in
reading order, and its parks in order. The renderer follows the layout;
an entry the layout does not place renders after the placed ones, by
id, and `check` says so. An entry knows its area and nothing about
where in the file it sits. *Why:* The order of acquisition's files is curated, not ascending: C108 sits with the store facts it extends and C85 beside C9. Order is a rendering concern, so it lives with the template, and an entry file stays a fact about one ruling. Decided 2026-10-10.

Parked, each with the trigger that reopens it:

- A date `amend` adds to an imported entry does not render: an entry with a `history` sentence renders that sentence and ignores `amended` → the dating clause in src/render.rs, or an `amend` that extends `history`. Trigger: acquisition adopting bothy, where 108 decisions carry `history` and the first of them is amended (the byte-for-byte render of acquisition depends on `history` rendering verbatim (B18), and how an amendment should read there is the owner's call when it is first needed)

## Assembly: what a session reads for one task

- **B15 — pack is the primary read surface** It takes `--area <name>`, `--path <file>` (the area resolved through the
paths `bothy.toml` declares) or `--for <id>`. Its order is fixed:
invariants, principles, cross-cutting rulings, the area's rulings, its
parks, the cross-cutting parks, the documents to read also, then the
citation map from id to file:line so the reader knows where each
mechanism lives. A document to read also is one the area's `also` list
names: the pack gives its path, its size and, when the list states one,
the reason to open it, and never its text (K11 holds inlining a
section). The ruling first promised "the open slice's checklist" here,
which was never built; the named document is what exists (amended
2026-10-10). A cross-cutting park is in every pack for the reason a
cross-cutting ruling is: scope deferred across every area is deferred
in this one (amended 2026-10-10, when the store pack of acquisition was
found to leave out the one park its `CONTEXT.md` shows every session). Its footer
carries bytes, a token estimate with the estimator's name, and the
commit it was built at. `--budget` trims from the tail and names what it
trimmed; nothing is trimmed silently. *Why:* It is the verb every session runs first; whether it is seamless is whether the tool is. Decided 2026-10-09; amended 2026-10-10.
- **B27 — A book states what the project is once, in bothy.toml, and every pack opens with it** `orientation` in `bothy.toml` is a few sentences for a reader who has
never seen the project: what it is, and what outranks everything else
in it. An area pack prints it directly under its heading, before any
ruling, because the rulings name things only it explains. A template
may print it as `{{ orientation }}`, and the built-in book does, so a
rendered file and a pack say the same words from one source (B2). It is
prose, not an entry: it rules nothing and nothing cites it. A sentence
in it that binds anyone belongs in an invariant or a principle instead.
The acquisition import takes the paragraph under `## Orientation` in
`CONTEXT.md` as the orientation and leaves a tag in its place. *Why:* The store pack of acquisition gave a reader five invariants about GGG, Cloudflare and rate-limit standing and never said what GGG is; CONTEXT.md says it in three sentences that the pack left behind. The reader who pays for that is the smallest model (B17). Decided 2026-10-10.

Parked, each with the trigger that reopens it:

- Token counting by the exact model tokenizer → the pack footer, beside the estimate. Trigger: a budget decision the estimate got wrong by more than ten percent
- Inlining a section of a document a pack names under 'read also' (a slice record's checklist, say), rather than naming the file with its size → the `also` list in an area's bothy.toml block, with a way to name a section. Trigger: the acquisition import landing, and a session that wanted the checklist inline rather than named (a whole slice record would swamp any budget; the right grain is a section, and no consumer has one yet)

## The command line, its output, and the servers over the same crate

- **B14 — Writes go through toml_edit; the tool never reorders keys, rewrites values it did not change, or strips comments** `amend` adds a line; it does not reformat the file. Optional fields are
omitted, not written empty, in files the tool creates. *Why:* An entry is its author's file. Decided 2026-10-09.
- **B22 — A write verb takes every field as a flag, never prompts, and leaves a blank only for what it was not given** `bothy new <kind> --area <name>` takes the next free id (B3), counting
files that do not parse, and creates the file; it never replaces one
that exists, so two sessions that race collide loudly (B5). A field
given as a flag is written as given, and `--ruling -` reads the text
from standard input so backticks need no quoting for the shell. A
field the kind expects and was not given is written blank under a
comment that says what belongs there. `check` refuses a decision with
no title and a park with no item or home (B20), so the gate stays red
until those are filled; a blank ruling under a title is legal and the
comment says when. An entry given whole carries no comments. The verb
then reads the book back and prints what `check` says about the new
file. A park is made by `new park`; there is no second spelling under
`park`.
The date a write verb records is the calendar day on the machine's own
clock and time zone, and `--date` overrides it. The verbs are functions
in the library that take the date as an argument; the command line is
one caller and a server will be another (K8). *Why:* The daily user is an agent in a shell, with no terminal to answer a prompt and no editor to be opened into. One call that writes a whole entry is the pleasant path; a blank that keeps the gate red until it is filled is the safe one. Decided 2026-10-10.
- **B23 — `amend` dates a change to a ruling in force, adds one line, and never replaces the owner's words** `bothy amend <id>` adds today's date to `amended`, once however many
times it runs that day, and with `--verdict` sets the owner's words
where the entry has none. It refuses a park, a ruling that is
superseded or retired (and names the ruling to amend instead), and a
second verdict. The author changes the ruling's text by hand first; the
verb dates the change. A field it adds goes on the line where a written
entry has it, and no other line changes: comments, quoting and key
order stay the author's (B14). The edited text is read back as an entry
before it is written, and an edit that would not read is not written. *Why:* A verdict is a quotation and a date list is a record; a tool that overwrites either leaves git as the only witness. The text of the change is the author's to write, so the verb has no flag for it. Decided 2026-10-10.
- **B24 — `park fire` stops a park once, by a ruling that exists, and makes the two name each other** `bothy park fire <park> --by <ruling>` sets `fired` to today and
`fired_by` to the ruling in the park's file, and adds the park's id to
the ruling's `evidence` unless the evidence already names it (B6). The
park keeps its file and leaves the lists, packs and projections (B3).
It refuses a park that already fired, an id that is not a park, and a
`--by` that is a park or does not exist: the ruling is written first.
A ruling an external namespace defines may fire a park and is left
unchanged, because bothy does not own its file. Both files are prepared
and read back before either is written (B14, B23). *Why:* The question a reflection asks is "what fired, and by what"; it is a list only if both ends were written at the moment of firing, and a park fired by a ruling nobody wrote is a park deleted. Decided 2026-10-10.

Parked, each with the trigger that reopens it:

- A web or TUI view of the book → a separate binary over the same crate. Trigger: two sessions wanting it
- An MCP server exposing the read verbs and the two write verbs over the same crate → a second binary or a subcommand; rmcp and tokio arrive with it (B12). Trigger: v0's done-criteria met; this is the first v1 item because a smaller model gains most from a typed tool call over a remembered flag (B17)
- The seat verb: the legibility seat's questions, pack and transcript, scored → a subcommand over pack. Trigger: the first pack worth measuring, after v0's done-criteria
- The stats verb (the baseline table) and a standalone blame verb → subcommands over the loaded book. Trigger: the next acquisition reflection wanting the baseline table from one command
- A verb that supersedes a ruling: writes the new one and sets the old one's `status` and `superseded_by` in one call → a `--supersedes <id>` flag on `bothy new decision`, over the same in-place edit `amend` uses (B23). Trigger: the first ruling superseded by hand, or a `check` failure on a superseded pair that a session got wrong (no ruling in this book has been superseded yet, and the two lines are short to write by hand)
- Writing and rendering claims and findings: `new claim`, `new finding`, a `list findings --status` filter, and a projection that renders a register back to markdown → src/write.rs for the verbs, a built-in template in src/render.rs for the registers. Trigger: master adopting bothy as its live book, which is the owner's call after acquisition's search slice closes (until then master's claims are authored in its documents and imported on demand, so nothing writes a claim through the tool)

## This repository: language, dependencies, consumers, release

- **B11 — Bothy lives in its own repository under Small Cat Labs; a consumer depends on it as a development tool, never as a workspace member** `cargo install --git` or a pinned binary. Acquisition's crate graph (its
C1) is untouched, and the tool's release cadence is its own. *Why:* A registry tool inside the consumer's workspace would be a dependency edge its own check would have to explain. Decided 2026-10-09.
- **B12 — Rust, edition 2024: clap, serde, toml and toml_edit, minijinja, globset and ignore, regex, jiff** No async in v0. `rmcp` and `tokio` arrive with the MCP server (K8) and
not before. A dependency is added when a verb needs it, in the commit
that adds the verb. Amended 2026-10-10: `jiff` arrived with `new`, which
needs today's date in the machine's time zone and not in UTC (B22). *Why:* Each is the plain choice for its job; async is a cost with no v0 buyer. Decided 2026-10-09; amended 2026-10-10.
- **B16 — Bothy is its own first consumer** Bothy's rulings and parks live in `bothy/` in this repository, written
by hand before the tool existed. The design brief is history once they
exist: where the brief and an entry disagree, the entry wins.
Acquisition's playground is the second consumer and master's claims and
findings the third. *Why:* A generalization wants two consumers before it is made; this is the cheapest one, and the one that tests writing the format by hand. Decided 2026-10-09.
- **B26 — The second-consumer test is on the verbs that read a book, not on the importer** The brief's done-criterion 4 says master's claims and findings import
under a second `bothy.toml` "with no code change". As written it could
not be met: the claim and finding kinds did not exist, and master's
markdown is not the playground's. What the criterion tests is that
nothing in the tool is acquisition's playground in disguise. So it is
read this way: once a kind exists (B25), everything that differs
between consumers is in each one's `bothy.toml`, and `check`, `list`,
`show` and `pack` hold no branch that names a consumer. A grammar under
`import` is the one place code may know a project's markdown: one
module per source, run once per adoption, and named for its source.
Master's is `bothy import acquisition-master`. Met 2026-10-10: master's
book declares three namespaces, no area and no projection, and the read
verbs run on it unchanged. *Why:* An importer reads a project's prose conventions, which differ per project by nature; the verbs read TOML, which does not. Holding the importer to 'no code change' would mean either never importing a second project or pretending two markdown registries share a grammar. Decided 2026-10-10.

Parked, each with the trigger that reopens it:

- Session capture in the style of Agent Trace or Entire checkpoints → a separate tool; bothy keeps rulings, not transcripts. Trigger: a review round whose question 'why did the agent do that' the commit message could not answer (acquisition note 35 §8 says why this is parked and not planned)
- Publishing to crates.io → a release workflow in this repository. Trigger: v0's done-criteria passing, and one consumer outside Small Cat Labs the owner would show it to
