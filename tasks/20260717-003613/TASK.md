# Diegetic HP v1: per-section mesh damage tint/glow + retire the generic health bar

- STATUS: CLOSED
- PRIORITY: 31
- TAGS: v0.7.0,hud,ui,spike

## Goal

Make the player ship its own health readout: grade each section's rendered
material by its integrity (bcs `Health` current/max) so a battered flank
visibly reddens/darkens while healthy sections stay clean and dead ones read
burnt, and retire the generic screen-space `HealthDisplay` bar for the player
ship in the same change. This is the primary diegetic channel; the exact
aggregate number is the separate chip task 20260717-003620.

## Steps

- [x] Add a new module `crates/nova_gameplay/src/sections/damage_tint.rs` with
  a `SectionDamageTintPlugin`, registered by `SpaceshipSectionPlugin` gated on
  its `render` flag (verified: sections only render when that flag is set).
- [x] Component `SectionDamageTint` on each section render mesh entity, storing
  the cloned `Handle<StandardMaterial>` plus the pristine `base_color` and
  `emissive` captured before any grading, so grading is reversible and
  per-section.
- [x] Capture system. IMPLEMENTED via `Added<MeshMaterial3d<StandardMaterial>>`
  rather than a `WorldAssetRoot` scene-ready hook: `Added` fires the frame any
  mesh (sync cuboid OR async-instantiated gltf node) gains a material, so it
  needs no scene-ready signal (and `WorldAssetRoot`'s definition was not on disk
  in the pinned bcs checkout - the observable-state hook sidesteps that
  entirely). For each new mesh: walk up `ChildOf` to the owning `SectionMarker`,
  confirm its root is a player ship, clone the material into a fresh unique
  handle (gltf materials are shared across every instance of the same
  `*.glb#Scene0`, so per-section cloning is mandatory), swap the entity onto the
  clone, and record pristine colours in `SectionDamageTint`. One code path
  covers cuboid and gltf.
- [x] Grading system (`grade_section_tints`, chained right after capture): for
  each captured mesh read its section's `Health` ratio and write the clone's
  `base_color` (redden) plus a brightness drop (darken, colour-blind-safe) and a
  rising red `emissive` glow. `SectionInactiveMarker` or ratio 0 -> dark burnt.
  Destroyed sections already detach via `explode.rs`. (Reads `Health` each frame;
  no strict ordering vs `aggregate_ship_health` needed - a 1-frame lag is
  invisible and grading only reads.)
- [x] Ramp as named constants at the top of the module (WARN_BELOW, GLOW_BELOW,
  MAX_REDDEN, MAX_DARKEN, colours). Linear on `current/max`; pulse-under-fire and
  threshold tuning deferred (noted in NOTES.md).
- [x] Scope guard: capture resolves the section's root via `ChildOf` and requires
  `PlayerSpaceshipMarker`, so only player-ship sections are ever tinted. Enemy
  tint deferred (0-HP-ghost bug 20260716-162701).
- [x] Retire the generic bar for the player ship: delete the `health_display`
  spawn in `hud/mod.rs::setup_hud_health` (mod.rs:519-541). Then verify no
  other call site spawns `health_display`; if none remain, also remove
  `remove_hud_health` (mod.rs:543+), the `setup_hud_health`/`remove_hud_health`
  observer registrations (mod.rs:214-215), and the `HealthDisplayPlugin`
  registration (mod.rs:162) and its system-ordering entry (mod.rs:197). Do NOT
  remove `HealthDisplay` from bevy_common_systems - it stays available for other
  games and non-player entities; Nova just stops spawning it for the ship.
- [x] Update `web/src/wiki/dev/architecture.md` promotion note: the generic
  health/status bar stays generic in bevy_common_systems; Nova's player-ship
  readout is diegetic + local, so NOT a promotion candidate.
- [x] Test: unit tests pin the `damage_look` ramp; an end-to-end ECS test
  (cuboid path) spawns a player root + section + mesh, damages via `Health`, and
  asserts the private material reddens/darkens while the shared source material
  stays pristine. `cargo test -p nova_gameplay damage_tint`: 3 passed.
