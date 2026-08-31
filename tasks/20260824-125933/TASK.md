# An agent plays the game, and a benchmark scores it

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, idea, autopilot

Ideation, broad on purpose. Stays a future promise (owner, 2026-08-31).
Absorbed its duplicate `20260824-125929` on the same date.

Follow-up of `20260820-174148` (landed v0.12.0): an external agent (an
LLM or a script) plays a full scenario over the stdin/stdout channel in
--norender step mode - reads snapshots, writes inputs, completes
objectives with no human at the keyboard. On top of that, a benchmark
harness scores autonomous runs - completion, time, damage taken, ammo
spent - across the scenario set, tracked release over release.
