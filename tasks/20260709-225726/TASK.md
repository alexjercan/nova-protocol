# AI behavior state machine skeleton (Idle/Patrol/Engage/Evade/Retreat)

- STATUS: CLOSED
- PRIORITY: 78
- TAGS: v0.4.0,ai,spike


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 1)

Goal: give the AI brain (crates/nova_gameplay/src/input/ai.rs) a small
behavior-state skeleton the other AI tasks plug into: an AIBehaviorState enum
component (Idle, Patrol, Engage, Evade, Retreat) on the AI ship root, plus one
transition system with tunable trigger constants. Today's chase/aim/shoot
logic becomes the Engage state's implementation; unimplemented states stub to
Engage-like defaults so behavior is unchanged until later tasks land.
Transitions must be unit-testable without a full sim.

Supersedes the umbrella task 20260708-162012 together with its siblings
(20260709-225727 .. 20260709-225734).

## Steps

- [x] `AIBehaviorState` component enum in `input/ai.rs`: `Idle`, `Patrol`,
      `Engage`, `Evade`, `Retreat`, with doc comments pointing each
      not-yet-implemented state at its arc task (Patrol/Idle flight
      20260709-225730, Evade 20260709-225731, Retreat 20260709-225734).
      Default `Engage` (today's only real behavior). Required by
      `AISpaceshipMarker` (the allegiance-by-require pattern), registered
      for reflection.
- [x] Transition system `update_behavior_state` with tunable trigger
      constants: the skeleton's one real transition is target presence -
      no hostile target in the world -> `Idle` (freeze/coast), hostile
      present -> back to `Engage`. Evade/Retreat triggers are later tasks;
      the match structure and constants live here so they slot in.
- [x] Gate the four AI systems on the state: `Engage` (and, until their
      tasks land, `Evade`/`Retreat`, which stub to Engage behavior) run
      today's logic; `Idle`/`Patrol` zero the thrusters, hold fire, clear
      the turret target input, and freeze the rotation command.
- [x] Tests: default state is Engage (existing AI tests stay green
      unchanged); transition to Idle without a player and back with one;
      Idle zeroes thrust + fire + turret input and freezes the command.
- [x] Verify: cargo fmt, cargo check --workspace, ai:: module tests (skip
      full local suite per user instruction; report skips honestly).

## Notes

- Relevant files: crates/nova_gameplay/src/input/ai.rs (all of it).
- The Single<player> queries currently make every AI system no-op when no
  player exists; the Idle state makes that explicit and testable instead of
  accidental, and gives later tasks a hook that does not depend on the
  player Single (225727 replaces it with AITarget).
- Deliberately NOT here: patrol waypoints, evade triggers/maneuvers,
  retreat thresholds, target selection - each is its own arc task.

## Resolution (20260709)

Shipped `AIBehaviorState` (required by the AI marker, default Engage so
behavior is unchanged at spawn), the pure `next_behavior_state` transition
(no hostile -> Idle from anywhere; hostile present -> passive states
engage, combat states hold), and the state gate on all four AI systems.
The player `Single` params became `Option<Single>` so Idle can actively
ZERO thrust/fire/aim instead of the old accidental freeze-at-last-value
when the player despawns - the systems now run and write explicit zeros.
The rotation command freezes (dead-helm semantics) in non-engaging states.
4 new tests (transition matrix, require default, idle/re-engage cycle,
Idle-zeroes-actuators); full nova_gameplay suite 201/201 green locally
this once (fast), fmt + check clean. Skipped per user instruction: clippy.

Reflection: the interesting design point was Idle-as-explicit-zero vs
skip - skipping leaves stale inputs (a thruster stuck at 1.0). Writing
zeros makes the state observable and testable. Chained the AI systems
after the transition so a flip takes effect the same frame.
