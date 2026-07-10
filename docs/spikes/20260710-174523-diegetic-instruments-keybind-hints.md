# Spike: Diegetic flight instruments and keybind hints - what visual language, what architecture?

- DATE: 20260710-174523
- STATUS: RECOMMENDED
- TAGS: spike, hud, autopilot, ux, v0.5.0

## Question

Task 20260709-103454 wants the autopilot to feel in-world - an instrument
treatment for the engaged maneuver (flip point, decel curve/ETA,
destination), not debug text. The user added a second requirement
(2026-07-10): "Arma Reforger"-style keybind hints - show the button and the
action in the UI, ideally more diegetic than a plain print, so nobody has to
memorize X/G/O/Z - and observed that once the UI covers the maneuvers, the
hints should be easy to add. Two uncertainties to reduce: (a) which diegetic
visual language the instruments use, and (b) what architecture makes hints a
substrate instead of a per-feature hack. A good answer picks languages the
codebase can already render, phases the work, and leaves hint authoring
one-line cheap for every future verb.

## Context

- **The HUD is UI-pass by decision** (weapons-hud spike): no second Camera2d
  (blacks out the 3D scene on Bevy 0.19), no gizmos for shipped UI (debug
  grade; the F11 gravity overlay is gizmos and stays debug-only).
- **The screen-projected-indicator substrate is proven** (spike
  20260709-164502, shipped): anchors (Entity/Point), sizing (fixed/apparent),
  offscreen policies (hide/clamp), consumers already include the torpedo
  reticle, the GOTO/ORBIT destination marker, the component-lock markers,
  and - load-bearing for this spike - the `[O] ORBIT` cue, which proved that
  a text chip inside an indicator works (hud/flight_status.rs).
- **One 3D in-world instrument already exists**: the velocity sphere
  (hud/velocity.rs), a world-space object around the ship. So the game
  already speaks two visual languages: world-anchored 3D for spatial,
  continuous things; screen-projected UI chips for text and markers.
- **The data the instruments need is already computed every tick** by
  autopilot_system (flight.rs): the arrival rule `v_allowed(d) =
  sqrt(2 a margin d)` gives the flip/brake point and an ETA; `Autopilot
  { action, phase }` gives the state machine; `OrbitPlan { radius, normal }`
  gives the ring; `DominantWell` + `GravityWell` give SOI geometry. Nothing
  new must be simulated - only surfaced.
- **Input bindings live in one place** (input/player.rs, bevy_enhanced_input
  `actions!` blocks: X STOP, G GOTO, O ORBIT, Z off, W/Space/RT burn).
  Verb availability is already computable: STOP always (with a computer),
  GOTO needs a lock (SpaceshipPlayerTargetLock), ORBIT needs a DominantWell,
  Z needs an engaged Autopilot.
- Deferred items parked on this task by earlier spikes: SOI rings and
  trajectory visualization for normal play (gravity spike), richer maneuver
  readouts (diegetic-autopilot spike).

## Options considered

### (a) Instrument visual language

- **A. Cockpit / modeled panel.** A 3D cockpit with gauges. Rejected: the
  game is third-person chase-camera; there is no cockpit view to put a panel
  in, and it would be an art project, not a UI task.
- **B. Screen-projected instruments only.** Extend the indicator substrate:
  flip-point chip projected on the flight path (Point anchor), ETA/decel
  text on the destination marker, orbit ring as a projected ellipse of
  chips. Pros: one substrate, cheap, everything text-capable. Cons: an
  orbit ring or trajectory drawn as UI chips reads as screen furniture, not
  a thing in the world - exactly the "debug text" feeling the task exists
  to kill, just with more nodes.
- **C. In-world 3D holo instruments only.** World-space meshes in the
  velocity-sphere language: a trajectory ribbon, a flip gate on the path,
  the orbit ring as a 3D loop. Pros: genuinely diegetic (the flight
  computer projects into space, matching the diegetic-autopilot fiction).
  Cons: numbers and labels in 3D are hard to keep legible; every element
  needs mesh/shader work; scope balloons.
- **D. Hybrid (recommended): 3D world-space for spatial geometry,
  projected chips for numbers.** The split the game already uses: the
  velocity sphere is 3D, the readouts are chips. Spatial, continuous things
  - the flip point ON the path, the planned orbit ring, later the SOI
  shell - become world-space holo elements; scalar things - ETA, distance,
  phase, v_circ - ride the existing indicator substrate anchored to those
  elements or to the destination.

### (b) Keybind-hint architecture

