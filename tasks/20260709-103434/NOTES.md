# Flight: manual Newtonian + diegetic autopilot

Task: `tasks/20260709-103434/TASK.md`; design settled with the user in
`tasks/20260709-103324/SPIKE.md`. This replaces the
velocity-servo flight assist (52b582d, design note `2026-07-09-flight-assist.md`,
now deleted): user verdict was that the computer should fly the ship through
its **real actuators**, not apply invisible RCS forces at the center of mass.

## The model

- **Manual (default).** The mouse points the hull (camera rig ->
  `ControllerSectionRotationInput` -> controller-section PD torque, as
  always); W / Space / right trigger is an **analog** main-drive burn;
  momentum persists. Pure Newtonian - the only thing kept from the servo era
  is the spool ramp, so the plume and engine hum ease in and out instead of
  snapping.
- **Autopilot (engaged per maneuver).** An `Autopilot` component on the ship
  root is the whole mode: present = engaged. It writes the *same seams the
  pilot uses* - the rotation command for the PD, the thruster inputs for the
  burn - so a maneuver is visible ship behavior: the hull physically swings,
  the plume is the brake light.
  - `X` - **STOP**: face retrograde, burn to rest, disengage.
  - `G` - **GOTO** the current aim-assist lock: burn toward it, flip at the
    arrival curve, decelerate, come to rest at `arrival_standoff` (50u,
    outside the 30u torpedo blast radius), disengage.

### One rule flies everything

Each tick the autopilot computes the **desired velocity** for its goal
(`Vec3::ZERO` for STOP; for GOTO, the arrival rule solved with a reaction
budget: the `v` satisfying `v * lead + v^2 / (2 * a * margin) = remaining`,
where both `a` and the lead come from the engine group the computer would
actually brake with - a retro-equipped ship brakes late and flat, a
main-drive-only ship budgets its 180 plus `arrival_spool_pad`), then:

1. clusters the live engines into **direction groups** (a section's local
   `Transform.rotation` is its thrust axis; greedy clustering inside the
   ~25 degree cone) and rotates the *cheapest group* onto the **velocity
   error** (`desired - actual`) - cheapest by
   `rotation_time * rotation_bias + burn_time`, so a retro thruster handles
   the small brake it already points at while a big burn still flips the
   main drive around. The nose is nothing special;
2. fires **every** live engine currently inside the `align_cos` cone of the
   error (per-engine hysteresis via its own spooled input), sharing the
   throttle across the firing set's summed authority;
3. disengages when the goal wants rest, the ship is at rest
   (`stop_speed_epsilon`), **and the engines have wound down** - releasing
   the ship mid-spool-down would let the dying burn push it off station.

Two terms exist because the physics-level tests demanded them: the arrival
lead budgets the un-braked travel that rotating the brake group costs
(without it the plan assumes instant retro thrust and sails through the
standoff at 30+ u/s), and `min_approach_speed` floors the GOTO closing speed
(the pure arrival curve reaches zero *at* the boundary, so the ship would
approach it asymptotically and never arrive).

The planner knobs `rotation_bias` (1.5) and `est_turn_rate_deg` (90) shape
the rotate-vs-burn tradeoff and are retune-owned, as is the
`arrival_spool_pad` on the dynamic brake lead (which replaced the fixed
`flip_lead_time`).

Two more came from playtest feel: `attitude_deadband` (0.4 u/s) marks
velocity errors as crumbs the computer never re-aims the hull for - it
finishes them axially if the nose already points the right way and otherwise
accepts the residual, which is what stops the ship twitching after perfection
at the end of a maneuver - and `align_hysteresis` keeps lit engines burning
until alignment falls a little below the ignition gate, so the plume does not
flicker at the boundary.

GOTO replans against the target's current position every tick: slow drift is
tracked, fast targets just take longer. There is no collision avoidance in
v1, and torque from off-center engines is deliberately unmodeled - the PD
fights it, an unbalanced ship flies badly, and that is diegetic (both
spike-recorded; torque-aware allocation via section positions/COM is the
follow-up).

### Authority handover

