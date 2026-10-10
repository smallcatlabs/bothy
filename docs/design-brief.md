# Bothy — design brief

**Written 2026-10-09** by Claude (Fable 5.1) with the owner (Tom), from
a grammar audit of the acquisition registries at `bfa59416`. Named
`bothy` 2026-10-09 (`github.com/smallcatlabs/bothy`): the first Small Cat
Labs project; acquisition is its first consumer.

A bothy is a stone shelter in the hills, left unlocked for whoever comes
next and kept by the people who use it. Inside is a book where each
traveler writes what they found for those who follow. This tool is that
book for a repository: the one the next session reads.

This is the document the first build session starts from. In the
acquisition idiom: design precedes code, rulings land here, and the
done-criterion is stated before the first line of Rust. Nothing in it
is settled until the owner says so; the candidate rulings are numbered
so a session can accept, amend or refuse each.

---

## 1. Charter

`bothy` keeps the typed, numbered facts a project's humans and agents
rule by — decisions, parks, principles, claims, findings, ledger rows —
as structured text in git, checks their integrity, renders the markdown
people read, and assembles the context an agent needs for one task,
measured in tokens. It replaces a project's hand-written registry linter
and the habit of loading whole documents to find one row. It does not
orchestrate agents, run sessions, or hold opinions about process beyond
"every fact has one id, one home, and a check that it is still cited."

The source of truth is text the owner can edit; the index is derived;
the markdown is rendered. Those three roles map onto acquisition's own
layers — intent, derivation, rendering — on purpose.

## 2. What the audit found (the evidence this brief rests on)

Run 2026-10-09 over `spikes/rust-playground` at `bfa59416` and master's
`docs/`. The script is `tools/audit.py` in this repo; re-run it against
any registry to see whether it is importable.

| Kind | Parsed | Exceptions | What varies |
|---|---|---|---|
| Decisions (`C`) | 108 / 108 | 0 | 13 marker orders; 15 title terminators; 14 without *Why*; 41 without a date; 34 carry a quoted verdict; 0 over 800 B |
| Parks | 54 / 54 | 0 on `→`; 3 without `Trigger:` | one is "never" by design; 5 over 600 B, the largest 2,221 B because it carries measured data |
| Principles (`P`), invariants | 6, 5 | 0 | — |
| Claims (`N`, `T`) | 80 / 80 | 4 without a `[sources — confidence]` bracket | 28 phrasings of confidence ("Confirmed for this account's map and unique tabs"); 20 source tokens, `TOM` and `OWNER` both in use |
| Findings (`F`) | 78 / 78 | 0 | two shapes for one kind: a heading while open, a table row once resolved |
| Ledgers | 90 rows | 1 row with a stray pipe | two tables in one file |

Three facts shape the design:

1. **The structure is fixed; the vocabulary is loose.** Every exception
   is a missing optional field or a free-text phrasing of an enum.
   Migration is mechanical; normalization is a one-time commit.
2. **Fourteen id letters are in use** (`C F N T P S Q D R M V H J L`),
   of two kinds. *Durable* (`C P N T F`) outlive a slice and are cited
   from code. *Slice-local* (`S Q D R M V H J L`) live in one slice's
   documents — and are cited from durable ones (64 `S` citations inside
   `C` entries). The tool must know a namespace it does not own, so a
   citation resolves, without owning it.
3. **The index is already wrong once.** `C86` is in
   `decisions/daemon.md` and absent from `CONTEXT.md`'s index table;
   `docs-check.sh` cannot see it. A generated index makes the class of
   error impossible rather than checked.

## 3. Invariants (few, properties not mechanisms)

1. **The source is structured text in git, one entry per file, and the
   owner can edit it with nothing but a text editor.** No binary, no
   server, no lock.
2. **Every derived artifact is reproducible from the source and is never
   the source.** The SQLite index is rebuilt on every run and is
   gitignored. A rendered markdown file is a projection; editing it is
   a mistake the check reports.
3. **An id is never reused.** A deleted entry's file is gone and git
   holds it; the next id is max+1 over history, not over the present
   (the tool reads `git log` for the high-water mark).
4. **The tool never writes to a consumer's code.** It scans source for
   citations; it does not insert them.

## 4. The schema

TOML, one file per entry, named by id. Prose fields are multiline
strings and carry the owner's words verbatim. Cross-references are not
maintained by hand: the tool extracts every `<letter><number>` token
from prose fields and resolves them against known namespaces.

Why one file per entry rather than one array per area: two sessions
adding an entry touch two files (no merge conflict), or the same new
file (a loud conflict), never a silent duplicate inside one array. The
research-track skill already records the trap of parallel subagents
writing into one tree; the layout is the structural answer.

