# Fix insert_spaceship_sections editor

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.3.1,refactor


Should not spawn full spaceship, only visual config preview. Legacy #124.

## Investigation (deferred - needs runtime verification)

Left OPEN deliberately. The fix depends on editor/scenario observer interplay and editor
interaction (picking), which cannot be validated from a compile check.

Current situation:
- `insert_spaceship_sections` lives in nova_scenario (objects/spaceship.rs) as an
  observer on `On<Add, SpaceshipRootMarker>`. It reads `SpaceshipSectionsConfig` +
  `SpaceshipController` and spawns full, functional sections (collider + health + physics
  + input bindings) - correct for the simulation.
- The editor (nova_editor, create_new_spaceship / on_click_spaceship_section) ALSO spawns
  a `SpaceshipRootMarker` entity and manually adds section children (base_section, which
  is full collider+health), building a PlayerSpaceshipConfig as you click.
- So the same `SpaceshipRootMarker` trigger drives both the scenario's full-ship builder
  and the editor's manual section spawning; the editor's "ship" ends up a full functional
  spaceship rather than a lightweight visual/config preview.

The task wants the editor's representation to be a visual config preview (what the config
will look like) and not a live combat ship. Open questions that need the running editor to
answer:
- The editor needs colliders for picking (clicking sections to place them), so "visual
  only" cannot drop colliders entirely - it likely means: no Health, no RigidBody::Dynamic,
  no combat/input wiring; keep a pickable static visual.
- How to stop the scenario `insert_spaceship_sections` observer from acting on the
  editor's SpaceshipRootMarker - e.g. an editor-only marker or a preview variant of the
  root, or gating the observer on scenario state.

Recommended approach (for a runtime-capable session):
1. Give the editor preview its own marker (e.g. SpaceshipPreviewMarker) distinct from the
   scenario SpaceshipRootMarker, or gate insert_spaceship_sections on being in a scenario.
2. Spawn preview sections from a lightweight bundle (render + pickable collider, no Health/
   RigidBody/input), driven by the PlayerSpaceshipConfig being edited.
3. On "continue to simulation", build the real spaceship from the config (already done via
   test_scenario/PlayerSpaceshipConfig).
4. Runtime-verify: editor picking/placement still works, no phantom physics in the editor,
   and the simulation ship still spawns correctly from the edited config.

Compile-only verification is insufficient here, so it was not landed on the cleanup branch.
