# Editor objectives and win/lose: destroy X, reach Y, survive T

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.12.0,editor,scenario

Successor to `20260714-081703` slice 2. **Blocked on `20260820-223059`
(Sequence).** Do not start before it lands: an objective set authored against
today's action vocabulary would be re-authored the day a story beat becomes one
handler instead of nineteen.

## Goal

Wire simple objectives and win/lose in the editor, and play them through:
destroy X, reach Y, survive T. The world already places the objects those
objectives point at; this is the layer that says what the player is meant to do
with them.

## What exists to build on

- Objectives are POSTED, not declared: `Objective` / `ObjectiveComplete` and
  the marker attach/detach actions (nova_scenario/src/actions/mod.rs:55-61).
  There is no declarative objective type to edit; the editor must either author
  the handler set or gain one.
- The world context is nodes on master: `ObjectNode`s under the scenario node,
  seeded by `ensure_document`, lowered by `nova_editor/src/scenario.rs`.
- The scenario node's inspector is an empty box today - see step 12 of the
  polish plan in `20260825-221015`. That panel is where objectives live.

## The open question

Whether an objective is a first-class config the editor edits and the runtime
posts, or a handler set the editor authors. `Sequence` will settle it: if a
story beat compiles to one handler, authoring handlers is tolerable; if it does
not, objectives need their own type. Decide with that task's shape in hand,
then build.

## Done when

- One authored scenario with ships, objects and an objective set saves,
  reloads, and completes its player path.
- A UI-harness walk covers author -> save -> reload -> play -> win; probe
  green.
