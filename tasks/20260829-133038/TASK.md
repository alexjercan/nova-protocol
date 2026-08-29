# Events mode: the editor takes the screen, expressions become nodes

- STATUS: OPEN
- PRIORITY: 63
- TAGS: v0.12.0, editor, ui, events

Successor to `20260825-223035` (editor events), from the owner's read of the
shipped panel: the Inspector is too narrow to hold what a handler says, the
hierarchy crops its own rows, and an expression reads as a line of text where it
should read as nodes.

## What is actually wrong

- `RAIL_W` 210 + `PANEL_W` 300 exist to keep the stage CENTRE clickable on a
  1024-wide window (`ui/mod.rs`), because the placement raycast goes there. That
  is a SCENE constraint. In Events mode nothing is being placed, so the width is
  paid for nothing.
- Rail rows are `NoWrap` + clip with the indent charged to left padding
  (`ui/rail.rs`), so a sequence step four levels down has almost no label left.
- The Inspector's value column is about 170px and `WIDE_CHOICE` is 3, so
  Player / AI / None is squeezed into it and a 26-option action switch becomes a
  wall of chips.

## The settled shape

TWO MODES, switched by the rail tabs that already exist.

- **Scene** stays what it is: stage, rail tree, Inspector on the right. (The
  owner wants this to become something more DIEGETIC later. Not this task.)
- **Events** takes the WHOLE SCREEN: the events hierarchy on the left, the
  events editor filling everything the rail does not. No stage behind it, no
  300px Inspector: the editor pane is the surface the handler is edited on.
- The tabs stay in the rail, so the mode switch is one click and the two trees
  keep one home.

## Expressions as nodes

An expression becomes document nodes like everything else - each operator a
node, its operands its children, kind-switched the way a filter is - and the
OPERATOR carries the emphasis: `==`, `+`, `>` are what the eye should find
first, not the operands. The text form (`nova_scenario/src/syntax.rs`) stays for
a leaf and for round-tripping the file.

## Done when

- Clicking EVENTS leaves the scene and opens the events editor full screen;
  clicking SCENE comes back with the selection intact.
- Nothing a handler holds is cropped: the deepest sequence step reads its whole
  label, and every choice row draws all its options.
- Player / AI / None is not cut off in Scene mode either.
- An expression reads as nodes with the operator emphasised, edits as nodes, and
  still lowers to the same tree the file holds.
- A UI-harness walk covers the mode switch and an expression edit; probe green.
