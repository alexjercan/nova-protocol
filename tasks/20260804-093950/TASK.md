# Deepen sections/ to multi-scene multi-round runs, merging com_range and torpedo_guidance

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.10.0, content, examples, testing

## Story

Deepen the `sections/` runs and collapse seven examples into five, one per
section family.

Today's section runs are mostly one scene, one section, one beat, with
wall-clock runways where an assertion belongs. With predicate steps a run can
walk several rounds across at least two scenes or section layouts - spawn,
drive, damage, destroy, reload, re-enter, assert the invariant again - and gate
each beat on the value it depends on rather than sleeping past it.

## Steps

Owner call 2026-08-04: "deepen" is bounded by a NAMED INVARIANT LIST per run,
not by a scene or round count. Done means every listed invariant is asserted.
Scenes and rounds are means to that end - if an invariant needs two scenes, the
run gets two scenes; if it does not, padding one in proves nothing.

ROSTER CONFIRMED at planning with one amendment: torpedo #5 is NEW, not
merged. `torpedo_guidance` only LOGS closest approach
(`track_closest_approach` -> `BestApproach`, torpedo_guidance.rs:234-245) and
asserts nothing; its autopilot is `nova_autopilot().input(autopilot_fire)`
(:60). Total stays 27: 14 exist, 4 merged, 9 new.

Mechanics all five runs adopt (see DECISION.md):

- Explicit `AutopilotPlugin` step list per run, replacing the
  `nova_autopilot()` wall-clock preset and the hand-rolled `elapsed <` stage
  offsets. `com_range.rs:70-101` and `systems/outcomes.rs:76-155` are the
  shapes to copy; every step carries `.deadline(...)` so a stall names the
  beat. Keep the deadline sum well under `DEFAULT_DEADLINE_SECS` (120s).
- RELOAD is two steps, no new API and no state:
  `.step("tear the rig down").on_enter(<re-trigger LoadScenario>)
  .until(nova_autopilot::predicate::not(any_entity::<With<SpaceshipRootMarker>>()))`
  then `.step("wait for the fresh rig").until(any_entity::<...>())`. The gap
  is real: `on_load_scenario` despawns scoped entities immediately and the
  ship respawns later off `OnStartEvent`
  (`nova_scenario/src/loader/lifecycle.rs:161-253`). `not` must be qualified -
  it clashes with `bevy::prelude::not`.
- Every invariant emits exactly ONE `nova_probe::probe_marker` named
  `outcome: <slug>` beside its `assert!`. The names are the roster; Step 6
  pins them. Existing markers that cover two invariants get split.
  ONE EXCEPTION, named in the roster test's doc (review round 1, R1.4): a
  ROUND-COMPLETION invariant ("1-7 hold again after the reload") has no assert
  of its own, because the fact it claims IS that the round's other asserts
  passed. It rides the round's LAST assertion, guarded on the round label,
  rather than getting a marker-only step.
- `AutopilotPlugin` must be named `nova_protocol::nova_debug::harness::` -
  qualified (`examples_name_drivers_through_the_nova_harness` enforces it).

Each step below is an atomic green checkpoint: run the example before
committing (`Xvfb :99 & DISPLAY=:99 NOVA_AUTOPILOT=1 nix develop --command
cargo run --example <name> --features debug`). `cargo check` does not catch
duplicate-component panics.

- [x] 1. `controller_section` (examples/sections/controller_section.rs) - 4
      invariants (2 exist, 2 new). Script: load rig A (controller + hull) ->
      assert 1 (command swept > 0.6 rad, :200) and 2 (tracking error <
      `TRACK_TOLERANCE_RAD`, :213) -> reload rig A -> assert 4 (tracking
      re-converges) -> load rig B (layout with a second hull offset off-axis,
      so the inertia tensor differs) -> assert 3 (tracking holds). Two rig
      builders, both local. Wait for tracking with a predicate on the live
      `Rotation`-vs-command error, not `elapsed`.
- [x] 2. `thruster_section` (examples/sections/thruster_section.rs) - 5
      invariants (3 exist, 2 new). Script: full burn -> assert 1 (nose speed
      grew, :189), 2 (plume material exists, :204), 3 (plume input == 1.0,
      :216) -> partial throttle round -> assert 4 (nose-speed gain at the
      partial setting is positive and strictly below the full-burn gain
      measured in round 1) -> release -> assert 5 (plume input returns to 0).
      Invariant 4 compares two MEASURED gains rather than a magic constant.
