# Diegetic HP v1 - design + implementation notes

Task: 20260717-003613. Spike: tasks/20260711-202901/SPIKE.md (Option 1).

## What shipped

- New module `crates/nova_gameplay/src/sections/damage_tint.rs`
  (`SectionDamageTintPlugin`), registered by `SpaceshipSectionPlugin` only when
  `render` is on. It makes the player ship its own health readout: each section's
  rendered mesh material is graded by that section's `Health`.
  - `capture_section_materials` keys on `Added<MeshMaterial3d<StandardMaterial>>`,
    walks up `ChildOf` to the owning `SectionMarker`, checks the section's root
    carries `PlayerSpaceshipMarker`, then clones that mesh's material into a
    private handle (recording the pristine `base_color`/`emissive` in a
    `SectionDamageTint` component) and swaps the mesh onto the clone.
  - `grade_section_tints` runs right after, reading each section's `Health` ratio
    and writing the clone's `base_color` (redden + darken) and `emissive` (rising
    red glow under 40%). `SectionInactiveMarker` or ratio 0 -> burnt/dark.
- Retired the generic bar for the player ship: removed the `HealthDisplay` spawn,
  the `setup_hud_health`/`remove_hud_health` observers and their registration, and
  the `HealthDisplayPlugin` registration + its `HealthDisplayPluginSystems::Sync`
  ordering from `crates/nova_gameplay/src/hud/mod.rs`. `HealthDisplay` remains in
  bevy_common_systems, unused by Nova's ship.
- Updated `web/src/wiki/dev/architecture.md`: the generic bar stays generic; the
  diegetic readout is game-specific and not a promotion candidate.

## The key discovery (why this differs from the spike's framing)

The spike proposed "v1 = default-cuboid colour swap, gltf gets an emissive
overlay later." Reading `assets/base/sections/base.content.ron` showed the
shipped player ship renders *every* section via gltf `WorldAssetRoot` scenes
(`gltf/hull-01.glb#Scene0`, the turret meshes, the torpedo bay) - the cuboid path
is only a fallback the base ship never hits. So a cuboid-only v1 would have tinted
nothing on the real ship. The mechanism had to handle gltf from the start.

Two consequences drove the design:

1. **Per-section material clones are mandatory.** A gltf scene's materials are
   shared `Handle<StandardMaterial>`s across every instance of the same mesh, so
   mutating one in place would tint every hull section identically. Each mesh gets
   a private clone instead. The end-to-end test asserts the shared source material
   is never mutated.
2. **No scene-ready signal needed.** Rather than hook `WorldAssetRoot`'s
   instantiation (its definition was not locatable in the pinned bcs source on
   disk), capture keys on `Added<MeshMaterial3d<StandardMaterial>>`, which fires
   the frame any mesh - sync cuboid or async gltf node - gains a material. Simpler
   and robust to async loading. `PlayerSpaceshipMarker` is inserted synchronously
   at ship spawn (`nova_scenario::objects::spaceship`), so it is reliably present
   before async gltf materials load; the player-ship gate is safe.

## Verification

- `cargo test -p nova_gameplay damage_tint`: 3 passed. Two unit tests pin the ramp
  (`damage_look`); one end-to-end ECS test spawns a player root + section + cuboid
  mesh, runs the systems, damages via `Health`, and asserts the private material
  reddens/darkens while the shared source stays pristine. The end-to-end test can
  fail: it caught nothing here, but it exercises capture + grade through real
  schedules.
- `cargo check --workspace`: green.

## Not done / follow-ups

- **On-screen playtest is still pending.** The automated test proves the grading
  logic and ECS wiring, but the actual on-ship *appearance* at combat framing (is
  the tint legible on the gltf meshes, is the ramp tuned right) needs a human
  running the base scenario - not runnable headless in this job. This is the
  spike's main open question (camera legibility) and should be a playtest note.
- Ramp tuning (thresholds, pulse-under-fire) left linear and deferred. When
  tuning, confirm the HDR emissive glow (`GLOW_PEAK`) reads as intended: it only
  "glows" with a bloom pass; without bloom it is a bright-red emissive (still a
  valid cue). Check bloom is enabled or tune the glow to read without it
  (review R1.3).
- Cinematic hide at `HudVisibility::None` intentionally out of scope (the tint is a
  world mesh, not a UI node).
- Non-player ships intentionally excluded in v1 (interacts with the 0-HP-ghost bug
  20260716-162701).
- Numeric hull chip is the separate backstop task 20260717-003620.

## Self-reflection

- Reading the content `.ron` early was what caught the gltf-vs-cuboid gap before a
  line of code was written from the wrong model - exactly the failure mode the plan
  skill warns about. Worth doing first every time.
- Time was spent hunting for `WorldAssetRoot`'s definition (not on disk in the
  pinned bcs checkout). The right move, taken eventually, was to stop needing it:
  `Added<MeshMaterial3d>` sidesteps the whole scene-readiness question. Reach for
  the observable-state hook before the library-internal one.
