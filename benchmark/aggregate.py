#!/usr/bin/env python3
"""Collapse a run's result directory into one aggregate.json plus a flat CSV.

Everything downstream - the HTML report, any spreadsheet, any comparison
between baseline and after - reads aggregate.json. Nothing reads the raw
result tree twice.

The tool-call count comes from the transcript, not from the agent's
self-report. Both are recorded and the report shows them side by side; a large
gap between them is itself a finding about the agent, not about the codebase.

Usage:
  ./aggregate.py <run-id> [<run-id> ...]     # writes results/<run>/aggregate.json
"""

import csv
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
RESULTS = HERE / "results"

# A persona is scored against the questions it was asked, never against 30.
ORDER = ["blind", "tree", "docs", "rustdoc", "modder", "owner"]

# Tier 1 grades are continuous in [0, 1]. Runs graded before that change carry
# the four-way enum instead; map it onto the same scale so an old run stays
# comparable rather than silently reading as all-zero.
LEGACY_GRADE_POINTS = {"right": 1.0, "partial": 0.5, "wrong": 0.0, "gave-up": 0.0}

# Reaching the network is not forbidden - the agent talks to the API over it -
# but pulling repository content over it would invalidate a run. The repo is a
# private remote and the container holds no key for it, so this is a
# belt-and-braces check on an already-closed door.
NETWORK_TOOLS = {"WebFetch", "WebSearch"}
NETWORK_SHELL = ("git clone", "git fetch", "git remote", "curl ", "wget ")


def read_json(path, default=None):
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError):
        return default