- [x] 3. `hull_section` (examples/sections/hull_section.rs) - 8 invariants (3
      exist, 4 merged from `com_range`, 1 new). The rig becomes `com_range`'s
      5-section line (controller, hull1..3, thruster) under
      `SpaceshipController::Player` with an empty `input_mapping` - required,
      not cosmetic: the chase camera (invariant 7) only follows a player ship.
      Round order matters:
      R1 damage a REAR hull: assert 1 (partial hit subtracts exactly, :193),
      2 (overkill destroys and despawns, :220), 3 (root + controller survive,
      :230,:234) - all three need the controller alive, so R1 comes first.
      R2 spin, kill the controller, kill hull1, settle: assert 4 (drift < 0.3,
      com_range.rs:381), 5 (local COM moved aft), 6 (`TransformInterpolation`,
      :394), 7 (camera anchor drift < 0.5, :408). RECOMPUTE the aft threshold
      from the surviving set - R1 already removed a rear hull, so
      com_range's `local_com.z > 2.4` / "near 2.75" no longer describes this
      run. Port `com_range`'s gizmos, hotkeys and status log with the beats.
      R3 reload, repeat R1+R2: assert 8 (1-7 hold again).
      Then delete examples/sections/com_range.rs, its `[[example]]` block
      (Cargo.toml:82-84) and its `SECTIONS` entry (tests/examples_smoke.rs:39).
- [x] 4. `turret_section` (examples/sections/turret_section.rs) - 4 invariants
      (2 exist, 2 new). Script: fire round -> assert 1 (a round fired, :478)
      and 2 (a gate took hits, :479) -> assert 3 (aim error vs the SWEEPING
      gate converges: the error `report_status` already computes, :397, falls
      below a stated threshold while the gate is moving) -> reload -> assert 4
      (1-3 again). Keep `turret_section/slider.rs` untouched.
- [x] 5. `torpedo_section` (examples/sections/torpedo_section.rs) - 6
      invariants (4 exist, 1 merged scene, 1 new). Script: gate round ->
      assert 1-4 (fired/armed/detonated/gate damaged, :517-:523) -> load the
      crosser scene (port `torpedo_guidance`'s crossing target, its
      `range_autotarget` lock-at-launch and its `BestApproach` tracker) ->
      assert 5 (closest approach falls under a stated bound - NEW, the source
      run only logged it) -> assert 6 (the fired/armed/detonated chain
      repeated in this second scene). Keep the run's longer runway budget
      (`RANGE_AUTOPILOT_SECS`, :52) as per-step deadlines.
      Then delete examples/sections/torpedo_guidance.rs, its `[[example]]`
      block (Cargo.toml:78-80) and its `SECTIONS` entry
      (tests/examples_smoke.rs:38).
- [x] 6. Pin the roster: `sections_assert_their_invariant_roster` in
      tests/examples_smoke.rs - a display-free source grep (same class as
      `examples_name_drivers_through_the_nova_harness`, :217) asserting each
      sections example contains each of its named `outcome: <slug>` markers,
      and that the totals are 27 across the five files. This is the stopping
      rule made executable; the run passing is what proves they HOLD.
