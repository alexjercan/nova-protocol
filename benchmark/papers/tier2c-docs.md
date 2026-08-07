# Tier 2c - add a scenario action and event

Your working directory `/work` holds the project's prose only: `AGENTS.md`,
`README.md`, `CONVENTIONS.md`, and the full player and developer wiki under
`wiki/`. **There is no source code.** If the documentation does not say it, you
cannot look it up.

Write your output to `/out`. Nothing else you write is kept.

## Style

Be terse. Fragments over sentences, bullets over prose. No preamble, no
restating the question, no summary of what you are about to do. Name the thing
and stop.

Length is not evidence. An answer that names the right path in one line scores
exactly what a page naming the same path scores. Padding a thin answer does not
move it: the grader scores what you located, never how much you wrote.

## Task

Scenario RON files declare handlers: an event fires, and a list of actions
runs. Plan the addition of a **new event and a new action**:

- a new event `OnDocked`, fired when a player ship comes to rest inside a
  scenario area tagged as a dock;
- a new action `SetHullRepairRate`, which changes how fast a scoped ship's hull
  sections regenerate.

You do not need to design the repair mechanic. You need to identify every part
of the codebase that has to change, and say what each change is.

## What to write

You are being asked to plan a change, not to make it. **Do not write code.**

Write `/out/NOTES.md` with two sections:

1. **What I understand.** How the affected subsystem is put together today -
   which crates and modules own which part, and how they connect.
2. **What I would change.** Every file you would touch, why, and in what order.
   Name files by path. If you are unsure whether a file is involved, say so
   rather than omitting it.

Naming a file that turns out not to be involved costs you less than silently
omitting one. Inventing a path that does not exist costs you the most.

## Output

Write two files:

- `/out/NOTES.md` - your answer, in the two sections described above. **Bullets
  and tables, not paragraphs. Target 400 words; 800 is the ceiling.** One line
  per file: the path, then what changes. A note that names every surface in 300
  words outscores one that names half of them in 2,000.
- `/out/meta.json` - `{"tool_calls": <your count>, "confidence": "high|medium|low"}`

The harness counts tool calls independently from the transcript. Report yours
honestly; the two are compared.
