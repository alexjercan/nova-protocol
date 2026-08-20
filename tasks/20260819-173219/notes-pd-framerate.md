# Point defence scales with frame rate

Lane `pd-framerate`, branched from `a145fdab`. Every number below is a COUNT
measured on that base; nothing here is a millisecond, and nothing here was
rebased mid-measurement.

**STATUS: mechanism CONFIRMED and CORRECTED. See "The mechanism" - the split is
real, but the decision that follows the render clock is not the trigger.**

## The instrument

The range could not previously answer the question, because a peak and a total
say nothing about which clock produced them. `stress_point_defense` now carries
a frame-rate axis (`48e27698`):

- **`NOVA_STRESS_PD_FRAME_MS`** holds each frame open to a floor, so one binary
  on one box reproduces a host of any speed. A floor, never a cap: a frame that
  already overruns is left alone.
- **Counts per FIXED STEP**, beside the per-second ones. Rounds are spawned on
  the fixed clock, so `rounds/step` is the quantity a frame rate must not move.
- **Trigger duty**: the fraction of mount-steps the battery held its trigger
  down, censused in `FixedUpdate`.
- **Mean muzzle aim error** over engaged mount-steps, in degrees, printed
  beside the 0.917 deg fire gate.
- **A per-second census trace** (`debug!`), because the fast arm STALLS before
  it reaches any assertion, and a stalled run must still say what it did.

Protocol: `NOVA_NORENDER=1 NOVA_AUTOPILOT=1`, default scale (12 mounts,
12 bays), one run per frame floor.

## Before: the outcome is a linear function of the frame rate

| frame floor | fps | frames/step | trigger duty | rounds/step | rounds/s | intercepts | envelope fill |
|---|--:|--:|--:|--:|--:|--:|--:|
| 8 ms | 106 | 1.65 | **0.606** | **11.07** | 709 | 782 @ 90 s | **never** (stall) |
| 16 ms | 60 | 0.93 | **0.353** | **6.37** | 408 | 34 | 10.1 s |
| 33 ms | 30 | 0.47 | **0.173** | **3.01** | 192 | 47 | 9.4 s |
| 50 ms | 20 | 0.31 | **0.097** | **1.49** | 95 | 55 | 9.2 s |

Trigger duty divided by frames-per-step: 0.367, 0.380, 0.368, 0.313. **The
battery's duty cycle is proportional to how many frames the host fits into one
physics step, across a 5x span, with no other term.** Rounds per step follows it
(`rounds/step ~= 18.3 x duty`, twelve mounts against one cadence).

Extrapolated, duty reaches 1.0 at about 2.7 frames per step - roughly 170 fps.
So the shipped span is a battery at **9.7% duty on a 20 fps machine and ~100% on
a 170 Hz one**.

**The range's pass/fail follows it too.** At and above ~60 fps the envelope never
fills inside `FILL_DEADLINE_SECS`, because point defence kills torpedoes faster
than twelve bays launch them; the run aborts at
`step 'open the tubes' stalled after 90.0s`. Free-running, 4 ms and 8 ms all
stalled on a quiet box; 16 ms stalled on one run and filled in 10.1 s on
another, which is the 60 fps tipping point and the reported "fails about two
runs in five".

This also explains the previously unexplained spread. The rounds yield spanned
708-2,425 free-running and 2,236-2,401 pinned: the pin removes the frame-rate
variance, and it is the only thing it removes.

## The mechanism

CONFIRMED that decision and consequence live on different clocks. CORRECTED on
which decision.

The rounds are spent by `shoot_spawn_projectile`
(`crates/nova_ship/src/sections/turret_section/mod.rs:251`) in `FixedUpdate`,
and its cadence timer ticks on the fixed clock, so the spawn RATE is already
invariant - given an open gate. Both gates that open it are geometric:

- `update_point_defense_trigger`
  (`crates/nova_ship/src/input/point_defense/ownership.rs:283`) asks
  `mount_may_shoot`, a 0.917 deg bearing cone;
- `shoot_spawn_projectile` asks `muzzle_on_target` again per muzzle,
  against the same aim point.

Both read geometry that advances **once per FRAME**:

| what | where it is written | schedule |
|---|---|---|
| torpedo position -> `TurretSectionTargetInput` | `update_point_defense_aim` | `Update` |
| lead solve -> `TurretSectionAimPoint` | `update_turret_aim_point` | `PostUpdate` |
| hinge demand -> `SmoothLookRotationTarget` | `update_turret_target_joints_system` | `PostUpdate` |
| hinge angle -> `SmoothLookRotationOutput` | `smooth_look_rotation_update_system` | `PostUpdate` |
| hinge angle -> joint `Transform` | `sync_turret_joint_rotation` | `Update` |

