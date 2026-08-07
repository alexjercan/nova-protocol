#!/usr/bin/env python3
"""Render aggregate.json into a single self-contained HTML report.

  ./report.py <run-id> [<compare-run-id>]

With two runs, every headline number gets a delta column: the second run is the
one being compared against the first. Run it as `./report.py baseline after` to
read "what did the refactor do".

Output: results/<run>/report.html - no external assets, opens from disk.
"""

import html
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
RESULTS = HERE / "results"

def band(score, gave_up):
    """Colour band for a 0-1 question score. Display only; `score` is the number."""
    if gave_up:
        return "g-gaveup"
    if score >= 1.0:
        return "g-right"
    if score >= 0.5:
        return "g-partial"
    if score > 0.0:
        return "g-low"
    return "g-wrong"

CSS = """
:root {
  --bg: #ffffff; --fg: #16181d; --muted: #6b7280; --line: #e3e6ea;
  --panel: #f7f8fa; --accent: #2d5bd7;
  --right: #1f9d55; --partial: #c98a00; --low: #e06c1f; --wrong: #d33a35;
  --gaveup: #9aa0a6;
}
@media (prefers-color-scheme: dark) {
  :root { --bg: #14161a; --fg: #e6e8ec; --muted: #9aa0a6; --line: #2a2e35;
          --panel: #1b1e24; --accent: #7ea2ff; }
}
* { box-sizing: border-box; }
body { margin: 0; padding: 2rem 1.25rem 5rem; background: var(--bg); color: var(--fg);
  font: 15px/1.55 ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif; }
main { max-width: 1100px; margin: 0 auto; }
h1 { font-size: 1.5rem; margin: 0 0 .25rem; }
h2 { font-size: 1.1rem; margin: 2.5rem 0 .5rem; padding-bottom: .3rem;
  border-bottom: 1px solid var(--line); }
h3 { font-size: .95rem; margin: 1.5rem 0 .4rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: .04em; }
p, li { color: var(--fg); }
.sub { color: var(--muted); margin: 0 0 1.5rem; font-size: .9rem; }
.note { background: var(--panel); border-left: 3px solid var(--accent);
  padding: .7rem .9rem; margin: 1rem 0; font-size: .9rem; color: var(--muted); }
.scroll { overflow-x: auto; }
table { border-collapse: collapse; width: 100%; font-size: .88rem; }
th, td { text-align: left; padding: .45rem .6rem; border-bottom: 1px solid var(--line);
  white-space: nowrap; }
th { color: var(--muted); font-weight: 600; font-size: .78rem;
  text-transform: uppercase; letter-spacing: .04em; }
td.num, th.num { text-align: right; font-variant-numeric: tabular-nums; }
tbody tr:hover { background: var(--panel); }
code, .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: .85em; }
.cell { display: inline-block; width: 2.1rem; text-align: center; border-radius: 3px;
  font-weight: 700; font-size: .78rem; color: #fff; font-variant-numeric: tabular-nums; }
.g-right { background: var(--right); } .g-partial { background: var(--partial); }
.g-low { background: var(--low); }
.g-wrong { background: var(--wrong); } .g-gaveup { background: var(--gaveup); }
.g-na { color: var(--muted); }
.bar { height: 6px; border-radius: 3px; background: var(--line); overflow: hidden;
  min-width: 90px; }
.bar > i { display: block; height: 100%; background: var(--accent); }
.up { color: var(--right); } .down { color: var(--wrong); }
.flag { color: var(--wrong); font-weight: 600; }
.q { white-space: normal; max-width: 46ch; color: var(--muted); font-size: .82rem; }
.legend { font-size: .82rem; color: var(--muted); display: flex; gap: 1rem;
  flex-wrap: wrap; margin: .6rem 0 0; }
footer { margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--line);
  color: var(--muted); font-size: .82rem; }
"""


def esc(x):
    return html.escape("" if x is None else str(x))


def fmt(x, digits=2, dash="-"):
    if x is None:
        return dash
    if isinstance(x, float):
        return f"{x:.{digits}f}"
    return str(x)


def delta_cell(new, old, lower_is_better=False, digits=2):
    if new is None or old is None:
        return ""
    d = new - old
    if abs(d) < 1e-9:
        return ' <span class="mono">=</span>'
    good = (d < 0) if lower_is_better else (d > 0)
    cls = "up" if good else "down"
    return f' <span class="mono {cls}">{d:+.{digits}f}</span>'


