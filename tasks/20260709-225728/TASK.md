# AI fire discipline: turret lead, burst cadence, range gating

- STATUS: OPEN
- PRIORITY: 74
- TAGS: v0.4.0,ai,spike,turret


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 1)

Goal: make AI gunnery honest instead of a continuous aligned-spray. Feed the
target root's LinearVelocity into TurretSectionTargetVelocity so AI turrets
actually lead via lead_intercept_point (the AI-side sibling of
20260709-173700); gate fire on an effective-range envelope, not just muzzle
alignment; add burst cadence (fire-window/cooldown timers) instead of holding
the trigger while aligned.

Depends on: 20260709-225726 (skeleton), pairs with 20260709-225727 (AITarget).
