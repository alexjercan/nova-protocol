# Tier 2b - add a NOVA OS app

Your working directory `/work` holds a single file, `TREE.txt`: the name of
every file in the Nova Protocol repository, one per line. **There is nothing
else, and no file contents exist in this sandbox.** Names are all you have.
Answer from the structure or say you cannot.

Write your output to `/out`. Nothing else you write is kept.

## Task

The in-game NOVA OS terminal can launch apps - today `map` and the ship viewer.
Plan the addition of a **new app**: a "cargo manifest" app, launched by typing
`cargo` at the NOVA OS prompt, that lists the player ship's sections and their
status, with its own key handling and its own brightness behaviour honouring
the monitor settings.

You do not need to design the visuals. You need to identify every part of the
codebase that has to change, and say what each change is.

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

- `/out/NOTES.md` - your answer, in the two sections described above.
- `/out/meta.json` - `{"tool_calls": <your count>, "confidence": "high|medium|low"}`

The harness counts tool calls independently from the transcript. Report yours
honestly; the two are compared.
