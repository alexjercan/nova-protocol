# Review: Deepen sections/ to multi-scene multi-round runs, merging com_range and torpedo_guidance

- TASK: 20260804-093950
- BRANCH: feat/sections-multi-round-invariants

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) examples/sections/controller_section.rs:155-169 - `ReloadStep`
  is a one-method extension trait with one impl and two call sites, wrapping
  what the four sibling runs all express as a plain free function
  (`damage_and_mass_rounds(script, round)` hull_section.rs:169,
  `fire_round(script, round)` turret_section.rs:669 and torpedo_section.rs:799).
  No Step names the trait, and consistency across the five parallel scripts is
  what this task is for. Delete lines 155-169 and replace with
  `fn then_reload(script: Script, layout: Layout) -> Script`, called as
  `then_reload(script, Layout::A)`.
  - Response: Fixed as filed. `ReloadStep` is gone; `fn then_reload(script:
    Script, layout: Layout) -> Script` replaces it and `attitude_script` now
    threads `let script = ...` between the two calls, the same shape as
    `damage_and_mass_rounds` / `fire_round`.
- [x] R1.2 (MINOR) examples/sections/controller_section.rs:332-335 -
  `command_offset` reads a PERIODIC quantity off the ABSOLUTE clock, so the
  guard is not monotonic: `angle_between` peaks at pi and wraps back under
  `COMMAND_SWEEP_GUARD_RAD` for a window later in the sweep. A rig that
  respawns inside that window can never satisfy `rig_reset` (:368-372) and the
  reload beat stalls its full 15s deadline on a slower box. The doc comment at
  :328-330 acknowledges the wrap but the reload gate does not account for it.
  Zero the sweep epoch on each rig load - store the load time in a resource and
  pass `now - epoch` to `command_at` - so the guard grows monotonically per
  round.
  - Response: Fixed, with one deliberate departure from the filed remedy. The
    wrap is real and both readers of it are fixed, but zeroing the epoch inside
    `command_at` itself would have BROKEN the reload gate: the command would
    snap back to identity at exactly the moment a fresh rig spawns there, so
    `rig_reset`'s "the live rig is far off the command" could never be true and
    that beat would stall out its whole 15s deadline. The command therefore
    keeps sweeping across a reload (which is also what makes invariant 4 a
    catch-up claim), and the epoch is applied to the two things that actually
    needed it:
    - The guard quantity. `command_offset` (periodic, wraps) is replaced by
      `sweep_since_spawn`, the INTEGRATED angle
      `(now - RigEpoch::spawned_at) * COMMAND_RAD_PER_SEC`, monotonic by
      construction. `RigEpoch` is stamped by `track_rig_epoch` on
      `Added<SpaceshipRootMarker>`, so the epoch is when the rig APPEARED, not
      when the load was triggered - the assert now measures exactly what its
      message claims.
    - The reload gate. `rig_reset` no longer compares angles at all: the
      reload's `on_enter` records the root it is replacing, and the gate waits
      for a root that is not it. A despawned entity's id never returns
      (generation bump), so this is wrap-free and independent of the clock.
    Verified: all three rounds now report `command swept 1.208 rad since the
    rig spawned` - i.e. every round clears the guard at the same in-round time
    rather than wherever the absolute sweep happened to be.
- [x] R1.3 (MINOR) examples/sections/hull_section.rs:667-676 -
  `mass_properties_settled` gates the step on
  `drift < COM_DRIFT_TOLERANCE && cam_drift < CAMERA_DRIFT_TOLERANCE`, which is
  the exact conjunction invariants 4 (:689) and 7 (:740) then assert with the
  same two constants. Neither assert can fail, so a real COM or camera
  regression is reported as a deadline stall on the unrelated beat name "settle
  onto the surviving mass" instead of by its invariant message. Gate the step on
  a weaker precondition that the despawn landed and the solve ticked (e.g.
  `ComputedMass` having dropped below the pre-kill value) and let the tolerance
  comparison be the deciding assert. Same shape in turret_section.rs:813-818 vs
  :866 and thruster_section.rs:335-340 vs :431.
  - Response: Fixed in all three, each gated on a precondition about the world
    having STOPPED CHANGING rather than about where it stopped:
    - hull: `mass_properties_settled` now waits for `SURVIVING_SECTION_COUNT`
      (2) sections attached to the root - the despawns landed, counted rather
      than timed - AND for the new `ComSettle` resource to have seen the same
      LOCAL `ComputedCenterOfMass` for `COM_SETTLE_FRAMES` (3) frames running.
      Local, not world: round 2 spins the rig, so its world COM moves whether
      or not the solve has anything left to do. Took the filed
      `ComputedMass`-dropped idea but not literally - the round kills TWO
      sections, so one "below the pre-kill value" reading cannot distinguish
      one propagated despawn from two, whereas a stable COM under a counted
      surviving set covers both.
    - turret: `aim_converged` keeps `GATE_TRAVEL_MIN` (the mover moved) and
      replaces the tolerance clause with `AimSettle::still_frames >=
      AIM_STEADY_FRAMES` (5 frames within `AIM_STEADY_DELTA_DEG` = 0.5 deg) -
      the slew reached steady state, whatever value it settled at. A turret
      that settles 20 deg off now fails invariant 3 by its own message.
    - thruster: `plume_idle` now waits for the plumes to EXIST plus
      `frames(RELEASE_SYNC_FRAMES)` (2). Exact rather than a guessed settle:
      `thruster_shader_update_system` ASSIGNS the uniform, so whatever it will
      write is written by the frame after the release. Frames, not seconds,
      because the claim is "the sync system has RUN".
    `sections/` now holds exactly two time-shaped predicates -
    controller's `elapsed(TRACK_SETTLE_SECS)` and this one - both settle beats
    with their reason stated at the predicate.
    Verified by running all five: hull reports COM and camera drift 0.000,
    turret 0.6 deg then 0.3 deg, thruster reaches idle. The beats still open
    promptly and each assert is now the deciding comparison.
- [x] R1.4 (MINOR) tests/examples_smoke.rs:370 - the roster doc states every slug
  sits "beside the `assert!` it belongs to", and TASK.md's mechanics list
  repeats it, but three slugs carry no assert at all: `damage invariants hold
  after reload` (hull_section.rs:772-780), `turret invariants hold after reload`
  (turret_section.rs:882-888) and `launch chain holds in the crossing scene`
  (torpedo_section.rs:922-928) are pure round-completion markers. Either amend
  the comment to name that class explicitly, or make each assert the fact it
  claims.
  - Response: Both halves, and one correction. The turret and torpedo slugs
    were NOT assert-free: both are emitted from inside
    `assert_aim_tracks_mover` / `assert_launch_chain`, guarded on the round
    label, immediately after that function's own asserts have run for the
    round in question - the cited line ranges are the tail of those functions,
    not separate steps. hull_section was the real outlier: it had a
    marker-only step and an `assert_reload_held` that asserted nothing. That
    step and function are gone; invariant 8 now rides the last assertion of
    the reloaded round in `assert_com_follows_sections`, matching the other
    two. All three round-completion slugs are now emitted only from a line a
    failing invariant would never reach.
    Also amended both docs, since a round-completion invariant genuinely has
    no assert of its OWN to sit beside: the `SECTION_ROSTER` doc comment now
    names the class, lists its three members, and states what a source-grep
    roster can and cannot bound; TASK.md's mechanics list carries the matching
    exception.
