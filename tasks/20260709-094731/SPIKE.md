# Spike: Flight feel - what makes a capital ship fly well without faking the physics?

- DATE: 20260709-094731
- STATUS: RECOMMENDED
- TAGS: spike, handling, juice, v0.4.0

## Question

Task 20260708-203655 wants flying to feel "weighty, precise, and readable",
realistic but not frustrating. What concretely should the handling overhaul
build - with the current capabilities where possible - and what new machinery
does it need? The open calls named in the task: assisted-vs-Newtonian default,
and how much realism (true 6DOF) vs playability.

A good answer picks a control paradigm, says exactly which pieces are new vs
retuned, resolves the two open calls with reasons, and hands /plan a scope it
can break into steps without re-litigating design.

## Context: how the ship flies today (read from the code)

- **Rotation.** Mouse/stick turns a `PointRotation` rig on the chase camera;
  `update_controller_target_rotation_torque` (input/player.rs:63) copies the
  camera quat into `ControllerSectionRotationInput`; the ship's controller
  section PD-torques the root toward it (bcs `PDController`, frequency 2.0,
  damping 2.0, max_torque 1.0 - controller_section.rs:30). So the ship chases
  wherever the camera points, with PD lag. This half already has believable
  dynamics; its constants are just not framed as handling stats.
- **Translation.** One input action, binary: `on_thruster_input` sets
  `ThrusterSectionInput` to 1.0 on press, 0.0 on release (input/player.rs:311).
  The player ship's blueprint has exactly one thruster (scenario.rs:96), so
  Space = main drive at 100%, release = coast. Thrust is honestly simulated -
  `apply_linear_impulse_at_point` along the section's -Z at the section's
  position (thruster_section.rs:129) - so off-axis thrusters would induce
  torque, and a destroyed thruster stops thrusting (`SectionInactiveMarker`).
- **No brake, no strafe, no throttle.** Stopping is a manual flip-and-burn.
  The AI literally implements this (input/ai.rs:54 points retrograde when too
  fast); the player gets no equivalent help.
- **Feedback.** The exhaust shader and audio hum track `ThrusterSectionInput`
  directly - a binary input means plume and sound snap 0-to-100 with no spool.
  The 3D velocity HUD (hud/velocity.rs) shows velocity direction + magnitude.
  The chase camera has a `smoothing` field (bcs chase.rs:86) that Nova leaves
  at 0.0 - the camera is rigidly bolted to the ship.
