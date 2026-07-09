# AI behavior state machine skeleton (Idle/Patrol/Engage/Evade/Retreat)

- STATUS: OPEN
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
