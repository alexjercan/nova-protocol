# Spike: Gravity and orbits - how does a ship park in orbit around an asteroid without n-body chaos?

- DATE: 20260709-193147
- STATUS: RECOMMENDED
- TAGS: spike, physics, gravity, handling, autopilot, v0.5.0

## Question

The user wants Newtonian-style orbital mechanics as a game mechanic: park the
ship near an asteroid and end up orbiting it. Explicitly NOT a precision
simulator - the ask is gamer-friendly gravity: "if we are close enough to
something we enter its orbit and then the physics applies", possibly the real
formula but gated by a fixed threshold, because unbounded mutual gravity would
make everything clump into one blob (reality balances this with things the
game does not have, so we improvise).

A good answer picks a gravity model that (a) produces real, flyable orbits,
(b) provably cannot clump the world, (c) honors the honest-Newtonian identity
the flight sagas established (no fake forces on the ship from the *computer*,
no drag), and (d) gives a one-input "park me in orbit" experience for players
who do not want to hand-fly an orbital insertion. It also has to say how
gravity coexists with the shipped diegetic autopilot (STOP/GOTO), the torpedo
PN guidance, and the turret lead prediction.

## Context

- **Physics.** Bevy 0.19 + avian3d 0.7. Global gravity is explicitly disabled
  (`Gravity::ZERO`, nova_gameplay plugin.rs:38). Ships move only via
  `Forces::apply_linear_impulse_at_point` from live thruster sections in
  FixedUpdate (thruster_section.rs:131). No drag, no speed cap; momentum
  persists. Adding a bounded external force is mechanically trivial - the
  design questions are all about *which* force and *who feels it*.
- **Flight identity (do not relitigate).** The velocity-servo assist was
  rejected by the user and replaced by the diegetic autopilot
  (tasks/20260709-103324/SPIKE.md): the computer flies the ship through its real
  actuators - STOP and GOTO verbs, Align/Burn phases, arrival rule
  `v_allowed(d) = sqrt(2 a margin d)`, any flight input breaks out. Invisible
  *computer* forces are out. Gravity is different in kind: it is a *world*
  force, physical in the fiction, and it acts on everything with mass - but it
  must be HUD-readable so a curving trajectory never feels like a bug.
- **Bodies.** Asteroids are `RigidBody::Static` husks with `AsteroidMarker`,
  `AsteroidRadius`, a noise-displaced trimesh collider, spawned by the
  scenario layer (nova_scenario objects/asteroid.rs). They do not move. Ships,
  torpedoes, and turret rounds are `RigidBody::Dynamic` with `LinearVelocity`.
- **Scale.** Asteroid-field scenario scatters rocks across a ~100u cube; ship
  spawn distances ~100u; manual flight speeds ~5-15 u/s; autopilot approach
  floor 1.5 u/s, arrival standoff 50u, torpedo blast radius 30u. Any orbital
  velocity in the 3-8 u/s range at radii of 30-80u is in the fun zone; that
  fixes the strength scale of the wells (see math below).
- **HUD seams that already exist.** Flight-status line (`MAN 12.3 u/s`,
  `AP GOTO - BURN`), the screen-projected-indicator substrate, and the 3D
  velocity sphere. An orbit readout is a one-line extension, not new UI tech.
- **Real-world note that licenses the improvisation.** True gravity at this
  scale is unplayably weak (orbiting a 500m rock happens at centimeters per
  second). So *any* orbit-capable gravity is already authored fantasy; deriving
  strength from collider mass would be both wrong and unfun. Strength is a
  designer stat, full stop.

## Options considered

- **A. Full symmetric n-body gravity.** Every massed body attracts every
  other. Rejected outright - it is the exact failure the user predicted:
  asteroid fields self-collapse (nothing holds them apart without the
  dispersion mechanisms reality has), O(n^2) pair forces, chaotic multi-well
  trajectories that no HUD can make readable, and combat prediction (turret
  lead, arrival rules) degrades everywhere all the time. Also pointless:
  static asteroids would need to become dynamic just to participate in a
  simulation we then have to fight.

