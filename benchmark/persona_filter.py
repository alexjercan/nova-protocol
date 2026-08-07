#!/usr/bin/env python3
"""Which tier 1 questions a persona is asked. The single source of truth.

This rule decides three things that must agree or the benchmark measures the
wrong thing: which questions a paper shows, which questions the grader is
allowed to mark against, and how many questions a score is a mean over. It was
implemented twice - once here for `make-papers.py`, once inline in `grade.sh` -
with nothing failing loudly if the two drifted.

Usage as a library:  tier1_questions(key, persona) -> [question, ...]
Usage from a shell:  ./persona_filter.py <key.json> <persona> <out.json>
"""

import json
import pathlib
import sys


def tier1_questions(key, persona):
    """The questions this persona is asked, in key order.

    A missing `personas` field means all of them. `owner` answers a fixed
    subset instead of all 30.
    """
    subset = key["_owner_subset"] if persona == "owner" else None
    out = []
    for q in key["questions"]:
        allowed = q.get("personas")
        if allowed is not None and persona not in allowed:
            continue
        if subset is not None and q["id"] not in subset:
            continue
        out.append(q)
    return out


def tier1_ids(key, persona):
    return [q["id"] for q in tier1_questions(key, persona)]


def main(argv):
    if len(argv) != 4:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    key_path, persona, out = argv[1:4]
    key = json.loads(pathlib.Path(key_path).read_text())
    qs = tier1_questions(key, persona)
    pathlib.Path(out).write_text(json.dumps({"questions": qs}, indent=2))
    print(f"  key filtered to {len(qs)} questions for {persona}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
