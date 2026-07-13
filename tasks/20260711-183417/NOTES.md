# Notes: thruster hum distance attenuation (20260711-183417)

## Mechanism

The hum had NO attenuation path and no per-ship attribution:
`update_thruster_loop_volume` drove one global, unpositioned loop entity from
the throttle averaged over EVERY active `ThrusterSectionMarker` in the world
(audio.rs, comment "per-ship attribution ... is left for when there is more
than one audible ship" - v0.5.0's AI ships made that deferral audible). The
one-shots attenuate in `play_positional`; the loop never called
`distance_attenuation` at all. Torpedo thrusters also matched the query, so a
flying torpedo raised the "engine hum" too.

## Diagnostic trace (real game)

Rig (exact, per record-the-exact-rig): the shipped app (`cargo run
--features debug`, i.e. `src/main.rs` `editor_app`), no autopilot, under
`Xvfb :99`, left sitting on the MAIN MENU for ~4 minutes so the menu
ambience scene runs: an AI ship (controller + hull + one thruster,
`AIControllerConfig { orbit: menu_planetoid }`) flying a thruster-driven
orbit, watched by the fixed cinematic menu camera (which carries
`SfxListenerMarker` via the scenario loader). A temporary `info!` trace in
the hum system logged the throttle, the volume, and the burning thruster's
distance to the listener each frame. This is the reported scenario in
miniature - a distant ship burning while the listener is far away.

Dead ends on the way to this rig, so the next session skips them: the
harnessed examples cannot show the bug - 13_menu_newgame's autopilot clicks
New Game ~1 s into the menu (before the ambience ship ever burns, and the
shakedown start has cold engines), and 06_torpedo_range's 6 s window ends
before its script fires a torpedo whose thruster would register. The plain
app sitting on the menu is the reliable burning-ship-at-range scene.

## Fix

Split the system in two (the `AudioSink` write cannot be tested headless -
no audio device in tests):

- `compute_thruster_hum_volume` groups active thrusters by hum source - the
  `SpaceshipRootMarker` ancestor (walk via `ChildOf`, mirroring
  `local_pose_in_root`), or the thruster itself when there is no such root
  (torpedo thrusters hang off the projectile; bare rigs) - averages throttle
  per source, scales by `distance_attenuation(listener -> source pose)`, and
  takes the LOUDEST source as the target (summing would stack distinct ships
  past the per-ship ceiling; the old code's average-everything predated
  multiple ships). Target + smoothed live in a new `ThrusterHumVolume`
  resource.
- `apply_thruster_loop_volume` copies the smoothed value onto the sink.

Player exemption, decided from the camera rig constants rather than a runtime
measurement (camera_controller.rs): `mode_camera_rig` offsets are 20.6-31.6 u
(Normal |(0,5,-20)|, FreeLook |(0,10,-30)|) - already past
`SFX_NEAR_DISTANCE` = 20 - plus `BURN_PUSH_DISTANCE` = 3 under burn, and the
orbit survey dolly stretches the rig up to `SURVEY_MAX_DISTANCE` = 250 u,
deep inside the 20..320 rolloff band. Without the exemption the fix would
fade the player's own engines whenever the camera pulls back; with it, the
player's ship always contributes at full throttle gain.

## Behavior changes beyond the bug

- Menu ambience: the backdrop ship's hum is now attenuated by its distance to
  the cinematic camera (~1.6x-3.6x the orbit radius as it circles), so the
  menu hum fades in and out with the orbit instead of droning at full
  throttle gain. Physically consistent - flagged for the user's next
  playtest in case the menu should get a dedicated ambience bed instead.
- Torpedo thrusters now attenuate at their own position (they used to feed
  the global average from any distance).
- Pre-sink smoothing (review R1.3): the old code froze `smoothed` until the
  AudioSink existed, so a hot-engine scene faded up from silence when the
  sink appeared; the compute half now advances `smoothed` unconditionally,
  so the first sink write lands at the caught-up level. Deliberate: those
  first frames have nothing to fade from, and a correct level beats a late
  ramp. Documented on `apply_thruster_loop_volume`.

## Alternatives considered

- Per-ship loop entities (one `AudioPlayer` per ship) with bevy spatial
  audio: the "right" long-term shape (adds panning), but a bigger lifecycle
  change (spawn/despawn loops with ships) than this bug needs; the module
  doc already scopes spatialization as a future step.
- Summing attenuated per-ship contributions instead of max: rejected, two
  adjacent burning ships would exceed the single loop's design ceiling.

## Trace numbers

Post-fix (11241 traced frames over ~4 min of menu ambience):

- Ship burning hard, far outside the band: throttle 0.99 at 415 u,
  throttle 0.82 at 466 u -> `target=0.0000` in both (SFX_FAR_DISTANCE =
  320). Pre-fix this same burn would have driven the hum at
  `engine_volume(0.99) = 0.297` of 0.3 max - a full-volume hum from a ship
  415 u away, which is the reported bug.
- Ship swinging through the rolloff band: throttle 0.245 at 258 u ->
  `target=0.0033` (audible but faint) - the band attenuates rather than
  gates, as designed.

Pre-fix (same rig, same scene, fix reverted; 10894 traced frames):

- `avg_throttle=0.904 applied_volume=0.2604 burning_thruster_dists=[340.9]
  one_shot_equivalent_attenuation=[0.0]` - the hum played at 87% of its
  0.3 maximum while the ONLY burning thruster in the world was 341 u away,
  where the same event as a one-shot would have been silent. Peak applied
  volume over the run: 0.2960. This is the reported bug, reproduced in the
  shipped menu scene with real numbers.

## Difficulties

- `xvfb-run` does not exist on this host (only raw `Xvfb`); the first trace
  run silently did nothing - the pipeline swallowed the 127. Started `Xvfb
  :99` manually, like the 20260713-175352 rig.
- The pre-fix trace run raced the fix edits (cold worktree build reads the
  source when the crate compiles, minutes after launch); resolved with an
  explicit file-copy A/B: run once from the checked-out pre-fix audio.rs +
  trace, once from the fixed audio.rs + trace.