So the barrel's pointing direction is a staircase with one step per frame, and
the fire gate is 0.917 deg wide. The residual of the hinge damper following a
crossing target is

```text
e* = omega * h / (1 - exp(-AIM_CORRECTION_RATE * h))
```

with `h` the FRAME period: `omega/r` as `h -> 0`, and growing with `h`. Add the
pipeline's own one-frame transport delay and the barrel's error is roughly
proportional to `h` over the range that matters. The gate does not move.
**The 0.917 deg cone sits inside the band that residual sweeps between 20 fps
and 170 fps, so the frame rate decides whether the gate is open at all.**

Measured mean aim error over engaged mount-steps: 11.6 deg at 20 fps, 12.3 at
30, 11.3 at 60, **3.97 at 106**. (The mean is dominated by mounts mid-slew, so
read the trend, not the absolute.)

Task `20260816-184718` made the damper's RATE frame-rate invariant, which
removed the leading-order term. It could not remove `h` from the expression
above, because `h` is the schedule's own period - the exponential form is exact
for a HELD target and this target ramps.

## What was tried, and what each arm measured

Three arms, one binary each, same protocol, same box, same scene.

### Arm A - the hypothesis: the PD decision chain into `FixedUpdate`

`update_point_defense_ownership`, `update_turret_point_defense`,
`update_point_defense_aim` and `update_point_defense_trigger` moved from
`Update` to `FixedUpdate`.

| fps | duty before | duty A | rounds/step before | rounds/step A |
|--:|--:|--:|--:|--:|
| 106 | 0.606 | 0.623 | 11.07 | 12.06 |
| 60 | 0.353 | 0.285 | 6.37 | 5.46 |
| 30 | 0.173 | 0.208 | 3.01 | 3.99 |
| 20 | 0.097 | 0.156 | 1.49 | 3.02 |

**Spread 7.4x -> 4.0x. Not the fix.** It lifts the SLOW end (the target feed now
refreshes at 64 Hz instead of 20) and leaves the fast end untouched, because the
thing that still follows the render clock is the barrel, not the trigger. The
106 fps arm still stalls.

Kept in the shipped change anyway: one decision to one step is correct on its
own terms and it is what makes the arm below meaningful. It is not what closes
the gap, and it must not be reported as if it were.

### Arm B - the barrel onto the physics clock

`update_turret_aim_point`, `update_turret_target_joints_system`,
`smooth_look_rotation_update_system` and `sync_turret_joint_rotation` all moved
onto the fixed clock, ordered solve -> ease -> write the joint pose -> fire
along it, inside one step.

| fps | frames/step | duty | rounds/step | intercepts @ 90 s | mean aim err |
|--:|--:|--:|--:|--:|--:|
| 106 | 1.40 | 0.8166 | 15.20 | 840 | 3.52 deg |
| 60 | 0.87 | 0.8135 | 15.14 | 845 | 3.65 deg |
| 30 | 0.46 | 0.8138 | 15.14 | 824 | 3.41 deg |
| 20 | 0.31 | 0.8107 | 15.08 | 827 | 3.63 deg |

**INVARIANT.** Across a 4.5x span of frames-per-step:

| quantity | before | arm B |
|---|--:|--:|
| trigger duty | 0.097 - 0.606 (**6.2x**) | 0.811 - 0.817 (**0.7%**) |
| rounds/step | 1.49 - 11.07 (**7.4x**) | 15.08 - 15.20 (**0.8%**) |
| intercepts @ 90 s | 34 - 782 (**>20x**) | 824 - 845 (**2.5%**) |

**But point defence got about 2.4x STRONGER at 60 fps** (duty 0.353 -> 0.814,
rounds/step 6.37 -> 15.14). Two things changed at once and only one of them is
invariance: moving `sync_turret_joint_rotation` into the same fixed step as the
spawner also removed a whole FRAME of transport delay from the fire gate, so the
gun now fires along where the barrel is rather than where it was. Mean aim error
fell from ~11-12 deg to ~3.5 deg. That is an accuracy change, and it is a
balance decision.

### Arm C - the attempt to be invariant AND strength-neutral

Same as B, except `shoot_spawn_projectile` reads the PREVIOUS step's joint pose
and aim point rather than this one's - which is the relationship the
render-clock build had (one frame), held at one fixed TICK instead. The idea was
to keep the shipped feel and change only the clock.

| fps | duty | rounds/step | intercepts @ 90 s |
|--:|--:|--:|--:|
| 106 | 0.8146 | 15.34 | 811 |
| 60 | 0.8043 | 15.15 | 832 |
| 30 | 0.8054 | 15.17 | 820 |
| 20 | 0.8081 | 15.22 | 813 |

