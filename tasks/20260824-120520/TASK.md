# Node editor foundations: the scenario node tree

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.12.0,editor

Child of the v0.12.0 editor epic (`20260812-131912`), second in the spine
after `20260824-011329`. Research: `tasks/20260815-231945/EDITOR-STATE.md`
sections 1 and 3a. Design settled with the owner on 2026-08-24; the
decisions below replace this task's earlier "per-root build state" framing.

## Goal

The editor stops keeping the ship in a singleton resource and starts keeping
a NODE TREE. Everything is a node: a scenario node holds ship nodes, a ship
node holds section nodes. Each node owns its config as a component. Lowering
to `ScenarioConfig` is a query walk over the tree, run on Play and later on
save.

Nothing user-visible changes beyond "two ships can exist" and a WIP scene
list to reach them.

## The model (settled)

```
ScenarioNode                     persistent across ExampleStates
 |- ShipNode {name, skin, style, controller: Player | AI}
 |   |- SectionNode {prototype, transform, modifications, binds}
 |   |- SectionNode ...
 |- ShipNode ...

every node:  |- NodeView (mesh, collider, Pickable)   DespawnOnExit(Editor)
```

- Node entities are DATA and persist across the Editor -> Scenario -> Editor
  round trip. Only the view child is torn down and rebuilt, so a forgotten
  component cannot ride into the Scenario state.
- `NodeId` is a minted literal, `<prototype>_<n>` from a counter on the
  parent, unique within the parent. That is exactly what `input_mapping`
  (per hull) and `BaseScenarioObjectConfig.id` (per scenario) each need.
  Same convention as `20260824-120524`.
- Picking hits the view. Every pointer path maps view -> parent node once.
- `EditContext` is the node you are inside. It starts at the scenario node.
- Play is enabled only at the scenario node. Which ship flies is the ship
  node's `controller` field, not the context.
- Enter/exit switches the context, relaxes validation inside, and validates
  at the boundary on exit (Godot edit context + Cosmoteer blueprint mode).
  The boundary hook ships with NO rules in it: every rule worth having
  changes what Play accepts today, and that is a separate decision.
- Mating stays the rule inside a ship node; free placement is a
  scenario-node rule. Free-dragging parts would build hulls the runtime's
  integrity graph rejects.

## What is deleted, not moved

`PlayerSpaceshipConfig` (config.rs:13-33) goes away. With it:

- the re-key pass in `rebuild_editor_preview_on_enter`
  (placement.rs:266-317). Rebuilding becomes a plain spawn from node data.
- the sort in `sync_editor_skin` (skin.rs:109-116), which exists only
  because the map is entity-keyed.
- section ids as stringified live entities (placement.rs:155-158).

## The conversion checklist

Five `Single<... With<SpaceshipPreviewMarker>>` sites, not three:
placement.rs:505 (solver), :741 (commit observer), :835 (`draw_link_points`),
:929 (`draw_ship_heading`), skin.rs:73.

About fifteen systems take `PlayerSpaceshipConfig`: attitude.rs:98,
keybind.rs:201, ui/mod.rs (5 sites), scenario.rs:201/238/451, placement.rs
(4 sites), skin.rs:69.

## Inert by construction

Today the preview is inert by GATING: `insert_preview_section`
(preview.rs:42-67) inserts the live kind bundle, and it stays quiet only
because the preview root is not `SpaceshipRootMarker` (config.rs:48-54) and
nova_scenario gates the ship system sets on scenario-liveness
(lib.rs:326-330).

Split each of the five kind bundles in nova_ship into a visual half (render
mesh, mesh transform, class) and a behaviour half; `thruster_section` and
its siblings stay as both. The editor view takes the visual half only. This
is a split rather than a deletion because `ThrusterSectionRenderMesh`
(thruster_section.rs:113) lives in the same bundle as
`ThrusterSectionInput`.

Section input bindings move onto the section node's config, so the view no
longer needs `SpaceshipThrusterInputBinding` and its siblings for the
keybind chips.

## WIP UI

Plain text and buttons, no styling pass. Enough to drive the model by hand
and to test it:

