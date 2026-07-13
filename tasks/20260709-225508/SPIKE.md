# Spike: Splitting "smarter enemy AI" into combat behavior work items

- DATE: 20260709-225508
- STATUS: RECOMMENDED
- TAGS: spike, ai, combat, v0.4.0

## Question

Task 20260708-162012 ("Smarter enemy AI") is a single umbrella covering
target selection, evasion, patrol and firing discipline. How should it be
decomposed into small, independently shippable work items across four axes -
how the AI uses weapons, how it flies, how it evades attacks, and how it
defends itself - given what the codebase already provides? A good answer is
an ordered set of direction-level tasks, each landable on its own, with the
shared skeleton they plug into named explicitly.

## Context

The AI brain today (`crates/nova_gameplay/src/input/ai.rs`, ~280 lines) is
four systems with no state: every frame, every `AISpaceshipMarker` ship
computes a chase direction toward the player (distance-scaled speed with a
brake regime), writes an absolute rotation command, thrusts when aligned
above 0.95, points its turrets at the player's live-structure anchor, and
fires when a muzzle aligns above 0.95. There is no target choice (the player
is a hardcoded `Single`), no reaction to being shot, no notion of standoff
range (pure pursuit converges to point-blank parking), no torpedo use, and
no self-preservation.

What the rest of the codebase already provides, which the split leans on:

- **Relations**: task 20260708-203708 (OPEN) adds the minimal
  hostile/neutral/own model. Target selection over "hostiles" instead of
  "the player" is blocked on it.
- **Flight**: `flight.rs` has `slew_rotation` + `hull_turn_rate` (the player
  rotation path), `FlightIntent`, and a phased `Autopilot`
  (Align/Burn/Brake with GOTO). Task 20260709-155921 (OPEN) moves the AI
  rotation path onto the slew helpers and fixes the delta-into-absolute
  input bug - the flight foundation for everything below.
- **Aim anchors**: `live_structure_anchor` (task 20260709-150711, CLOSED)
  already de-bugged where the AI aims; 155921 notes the own-side of the
  chase vector still reads the root origin.
- **Weapons**: turrets consume `TurretSectionTargetInput` (point) +
  `TurretSectionTargetVelocity` -> `lead_intercept_point`; the AI feeds no
  velocity, so its turrets never lead (the player-side fix is
  20260709-173700). Torpedo bays consume `TorpedoSectionInput(bool)` with
  commit-on-launch targeting from the shooter's lock; the AI never fires
  them.
- **Damage signals**: `HealthApplyDamage` events and the integrity/section
  pipeline exist, so "I am being shot" and "I am badly damaged" are
  observable without new plumbing.
- **Targeting state pattern**: `input/targeting.rs` (lock, focus, component
  lock) shows the resource/component state pattern the AI-side state should
  mirror.

## Options considered

- **Keep the umbrella, implement it in one task.** One giant /work pass over
  ai.rs. Rejected: the four axes have different dependencies (factions,
  flight slew, torpedoes), different risk, and different test harnesses;
  one branch that touches all of them reviews badly and blocks partial
  delivery.

- **Split by system (rotation / thrusters / turrets / torpedo).** Matches
  the current file layout, but every behavior (evade, patrol, retreat) cuts
  across all four systems, so each task would still touch everything.
  Rejected.

- **Split by behavior on a shared state skeleton (chosen).** A small
  behavior-state component (Idle, Patrol, Engage, Evade, Retreat) with a
  transition system owns "what am I doing"; each seeded task implements one
  behavior or one weapon competency against that skeleton. Tasks become
  independently shippable: the skeleton with only Engage wired reproduces
  today's behavior, and each later task adds one state or sharpens one
  competency.

- **A behavior tree / utility-AI library.** Overkill for one enemy archetype
  and five states; the transition logic is a dozen lines of thresholds.
  Revisit if archetypes multiply (a future spike), the state enum does not
  paint us into a corner - behavior trees can drive the same state
  component later.

- **Do nothing.** Enemies stay editor-only chase drones; the mission work
  (133026-133029) has nothing to fight. Rejected - this is the 0.4.0
  enabler.

## Recommendation

Split along behaviors on a shared state skeleton, in three waves. Existing
tasks 20260708-203708 (factions) and 20260709-155921 (AI rotation path) are
the unchanged prerequisites; the umbrella 20260708-162012 is superseded by
the tasks below and closed with a pointer here.

**Wave 1 - skeleton and aiming (no new dependencies):**