- [x] 7. Docs in the same task: `web/src/wiki/dev/development.md:162-163`
      (drop both merged names from the sections list), `:220` (`torpedo_guidance`
      cited as a scenario-load asserter), `:251` (`com_range` cited as the
      wait-on-the-world style - repoint to `hull_section`);
      `web/src/wiki/dev/automation-harness.md:115-129` (the before/after
      worked example is `com_range`'s script - repoint to `hull_section`);
      one `Internals & Tooling` CHANGELOG line under `[Unreleased]`. Historical
      CHANGELOG and `web/src/news/0.4.0.md` mentions stay - they are release
      history, not live references.
- [x] 8. Build the ship fixture LOCALLY here. Do NOT extract a shared builder:
      owner call 2026-08-04, `20260804-094006` is the third caller and does the
      extraction, having seen all three shapes. One caller is not an
      abstraction. This applies to the ship builders only - the reload gate is
      two lines of existing predicate vocabulary per run, not a helper.

## Definition of Done

- Every one of the 27 invariants is NAMED by its run: the roster test lists
  them and each is present as an `outcome: <slug>` marker beside its assert.
  Red on base (no sections example carries a roster today).
  (test: `sections_assert_their_invariant_roster`)
- Every invariant HOLDS: all five runs pass headless, where a failed assert
  panics the process and `process_exit` fails the report
  (nova_probe/src/run_report/checks.rs:130-133).
  (cmd: `nix develop --command cargo run -p nova_probe -- run sections`)
- No run advances a beat on `elapsed` where a world value is available: every
  script is an `AutopilotPlugin` step list, and `elapsed`/`frames` appear only
  as settle beats with a stated reason.
  (manual: reviewer reads the five scripts)
- The two merged runs are gone and their assertions live in the absorbing run.
  Red on base (both names are still referenced).
  (cmd: `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests`)
- The catalog, disk and smoke lists agree after the merges. A CONSISTENCY
  GATE, not a red proof: green on base, and it goes red the moment a file is
  deleted without its catalog and smoke entries.
  (test: `catalog_matches_disk`)

## Notes

Roster per the spike (`20260804-003244`) - each run gets harder, not thinner:

| Run | Change |
| --- | --- |
| `controller_section` | Deepen. PD attitude control across multiple layouts and repeated rounds. |
| `thruster_section` | Deepen. Throttle -> impulse + plume, same shape. |
| `hull_section` | Deepen, ABSORBS `com_range`. |
| `turret_section` | Deepen. PDC tracking + firing. |
| `torpedo_section` | Deepen, ABSORBS `torpedo_guidance`. |

The two merges:

- `com_range` -> `hull_section`. `hull_section` owns the damage -> destroy
  pipeline, and COM-follows-destruction is that pipeline's consequence, not a
  separate subject. `com_range.rs:374` (`assert_com_follows_sections`) becomes
  a round after the destroy round. `com_range` is already predicate-driven, so
  the beats port directly.
- `torpedo_guidance` -> `torpedo_section`. Both are the torpedo bay family and
  one example per family is the contract. The PN closest-approach assertion
  becomes the lead-a-crosser round of the merged run.

- Assert through predicates on the values the section family owns (mass/COM,
  thrust, integrity, guidance, lock, range), not through elapsed time.
- `turret_section` carries a 203-line interactive slider submodule for human
  tuning. It stays; if the probe path never touches it, a later task may
  extract it to a shared dev-widget module. Not blocking here.
- Ship builders stay LOCAL here. `20260804-094006` extracts the shared `fn`
  with its count knob as the third caller (owner call 2026-08-04), designing
  the signature from three visible shapes instead of from this one.
- `sections/` carries no fps window.
- Examples must be RUN under Xvfb :99, not only checked.

## Step 4 close-out - `turret_section`

WHAT: two-round predicate script (fire round -> reload -> fire round) carrying
4 named invariants: `turret fired`, `gate damaged`, `turret tracks the mover`,
`turret invariants hold after reload`. Steps 5-8 remain.

Four defects surfaced, all of which `cargo check` and a single-round run miss.
Each was found by measuring, not by reading:

1. **Lost trigger edge.** The held-input presses were an unordered `Update`
   system, racing `SpaceshipInputSystems`. The stance still went hot (the
   safety reads the HELD button) but the trigger's one `just_pressed` edge was
   cleared before the weapon read it, so the range aimed to 0.1 deg and never
   fired. Moved to the script's `AutopilotPlugin::input` hook, which exists for
   exactly this and runs in `PreUpdate` after `InputSystems`
   (nova_autopilot/src/autopilot.rs:384).

2. **Latched key across the reload.** Round 2 re-pressed a key that was still
   `pressed` from round 1, which is not an edge. The reload now RELEASES it.

3. **Round opened before its target existed.** `open_fire` captured the mover's
   start position at step entry, but the reload gate waited only on the ship.
   A `None` capture stalled the tracking beat silently for its full 30s
   deadline, intermittently. Both round-opening beats now wait on
   `moving_gate_present()`, and a missing mover panics instead of stalling.

4. **The mover did not move.** The root cause of the residual ~50% flake. As a
   dynamic body the gate was pulled by the planetoid, shoved by bullet impacts,
   damaged to death by contact with `gate_front` where its sweep crossed x = 0,
   and - measured directly - carried an 18 u/s `LinearVelocity` on the driven
   axis while translating ~0.05 u/s. It is now a KINEMATIC body placed on an
   authored sine path, with `LinearVelocity` written as that path's analytic
   derivative because `range_aim` feeds it to the turret's lead solution. The
   amplitude (22 u) keeps the path clear of the static gates.

ALTERNATIVES: raising the mover's lane (made it worse - a 2000-damage one-shot
at spawn); `SleepingDisabled` alone (kept, but did not move the flake rate);
`invulnerable` on the mover (rejected - invariant 2 must OBSERVE damage on it).

