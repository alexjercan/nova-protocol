# Close the audio review's architectural leftovers

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.13.0,audio,hud,refactor,perf

Split out of the review of `20260824-125955` and `20260824-125947` on
2026-09-02, at owner direction. The review fixed 21 defects; these are
the items it deliberately left because they are design calls rather than
bugs. The owner has now accepted the recommended fix for each, so this
task is execution, not exploration.

Ordered by value for effort. Take them in this order; each is
independent and each is its own commit.

## 1. Pause a capped sink instead of silencing it

`MAX_EXTERIOR_LOOP_VOICES` is 8. In a crowded scene ~200 exterior loops
exist, the cap keeps the loudest 8, and the other ~192 get `gain 0.0`
while their rodio sinks stay open and keep mixing silence every audio
callback. That is the whole of the remaining underrun noise: the probe's
`log_clean` check still FAILS at 65 lines (down from 137 before the loop
churn fix).

Fix: in `drive_sfx_voices` (`crates/nova_gameplay/src/audio/voice.rs`),
where the silenced branch writes `gain = 0.0`, also `sink.pause()`, and
`sink.play()` on the way back. Guard both with `is_paused()` so it is not
a redundant call every frame. The machinery is already proven -
`pause_world_voices` / `resume_world_voices` do exactly this on the pause
overlay.

Two things the implementation must handle:

- A resumed loop restarts where it paused, not where it would have been.
  Probably inaudible on a thruster hum; check a tonal loop before
  accepting it.
- The cap is a rank, so the boundary voice can flicker in and out frame
  to frame. Add a small hysteresis band around the cap rather than
  pausing and resuming on a tie.

Done when `log_clean` passes on the torpedo stress probe.

## 2. Cache the loop handle instead of resolving it every frame

`AssetRef::resolve` calls `asset_server.load(path)`. That is idempotent -
the file is NOT re-read - but it still parses the path string, takes the
`AssetServer` lock, hashes and looks up, once per source per frame. Three
sites: `crates/nova_ship/src/ship_audio/loops.rs:84`, `:136`, `:209`.
With ~200 loops that is 200 string parses and 200 lock acquisitions a
frame to arrive at a handle that has not changed since spawn.

Fix: resolve once and cache the handle on the component.

## 3. The one-frame emitter desync

`AudioSystems` runs in `PostUpdate` `.before(TransformSystems::Propagate)`.
That ordering is correct and deliberate - bevy's own audio playback runs
AFTER Propagate, so placing and levelling first means no cue is ever
heard at the wrong volume for a frame. The cost is that every
`GlobalTransform` the pass reads is last frame's.

The inaudible half: a `Follow(entity)` voice trails its emitter by one
frame (0.5 u on a 32 u/s torpedo at 60 fps).

The AUDIBLE half: a newly spawned emitter has not been propagated at all,
so its `GlobalTransform` is still identity and its loop is placed at the
WORLD ORIGIN for its first frame. A torpedo lighting its drive 3 km out
gets one frame of full-volume, centre-panned hum.

Fix: seed a spawning emitter's `GlobalTransform` at spawn so the first
frame is right. (The alternative - splitting the pass so placement reads
after Propagate while levelling stays before it - is more invasive and
buys only the inaudible half.)

Second, quieter issue in the same place: `drive_sfx_voices` takes
`&mut GlobalTransform` and writes the emitter pose directly
(`voice.rs:339`). That works ONLY because a voice entity has a
`GlobalTransform` and no `Transform`, so Propagate skips it. Give a voice
entity a `Transform` - directly, or through a bevy `#[require]` on some
component added later - and Propagate silently overwrites the placement
every frame. Add a test that pins the invariant.

## 4. `SectionClass::is_weapon()`

The same three-variant match is written out verbatim three times in
`nova_os_ui`: `ship/app.rs:273`, `ship/app.rs:350`,
`ship/sections.rs:467`.

This is exactly the shape that produced five of the six railgun review
findings - a kind enumerated by hand in places nobody remembers. A fourth
weapon kind needs all three edited and nothing catches a miss.

Fix: one method on `SectionClass`, three call sites deleted.

## 5. Three missing system sets

