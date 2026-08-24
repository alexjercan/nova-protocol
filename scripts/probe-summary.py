#!/usr/bin/env python3
"""Render a probe sweep's index.json as a GitHub job-summary table.

Formatting only: every verdict here is read from index.json, which the probe
binary owns. This must never re-derive one - two verdict implementations can
disagree, and index.json is the one that gated the job.

Advisory by contract: a missing or unreadable index.json prints a pointer to
the artifact and exits 0. The sweep step is the gate; this only reports.
"""

import argparse
import json
import sys
from pathlib import Path

# Statuses a check can carry without being a problem. Anything else is named
# in the row so a red run says WHICH check failed without a download.
OK_STATUS = {"PASS", "N/A", "-"}


def find_index(runs: Path) -> Path | None:
    """The newest <runs>/<short-sha>/index.json. The sweep keys its output
    directory by commit, so a warm runner may hold more than one."""
    found = sorted(runs.glob("*/index.json"), key=lambda p: p.stat().st_mtime)
    return found[-1] if found else None


def failing(row: dict) -> str:
    checks = [c["name"] for c in row.get("checks", []) if c.get("status") not in OK_STATUS]
    return ", ".join(checks) if checks else ""


def render_row(row: dict) -> str:
    detail = failing(row) or (row.get("error") or "")
    return "| {} | {} | {} | {} | {} |".format(
        row.get("example", "?"),
        row.get("verdict", "?"),
        row.get("measured", "-"),
        row.get("duration_secs", 0),
        detail.replace("|", "\\|")[:200],
    )


HEADER = "| example | verdict | measured | secs | detail |\n|---|---|---|---|---|"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--runs", required=True, type=Path, help="the sweep's --out directory")
    ap.add_argument("--title", default="probe", help="heading for this shard")
    ap.add_argument("--artifact", default="", help="artifact name to point at for the HTML")
    args = ap.parse_args()

    index = find_index(args.runs)
    if index is None:
        print(f"## {args.title}\n")
        print("No `index.json` under the run directory - the sweep died before it")
        print("aggregated. The step log above has the failure.")
        return 0
    try:
        data = json.loads(index.read_text())
    except (OSError, json.JSONDecodeError) as err:
        print(f"## {args.title}\n\nCould not read `{index}`: {err}")
        return 0

    rows = data.get("rows", [])
    overall = data.get("overall", "?")
    tally = {}
    for row in rows:
        tally[row.get("verdict", "?")] = tally.get(row.get("verdict", "?"), 0) + 1
    counts = ", ".join(f"{n} {verdict}" for verdict, n in sorted(tally.items()))
    minutes = sum(row.get("duration_secs", 0) for row in rows) / 60

    print(f"## {args.title} - {overall}\n")
    print(
        f"{len(rows)} example{'' if len(rows) == 1 else 's'} ({counts}), "
        f"{minutes:.1f} min of runs. "
        f"spec `{data.get('spec', '?')}`, git `{data.get('git_sha', '?')}`.\n"
    )

    # Anything past OK leads, uncollapsed - a red run must not need a click.
    bad = [row for row in rows if row.get("verdict") != "OK"]
    good = [row for row in rows if row.get("verdict") == "OK"]
    if bad:
        print(HEADER)
        for row in bad:
            print(render_row(row))
        print()
    if good:
        print(f"<details><summary>{len(good)} OK</summary>\n")
        print(HEADER)
        for row in good:
            print(render_row(row))
        print("\n</details>\n")
    if args.artifact:
        print(
            f"Full HTML report (`index.html` + a `report.html` per example): "
            f"the **{args.artifact}** artifact on this run."
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
