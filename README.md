# bothy

*The book the next session reads.*

A bothy is a stone shelter in the hills, left unlocked for whoever
comes next and kept by the people who use it. Inside is a book where
each traveler writes what they found — which paths are open, which are
blocked and until when — so the next person, arriving cold, can act on
it.

`bothy` is that book for a repository worked on by humans and coding
agents. It keeps the typed, numbered facts a project rules by —
decisions, parks (scope deferred with the trigger that reopens it),
principles, claims about the world and how well they are known,
findings, ledger rows — as structured text in git. It checks that every
id is unique, every citation resolves, and every ruling is still cited
by the code or test that holds it. It renders the markdown people read.
And it assembles the context an agent needs for one task, in one call,
measured in tokens.

```sh
bothy check                    # ids, fields, citations; exit 1 on any failure
bothy show B3                  # one ruling, what it names, what cites it
bothy list parks --area cli    # everything this area has deferred, with its trigger
bothy list decisions --uncited # rulings no code, test or document names yet
bothy pack --area check        # what a session reads before touching the checks
bothy render --check           # the markdown on disk matches the source (coming)
```

The examples are real: they run against this repository's own book.

The source is TOML anyone can edit with a text editor, one entry per
file. The index is built in memory every run and never written. The
markdown is a projection. Those three roles — intent, derivation,
rendering — are the design.

Status: `check`, `list`, `show` and `pack` work, against bothy's own
book under `bothy/`: twenty rulings and eleven parks, the first thirty
written by hand before the tool existed. The tool is its own first
consumer; its second is
[acquisition](https://github.com/gerwaric/acquisition), whose registry
the design brief was audited from. `import`, `render`, `new` and
`amend` are next. Everything the tool writes for a reader is written
for the smallest model in use. Start at `AGENTS.md`; the design is
`docs/design-brief.md`.

Leave it as you'd wish to find it.
