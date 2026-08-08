# Fix insert_spaceship_sections editor

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

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

## Steps

- [x] Give the editor preview its own root marker (`SpaceshipPreviewMarker`), distinct from
      the gameplay `SpaceshipRootMarker`, so `insert_spaceship_sections` and the integrity/
      health systems never act on it.
- [x] Add a lightweight `preview_section` bundle (SectionMarker + pickable Collider +
      Visibility, no Health/ColliderDensity/ExplodableEntity) in nova_gameplay.
- [x] Spawn editor sections with `preview_section` instead of `base_section`; drop
      `RigidBody` from the preview root.
- [x] Runtime-verify: editor picking/placement still works, no phantom physics/health in
      the editor, and the simulation ship still spawns correctly from the edited config.

## Implementation notes

Root cause of "full functional spaceship": the editor spawned its preview root with
`SpaceshipRootMarker` + `RigidBody::Dynamic` and built its sections from `base_section`
(Collider + Health + ColliderDensity + ExplodableEntity). That made avian link the section
colliders to the root (`ColliderOf`), which drove the whole integrity pipeline in the
editor - graph construction, collision events, and `aggregate_ship_health` - i.e. a live
combat ship sitting in the editor.

Fix keys on one avian detail (confirmed by reading avian 0.7
`collider_hierarchy/plugin.rs` and `collider_tree/update.rs`): a `Collider` with no
`RigidBody` ancestor is never given a `ColliderOf`, but it still lives in the standalone
spatial-query tree, so it remains pickable via `PhysicsPickingPlugin`. So the preview ship
needs colliders (for picking) but no rigid body anywhere:

- `SpaceshipPreviewMarker` (nova_editor) replaces `SpaceshipRootMarker` on the preview root,
  and the root no longer carries `RigidBody`/`SpaceshipSectionsConfig`/`SpaceshipController`.
  `insert_spaceship_sections` (keyed on `SpaceshipRootMarker`) therefore never fires for the
  editor ship.
- `preview_section` (nova_gameplay, sibling of `base_section`) provides `SectionMarker` +
  `Collider::cuboid(1,1,1)` + `Visibility`, but none of `destructible_body`'s
  Health/ColliderDensity/ExplodableEntity. The kind-specific `*_section` bundle (still
  inserted alongside) supplies the gltf visual.

With no `RigidBody` in the preview hierarchy, no `ColliderOf` is linked, so
`on_collider_build_integrity_graph` never runs for the preview, the root never gets an
`IntegrityGraph`, and `aggregate_ship_health` never touches it (which also avoids the trap
where it would have inserted `Health{0,0}` on a graph-bearing root with no healthy sections
and destroyed the editor ship). The picking observers still query `With<SectionMarker>`, so
placement is unchanged. The real player ship is unaffected: it is still built from
`PlayerSpaceshipConfig` via `test_scenario` -> `spaceship_scenario_object` ->
`insert_spaceship_sections` -> `base_section`.

The thruster/turret/torpedo input-binding components are still recorded on preview sections
as before; their systems are gated to the Scenario state, so they stay inert in the editor.
