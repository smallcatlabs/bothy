#!/usr/bin/env python3
"""Throwaway grammar audit of the acquisition registries — the first draft of `bothy import`.

Usage: python3 -I grammar_audit.py <playground-dir> <master-docs-dir>

Parses decisions, parks, principles (playground), claims and findings
(master docs), ledger tables, and reports field coverage + exceptions.
Read-only. Prints a markdown report to stdout and a JSON dump to
audit.json beside this script.
"""
import json
import os
import re
import sys
from collections import Counter, defaultdict

PG, MD = sys.argv[1], sys.argv[2]
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "audit.json")

ID_RE = re.compile(r"\b([CNTFPSHMJDLVRQ])(\d+)\b")
DATE_RE = re.compile(r"(Decided|Ruled|Amended|confirmed live|Confirmed live|Validated|Measured)\**\s+(\d{4}-\d{2}-\d{2}(?:,\s*\d{4}-\d{2}-\d{2})*)")
MARKERS = ["Why", "Details", "Pinned", "Evidence"]
QUOTE_RE = re.compile(r'"([^"]{3,})"')
BACKTICK_RE = re.compile(r"`([^`]+)`")

report = []
dump = {}


def h(s):
    report.append("\n" + s + "\n")


def row(s):
    report.append(s)


# ---------- decisions ----------
dec_re = re.compile(r"^- \*\*C(\d+)\s+—\s+(.*?)\*\*\s*(.*)$")
decisions = []
dec_exceptions = []
files = [os.path.join(PG, "CONTEXT.md")] + sorted(
    os.path.join(PG, "decisions", f) for f in os.listdir(os.path.join(PG, "decisions"))
)
for path in files:
    rel = os.path.relpath(path, PG)
    section = None
    with open(path, encoding="utf-8") as fh:
        for ln, line in enumerate(fh, 1):
            line = line.rstrip("\n")
            if line.startswith("## "):
                section = line[3:]
            if line.startswith("- **C") and "—" not in line.split("**")[1][:4]:
                pass
            m = dec_re.match(line)
            if line.startswith("- **C") and not m:
                dec_exceptions.append((rel, ln, "bullet starts like a decision but did not match the grammar", line[:100]))
                continue
            if not m:
                continue
            cid, title, body = int(m.group(1)), m.group(2), m.group(3)
            entry = {
                "id": f"C{cid}", "file": rel, "line": ln, "section": section,
                "title": title, "title_terminator": title[-1] if title else "",
                "bytes": len(line.encode()),
                "markers": {}, "marker_order": [], "dates": {}, "quotes": [], "cites": [],
                "backticks": BACKTICK_RE.findall(body),
            }
            # markers
            positions = []
            for mk in MARKERS:
                for mm in re.finditer(r"\*" + mk + r":\*", body):
                    positions.append((mm.start(), mk))
            positions.sort()
            entry["marker_order"] = [mk for _, mk in positions]
            # slice marker text
            for i, (pos, mk) in enumerate(positions):
                end = positions[i + 1][0] if i + 1 < len(positions) else len(body)
                txt = body[pos + len(mk) + 3:end].strip()
                entry["markers"].setdefault(mk, []).append(txt)
            ruling_end = positions[0][0] if positions else len(body)
            entry["ruling_tail"] = body[:ruling_end].strip()
            for dm in DATE_RE.finditer(line):
                entry["dates"].setdefault(dm.group(1).lower(), []).append(dm.group(2))
            entry["quotes"] = QUOTE_RE.findall(body)
            cites = Counter(f"{a}{b}" for a, b in ID_RE.findall(body))
            cites.pop(f"C{cid}", None)
            entry["cites"] = sorted(cites)
            decisions.append(entry)

