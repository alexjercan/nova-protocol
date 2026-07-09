# AI evasion under fire: threat model + jink maneuvers

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.4.0,ai,spike,handling


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 3)

Goal: the AI reacts to being shot. A cheap threat model (received
HealthApplyDamage recently; a hostile within range is aiming at me) drives
Engage -> Evade: timed jink maneuvers (lateral thrust bursts, heading changes
off the pursuit vector) that decay back to Engage. Start with the cheap
signals; true incoming-projectile proximity detection is a follow-up if
evasion feels blind.

Depends on: 20260709-225726 (skeleton), 20260709-225729 (engagement flight,
for sane re-entry into Engage).
