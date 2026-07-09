# AI torpedo usage from Engage: launch envelope + cooldown

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.4.0,ai,spike,torpedo


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 3)

Goal: AI ships fire their torpedo bays. From Engage, write
TorpedoSectionInput when inside a launch envelope: range band, rough
alignment, per-bay cooldown; reuse the commit-on-launch targeting the player
path already has (input/player.rs). Needs standoff flight to read well: a
point-blank launch self-hits (see 20260709-140559 on blast self-harm).

Depends on: 20260709-225727 (AITarget), 20260709-225729 (standoff flight).