EVIDENCE: 8/8 consecutive `cycle complete, no panic` at ~5s, after a measured
1-in-2 stall rate before the kinematic change. `cargo fmt` + `clippy` clean.

REFLECTION: every one of these four reads as green under `cargo check` and
three of the four under a single-round run. The multi-round script is what
exposed them - which is the point of this task. The debugging turned on
logging the two quantities the failing predicate waits on; the aim-error log
alone looked healthy throughout and pointed the wrong way for several rounds.

NEXT: Step 5 (`torpedo_section` + `torpedo_guidance` merge).

## Steps 5-8 close-out - `torpedo_section`, the roster test, the docs

WHAT: `torpedo_section` becomes a two-scene predicate script carrying 6 named
invariants - gate round (`torpedo fired`, `torpedo armed`, `torpedo detonated`,
`gate damaged`), then a load of a second scene built locally (`crossing_range`,
porting `torpedo_guidance`'s crossing target, its lock-at-launch
`range_autotarget` and its `BestApproach` tracker) carrying `launch chain holds
in the crossing scene` and `torpedo leads the crosser`.
`examples/sections/torpedo_guidance.rs` is deleted with its `[[example]]` block
and its `SECTIONS` entry. `sections_assert_their_invariant_roster`
(tests/examples_smoke.rs) pins all 27 slugs, matched BOTH ways per file, plus
the 27 total and the roster-set == `SECTIONS` identity. Docs: the sections
list, the scenario-load-asserter caveat and the wait-on-the-world citation in
`development.md`, the `com_range` worked example in `automation-harness.md`
(kept as history, relabelled and repointed at `hull_section`), the `sections/*`
row in the probe skill's marker table, and one `Internals & Tooling` CHANGELOG
line. Ship fixtures stay LOCAL (`torpedo_ship`, :220) per Step 8;
`20260804-094006` is the third caller that extracts.

WHY / ALTERNATIVES: invariant 5 was planned as "closest approach under a stated
bound" and had to change metric - the proximity fuze detonates and despawns at
half the blast radius, so the metric floors at ~14.5 u and any satisfiable
bound is already implied by invariant 3. It asserts the LEAD ANGLE in late
midcourse (16-30 u) instead, which is the claim the merge was for; full
reasoning and the rejected alternatives are in DECISION.md. Second amendment:
`turret_section` and `hull_section` both stated deadline sums summed from
source literals while their round helpers are called twice, so the real
runtime sums (190s, 135s) sat over the 120s run backstop the ordering exists to
beat. Deadlines are resized off measured runs to 97s and 85s.

DIFFICULTIES: the lead-angle sample window is the load-bearing part - a torpedo
leaves the bay pointing down the ship's nose, which for this geometry already
sits ~24 deg off the line of sight ON THE SAME SIDE as the lead, so an early
sample cannot distinguish "has not turned yet" from "is leading". Bounding the
sample to 16-30 u excludes that transient and stays outside the fuze.

EVIDENCE:
- `cargo run -p nova_probe -- run sections`: aggregate OK, all five ranges
  5/6 checks (fps SKIPPED, no baseline) - controller 12s, thruster 8s, hull 6s,
  turret 11s, torpedo 11s.
- `cargo test --test examples_smoke`: `sections_assert_their_invariant_roster`,
  `catalog_matches_disk`, `examples_name_drivers_through_the_nova_harness` pass.
- `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests` - no hits.
- `cargo check --examples --features debug` clean; `cargo fmt --check` clean.
- `manual:` (reviewer reads the five scripts) stays pending. Supporting grep:
  the only `elapsed`/`frames` left in `sections/` is
  `controller_section.rs:356`, ANDed into `tracking_converged` as a settle
  clause with its reason stated at `TRACK_SETTLE_SECS`.

