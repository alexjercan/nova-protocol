# Spike: why are the second scenarios brutally hard, and how do we rework scenarios to be fair without dumbing down AI or damage

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: spike, v0.7.0, scenario, balance

Research spike, deliverable is SPIKE.md in this folder. Question: why do the
second scenarios (broadside, ledger_ch2) overwhelm even a top-percentile
player, and what scenario-level reworks fix it without dumbing down AI,
reducing player damage taken, or easing controls.

Verdict: RECOMMENDED. Root causes, ranked: better_turret loadouts on mook
enemies (400 dps each, perfect lead), spawns inside kill range, unstaggered
crossfire, absent-or-paper cover plus an AI with no line-of-sight concept,
zero-delay wave chaining (engine has no timers), an escort that dies to the
player's own dodged bursts, and full-scenario restarts on death.

Seeded: 20260717-112622 (AI LOS fire gate), 20260717-112630 (ledger_ch2
rework), 20260717-112639 (broadside rework), 20260717-112647 (scenario timer
primitive), 20260717-112656 (balance audit rig).
