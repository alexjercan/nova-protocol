# Retro: Floating chip background covers only a corner of its label

- TASK: 20260730-122909
- BRANCH: fix/chip-full-background
- REVIEW ROUNDS: 2 (APPROVE in round 1; both rounds out-of-context)

## What went well

- The plan's "reproduce FIRST" step paid the whole cycle. The rig printed
  `chip fill Vec2(20.0, 10.0)` vs an independent `Vec2(58.0, 15.0)` layout of
  the same string before a line of production code changed, so the fix was
  aimed rather than guessed, and the numbers went straight into NOTES.md.
- Pinning the MECHANISM against the engine, not the theory, cost about ten
  minutes and left a durable artifact: `taffy_drops_the_text_measure_when_a_text_node_has_children`
  lays the same bundle out as a leaf and as a container and asserts the
  container collapses to EXACTLY its frame. If a future bevy measures container
  text, that test fails and tells the next reader the chips can be simplified.
- The independent-reference trick made the assertion non-tautological: the chip
  is compared against a bare leaf `Text` node laid out by the same engine, not
  against a re-multiplied em fraction (ledger
  `test-must-not-reuse-the-formula-under-test`).
- The out-of-context reviewer earned its keep twice: it re-ran the fail-first
  experiment itself, and it caught a regression I could not see (below).

## What went wrong

- **R1.1, the one real defect: a scripted `str.replace` edited two sites when I
  meant one.** I shrank the capture beat's beacon radius with a Python
  `s.replace(old, new)` whose anchor (`label: "WAYPOINT".to_string(),\n radius:
  2.0,`) also matched the SCENARIO's beacon - the subject of the published
  `tutorial-radar-lock.png`. Python's `str.replace` is replace-ALL by default;
  I passed no count. Root cause: I used a bulk text tool for a single-site edit
  and then verified the OUTCOME I was looking for (the new shot) without
  re-reading the produced diff. The AGENTS.md rule "an edit you believe you made
  is a hypothesis until the artifact shows it" applies to edits you did NOT
  intend to make too - the artifact to read is the diff, not just the result.
- **Three rig-bring-up dead ends, all the same shape.** The first App panicked
  every frame on `Resource does not exist` with no system name (bevy hides it
  without its `debug` feature). `UiPlugin` silently pulls in the accessibility
  and picking backends and runs `ui_focus_system`, and the text/image content
  passes want `Assets<Image>` / `Assets<TextureAtlasLayout>` that only render
  plugins provide. I found each one by re-running with `BEVY_BACKTRACE=full` and
  grepping the backtrace for the system's PARAMETER SIGNATURE - which works, but
  I only reached for it on the second failure. Root cause: I assembled the rig
  from first principles instead of starting from bevy_ui's own
  `setup_ui_test_app`, which I had already read.
- **The capture beat's teardown reset its own spawn guard.** `chip_subjects.take()`
  set the `Option` back to `None`, which was also the spawn condition, so the
  subjects respawned every frame - a log flood plus a starved script that never
  fired `feature-autopilot.png`. The `Option` was doing double duty as both
  state and guard.

## What to improve next time

- Never use an unbounded `str.replace` for a single-site source edit. Use the
  Edit tool (which fails loudly on an ambiguous match) or pass `count=1` with a
  uniqueness assertion - and read the resulting `git diff` before believing the
  edit, not just the artifact it was supposed to produce.
- When standing up a rig for an engine subsystem, copy the ENGINE'S OWN test
  harness first (bevy_ui ships `setup_ui_test_app` in `layout/mod.rs`) and
  mutate it, rather than deriving the plugin set from the plugin's `build`.
  This is the existing `reuse-known-good-stack` lesson applied one level out:
  the nearest known-good rig may live in the dependency, not the repo.
- On an anonymous bevy system-param panic, go straight to
  `BEVY_BACKTRACE=full` + grep for `run_unsafe<fn(` - the parameter signature
  names the system when the feature flag does not.
- A one-shot guard and the state it guards must be separate fields.

## Action items

- [x] Ledger: added `bulk-replace-edits-more-than-you-aimed-at`,
      `copy-the-engines-own-test-harness`,
      `bevy-anonymous-system-param-panic-read-the-signature` and
      `one-shot-guard-separate-from-its-state`.
- [x] Fixed `DECISION.md`'s STATUS line, which `tatr check` flagged as
      `bad-decision-status` (the provenance moved to its own `ACCEPTED BY:`
      line).
- No follow-up code tasks: the DoD 3 sweep found no other real instance of the
  `Text` + `children!` shape, and the two chips were the only offenders.