- [ ] Visual verify: run the base scenario (gltf ship), damage a section in
  combat, and confirm the tint reads on the real gltf meshes at combat framing.
  NOT DONE - not runnable headless in this job; needs a human playtest (this is
  the spike's camera-legibility open question). See NOTES.md.
- [x] Append a one-line entry to the spike's Fix record
  (`tasks/20260711-202901/SPIKE.md`) when this lands.

## Notes

- Spike: `tasks/20260711-202901/SPIKE.md` (Option 1, recommended).
- KEY DISCOVERY (corrects the spike's v1 framing): the shipped player ship
  renders via gltf `WorldAssetRoot` scenes, NOT default cuboids -
  `assets/base/sections/base.content.ron` sets `render_mesh:
  Some("gltf/hull-01.glb#Scene0")` for hull (lines 11, 98), turret meshes
  (58/64/70, 119/125/131) and torpedo bay (159). The cuboid material-swap path
  the spike described is only a fallback the base ship never hits, so the real
  mechanism is per-section gltf material cloning + grading. This is more work
  and carries real risk (async scene readiness, shared-material cloning); it is
  the headline of this task.
- Verified facts:
  - Sections carry bcs `Health {current,max}`; each is a direct child of the
    ship root; `integrity/glue.rs::aggregate_ship_health` (glue.rs:130-186) sums
    living sections into the root every frame (the number the old bar read).
  - Render children carry `SectionRenderOf(section)` and either
    `WorldAssetRoot(scene)` (gltf) or `Mesh3d`+`MeshMaterial3d` (cuboid) across
    all five kinds: hull, thruster, controller, turret, torpedo
    (`sections/*.rs`).
  - Section death is already visualized: `SectionInactiveMarker` on disable
    (glue.rs:26-49), debris + despawn on destroy (`integrity/explode.rs`). The
    damaged-but-alive gradient is the gap this task fills.
  - There is no `hud/health.rs` today; the bar is spawned straight from bcs
    `health_display` in `hud/mod.rs`. The architecture doc's "promotion
    candidate" is about the (future) extracted widget.
  - Bevy 0.19 (`Cargo.toml:29`); `StandardMaterial` has `base_color` +
    `emissive`.
- The tint is a world mesh, so the `HudVisibility`/`HudTier` UI system does not
  hide it; a cinematic-hide toggle is out of scope for v1.
- Related: `crates/nova_gameplay/src/{integrity,sections,hud}/`, sibling
  chip-language spike `tasks/20260710-234019/SPIKE.md`.
- Depends on: nothing (20260717-003620, the numeric chip, is independent).

## Close record

Landed the diegetic per-section health tint and retired the generic bar. Full
design + reasoning in `NOTES.md`; the headline:

- `sections::damage_tint` clones each player-ship section mesh's material on
  `Added<MeshMaterial3d>` and grades the clone by the section's `Health`
  (redden + darken + red glow; burnt when disabled/dead).
- Removed the `HealthDisplay` spawn and its observers/plugin wiring from
  `hud/mod.rs`; `HealthDisplay` stays in bcs, unused by Nova's ship.
- `web/src/wiki/dev/architecture.md` promotion note updated.

Verification: `cargo test -p nova_gameplay damage_tint` (3 passed, incl. an
end-to-end ECS grading test that also proves the shared source material is never
mutated); `cargo check --workspace` green.

What went differently vs the plan: the plan assumed a cuboid colour-swap could be
v1 and gltf was a later overlay. Reading the content `.ron` showed the shipped
ship is entirely gltf `WorldAssetRoot` meshes, so gltf material cloning was the
actual v1 mechanism from the start. The scene-ready hook the plan flagged as
verify-first turned out unnecessary: `Added<MeshMaterial3d>` fires when the async
gltf mesh appears, sidestepping `WorldAssetRoot`'s internals (which were not even
present in the pinned bcs checkout on disk).

Not done: on-screen legibility playtest on the real gltf ship (needs a human; not
runnable headless). Left unchecked above and flagged in NOTES.md - it is the
spike's camera-legibility open question and should be confirmed before the ramp is
considered tuned.

Self-reflection: grounding in the content data before coding is what turned a
wrong-mechanism plan into a right one; and preferring an observable-state ECS hook
(`Added`) over chasing a library-internal signal saved the capture system. Next
time, check the shipped content assets during planning, not implementation.
