# Reflection after the sittings — the method, measured; what to keep, what to retire, what to decide

**Written 2026-10-09** by Claude (Fable 5.1), in a cloud session on a
read-only clone at `bfa59416` (the tip, 2026-10-04). The owner asked for
a look at the state of the project with the playground as its center:
the history of this branch, its known future and the future beyond it,
and an opinion on whether to continue with hand-rolled processes or
trial something new. He asked for the agent's own view as a stakeholder.

Like 07 and 08, this is disposable history: nothing here is real until
it lands as a ruling, a park, a test, or a tool. It reads after 07, 08,
`CONTEXT.md`, and `AGENTS.md`. Unlike 07 and 08 it is written in plain
prose rather than the branch's compressed register, on purpose; section
6 says why.

It is written for a **future meta-analysis session** — one held after
the search slice closes, or at some later stop — so it carries the
measurements and the commands that produced them, so the next
reflection can diff against this one rather than start over.

---

## 1. How this was made, and what it could not see

Two read-only subagents ran in parallel over the clone: one traced the
process history (`git log --follow` on every control-plane file, sampled
sizes every ~100 commits, read `docs-check.sh`); the other assessed the
code (crate inventory, `cargo test --workspace`, test strategy as
practiced, the `search/` directory, debt markers, the run ledger, the gap
against the C++ `src/` on master). The session itself read: both
`AGENTS.md`s, `README.md`, `CONTEXT.md`, all seven `decisions/*.md`
Parked sections, notes 07 and 08, the five skills, ADR 0003, master's
`findings.md` headings, the master log, and the commit messages of
`1ff288ab`, `a4dec10a`, `8eeabcc9`, `e43c4a34`, `ee9cf984`, `71b3c525`,
`2f2f0f21`.

**Not seen, and the next reflection should get them:**

- **The GitHub issue and PR lists.** The API was blocked from the
  session and the owner's laptop was not reachable. Everything below
  called "known future" comes from the repo's own registers
  (`findings.md`, the Parked sections, `search/BUILD-PLAN.md`), not
  from GitHub.
- **Cost.** No token or dollar ledger exists in the repo. The branch is
  1,127 commits in eight weeks (~20 a day), about 85% co-authored by
  Fable; the rest are probably Codex (inferred from missing trailers,
  not stated anywhere). What that cost in plan quota is unknown to me.
- **The owner's own friction.** Note 07 §6 ruled that the owner-truth
  channel is conversation, recorded verbatim. Those verdicts are in the
  registry, but there is no record of how the owner *experienced* the
  sittings, the review rounds, or the reading cost — only the agents'
  record of them.
