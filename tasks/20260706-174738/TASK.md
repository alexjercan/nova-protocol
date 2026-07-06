# Sections disable but never destroy; ship does not die at zero health

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.3.1, bug, health

Reported in play: when a spaceship takes damage, some sections (e.g. the controller)
get *disabled* when their health hits zero, but they are never *destroyed* (not removed,
not exploded), and the ship as a whole never "dies" when its health is depleted. So the
destruction pipeline stalls at the disabled stage.

Expected: a section at zero health should be disabled and then destroyed (removed +
exploded via the leaf/chain rules), and when the whole ship is destroyed the ship dies
(player death handling fires, camera reverts, etc.).

Investigate the integrity pipeline (crates/nova_gameplay/src/integrity/): the
disabled -> leaf -> destroy chain (handle_destroy / handle_chain_destroy /
handle_parent_destroy), the IntegrityGraph construction, and the leaf-marker logic. Likely
a Bevy 0.19 behavioral change in an observer/marker or the graph not updating.

## Steps

- [x] Diagnose why sections disable but never destroy (runtime logging confirmed the
      IntegrityGraph was empty of sections).
- [x] Fix the graph construction so all sections + adjacency are captured.
- [x] Despawn destroyed sections that have no mesh to explode.
- [x] Make the ship die when its sections are gone.
- [x] Verify in-game.

## Resolution (CLOSED)

Three distinct bugs, found and fixed by running the game and adding targeted diagnostics
(compile checks could not have caught any of them):

1. IntegrityGraph construction ordering (crates/nova_gameplay/src/integrity/plugin.rs).
   The graph was built by two observers: `on_section_graph_create` on `Add<SectionMarker>`
   (spawn) and `on_rigidbody_graph_create` on `Add<ColliderOf>`. But avian adds `ColliderOf`
   *after* spawn, so the section builder ran before the graph existed and silently dropped
   every section - only the one collider that created the graph was in it. So no section was
   ever a leaf, and the disable -> leaf -> destroy chain never fired (sections only ever got
   `SectionInactiveMarker`). Replaced both with a single `on_collider_build_integrity_graph`
   keyed on `ColliderOf`, which rebuilds the whole graph (sections + unit-distance
   adjacency) once colliders are physics-linked. A runtime `DIAG` log confirmed the empty
   graph before the fix.

2. Destroyed meshless sections never despawned (integrity/explode.rs). Sections render via
   a gltf `WorldAssetRoot` and have no `Mesh3d`, so the mesh-explosion path (which owned the
   despawn) skipped them - they lingered at zero health, still colliding and working. Added
   `despawn_destroyed_without_mesh`.

3. The ship never died (integrity/plugin.rs). The root's aggregate health was summed once
   at spawn by an `On<Add, Health>` observer and never updated as sections took damage, so
   it stayed full. Replaced it with `aggregate_ship_health`, which recomputes the root's
   health each frame as the sum of its living sections. When it reaches zero (all sections
   gone) the root is marked disabled -> destroyed and, being meshless, is despawned by the
   fix from (2) - removing PlayerSpaceshipMarker, which reverts the camera to WASD and
   clears the HUDs.

Verified in-game: sections destroy and cascade, the health HUD drops as sections take
damage, and the ship dies (despawns, camera reverts to WASD) once all sections are gone.

Deliberately out of scope: a *visual* explosion for destroyed sections (they currently
despawn silently) - filed as v0.4.0 follow-up 20260706-182758, because it needs the gltf
submeshes sliced or a debris/particle burst, which is a separate effort.

Self-reflection: the root causes were all runtime-only (observer ordering vs avian's
ColliderOf timing, mesh-coupled despawn, once-only health sum). Adding one cheap `DIAG`
warn to confirm the empty-graph hypothesis before rewriting the construction was what made
the fix confident rather than a guess.
