# Bullets twitch badly at high spaceship velocity

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.5.0, rendering, physics, bug

## Goal

Playtest bug (user, 2026-07-10): bullets look funky - they twitch really
badly and "spew out" non-linearly at high spaceship velocity. Root cause
(docs/spikes/20260711-103527-twitching-family-two-clocks.md): bullets spawn
in Update from the EASED muzzle pose with RAW inherited velocity, the fire
timer quantizes shots to render frames, the only compensation is a static
`muzzle_exit_velocity * 0.01`, and a mid-frame-spawned bullet freezes until
the next physics tick. Every term errs by ~V * tick with per-shot phase, so
streams scatter at high ship velocity.

## Steps

- [x] Move fire timing to FixedUpdate. Done by absorbing the old
      `update_barrel_fire_state` into `shoot_spawn_projectile` (the two
      were UNORDERED within the same Update set - one more phase-jitter
      source): the cooldown ticks fixed dt whether or not the trigger is
      held, and the pre-tick `elapsed` recovers the sub-tick overshoot
      that Bevy's Once-mode Timer clamps away. Multi-shot per tick is
      preserved via a reset-and-advance loop (bounded by
      MAX_SHOTS_PER_TICK = 8); NOTE: the shipped default fire_rate is
      100 rounds/s, ABOVE the 64 Hz tick rate, so the old render-schedule
      path was silently capping the real cadence at one bullet per frame -
      the loop restores the authored rate.
- [x] Move `shoot_spawn_projectile` to FixedUpdate reading the raw pose:
      muzzle pose composed from the root's avian `Position`/`Rotation`
      plus the local mount chain (new `local_pose_in_root` walk over
      `ChildOf`, scale ignored per the flight-layer convention). Rotator
      locals written by the Update aim systems are at most one frame old -
      control-input staleness, not velocity-proportional error (noted in
      code).
- [x] All velocity terms from the same raw state: COM lifted with the raw
      pose (was the eased ship GlobalTransform), point velocity and muzzle
      exit unchanged in form.
- [x] Replace the `+ muzzle_exit_velocity * 0.01` fudge with EXACT
      sub-tick lead compensation: spawn at
      `muzzle_position - muzzle_exit_velocity * lead` where lead is the
      time into the tick when the shot came due. Derivation: the bullet
      due at lead must sit at the due-moment muzzle after this tick's
      integration; the ship-motion terms cancel because
      v_bullet - v_muzzle = muzzle_exit_velocity. This makes stream
      spacing exactly muzzle_speed * interval at ANY ship velocity -
      stronger than the originally planned "advance by full velocity *
      overshoot", which still leaked ship-velocity-proportional scatter.
- [x] Regression `bullet_stream_stays_linear_at_high_ship_velocity`
      (turret_section.rs tests): live-physics rig, interpolated ship at
      150 u/s cross to the muzzle, 24 rounds/s beating against the 64 Hz
      tick; asserts every consecutive bullet delta is the SAME vector
      (uniform + collinear) with a delivery guard pinning the stride to
      muzzle_speed/fire_rate. A/B-proven: fails with lead compensation
      disabled AND against the pre-fix Update-schedule path.
      Plus `fire_rate_above_the_tick_rate_keeps_its_true_cadence`:
      ~100 bullets in one second at the shipped 100 rounds/s.
- [x] Muzzle flash verified attached: the effect entity is a CHILD of the
      muzzle (on_projectile_marker_effect observer configures it in
      place), so it renders on the muzzle's render-clock pose regardless
      of where the spawn system runs. Intentional render-vs-physics
      offset: the bullet's physics origin is up to one tick ahead of the
      eased muzzle render pose; at bullet speeds this reads as normal
      muzzle exit.
- [x] cargo check (workspace) + fmt clean; full nova_gameplay lib suite
      354/354; spike doc fix record extended.

## Notes

- Evidence (pre-fix): turret_section.rs:240 (spawn in Update), :803 (eased
  muzzle via TransformHelper), :783 (raw velocities), :826 (eased COM
  lift), :834 (static fudge), :237 (render-rate fire timer).
- Bullets keep `TransformInterpolation`; spawning inside FixedUpdate makes
  their first rendered frames well-defined.
- Same investigation umbrella as 20260710-231928/229/231; spike covered
  all four - do not re-spike.

## Resolution

What changed: see Steps - one system rewrite (tick + spawn merged, raw
pose, exact lead), one helper (`local_pose_in_root`), registration moved
to FixedUpdate, two behavioral regressions, rig updates (muzzle now in the
ship's ChildOf tree with the ship's raw pose components, as production).

Alternatives considered:

- Switching the fire timer to TimerMode::Repeating (native overshoot via
  times_finished_this_tick): rejected - a repeating timer cycles while
  idle, so a trigger pull would wait up to a full interval instead of
  firing immediately (feel regression vs the Once+finish convention).
- Keeping spawn in Update with raw-pose sampling: rejected - shot times
  would stay render-quantized and the sub-tick lead would have to bridge
  a variable frame-to-tick gap; FixedUpdate makes the window exact.

Difficulties:

- The stream regression first failed because the rig fired DURING
  settle() while ship velocity was still zero - two velocity families in
  one stream. Fixed by arming the trigger only after the velocity is set.
- Process slip: ran the first A/B (sabotage lead compensation) BEFORE
  committing the rewrite, then reverted the sabotage with a file-level
  `git checkout` - which discarded the entire uncommitted rewrite. Redone
  from context in minutes, but the rule is: commit the fix, THEN sabotage
  for the A/B, so the revert has a clean base.

Self-reflection: the derivation-first approach to the lead formula paid
off - the planned formula from the spike ("full velocity * overshoot")
would have shipped a subtler version of the same scatter; writing the
tick-window algebra down exposed the cancellation and produced a simpler,
exact formula.