- **1. Hand-placed hints (status quo).** The `[O] ORBIT` cue is one. Does
  not scale: every verb re-implements presence logic, and key labels are
  hardcoded strings that drift from the real bindings.
- **2. Contextual hint substrate (recommended).** Two pieces:
  - A **verb-availability resolver**: one system that computes, for the
    player ship, the set of currently available flight verbs and their
    state - STOP (computer alive), GOTO (lock present), ORBIT (dominant
    well present, not already orbiting), CANCEL (engaged) - as a small
    resource/component the UI consumes.
  - Two presentations fed by it: a **hint cluster** docked with the
    flight-status line (one row per available verb: key chip + verb name,
    dim when unavailable, e.g. `[G] GOTO` lights up with a lock), and
    **anchored hints** on the object the verb applies to via the indicator
    substrate (the existing `[O] ORBIT` on the well, a `[G] GOTO` on the
    lock) - the "more diegetic than a print" part: the hint sits on the
    thing you would act on. The ORBIT cue gets absorbed as the first
    consumer.
  - **Key labels derive from the live input bindings** where feasible
    (bevy_enhanced_input actions are entities carrying their `bindings!`),
    so a future remap screen cannot desync the hints; fall back to an
    authored label table if binding introspection turns out awkward
    (open question below).
- **3. Fully diegetic 3D hint glyphs** (floating key meshes in world).
  Charming, unjustified: text chips on the indicator substrate already read
  as "the computer labels the world", at a tenth of the cost. Defer
  indefinitely.
- **Do nothing.** The status line and the one cue exist; players memorize
  four keys. Real option, but the user explicitly asked, and every future
  verb (dock, match-velocity, ...) makes memorization worse.

## Recommendation

Build **D + 2**, phased so the instruments (the user's stated priority)
land first and the hints reuse what they build:

1. **Maneuver instruments v1 (existing task 20260709-103454, re-scoped).**
   Chips first, geometry second: (i) enrich the destination indicator with
   ETA + closing speed + standoff distance (data from the arrival rule);
   (ii) flip-point marker - a Point-anchored indicator on the flight path
   where `v_allowed(d)` says the flip happens, labeled with seconds-to-flip;
   (iii) ORBIT ring as the first world-space holo element (3D line loop at
   `OrbitPlan { radius, normal }`, velocity-sphere visual language), with
   the r/v_circ chip anchored to the ring's nearest point. The ring is the
   scope-bounded 3D pilot: it proves the holo language on the simplest
   possible geometry (a circle) before anyone commits to trajectory ribbons.
2. **Keybind-hint substrate (new task, seeded below).** The availability
   resolver + hint cluster + anchored-hint pattern, absorbing the `[O]`
   cue; labels from live bindings if introspection cooperates, authored
   table otherwise.
3. **Holo expansion (new task, seeded below, lower priority).** Trajectory
   ribbon for engaged GOTO/STOP, SOI shell on approach, flip gate as world
   geometry. Only after the ring proves the language.

This beat B (all-chips) because the orbit ring and flip point are spatial
facts the pilot steers by - they belong in the world; and it beat C
(all-3D) because ETA and key hints are text, and the chip substrate already
solved text. The hint architecture beat hand-placing because the resolver
is ~one system and every future verb then costs one table row.

## Open questions

- **Binding introspection:** can the hint layer read the actual key/button
  from the bevy_enhanced_input action entities cleanly (including gamepad
  vs keyboard variants), or is an authored label table the honest v1?
  Resolve at /plan time with a 30-minute code read; the table is an
  acceptable fallback since bindings are static today.
- **Input-device awareness:** show keyboard chips, pad chips, or both?
  V1 proposal: keyboard labels only (matches the current placeholder
  style); revisit when a pad-detection signal exists.
- **Ring shader:** flat unlit color vs the velocity sphere's treatment -
  decide in /work by eye, not here.
- **Hold-phase presentation:** whether the ring pulses/dims between Burn
  and Hold - playtest question once the ring exists.

## Next steps

Direction-level tasks (for /plan to break into steps when picked up):

- tatr 20260709-103454 (existing, re-scoped by this spike): maneuver
  instruments v1 - destination ETA chip, flip-point marker, ORBIT holo
  ring + ring chip.
- tatr <seeded>: contextual keybind-hint substrate - availability resolver,
  hint cluster by the status line, anchored hints absorbing the [O] cue.
- tatr <seeded>: world-space holo expansion - trajectory ribbon, SOI shell,
  flip gate (after the ring proves the language).
