# Epic: hold the frame rate, then say what the game does

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.11.0,epic,performance,example,docs

Owner call, 2026-08-18: v0.11.0 is no longer the editor release. The editor
epic `20260812-131912` and its children move to the backlog, and this release
becomes the spike that makes the editor release possible: a game that holds
frame rate, examples a human can actually play, and docs that match what
shipped.

The trigger was `0ee9cbb0` ("Give every destructible body damage it wears in
its own geometry"). It landed correct and it landed slow: the `asteroid_field`
sandbox is unplayable. That is the proof that the project has no standing
answer to "does this cost a frame", and the answer is what this release buys.

## The rule this release establishes

**The main thread is never blocked outside a loading screen.**

Everything below is a consequence of that one rule. Where a choice exists
between a correct visual and a held frame rate, the frame rate wins - the owner
is explicit that a placeholder visual is preferable to a stutter, because a
visual that costs gameplay is a visual nobody sees.

Three ways to obey it, in the order to prefer them:

1. **Bake it at load.** Work whose input is known when the scenario loads
   belongs in the loading screen, not on first hit. Lazy computation is the
   default failure mode here and asteroids are the current example.
2. **Move it off-thread.** Work that cannot be predicted runs on a worker
   across as many frames as it needs, with a placeholder drawn until it
   resolves.
3. **Do less of it.** A volume scan where only a surface can change is the
   clearest case, but the same question applies to LoD, to pooling, and to
   every `count^3` in the tree.

## Workstreams

Performance:

- `PERF-REGRESSION` - carve fields must never cost a frame. Blocks play.
- `PERF-HARNESS` - stress cases that reproduce the stutter, in the probe.
- `PERF-OFFLOAD` - the worker + placeholder pattern, as shared machinery.
- `PERF-SURFACE` - mesh the rock's surface, not its volume.
- `PERF-BAKE` - move scenario work into the loading screen.
- `PERF-PRELOAD` - load the next scenario behind the current one.
- `PERF-LOD` - distant bodies stop paying full price.

Examples and docs:

- `EX-PLAYABLE` - every example is playable by hand, or says why it is not.
- `DOC-DESTRUCTION` - the landed destruction model reaches all three surfaces.
- `DOC-VISUALS` - the wiki stops being walls of text (`20260818-181812`).

## What "done" means for the release

- **A 1v1 holds 60 FPS.** Owner, 2026-08-20, replacing the 4v4 target
  (`DECISIONS.md` D11). 4v4 at 60 was unreachable by construction: eight hulls
  cost 16.0 ms of protected non-drawn work against a 16.67 ms budget. At 1v1
  that is 4.00 ms, so the protected half stops binding and the target is real
  work rather than an arithmetic impossibility. Today 1v1 measures 26.3 ms
  (38 FPS uncapped, reading ~30 on a vsynced hand-run). The floor alone is
  16.74 ms - 100.4% of the budget - so cutting it is a precondition, not an
  optimisation.
- A `wfc_arena` 4v4 still gets a MEASURED number, as the scaling check - **but
  not yet that number.** The 295.76 ms worst frame quoted from `20260819-123928/NOTES.md`
  is RETRACTED, and that page now says so. Two faults, either one fatal: the
  capture opened on a scoreboard predicate with no upper bound, so 2 of 10
  repeats ran past `match ended` into a PAUSED result screen (one spent 555 of
  its 900 frames with the simulation stopped, at 88 ms/frame, and still read as
  a plausible 93 ms mean); and 295.76 ms was ONE slowest frame of ONE window on
  a subject whose worst frame spreads 169% of its median across honest repeats.
  The INSTRUMENT is fixed (`20260819-173219` phase B1): the arena's window is
  bounded to a fixed count of frames the simulation actually ran through, and
  any capture that meets a stopped `Time<Virtual>` is REFUSED - it writes no
  statistics and fails the run under its own check name. The replacement figure
  is pending a repeat set on an idle box.
  `asteroid_field` was named here in error - it was deleted in `d20a37c4`, the
  same day this epic was written.
- No system in a profiled fight owns a frame on its own.
- A human can load every shipped example and do something in it, or read one
  line in its description saying it is a capture rig.
- `/wiki`, `/create` and the dev book describe the game that actually shipped,
  and the wiki leads with visuals.

## What may be traded for frame rate, and what may NOT

Owner, 2026-08-20, and this governs every optimisation in the epic.

**PRESENTATION is negotiable.** If the game feels and looks the same with
simpler, faster code, take the simpler code. Damage cracks quantised into eight
buckets instead of a continuous per-section value is the worked example: the
signal a player needs is "that section looks wrong", not "that section is 47%
damaged". Nobody will ever see the difference, and it is worth a 2x.

**PHYSICS and GAMEPLAY LOGIC are NOT.** They are the main focus of the game.
Approximating them to buy frames trades the thing people are here for. Be
careful around the detail: a shortcut in the solver, in collision, in damage
propagation or in flight is not the same kind of trade as a shortcut in a
material.

The test to apply before proposing any optimisation: **would a player notice
this as a change to what the game DOES, or only to how it looks?** Only the
second is on the table by default. The first needs the owner, every time.

Corollary for ablation work: a measured 10% in physics is a worse lead than a
measured 10% in presentation, even though the numbers match.

## Standing measurement rule

A performance claim without a before and an after is not a result. Report the
WORST frame and the top system self-time; a stutter is a tail, and a mean hides
it. A number that did not move gets reported as a number that did not move.