### 4.1 Decision — `bothy/decisions/C054.toml`

Worked from the real C54.

```toml
id = "C54"
area = "store"               # omitted or "cross-cutting" for CONTEXT.md
title = "Authority is the coarser observation's"
ruling = """
A listing owns membership, address, league and liveness; a fetch owns
contents and nothing more; a fetch never revives what a listing retired.
A location is its full coordinate; a parent tab's fetch is its
substashes' listing; a listing that revives a retired row clears its
`fetched_at`; membership is per response, never a timestamp match.
"""
why = "Five review rounds on 2026-09-02 found this shape in five places before it was stated once (REFRESH-SLICE.md)."
details = ["acquisition-store/src/lib.rs doc, C54"]
pinned = []                  # test files or test names
evidence = []                # claim ids ("N42") or ledger rows ("run:2026-09-08-r3")
decided = 2026-09-02         # TOML date; the import fills it from git blame when the prose has none
amended = []                 # [2026-09-09, 2026-09-17]
verdict = ""                 # the owner's words when quoted: "I agree to 1 entirely"
status = "ruled"             # ruled | superseded (by = "C110") — deleted entries are removed files
```

Optional fields are omitted, not emptied, in files the tool writes.
`title` and `ruling` are separate fields, which retires the
title-terminator question (15 variants today) and the marker-order
question (13 orders today) at once.

### 4.2 Park — `bothy/parks/K012.toml`

Parks have no id today. They need one so `fire`, `list` and citations
can name them. `K` is free; the letter is the owner's call.

```toml
id = "K12"
area = "store"
item = "User-scoped annotations home (`user.db`) + scope taxonomy"
home = "the store crate"
trigger = "the first user-scoped kind actually written (shop template, currency ratios, saved searches)"
why = ""                     # at most one clause, per the existing rule
parked = 2026-09-03
# trigger = "never" is legal and means what store.md:30 means by "opportunistic, or never"
# measured = """...""" holds the data-bearing tail the retention park carries today (2,221 B);
#   the import moves it here so `item`/`home`/`trigger` stay one line each
```

Firing a park is `bothy park fire K12 --by C110`: the file is
deleted and the next decision's `evidence` names the park's id, which
git resolves forever.

### 4.3 Claim — `bothy/claims/N002.toml` (master's second consumer)

Worked from the real N2.

```toml
id = "N2"
layer = "1"                  # the document's section, free text
statement = """
The one known Cloudflare block (July 2024, v0.11.1-alpha.11) was
triggered by over a thousand HEAD requests in one minute — redundant
per-tab probes, repeated ~20+ times per stash tab.
"""
sources = ["incident", "ggg-email"]   # a closed vocabulary per project; the import maps TOM→owner
confidence = "confirmed"              # confirmed | high | provisional | hypothesis
scope = ""                            # the qualifier: "for this account's map and unique tabs"
observed = 2024-07-01
body = """
GGG support relayed the report, verbatim (July 2024):

> "Just letting you know that someone has come to us ..."

The client was firing one HEAD per stash tab ...
"""
```

The 28 confidence phrasings in use today become four enum values plus a
`scope` string, with nothing lost: "Confirmed for 15 of 22 groups" is
`confidence = "confirmed"`, `scope = "15 of 22 groups"`.

### 4.4 Finding — `bothy/findings/F022.toml`

```toml
id = "F22"
title = "Dual persistence paths in BuyoutManager"
status = "confirmed"         # suspected | likely | confirmed | resolved | retired | superseded
disposition = "won't fix unless touched"
body = """..."""
resolved_in = ""             # "Phase 6", "PR #161", "items-pipeline M1"
```

One shape for both states retires the heading-vs-table-row split; the
resolved ledger becomes a projection (`list finding --status resolved`).

### 4.5 Ledger row — `bothy/runs/2026-08-22-r1.toml`