ids = Counter(d["id"] for d in decisions)
dups = [i for i, c in ids.items() if c > 1]
no_why = [d["id"] for d in decisions if "Why" not in d["markers"]]
no_date = [d["id"] for d in decisions if not d["dates"]]
over = [(d["id"], d["bytes"]) for d in decisions if d["bytes"] > 800]
term = Counter(d["title_terminator"] for d in decisions)
orders = Counter(" > ".join(d["marker_order"]) or "(none)" for d in decisions)
multi_marker = [d["id"] for d in decisions if any(len(v) > 1 for v in d["markers"].values())]
date_words = Counter(k for d in decisions for k in d["dates"])
with_quotes = [d["id"] for d in decisions if d["quotes"]]
backtick_in_title = [d["id"] for d in decisions if "`" in d["title"]]

# index table in CONTEXT vs present
ctx = open(os.path.join(PG, "CONTEXT.md"), encoding="utf-8").read()
index_ids = set()
for m in re.finditer(r"^\| `decisions/(\w+)\.md` \|.*?\| (.*?) \|$", ctx, re.M):
    for i in re.findall(r"C\d+", m.group(2)):
        index_ids.add((i, m.group(1)))
present = {(d["id"], os.path.basename(d["file"])[:-3]) for d in decisions if d["file"].startswith("decisions/")}
index_only = sorted(i for i in index_ids - present)
present_only = sorted(i for i in present - index_ids)

# citations from code
crate_text = ""
for root, _, fs in os.walk(os.path.join(PG, "crates")):
    if "/target" in root:
        continue
    for f in fs:
        if f.endswith(".rs"):
            crate_text += open(os.path.join(root, f), encoding="utf-8", errors="replace").read()
tools_text = ""
for f in os.listdir(os.path.join(PG, "tools")):
    p = os.path.join(PG, "tools", f)
    if os.path.isfile(p):
        tools_text += open(p, encoding="utf-8", errors="replace").read()
code_cites = Counter(re.findall(r"\bC(\d+)\b", crate_text))
cited_from_code = {f"C{n}" for n in code_cites}
uncited_code = sorted((d["id"] for d in decisions if d["id"] not in cited_from_code), key=lambda s: int(s[1:]))

h("## 1. Decisions")
row(f"- Entries parsed: **{len(decisions)}** across {len(files)} files; duplicate ids: {dups or 'none'}")
row(f"- Bullets that start like a decision but failed the grammar: **{len(dec_exceptions)}**")
for e in dec_exceptions:
    row(f"  - `{e[0]}:{e[1]}` — {e[2]}: `{e[3]}`")
row(f"- Title terminator (char before `**`): {dict(term)}")
row(f"- Backticks inside titles: {len(backtick_in_title)} ({', '.join(backtick_in_title[:8])}{'…' if len(backtick_in_title) > 8 else ''})")
row(f"- Marker presence: " + ", ".join(f"{mk} {sum(1 for d in decisions if mk in d['markers'])}" for mk in MARKERS))
row(f"- Entries with no *Why:* ({len(no_why)}): {', '.join(no_why)}")
row(f"- Entries with no date word ({len(no_date)}): {', '.join(no_date)}")
row(f"- Date words used: {dict(date_words)}")
row(f"- Entries with a marker repeated ({len(multi_marker)}): {', '.join(multi_marker)}")
row(f"- Entries over 800 bytes ({len(over)}): {over}")
row(f"- Entries carrying a quoted owner verdict: {len(with_quotes)}")
row(f"- Marker orders seen ({len(orders)} distinct):")
for o, c in orders.most_common():
    row(f"  - {c:3d}  {o}")
row(f"- CONTEXT index vs files: in index but not in file: {index_only or 'none'}; in file but not in index: {present_only or 'none'}")
row(f"- Decisions cited from `crates/` by id: {len(cited_from_code & set(ids))} of {len(ids)}; not cited from code ({len(uncited_code)}): {', '.join(uncited_code)}")
cite_targets = Counter(c[0] for d in decisions for c in d["cites"])
row(f"- Cross-references inside entries, by namespace letter: {dict(cite_targets)}")
dump["decisions"] = decisions