**Indistinguishable from arm B.** One tick of staleness is worth nothing here,
so there is NO strength-neutral ordering to choose: the strength gain does not
come from removing the last hop, it comes from removing the several frames of
accumulated render-clock latency across the whole chain (solve in `PostUpdate`
frame N, joint written in `Update` frame N+1, propagated in `PostUpdate` frame
N+1, read by `FixedUpdate` frame N+2). **Invariance and strength are coupled;
you cannot buy one without the other by reordering.** Arm B shipped, because at
equal measurements the coherent order wins - fire along where the barrel IS.

## What shipped

Everything that decides whether a barrel bears now runs on the fixed clock, in
one ordered chain per step:

```text
point-defence ownership -> assignment -> aim -> trigger      (FixedUpdate)
  -> intercept solve -> hinge demand                         (TurretSectionAimSystems)
  -> SmoothLookRotation ease                                 (SmoothLookRotationSystems::Sync)
  -> joint Transform                                         (sync_turret_joint_rotation)
  -> round spawn                                             (shoot_spawn_projectile)
```

Deliberately NOT moved:

- **The point-defence lines** (`draw_point_defense_lines`) stay in `PostUpdate`
  after transform propagation. They are a readout, nothing branches on them.
- **The turret lead pip** (`nova_hud::turret_lead`) stays in `PostUpdate`. It now
  reads an aim point up to one TICK old instead of same-frame; 15.6 ms, bounded,
  where the render clock bounded it at one frame and got worse as the machine
  struggled.
- **`AIM_CORRECTION_RATE` and every other tuning number.** Not touched. The fix
  is a schedule, not a retune.

## Did 60 fps behaviour move? YES, by about 2.4x

| at 60 fps | before | after |
|---|--:|--:|
| trigger duty | 0.353 | 0.814 |
| rounds/step | 6.37 | 15.14 |
| mean aim error | 11.3 deg | 3.6 deg |

**Point defence is roughly 2.4x stronger at 60 fps than the build this branched
from.** Not retuned to compensate, per the brief. What this is: the chain used
to carry several frames of latency into a 0.92 deg gate, and the gate was shut
for most of the time as a result. Removing the latency is what makes the outcome
invariant; it is not separable from it (arm C).

The owner's call, with the numbers to make it on:

- If a battery this effective is wrong, the lever is content
  (`KINETIC_PDC_BULLET_DAMAGE`, the PDC `fire_rate`, `AI_POINT_DEFENSE_RANGE`)
  or `TURRET_ON_TARGET_RAD`, not the schedule.
- **Proposal, not applied:** `ROUNDS_PER_MOUNT` is 40 (a 480 floor) and the
  measured peak is now 2204-2437, so the floor could be drawn at roughly
  120/mount and still keep a 25% margin. It was left at 40 because a scale
  claim should not be tight against one machine, and because the scene may be
  rebalanced.

## What the fix did to the range itself

A battery that works outguns twelve bays, so the envelope stopped reaching
`INBOUND_PER_BAY * bays` at ANY frame rate and every arm stalled on the fill
deadline. The settled population against a working battery is 5.0-5.6 torpedoes
a bay, not the 6 the gate wanted, so **`INBOUND_PER_BAY` is 6 -> 4** (a 25%
margin under the measured settle). Six was drawn against a battery that only
held its trigger a third of the time.