1. *AI behavior state machine skeleton.* `AIBehaviorState` enum component
   (Idle, Patrol, Engage, Evade, Retreat) + one transition system with
   tunable trigger constants; today's chase/shoot logic becomes the Engage
   state's implementation, everything else stubs to Engage-like defaults.
   Pure refactor + scaffolding; lands first so later tasks slot in.
2. *Threat-scored target selection.* Replace the hardcoded player `Single`
   with a per-ship `AITarget(Option<Entity>)` picked over hostiles (via the
   relation model), scored by distance, recent-damage-to-me, and type
   (ships over torpedoes over asteroids), with switch hysteresis so the
   pick does not flip-flop. All four AI systems consume `AITarget`.
   Blocked on 203708.
3. *Fire discipline: lead, bursts, range gating.* Feed the target root's
   `LinearVelocity` into `TurretSectionTargetVelocity` so AI turrets
   actually lead (the AI-side sibling of 20260709-173700); gate fire on an
   effective-range envelope, not just alignment; burst cadence
   (fire-window/cooldown timers) instead of continuous spray.

**Wave 2 - flight (blocked on 155921):**

4. *Engagement flight: standoff orbit/strafe.* Replace pure pursuit with a
   range envelope: approach when far, hold/orbit at preferred weapons
   range (lateral component in the desired direction), extend when too
   close. Kills both the ramming convergence and the parked-turret duel.
5. *Patrol and idle flight.* Waypoint patrol (reusing the GOTO
   autopilot/FlightIntent machinery where possible) plus hostile-detection
   range that transitions Patrol -> Engage; Idle = station-keeping drift.
   Makes AI ships placeable in scenarios before combat starts.

**Wave 3 - reaction and survival:**

6. *Evasion under fire.* A threat model (took `HealthApplyDamage` recently,
   hostile within range aiming at me) drives Engage -> Evade: timed jink
   maneuvers (lateral thrust bursts, heading changes off the pursuit
   vector), decaying back to Engage. This is the axis that makes fights
   read as "it noticed me shooting".
7. *AI torpedo usage.* Fire `TorpedoSectionInput` from Engage when inside a
   launch envelope (range band, rough alignment, per-bay cooldown), reusing
   the commit-on-launch targeting the player path already has. Small once
   `AITarget` exists; listed here because it only reads well with standoff
   flight (a point-blank torpedo self-hits, see 20260709-140559).
8. *Torpedo threat response: evade + point-defense.* Detect a hostile
   torpedo targeting me; prioritize it as a turret target (PDC role,
   falls out of target selection's type scoring) and/or break with a
   perpendicular burn while it closes. First consumer of target-type
   scoring beyond ships.
9. *Self-preservation: retreat on low integrity.* Section-loss/health
   threshold flips to Retreat: burn away from the current threat,
   optionally re-engage when the threat de-aggros. Defines the AI's
   "defend itself" endgame and gives fights a natural end state.

Each task carries its own physics/behavior tests on the existing flight
test-harness patterns (the 155921 and 150711 test styles); the state
skeleton makes transitions unit-testable without a full sim.

## Open questions

- Trigger tuning (evade thresholds, standoff range, retreat integrity
  fraction) - all constants; playtest knobs, decide per task.
- Does Evade need true incoming-projectile detection (raycast/proximity
  over projectile entities) or is recent-damage + being-aimed-at enough?
  Start with the cheap signals; a projectile-proximity upgrade is a
  follow-up if evasion feels psychic-less.
- AI component-targeting (aiming at specific sections, per the
  component-lock spike 20260709-192358) is explicitly out of scope - that
  spike marked it future work.
- Multiple enemy archetypes (interceptor vs sniper vs bomber personalities
  via constant sets) - future spike once the behaviors exist to vary.

## Next steps

Prerequisites (existing, unchanged): tatr 20260708-203708 (factions),
tatr 20260709-155921 (AI rotation path). Superseded: tatr 20260708-162012
(umbrella, closed pointing here).

Seeded direction-level tasks, priorities encode intended order:

- tatr 20260709-225726: AI behavior state machine skeleton (Idle/Patrol/Engage/Evade/Retreat)
- tatr 20260709-225727: AI threat-scored target selection over the relation model
- tatr 20260709-225728: AI fire discipline: turret lead, burst cadence, range gating
- tatr 20260709-225729: AI engagement flight: standoff orbit/strafe envelope
- tatr 20260709-225730: AI patrol and idle flight states
- tatr 20260709-225731: AI evasion under fire (threat model + jink maneuvers)
- tatr 20260709-225732: AI torpedo usage from Engage (launch envelope + cooldown)
- tatr 20260709-225733: AI torpedo threat response: point-defense + break-off burn
- tatr 20260709-225734: AI self-preservation: retreat on low integrity