def parse_transcript(path):
    """Count tool calls out of a Claude Code stream-json transcript."""
    out = {
        "tool_calls": 0,
        "tools": {},
        "turns": None,
        "duration_s": None,
        "cost_usd": None,
        "network_hits": [],
        "transcript": path.exists(),
    }
    if not path.exists():
        return out

    for line in path.read_text(errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue

        if ev.get("type") == "assistant":
            for block in ev.get("message", {}).get("content", []) or []:
                if block.get("type") != "tool_use":
                    continue
                name = block.get("name", "?")
                out["tool_calls"] += 1
                out["tools"][name] = out["tools"].get(name, 0) + 1
                if name in NETWORK_TOOLS:
                    out["network_hits"].append(name)
                elif name == "Bash":
                    cmd = str(block.get("input", {}).get("command", ""))
                    for pat in NETWORK_SHELL:
                        if pat in cmd:
                            out["network_hits"].append(cmd[:120])
                            break

        elif ev.get("type") == "result":
            out["turns"] = ev.get("num_turns")
            if ev.get("duration_ms") is not None:
                out["duration_s"] = round(ev["duration_ms"] / 1000, 1)
            out["cost_usd"] = ev.get("total_cost_usd")

    return out


def grade_score(g):
    """The grader's 0-1 score for one question, plus whether it gave up.

    Continuous, so a three-part question answered two-thirds reads as 0.67
    rather than collapsing into the same bucket as a one-third answer. The
    key's `notes` still pin the common cases to 0.5 - see papers/grade-tier1.md.
    """
    if "score" in g:
        try:
            score = min(1.0, max(0.0, float(g["score"])))
        except (TypeError, ValueError):
            score = 0.0
        return round(score, 3), bool(g.get("gave_up"))
    legacy = g.get("grade", "wrong")
    return LEGACY_GRADE_POINTS.get(legacy, 0.0), legacy == "gave-up"


def score_tier1(answers, grades):
    """Per-question rows plus the headline band counts."""
    by_id = {a.get("id"): a for a in (answers or {}).get("answers", []) or []}
    rows, tally = [], {"full": 0, "partial": 0, "zero": 0, "gave-up": 0}

    for g in (grades or {}).get("grades", []) or []:
        score, gave_up = grade_score(g)
        # Bands are for the report only; `score` is the number that counts.
        # gave-up is split out of zero because an honest refusal and a
        # confident wrong answer are the same points and a different finding.
        if gave_up:
            tally["gave-up"] += 1
        elif score >= 1.0:
            tally["full"] += 1
        elif score > 0.0:
            tally["partial"] += 1
        else:
            tally["zero"] += 1
        a = by_id.get(g.get("id"), {})
        rows.append(
            {
                "id": g.get("id"),
                "score": score,
                "gave_up": gave_up,
                "why": g.get("why"),
                "answer": a.get("answer"),
                "self_tool_calls": a.get("tool_calls"),
                "confidence": a.get("confidence"),
                "detours": len(a.get("detours") or []),
            }
        )

    asked = len(rows) or len(by_id)
    points = round(sum(r["score"] for r in rows), 3)
    return {
        "asked": asked,
        "answered": len(by_id),
        "tally": tally,
        "points": points,
        "score": round(points / asked, 3) if asked else None,
        "self_tool_calls_total": sum(
            r["self_tool_calls"] or 0 for r in rows if isinstance(r["self_tool_calls"], int)
        ),
        "questions": rows,
    }


def score_tier2(grades):
    if not grades:
        return None
    s = grades.get("scores", {}) or {}
    dims = ["ownership", "completeness", "no_phantom_structure", "cost_of_arrival"]
    got = [s.get(d) for d in dims]
    return {
        "scores": {d: s.get(d) for d in dims},
        "total": sum(v for v in got if isinstance(v, int)),
        "max": 12,
        "missed_required": grades.get("missed_required", []),
        "phantom_paths": grades.get("phantom_paths", []),
        "citations": grades.get("citations", {}),
    }


def collect(run_dir):
    rows = []
    for persona_dir in sorted(p for p in run_dir.iterdir() if p.is_dir()):
        persona = persona_dir.name
        for paper_dir in sorted(p for p in persona_dir.iterdir() if p.is_dir()):
            paper = paper_dir.name
            meta = read_json(paper_dir / "meta.json", {}) or {}
            run_meta = read_json(paper_dir / "run.json", {}) or {}
            tr = parse_transcript(paper_dir / "transcript.jsonl")

            row = {
                "persona": persona,
                "paper": paper,
                "model": run_meta.get("model", "unknown"),
                "image_built_from": run_meta.get("image_built_from"),
                "exit_code": run_meta.get("exit_code", meta.get("exit_code")),
                "tool_calls": tr["tool_calls"],
                "tool_calls_self": meta.get("tool_calls"),
                "tools": tr["tools"],
                "turns": tr["turns"],
                "duration_s": tr["duration_s"],
                "cost_usd": tr["cost_usd"],
                "confidence": meta.get("confidence"),
                "network_hits": tr["network_hits"],
                "has_transcript": tr["transcript"],
            }

            grades = read_json(paper_dir / "grade" / "grades.json")
            if paper == "tier1":
                answers = read_json(paper_dir / "answers.json")
                row["tier1"] = score_tier1(answers, grades)
                if row["tool_calls_self"] is None:
                    row["tool_calls_self"] = row["tier1"]["self_tool_calls_total"] or None
                row["graded"] = grades is not None
            elif paper.startswith("tier2"):
                row["tier2"] = score_tier2(grades)
                row["graded"] = grades is not None
                row["has_notes"] = (paper_dir / "NOTES.md").exists()
            elif paper == "tier3":
                verdict = read_json(paper_dir / "verdict.json", {}) or {}
                row["tier3"] = {
                    "verdict": verdict.get("verdict", "not-verdicted"),
                    "lint_output": verdict.get("lint_output"),
                    "notes": verdict.get("notes"),
                    "gaps_present": (paper_dir / "GAPS.md").exists(),
                    "mod_files": sorted(
                        str(f.relative_to(paper_dir))
                        for f in (paper_dir / "salvage-run").rglob("*")
                        if f.is_file()
                    )
                    if (paper_dir / "salvage-run").is_dir()
                    else [],
                }
                row["graded"] = verdict.get("verdict") is not None
            rows.append(row)
    return rows


def deltas(rows):
    """The comparisons the benchmark exists to make.

    A refactor that raises `docs` but not `blind` or `tree` moved code without
    making it findable. These three numbers are how that shows up.
    """
    t1 = {r["persona"]: r for r in rows if r["paper"] == "tier1"}

    def score(p):
        return (t1.get(p, {}).get("tier1") or {}).get("score")

    def calls(p):
        return t1.get(p, {}).get("tool_calls")

    def diff(a, b, fn):
        x, y = fn(a), fn(b)
        return round(x - y, 3) if x is not None and y is not None else None

    return {
        "docs_minus_blind_score": diff("docs", "blind", score),
        "docs_minus_blind_tool_calls": diff("docs", "blind", calls),
        "owner_minus_docs_score": diff("owner", "docs", score),
        "tree_score": score("tree"),
        "blind_score": score("blind"),
        "note": (
            "docs_minus_blind should SHRINK after the refactor: the tree starts "
            "answering what prose was carrying. tree_score rising is the literal "
            "target. docs rising alone is the failure mode."
        ),
    }


def write_csv(path, rows):
    cols = [
        "persona", "paper", "model", "tool_calls", "tool_calls_self",
        "duration_s", "cost_usd", "score", "full", "partial", "zero",
        "gave_up", "tier2_total", "tier3_verdict", "network_hits",
    ]
    with path.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=cols)
        w.writeheader()
        for r in rows:
            t1 = r.get("tier1") or {}
            tally = t1.get("tally") or {}
            w.writerow(
                {
                    "persona": r["persona"],
                    "paper": r["paper"],
                    "model": r["model"],
                    "tool_calls": r["tool_calls"],
                    "tool_calls_self": r["tool_calls_self"],
                    "duration_s": r["duration_s"],
                    "cost_usd": r["cost_usd"],
                    "score": t1.get("score"),
                    "full": tally.get("full"),
                    "partial": tally.get("partial"),
                    "zero": tally.get("zero"),
                    "gave_up": tally.get("gave-up"),
                    "tier2_total": (r.get("tier2") or {}).get("total"),
                    "tier3_verdict": (r.get("tier3") or {}).get("verdict"),
                    "network_hits": len(r["network_hits"]),
                }
            )


def main(argv):
    if len(argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    for run in argv[1:]:
        run_dir = RESULTS / run
        if not run_dir.is_dir():
            print(f"aggregate.py: no run at {run_dir}", file=sys.stderr)
            return 1

        rows = collect(run_dir)
        rows.sort(key=lambda r: (ORDER.index(r["persona"]) if r["persona"] in ORDER else 99, r["paper"]))
        agg = {"run": run, "rows": rows, "deltas": deltas(rows)}

        (run_dir / "aggregate.json").write_text(json.dumps(agg, indent=2) + "\n")
        write_csv(run_dir / "aggregate.csv", rows)

        ungraded = [f"{r['persona']}/{r['paper']}" for r in rows if not r.get("graded")]
        flagged = [f"{r['persona']}/{r['paper']}" for r in rows if r["network_hits"]]

        print(f"{run}: {len(rows)} results -> aggregate.json, aggregate.csv")
        if ungraded:
            print("  ungraded: " + ", ".join(ungraded))
        if flagged:
            print("  NETWORK HITS (check before trusting): " + ", ".join(flagged))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
