# AI self-preservation: retreat on low integrity

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.7.0,ai,spike,health,wontdo

Spike: tasks/20260709-225508/SPIKE.md (wave 3)

Goal: fights get an end state. A section-loss / integrity threshold flips
the AI to Retreat: burn away from the current threat at full thrust,
optionally re-engaging if the threat de-aggros or never. Defines the AI's
self-preservation endgame; tunable retreat threshold constant.

Depends on: 20260709-225726 (skeleton), 20260709-225729 (flight envelope
machinery for the disengage vector).

## Reason

I don't want the AI to retreat (at least the enemy). We will see if something
like this is needed later;
