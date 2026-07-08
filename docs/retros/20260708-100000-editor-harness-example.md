# Retro: Harnessed editor example for smoke testing

- TASK: 20260708-100000
- BRANCH: feat/editor-harness-example
- PR: #45 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260708-100000/TASK.md`. This closes the "editor is unverifiable headless" action
item the 20260706-212909 retro raised - the previous task's lesson turned into the next task's
deliverable.

## What went well

- Acted on a prior retro's action item directly. The editor-preview retro said "if editor bugs
  keep coming, an editor harness would pay for itself." When the user asked for exactly that, the
  design was already scoped: reuse the existing `nova_autopilot`/`nova_screenshot` presets against
  the editor app. Retros feeding the next task is the whole point of the loop.
- Found the one seam that makes editor UI drivable headless. The editor has no keyboard shortcuts
  for its actions, but its buttons carry `Name`s and fire `ui_widgets::Activate` (an EntityEvent).
  Reading the button wiring showed the autopilot can find a button by `Name` and `world.trigger`
  an `Activate` at it - no test-only hooks added to the editor. Reading the event's target
  semantics (from the earlier slider work) is what made this obvious.
- Made the example the *same* app as the game, not a lookalike. Factoring `editor_app` means a
  future divergence between "what the binary runs" and "what the test runs" is impossible by
  construction - which is the whole value of a smoke test for the shipped app.
- Got double duty out of one example: it is both an editor smoke test and a live regression test
  for the 20260706-212909 fix (create a controller ship -> assert no `q_root` spam), verified in
  the same headless run.

## What went wrong

- One clippy round-trip on a feature-gated import: `use bevy::prelude::*` is only needed by the
  debug-gated autopilot, so it was unused in the default (`--all-targets`, no-debug) build. Root
  cause: wrote the imports for the debug path and did not consider the non-debug compile. For an
  example with a `#[cfg(feature = "debug")]` harness block, imports used only inside it must be
  gated too.

## What to improve next time

- When an example (or module) has a `#[cfg(feature = "...")]` block, audit top-level imports for
  ones only that block uses and gate them the same way - and run `clippy --all-targets` (default
  features), which is the build that catches it, not just the feature-on build.
- Editor/UI actions that need driving in a test: look for the widget event (Activate/ValueChange)
  + a stable entity identifier (Name/marker) rather than simulating pointer input; triggering the
  event directly is far more robust headless than faking clicks.

## Action items

- [ ] Possible extension: drive section *placement* in the editor autopilot (needs simulating
      grid pointer picking) to cover more of the editor, and/or a screenshot baseline for the
      editor UI.
- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro).