- [x] R1.5 (MINOR) examples/sections/hull_section.rs:395 - `freeze_spawn_com`
  now runs once per RELOAD, and its only settle guard is `mass.value() > 0.5`,
  which a single attached section already satisfies. A capture taken before all
  five sections aggregate silently moves invariant 5's baseline; the measured
  shift is 0.90 against a 0.5 bound. Gate the freeze on the expected
  attached-section count (5) as well as on finite mass.
  - Response: Fixed, plus the frame the count alone would still have missed.
    `freeze_spawn_com` now requires `attached == RIG_SECTION_COUNT` (5) as
    filed, AND that the solve reproduced the same local COM on two consecutive
    frames - avian propagates mass properties on its own schedule, so the
    first frame with five children can still carry a COM computed from four.
    The stability `Local` is keyed by root entity so a reload cannot match the
    previous rig's reading.
- [x] R1.6 (MINOR) web/src/wiki/dev/guide-add-section.md:178 - "modelled on the
  existing per-section examples (`hull_section.rs` is the smallest)" went stale
  with the `com_range` merge: hull_section.rs is now 780 lines, second largest
  of the five, and controller_section.rs (440) is the smallest. Repoint the
  parenthetical at `controller_section.rs`.
  - Response: Fixed as filed - the parenthetical now reads
    `controller_section.rs is the smallest`.
- [x] R1.7 (NIT) examples/sections/turret_section.rs:564-597,743-757 -
  `HeldInput`, `hold_inputs`, `weapons_are_hot` and `scenario_is` are
  byte-identical to torpedo_section.rs:679-712,853-867. The reviewer proposed
  hoisting `scenario_is`/`weapons_are_hot` into `crates/nova_debug/src/harness.rs`
  beside `player_ship_present`/`section_gone`. Downgraded to NIT rather than
  taken as filed: the repo's recorded extraction trigger is the THIRD caller
  (DECISION.md, Step 8), and `20260804-094006` exists to do exactly that with
  three shapes visible. Defer unless a third caller lands first.
  - Response: Accepted, deferred to `20260804-094006` as the reviewer framed
    it. No third caller landed in this round's fixes, so the extraction
    trigger is still unmet.

Process signal: the two proofs that pin the roster are source greps, so the
whole "27 invariants" claim rests on marker literals rather than on assertions
firing. R1.4 is where that gap shows. Worth a line in the retro about what a
roster test can and cannot bound.

Verified in this round (re-derived independently, not taken from the reviewer):

- Proof 1 `sections_assert_their_invariant_roster` - PASS, run directly.
- Proof 5 `catalog_matches_disk` - PASS, run directly.
- Proof 4 `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests` - no
  hits.
- Proof 2 `cargo run -p nova_probe -- run sections` - reviewer reports aggregate
  OK, five ranges 5/6 (fps SKIPPED, no baseline), all 27 slugs present in the
  timelines.
- `cargo fmt --check` and `cargo check --examples --features debug` clean.
- All seven findings re-read at their cited `file:line` in the worktree before
  recording; every site matches its description.
- `ui_reach_playing_without_panic` and `systems_reach_playing_without_panic`
  fail under a parallel full-file `cargo test --test examples_smoke` on this
  branch. Independently re-derived as NOT a branch regression: both pass
  serially on the branch (2/2 runs, 42s and 44s) and on master at 629cffe9
  (2/2, 59s and 41s). The stall is the wall-clock "walk the scenarios picker"
  step losing its 100s deadline to Xvfb contention. Pre-existing, and outside
  this diff - no finding filed.

Pending user check (not resolvable by review):

- `manual:` reviewer reads the five scripts. Supporting grep confirms
  controller_section.rs:356 is the only `elapsed(`/`frames(` left in
  `sections/`, ANDed into `tracking_converged` as a settle clause with its
  reason stated at `TRACK_SETTLE_SECS`.

Not checked: real-GPU behavior (llvmpipe only), and flake rate - each range ran
once here, so the turret close-out's "8/8 consecutive" is unreproduced.

## Round 2

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R2.1 (MAJOR) examples/sections/hull_section.rs:760 -
  `mass_properties_settled` ANDs "2 sections left" with
  `ComSettle::still_frames >= COM_SETTLE_FRAMES`, but `track_com_settle` (:267)
  is an `Update` system while avian recomputes `ComputedCenterOfMass` in
  `FixedPostUpdate` (avian3d-0.7.0 `MassPropertyPlugin::default()` ->
  `FixedPostUpdate`, mass_properties/mod.rs:273; nova adds
  `PhysicsPlugins::default()` in nova_gameplay/src/plugin.rs:62). N consecutive
  UPDATE frames unchanged does not imply the fixed-rate solve has run even
  once. Worse, the counter is never reset when the attached-section count
  changes, and it saturates during the preceding `until(section_gone("hull1"))`
  wait, where the local COM is constant. Failure: hull1's despawn flushes on
  Update frame N; frame N+1 sees `count == 2` with no fixed tick in between, so
  the COM is still the 3-section value and `still_frames` is already >= 3. The
  gate opens on a stale COM and invariant 4 compares it against the fresh
  2-section centroid (~0.9u apart, 0.3 bound) and panics. Invisible on this box
  only because llvmpipe runs the example at ~14 fps (probe: hull `run_end at
  frame 71`, 5s), so several fixed ticks fall between Update frames; above
  64 fps roughly `1 - 15.6ms/dt` of frames carry no tick at all. Fix: store the
  attached-section count in `ComSettle` and zero `still_frames` when it
  differs, or move `track_com_settle` to `FixedPostUpdate` after
  `PhysicsSystems::Prepare` so one sample equals one solve pass.
  - Response: Fixed, taking BOTH filed remedies plus a third the finding
    implies. Each alone still leaves the hole:
    - `track_com_settle` moved to `FixedPostUpdate`
      `.after(PhysicsSystems::Prepare)`. One sample is now one recompute, so
      `still_frames` became `still_passes` and `COM_SETTLE_FRAMES` became
      `COM_SETTLE_PASSES` - the names were part of the wrong claim.
    - `ComSettle` now carries the whole reading (`ComSample`: root, attached
      count, local COM) and resets the streak when ANY of the three changes,
      so a despawn cannot be counted as "held still".
    - The third: `mass_properties_settled` no longer reads the section count
      LIVE off the world. That was the actual failure path in the finding - the
      despawn flushes on `Update`, so the world can report 2 sections while the
      newest solve pass still carries the 3-section COM and a streak saturated
      during the `section_gone` wait before it. Both clauses now come from the
      same post-solve sample. `attached_section_count` had no other caller and
      is gone.
    Verified: hull passes with COM and camera drift 0.000 on both rounds, and
    the run got FASTER (5s -> 1.4s standalone), which is the expected sign -
    the beat now opens on the first settled solve rather than on whichever
    `Update` frame happened to follow one.