# ---------- parks ----------
parks = []
park_exc = []
park_files = files
for path in park_files:
    rel = os.path.relpath(path, PG)
    inpark = False
    with open(path, encoding="utf-8") as fh:
        for ln, line in enumerate(fh, 1):
            line = line.rstrip("\n")
            if line.startswith("## "):
                inpark = line.startswith("## Parked")
                continue
            if not inpark or not line.startswith("- "):
                continue
            body = line[2:]
            e = {"file": rel, "line": ln, "bytes": len(line.encode()), "text": body[:120]}
            if "→" in body:
                item, rest = body.split("→", 1)
                e["item"] = item.strip()
                if "Trigger:" in rest:
                    home, trig = rest.split("Trigger:", 1)
                    e["home"], e["trigger"] = home.strip(" .;"), trig.strip()
                else:
                    e["home"], e["trigger"] = rest.strip(), None
            else:
                e["item"], e["home"], e["trigger"] = None, None, None
            e["trigger_count"] = body.count("Trigger:")
            e["has_measured"] = bool(re.search(r"Measured \d{4}", body))
            e["has_owner_quote"] = bool(QUOTE_RE.search(body)) or "owner" in body.lower()
            e["has_date"] = bool(re.search(r"\d{4}-\d{2}-\d{2}", body))
            e["cites"] = sorted({f"{a}{b}" for a, b in ID_RE.findall(body)})
            parks.append(e)

h("## 2. Parks")
row(f"- Entries parsed: **{len(parks)}** across {len({p['file'] for p in parks})} files")
row(f"- With `→` (item → home): {sum(1 for p in parks if p['item'] is not None)}; without: {[(p['file'], p['line']) for p in parks if p['item'] is None]}")
row(f"- With `Trigger:`: {sum(1 for p in parks if p['trigger'])}; without ({sum(1 for p in parks if not p['trigger'])}):")
for p in parks:
    if not p["trigger"]:
        row(f"  - `{p['file']}:{p['line']}` {p['text'][:90]}…")
row(f"- More than one `Trigger:` in an entry: {[(p['file'], p['line']) for p in parks if p['trigger_count'] > 1]}")
row(f"- Carrying measured data (`Measured <date>`): {sum(1 for p in parks if p['has_measured'])}; carrying a date: {sum(1 for p in parks if p['has_date'])}; naming the owner or quoting: {sum(1 for p in parks if p['has_owner_quote'])}")
sizes = sorted(p["bytes"] for p in parks)
row(f"- Size: min {sizes[0]}, median {sizes[len(sizes)//2]}, max {sizes[-1]} bytes; over 600 bytes: {sum(1 for s in sizes if s > 600)}")
dump["parks"] = parks

# ---------- principles & invariants ----------
prin = re.findall(r"^- \*\*P(\d+)\.\*\* (.*)$", ctx, re.M)
inv = re.findall(r"^(\d+)\. \*\*(.*?)\*\*", ctx, re.M)
h("## 3. Principles and invariants (CONTEXT.md)")
row(f"- Principles parsed: {len(prin)} (P{prin[0][0]}–P{prin[-1][0]}); invariants parsed: {len(inv)}")
dump["principles"] = prin
dump["invariants"] = inv

# ---------- claims (master) ----------
claims = []
claim_exc = []
for fname, letter in (("network-ground-truth.md", "N"), ("trade-ground-truth.md", "T")):
    text = open(os.path.join(MD, "design", fname), encoding="utf-8").read()
    # join a claim head spanning lines: from **N1. up to the closing ** then [ ... ]
    for m in re.finditer(r"^\*\*(" + letter + r"\d+)\.\s+(.*?)\*\*\s*(\[(.*?)\])?", text, re.M | re.S):
        cid, stmt, bracket = m.group(1), " ".join(m.group(2).split()), m.group(4)
        c = {"id": cid, "file": fname, "statement": stmt[:140], "bracket": bracket}
        if bracket:
            b = " ".join(bracket.split())
            if " — " in b:
                src, conf = b.rsplit(" — ", 1)
            else:
                src, conf = b, None
            c["sources"], c["confidence"] = src, conf
        else:
            c["sources"], c["confidence"] = None, None
        # body until next claim or heading
        start = m.end()
        nxt = re.search(r"^(\*\*[NT]\d+\.|#)", text[start:], re.M)
        body = text[start:start + nxt.start()] if nxt else text[start:]
        c["body_bytes"] = len(body.encode())
        c["has_blockquote"] = ">" in body
        c["cites"] = sorted({f"{a}{b}" for a, b in ID_RE.findall(body)} - {cid})
        claims.append(c)
