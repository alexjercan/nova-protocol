# AI torpedo threat response: point-defense + break-off burn

- STATUS: OPEN
- PRIORITY: 64
- TAGS: v0.4.0,ai,spike,torpedo,turret


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 3)

Goal: the AI defends itself against torpedoes. Detect a hostile torpedo
whose target is me; prioritize it as a turret target (PDC role - this should
mostly fall out of target-type scoring in 20260709-225727) and/or break with
a perpendicular burn while it closes (stressing PN guidance). First consumer
of target-type scoring beyond ships.

Depends on: 20260709-225727 (target selection), 20260709-225731 (evade
maneuver machinery).