- While engaged, the manual rotation copy
  (`update_controller_target_rotation_torque`) is gated off with
  `Without<Autopilot>` - so the mouse, which keeps driving the camera rig,
  becomes **camera-only free-look for free**. You can watch the maneuver from
  any angle without cancelling it.
- **Any flight input disengages**: grabbing the throttle (W/Space), `Z`
  (plain off), `X`/`G` re-presses (toggle semantics; `X` during a GOTO
  overrides into a STOP - braking always wins).
- On disengage, the normal rotation rig's `PointRotation` is re-seeded from
  the ship's *current* attitude (`camera_controller::on_autopilot_disengaged`),
  so the PD holds the hull where the maneuver left it instead of violently
  swinging back to a stale mouse command.
- **The computer commands every live engine.** A thruster with a manual
  per-section binding (the editor binds keys straight to thrusters) is only
  reserved in MANUAL mode - the W-burn path leaves it to its own key. An
  engaged autopilot drives bindings included (excluding them left
  editor-built ships with a computer that could rotate but never burn - the
  2026-07-09 playtest bug), pressing a bound thruster key disengages like
  any flight input, and on release the autopilot cools every engine it was
  driving (a residual input on a bound thruster would ghost-burn forever,
  since nothing else writes it between key events).
- **Degradation is diegetic.** The flight computer is the controller section:
  if it dies (or is disabled), the autopilot disengages and the ship is
  adrift on manual thrust. Engines shot off leave the autopilot aligning at
  zero authority - it points the right way and cannot burn, exactly like the
  pilot.

## Input map

| Input | Manual | Autopilot engaged |
|---|---|---|
| Mouse | points the hull (and camera) | camera free-look only |
| W / Space / RT | analog main burn | disengages (and burns) |
| X / pad East | engage STOP | STOP: off; GOTO: switch to STOP |
| G / pad North | engage GOTO on the lock (no lock: no-op) | GOTO: off; STOP: switch to GOTO |
| Z / pad West | - | off |
| Alt free-look, RMB turret mode | unchanged | unchanged (camera/turret rigs, not flight inputs) |

## Readouts (minimal, by design)

`hud/flight_status.rs`: one text line - `MAN 12.3 u/s`,
`AP STOP - ALIGN | 12.3 u/s`, `AP GOTO - BURN | 12.3 u/s | 320m` - plus a
fixed-size cyan-tinted projected marker on the engaged GOTO destination
(same UI-pass projection the torpedo reticle uses). The real diegetic
instrument treatment (maneuver plan, flip point, ETA) is task
20260709-103454 on the HUD projection substrate.

## What was removed (and why)

From the servo era: `FlightAssistMode` (Assisted/Newtonian), `FlightCommand`,
the velocity-hold servo, `FlightRcsImpulse` + the RCS impulse at the COM,
`ControllerSectionRcsMagnitude` (component and config field), and the
ADQE/strafe + brake-latch inputs. With one rear thruster and no RCS hardware,
lateral intent was fiction, and the always-on servo hid the ship's real
dynamics. Kept: the spool ramp, capability-from-live-sections, the flight
input rig shape, the status HUD, and the physics-test harness reuse.

## Testing

Pure helpers: arrival rule (zero at goal, monotonic, margin scales), GOTO
desired velocity (direction, rest inside standoff), burn clamp, spool,
alignment, status-line formatting. Physics-level (shared integrity harness +
the real bcs PD controller + controller-section glue + thruster impulse
system, all first-party systems, no synthetic forces): STOP from a sideways
coast physically flips the hull and reaches rest, GOTO arrives near the
standoff at rest and disengages, a vanished destination disengages, a dead
controller section disengages and the ship coasts, manual burn accelerates
and is overridden while a maneuver is engaged.

## Deliberately deferred

FOLLOW/velocity-match action, collision avoidance / trajectory planning,
multi-thruster blueprints + thrust allocation, the AI adopting the shared
maneuver math, diegetic instruments (20260709-103454), and the feel retune
(20260709-095043 - every constant here is a reasoned guess until flown).
