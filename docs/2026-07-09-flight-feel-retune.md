# Flight feel retune: torque-budget turn rates, camera weight

Task: `tasks/20260709-095043`. Builds on the diegetic autopilot
(`docs/2026-07-09-diegetic-autopilot.md`) and executes the torque-budget
decision from the COM investigation
(`docs/2026-07-09-com-section-destroy.md`): ship mass should be legible in
handling - a stripped hull snaps, a full build lumbers.

## What changed and why

### Turn rate derives from the hull, not a constant

`FlightSettings::est_turn_rate_deg` (fixed 90 deg/s) slewed every rotation
command - mouse and autopilot alike - at the same rate regardless of the
ship, which re-normalized turn feel even though the physics underneath was
honest. It is replaced by `flight::hull_turn_rate`:

```text
turn_rate = clamp(scale * sqrt(pi * max_torque / inertia) / 2, min, max)
```

`sqrt(pi * alpha) / 2` is the average rate of a torque-limited bang-bang 180
at angular acceleration `alpha = max_torque / inertia`; `turn_rate_scale`
(0.9) commands slightly under that optimum so the PD tracks instead of riding
saturation; the clamps (10..240 deg/s) keep torque-starved barges answering
the helm and over-torqued skiffs from teleporting. Inertia is the largest
principal component of the body's live `ComputedAngularInertia`, and
`max_torque` is the strongest live flight computer's cap - so losing sections
(or the computer) changes handling immediately and diegetically.

Consumers: the player's mouse-command slew (`input/player.rs`) and the
autopilot's rotation-time scores, arrival lead, and command slew
(`flight.rs`) - the planner budgets flips more accurately for heavy ships as
a side effect.

### Ship controller max_torque: 100 -> 40

At 100 the budget never bound anything: the old fixed 90 deg/s slew was the
limit, and every build turned identically. 40 keeps the shipped flagship at
its familiar command rate while stripped hulls visibly out-turn it. Derived
command rates and bang-bang OPTIMA per ship (delivered flips run ~25-30%
longer - the command slews at 0.9x the optimum and the PD tracks the ramp
with ~0.5*w rad of steady-state lag):

| Ship | max principal inertia | command rate | 180 optimum |
|------|----------------------|--------------|-------------|
| asteroid_field flagship (5 sections) | ~10.8 | ~88 deg/s (old: 90 fixed) | ~1.8s |
| flight-test rig (3 sections) | ~2.3 | ~190 deg/s | ~0.85s |
| hull+thruster remnant | ~0.9 | 240 deg/s (ceiling) | ~0.5s |

Torpedoes keep their own config (freq 4, torque 10 on a tiny body), fully
isolated from FlightSettings, and stay agile.

### Blast radius of the torque cut (documented tradeoffs)

- **Off-center engines now out-torque the computer.** The PD's cap holds
  roughly `max_torque / 64` units of lateral lever arm per unit of thruster
  magnitude (~0.6 units at 40): an asymmetric editor build or a
  damage-shifted COM pulls under burn - diegetic, covered by the
  `off_center_burn_pulls_but_a_centered_drive_is_held` physics test; thrust
  balancing is a follow-up task.
- **AI ships inherit the cut with an unslewed command** - the saturation
  regime the player path was fixed for. No shipped scenario spawns an AI
  controller today (editor-only exposure); the AI adopting
  `slew_rotation`/`hull_turn_rate` is a follow-up task.

### Camera weight

- `ChaseCamera::smoothing` was the bcs default 0.0 (bolted on); the gameplay
  modes now set `CAMERA_SMOOTHING = 0.15`, so the camera trails maneuvers.
- `update_camera_rig` owns every ChaseCamera field per frame:
  `offset = mode rig + heat * 3.0` along anchor -Z (heat = hottest live
  forward-mounted thruster's spooled input, the flight layer's main-drive
  definition), the mode's focus offset, and the smoothing. Per-frame
  ownership is load-bearing: respawn re-inserts a default ChaseCamera, and
  anything applied only on mode change would be lost after the first life
  (review R1.1). In Normal mode the push reads as leaning back off the hull;
  in FreeLook/Turret the offset lives in the mouse-rig frame, so it is a
  dolly-out - acceptable juice either way. The per-mode rigs live in
  `mode_camera_rig` as the single source of truth.

## Tuned values (deliberate, to be confirmed in playtest)

| Knob | Value | Reasoning |
|------|-------|-----------|
| turn_rate_scale | 0.9 | just under the bang-bang optimum; PD tracks, no saturation wobble |
| turn_rate_min_deg | 10 | a crippled hull still answers the helm |
| turn_rate_max_deg | 240 | snappy, not teleporting |
| ship controller max_torque | 40.0 | flagship keeps ~88 deg/s (baseline feel); remnant pins the ceiling |
| CAMERA_SMOOTHING | 0.15 | visible trailing without seasickness |
| BURN_PUSH_DISTANCE | 3.0 | readable lean at full burn (base offsets 10-30) |
| spool/deadband/hysteresis/margins | unchanged | no evidence against them yet; playtest owns them |

## Verification

- `hull_turn_rate` unit tests (mass legibility, clamps, degenerate inputs).
- `a_camera_flip_reaches_the_command_over_many_frames` (player path lag: a
  mouse 180 moves the command but nowhere near the target in one frame).
- `burn_push_leans_back_and_returns_to_baseline` (camera offset composes and
  returns exactly to the mode rig), plus a smoothing assertion in the
  mode-switch test.
- `off_center_burn_pulls_but_a_centered_drive_is_held` pins the torque-cut
  regime change as documented behavior.
- All nova_gameplay unit tests pass (including the 23 pre-existing flight
  physics tests, unchanged - they assert maneuver outcomes, which the
  derived turn rates still deliver).
- Headless smokes green: `11_com_range` (COM + camera anchor, with the
  controller killed mid-run exercising the no-computer command freeze) and
  `06_torpedo_range`.

## Difficulties

- The feared risk - existing autopilot physics tests breaking under changed
  flip timing - did not materialize (they assert maneuver outcomes, not
  durations). The real problems arrived in review: the camera weight applied
  only on mode change and died with the first respawn (R1.1 - per-frame rig
  ownership fixed it), the initial torque value of 10 was tuned against the
  flight-test rig and silently halved the shipped flagship's turn rate
  (R1.3 - retuned to 40 with per-ship numbers), and the docs quoted
  bang-bang optima as delivered times (R1.4).

## Self-reflection

- Deriving the turn rate from `max_torque / inertia` closed a loop the
  original knob comment already flagged ("a knob rather than a derivation ...
  recorded for the retune") - reading the knob's own documentation surfaced
  the intended design.
- The camera rig extraction (`mode_camera_rig`) was forced by the two-writer
  problem (mode switch vs per-frame push); single-source-of-truth first would
  have been cheaper than discovering the fight in review.
