#!/usr/bin/env python3
"""Generate papers/<paper>-<persona>.md - the exam papers handed to the agents.

Papers are generated, never hand-edited. Two reasons:

- tier 1 has one source of truth. The questions live in keys/tier1.json next to
  their answers; drift between a hand-kept question sheet and the key would
  silently change what is being measured.
- a persona is never shown a question it is not asked. Seeing a question is
  itself a hint about what exists.

Question bodies for tiers 2 and 3 are hand-written in papers/src/. This script
only wraps them in the envelope: what the sandbox holds, and what to write out.

Usage: ./make-papers.py [--check]
"""

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
KEY = HERE / "keys" / "tier1.json"
SRC = HERE / "papers" / "src"
OUT = HERE / "papers"

# Every persona that answers a paper, including the two with no image.
PERSONAS = ["blind", "docs", "tree", "rustdoc", "modder", "owner", "human"]

# What the persona will find when it looks. Stated up front so that discovering
# the shape of the sandbox does not itself cost tool calls - tool calls are the
# metric, and they must measure navigating the codebase, not the harness.
PAYLOAD = {
    "blind": """Your working directory `/work` holds the complete Nova Protocol source
tree - every crate, every asset, the build files. **There are no `.md` files.**
Not the README, not the wiki, not the architecture notes; they have been
removed. Code and filenames are all you have.""",
    "docs": """Your working directory `/work` holds the project's prose only: `AGENTS.md`,
`README.md`, `CONVENTIONS.md`, and the full player and developer wiki under
`wiki/`. **There is no source code.** If the documentation does not say it, you
cannot look it up.""",
    "tree": """Your working directory `/work` holds a single file, `TREE.txt`: the name of
every file in the Nova Protocol repository, one per line. **There is nothing
else, and no file contents exist in this sandbox.** Names are all you have.
Answer from the structure or say you cannot.""",
    "rustdoc": """Your working directory `/work` holds the rendered `cargo doc` output for
every crate in the workspace: the public API and its doc comments, as HTML.
**The `[source]` pages have been removed** - you cannot read the source, only
what the public API and its documentation disclose.""",
    "modder": """Your working directory `/work` holds four wiki pages under `wiki/` and two
complete worked mods under `webmods/`. **There is no source code and no
repository** - mods are data, and the wiki plus the worked examples are exactly
what a real modder starts from.""",
    "owner": """You are working in the repository itself, with everything you already know
about it. You are the control: your job is to establish the ceiling and to
validate that the questions are answerable at all.""",
    "human": """You are working in the repository itself. You are the human control.""",
}

ENVELOPE_HEAD = """# {title}

{payload}

Write your output to `/out`. Nothing else you write is kept.
"""

TIER1_HOWTO = """
## How to answer

Answer at the grain the question asks for - crate, module, file or symbol. Some
questions have two or three parts; answer every part, they are graded
separately. "None" and "nowhere" are legitimate answers to some of these.

If you cannot answer, use `"gave-up"` as the answer and record the tool calls
you spent getting there.

**Do not guess silently.** A confident wrong answer and an honest `gave-up` are
graded differently, and the difference is the point of the exercise.

## Output

Write `/out/answers.json`:

```json
{
  "answers": [
    {
      "id": "t1-001",
      "answer": "your answer, at the grain asked",
      "tool_calls": 3,
      "detours": ["paths you opened that were not on the path to the answer"],
      "confidence": "high | medium | low"
    }
  ]
}
```

One entry per question below, in order. `tool_calls` is your own count for that
question; the harness counts independently from the transcript, so do not
inflate or round it - the two are compared.

Record `confidence` before you check anything. It is colour, not a score.

## Questions
"""

TIER_OUTPUT = {
    "tier2": """
## Output

Write two files:

- `/out/NOTES.md` - your answer, in the two sections described above.
- `/out/meta.json` - `{"tool_calls": <your count>, "confidence": "high|medium|low"}`

The harness counts tool calls independently from the transcript. Report yours
honestly; the two are compared.
""",
    "tier3": """
## Output

Everything goes under `/out`:

- `/out/salvage-run/` - the mod itself
- `/out/GAPS.md` - the wiki gaps, as described above
- `/out/meta.json` - `{"tool_calls": <your count>, "confidence": "high|medium|low"}`
""",
}

TITLES = {
    "tier1": "Tier 1 - locate",
    "tier2a": "Tier 2a - add a new ship section type",
    "tier2b": "Tier 2b - add a NOVA OS app",
    "tier2c": "Tier 2c - add a scenario action and event",
    "tier3": "Tier 3 - build a mod",
}

# Papers put to each persona. tier 3 is the modder's only paper, and the modder
# answers nothing else - it is a pass/fail regression guard, not a delta.
ASKED = {
    "blind": ["tier1", "tier2a", "tier2b", "tier2c"],
    "docs": ["tier1", "tier2a", "tier2b", "tier2c"],
    "tree": ["tier1", "tier2a", "tier2b", "tier2c"],
    "rustdoc": ["tier1", "tier2a", "tier2b", "tier2c"],
    "modder": ["tier3"],
    "owner": ["tier1", "tier2a", "tier2b", "tier2c"],
    "human": ["tier1"],
}


def tier1_questions(key, persona):
    """The questions this persona is asked, in order.

    A missing `personas` field means all of them. `owner` and `human` answer a
    fixed subset instead of all 30.
    """
    subset = key["_owner_subset"] if persona in ("owner", "human") else None
    out = []
    for q in key["questions"]:
        allowed = q.get("personas")
        if allowed is not None and persona not in allowed:
            continue
        if subset is not None and q["id"] not in subset:
            continue
        out.append(q)
    return out


def render_tier1(key, persona):
    qs = tier1_questions(key, persona)
    body = [ENVELOPE_HEAD.format(title=TITLES["tier1"], payload=PAYLOAD[persona])]
    body.append(
        f"\nThere are **{len(qs)}** questions. They are numbered non-consecutively;"
        "\nthat is expected and means nothing.\n"
    )
    body.append(TIER1_HOWTO)
    for q in qs:
        # `grain` is deliberately withheld: it hints how specific an answer to
        # reach for, which is part of what the question is testing.
        body.append(f"\n**{q['id']}.** {q['question']}\n")
    return "".join(body)


def render_src(paper, persona):
    src = (SRC / f"{paper}.md").read_text()
    tier = "tier3" if paper == "tier3" else "tier2"
    return (
        ENVELOPE_HEAD.format(title=TITLES[paper], payload=PAYLOAD[persona])
        + "\n"
        + src.rstrip()
        + "\n"
        + TIER_OUTPUT[tier]
    )


def main():
    check = "--check" in sys.argv
    key = json.loads(KEY.read_text())
    written, stale = [], []

    for persona in PERSONAS:
        for paper in ASKED[persona]:
            text = (
                render_tier1(key, persona)
                if paper == "tier1"
                else render_src(paper, persona)
            )
            path = OUT / f"{paper}-{persona}.md"
            if check:
                if not path.exists() or path.read_text() != text:
                    stale.append(path.name)
            else:
                path.write_text(text)
                written.append(path.name)

    if check:
        if stale:
            print("stale papers: " + ", ".join(stale), file=sys.stderr)
            return 1
        print("papers up to date")
        return 0

    print(f"wrote {len(written)} papers to {OUT}")
    for persona in PERSONAS:
        n = len(tier1_questions(key, persona)) if "tier1" in ASKED[persona] else 0
        print(f"  {persona:8s} tier1: {n:2d} questions   papers: {', '.join(ASKED[persona])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
