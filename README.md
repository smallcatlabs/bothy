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
bothy pack --area store        # what a session reads before touching the store
bothy show C54                 # one ruling, what it cites, what cites it
bothy park list --trigger GUI  # everything waiting on the GUI slice
bothy park fire K12 --by C110  # a trigger fired; the park leaves, the ruling arrives
bothy check                    # ids, citations, budgets, projections current
bothy render --check           # the markdown on disk matches the source
```

The source is TOML the owner can edit with a text editor, one entry per
file. The index is SQLite, rebuilt every run, never committed. The
markdown is a projection. Those three roles — intent, derivation,
rendering — are the design.

Status: a design brief and a grammar audit of its first consumer,
[acquisition](https://github.com/smallcatlabs/acquisition). No code
yet. See `docs/design-brief.md`.

Leave it as you'd wish to find it.