- [x] R2.2 (MINOR) examples/sections/hull_section.rs:468 - the R1.5 stability
  check has the same Update-vs-`FixedPostUpdate` hole: "the same local COM on
  two consecutive frames" is two consecutive UPDATE frames, which above
  ~128 fps can both precede the first fixed tick after the fifth section
  attaches. `previous` is cleared only when `attached != RIG_SECTION_COUNT`, so
  both frames can carry the 4-section COM and freeze exactly the mid-assembly
  baseline R1.5 was filed against. Fix as R2.1 - sample after the physics
  prepare set, or require `ComputedCenterOfMass` to have actually changed
  (`Ref<..>::is_changed()`) since the count reached 5.
  - Response: Fixed by the first filed remedy - `freeze_spawn_com` moved to
    `FixedPostUpdate` `.after(PhysicsSystems::Prepare)` alongside
    `track_com_settle`, so its two consecutive samples are two consecutive
    solve passes and cannot both precede the recompute that follows the fifth
    section attaching. The count gate and the root-keyed `Local` are unchanged.
    Took the schedule move rather than `Ref::is_changed()` so both COM samplers
    in this file rest on the same rule, and the rule is stated once.
- [x] R2.3 (MINOR) examples/sections/turret_section.rs:190 -
  `AIM_STEADY_DELTA_DEG` is a PER-FRAME delta, so "steady state" is
  framerate-dependent: the joints slew via `SmoothLookRotation`, whose `speed`
  is rad/SECOND, giving `delta = speed * dt`. Under llvmpipe (dt ~70ms here) a
  slewing barrel moves ~10 deg/frame and the gate correctly refuses; at ~360fps
  a full-rate slew moves under 0.5 deg/frame, five such frames satisfy
  `still_frames >= AIM_STEADY_FRAMES` MID-SLEW, and invariant 3 then reads an
  error tens of degrees out and panics. Fix: divide the delta by
  `Time::delta_secs()` and compare against a RATE (deg/s), or sample the aim
  error on a fixed cadence rather than per frame.
  - Response: Fixed by the first filed remedy. `AIM_STEADY_DELTA_DEG` (0.5
    deg/frame) is now `AIM_STEADY_RATE_DEG_PER_SEC` (10.0), and
    `track_aim_settle` divides the frame-to-frame delta by
    `Time::delta_secs()`, skipping the sample when `dt` is zero. 10 deg/s is an
    order of magnitude under the ~140 deg/s the joints slew at (`speed` is pi
    rad/s per the section config) and above the wobble a barrel holding a swept
    target shows, so the threshold now separates slewing from tracking at any
    framerate.
    NOT the fixed-cadence alternative: the aim error is written on `Update`, so
    consecutive fixed ticks inside one `Update` frame would read the SAME value
    and the streak would saturate mid-slew - the opposite failure. The rate is
    framerate-independent without that.
    Verified: round 1 asserts at 0.6 deg, round 2 at 0.4 deg (tolerance 3.0),
    and the log shows the gate correctly refusing the 24.9 / 28.8 deg
    acquisition frames.
- [x] R2.4 (MAJOR) examples/sections/controller_section.rs:368 - swapping
  `command_offset` for `sweep_since_spawn` fixed the wrap stall but dropped the
  property the delivery guard exists for. The doc at :336 still claims "below
  this a hull frozen at spawn would pass on the tolerance alone", which held
  only because the OLD quantity was exactly the frozen hull's error; the new one
  is arc length travelled, and the command keeps its absolute phase across a
  reload. A rig that spawns while the command sits at phase ~-1.2 rad has the
  command back AT identity precisely when `swept` crosses
  `COMMAND_SWEEP_GUARD_RAD`, so a completely dead PD parked at identity
  satisfies invariant 1 (`swept > 1.2`) and invariant 2 (`error < 0.35`)
  together for a ~2s window - the exact false pass the guard was written to
  exclude. Keep `sweep_since_spawn` as the monotonic BEAT, and additionally
  assert the exclusion the guard names, e.g.
  `command_at(now).angle_between(spawn_rotation) > TRACK_TOLERANCE_RAD`.
  - Response: Fixed as filed, with the beat amended too. The delivery guard is
    now explicitly TWO claims, and the doc at the constant says so:
    - `COMMAND_SWEEP_GUARD_RAD` over `sweep_since_spawn` - how much chasing the
      PD was asked to do. Monotonic, so it stays the usable beat.
    - NEW `command_offset_from_spawn(world) > TRACK_TOLERANCE_RAD` - the
      exclusion the guard is named for. At exactly the tracking tolerance a
      hull frozen at spawn is outside it by construction, which is the property
      the old `command_offset` had and arc length does not.
    `RigEpoch` gained `spawn_rotation`, stamped by `track_rig_epoch` from the
    root's live `Rotation` rather than assumed to be identity, so the assert
    measures the separation a frozen hull would actually show.
    The beat (`tracking_converged`) carries the new clause as well as the
    assert. Required, not belt-and-braces: if only the assert had it, a beat
    that opened while the command sat near the spawn attitude would panic a
    HEALTHY PD. Adding it to the beat cannot stall - the offset is under 0.35
    rad for ~2s of each ~18s turn, unlike the 1.2 rad guard whose dead window
    was ~7s and which is what made R1.2's wrap a stall.
    Verified: all three rounds still assert (0.184 / 0.180 / 0.186 rad lag),
    and the log line now reports the offset beside the swept angle.
- [x] R2.5 (NIT) web/src/wiki/dev/guide-add-section.md:178 - the R1.6 repoint is
  already wrong: this branch grew `controller_section.rs` to 475 lines, above
  `thruster_section.rs` at 460. Point the parenthetical at
  `thruster_section.rs`, or drop the superlative so it cannot go stale again.
  - Response: Fixed by the second remedy - the superlative is gone. The
    parenthetical now reads "`controller_section.rs` and `thruster_section.rs`
    are the most compact of the five", which stays true under either ordering
    and does not need re-checking every time a run gains an invariant.

Also taken from round 2's process signal, since it is a repo-wide inference
rather than one file's bug: `web/src/wiki/dev/automation-harness.md` gains a
paragraph in the predicate-vocabulary section stating that a settle predicate
over a physics quantity belongs on the physics schedule (sample in
`FixedPostUpdate` after `PhysicsSystems::Prepare`, carry the beat's other facts
in the same sample, and compare a RATE rather than a per-frame delta), with
`ComSettle` and `AimSettle` named as the worked examples.

