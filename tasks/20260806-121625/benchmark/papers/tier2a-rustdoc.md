# Tier 2a - add a new ship section type

Your working directory `/work` holds the rendered `cargo doc` output for
every crate in the workspace: the public API and its doc comments, as HTML.
**The `[source]` pages have been removed** - you cannot read the source, only
what the public API and its documentation disclose.

Write your output to `/out`. Nothing else you write is kept.

## Task

Nova Protocol ships have modular sections: hull, thruster, controller, turret,
torpedo. Plan the addition of a **new section type** - a "shield section" that
projects a damage-absorbing bubble around the ship while it has power, is
toggled by a player keybind, and shows its remaining charge on the HUD.

You do not need to design the physics. You need to identify every part of the
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