conf = Counter((c["confidence"] or "(none)").split(";")[0].split(",")[0].strip() for c in claims)
nobr = [c["id"] for c in claims if not c["bracket"]]
h("## 4. Ground-truth claims (master: `docs/design/*-ground-truth.md`)")
row(f"- Claims parsed: **{len(claims)}** ({Counter(c['id'][0] for c in claims)})")
row(f"- Without a `[sources — confidence]` bracket ({len(nobr)}): {', '.join(nobr) or 'none'}")
row(f"- Confidence words (first token): {dict(conf.most_common())}")
srcw = Counter(w.strip() for c in claims if c["sources"] for w in re.split(r"\+|,", c["sources"]))
row(f"- Source tokens ({len(srcw)} distinct), top: {srcw.most_common(12)}")
bb = sorted(c["body_bytes"] for c in claims)
row(f"- Body size: median {bb[len(bb)//2]}, max {bb[-1]} bytes; with a verbatim blockquote: {sum(1 for c in claims if c['has_blockquote'])}")
dump["claims"] = claims

# ---------- findings (master) ----------
ftext = open(os.path.join(MD, "cleanup", "findings.md"), encoding="utf-8").read()
open_f = re.findall(r"^### (F\d+)\. (.*?) — (.*)$", ftext, re.M)
resolved = re.findall(r"^\| (F\d+) \| (.*?) \| (.*?) \|$", ftext, re.M)
statuses = Counter(s.split(";")[0].split("(")[0].strip() for _, _, s in open_f)
h("## 5. Findings (master: `docs/cleanup/findings.md`)")
row(f"- Open headings `### F<n>. title — status`: **{len(open_f)}**; resolved-ledger table rows: **{len(resolved)}**")
row(f"- Open status words: {dict(statuses)}")
allf = {f for f, _, _ in open_f} | {f for f, _, _ in resolved}
nums = sorted(int(f[1:]) for f in allf)
missing = [n for n in range(1, nums[-1] + 1) if n not in nums]
row(f"- Ids seen F1–F{nums[-1]}; ids with neither an open heading nor a ledger row: {missing or 'none'}")
dump["findings_open"] = open_f
dump["findings_resolved"] = resolved

# ---------- ledgers ----------
def table_rows(path):
    rows = [l for l in open(path, encoding="utf-8") if l.startswith("|") and not l.startswith("| ---")]
    heads = [l for l in rows if re.match(r"^\| [A-Z]", l) and "---" not in l]
    widths = Counter(l.count("|") - 1 for l in rows)
    return rows, widths

h("## 6. Ledgers")
for rel in ("RUN-LEDGER.md", "search/LEDGER.md"):
    rows_, widths = table_rows(os.path.join(PG, rel))
    row(f"- `{rel}`: {len(rows_)} table rows (headers included); column counts seen: {dict(widths)}")

# ---------- id namespaces ----------
h("## 7. Id namespaces in use (letter + number tokens, across playground .md)")
alltext = ""
for root, _, fs in os.walk(PG):
    if "/target" in root or "/search/" in root + "/":
        continue
    for f in fs:
        if f.endswith(".md"):
            alltext += open(os.path.join(root, f), encoding="utf-8", errors="replace").read()
ns = Counter(a for a, _ in ID_RE.findall(alltext))
row(f"- Outside `search/`: {dict(ns.most_common())}")
stext = ""
for root, _, fs in os.walk(os.path.join(PG, "search")):
    for f in fs:
        if f.endswith(".md"):
            stext += open(os.path.join(root, f), encoding="utf-8", errors="replace").read()
ns2 = Counter(a for a, _ in ID_RE.findall(stext))
row(f"- Inside `search/`: {dict(ns2.most_common())}")

json.dump(dump, open(OUT, "w"), indent=1, default=str)
print("\n".join(report))