REFLECTION: the plan named a metric for the new invariant before anyone had
measured what that metric can physically read, and the fuze made it
unsatisfiable-or-vacuous with no middle. Cheap to catch - one instrumented run
printed the floor - but only because the metric was written down and therefore
falsifiable. The deadline-sum bug is the same class read from the other side: a
number stated in a doc comment and never derived from the shape of the code
that consumes it.

## Review round 1 close-out - the settle gates and the sweep epoch

WHAT: answered all seven round-1 findings. R1.1 traded `controller_section`'s
one-method `ReloadStep` extension trait for `fn then_reload(script, layout)`,
so all five scripts append rounds the same way. R1.2 removed the wrapping
sweep guard. R1.3 rewrote three settle gates that were tautological with the
asserts that followed them. R1.4 folded `hull_section`'s marker-only reload
step into the round's closing assertion. R1.5 tightened the spawn-COM freeze.
R1.6 repointed a stale wiki parenthetical. R1.7 stays deferred to
`20260804-094006`, which is the recorded third-caller extraction.

WHY R1.3 is the substantive one: `mass_properties_settled`, `aim_converged`
and `plume_idle` each gated their step on the exact condition the next step
then asserted, with the same constants. That makes the assert unfailable and
converts every real regression into a deadline stall on an unrelated beat
name. The fix is the same shape in all three - gate on the world having
STOPPED CHANGING, assert on WHERE it stopped:

| run | gate now | assert still |
| - | - | - |
| hull | 2 sections left on the root, local COM steady 3 frames | drift < 0.3, camera drift < 0.5 |
| turret | gate travelled > 8u, aim error steady 5 frames | error < 3 deg |
| thruster | plumes exist, 2 frames since the release | every input == 0 |

ALTERNATIVES: R1.2 was filed as "zero the sweep epoch and pass `now - epoch` to
`command_at`". Taken in substance, rejected literally - resetting the COMMAND
would snap it back to identity exactly where a fresh rig spawns, so the reload
gate's "the live rig is far off the command" could never hold and that beat
would stall its full deadline. Instead the command keeps sweeping (which is
what makes invariant 4 a catch-up claim), `sweep_since_spawn` replaces the
periodic `command_offset` with the integrated angle since the rig APPEARED,
and the reload gate drops angles entirely for entity identity: `RigEpoch`
records the root being replaced and waits for a different one. R1.3's filed
"`ComputedMass` dropped below the pre-kill value" was likewise taken in
substance but not literally - the round kills two sections, so one
below-threshold reading cannot tell one propagated despawn from two.

DIFFICULTIES: the first attempt at R1.3 used a LOOSER tolerance on the same
quantity (gate at 4x the assert bound). It reads as weaker but is not safe: a
stale pre-solve COM sits ~0.7u from the new centroid, inside a 1.2u slack gate,
so the beat would have opened on a stale value and invariant 4 would have
failed spuriously. Diagnosed by computing what the stale reading actually is
from the rig geometry rather than by running it. The stability-plus-count gate
has no such window - it cannot open until the solve has stopped moving.

EVIDENCE:
- All five examples RUN headless under Xvfb :99 with `NOVA_AUTOPILOT=1`, exit
  0, `autopilot: cycle complete, no panic`. controller now logs
  `command swept 1.208 rad since the rig spawned` on all three rounds (the
  same in-round figure each time, which is the monotonic guard working).
- `cargo run -p nova_probe -- run sections`: aggregate OK, five ranges 5/6
  (fps SKIPPED, no baseline). 26 distinct `outcome:` slugs in the timelines -
  27 roster entries with `gate damaged` shared by turret and torpedo.
- `sections_assert_their_invariant_roster`, `catalog_matches_disk`,
  `examples_name_drivers_through_the_nova_harness` pass.
- `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests` - no hits.
- `cargo check --examples --features debug` and `cargo check --examples`
  (no-debug, for the cfg gating) clean; `cargo fmt --check` clean.
- `manual:` stays pending. `sections/` now holds exactly two time-shaped
  predicates: `controller_section`'s `elapsed(TRACK_SETTLE_SECS)` and
  `thruster_section`'s `frames(RELEASE_SYNC_FRAMES)`, both settle beats with
  their reason stated at the predicate.