- **bcs inventory** (per the audio retro's "check bcs first"): rotational
  `PDController` yes; chase camera with smoothing yes; camera shake yes
  (juice feeds it). There is **no** linear velocity controller / flight-assist
  module and no thrust-allocation anything - that part would be new code.
- **Destruction couples in.** Sections are the ship. Controller section dead =
  no rotation authority already. Any assist layer should inherit this logic
  rather than bypass it.

Why this feels bad (model-level; validate in playtest): binary thrust makes
fine approach impossible and overshoot constant; killing velocity needs a
pixel-timed manual counter-burn; there is no lateral authority at all, so all
maneuvering is point-nose-then-burn jousting; and full-power-instantly kills
the weight illusion in plume, sound, and camera alike.

## Options considered

- **A. More realism, no assist ("pilot everything by hand").** Add
  retro/lateral thruster sections to blueprints, bind six thrust axes plus an
  analog throttle, keep zero computer help. Pros: purest simulation, emergent
  flip-and-burn. Cons: fails the "not frustrating" requirement head-on -
  every engagement becomes Kerbal docking under fire; keybinding sprawl; and
  with one-thruster blueprints there is nothing to bind yet. Rejected as the
  default, but its *control seam* (direct thruster-group input) is exactly
  what Newtonian mode should expose.
- **B. Flight computer / assist layer (recommended).** Do what real spacecraft
  (and the Expanse fiction) do: ships have a flight control system. The player
  states *intent* - a desired velocity - and the FCS turns it into honest,
  capability-clamped forces. Two modes:
  - **Assisted (default):** velocity-command. Forward/back input nudges the
    commanded velocity along the nose, strafe keys command lateral/vertical
    velocity, no input = hold current velocity (a real Newtonian hold, not
    drag), a dedicated brake input commands zero. The FCS applies a force
    toward the commanded velocity, clamped by what the surviving sections can
    produce - so a heavy ship still *slews*, it just slews for you.
  - **Newtonian (toggle, "FA off"):** the assist drops out entirely; thrust
    keys drive thruster groups directly (today's behavior, plus retro/lateral
    groups when blueprints have them); momentum persists; you burn to stop.
  Pros: approachable default, depth behind a toggle, physics stays honest
  underneath (forces never exceed thruster capability, capability dies with
  sections), matches the fiction. Cons: new machinery (intent components, FCS
  system, capability model) and real tuning work.
- **C. Arcade damping (space drag + speed cap).** Add artificial linear
  damping so releasing the stick slows the ship. Pros: trivially friendly.
  Cons: betrays the hard-sci-fi identity the roadmap spike fixed ("heavy ship
  must feel heavy", positioning-and-burns), and silently changes the physics
  that torpedo PN guidance and the AI brake logic are tuned against. Rejected.
- **Do nothing / feedback-only.** Only ramp the exhaust, lag the camera, and
  tune PD constants. Cheapest, and those pieces are genuinely wanted - but it
  leaves the actual frustrations (no brake, no fine control) untouched; the
  task explicitly asks for the handling layer. Rejected as the whole answer,
  absorbed as the feedback half of B.

### Resolved design calls (within B)

1. **Assisted is the default** (task leaned this way; confirmed). Newtonian is
   the skill toggle, one key, HUD-visible. Assisted must never fight a player
   who wants to drift: no-input means *hold velocity*, not auto-brake to zero -
   braking is an explicit input. (The task sketch said "auto-brake to zero
   relative velocity"; zeroing velocity uninvited turns space into soup and
   makes drifting broadsides impossible. The brake key gives the same comfort
   on demand.)
2. **Full 6DOF translation intent, soft-capped commanded speed.** Realism
   knob: the cap applies only to what the *computer* will command in assisted
   mode (the FCS refuses to fly faster); Newtonian mode is uncapped. No drag
   anywhere. This is the playability compromise that does not fake physics.
3. **Capability model, not a thrust-allocation solver.** With one thruster per
   blueprint there is nothing to allocate. v1: main-drive authority = sum of
   live forward thruster sections (their existing magnitudes); RCS
   (lateral/retro/vertical) authority = a new stat on the *controller* section
   (the flight computer owns the RCS quads, abstracted). FCS force is applied
   at the COM (no spurious torque), clamped per-axis by those authorities, and
   it scales/dies with the owning sections - engines shot off = no main burn,
   controller shot off = adrift with raw thrusters only. Per-thruster *visuals*
   are driven by projecting the commanded acceleration onto each live
   thruster's direction (cosine), so plumes read correctly without a solver. A
   real allocator becomes worthwhile only when multi-thruster blueprints
   exist - explicitly deferred.
4. **Rotation becomes handling stats, command gets a slew limit.** Keep
   camera-points-ship, but rate-limit how fast the *commanded* quat tracks the
   camera (`smooth_nudge`/max deg/s) so a capital ship's nose visibly commits
   to a turn, and surface PD frequency/damping/max_torque + slew rate as the
   ship's handling block instead of buried defaults. Mouse stays instant for
   the camera; the hull is what lags.
5. **Feedback rides existing seams.** Exhaust shader + audio hum get analog
   input for free once `ThrusterSectionInput` is a ramped 0..1 (spool up/down
   smoothing in the FCS output); chase camera gets `smoothing` > 0 plus a
   small offset push under main burn; shake stays owned by juice. New
   particles are out (wasm-blocked, 162908). HUD: reuse the existing velocity
   HUD; add at most a minimal assist-mode/commanded-speed text readout, and
   leave real HUD work to the weapons-HUD tasks.
6. **AI keeps writing the raw seams** (`ThrusterSectionInput`,
   `ControllerSectionRotationInput`) and is untouched in v1; moving the AI
   brain onto the velocity-intent API is a natural later cleanup, not this
   task.

## Recommendation

Build **B, phased inside task 20260708-203655**: a `flight` module in
`nova_gameplay` (sibling shape to `juice.rs`/`audio.rs`) owning (a) intent
components on the ship root (`FlightIntent`, assist mode), (b) the FCS
FixedUpdate system (velocity-hold + brake + soft cap in assisted; direct
groups in Newtonian), (c) the capability model from live sections, (d) ramped
`ThrusterSectionInput` output driving the existing plume/audio, plus the
rotation slew/handling stats and camera smoothing retune. All tunables in one
reflected settings/handling tree (register the whole tree - juice retro R1.1),
pure helpers for the math, observer/system-level App tests from day one (the
lesson that bit audio then juice), and physics-level tests via the existing
`integrity/test_support` harness. Expect a dedicated playtest-retune step:
every feel constant is a decision, not a default (juice retro).

The generic core (velocity-hold FCS over a force-capped body) is deliberately
game-agnostic and a future bcs promotion candidate; keep it in a pure-math
layer to make that split cheap later.

## Open questions

- **Match-target-velocity** (zero *relative* velocity to the locked target) is
  the real capital-combat comfort feature and composes with the existing lock;
  deferred to its own task so v1 stays absolute-frame.
- Whether the soft cap value should scale with ship mass/authority or be one
  global constant - decide during playtest tuning.
- Key layout for strafe axes (ADQE vs arrows) and whether Space stays
  "full burn" in assisted mode - decide at plan time, cheap to change.
- Newtonian-mode thruster grouping (retro/lateral groups) is blueprint work
  that only pays off once ships carry those sections; v1 Newtonian = today's
  main-drive behavior, groups arrive with multi-thruster blueprints.

## Next steps

Direction-level outcome, for /plan to break into steps:

- tatr 20260708-203655 (this task): implement the assisted/Newtonian flight
  layer + handling stats + feedback ramp as above. /plan owns the step list.
- Deferred (created only when their prerequisites land, recorded here so they
  are not lost): match-target-velocity assist; thrust-allocation solver +
  multi-thruster blueprints; AI brain on the intent API; bcs promotion of the
  generic FCS core.