Round 1 findings, verified at their cited sites before ticking:

- R1.1 CONFIRMED FIXED - the trait is gone, `fn then_reload(script, layout)`
  replaces it, threaded like the sibling runs. Ticked.
- R1.2 PUSHBACK SOUND and the wrap is gone - zeroing the command really would
  have made `rig_reset` unsatisfiable, and the entity-identity gate is
  wrap-free. Ticked. The substitute guard quantity lost a separate property;
  that is R2.4, not a reopening of R1.2.
- R1.3 PARTIALLY FIXED - thruster is confirmed (the sync is an `Update` assign,
  so 2 frames suffices and the assert now decides). hull and turret removed the
  tautology but replaced it with a framerate-dependent one: R2.1, R2.3. Left
  unticked.
- R1.4 CONFIRMED FIXED and the pushback is SOUND - verified turret_section.rs
  and torpedo_section.rs emit their round-completion slugs inside `assert_*`
  after those functions' own asserts; hull's marker-only step and
  `assert_reload_held` are gone. Both doc surfaces amended. Ticked.
- R1.5 PARTIALLY FIXED - the count gate landed as filed; the added stability
  check does not prove what it claims (R2.2). Left unticked.
- R1.6 FIXED as filed, but the new pointer is itself inaccurate (R2.5). Ticked
  on the filed remedy.
- R1.7 PUSHBACK SOUND - the deferral matches DECISION.md Step 8 and no third
  caller landed. Ticked.

Process signal: three of the four round-1 settle-gate rewrites assume "N
consecutive Update frames unchanged" implies the fixed-rate solve has run. That
inference is wrong repo-wide, and the ~14fps llvmpipe verification box is the
most forgiving hardware possible for it - every one of them passed here. Worth
a retro line and possibly a repo-level note: a settle predicate over a physics
quantity belongs on the physics schedule, not on `Update`.

Verified in this round (re-derived independently, not taken from the reviewer):

- The load-bearing schedule claim, read from the vendored source myself:
  avian3d-0.7.0 mass_properties/mod.rs:273 - `impl Default for
  MassPropertyPlugin { fn default() -> Self { Self::new(FixedPostUpdate) } }`.
  Both `track_com_settle` (hull_section.rs:250) and `track_aim_settle`
  (turret_section.rs:171) are registered on `Update`. R2.1-R2.3 follow.
- R2.4 re-derived from the code rather than accepted: `command_at` is still
  absolute (`time.elapsed_secs()`), rigs spawn at `Quat::IDENTITY`, and
  `sweep_since_spawn` is arc length - so the guard no longer bounds the
  command's distance from the spawn attitude.
- R2.5 line counts by `wc -l`: thruster 460, controller 475.
- Proofs 1, 2, 4, 5 all pass (roster test, `probe run sections` aggregate OK
  five ranges 5/6 with fps SKIPPED, the `rg` proof returns no hits,
  `catalog_matches_disk`). `cargo fmt --check`,
  `cargo check --examples --features debug` and `cargo check --examples` clean.
- TASK.md's round-1 close-out numbers check out against the run logs and the
  probe timelines; no honesty finding. The one overstatement is its DIFFICULTIES
  claim that the stability gate "cannot open until the solve has stopped
  moving", which R2.1 shows is false on a faster box.

Pending user check (not resolvable by review):

- `manual:` reviewer reads the five scripts.

Not checked: behavior above ~64fps - R2.1-R2.3 are derived from the schedules
rather than reproduced, since llvmpipe caps this box at ~14fps; and flake rate,
each range having run once.

## Round 3

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R3.1 (MAJOR) examples/sections/controller_section.rs:426 - the R2.4 fix
  added a second shared constant to `tracking_converged` but left the first:
  the beat still gates on `tracking_error(world) < TRACK_TOLERANCE_RAD`, the
  exact comparison `assert_tracking` (:459-463) then makes with the same
  constant. Invariants 2, 3 and 4 therefore cannot fail - a PD that never
  converges stalls the "converge on the command" beat at its 12s deadline
  instead of reporting "the PD is not tracking". This is the last live instance
  of the class R1.3 filed, after hull, turret and thruster were fixed. Filed
  MAJOR rather than the MINOR R1.3 carried, on impact rather than on
  precedent: three of the 27 invariants the Definition of Done is written over
  are not actually pinned, so the DoD's "every invariant HOLDS, where a failed
  assert panics the process" is not true of them. Concrete change: drop the
  tolerance clause from `tracking_converged` and gate on the error having
  STOPPED CHANGING - an error-rate settle in the shape of turret's `AimSettle`,
  or `elapsed(TRACK_SETTLE_SECS)` with the two delivery-guard clauses alone -
  leaving `error < TRACK_TOLERANCE_RAD` as the deciding assert.
  - Response: Fixed by the second filed remedy - the tolerance clause is gone
    and the beat is `elapsed(TRACK_SETTLE_SECS)` plus the two delivery guards
    alone. No error-rate settle: both remaining clauses are about the COMMAND,
    i.e. the stimulus, so the beat now reads nothing the assert reads and the
    assert is the only thing that looks at the hull. That is stronger than the
    `AimSettle` shape here, not weaker - turret needs a settle because its
    stimulus is the mover's travel and the response has its own transient,
    whereas this run's runway is already set by the sweep guard (3.4s, against
    a PD that converges in a fraction of it).
    Two renames follow from it, because the old names asserted the thing that
    was removed: `tracking_converged` -> `command_delivered`, and the three
    beats `converge on ...` -> `sweep the command clear of <rig>'s spawn
    attitude`. A beat's name is what a stall message carries, and these beats
    no longer wait for convergence. The module doc and `attitude_script`'s doc
    both claimed "every beat waits on the live tracking error"; both amended.
    Verified by SABOTAGE, since the whole finding is that the assert could not
    fail: with `TRACK_TOLERANCE_RAD` cut to 0.01 the run now panics
    `attitude probe (rig a): hull is 0.183 rad off the command (tolerance
    0.01); the PD is not tracking` at controller_section.rs:478. Before this
    fix that same sabotage would have stalled the beat instead. Constant
    restored.