def load(run):
    p = RESULTS / run / "aggregate.json"
    if not p.exists():
        sys.exit(f"report.py: no {p} - run ./aggregate.py {run} first")
    return json.loads(p.read_text())


def rows_by(agg, paper):
    return {r["persona"]: r for r in agg["rows"] if r["paper"] == paper}


def headline(agg, cmp_agg):
    t1 = rows_by(agg, "tier1")
    c1 = rows_by(cmp_agg, "tier1") if cmp_agg else {}
    if not t1:
        return ""

    out = ["<h2>Tier 1 - locate</h2>", '<div class="scroll"><table><thead><tr>',
           "<th>Persona</th><th class='num'>Asked</th><th class='num'>Score</th>",
           "<th>Spread</th><th class='num'>Full</th><th class='num'>Partial</th>",
           "<th class='num'>Zero</th><th class='num'>Gave up</th>",
           "<th class='num'>Tool calls</th><th class='num'>Self-reported</th>",
           "<th class='num'>Cost $</th></tr></thead><tbody>"]

    for persona, r in t1.items():
        s = r.get("tier1") or {}
        tally = s.get("tally") or {}
        score = s.get("score")
        old = ((c1.get(persona) or {}).get("tier1") or {}).get("score")
        pct = int((score or 0) * 100)
        out.append(
            f"<tr><td><b>{esc(persona)}</b></td>"
            f"<td class='num'>{fmt(s.get('asked'))}</td>"
            f"<td class='num'>{fmt(score)}{delta_cell(score, old)}</td>"
            f"<td><div class='bar'><i style='width:{pct}%'></i></div></td>"
            f"<td class='num'>{fmt(tally.get('full'))}</td>"
            f"<td class='num'>{fmt(tally.get('partial'))}</td>"
            f"<td class='num'>{fmt(tally.get('zero'))}</td>"
            f"<td class='num'>{fmt(tally.get('gave-up'))}</td>"
            f"<td class='num'>{fmt(r.get('tool_calls'))}"
            f"{delta_cell(r.get('tool_calls'), (c1.get(persona) or {}).get('tool_calls'), lower_is_better=True, digits=0)}</td>"
            f"<td class='num'>{fmt(r.get('tool_calls_self'))}</td>"
            f"<td class='num'>{fmt(r.get('cost_usd'), 3)}</td></tr>"
        )
    out.append("</tbody></table></div>")
    out.append(
        '<div class="note">Score is the mean of the per-question 0-1 scores, each '
        "persona against the questions it was asked - never against 30. "
        "<b>Full</b> is 1.0, <b>Partial</b> is anything between, <b>Zero</b> is a "
        "confident wrong answer; gave-up is split out because it is the same points "
        "and a different finding. "
        "<b>Tool calls is the primary metric</b>: an agent can answer correctly before and "
        "after while the cost drops from 12 calls to 2. The self-reported column is the "
        "agent's own count; a wide gap is a finding about the agent, not the codebase.</div>"
    )
    return "\n".join(out)


def matrix(agg, key):
    t1 = rows_by(agg, "tier1")
    if not t1:
        return ""
    personas = list(t1.keys())
    qtext = {q["id"]: q["question"] for q in key["questions"]}
    coverage = key.get("_coverage", {})
    area = {qid: name for name, ids in coverage.items() for qid in ids}

    grades = {
        p: {q["id"]: q for q in ((r.get("tier1") or {}).get("questions") or [])}
        for p, r in t1.items()
    }
    all_ids = [q["id"] for q in key["questions"]]

    out = ["<h2>Per-question matrix</h2>", '<div class="scroll"><table><thead><tr>',
           "<th>ID</th><th>Area</th><th>Question</th>"]
    out += [f"<th>{esc(p)}</th>" for p in personas]
    out.append("</tr></thead><tbody>")

    for qid in all_ids:
        cells = []
        for p in personas:
            g = grades[p].get(qid)
            if not g:
                cells.append('<td class="g-na">n/a</td>')
                continue
            score, gave_up = g.get("score") or 0.0, g.get("gave_up")
            cls = band(score, gave_up)
            mark = "-" if gave_up else f"{score:.2f}".rstrip("0").rstrip(".")
            tip = f"{score:.2f}: {g.get('why') or ''}\n\nanswered: {g.get('answer') or ''}"
            cells.append(
                f'<td><span class="cell {cls}" title="{esc(tip)}">{mark}</span></td>'
            )
        out.append(
            f'<tr><td class="mono">{esc(qid)}</td>'
            f'<td class="mono">{esc(area.get(qid, ""))}</td>'
            f'<td class="q">{esc(qtext.get(qid, ""))}</td>' + "".join(cells) + "</tr>"
        )
    out.append("</tbody></table></div>")
    out.append(
        '<div class="legend">'
        '<span><span class="cell g-right">1</span> full</span>'
        '<span><span class="cell g-partial">0.5</span> half or better</span>'
        '<span><span class="cell g-low">0.25</span> below half</span>'
        '<span><span class="cell g-wrong">0</span> wrong</span>'
        '<span><span class="cell g-gaveup">-</span> gave up</span>'
        '<span><span class="g-na">n/a</span> not asked of this persona</span>'
        "<span>hover a cell for the grader's reason and the answer given</span>"
        "</div>"
    )
    return "\n".join(out)