REFLECTION: "wait for X, then assert X" reads as rigorous and is the opposite -
it is a test that cannot fail, wearing the costume of one that can. It slipped
into three of five runs independently, which means it is a shape to look for
rather than three separate mistakes: whenever a beat's predicate and the next
step's assert share a constant, one of them is decorative. The general rule
that came out of it - gate on the world having stopped changing, assert on
where it stopped - is worth carrying into the next predicate-driven run.

## Review round 2 close-out - the physics schedule and the delivery guard

WHAT: five findings, all fixed, no pushback. Two themes.

Theme one (R2.1, R2.2, R2.3) is round 1's own fix applied wrongly. Round 1
replaced three tautological settle gates with "the value held still for N
frames" - and on `Update` that does not mean "the solve has run". Avian's
`MassPropertyPlugin` defaults to `FixedPostUpdate`, so above the fixed rate
most `Update` frames carry no solve pass at all. Both hull COM samplers moved
to `FixedPostUpdate` after `PhysicsSystems::Prepare`, so one sample is one
recompute (`still_frames` -> `still_passes`, `COM_SETTLE_FRAMES` ->
`COM_SETTLE_PASSES` - the names carried the wrong claim). `ComSettle` now
carries the whole reading in one `ComSample`, and `mass_properties_settled`
reads BOTH its clauses from it rather than reading the section count live off
the world - that live read was the actual failure path, since a despawn flushes
on `Update` and can pair a fresh count with a stale COM's saturated streak.
Turret's `AIM_STEADY_DELTA_DEG` (per frame) became
`AIM_STEADY_RATE_DEG_PER_SEC` (per second), divided by `Time::delta_secs()`.

Theme two (R2.4) is a substitution that dropped a property. Round 1 swapped the
periodic `command_offset` for the monotonic `sweep_since_spawn` to kill a wrap
stall, and the delivery guard silently stopped excluding what it was named for:
arc length travelled says nothing about where the command IS, so a dead PD
parked at spawn could clear both invariants together. The guard is now
explicitly two claims - the monotonic sweep as the beat, plus
`command_offset_from_spawn > TRACK_TOLERANCE_RAD` as the exclusion - and
`RigEpoch` stamps the rig's real spawn attitude instead of assuming identity.

ALTERNATIVES: R2.2 offered `Ref::is_changed()` instead of the schedule move;
took the move so both COM samplers rest on the same rule stated once. R2.3
offered a fixed sampling cadence; rejected with reason - the aim error is
written on `Update`, so consecutive fixed ticks inside one frame would read the
same value and saturate the streak mid-slew, the opposite failure.

DIFFICULTIES: the R2.4 fix needed the new clause on the BEAT as well as the
assert. Putting it only on the assert would have panicked a healthy PD whenever
the beat opened near the spawn attitude. Adding it to the beat is safe where
round 1's 1.2 rad guard was not, because 0.35 rad is under water for ~2s of
each ~18s turn rather than ~7s.

EVIDENCE:
- All five examples RUN headless under Xvfb :99, exit 0. hull asserts COM and
  camera drift 0.000 on both rounds and got FASTER (5s -> 1.4s standalone) -
  the expected sign, the beat now opens on the first settled solve. turret
  asserts 0.6 deg then 0.4 deg (tolerance 3.0) and the log shows the rate gate
  correctly refusing the 24.9 / 28.8 deg acquisition frames. controller asserts
  0.184 / 0.180 / 0.186 rad lag across its three rounds.
- `cargo run -p nova_probe -- run sections`: aggregate OK, five ranges 5/6.
- `sections_assert_their_invariant_roster` and `catalog_matches_disk` pass;
  `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests` no hits;
  `cargo check --examples --features debug`, `cargo check --examples` and
  `cargo fmt --check` clean.
- `manual:` stays pending.
- NOT verified: behavior above ~64fps. llvmpipe caps this box at ~14fps, so
  R2.1-R2.3 were fixed from the schedules, not reproduced. The fixes are
  framerate-independent by construction, which is the point, but the failures
  they remove remain unobserved here.

REFLECTION: round 1's retro rule - "gate on the world having stopped changing,
assert on where it stopped" - was right and still produced three wrong gates,
because "stopped changing" was measured on the wrong clock. The missing half is
that a settle predicate has to be sampled on the schedule that PRODUCES the
value; `Update` is the default and is almost always the wrong one for a physics
quantity. That now lives in `automation-harness.md` rather than only here.
Second, R2.4 is the cost of a substitution: `sweep_since_spawn` fixed exactly
what R1.2 filed and quietly dropped a property the old quantity carried
incidentally. When a fix swaps the QUANTITY a guard reads, the guard's doc
comment is the checklist - it had named the frozen-hull exclusion all along.

