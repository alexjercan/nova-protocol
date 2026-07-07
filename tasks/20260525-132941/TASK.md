# Camera mode switching: WASD without spaceship, chase with spaceship

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.3.1, bug

When no spaceship is present, the camera should be WASD-controlled; with a spaceship it should be chase cam. Legacy #100.

## Steps

- [x] Trace how the scenario camera switches between WASD and chase.
- [x] Find why "no spaceship -> WASD" never happens after the ship dies.
- [x] Fix the destroyed-side observer to fire on the real "spaceship gone" signal.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

The scenario camera (nova_scenario/src/loader.rs) spawns with WASDCameraController and
switches to chase when the player ship appears. Two observers drove this:
- on_player_spaceship_spawned: On<Add, PlayerSpaceshipMarker> -> chase. Correct.
- on_player_spaceship_destroyed: On<Add, HealthZeroMarker> + `if add.entity != <player
  ship root> { return }` -> WASD. Broken: HealthZeroMarker is added to individual
  ship *sections*, never to the PlayerSpaceshipMarker root entity, so the guard always
  returned early and the camera never went back to WASD when the ship was destroyed.

Fix: rewrote on_player_spaceship_destroyed to trigger on `On<Remove,
PlayerSpaceshipMarker>` - the same "spaceship gone" signal the HUD cleanup observers in
nova_gameplay/src/hud/mod.rs already use. When the player ship entity is despawned the
marker is removed, the observer fires, and the camera swaps SpaceshipCameraController
for WASDCameraController. Dropped the now-unnecessary `spaceship` Single and the entity
guard.

Why this is correct: the marker's presence is exactly the "is there a player ship"
condition the task describes, and Remove fires whenever the ship goes away (death,
despawn, scenario unload) regardless of the internal destruction mechanism. If the
scenario camera has already been despawned (e.g. during unload), the camera Single
matches nothing and the observer is simply skipped - safe.

Verified: build --all-targets, clippy, fmt all green. Could not exercise at runtime
(no display), but the change aligns the camera with the established Add/Remove marker
pattern used elsewhere for the same entity.

Self-reflection: the fix came straight from spotting that the rest of the codebase
already had the right pattern (HUD Remove observers) - matching existing conventions
beat inventing a new presence-polling system.
