# Editor UX: hierarchy tree, context actions, focused editing

- STATUS: IN_PROGRESS
- PRIORITY: 82
- TAGS: v0.12.0,editor,ui

Child of the v0.12.0 editor epic (`20260812-131912`). The editor UI panels
the foundations task (`20260824-120520`) noted were owned by no task, plus
the interaction polish the owner asked for on 2026-08-25. First half of the
editor UX pass; the engineer readout (`20260824-120535`) stays its own task
and lands inside the layout this one builds.

## Goal

The WIP furniture becomes an editor that reads like one: a real hierarchy
tree, actions split per context, and an edit context you can see.

## Decisions (owner, 2026-08-25)

- Single click in the hierarchy is the gesture: a ship row enters, the
  scenario row leaves, a section row selects. Double-click dies.
- The hierarchy is a TREE: scenario root, ships nested, the entered ship's
  sections nested under it, ASCII connectors and per-kind glyphs.
- Entering a node FOCUSES it: the viewport shows only the entered ship.
  The scenario context shows everything.
- Actions split per context in a top bar: scenario = Add Ship + Play,
  ship = Parts + Delete + Rebind (acts on the selected section). The rail
  keeps the tree; ship settings (skin, look, attitude) show only inside a
  ship.
- Add Ship creates a BLANK ship; the first armed part founds it at the
  ship origin. The seeded V1/V2 buttons die.
- With no part armed, a world click SELECTS: a section inside a ship, a
  ship at the scenario node. Entering by world click dies; the hierarchy
  is the door. Rebind moves from click-on-section to the top-bar action.
- One transform gesture now: dragging a ship at the scenario node moves
  it on the ground plane, to find how the rest will look.

## Out of scope (second half or later)

- Node settings and the section inspector; enter at section level.
- Non-ship object kinds and the add-object menu beyond the one Ship entry.
- Rotation and scale gizmos; only the plane drag lands now.
- Converting the sandbox scene into the editor.

## Landing order

1. The hierarchy tree, single-click enter and select.
2. Top bar, per-context panels, blank Add Ship, world select, Rebind.
3. Focus isolation, ship dragging, live-range coverage.

## Done when

- The tree shows scenario, ships and sections with the entered branch
  open, and one click enters, leaves or selects.
- Inside a ship only that ship is visible; at the scenario node all are.
- Add Ship starts blank and the first part founds it at the origin.
- The driven editor range covers: found a blank ship, select in the world,
  enter via the tree, isolation, and a plane drag. It stays green.