## Review round 3 close-out - the beat is the stimulus, the assert is the response

WHAT: four findings, all fixed, no pushback. Two of them (R3.1, R3.2) are the
same defect rounds 1 and 2 chased, at its last two hiding places, and both live
in `controller_section`'s converge beat.

R3.1: round 1 filed "beat and assert share a constant" against three files and
fixed those three. The fourth site was never scoped, so controller's beat still
required `tracking_error < TRACK_TOLERANCE_RAD` - the exact comparison its
assert makes - and invariants 2, 3 and 4 could not fail. Fixed by removing the
clause rather than by replacing it with a settle: the two surviving clauses are
both about the COMMAND, so the beat now reads nothing the assert reads. Names
followed the meaning - `tracking_converged` became `command_delivered` and the
three `converge on ...` beats became `sweep the command clear of <rig>'s spawn
attitude`, because a beat name is what a stall message carries and these beats
no longer wait for convergence.

R3.2: the round-2 fix put `command_offset_from_spawn > TRACK_TOLERANCE_RAD` on
the beat AND in the assert, but that quantity is a triangle wave and the
autopilot enters the next step on the following frame, so a beat opening on the
descending edge would hand the assert a failing value and panic a healthy PD.
The beat now carries `OFFSET_BEAT_MARGIN_SECS` (0.5s of sweep, 0.175 rad) of
margin over the assert.

R3.3 and R3.4 are doc accuracy: the round-2 harness note stated the schedule
rule and then cited `AimSettle`, which deliberately does NOT follow it, as
evidence; and CHANGELOG/development.md claimed "at least two scenes or rig
layouts" of all five runs, which is false of `thruster_section`.

ALTERNATIVES: R3.1 offered an error-rate settle in `AimSettle`'s shape.
Rejected as the weaker option here - turret needs one because its response has
a transient the stimulus does not bound, whereas this run's runway is already
set by the sweep guard (3.4s, against a PD that converges in a fraction of it).
Gating purely on the stimulus is the stronger property and costs a resource
less.

DIFFICULTIES: the sabotage run cost the edits. `git checkout -- <file>` to
restore the sabotaged constant reverted the whole file to HEAD, including the
uncommitted fix - verify.md says "commit before each sabotage" and this is why.
Reapplied and committed first, then re-verified.

EVIDENCE:
- SABOTAGE, the load-bearing one for R3.1: with `TRACK_TOLERANCE_RAD` cut to
  0.01 the run panics `attitude probe (rig a): hull is 0.183 rad off the
  command (tolerance 0.01); the PD is not tracking` at
  controller_section.rs:478. Before the fix the same sabotage stalled the beat
  and never reached the assert. Constant restored, re-run green.
- controller RUN headless: three rounds assert at 0.182 / 0.180 / 0.188 rad
  lag, offsets 1.579 / 2.806 / 2.251 rad against the 0.525 rad beat threshold.
- `cargo run -p nova_probe -- run sections`: aggregate OK, five ranges 5/6
  (fps SKIPPED, no baseline).
- `sections_assert_their_invariant_roster`, `catalog_matches_disk` and
  `examples_name_drivers_through_the_nova_harness` pass;
  `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests` no hits;
  `cargo check --examples --features debug`, `cargo check --examples` and
  `cargo fmt --check` clean.
- `manual:` stays pending.
- NOT verified: behavior above ~64fps, same as round 2 - llvmpipe caps this box
  at ~14fps, so R3.2's descending-edge window is reasoned about, not observed.

REFLECTION: the through-line of all three rounds is that a class-shaped finding
got fixed at its cited sites. R1.3 named the shape precisely and listed three
`file:line`s; the fix took the list as the scope, and the fourth site shipped
two more rounds. When a finding says "same shape in X and Y", the remedy is a
grep for the shape, not a visit to X and Y. Second, every round's fix created
the next round's bug in the same predicate - R1.2's substitution caused R2.4,
R2.4's fix caused R3.2 - because each one changed WHAT a guard reads without
re-deriving what the guard had guaranteed. The rule now written into
`automation-harness.md`, that a beat must be strictly weaker than its assert
and needs a margin when they share a quantity, is the general form of both.