- **The collaborator's view.** Two of 1,127 commits are by a second
  author (`aurokin`, PRs #192 and #195 on the master side). Nothing on
  this branch records what a second human would need.
- **`cargo test` in a built tree.** 607 passed, 28 failed, 1 ignored;
  all 28 failures are the sandbox (26 need `cargo build --workspace`
  first for the `acqd` sibling; 2 are permission tests that cannot fail
  as root). The suite is believed green on the owner's machine.

---

## 2. The baseline, measured at `bfa59416`

Re-run this block at the next reflection from `spikes/rust-playground`
on the branch, and put the two columns side by side.

```sh
git rev-parse --short HEAD; git log -1 --format=%ad --date=short
git log --oneline master..HEAD | wc -l                     # commits since merge-base
git log --oneline master..HEAD -- . | wc -l                # of which touch the playground
find crates -name '*.rs' -not -path '*/target/*' -print0 | xargs -0 cat | wc -lc
find . -name '*.md' -not -path '*/target/*' -print0 | xargs -0 cat | wc -c
find search -name '*.md' -print0 | xargs -0 cat | wc -c
find brainstorming-notes -name '*.md' -print0 | xargs -0 cat | wc -c
cat AGENTS.md README.md CONTEXT.md | wc -c                 # always-loaded set
cat decisions/*.md | wc -c
grep -ohE '^\- \*\*C[0-9]+' CONTEXT.md decisions/*.md | wc -l
for f in decisions/*.md CONTEXT.md; do awk '/^## Parked/{p=1;next} p && /^- /' $f; done | wc -l
du -sh search | cut -f1
grep -rhoE '#\[(test|tokio::test)' crates --include=*.rs | wc -l
grep -cE '^\| 2026' RUN-LEDGER.md; grep -oE '^\| 2026-[0-9-]+' RUN-LEDGER.md | tail -1
wc -l < tools/docs-check.sh
git log --format='%s' --since=<last reflection date> -- . | grep -ciE 'review|look|audit'
git log --oneline --since=<date> -- crates | wc -l; git log --oneline --since=<date> -- search tools | wc -l
```

| Measure | 2026-10-04 (`bfa59416`) |
|---|---|
| Commits since merge-base with master (`68ce6677`) | 1,127 (862 touch the playground) |
| Rust, `crates/` | 97,768 lines / 3.89 MB |
| Markdown, whole playground | 1.49 MB |
| — of which `search/` | 763 KB |
| — of which `brainstorming-notes/` | 341 KB |
| Always-loaded set (AGENTS + README + CONTEXT) | 34.1 KB (budgets 8 + 15 + 20) |
| `decisions/*.md` | 84.4 KB, 108 decisions |
| Parked entries, all areas | 64 |
| `search/` on disk (committed `data/` extracts) | 31 MB |
| Python scripts / CSVs | 68 / 61 |
| `#[test]` functions | 636 |
| Live runs (`RUN-LEDGER.md`) | 44 rows; last 2026-09-12; zero 429s ever |
| `tools/docs-check.sh` | 502 lines |
| Search-build commits since 09-21 that are a review, look, or audit | 55 of 197 (28%) |
| Commits since 09-26: `crates/` vs `search/`+`tools/` | 24 vs 82 |
| Owner sittings recorded (hand-run trade-site searches) | ~780 (310 + 212 + 111 + 98 + 40 + 38 + 15 + 12 across the pseudo-stats and category tracks) |
| C++ on master: last code commit | 2026-09-04 (0.18.6); docs only since |

Trajectory, from the history subagent (blob sizes; CONTEXT and AGENTS
in lines):

| Date | Commit | CONTEXT | AGENTS | docs-check | playground .md | playground .rs |
|---|---|---|---|---|---|---|
| 08-22 | `5d4ba816` | 85 | — | — | 42 KB | 406 KB |
| 08-31 | `8363f8b8` | 247 | 51 | — | 294 KB | 661 KB |
| **09-02** | `d660d1f5` | **1,140** | ~60 | — | 416 KB | 1,258 KB |
| 09-02 | `12faa649` | 106 | — | 120 | 365 KB | 1,330 KB |
| 09-08 | `c5b7e314` | 81 | 102 | 178 | 547 KB | 1,926 KB |
| 09-14 | `377158df` | 81 | 126 | 455 | 681 KB | 2,435 KB |
| 09-19 | `bdfda2a8` | 82 | 120 | 461 | 1,298 KB | 2,564 KB |
| 10-04 | `bfa59416` | 82 | 125 | 502 | 1,450 KB | 3,847 KB |

The one-line reading: `CONTEXT.md` has been flat since the settling of
09-02/03, and total markdown tripled in the same period. The growth did
not stop; it moved down the ladder to on-demand files, which is what P5
says should happen — and it is still growth.

---

## 3. The history as waves

Each wave is a period where ceremony grew or was cut. The pattern
repeats, and seeing it whole is the point.

| Wave | Dates | What happened | Representative commits |
|---|---|---|---|
| **A. Maximal ceremony** | 08-09 → 08-15 | rate-limit-core: charter, independent briefs, four external review rounds, "charges", a clause registry, review rounds SD-R1…R8 that kept reopening; 85 commits on 08-15 including seven `[review mutation, will be reverted]` pairs. Ended at 821 KB of .md against 742 KB of .rs. | `4218efc5` "slice hardening rules from audit post-mortem"; `d5e00d0a` "make review closure a written repo rule"; `daa3bff9` "repeated re-close review … finds SD-R8-F24; round stays open" |
| **B. Restart** | 08-16 | The playground restarts as daemon + CLI; CONTEXT distilled the same day. | `73882a7c` "distill CONTEXT.md — properties over mechanism, current state over narrative" |
| **C. Regrowth** | 08-21 → 09-02 | N1–N6 ledger, the live ladder (rungs 1–10, soaks), tracers with review rounds 2–5 each, the store's five same-day rounds. CONTEXT reaches 1,140 lines / 110 KB. | `25d28b14` AGENTS/CLAUDE born; `8659cce0` "retire the ladder's hypothesis-first rule" (a counter-move) |
| **D. The great settling** | 09-02 → 09-03 | Notes 07–09; `docs-check.sh` born; the decision registry (C1–C63 at 800 B each); the split into `decisions/`; three skills. CONTEXT 110 KB → 13.6 KB in two days. | `74b3dddb`, `1288b836`, `56bec954`, `12faa649`, `0e4a4d9f`, `a43377a5` |
| **E. New ceremony, pruning alongside** | 09-03 → 09-12 | Pricing (owner-trimmed entries, outside review recorded). Daemon split: 27 review rounds in two days, each with a paired record commit (75 commits). P5 and P6 added. 17 notes pruned. | `2f2f0f21` P5/P6; `71b3c525` notes pruned; `cd2890e7` split record cut 87 → 27 KB |
| **F. Search research** | 09-13 → 09-21 | Ten tracks under committed briefs; a digest with per-track byte budgets; two blind proposals (Fable, Astra); audits, reviews, reconciliation, ruling packet. Search .md 258 → 634 KB in six days. | `64eda203` digest budgets; `1ff288ab`, `a4dec10a`, `8eeabcc9` session lessons; `3c4331f5` stage 5 closed, C89–C104 |
| **G. Search build and the budget fights** | 09-21 → 10-04 | Steps 1–9c4; step 9c4 got four looks. The slice record hit its cap repeatedly; text moved out by kind. The sittings: rounds i–l. Holes J1–J9 ruled 10-04. | `6bcfa15b` findings → checklist; `e43c4a34` "a closed step is a line"; `ee9cf984` rows → `search/LEDGER.md`; `894dd610` holes ruled |

The diagnosis the project wrote on 09-02 (note 09, cited in
`docs-check.sh`'s header) remains the correct one: **growth is
proportional to review rounds.** Its proposed fix — a round produces a
finding row, not a paragraph — was half-adopted: the finding became a
test (good), and the round still got a narrative commit (the half not
adopted). `e43c4a34` measured the consequence at the search record: "the
passes that held removed a kind of text by giving it a home … the pass
that trimmed instances was refilled by the next commit." `ee9cf984`
added: "a byte budget cannot tell growth by construction from
narrative."

The owner's visible interventions follow the same rhythm. Each
simplification wave has an owner ruling at its head (the 09-02 stop that
produced 07–09; "I agree with C" at `ee9cf984`; "A / yes, this session /
yes, rule only going forward" at `e43c4a34`). The regrowth waves do not;
they are the agents' default. **The process's resting state is
accretion; compression needs the owner.** That is the single most
useful fact in this note for a session that wants to change the
process rather than the code.

---

## 4. The scorecard, updated from note 07

| Property | 09-02 (note 07) | 10-04 |
|---|---|---|
| Safe to the GGG relationship, by structure | Done | Done; 44 runs, zero 429s, C88 real-by-default since 09-12 |
| Usable by scripts and agents | Done | Done; MCP still at the old substring search (step 11 wires the new one) |
| Multi-account, multi-realm | Done | Done |
| Never loses intent | Designed, never tested | Tested by pricing: buyouts with CAS and undo, imported from the C++ store, rendered to forum pages (C64–C81) |
| Finds items | Not done (substring) | A typed query language with counts, crosses, pseudo-stats, groups; steps 9d, 10, 11 open |
| Prices | Not done | Done offline, no daemon (`acq price`, `acq shop render`) |
| Publishes | Not done | Parked by owner ruling: "no shop publishing … much later or not at all" |
| Runs where players are (Windows, GUI) | Not done | **Not done.** Unix sockets only; C16 (Tauri) is cited by nothing; `dash` is the only visual surface |

Note 07's recommended order — settle, pricing, search, GUI — was
followed exactly. Three of four are done. The architecture has nothing
left to prove to itself; what remains is the product's three
player-facing gaps, and all three sit behind one unmade decision
(section 8).

---

## 5. What to keep — from the agent's seat

These are the parts of the method I would defend against any
replacement, because they make a fresh session able to act correctly
without rediscovery:

- **The decision registry.** A numbered ruling with its *Why*, its
  *Pinned* test, and the owner's verbatim words, cited from the code and
  the test that hold it. Nothing in the tool ecosystem (section 9) does
  this; Entire's checkpoints and the Agent Trace spec capture "why"
  after the fact from transcripts, this captures it as a rule before the
  code.
- **`docs-check.sh`'s dependency-edge guard.** The invariant that
  protects the GGG relationship (one sender, one gate) is a lint over
  `cargo metadata`, not a hope. This is P5 at its best.
- **The send journal as the contract oracle, and the rails.** Tests
  pin boundaries; the journal is the boundary.
- **Parks with triggers.** 64 of them; deferral never re-argued.
- **One authoritative home per fact, and the ladder.** The routing
  table in `AGENTS.md`.
- **Commit messages as the narrative home.** `git log` on a slice's
  range is a readable history, and that is rare.
- **The runner/reviewer split in the research-track skill.** It is the
  right shape for mixing model tiers (section 9).

---

## 6. What costs more than it returns

Each claim carries its lane (measured / inferred), its evidence, and a
test the next reflection can run. The claim-lanes convention is
`docs/redesign`'s (`7dbc4abd`).

### 6.1 Byte budgets optimize the wrong quantity — inferred

The budget rewards density. Density produced an idiolect: "a closed
step is a line," "the sitting," "round k," "J8's pair," "a number beside
two rows named together is refused." The branch's prose, including its
commit messages, is now compressed, allusive, and low-redundancy. An
agent can decode it; every fresh session pays a decoding tax, and a
second human would find it a wall. Note 07 §7 named this on 09-02
("density is a system property of this project's writing register").
The budget regime since has made it worse, because compression is what
the check rewards. The quantity that matters to a session is
tokens-to-correct-action, which bytes do not measure.

*Evidence:* `e43c4a34` and `ee9cf984` (budget met five commits running;
trims refilled); the register itself. *Test for the next reflection:*
section 10's legibility seat.

### 6.2 Review rounds are where ceremony multiplies — measured

27 rounds for the daemon split in two days; four looks at step 9c4;
seven commit/revert pairs on 08-15; 28% of search-build commits since
09-21 are a review, look, or audit. Each round is a commit with a
several-hundred-word story.

*Evidence:* the counts above. *Test:* the same grep at the next stop;
looks per step from `search/LEDGER.md`.

### 6.3 Owner attention is unmetered — measured count, inferred cost

The project meters GGG sends obsessively (journal, ledger,
`ACQ_MAX_SENDS`, a standing rule). It meters the owner's time not at
all. The pseudo-stats and category tracks consumed roughly 780
hand-run searches with a browser recorder across rounds i–l, to
establish things like beast colors (250 yellow, 79 red, 32 unknown).
This is rigorous under P2 and C79, and the oracle is a surface GGG does
not sanction for tooling, so a human had to sit. The question is
whether that fidelity was needed before a GUI exists or anyone but the
owner runs a search. The owner has said attention is his scarce
resource; the process does not know it.

*Evidence:* the sittings count; 82 commits to `search/`+`tools/` vs 24
to `crates/` since 09-26. *Test:* sittings per slice, in the ledger.

### 6.4 The reference-implementation fiction — measured

`README.md`: "whether this code ships is the owner's decision (ADR
0003) and nothing here anticipates it." `AGENTS.md`: "ADR 0003 … needs
nothing from you — ignore it." Meanwhile: Windows transport,
packaging, signing, the GUI, and legacy import are all parked on that
ADR; six frontends parks fire on "the GUI slice"; the C++ app has had no
code commit since 09-04; the Rust side has 636 tests to the C++ side's
Qt Test suite. Note 07 Q7 flagged the drift on 09-02. Five weeks later
the drift is larger. Agents being told to ignore the biggest open
question means the work the ADR is waiting for — a GUI judged by a
player — is nobody's job.

*Evidence:* the parks; the master log. *Test:* is ADR 0003 still
Proposed?

### 6.5 `search/` is a research directory committed as product — measured

31 MB on disk, 68 Python scripts, 61 CSVs, 763 KB of markdown, with
`data/` described as regenerable and committed anyway. Step 11 says
"shrink `search/`"; until it runs, git carries all of it forever (a
later `git rm` does not shrink history). This is the one place where the
hand-rolled process has no rung for "evidence that was load-bearing once
and is not now."

*Test:* `du -sh search` and `git count-objects -vH` at the next stop.

### 6.6 The process assumes one owner — inferred

Every procedure addresses "the owner" and "the session." There is an
`AGENTS.md` and no equivalent for a human collaborator. The entry cost
for a second human is the whole corpus in the idiolect. If the
collaboration the owner has described is meant to reach this branch,
that is a wall; if the Rust side is solo until the GUI, it is a
non-issue and should be said.

---

## 7. The future

### 7.1 Known, from the registers

- **Search:** 9d (percentile, category), 10 (`--explain`, `--next`,
  `--request`, `--membership all`), 11 (the MCP tool, retire `acq items
  search`, shrink `search/`, move `DESIGN.md` into module docs). 9d's
  holes were ruled on 10-04 (`894dd610`).
- **Master findings open:** F74 (POESESSID not host-scoped), F75
  (user-agent format), F77 (no OAuth de-arming); F22, F54, F63, F66
  contained or deferred by decision. F74/F75/F77 are C++-side and
  partly moot if the Rust side is the path; F75 is worth a glance on the
  Rust side (invariant 4 keeps the string continuous, which may mean it
  inherits the same format gap).
- **README known gaps:** two endpoints on declared route knowledge
  (N38, N39, until GGG corrects the HEAD defect); mock quantization;
  `policy set` replaces the whole policy (per-realm merge parked, trigger
  "a second realm in daily use — PoE2 1.0 ships December 2026"); Unix
  only.
- **Parked on the GUI slice** (`decisions/frontends.md`): queue UI,
  results over a subtree, a watch that survives the daemon, the
  structured-error taxonomy on the wire, sticky default realm/league.
  **Parked on ADR 0003** (`decisions/daemon.md`): packaging, signed
  binaries, installer, CLI symlink.
- **C16 (Tauri) is cited by nothing.** `docs-check` already reports it.

### 7.2 Beyond the registers

- **PoE2 1.0 in December 2026** is the one external date. It fires the
  per-realm merge trigger and will stress the search's realm handling
  and the currency table's realm tags.
- **The GUI is where the Rust side will finally be judged by a player
  against the C++ app.** Both 07 and 08 said GUI after search. The C++
  app wins today on the three things a player touches daily; two of
  them (find, price) are now done in Rust with no surface a player would
  use.
- **Legacy import (C73) is parked indefinitely.** "Migration of existing
  users' data" is an ADR 0003 driver; the pricing import already proved
  the 0.18 userstore is GGG-id keyed. Whether the rest of a user's state
  (saved searches, settings) needs to carry over is an unasked question.
- **The collaborator.** Section 6.6.
- **Cost.** Section 1. A Max plan is a fixed envelope; this branch's
  cadence (~20 commits a day, many of them long sessions) is a draw on
  it that nothing records.

---

## 8. Systems: hand-rolled, or something new

A survey of the agent-native tooling that exists as of October 2026
(Entire's checkpoints and Git network, Cursor Origin, the Agent Trace
spec, Jujutsu, GitButler, Beads/Gas Town, worktrees) was done in the
same conversation. Against this project:

| Tool | What it would replace here | Verdict |
|---|---|---|
| Beads (issues as JSONL in the repo) | parks-with-triggers, `findings.md` | No. Parks have a structure (item → home → trigger) Beads does not model; the implementation is questioned; migration cost for a worse fit. |
| Entire checkpoints (agent sessions stored beside commits) | commit messages + verbatim owner verdicts | No. The project ruled transcripts are history, not authority; the distilled narrative in commit messages is better for its purposes; git already carries 31 MB it should not. |
| Entire's Git network, Cursor Origin | GitHub | No. Fleet-scale throughput is not this project's problem. |
| Agent Trace (line-level AI attribution) | `Co-Authored-By` trailers | No. Commit-granularity attribution is already there. |
| Git worktrees (Claude Code `--worktree`) | the research-track skill's shared tree | **Yes, trial.** Fixes a recorded trap verbatim: "`git add search` while another subagent is still writing sweeps its half-written files into your commit" (research-track skill, 2026-09-13). One line in the skill. |
| Jujutsu, colocated | the review-round commit pattern | **Maybe, trial.** `jj squash` and `jj undo` are built for "four looks at one step": the finding stays, the churn collapses. Low risk on an existing git repo; the real risk is an agent mixing `git` and `jj` in one workspace, which needs its own ruling before the trial. |

**The judgment:** the hand-rolled process is better fitted to this
project than anything on the market, because its structure encodes the
project's own invariants — the GGG relationship as a lint, the four
layers as dependency edges. Nothing external knows those. Do not
replace it. The changes worth making are to the process, not the
tooling, and they are in section 10.

What a trial would measure, so it can be judged under P6:

- Worktrees: zero sweep incidents across N parallel tracks; the
  reviewer's merge cost per track (a number, in minutes or in commits).
- jj: looks per step and bytes of narrative per step, before and after,
  on two steps of comparable size; and any incident of mixed `git`/`jj`
  state.

---

## 9. Candidate decision lines and parks

In the shape note 07 used, so a session can promote or discard each.

**Candidate rulings:**

- **The legibility seat.** The validating consumer of the always-loaded
  set is a fresh session. Measure it: a session with only `AGENTS.md`,
  `README.md`, `CONTEXT.md` and the area file, asked ten questions whose
  answers the registry holds, scored. The byte budgets stay as a floor;
  the seat is the ceiling. (P2 applied to the control plane; P4 says a
  property with real stakes gets a recorded test.)
- **Two looks per step.** A third look is a new step or a park. The
  finding goes in the test, the round in a one-line ledger row, the
  commit message says what changed and not the round's story.
- **Owner sittings are metered like sends.** A count per slice in the
  ledger and a budget stated in the brief, so a brief that would spend
  200 of the owner's searches says so before it runs.
- **Runners on a cheaper tier.** The research-track skill's runner reads
  fixed inputs under a committed brief; that is the right place for a
  non-frontier model. The reviewer and the design sessions stay on the
  frontier tier. Trial on one track and measure restores-by-reviewer.
- **Plain register for always-loaded text.** The three always-loaded
  files are written for a reader who has never seen the project; the
  idiolect is allowed in on-demand records and commit messages. (This
  is the one that fights the byte budget directly; the seat decides
  which wins.)

**Candidate parks:**

- Jujutsu trial → after the search slice closes, on the first step of
  the next slice, under a ruling on `git`/`jj` mixing. Trigger: the
  next slice's first review round.
- A `HUMANS.md` (or a plain-prose orientation for a second human) →
  trigger: the collaborator's first commit on this branch, or the owner
  saying the branch stays solo.
- `search/` history rewrite (filter the committed `data/` out of git)
  → trigger: step 11's shrink, and only if the owner wants the history
  smaller; otherwise accept the 31 MB and say so.

**One chosen slice:** none from this note. Search is open at 9d of 11
and should close first (section 10, Q2). The next slice is the owner's
call and is downstream of ADR 0003.

---

## 10. Questions for the owner

1. **ADR 0003.** Rewrite it to say what is true: the Rust implementation
   is the path, master is maintenance-only, the GUI slice is the
   acceptance test? This un-parks six items at once and ends the
   "reference implementation" language. From the agent's seat the
   evidence is in; the decision is yours.
2. **Close search at a line, not a completion?** Ship 9d with what the
   sittings established, rule the rest as listed gaps (the project has
   that idiom: "a listed gap, never counted as reach"), do 10 minimally,
   do 11 in full because 11 is the step that deletes most of `search/`.
3. **Which of the candidate rulings in section 9, if any?** The
   legibility seat is the one I would take first, because it decides the
   others (whether plain prose beats the byte budget, whether two looks
   suffice).
4. **Is the branch solo until the GUI?** If yes, say so and 6.6 is
   closed. If no, the orientation for a human is a real cost before the
   collaborator arrives.
5. **Cost.** Is there a number you track, and should the ledger carry
   it? A reflection that cannot see cost cannot say whether the method
   is affordable, only whether it is coherent.

---

## 11. For the next reflection

Hold it on a trigger, not a date (P1): the search slice's close, the
ADR's rewrite, or the first GUI commit, whichever comes first — or
whenever the owner feels the process is heavier than the work, which is
the signal that preceded both 09-02 and this note.

**Re-measure** section 2 and put the columns side by side. The numbers
that matter most:

- always-loaded bytes (should be flat), total .md (is it still
  tripling?), `search/` on disk (did step 11 shrink it?);
- review/look/audit commits as a share of the slice's commits (28% is
  the baseline);
- sittings per slice (780 is the baseline; zero would be fine);
- commits to `crates/` vs to `search/`+`tools/` (24 vs 82 is the
  baseline; the sign should flip during a build);
- `#[test]` count against Rust lines (636 / 97.8k);
- ADR 0003's status line.

**Re-ask:**

- Did any wave H happen — a new kind of ceremony that grew between this
  note and the next? Name it and the owner ruling (if any) that cut it.
  If no owner ruling cut it, section 3's claim stands: the resting state
  is accretion.
- Did any candidate from section 9 get ruled? If the legibility seat
  ran, what did it score, and what did it change?
- Did the worktree or jj trial run, and what did it measure (section 8)?
- Did the GUI start, and did it fire the six frontends parks as
  predicted? Were there GUI needs none of the parks anticipated? (Note
  08 predicted pagination, event replay, subscriptions, partial-failure
  presentation.)
- Did PoE2 1.0 fire the per-realm trigger, and what broke?
- What could this note not see (section 1) that the next one can?

**Method notes for whoever runs it:** two parallel read-only subagents
(history, code) cost about an hour of wall time and gave the baseline;
the session should read 07, 08, this note, the Parked sections, and the
commit messages of whatever `docs-check.sh`'s header cites at the time.
Fetch the GitHub issues first, from a machine that can. Do not read
`search/` whole; `grep -n '^##'` then by range, as the research-track
skill says.

---

## 12. Where this note may be wrong

- The idiolect critique (6.1) is an agent's experience, not a
  measurement. The owner reads the register daily and may find it
  fast. The seat is the test; until it runs, 6.1 is a hypothesis.
- The sittings critique (6.3) may undervalue what the category and
  pseudo-stats tracks bought. If a player searching "yellow beasts" is a
  real use, the fidelity was the point. I could not see a player.
- "The resting state is accretion" (section 3) is read from eight weeks
  of one branch. A longer history, or a different owner cadence, could
  show otherwise.
- The tooling verdicts (section 8) are from a survey, not from running
  any of the tools in this repo. A trial could surprise in either
  direction.
- The infrastructure phase ended on 09-02, as 07 and 08 said. What
  this note cannot judge is whether a process tuned for proving — which
  is what this one is, and it has proved a great deal — can be tuned
  for shipping without losing what made the proofs hold. That is the
  question the GUI slice will answer, and it is the one I would most
  want the next reflection to report on.
