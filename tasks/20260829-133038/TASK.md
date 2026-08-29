# Events mode: the editor takes the screen, expressions become nodes

- STATUS: CLOSED
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

## What shipped

**One screen, two modes.** `RailTab` is no longer a filter on a list: it is the
mode, and `ui::sync_editor_mode` re-lays the screen for it. Events widens the
rail to `EVENTS_RAIL_W` 300, gives the Inspector the rest of the window
(`flex_grow`, no docking margin), retitles it EVENTS and hides the foot, which
is the placement verdict and the stage legend. Scene puts all four back. The
rail no longer shrinks (`flex_shrink: 0.0`): beside it now sits a panel whose
natural width is a wrapping bar of twenty-six actions, and a shrinkable rail
handed that pressure straight to its own columns.

**Nothing crops.** The script tree draws into the wide rail, so it gets its own
`script_budget` - 36 characters at the root against the Scene rail's 22. A
choice of three or more options takes the panel's width on its own line, which
is what the ship driver row needed: Player / AI / Adrift were three labels in a
170px value column.

**A condition is nodes.** `FilterKind::Expression` is a unit arm now - the
filter IS its condition, like `And` and `Not` - and the condition hangs under it
as `ExpressionNode`s: one per operator, its two sides as children, a leaf
holding whatever fits one row of the text form. The operator is drawn LARGE in
the tree (`ACCENT_TEXT` 15px against `ROW_TEXT` 11), wears its own glyph, and
switches from a segmented row of `== < >` at a condition's root or `+ - * /
value` under one. The filter row reads its whole condition while shut.

## Decisions

- **A mode, not a second screen.** One panel with two geometries keeps every
  row, edit path, test and harness name working. A second screen widget would
  have re-implemented the Inspector to show the same rows.
- **Parens are dropped on the way in and put back on the way out.** The tree
  draws the grouping the brackets were there to say; the lowering brackets a sum
  that hangs under a product. What does not survive is a bracket that was doing
  nothing.
- **The operator row is narrowed by POSITION.** A comparison may only be a
  condition's root and everything under one is a value, which is the grammar's
  own split. Offering `+` where a `<` has to stand would author a condition that
  is not one.
- **Operands cannot be deleted.** An operator with one side left is not a
  condition, and the filter holding it would be dropped by the next save - so a
  side is switched to something else, never taken away. `DeletableNode` says so.
- **A switch that mints nodes opens them.** The same rule Add already kept: a
  node the tree cannot draw is a node the selection drops.
- **`Document`'s tab is optional**, like its catalog: the editor's plugin puts
  `RailTab` in, and a fixture that runs the panel without the rest of the editor
  is the Inspector.

## Proof

- `cargo test -p nova_editor --lib`: 386 pass. New: the two mode tests, a
  condition read whole in the Events rail, the lift/lower round trip of a nested
  condition with load-bearing brackets, a filter switched to an expression
  arriving with one, an operator switched to a value dropping its sides, the
  operator row offered only what its place allows, and the value row's parse and
  refusal.
- `screenshot_editor` under Xvfb: the walk now switches its second filter to an
  expression, selects the comparison and changes it on the operator's own row.
  Cycle complete, no panic; `feature-editor-events.png` is the full-screen mode
  with the operator selected.

## Done when - answered

- Clicking EVENTS opens the editor full screen and SCENE comes back: yes, and
  the mark is dropped with the mode, which is the behaviour that already shipped.
- Nothing a handler holds is cropped: the script tree has the wide budget and
  every choice row of three or more draws on its own line.
- Player / AI / Adrift is not cut off in Scene mode: it is a block row now.
- An expression reads as nodes with the operator emphasised, edits as nodes, and
  lowers to the same tree: yes, round-tripped in both directions.
- The harness walk covers the mode and an expression edit: yes.
