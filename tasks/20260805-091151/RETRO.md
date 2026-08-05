# Retro: A stray cursor event mid-click cancels the driven click

- TASK: 20260805-091151
- BRANCH: master (landed in place, `87bcb956`)
- REVIEW ROUNDS: 1

## What went well

- **Reading the dependency beat trusting the brief.** The task arrived with a
  mechanism marked CONFIRMED. Thirty minutes in the vendored `bevy_picking`
  source disproved it: window events become `PointerInput` a frame later with
  the Move stamping the Press location, so warp-and-press in one call is
  self-consistent, and a defect there would fail every run rather than one in
  three. Everything after that was aimed at the right thing.
- **Injecting the hypothesis when the ambient trigger would not come out.**
  218 runs across three shapes reproduced nothing. One injected stray
  `CursorMoved` reproduced the owner's log to the character, in seconds,
  repeatably. A rig that MAKES the failure beat waiting for it.
- **Fixing the class rather than the source.** Which ambient X event fires in
  CI is still unknown. The pin does not care, and the record says so instead
  of implying the question was answered.
- **The fail-first number is real.** Deleting the registration turns the guard
  red; that is in `TASK.md`, not asserted from memory.

## What went wrong

- **Two experiments were wasted on a hypothesis that was wrong for a knowable
  reason.** 40 concurrent runs went looking for cross-process pointer warping,
  when a grep would have shown that only `ui/` examples warp the cursor and the
  smoke suite runs that category SEQUENTIALLY. The cheap check that kills a
  hypothesis should come before the expensive one that tests it.
- **The first reproduction loop ran from the wrong directory.** All 40 runs
  exited 1 on `path "/tmp" is not part of a flake`, which read as a 40/40
  reproduction for one confusing minute. The results file recorded exit codes
  and nothing else, so nothing in it said what actually happened.
- **The prototype's rig and the landed fix disagreed.** The prototype injected
  a message-only stray and re-asserted the pointer every frame. The landed pin
  detects a stray from the WINDOW, so the prototype's rig would not exercise
  it. Caught while writing the notes, not while writing the code; the rig had
  to be rebuilt faithfully to confirm the fix end to end.
- **The DoD command was written but not run until closing time.** It failed on
  its first run - on an unrelated fault (`20260805-111329`), but the point
  stands: a proof nobody has executed is a plan, not a proof.

## What to improve next time

- Kill a hypothesis with a grep before testing it with a fleet of runs.
- A reproduction loop records what it observed, not just an exit code; the
  first thing to check on a surprising result is whether the harness ran at
  all.
- When a prototype becomes a decision, re-derive the rig against what actually
  landed - "the prototype proved it" is only true while the two match.
- Run every DoD proof once at the moment it is written.

## Action items

- `20260805-111329` filed: `menu_scenarios` killed by a signal in the ui
  smoke, ~1 run in 5. Found by the DoD command, unrelated to this fix, gates
  CI.
- Idea 2 in `NOTES.md` (`hovered_named` / `pressed_named` predicates, retiring
  the `frames(SETTLE)` anti-pattern at the nine `click_named` call sites) is
  unclaimed. It belongs to the epic's "advance on observed state" line, not to
  this fix, and no task carries it yet.