Also invalidated and rewritten: `ROUNDS_PER_MOUNT`'s docstring, which described
the defect as the reason the floor was loose ("the trigger is decided once per
frame ... so a free-running host buys a different number of steps per decision
every run"). That sentence is no longer true of the code.

## The evidence

### Rounds yield spread - the headline

| | spread |
|---|---|
| before, free-running (task notes, 16 captures) | 708 - 2,425 (**3.4x**) |
| before, pinned `NOVA_PERF_MAX_DELTA=0.015625` | 2,236 - 2,401 (1.07x) |
| **after, free-running, 20-503 fps** | **2,204 - 2,437 (1.11x)** |

The pinned spread, without the pin. The top of that range is the probe's own
free-running headless pass at 503 fps (7.86 frames per fixed step): envelope
fill 7.2 s, peak 2,318 rounds - inside the band the 20 fps arm sits in. **A 25x
frame-rate span moves the yield by 11%.**

### Rendered against headless - the convergence

Same binary, both free-running, a 13x frame-rate gap between them.

| | rendered (Xvfb, 33 fps) | headless (435 fps) |
|---|--:|--:|
| envelope fill | 7.2 s | 7.1 s |
| peak rounds | 2,455 | 2,411 |
| peak inbound | 59 | 54 |

Before: fill 9.5 s against 86.3 s (**9.1x**), peak rounds 1,668 against 2,418.
After, fill agrees to **1.4%** and peak rounds to **1.8%**.

The raw intercept tally is NOT comparable across transports and never was: the
range's `hold the saturation` step is a FRAME count (`HOLD_FRAMES`), so the
simulated time it measures is itself a function of frame rate - 3.6 s at 33 fps
against 0.28 s at 435 fps. The invariant intercept reading is the 90 s saturated
series above (824-845, 2.5%). **Reported, not changed:** `HOLD_FRAMES` is a
frame count because the perf capture wants a frame window, and another lane has
just tuned against it.

### Range assertions

`stress_point_defense` passes at 8, 16, 33 and 50 ms frame floors, where before
it stalled at 8 and 16 and passed only at 33 and 50. `system_turret_gunnery`
passes headless and rendered, `assert the barrel tracks the mover` included.
`cargo test --lib`: `nova_ship` 682 passed, `nova_gameplay` 191 passed.
`cargo check --all-targets` and `cargo fmt --check` clean.

`probe run stress_point_defense --norender --correctness-only`: **OK**, six
checks PASS and none SKIPPED (`process_exit`, `run_completed`, `reached_playing`,
`invariants_held` over 10,857 frames, `log_clean`, `artifacts_loadable`), with
all eight outcome markers on the timeline.

## The drawn side, honestly

`SmoothLookRotation` and the joint sync moved, so a turret's joint angle now
advances at 64 Hz instead of once per rendered frame.

- At 60 Hz refresh this is a wash (64 updates against 60).
- **Above 64 Hz it is fewer distinct barrel poses than before.** The joint speed
  is 180 deg/s, so a saturated re-acquisition slew steps at most 2.81 deg; during
  tracking the CCD moves `gain * error` with `gain = 0.332` at 64 Hz against a
  settled ~3.6 deg error, so about **1.2 deg a step**.
- The hull is `TransformInterpolation`-smoothed, so above 64 Hz the barrel is now
  the only un-interpolated moving part of the ship.

**Not eyeballed in motion.** `system_turret_gunnery` passes rendered and its
barrel-tracks-the-mover claim holds, but the screenshot drivers fire 30 settled
frames after `Playing` and under Xvfb that lands during the scenario load, so no
usable still was captured. The bound above is arithmetic, not observation.

**Follow-up, not done here:** interpolate `SmoothLookRotationOutput` for render -
the same treatment avian already gives the hull. That restores render-rate
smoothness without putting the DECISION back on the render clock, and it is the
only thing that makes the two goals independent.

## Changelog

**No entry.** Every point-defence and turret-fire-gate entry in the
`[Unreleased]` block is unreleased - the per-turret assignment, the Flight
Computer borrowing idle PDCs, and the 0.92 deg on-target gate itself. Without
that gate there is no defect of this shape, so this is a bug introduced and
fixed inside one cycle, and `CONVENTIONS.md` Changelog rule 3 says it never
existed for a reader.

The dev book DID owe a change: `docs/architecture.md`'s "Update vs FixedUpdate -
which schedule does my system go in?" told you to put a drawn thing in
`PostUpdate` and said nothing about a drawn thing that a fixed-step system
branches on. That rule and this worked example are now in it.

## Other schedule splits found

Surveyed every `FixedUpdate` / `FixedPostUpdate` registration in `nova_ship`,
`nova_gameplay`, `nova_scenario` and `nova_hud` against the `Update` writers of
what they read. **No second instance of this defect.** Reported, not fixed:

- `hold_scripted_torpedo_trigger` (`torpedo_section/scripted.rs:30`, `Update`)
  and `update_torpedo_section_input` (`input/ai/torpedo.rs:88`, `Update`) both
  feed `TorpedoSectionInput`, consumed by `torpedo_section/bay.rs:138` in
  `FixedUpdate`. Same SHAPE as the point-defence trigger, but the launch moment
  is gated by a `Cooldown` ticked inside `FixedUpdate` and the trigger is a
  LEVEL re-asserted every frame, so no launch is dropped or doubled. Worth a
  second look only because it is the same shape - it is not the same defect.
- `sever_disconnected_structures` (`sections/integrity.rs:231`, `Update`) is a
  genuine one-shot whose consequence lands in `FixedPostUpdate`
  (`integrity.rs:405`), but the decision is topological rather than timed and
  the applied kick is a flat constant, so it is identical on tick N or N+1.
- `update_fire_cadence` (`input/ai/guns.rs:183`) is the mirror image done
  right: deliberately in `FixedUpdate`, with
  `the_burst_window_closes_on_a_fixed_step_not_a_frame_boundary` pinning it.
