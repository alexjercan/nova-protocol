# Turret: manual free-aim while holding CTRL

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,feature

## Goal

Allow moving the turret manually in manual mode by holding CTRL. While CTRL
is held the turret should ignore the current target lock and just follow the
mouse cursor (free aim), instead of sticking to / snapping back to the lock.

Currently pressing CTRL keeps the turret glued to the lock. The desired
behavior: hold CTRL -> turret detaches from lock and tracks the mouse; release
CTRL -> normal locked/manual behavior resumes.

## Steps

- [x] In `update_turret_target_input` (input/player.rs:338 - the player's turret
  aim feed) add a `keys: Res<ButtonInput<KeyCode>>` param and compute
  `free_aim = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)`
  (both CTRL keys, matching the existing ship-lock-cycle modifier binding
  player.rs:590).
- [x] Change the tier pick (player.rs:404) from
  `component_tier.or(lock_tier).unwrap_or(ray_tier)` to: when `free_aim`, use
  `ray_tier` directly (the camera crosshair = where the mouse points, velocity 0);
  otherwise the existing `component_tier.or(lock_tier).unwrap_or(ray_tier)`.
  Comment the behavior and the interaction note below.
- [x] The system now requires `ButtonInput<KeyCode>`: init it in the existing
  turret-feed tests so they keep running with default (no CTRL) behavior -
  `turret_feed_world()` (player.rs:1710) and
  `turret_aim_ray_bases_on_the_live_structure_anchor` (player.rs:1664). Default =
  no keys pressed = unchanged assertions.
- [x] Add a test `holding_ctrl_free_aims_at_the_camera_ray_over_the_lock`: build
  `turret_feed_world()` (has a ship lock), also set a component lock (the
  strongest tier), press `ControlLeft`, run the feed, and assert the turret aims
  at the camera ray point `(0,0,-100)` with zero velocity - NOT the locked
  section `(1,0.5,-199)`. Then release CTRL, re-run, assert it snaps back to the
  component-lock point. Falsifiable: fails if CTRL is ignored.
- [x] Verify `cargo check --workspace --all-targets` + the player input tests +
  `cargo fmt`. Add a CHANGELOG Unreleased line.

## Notes

- "Manual mode" = the player's turret feed (`update_turret_target_input`, which
  runs only for the player ship's turrets). There is no separate turret auto/
  manual mode enum in the codebase; the turret normally AUTO-aims at the lock
  (three-tier feed: component -> ship -> camera ray), and this task adds the
  CTRL-held manual override that forces the camera-ray tier. AI turrets
  (input/ai.rs) and torpedoes are unaffected.
- The camera is mouse-look (crosshair = screen center), so the `ray_tier` forward
  IS where the mouse has aimed the view - "follow the mouse cursor" == the ray
  tier. Free aim uses zero target velocity, so no lead offset (correct for a
  hand-aimed shot).
- INTERACTION: CTRL is also the modifier for the CTRL+scroll ship-lock cycle
  (player.rs:515,590). With this change, holding CTRL to cycle ship locks also
  free-aims the turret until release. Acceptable for a first cut (you are looking
  around while choosing a lock); flagged for playtest - if unwanted, a later tweak
  can suppress free-aim mid-cycle or move one gesture to another modifier.
- Relevant: input/player.rs (`update_turret_target_input` :338, tier pick :404,
  tests :1663-1816); input/targeting.rs (the lock resources it reads); the
  turret follows `TurretSectionTargetInput` via `update_turret_aim_point`
  (sections/turret_section.rs).

## Implementation record

`update_turret_target_input` (input/player.rs) now reads `Res<ButtonInput<KeyCode>>`
and, when either CTRL is held, uses the camera-ray tier directly instead of
`component_tier.or(lock_tier).unwrap_or(ray_tier)` - so the turret free-aims at the
crosshair while CTRL is down and snaps back to the lock on release. Existing
turret-feed test rigs gained `init_resource::<ButtonInput<KeyCode>>()` (default =
no CTRL = unchanged); added `holding_ctrl_free_aims_at_the_camera_ray_over_the_lock`
(falsifiable: a component lock is set, yet CTRL forces the ray point).

Interaction (see Notes): CTRL is shared with the CTRL+scroll ship-lock cycle, so
holding CTRL to cycle also free-aims until release. Left as-is for playtest.

Verify: cargo check --workspace --all-targets clean; input::player tests 17/17
(incl. the new one) and turret tests green; cargo fmt clean.