`ShipAudioPlugin` adds five one-shot cue systems to `Update` with no set
and no ordering (`crates/nova_ship/src/ship_audio/mod.rs:204-218`):
`play_lock_cues`, `play_safety_engaged_cue`, `play_dry_fire_cue`,
`play_threat_lock_cue`, `play_hull_warning_cue`. The menu cues and the
editor cues have the same gap.

The correctness case is defensible today and the code says why - these
fire on messages whose writers are already pause-gated. The costs are
that nothing outside can order against them, they cannot inherit a run
condition as a group, and the repo's own `<Subsystem>Systems` convention
is broken in three plugins.

Fix: add `ShipAudioSystems`, `MenuCueSystems` and `EditorCueSystems`, and
state their cross-plugin ordering explicitly. Mechanical.

NOTE for whoever picks this up: this does NOT fix item 3. The desync is
caused by where `AudioSystems` sits relative to `Propagate`, not by the
missing sets. Adding the sets is the right shape and makes the item 3
fix expressible; it changes no behavior on its own.

## 6. Document the seven authored sound fields, and the lance

Seven authored sound fields have no creator documentation at all, so a
modder cannot discover them without reading Rust: `door_sound`,
`ammo_dry_sound`, `warn_hull_sound`, `warn_lock_sound`,
`stow_open_sound`, `stow_close_sound`, `collapse_sound`.

The railgun got its `/create` chapter in the review (c07eaa1e).
`/wiki` still has no page telling a player what the lance does, and
`/dev` has one table row.

## 7. HUD leftovers from the first review pass

**Bore-sight occlusion and contrast.** The sight is a real 3D cylinder,
`AlphaMode::Blend`, `unlit: true`, ordinary depth testing
(`crates/nova_hud/src/bore_sight.rs:192-199`). It is occluded by geometry
it passes behind, so aiming past your own hull loses the line. Contrast
is fixed (`LINE_ALPHA 0.5`, `MARK_ALPHA 0.85`) whatever the background,
so it washes out on a bright skybox and glares on empty space. Fixes are
cheap - a depth bias, or a screen-space overlay pass - but each changes
the LOOK, so bring an option to the owner rather than picking one.

**The one-pip reload gauge.** The shipped lance is `ammo_capacity: 1`, so
its gauge is a single pip. Its alpha ramps with reload progress and that
is the only signal for a TWELVE second wait - the weakest possible
readout for the longest cadence in the game. Knowing when the shot comes
back is most of what the lance feels like.

**Kill-ring respawn churn.** `sync_bore_sight` reconciles kill marks by
list INDEX (`bore_sight.rs:453-470`). As the nose sweeps, the traced hit
count changes every frame, so tail marks despawn and respawn
continuously - each a real entity with `Mesh3d` and `MeshMaterial3d`, so
each is an archetype move plus a render-world sync. Capped at 24 per
lance, so not catastrophic, but it is per-frame entity churn in the
render path. Fix: key marks by hit identity rather than index, or pool a
fixed 24 and toggle visibility.

**`RoundBitten` 8 vs 32.** `BITE_MEMORY` is 8 (colliders remembered
ACROSS steps), `MAX_BITES_PER_STEP` is 32 (resolvable WITHIN one step).
The split is deliberate and documented, and no case breaks today: a lance
slug covers 23 u per fixed step, so it exits any ship entirely with
nothing still overlapping at the boundary. This is a MISSING INVARIANT,
not a live bug - nothing pins that 8 stays above the layers a round can
still be inside, so a slower round through thinner plating could quietly
go wrong later. A test asserting the relationship is the whole fix.
Lowest priority in this task.

## Deliberately NOT in this task

The 21 cue-volume constants in `crates/nova_gameplay/src/audio/mod.rs:222-285`
are used from `nova_menu`, `nova_editor`, `nova_os_ui` and
`nova_scenario`, so the engine crate names its own consumers. Real
layering violation, and the module docstring concedes it. But the
argument for keeping them together is also real: one file to open when
balancing the mix by ear, which is how this pass was tuned. Reviewed and
left on purpose - revisit when a fifth consumer appears.

Also not here: no global one-shot voice budget. The per-area-cell
throttle covers the known worst case (a blast striking forty colliders)
and one-shots are short-lived.