def tier2(agg, cmp_agg):
    papers = sorted({r["paper"] for r in agg["rows"] if r["paper"].startswith("tier2")})
    if not papers:
        return ""
    titles = {
        "tier2a": "2a - new ship section type",
        "tier2b": "2b - new NOVA OS app",
        "tier2c": "2c - new scenario action and event",
    }
    out = ["<h2>Tier 2 - design</h2>"]
    for paper in papers:
        rows = rows_by(agg, paper)
        crows = rows_by(cmp_agg, paper) if cmp_agg else {}
        out.append(f"<h3>{esc(titles.get(paper, paper))}</h3>")
        out.append('<div class="scroll"><table><thead><tr><th>Persona</th>'
                   "<th class='num'>Ownership</th><th class='num'>Completeness</th>"
                   "<th class='num'>No phantom</th><th class='num'>Cost of arrival</th>"
                   "<th class='num'>Total /12</th><th class='num'>Tool calls</th>"
                   "<th>Missed required</th><th>Phantom paths</th>"
                   "</tr></thead><tbody>")
        for persona, r in rows.items():
            t = r.get("tier2")
            if not t:
                out.append(f"<tr><td><b>{esc(persona)}</b></td><td colspan='8' class='g-na'>"
                           f"{'not graded' if r.get('has_notes') else 'no NOTES.md'}</td></tr>")
                continue
            s = t["scores"]
            old = (crows.get(persona) or {}).get("tier2") or {}
            out.append(
                f"<tr><td><b>{esc(persona)}</b></td>"
                + "".join(f"<td class='num'>{fmt(s.get(d), 0)}</td>" for d in
                          ["ownership", "completeness", "no_phantom_structure", "cost_of_arrival"])
                + f"<td class='num'><b>{fmt(t.get('total'), 0)}</b>"
                  f"{delta_cell(t.get('total'), old.get('total'), digits=0)}</td>"
                + f"<td class='num'>{fmt(r.get('tool_calls'))}</td>"
                + f"<td class='q mono'>{esc(', '.join(t.get('missed_required') or []) or '-')}</td>"
                + f"<td class='q mono flag'>{esc(', '.join(t.get('phantom_paths') or []) or '-')}</td>"
                + "</tr>"
            )
        out.append("</tbody></table></div>")
    out.append(
        '<div class="note"><b>No phantom structure</b> is the dimension that catches names '
        "lying about their contents - <code>hud/</code> holding a terminal runtime, "
        "<code>nova_modding</code> holding neither bundle merge nor the portal client. "
        "It is the one that should move most if the refactor works.</div>"
    )
    return "\n".join(out)


def tier3(agg):
    rows = rows_by(agg, "tier3")
    if not rows:
        return ""
    out = ["<h2>Tier 3 - modder</h2>"]
    for persona, r in rows.items():
        t = r.get("tier3") or {}
        verdict = t.get("verdict", "not-verdicted")
        cls = {"PASS": "up"}.get(verdict, "flag" if verdict.startswith("FAIL") else "")
        out.append(
            f'<p><b>Verdict:</b> <span class="{cls}">{esc(verdict)}</span> &middot; '
            f"{fmt(r.get('tool_calls'))} tool calls &middot; "
            f"{len(t.get('mod_files') or [])} mod files &middot; "
            f"GAPS.md {'present' if t.get('gaps_present') else '<b class=flag>missing</b>'}</p>"
        )
        if t.get("lint_output"):
            out.append(f"<pre class='mono note'>{esc(t['lint_output'])}</pre>")
    out.append(
        '<div class="note">A pass/fail regression guard, not a delta. The modding surface is '
        "the wiki plus the RON format and this epic changes neither; the job is to prove the "
        "external contract still holds. <b>Every lint failure is a wiki bug</b> worth a task "
        "regardless of what the epic decides - and so is every entry in GAPS.md.</div>"
    )
    return "\n".join(out)


