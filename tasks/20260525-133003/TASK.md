# Improve on_destroy handling

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: v0.4.0, health

Delay despawn so systems can react before the entity is gone. Legacy #129.

## Resolution (CLOSED - already done, 2026-07-08)

Closed as obsolete: the current destruction pipeline already delays despawn exactly as
asked. It was built this way when integrity was promoted to bevy_common_systems, so this
legacy TODO was silently resolved.

Death never despawns directly. It inserts markers and lets the game react:

1. Health hits zero -> `IntegrityDisabledMarker` (bcs `on_health_depleted_insert_disabled`).
2. A disabled leaf/root -> `IntegrityDestroyMarker` (bcs `handle_destroy` /
   `handle_chain_destroy` / `handle_parent_destroy`). This marker is the deliberate delayed
   seam - bcs prunes the graph but does NOT despawn (see plugin.rs module doc: "the game
   observes that marker to explode, spawn debris, or despawn").
3. With the entity still alive, observers react to `On<Add, IntegrityDestroyMarker>`:
   - `on_destroyed_entity` (explode.rs) fires `OnDestroyedEvent` -> scenario scoring, player
     death / asteroid-destroyed messages.
   - `on_explode_entity` slices the mesh into debris fragments.
   - `despawn_destroyed_without_mesh` handles meshless sections.
4. Only then does despawn happen, and it is a deferred command (`try_despawn`), so every
   `On<Add>` observer runs against a live entity by construction.

The remaining raw `despawn` calls (loader.rs scenario unload, torpedo/projectile.rs torpedo
expiry) are unrelated lifecycles, not the health/destroy path #129 referred to.
