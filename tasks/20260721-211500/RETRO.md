# Retro: hide cursor while flying (20260721-211500)

## What went well

- Verify-first paid off. The filed brief read as "cursor never hidden", but the
  grab mechanism already existed and was state-driven; the real defect was a
  `cfg!(not(feature = "debug"))` carve-out that made it a no-op in exactly the
  build a developer playtests (`--features dev`). Reading the code before
  touching it (the `verify-stale-brief-against-current-tree` lesson) reframed a
  "build cursor hiding" task into a small "remove a compile-time carve-out +
  reconcile with the inspector" task.
- Surfacing the real fork early. The debug-inspector tension (the egui panel
  needs a pointer) was a genuine design fork, so it went to the owner before any
  code. The chosen option (inspector reclaims the cursor, defaults off) drove a
  clean, minimal change.
- Blast-radius discipline. Found the ammo-readout mirror in nova_gameplay that
  keys off nova's overlay `DebugEnabled(true)`; flipping only the bcs inspector
  default (not nova's overlay default) avoided a desync and kept the change to
  three crates.

## What went wrong / was tricky

- Two `DebugEnabled` resources with the same name (nova's overlay one and bcs's
  inspector one), both F11-toggled, was a genuine trap. It took reading bcs's
  `inspector.rs` to confirm the inspector has its OWN F11 toggle - which is also
  what let me answer the reviewer's MAJOR finding as a non-bug rather than
  adding a redundant toggle that would have double-flipped the resource.
- The reviewer's top findings (F11 does not toggle the inspector) were both
  false alarms rooted in not having bcs's source in view. Cheap to resolve
  (one grep), but a reminder that a cross-repo dependency's behavior needs to be
  cited in-comment or the next reader (human or agent) re-raises it.

## Lessons / what to do differently

- When a cfg/feature carve-out disables a behavior, the reproduction that
  actually fails is a test compiled WITH that feature - a default (feature-off)
  test build passes and proves nothing. Here the honest pins had to live in
  nova_debug (builds with `debug`) and be reasoned as fail-first, since the
  editor/menu test builds are feature-off. Worth stating the fail-first
  reasoning explicitly in the task when the feature matrix hides the bug.
- Name the external owner of a shared toggle in the comment. The
  "F11 raises it (bevy_common_systems' InspectorDebugPlugin owns the toggle)"
  note was added only after a reviewer stumbled on the same gap I did.

## Follow-ups

- None blocking. One `manual:` DoD item remains for the Finish checkpoint: the
  owner replays a `--features dev` build and confirms no cursor while flying
  (the headless harness has no real window to observe).