## Review round 4 close-out - one sweep, and settle beats where no stimulus exists

WHAT: closed the beat-stronger-than-its-assert class across ALL five scripts in
one pass instead of at the four cited sites.

- `torpedo_section`: `hold the trigger` gates on `fired && armed` plus
  `LAUNCH_SETTLE_SECS` (10s, from ~90 u at a ~35 u/s cap); `lead the crosser`
  stops reading the angle entirely - `BestLeadDeg` now records
  `first_sample_at` and `lead_sample_settled()` waits for a midcourse leg to
  have been flown. `assert_launch_chain` and `assert_leads_the_crosser` are the
  deciding comparisons for five invariants that could not fail before.
- `turret_section`: `hold the trigger` gates on `range_fired()` plus
  `HIT_SETTLE_SECS` (4s, from the ~140 deg/s slew and 48-65 u gates at 100 u/s);
  `gate_took_damage()` is deleted. The tracking beat gained
  `GATE_TRAVEL_BEAT_MARGIN` (6.6 u = half a second of the sweep's peak speed)
  because its travel clause IS the assert's guard, and the sweep is a sine, so
  the shared threshold could be crossed downward.
- `thruster_section`: `burn_window_held(throttle)` gates on the drive having
  taken the throttle off the production `ThrusterSectionInput` seam plus
  `BURN_WINDOW_SECS` (2.5s), so `gain > 0.0` and `rate > 0.0` decide.
- `controller_section`: the inert `elapsed(TRACK_SETTLE_SECS)` and its constant
  are gone; the sweep guard already forces 3.4s of chasing before the beat can
  open, which is what the settle was documented to buy.
- `hull_section`: checked, unchanged. Its two pairs are already strictly weaker
  (`<=` beat against an exact-equality assert; settle-passes/attached-count beat
  against COM-drift asserts).

WHY: a beat that reads the quantity its assert decides makes the assert
unfailable and turns every regression into a deadline stall on the beat's name.
Nine of the 27 invariants were in that state.

ALTERNATIVES: the reviewer's concrete change for the lead beat was a margin over
`LEAD_MIN_DEG`. Rejected with reasoning on the finding: a margin works on a
shared GUARD clause (R4.3, and `OFFSET_BEAT_MARGIN_SECS`) but on the DECIDING
comparison it just re-creates the defect. The stimulus-side observable the beat
needed - "a midcourse sample was taken" - did not exist, so it was added.

DIFFICULTY: settle constants are runways, which this task spent three rounds
removing. The distinction that makes them legitimate: a settle bounds the
MECHANISM (flight time, slew time, one shader sync) and never the outcome, and
it owes a derivation on its constant. That rule is now written into
`automation-harness.md` beside the weaker-than rule.

EVIDENCE:
- All FIVE examples RUN headless (Xvfb :99, `NOVA_AUTOPILOT=1`), exit 0:
  thruster t=6.2s (21.833 vs 8.608 u/s^2), controller t=11.3s (0.181 / 0.182 /
  0.187 rad), turret t=10.3s (0.5 and 0.9 deg, travel 22.1 and 15.6 u), torpedo
  t=21.4s (lead 69.1 deg, approach 14.8 u), hull t=1.4s.
- torpedo's 21.4s against the 120s backstop measures the budget round 4 could
  only derive: the 111s deadline sum is a worst case with wide headroom.
- `sections_assert_their_invariant_roster`, `catalog_matches_disk`,
  `examples_name_drivers_through_the_nova_harness` pass; no `com_range` or
  `torpedo_guidance` hits; `cargo clippy --examples --features debug` and
  `cargo fmt --all -- --check` clean.
- `manual:` stays pending.
- NOT re-run: the full `examples_smoke` suite - round 4's two failures
  (`systems_`, `ui_`) are outside this diff and owned by the open
  `check/master-scenarios-flake` worktree.

REFLECTION: round 3's reflection said the remedy for a class-shaped finding is a
grep for the shape, and round 4 filed the same class again anyway - because the
fix pass still worked from the finding list. What actually closed it was
enumerating every `.until(...)`/`assert_*` pair in the five files first and
judging each, including the pairs no round had cited. The cheap version of that
is a mechanical listing, not a careful reading: the pairs are greppable.