- a Scene block on the rail listing the nodes in the current context
- click selects, double-click enters, an Up row exits
- New Ship creates a ship node in the scenario node
- the Play button is disabled outside the scenario node

The real hierarchy panel, node overview and section inspector are follow-up
work.

## Landing order

1. nova_ship: split the kind bundles, and point the editor's view spawner at
   the render halves. Self-contained - the preview is inert by construction
   from here on and nothing else moves yet.
2. nova_editor: the node module, the view child, `EditContext`, lowering as
   a query walk, every caller converted. Large and indivisible - the crate
   does not compile between deleting the resource and converting its
   readers, and a bridge type is the compatibility machinery AGENTS.md
   rules out. Pulls the ship's geometry onto `EditorProbe` forward from
   step 3: the harness read section poses off the scene, and the thing
   carrying `SectionMarker` is now a view whose own transform is identity,
   so the range cannot stay green without it.
3. WIP UI, the remaining probe fields, tests.

## Done when

- Two ship nodes can be edited in one session. Entering one scopes every
  editor system to it; exiting returns to the scenario node.
- Section ids survive exit and re-entry, and are entity-independent.
- Play from the scenario node lowers the tree and flies it. No
  byte-equivalence bar against today's hand-off and no regression test
  pinning one: the range you fly is whatever backdrop the document was
  started from.
- The existing editor ranges stay green (state waits from
  `20260824-011329`, not SETTLE).

## Out of scope

- Affordance overlays: thrust direction, turret arcs, the section inspector.
- Non-ship node kinds and their preview spawners (`20260714-081703`).
- Save and load to disk, prefab stamping (`20260824-120524`). Lowering emits
  the `Player`-driven ship only; ships built beside it are designs, and
  stamping a design into a scenario as an instance is that task's job.
- The sandbox range stays code-authored constants in `sandbox_scenario`
  (scenario.rs:34-196), supplied at lowering exactly as today. It is the
  fixed backdrop until templates land, at which point "start from the
  sandbox range" is the template that supplies it.

## Proof

Landed as `af046782` (the nova_ship split), `1cf5c826` (the node tree),
`3e5b7abc` (two review defects) and the WIP UI commit that closes this.
Quick Review approved the first two ranges.

The driven editor range proves the Done-when, in its own words:

- `editor: leaving a ship of 8 sections: ["basic_controller_section_1",
  "pdc_kinetic_turret_section_7", "reinforced_hull_section_2", ...]`
- `editor: inside 'ship_2', and the first ship's 8 sections are elsewhere`
- `editor: back at the scenario node, listing ["ship_1", "ship_2"]`
- `editor: re-entered the first ship on the same ids: [...]` - byte-identical
  to the stamp above, which is the entity-independence claim
- `editor: the finished ship derives 7 mates over 8 sections` then
  `editor: the flown ship carries the same 7 mates`, in 26 plates
- `autopilot: cycle complete, no panic (t=5.1s)`, EXIT=0

The ids are non-contiguous (`_1,_2,_4..._9`) because the run deletes a section
part-way through and the ordinal counter never reuses. That is the design, and
the log is where it shows.

96 unit tests in `nova_editor`; `cargo check --workspace --all-targets` clean.

## Follow-ups this did not do

- Double-click-to-enter is covered by unit tests, not by the live range: the
  gesture is a wall-clock threshold, and asserting one against lavapipe frame
  times would be flaky for no coverage. The range enters by clicking the ship
  instead, which is the path a builder actually uses.
- A preview section carries no behaviour, but a generated child tree (a torpedo
  bay's spawner) is still inert by GATING rather than by construction. Left to
  whichever task next touches those entities.
- New Ship no longer carries the cladding toggle onto the new ship; skin is a
  property of a ship now, and a new one starts bare. Flagged in review, not
  answered.

## Owner notes, not scoped anywhere yet

- Scenario TEMPLATES: a gallery of starting points, the sandbox range being
  the first one. A template is a `Content::Scenario` in the same bundle
  format `20260824-120524` defines, so it extends that task's gallery
  rather than this one.
- The editor UI panels (hierarchy, node overview, section inspector) are
  assumed by every task in the spine and owned by none.
