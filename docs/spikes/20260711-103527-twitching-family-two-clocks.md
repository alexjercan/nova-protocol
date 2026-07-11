# Spike: the twitching family - two clocks, one bug

Umbrella task: `tasks/20260711-094915` (P99). Members:

- `tasks/20260710-231931` ship renders twitchy at high velocity (P90)
- `tasks/20260710-231930` bullets twitch badly at high ship velocity (P85)
- `tasks/20260710-231928` HUD text anchored to moving objects twitches (P82)
- `tasks/20260710-231929` turret crosshair twitches while tracking (P80)
- `tasks/20260711-103527` thruster impulses push from the stale eased pose
  (P95, created by this spike; carries the umbrella's flip-on-decel symptom)

Playtest report (user, 2026-07-10): at high velocities/distances everything
gets janky - the chase camera feels unstable, PDC bullets "spew out" and are
not linear, HUD text and the crosshair jitter, and holding retrograde from
high speed makes the ship twitch and sometimes flip.

## The mechanism: two pose clocks since the 2026-07-09 interpolation fix

`docs/2026-07-09-camera-twitch-interpolation.md` opted every dynamic body
into avian's `TransformInterpolation`. Since then the game has TWO pose
representations that tick on different clocks:

1. **Raw physics pose**: avian `Position`/`Rotation`, advanced on 64 Hz
   FixedUpdate ticks. The truth the simulation integrates.
2. **Render pose**: `Transform`, eased between the previous and current
   physics states in `RunFixedMainLoop::AfterFixedMainLoop`, i.e. up to one
   tick (~15.6 ms) behind raw. `GlobalTransform` is propagated from it in
   PostUpdate - so during FixedUpdate of frame N, `GlobalTransform` still
   holds the eased pose propagated in frame N-1's PostUpdate.

Every symptom in this family is one consumer mixing the two clocks (or
reading one of them a frame stale). The error in all cases is proportional
to velocity - which is exactly the playtest signature: fine at low speed,
janky at high speed. "High distances" is a red herring (see falsified
hypotheses); the correlation is that high speed is how you get far.

Rule of thumb going forward:

- FixedUpdate consumers (forces, spawns that feed physics, guidance) must
  read raw `Position`/`Rotation` (or compose root raw pose with local
  offsets). `GlobalTransform` in FixedUpdate is stale render state.
- Render-rate consumers (camera, HUD projection, effects) must read the
  eased `Transform`/`GlobalTransform`, and every pose in one on-screen
  computation must come from the same frame.

## Root causes per symptom

### 1. Flip/twitch when decelerating from high speed (umbrella symptom)

`thruster_impulse_system` (FixedUpdate,
`crates/nova_gameplay/src/sections/thruster_section.rs:131-159`) applies
impulses at `world_point = transform.translation()` - the thruster child's
`GlobalTransform`, i.e. the PREVIOUS frame's eased pose, up to ~2 ticks of
ship motion behind raw physics. The thrust direction on line 153 comes from
the thruster's avian `Rotation` (raw), so the system mixes both clocks in
one impulse.

The balancer that allocates throttles to null torque
(`crates/nova_gameplay/src/flight.rs:1499`) computes lever arms from the raw
pose: `pos_world = position.0 + rotation.mul_vec3(transform.translation)`,
and its comment claims this is "the same point the impulse system pushes
from". That was true before 2026-07-09; it is false now.

Magnitude: application point error ~ V * (0..2 ticks). At V = 100 u/s that
is up to ~3 u of misplacement for a full main-drive impulse. Lever-arm error
of ~3 u times main-drive force = large uncompensated torque, re-randomized
every frame by interpolation phase. The balancer cannot cancel a torque it
cannot see; the PD (max_torque = 40) fights it and loses at high speed. This
is very plausibly ALSO the "ship renders twitchy" root: the hull genuinely
jitters in attitude at high speed under thrust.

### 2. Bullets spew and are not linear

`shoot_spawn_projectile` (Update,
`crates/nova_gameplay/src/sections/turret_section.rs:752-865`):

- Muzzle pose from `TransformHelper::compute_global_transform` (line 803) =
  eased render pose, 0..1 tick behind raw.
- Inherited velocity from raw `LinearVelocity`/`AngularVelocity` (line 783)
  = current tick. Position and velocity disagree by ~0.5 * V * tick on
  average, and the disagreement varies per shot with interpolation phase.
- COM lift for point velocity uses the ship's eased GlobalTransform
  (line 826) - a third clock mix.
- Fire timer ticks with render dt in Update (`update_barrel_fire_state`,
  line 237): shot times quantize to frame boundaries, not tick boundaries.
- Compensation is a hard-coded `+ muzzle_exit_velocity * 0.01` (line 834):
  static, ignores inherited ship velocity and the actual timer overshoot.
- A bullet spawned mid-frame gets its avian `Position` synced at the next
  physics step and does not move until then: each bullet freezes for a
  different 0..15.6 ms slice while the ship keeps moving.

Sum: consecutive bullets start with irregular offsets both along and across
the stream, error ~ V * tick - at high ship velocity the stream visibly
"spews". At V = 200 u/s the per-shot scatter is ~1.5-3 u.

### 3. HUD text anchored to moving objects twitches

`update_screen_indicators` (Update,
`crates/nova_gameplay/src/hud/screen_indicator.rs:176-179`) projects anchor
world positions with the camera's `GlobalTransform` - but the chase camera
moves LATER, in PostUpdate (bcs `chase.rs`, systems in PostUpdate). So HUD
nodes are placed with the frame N-1 camera pose while the world renders with
the frame N pose: one full frame of camera motion of error, i.e. jitter
proportional to camera speed. Additionally, Point anchors are computed from
raw avian `Position` (e.g. the radius spoke,
`crates/nova_gameplay/src/hud/maneuver_instruments.rs:344-349`) while Entity
anchors resolve eased `GlobalTransform` - two pose families in the same
overlay.

### 4. Turret crosshair twitches while tracking

`drive_pip_anchors` (Update, `crates/nova_gameplay/src/hud/turret_lead.rs:78-84`)
consumes `TurretSectionAimPoint`, which is computed in PostUpdate
(`crates/nova_gameplay/src/sections/turret_section.rs:245-255`, after
`TransformSystems::Propagate`) - i.e. the pip always points at the PREVIOUS
frame's intercept solution, then gets projected with the stale camera pose
from (3). Jitter proportional to target relative motion plus camera motion.

## Falsified hypotheses (checked, ruled out)

- **Missing interpolation components**: the 2026-07-09 fix is correctly in
  place - ships/asteroids via `base_scenario_object`
  (`crates/nova_scenario/src/actions.rs:174`), turret bullets
  (`turret_section.rs:854`), torpedoes (`torpedo_section/mod.rs:538`). Not
  the cause.
- **Frame-rate-dependent camera lerp**: bcs `lerp_and_snap` is exponential
  decay in dt (`bevy-common-systems/src/meth/lerp.rs`), frame-rate
  independent. Not the cause.
- **f32 precision at large coordinates**: documented play space is a few
  hundred units (gravity well at 250 u, survey cap 250 u). ULP at 500 u is
  ~6e-5 u - orders of magnitude below visibility. Precision only becomes
  plausible past ~1e5 u from origin; nothing in the scenarios goes there
  today. Ruled out for THESE bugs; worth a floating-origin task only if
  scenarios ever grow unbounded cruises.
- **Aim solver oscillation** (user hypothesis "target calculation takes
  time"): the lead solve is closed-form per frame; the visible lag is the
  one-frame-stale consumption plus stale-camera projection, not solver
  flip-flop. Re-verify after the fix in case a residual remains.

## Fix plan (one task per branch)

1. `20260711-103527` (P95): thruster impulse application point from raw
   pose; audit ALL FixedUpdate readers of `Transform`/`GlobalTransform` and
   move them to raw physics state. Diagnostic-first per the residual-roll
   retro: tick-trace test showing assumed vs actual application point
   diverging with V, then the fix, then a tight regression assert.
2. `20260710-231931` (P90, depends on 1): re-test ship twitch at speed; the
   attitude jitter from (1) is the leading explanation since the camera
   wiring itself checked out. Add a no-input-straight-line spin regression;
   only dig into the camera if a residual remains.
3. `20260710-231930` (P85): move fire timing and bullet spawn to FixedUpdate
   on the raw pose, all velocity terms from the same raw state, spawn
   advanced by bullet velocity times timer overshoot. Linear-stream
   regression test.
4. `20260710-231928` (P82): move screen-indicator projection to PostUpdate
   after the chase camera and propagation (before UI layout); unify anchor
   pose sources on the render clock. Same-frame-camera regression test.
5. `20260710-231929` (P80, depends on 4): chain pip anchor driving after
   `update_turret_aim_point` in PostUpdate so the pip consumes this frame's
   intercept; keep the whole crosshair path on one clock.

The umbrella `20260711-094915` closes last: combined high-speed verification
(steady hull, linear bullet streams, stable HUD/crosshair) plus the user
playtest checklist.

## Notes for implementers

- avian keeps `Position`/`Rotation` on child collider entities in sync with
  the body's raw pose during the physics step - verify this holds for the
  thruster children (their query already reads avian `Rotation`), then
  prefer those components over recomposing from the root.
  UPDATE (20260711-103527): verified, and they are NOT good enough -
  `update_child_collider_position` runs before integration, so the child's
  avian pose is one tick stale too. Compose from the root's raw pose.
- Cross-repo: if PostUpdate ordering against the bcs chase-camera systems
  needs a public system set exported from bevy-common-systems, that change
  gets its own task and cycle in the bcs repo (precedent: residual-roll,
  `docs/retros/20260709-125640-residual-roll-release.md`).
- Do not "fix" bullets by spawning at the eased pose with eased velocity:
  physics would then integrate a pose that disagrees with every other body's
  raw state. Raw pose + raw velocity in FixedUpdate is the consistent choice;
  the muzzle flash stays on the render clock so visuals remain attached.

## Fix record

### 20260711-103527: thruster impulses (landed)

- `thruster_impulse_system` now composes application point AND thrust
  direction from the root's raw `Position`/`Rotation` (avian `Forces` item
  accessors) and the engine's local mount Transform - the balancer's exact
  lever-arm math. The GOTO goal pose in `autopilot_system` prefers the
  target's raw `Position` with a GlobalTransform fallback for static
  markers. Full audit table and diagnostic trace in
  `tasks/20260711-103527/TASK.md`.
- Mechanism refinement the diagnostic forced on the spike: the stale offset
  is PARALLEL to a prograde/retrograde burn (torque-benign); the flip comes
  from LATERAL thrust at speed (balancer recruits, drift correction), where
  the offset is perpendicular - plus rotation staleness mid-flip. Severity
  measured: a zero-true-torque lateral engine at 150 u/s spun the hull to
  7.1 rad/s in 15 frames pre-fix; 0 rad/s post-fix.
- Family status: with impulses on the raw clock, re-test 20260710-231931
  (ship render twitch) - the hull attitude jitter under thrust at speed is
  gone at the source.

### 20260710-231931: ship render twitch at high velocity (re-tested)

- Confirmed resolved-by-20260711-103527 at the mechanism level: a full
  production stack (PD at the shipped 40 budget, TransformInterpolation,
  centered drive) burning ACROSS 150 u/s of travel - the perpendicular
  regime that models a decel path with drift correction - held max
  angular velocity at ~0 over 180 frames, where the pre-fix impulse code
  ran away to 4.26 rad/s in an A/B. Regression:
  `cross_velocity_burn_keeps_the_hull_steady_at_high_speed` (flight.rs).
- No further render-side mechanism found: camera anchor/interpolation
  wiring was verified sound by the spike, and with the hull physically
  steady there is nothing left to alias. Final visual feel confirmation
  stays with the umbrella's user playtest step.

### 20260710-231930: bullet streams (landed)

- Fire timing and spawn moved to FixedUpdate on the raw root pose (new
  `local_pose_in_root` chain walk), all velocity terms on the same raw
  state, and the static 0.01 s nudge replaced by EXACT sub-tick lead:
  spawn at `muzzle - muzzle_exit_velocity * lead`. The ship-motion terms
  cancel in the derivation (v_bullet - v_muzzle = muzzle exit), so the
  planned "advance by full velocity * overshoot" formula was WRONG - it
  leaked ship-velocity scatter; the algebra is in the task file.
- Bonus finding: the shipped default fire rate (100 rounds/s) exceeds the
  64 Hz tick rate, and the old render-schedule path silently capped it at
  one bullet per frame. The bounded multi-shot loop restores authored
  cadence (regression pins ~100 bullets/s).
- Regressions: `bullet_stream_stays_linear_at_high_ship_velocity`
  (uniform + collinear deltas at 150 u/s cross-travel, A/B-proven) and
  `fire_rate_above_the_tick_rate_keeps_its_true_cadence`.

### 20260710-231928: HUD projection (landed)

- `update_screen_indicators` moved from Update to the PostUpdate slot
  `after(ChaseCameraSystems::Sync).before(UiSystems::Layout)`, composing
  fresh camera + anchor poses via TransformHelper (bevy_ui runs layout
  BEFORE transform propagation, so pre-propagation GlobalTransform reads
  would be stale - the spike's planned "after Propagate" slot was
  impossible).
- Bonus finding: bcs leaves `ChaseCameraSystems::Sync` unordered against
  `TransformSystems::Propagate` - a per-build coin flip that can render
  EVERY frame with last frame's camera pose. Nova pins the order in its
  camera controller plugin (additive configure_sets, no bcs change
  needed); upstreaming to bcs is nice-to-have.
- Ship-attached holo geometry (spoke line + chip midpoint, ribbon ship
  end, flip gate direction) now reads the ship's eased root Transform -
  the rendered pose - instead of raw avian Position.
- Regression: `indicator_projects_with_the_frames_final_camera_pose`
  (sub-pixel match against end-of-frame rendered poses every frame,
  A/B: the old Update schedule misses by 54 px in the same rig).

### 20260710-231929: turret crosshair (landed)

- The aim chain (intercept solve + rotator targets) moved early in
  PostUpdate on TransformHelper-composed fresh poses under the new public
  `TurretSectionAimSystems` set; the lead pips consume it SAME-frame
  between the aim chain and the indicator projection. The user hypothesis
  (solver oscillation) stayed falsified: the closed-form solve is
  stateless, and the regression pins monotone intercept motion.
- Regression: `pip_anchor_carries_the_same_frame_intercept` (pip anchor
  equals the same frame's aim point on every frame against a 60 u/s
  target; A/B: the old Update-schedule pips fail on the first frame).
- Family code status: all four members + the impulse root are landed.
  Remaining before the umbrella closes: combined verification + user
  playtest.
