# AI self-preservation: retreat on low integrity

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.4.0,ai,spike,health


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 3)

Goal: fights get an end state. A section-loss / integrity threshold flips
the AI to Retreat: burn away from the current threat at full thrust,
optionally re-engaging if the threat de-aggros or never. Defines the AI's
self-preservation endgame; tunable retreat threshold constant.

Depends on: 20260709-225726 (skeleton), 20260709-225729 (flight envelope
machinery for the disengage vector).