- [x] R3.2 (MAJOR) examples/sections/controller_section.rs:426 - the new
  `command_offset_from_spawn > TRACK_TOLERANCE_RAD` clause sits on the beat AND
  is re-read by the assert (:451-457), but unlike `sweep_since_spawn` the
  quantity is a triangle wave, and the driver enters the next step on the
  FOLLOWING frame (`nova_autopilot/src/autopilot.rs:406`: "entry gets its own
  frame, so the state transition has applied ... before any predicate is
  polled" - re-derived in the recording pass). If the beat first holds while
  the offset is DESCENDING through 0.35 rad, the assert one frame later reads
  `offset - COMMAND_RAD_PER_SEC * dt` and panics a healthy PD. The doc at
  :417-419 ("the assert never fires on a moment the beat would not have opened
  on") is only true for a monotonic quantity, which this one is not. Raised
  from the reviewing agent's MINOR by the recording pass, on impact: this is a
  spurious panic in an example CI runs headless, i.e. a green branch failing
  its own build at the reviewer's derived ~0.4%/round, ~1%/run. Concrete
  change: make the beat strictly stronger than the assert - gate on
  `offset > TRACK_TOLERANCE_RAD + COMMAND_RAD_PER_SEC * 0.5` while the assert
  keeps the bare `TRACK_TOLERANCE_RAD`.
  - Response: Fixed as filed. The beat's clause is now
    `offset > TRACK_TOLERANCE_RAD + COMMAND_RAD_PER_SEC * OFFSET_BEAT_MARGIN_SECS`
    (0.5s of sweep = 0.175 rad of margin) while `assert_tracking` keeps the
    bare `TRACK_TOLERANCE_RAD`, so the beat is strictly stronger and the
    one-frame gap cannot cross it at any framerate. The new constant's doc
    states the mechanism - "entry gets its own frame" - so the next reader does
    not have to re-derive it.
    The stale doc claim the finding cited ("the assert never fires on a moment
    the beat would not have opened on", true only for monotonic quantities) is
    replaced by one that names the margin instead of asserting the property.
    Verified: the three rounds assert at 1.579 / 2.806 / 2.251 rad of offset,
    all far above the 0.525 rad beat threshold.
- [x] R3.3 (NIT) web/src/wiki/dev/automation-harness.md:124 - the new
  paragraph's rule is "sample in `FixedPostUpdate` after
  `PhysicsSystems::Prepare`" and it names "`hull_section`'s `ComSettle` and
  `turret_section`'s `AimSettle`" as the worked examples, but `track_aim_settle`
  is deliberately registered on `Update` (turret_section.rs:640) because the aim
  error is produced there - the R2.3 response's own reasoning. A cold reader
  following this paragraph would move it and saturate the streak mid-slew, the
  exact failure that response rejected. Concrete change: attribute each half -
  `ComSettle` for the schedule rule, `AimSettle` for the rate rule - and add one
  clause saying `AimSettle` stays on `Update` because that is where its quantity
  is written.
  - Response: Fixed as filed, and the paragraph was wrong in the way the
    finding says - it stated the schedule rule and then named a counter-example
    as evidence for it. Split in two: `ComSettle` carries the schedule rule,
    and a second paragraph states the actual rule ("sample where the quantity is
    WRITTEN", not "sample on the fixed schedule"), says `AimSettle` stays on
    `Update` because `SmoothLookRotationPlugin` writes in `PostUpdate`, and
    gives the rate rule as what you buy framerate independence back with.
    Added a third paragraph while there, since R3.1 and R3.2 are the same
    omission at repo level: a beat must be strictly WEAKER than the assert that
    follows it, and where both must read one quantity the beat needs a margin
    because entry costs a frame.
- [x] R3.4 (NIT) CHANGELOG.md:21 - "each walking a NAMED roster of invariants
  over several predicate-gated rounds and at least two scenes or rig layouts" is
  not true of `thruster_section`: one scene, one rig, three rounds, no reload
  (re-derived in the recording pass - its script is one `LoadScenario` at
  :155 and no reload step). Same claim at web/src/wiki/dev/development.md:164.
  TASK.md's own owner call says scenes and rounds are means, not the bound.
  Concrete change: in both places, replace "and at least two scenes or rig
  layouts" with "across as many scenes or rig layouts as its invariants need".
  - Response: Fixed as filed in both places. The claim was false of
    `thruster_section` (one `LoadScenario`, no reload step) and contradicted
    TASK.md's own owner call that scenes and rounds are means to the invariant
    list rather than the bound.

Round 2 findings, verified at their cited sites before ticking:

- R2.1 CONFIRMED FIXED - `track_com_settle` is in `FixedPostUpdate`
  `.after(PhysicsSystems::Prepare)`; avian's mass-property sets are chained
  `.in_set(PhysicsSystems::Prepare)` in that schedule
  (`mass_properties/mod.rs:299-309`), so one sample is one recompute.
  `ComSample` carries root + count + COM and the streak resets on any change;
  `mass_properties_settled` reads both clauses from that one post-solve sample;
  `attached_section_count` is gone. Ticked.
- R2.2 CONFIRMED FIXED - `freeze_spawn_com` on the same schedule and ordering,
  count gate and root-keyed `Local` intact. Ticked.
- R2.3 CONFIRMED FIXED and the pushback is SOUND - `SmoothLookRotationPlugin`
  runs in `PostUpdate`, so the aim error genuinely is an Update-clock quantity
  and a fixed cadence really would saturate the streak. The rate divide is
  framerate-independent. Ticked.
- R2.4 CONFIRMED FIXED, with a regression. The exclusion is restored on both
  beat and assert, and `RigEpoch.spawn_rotation` reads a real value (the run
  logs 1.578 / 2.803 / 2.252 rad). The regression is the non-monotonic re-read:
  R3.2. Ticked on the filed remedy.
- R2.5 CONFIRMED FIXED - superlative dropped, no other stale pointer. Ticked.
- R1.3 CONFIRMED FIXED at its three cited sites; left ticked. The class
  survives at a fourth site rounds 1-2 never scoped - R3.1.
- R1.5 CONFIRMED FIXED - count gate plus two consecutive solve passes. Ticked.

Process signal: rounds 1-3 are the same finding three times, at four sites,
each round's fix creating the next round's version of it. R1.3 named the shape
(beat and assert sharing a constant) but scoped itself to the three sites it
had read, so the fourth shipped; R2.4's substitution introduced R3.2 because
the fix changed the QUANTITY without re-reading what the beat had guaranteed.
Worth a retro line on scoping a class-shaped finding to the class rather than
to the cited sites, and on re-checking beat-vs-assert strength whenever either
side's quantity changes.

Verified in this round (re-derived independently, not taken from the reviewer):

- R3.2's load-bearing claim, read from the source myself:
  `nova_autopilot/src/autopilot.rs:406-426` - the entry branch returns before
  any predicate is polled, so `on_enter` runs a frame after the beat held.
- R3.4 re-derived: `thruster_section` triggers `LoadScenario` once (:155) and
  its step list (:115-143) carries no reload, so "at least two scenes or rig
  layouts" is false of it.
- Proofs 1, 2, 4, 5 all pass: `sections_assert_their_invariant_roster` and
  `catalog_matches_disk` PASS, `probe run sections` aggregate OK with all five
  ranges 5/6 (fps SKIPPED, no baseline), the `rg` proof returns no hits.
  `cargo fmt --check`, `cargo check --examples --features debug` and
  `cargo check --examples` clean.
- TASK.md's round-2 close-out numbers check out against the run logs and the
  probe timelines; no honesty finding.

Pending user check (not resolvable by review):

- `manual:` reviewer reads the five scripts.

Not checked: behavior above ~64fps (llvmpipe caps this box at ~14fps), so
R2.1-R2.3 remain verified from the schedules rather than reproduced; and flake
rate, so R3.2's window is derived, not observed.

## Round 4

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R4.1 (MAJOR) examples/sections/torpedo_section.rs:814 - the beat is
  IDENTICAL to the assert it precedes, so 5 of the run's 6 invariants cannot
  fail. `launch_chain_complete()` (:882) is
  `fired && armed && detonated && gate_damaged`, and `assert_launch_chain`
  (:899-914) asserts those same four flags; `lead_angle_over(LEAD_MIN_DEG)`
  (:787, :890) is `best.0 > LEAD_MIN_DEG`, and `assert_leads_the_crosser`
  (:940) asserts `lead > LEAD_MIN_DEG` with the same constant. A dud torpedo or
  a pure-pursuit guidance surfaces only as a deadline stall on `hold the
  trigger` / `lead the crosser`, never as the invariant message. Same class as
  R1.3 and R3.1, at sites no round scoped, and it contradicts the rule this
  diff itself adds at web/src/wiki/dev/automation-harness.md:136 ("a beat must
  be strictly weaker than the assert that follows it"). Concrete change: gate
  the launch beat on a strictly weaker prefix of the chain (`fired && armed`)
  plus a bounded settle, and give the lead beat a margin over `LEAD_MIN_DEG`
  the way `OFFSET_BEAT_MARGIN_SECS` does, leaving the full four-flag and
  `> LEAD_MIN_DEG` comparisons as the deciding asserts.
  - Response: fixed. `hold the trigger` now gates on
    `and(torpedo_armed(), elapsed(LAUNCH_SETTLE_SECS))` - `fired && armed`, the
    strictly weaker prefix, plus a 10s window derived from the ~90 u range at a
    ~35 u/s cap. `assert_launch_chain` decides `detonated` and `gate_damaged`.
    `lead the crosser` no longer reads the angle at all: `BestLeadDeg` gained
    `first_sample_at`, and `lead_sample_settled()` waits for a midcourse leg to
    have been FLOWN (a 14 u window crossed at ~31 u/s, held `LEAD_SETTLE_SECS`
    = 1.5s after the first sample), leaving `> LEAD_MIN_DEG` as the deciding
    comparison. Deliberately NOT the proposed margin over `LEAD_MIN_DEG`: a
    margin on the DECIDING comparison leaves invariant 5 unfailable in exactly
    the way R4.4 names. The margin pattern belongs on a shared GUARD clause
    (R4.3); what was missing here was a stimulus-side observable - "a sample
    was taken", which the angle alone cannot tell from "no torpedo got close
    enough to sample".
- [x] R4.2 (MAJOR) examples/sections/turret_section.rs:734 - same defect:
  `.until(and(range_fired(), gate_took_damage()))` is the exact conjunction
  `assert_fired_and_connected` (:890-896) then asserts, so invariants 1 and 2
  cannot fail. A turret that never fires - the bug this task's own close-out
  records twice - reports "step `hold the trigger` exceeded its deadline"
  instead of "no turret round fired in the window". Concrete change: gate the
  beat on `range_fired()` alone plus a bounded flight settle, and let
  `assert_fired_and_connected` be the deciding comparison for `gate_damaged`.
  - Response: fixed. `and(range_fired(), elapsed(HIT_SETTLE_SECS))`, with
    `HIT_SETTLE_SECS` = 4.0 derived from the ~140 deg/s slew and 48-65 u gates
    at a 100 u/s muzzle speed (~1.7s for a healthy turret). `gate_took_damage()`
    is deleted; `assert_fired_and_connected` is now the deciding comparison for
    both flags.
- [x] R4.3 (MAJOR) examples/sections/turret_section.rs:872 - R3.2's defect at a
  second, unfixed site, and a live flake. `aim_converged` gates on
  `gate_travel > GATE_TRAVEL_MIN` and `assert_aim_tracks_mover` (:920) re-reads
  the same quantity against the same constant one frame later, but
  `gate_travel` is `|x(t) - x(t0)|` on a sine sweep (`drive_moving_gate`, :466)
  and is NOT monotonic - amplitude 22.0, rate 0.6, so it returns to 0 every
  ~10.5s and peaks at 13.2 u/s (~0.9 u per frame at 14fps). If the `AimSettle`
  streak saturates while travel is FALLING through 8.0, the beat opens and the
  assert one frame later reads under `GATE_TRAVEL_MIN` and panics a healthy
  turret. Concrete change: mirror `OFFSET_BEAT_MARGIN_SECS` - gate on
  `travel > GATE_TRAVEL_MIN + SWEEP_AMPLITUDE * SWEEP_RATE * 0.5` while the
  assert keeps the bare `GATE_TRAVEL_MIN`. The error clause of that assert is
  correctly gated on steady state and is not at issue.
  - Response: fixed. `GATE_TRAVEL_BEAT_MARGIN` = `SWEEP_AMPLITUDE *
    SWEEP_RATE * 0.5` = 6.6 u, on the BEAT only; the assert keeps the bare
    `GATE_TRAVEL_MIN`. Confirmed live this session: round 2 opened the beat at
    15.6 u and the assert read 15.6 against 8.0, so a falling edge no longer
    reaches the assert.
- [x] R4.4 (MAJOR) examples/sections/thruster_section.rs:280 - `gain_reached()`
  requires `speed - baseline >= BURN_GAIN_TARGET` (20.0) and `assert_full_burn`
  (:364) then asserts `gain > 0.0`, so the beat is strictly STRONGER and
  invariant 1 ("burn accelerates") is unfalsifiable, as is the `rate > 0.0`
  half of invariant 4 (:416); only `rate < full_rate` is decided by an assert.
  Filed MAJOR rather than the MINOR the round-4 reviewer proposed, for the same
  reason R3.1 was raised over R1.3: it is the identical class as R4.1/R4.2 and
  costs three more of the 27 invariants the DoD is written over. Concrete
  change: gate both burn rounds on the throttle having been delivered plus a
  fixed burn window, and let `gain > 0` / `rate < full_rate` decide -
  `measure_round` already divides by the round's own duration, so the rates
  stay comparable across unequal windows.
  - Response: fixed, and agreed on the severity. `burn_window_held(throttle)`
    gates on the drive having TAKEN the round's throttle - `ThrusterSectionInput`
    on the same production seam `hold_throttle` writes - plus `BURN_WINDOW_SECS`
    = 2.5. Nothing in the beat reads the hull's speed, so `gain > 0.0` and
    `rate > 0.0` both decide. Measured this session: 21.833 u/s^2 full (gain
    55.7 u/s) and 8.608 partial (gain 21.9 u/s), so the window clears the noise
    floor on both rounds.
- [x] R4.5 (MINOR) examples/sections/controller_section.rs:407 - stale doc plus
  a now-inert clause, introduced by the R3.1 fix. `TRACK_SETTLE_SECS` is
  documented as "how long a converge beat holds after the error first drops
  inside the tolerance", with the PD catch-up transient as its stated reason -
  but R3.1 removed the tolerance clause from `command_delivered` (:440), so
  `elapsed(TRACK_SETTLE_SECS)` now measures 1.5s of in-step time from the
  step's entry and has nothing to do with the error crossing anything. The
  module doc at :20-24 ("Every beat waits on a world value ... never on a
  runway") is false while it stands, and this is the one `elapsed` the DoD's
  "settle beats with a stated reason" clause rests on. Concrete change: drop
  the `elapsed(TRACK_SETTLE_SECS)` clause and the constant -
  `COMMAND_SWEEP_GUARD_RAD` already supplies the runway the old doc argued for.
  - Response: fixed. `elapsed(TRACK_SETTLE_SECS)` and the constant are gone;
    `command_delivered` documents the sweep guard as the runway instead.
    Confirmed the guard subsumes the old 1.5s: all three rounds opened at
    swept 1.207-1.209 rad, i.e. 3.45s of chasing after the rig spawned
    (1.2 rad / 0.35 rad/s), so the module doc at :20-24 is now true.
- [x] R4.6 (NIT) CHANGELOG.md:21 - the R3.4 edit left a 94-column line inside
  an `[Unreleased]` block whose other lines wrap at ~75. Re-wrap the paragraph
  so "`com_range` folds into `hull_section` and" starts a new line.
  - Response: fixed, paragraph re-wrapped to the block's ~75 columns.
- [x] R4.7 (NIT) web/src/wiki/dev/development.md:225 - the insertion left a
  29-column orphan line ("only drive the shipped scenes") mid-paragraph in an
  otherwise ~75-column file. Re-flow lines 222-227.
  - Response: fixed, lines 222-227 re-flowed.

Process signal: the beat-must-be-weaker-than-its-assert defect has now been
filed in every round - R1.3, R3.1, R3.2, and R4.1-R4.4 - because each fix was
scoped to the site the finding cited. The remaining sites should be closed as
one sweep over all five scripts, and the rule the diff added at
automation-harness.md:136 checked against each `.until(...)`/`assert_*` pair,
rather than one more site at a time.

Re-derived in session, not taken from the reviewer:

- R4.1, R4.2 and R4.4 read at their sites: the predicate bodies and the assert
  bodies are the same comparisons against the same constants in all three.
- R4.3's arithmetic re-derived from `SWEEP_AMPLITUDE` (22.0), `SWEEP_RATE`
  (0.6) and `drive_moving_gate`'s `sin` path: period ~10.5s, peak 13.2 u/s,
  so travel crosses 8.0 downward once per period.
- R4.5 read at both sites: `command_delivered` contains no tolerance clause.
- R4.6/R4.7 checked against the surrounding wrap width rather than the file's
  historical long lines.

Checks run in session:

- `cargo fmt --all --check` clean.
- `sections_assert_their_invariant_roster`, `catalog_matches_disk` and
  `examples_name_drivers_through_the_nova_harness` PASS.
- `cargo test --test examples_smoke` FAILED 2 of 8:
  `systems_reach_playing_without_panic` (stall on `round 1: sweep the prey into
  a combat lock`, 20s) and `ui_reach_playing_without_panic`. Both are outside
  this diff - it touches no `examples/systems/` or `examples/ui/` file - and a
  `check/master-scenarios-flake` worktree is already open on that flake. Not a
  finding against this branch; not cleared as unrelated either, since it was
  not reproduced on master this round.
- `controller_section` RUN headless under Xvfb :99 by the out-of-context
  reviewer: exit 0, three rounds at 0.190 / 0.180 / 0.182 rad.

Pending user check (not resolvable by review):

- `manual:` reviewer reads the five scripts.

Not checked: the other four examples were not run this round; behavior above
~64fps; flake rate. `torpedo_section`'s deadline sum is 111s against the 120s
backstop - derived from source, thin, and unmeasured here.

Fix pass (work, round 4):

The process signal is taken: rather than patching the four cited sites, every
`.until(...)` / `assert_*` pair in all five scripts was checked against the rule
at automation-harness.md:136. `hull_section`'s two pairs are the only ones that
needed no change, for a reason worth recording: `side_hull_took_the_partial_hit`
is `health <= before - PARTIAL_HIT` while `assert_partial_hit` demands EXACT
equality, so an over-subtracting damage path opens the beat and FAILS the
assert; and `mass_properties_settled` reads settle passes plus an
attached-section count, which no assertion in `assert_com_follows_sections` (COM
drift, aft shift, camera anchor) touches. `controller_section`'s offset clause
keeps its margin because it is a shared GUARD, not the deciding comparison.

automation-harness.md gained the other half of the rule these fixes rest on:
where an invariant has no stimulus-side observable, the beat is the stimulus
plus a bounded settle sized off the mechanism, never the outcome - and a settle
owes a derivation on its constant.

Checks run in this pass:

- `cargo fmt --all -- --check` clean; `cargo clippy --examples --features debug`
  clean (no warnings in `examples/sections/`).
- All FIVE examples RUN headless under Xvfb :99 with `NOVA_AUTOPILOT=1`, exit 0:
  `thruster_section` (t=6.2s), `controller_section` (t=11.3s, 0.181 / 0.182 /
  0.187 rad), `turret_section` (t=10.3s, 0.5 and 0.9 deg off, travel 22.1 and
  15.6 u), `torpedo_section` (t=21.4s, lead 69.1 deg, closest approach 14.8 u),
  `hull_section` (t=1.4s). That also answers round 4's "not checked" note on
  `torpedo_section`'s budget: the run takes 21.4s of the 120s backstop, so the
  111s deadline sum is a worst case with wide headroom, now measured rather
  than derived.
- `sections_assert_their_invariant_roster`, `catalog_matches_disk` and
  `examples_name_drivers_through_the_nova_harness` PASS; the roster is
  unchanged, since no marker name moved.
- `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests` green.

Not re-run: the full `examples_smoke` suite. The two failures round 4 recorded
(`systems_`, `ui_`) are outside this diff and are owned by the open
`check/master-scenarios-flake` worktree; nothing in this pass touches
`examples/systems/` or `examples/ui/`.

## Round 5

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R5.1 (MINOR) examples/sections/torpedo_section.rs:778 - `LEAD_SETTLE_SECS`
  is inert in practice and its doc does not describe the run. `lead_sample_
  settled` measures from `BestLeadDeg::first_sample_at` on the GLOBAL clock,
  but the first midcourse sample is taken during the preceding 10s `hold the
  trigger` beat, so the predicate is already true on entry and `lead the
  crosser` lasts one frame (52.316s -> 52.355s in both runs made this round).
  The step therefore buys no extra flight, and the comment above :834 ("each
  fresh torpedo gets another run at the intercept") is not delivered. Not
  MAJOR: the beat still reads no quantity the assert decides, so invariant 5
  stays falsifiable. Concrete change: make the beat
  `and(lead_sample_settled(), elapsed(LEAD_SETTLE_SECS))` so the settle is
  spent in-step, or drop the constant and state that the sample lands in the
  prior beat.
  - Response:
- [ ] R5.2 (MINOR) examples/sections/torpedo_section.rs:763 - the three new
  settle docs describe a clock `elapsed` does not use. `elapsed` counts
  IN-STEP time from step entry (`crates/nova_autopilot/src/predicate.rs:58`),
  but `LAUNCH_SETTLE_SECS` is documented as running "after a torpedo has
  ARMED", `HIT_SETTLE_SECS` (turret_section.rs:658) "after the first round
  leaves the barrel", and `BURN_WINDOW_SECS` (thruster_section.rs:64) "once
  the drive has taken it". Measured: the torpedo trigger step runs 32.016 ->
  42.050 = 10.03s from ENTRY while the arm lands at 32.55, so the
  post-stimulus window the derivation sizes is silently shorter than stated
  and shrinks further if the stimulus is slow. Concrete change: restate all
  three as in-step seconds from the beat's entry, naming the bound on how
  late the stimulus can land, or measure from the stimulus the way
  `lead_sample_settled` does.
  - Response:
- [ ] R5.3 (NIT) examples/sections/thruster_section.rs:282 - the doc "a binding
  that never reaches the section stalls this beat by name" overstates what the
  clause can catch: the only writer of `ThrusterSectionInput` in this rig is
  the example's own `hold_throttle` (:233), which assigns the exact value every
  frame, so the clause can only fail when no active thruster exists - which the
  `load the rig` beat already guarantees. Concrete change: restate as "guards
  against the section being despawned or deactivated mid-round".
  - Response:

Rounds 3 and 4 verified at their cited sites before ticking:

- R3.1 CONFIRMED FIXED - `command_delivered` (controller_section.rs:437-444)
  reads only `sweep_since_spawn` and `command_offset_from_spawn`, both
  command-side; `tracking_error` appears in no beat, and `assert_tracking`
  (:471-476) is the only reader of the hull's error. Ticked.
- R3.2 CONFIRMED FIXED - the beat carries
  `TRACK_TOLERANCE_RAD + COMMAND_RAD_PER_SEC * OFFSET_BEAT_MARGIN_SECS` (:441)
  while the assert keeps the bare constant (:466). Ticked.
- R3.3 CONFIRMED FIXED - automation-harness.md:115-141 now splits the two
  rules: `ComSettle` carries the schedule rule, a second paragraph states
  "sample where the quantity is WRITTEN" and says `AimSettle` stays on
  `Update` because `SmoothLookRotationPlugin` writes in `PostUpdate`. Ticked.
- R3.4 CONFIRMED FIXED - "across as many scenes or rig layouts as its
  invariants need" in both CHANGELOG.md:19-21 and development.md:165. Ticked.
- R4.1 CONFIRMED FIXED - `hold the trigger` is
  `and(torpedo_armed(), elapsed(LAUNCH_SETTLE_SECS))` (torpedo_section.rs:862)
  and `torpedo_armed` (:929) is `fired && armed` only, a strict prefix of the
  four flags `assert_launch_chain` decides; `lead the crosser` (:834) gates on
  `lead_sample_settled()` (:936), which reads `first_sample_at` and no angle,
  leaving `lead > LEAD_MIN_DEG` (:994) as the deciding comparison. Re-derived
  by reading both sites in the recording pass. Ticked.
- R4.2 CONFIRMED FIXED - `and(range_fired(), elapsed(HIT_SETTLE_SECS))`
  (turret_section.rs:767); `gate_took_damage` has no occurrence outside
  `tasks/`; `assert_fired_and_connected` (:925) decides both flags. Ticked.
- R4.3 CONFIRMED FIXED - `GATE_TRAVEL_BEAT_MARGIN` (:653) is on the beat only
  (:904) and the assert keeps the bare `GATE_TRAVEL_MIN`; observed live, the
  round-2 assert read travel 15.0 u against 8.0. Ticked.
- R4.4 CONFIRMED FIXED - `burn_window_held` (thruster_section.rs:288) reads
  `ThrusterSectionInput` plus `elapsed(BURN_WINDOW_SECS)`; no speed or gain in
  the beat, so `assert_full_burn` (:376) decides `gain > 0.0`. Ticked.
- R4.5 CONFIRMED FIXED - `TRACK_SETTLE_SECS` is gone from the source tree and
  `command_delivered` is the two guard clauses alone. Ticked.
- R4.6 CONFIRMED FIXED - CHANGELOG.md:19-25 wraps within the file's width.
- R4.7 CONFIRMED FIXED - development.md:222-229 re-flowed. Ticked.

Checks run in the recording pass (worktree, `nix develop --command`):

- `cargo fmt --all -- --check` clean (empty output, exit 0).
- `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests` green - zero
  hits, so the merged-runs DoD proof holds.
- `cargo test --test examples_smoke -- roster catalog_matches_disk
  examples_name_drivers`: 3 passed, 0 failed.
- The out-of-context reviewer RAN all five `sections/` examples headless under
  Xvfb :99 with `NOVA_AUTOPILOT=1`, exit 0 on each, and re-checked every
  `.until(...)`/`assert_*` pair across the five scripts: no remaining beat
  reads a quantity its assert decides. That is the class rounds 1-4 chased,
  now closed.
- Close-out honesty spot-checked against those runs: torpedo t=21.3s vs 21.4s
  recorded, thruster 21.681/8.608 u/s^2 vs 21.833/8.608, turret 0.4/0.5 deg,
  controller 0.179/0.180/0.184 rad. The torpedo lead angle differs materially
  (34.1-34.3 deg this round vs 69.1 recorded) - run variance in a running
  maximum against a 25 deg floor, not a false record.

Process signal: five rounds, four of them the SAME class (a beat that reads
the quantity its assert decides). Rounds 1-3 fixed it site by site as findings
named sites; round 4's fix pass enumerated all five scripts at once and closed
it. The enumeration should have been the first response, not the fourth - the
retro's material.

Pending user check (not resolvable by review):

- `manual:` reviewer reads the five scripts.

Not checked: the full `examples_smoke` suite (round 4 recorded two failures,
`systems_reach_playing_without_panic` and `ui_reach_playing_without_panic`,
both outside this diff and owned by the open `check/master-scenarios-flake`
worktree; nothing since has touched `examples/systems/` or `examples/ui/`);
flake rate beyond one run per example; behavior above ~64fps.