Append-only by construction (one file per run, never edited; "a later
fact is a new row"). Columns are the file's fields; the table is a
projection. The one 9-column row today is a stray pipe inside a cell,
which a field cannot produce.

### 4.6 Namespaces the tool knows but does not own

```toml
# bothy.toml, in the consumer
[namespace.S]
kind = "external"
scope = "slice:search"
defined_in = "spikes/rust-playground/search/DESIGN.md"
pattern = '^\*\*S(\d+)\.'      # how a definition looks in that document
```

An external namespace is parsed for *existence* only, so `C108`'s
citation of `S131` resolves, and `check` reports a citation of an
`S` id that no longer exists. When a slice closes and its document is
cut, the ids it defined are tombstoned in git and `check` says so.

### 4.7 The consumer's configuration — `bothy.toml`

```toml
root = "spikes/rust-playground"

[namespace.C]
kind = "decision"
dir = "bothy/decisions"
cite_in = ["crates/**/*.rs", "tools/*", "*.md", "decisions/*.md", ".claude/**/*.md"]
[namespace.C.budget]
cross_cutting = 15
entry_bytes = 800

[namespace.K]
kind = "park"
dir = "bothy/parks"

[namespace.P]
kind = "principle"
dir = "bothy/principles"

[namespace.N]
kind = "claim"
dir = "../../docs/ground-truth/network"     # master-side; cherry-picked, never authored here

[[projection]]
out = "CONTEXT.md"
template = "templates/context.md"
budget_tokens = 5000

[[projection]]
out = "decisions/{area}.md"
template = "templates/area.md"
for_each = "area"

[check]
extra = ["tools/docs-check-local.sh"]     # the project's own checks: dependency edges, README form, group reader
```

## 5. Verbs

CLI first; the MCP server exposes the read verbs and the two write
verbs over the same crate, so a session's first call can be one `pack`.

| Verb | What it does | Replaces today |
|---|---|---|
| `bothy check` | integrity: ids unique and monotone, every citation resolves, every entry cited or reported, budgets, projections current, extra checks run | `docs-check.sh` §1, §2, §6 |
| `bothy render [--check]` | write every projection; `--check` fails if any rendered file differs from disk | the hand-maintained index tables |
| `bothy show C54 [--json]` | the entry, what it cites, what cites it (code, tests, docs, help strings) | `grep -rn C54` |
| `bothy list decision --area store [--uncited] [--since 2026-09-01]` | filtered rows, one line each | reading `decisions/store.md` whole |
| `bothy cite C54` | every citing location with path:line | `docs-check`'s uncited report, inverted |
| `bothy new decision --area store` | allocates the next id over history, writes the template, validates | editing the bullet by hand and hoping the id is free |
| `bothy amend C54 [--verdict "..."]` | appends today's date, optional verdict | hand-editing "Amended …" |
| `bothy park new / list [--trigger GUI] / fire K12 --by C110` | the park lifecycle | the Parked sections |
| `bothy pack --area store [--slice search] [--budget 8000]` | invariants + cross-cutting + the area's decisions and parks + the open slice's checklist, as one document with its byte and token count | reading five files at session start |
| `bothy blame C90` | the commit that introduced the entry; fills a missing `decided` on import | — |
| `bothy import <path>` | the one-time migration from today's markdown; the audit script grown up | — |
| `bothy seat --pack ... --questions seat.toml` | the legibility seat: the questions, the pack, a transcript to score | — (note 35 §9) |
| `bothy stats` | the baseline table (entries by kind, bytes, tokens, uncited count, parks by area) | note 35 §2's shell block |

Token counts: v0 reports bytes and an approximation (a BPE tokenizer
offline; the Anthropic count-tokens endpoint when a key is present and
the owner says so). Budgets are stated in tokens on projections and
packs, not bytes on files; the byte budget per entry stays as the
existing rule.

## 6. What stays in the consumer

Of `docs-check.sh`'s seven checks, three are acquisition's own and stay
there, run by `check.extra`: §4 README form (one tour line per verb,
one row per knob), §5 dependency direction over `cargo metadata`, §7
the group-reader rule. §3 (backticked tokens must exist under `crates/`
or `tools/`) is generic enough to be a tool check with configurable
roots; v0 leaves it in the consumer and v1 decides.

The result: `docs-check.sh` shrinks from 502 lines to the three checks
no generic tool should know, and the script's six-review-round
dependency guard is untouched.

## 7. Done-criterion for v0

Stated before the code, in the consumer's idiom:

1. `bothy import` over `spikes/rust-playground` at `bfa59416`
   produces 108 decisions, 54 parks (53 area, 1 cross-cutting), 6
   principles and 5 invariants as
   files, and `bothy check` passes on them with the same findings
   `docs-check.sh` reports today (one uncited decision, C16; the notes
   nothing cites) **plus** the C86 index gap.
2. `bothy render --check` reproduces `CONTEXT.md` and the seven
   `decisions/*.md` byte for byte, after one normalization commit on
   the acquisition side whose diff is only whitespace, marker order
   and title terminators.
3. `bothy pack --area store` returns in one call what a session
   reads today across `AGENTS.md`, `CONTEXT.md` and
   `decisions/store.md`, with its token count.
4. Master's claims and findings import under a second `bothy.toml`
   with no code change — the P3 second-consumer test.

Nothing else is v0. The MCP server, `seat`, `blame` and `stats` are v1,
each earned by a session that wanted it.

## 8. Candidate rulings (numbered for the owner)

- **D1.** The source is TOML, one file per entry, named by zero-padded
  id (`C054.toml`), in a `bothy/` directory beside the documents it
  renders. *Alternative considered:* one array per area; rejected for
  parallel-edit conflicts and silent duplicate ids.
- **D2.** Parks get ids. Letter proposed: `K`.
- **D3.** Enumerations are closed per project and declared in
  `bothy.toml`: confidence (4 values), source tokens, finding status.
  A phrasing outside the enum is a `check` failure, and the qualifier
  goes in `scope`.
- **D4.** The index is SQLite, rebuilt per run, gitignored. *Why:* the
  queries `list`, `cite` and `pack` need joins over citations; grep
  over 500 files is slower and the queries are harder to get right.
- **D5.** Templates are files in the consumer (minijinja syntax), with
  built-in defaults that match acquisition's current form, so v0 passes
  the golden test without the consumer writing a template.
- **D6.** Slice-local namespaces are declared external and parsed for
  existence only (§4.6). The tool never owns a slice's working
  documents.
- **D7.** The tool lives in its own repo under the Small Cat Labs
  organization; acquisition depends on it as a dev tool (`cargo install
  --git` or a pinned binary), never as a workspace member. C1's graph
  is untouched.
- **D8.** Rust, with clap, serde, toml, rusqlite (bundled), minijinja,
  globset/ignore, regex; rmcp in v1. No async in v0; the MCP server
  brings tokio when it arrives.
- **D9.** Dates the prose lacks are filled by `blame` from git history
  at import, marked `decided_from = "git"` so a human can tell a recalled
  date from a measured one — the project's own rule about sizes stated
  from memory, applied to dates.

## 9. Parked (with triggers)

- A web or TUI view → trigger: the owner asking, twice.
- Token counting by the exact model tokenizer → trigger: a budget
  decision that the approximation got wrong by more than 10%.
- A generic "backtick tokens resolve" check (§6) → trigger: the second
  consumer wanting it.
- Agent Trace or checkpoint-style session capture → trigger: a review
  round whose question "why did the agent do that" the commit message
  could not answer. (Note 35 §8 says why this is parked and not planned.)
- Publishing → trigger: v0's done-criterion 4 passing, and one
  non-acquisition consumer the owner would show it to.
- Jujutsu-aware `blame` (change ids instead of commit hashes) → trigger:
  the jj trial in acquisition surviving its first slice.

## 10. The first build session

In Claude Code, on the owner's laptop, with acquisition's clone beside
the new repo. One deliverable: done-criterion 1 and 2, in that order.

1. `cargo new bothy`; the schema types from §4 as serde structs;
   `toml` round-trip tests on the four worked examples above.
2. `import`: port `tools/audit.py`'s parsers to Rust; write the files;
   run `check`.
3. `render` with the built-in templates; diff against the real files;
   iterate until the diff is the normalization commit and nothing else.
4. Commit the normalization on the acquisition side as its own PR,
   message carrying the diff's three kinds.
5. `pack` for one area, with byte and approximate token count.

What the owner can do before that session: rule on D1–D9 (a yes/no each
is enough); pick the park letter; create the empty repo under the
organization; decide whether master's claims move to a `bothy/` tree
on master or stay in their documents and import on demand (the brief
assumes the latter for v0).

## 11. Questions

1. D2's letter, and whether parks should carry a `why` at all (the
   current rule allows one clause; 25 of 54 carry an owner reference
   instead).
2. Should `ruling` stay the owner's verbatim words with the tool
   refusing to reflow them, or may `render` wrap at 72 columns? The
   golden test needs an answer before step 3.
3. Does master get its own `bothy.toml` in v0 (done-criterion 4), or
   is that v1? It is the cheapest test of generality available and I
   would keep it in v0.
4. ~~The name.~~ Resolved 2026-10-09: `bothy`, free on crates.io and at
   `smallcatlabs/bothy`; chosen by Claude from the tool's purpose (a
   marker left for whoever comes next) after cairn, trellis, docket,
   writ and seven others were found taken.

## Appendix — audit script

`tools/audit.py` (this repo; delivered as `bothy-audit.py`) is the parser the audit ran; it is the
first draft of `import`'s grammar and the regression test for it: the
Rust importer must produce the same counts and the same exception list
on `bfa59416`.