- **B. Authored one-way gravity wells with a sphere of influence
  (patched-conics-lite).** Designated bodies carry a `GravityWell` component
  (strength + SOI radius, defaults derivable from `AsteroidRadius`, per-body
  override in the scenario). Wells pull only entities that opt in via a marker
  (ships, torpedoes); wells never pull other wells, and sources stay
  `RigidBody::Static`. Inside the SOI the acceleration is the real inverse
  square `a = mu / r^2` toward the center, clamped below a floor radius (no
  singularity slingshots), smoothly faded to zero over the outer band of the
  SOI (no force step at the boundary); zero outside. When SOIs overlap, only
  the dominant well (strongest pull at the ship's position, with hysteresis)
  applies - you are always in exactly one body's orbit or in flat space,
  which is literally the user's "enter *its* orbit" phrasing. This is the KSP
  answer: bodies on rails, small craft feel gravity, and it cannot clump by
  construction - the missing "dark matter" is simply that rocks do not pull
  rocks.
  Pros: real formula inside the gate (honest ellipses, hand-flyable orbits in
  manual mode), bounded and predictable, cheap (N_affected x 1 dominant well).
  Cons: on its own it only makes gravity - a player who "parks" (zeroes
  velocity) inside a well just falls onto the rock. The fantasy needs an
  insertion aid.

- **C. Scripted orbit-lock (rails).** No real force. When close enough and
  slow enough, snap the ship onto a kinematic circular orbit (parenting or
  scripted motion); leaving is a state exit. Pros: zero tuning, perfectly
  stable orbits, trivially readable. Rejected: it is exactly the "invisible
  hand" model the user already threw out for the flight assist, but worse - a
  kinematic ship stops being a physics participant (thruster impulses, torpedo
  hits, and collisions either do nothing or fight the rail), and a special
  movement state leaks into every combat system. It also kills the emergent
  play B gives for free (aerobrake-style flybys, hand-flown orbits, orbital
  decay when your engines are shot off).

- **D. B + a diegetic ORBIT autopilot verb (recommended).** Keep B as the
  physics substrate and make "gamer-friendly" a third autopilot verb next to
  STOP and GOTO: inside a well, one input engages ORBIT; the autopilot flies a
  real insertion through the real actuators - align, burn to the tangential
  circular-orbit velocity at (roughly) the current radius, then a Hold phase
  that station-keeps with micro-burns against integrator drift and fade-band
  error. Breakout on any flight input, same as STOP/GOTO. The plume is the
  insertion burn; a dead controller section means no ORBIT; dead engines mean
  the autopilot aligns but cannot circularize - destruction couples in for
  free, and station-keeping quietly stops when the ship dies, so derelicts
  really do decay out of orbit.

- **Do nothing / defer.** Gravity is new mechanics and the current sprint is
  combat feel. Real option, but the spike exists because the flight/autopilot
  seams are hot right now: the maneuver machine, capability model, and HUD
  line were all built this cycle, and ORBIT is a small extension of them. The
  answer is to seed it clearly for the v0.5.0 arc, not to build it this
  sprint.

### Resolved sub-decisions (within D)

1. **Strength is authored, not mass-derived.** Author `surface_gravity` (u/s^2
   at the body's nominal radius) and derive `mu = surface_gravity * radius^2`.
   Sanity math at game scale: a 20u-radius rock with surface gravity 3 u/s^2
   gives mu = 1200; circular orbit at r = 50u flies at
   v = sqrt(mu / r) ~ 4.9 u/s with a ~64s lap - visible motion, parkable
   speeds, and well under combat velocities. Defaults derive from
   `AsteroidRadius` via global tunables; scenario args can override per body;
   tiny field rocks below a radius threshold get no well at all by default.
2. **Guardrail: gravity never out-muscles a live ship.** Cap well strength so
   peak acceleration stays a tunable fraction (well under 1.0) of a typical
   main-drive acceleration. Gravity should shape trajectories and create
   parking, not create inescapable traps - a functioning ship can always
   climb out.
3. **Force profile.** `a(r) = mu / r^2` toward the center; clamp to the
   surface value below `body_radius + margin`; multiply by a smoothstep fade
   over the outer ~15% of the SOI so the force reaches exactly zero at the
   boundary (protects the autopilot from chattering on a discontinuity and
   keeps trajectories kink-free). Orbits are only trusted inside the unfaded
   core; the ORBIT verb clamps its target radius into that band.
4. **One dominant well, with hysteresis.** Evaluate wells containing the
   entity, apply only the strongest pull; switch ownership only when the
   challenger beats the incumbent by ~10% so SOI-boundary flicker cannot flip
   wells tick to tick. Overlapping SOIs in dense fields degrade to "nearest
   big rock wins", which is predictable and readable.
5. **Opt-in affected set.** `GravityAffected` marker on ship roots (player
   AND AI - one arena, one physics) and torpedoes (PN guidance is closed-loop
   on line-of-sight rate; it self-corrects through wells). Turret rounds skip
   v1: flight times are short, the lead pip assumes straight-line ballistics,
   and per-bullet well queries are pure cost for imperceptible curvature.
   Section debris skips v1 (perf; wreckage raining onto the rock is a nice
   later flourish).
6. **Autopilot interactions stay honest.** STOP inside a well completes and
   hands back control while you fall - correct and intentional (the HUD shows
   the well; ORBIT is one key away). GOTO's arrival rule ignores gravity in
   v1: with the strength guardrail the error is small and per-tick replanning
   absorbs it; a gravity feedforward term in the burn solver is a recorded
   follow-up, not v1 scope. ORBIT phases: Plan (pick target radius = clamp of
   current radius into the stable band; orbit plane from the current r x v,
   falling back to the ship's up axis when velocity is near-radial or
   near-zero) -> Align -> Burn (match tangential v_circ through the real
   engine-cluster planner) -> Hold (micro-burn station-keeping when radius or
   tangential-speed error exceeds tolerance). Hold is the honest answer to
   semi-implicit-Euler energy drift: instead of freezing physics, the flight
   computer visibly earns the parking.
7. **HUD v1 is one line plus one cue.** Flight-status line grows well/orbit
   states (`GRAV <name> 0.8g`, `AP ORBIT - BURN | r 52 | 4.9 u/s`), and the
   screen-indicator substrate gets an "orbit available" cue while inside an
   SOI. Trajectory arcs, SOI rings, and orbit-path prediction belong to the
   parked diegetic-instruments task (20260709-103454), not v1.
8. **Module home and promotion posture.** `gravity.rs` in nova_gameplay as a
   sibling of `flight.rs`: a `GravityWell`/`GravityAffected` component pair
   and one FixedUpdate force system, scheduled with the existing flight sets.
   The well-force core is deliberately game-agnostic (component + pure force
   profile) - a future bevy_common_systems promotion candidate; keep the math
   in pure helpers to make that split cheap, per the promotion-eligible
   pattern.
9. **Tests ride the proven harness.** Pure-helper unit tests (force profile
   incl. clamp + fade, v_circ, dominant-well hysteresis) plus physics-level
   integration tests like the autopilot ones: seed a dynamic body at radius r
   with tangential v_circ and assert bounded radius over many ticks; assert
   zero force outside the SOI; engage ORBIT from near-rest inside a well and
   assert it reaches and holds a bounded orbit; assert breakout restores
   manual authority. All gravity tunables live in one reflected settings tree
   (juice retro R1.1: register the whole tree).

## Recommendation

Build **D** in two direction-level steps for the v0.5.0 arc: first the
gravity-well substrate (B: components, dominant-well force system with clamp +
fade + hysteresis, authored strength with radius-derived defaults, settings
tree, physics tests), then the ORBIT autopilot verb + minimal HUD on top of
the existing maneuver machine. The substrate is honest physics inside a
bounded gate - the real formula where it is fun, provably no clumping because
wells are one-way and bodies stay on rails. The verb is the gamer-friendly
part: "park near an asteroid, press one key, the ship visibly flies itself
into orbit and keeps it" - the same diegetic contract as STOP and GOTO, so it
inherits breakout, capability, destruction coupling, and the HUD line without
new concepts.

## Open questions

- **SOI density in asteroid fields.** With ~100u spacing and radius-derived
  SOIs, how often does a cruising ship hand off between wells, and is that
  fun or noisy? Playtest question; the knobs (SOI factor, minimum-radius
  threshold for wells) already exist in the design.
- **Turret lead vs curved targets.** Inside a well, targets and (later)
  rounds curve; the lead pip assumes straight lines. Likely imperceptible at
  PDC ranges given the strength guardrail - measure in the turret range
  before deciding whether the pip needs a gravity term.
- **AI in wells.** AI ships feel gravity (decision 5) but the AI brain does
  not know about it. Its constant thrust probably masks the pull; revisit
  with the smarter-AI task (20260708-162012) whether the AI needs well
  avoidance or its own ORBIT usage.
- **Gravity feedforward in STOP/GOTO.** Recorded follow-up: add the well
  acceleration into the arrival-rule solve when engaged inside an SOI, so
  long brakes near big bodies stay tight.
- **Fuel.** Station-keeping is free today because thrust is free. If fuel or
  reaction mass ever exists, Hold gets a real economy (orbits decay when you
  stop paying) - deliberately out of scope, but the diegetic design already
  supports it.

## Next steps

Direction-level tasks this spike seeded, for /plan to break into steps when
the v0.5.0 arc is planned:

- tatr 20260709-193338: gravity wells - bounded one-way gravity with SOI
  (physics substrate).
- tatr 20260709-193339: ORBIT autopilot verb - circularize and station-keep
  inside a gravity well (+ flight-status/HUD cue).

Deferred, recorded here so they are not lost: SOI/trajectory visualization
(fold into diegetic instruments 20260709-103454), gravity feedforward in
STOP/GOTO, debris/wreckage as gravity-affected, AI well-awareness, turret
lead gravity term, bcs promotion of the well-force core.