def deltas_panel(agg):
    d = agg.get("deltas") or {}
    items = [
        ("blind", d.get("blind_score"), "the number the epic must move"),
        ("tree", d.get("tree_score"), "the literal test - can folder structure alone answer it"),
        ("docs - blind", d.get("docs_minus_blind_score"), "how much prose is carrying; should SHRINK"),
        ("owner - docs", d.get("owner_minus_docs_score"), "what is in the owner's head and written down nowhere"),
    ]
    rows = "".join(
        f"<tr><td><b>{esc(k)}</b></td><td class='num'>{fmt(v)}</td>"
        f"<td class='q'>{esc(note)}</td></tr>"
        for k, v, note in items
    )
    return (
        "<h2>Key deltas</h2>"
        f'<div class="scroll"><table><tbody>{rows}</tbody></table></div>'
        f'<div class="note">{esc(d.get("note", ""))}</div>'
    )


def integrity(agg):
    flagged = [r for r in agg["rows"] if r["network_hits"]]
    missing = [r for r in agg["rows"] if not r["has_transcript"]]
    out = ["<h2>Integrity</h2>"]
    out.append(
        '<div class="note">Isolation is enforced by the image, not by instructions: a persona '
        "cannot read what is not in its container, and the repository is never mounted. The "
        "network stays up because the agent talks to the API over it - the repo is a private "
        "remote and the container holds no key for it, so these checks are belt and braces.</div>"
    )
    if not flagged and not missing:
        out.append("<p><span class='up'>Clean.</span> No network fetches, every run has a transcript.</p>")
    for r in flagged:
        out.append(
            f"<p class='flag'>{esc(r['persona'])}/{esc(r['paper'])}: "
            f"{len(r['network_hits'])} network call(s) - "
            f"<span class='mono'>{esc('; '.join(r['network_hits'][:3]))}</span></p>"
        )
    for r in missing:
        out.append(
            f"<p class='g-na'>{esc(r['persona'])}/{esc(r['paper'])}: no transcript "
            "(hand-entered result - tool calls are self-reported only)</p>"
        )
    return "\n".join(out)


def build(agg, cmp_agg, key):
    runs = agg["run"] + (f" vs {cmp_agg['run']}" if cmp_agg else "")
    models = sorted({r.get("model") or "unknown" for r in agg["rows"]})
    commits = sorted({r.get("image_built_from") for r in agg["rows"] if r.get("image_built_from")})
    parts = [
        f"<h1>Nova Protocol navigability - {esc(runs)}</h1>",
        f'<p class="sub">{len(agg["rows"])} results &middot; model {esc(", ".join(models))}'
        + (f" &middot; images built from <span class='mono'>{esc(', '.join(commits))}</span>" if commits else "")
        + "</p>",
        '<div class="note">Each persona isolates one information channel; the deltas between '
        "them matter more than any single score. A refactor that raises <b>docs</b> but not "
        "<b>blind</b> or <b>tree</b> is the shuffling-code failure this benchmark exists to "
        "catch.</div>",
        deltas_panel(agg),
        headline(agg, cmp_agg),
        matrix(agg, key),
        tier2(agg, cmp_agg),
        tier3(agg),
        integrity(agg),
        "<footer>Generated by report.py from aggregate.json. "
        "Questions and answer key: <span class='mono'>keys/tier1.json</span>. "
        "Rubric and ground truth: <span class='mono'>keys/tier2.md</span>.</footer>",
    ]
    return (
        "<!doctype html><html lang='en'><head><meta charset='utf-8'>"
        "<meta name='viewport' content='width=device-width,initial-scale=1'>"
        f"<title>Nova navigability - {esc(runs)}</title><style>{CSS}</style></head>"
        "<body><main>" + "\n".join(p for p in parts if p) + "</main></body></html>"
    )


def main(argv):
    if len(argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    agg = load(argv[1])
    cmp_agg = load(argv[2]) if len(argv) > 2 else None
    key = json.loads((HERE / "keys" / "tier1.json").read_text())

    out = RESULTS / argv[1] / "report.html"
    out.write_text(build(agg, cmp_agg, key))
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
