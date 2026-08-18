# Profiling loads that reproduce the stutter, in the probe

- STATUS: IN_PROGRESS
- PRIORITY: 90
- TAGS: v0.11.0, performance, test, harness

Epic: `20260818-220812`.

The regression in `PERF-REGRESSION` reached master because nothing in the
checks answers "does this cost a frame". `nova_probe` already has the
machinery - clean, frametime, profiled and samply passes - what it does not
have is loads worth profiling.

## Build the cases

- **`wfc_arena` 4v4.** The owner's named candidate. A real fight with real
  ships, turrets, torpedoes and rocks, driven by autopilot so it is repeatable.
  This is the headline case.
- **The sandbox as it ships.** `asteroid_field` is the case that broke; it
  should be a standing profile target, not a thing someone happens to fly.
- **Extend the existing stress set.** `examples/systems/stress_bullets`,
  `stress_many_structures`, `stress_one_structure`, `stress_torpedoes` exist
  but are not framed as profiling loads. Give each a frametime pass and a
  recorded budget.

## Make the numbers land

- Report the WORST frame and top system self-time, not the mean. Every stutter
  this project has hit was a tail.
- Record a budget per case, so a future regression fails a check instead of
  waiting for someone to fly it.
- The chrome-trace reader pairs B/E spans for self time (tracing-chrome emits
  no `ph:"X"`) - that is already known, do not rediscover it.

## Then answer the open questions with data

The owner raised pooling as a maybe ("this is ECS so it doesn't need pools, but
maybe there are certain things that can benefit"). That is a profile finding,
not a design decision - do NOT build pooling machinery speculatively. Carve
shards are the one population with an obvious churn shape (2+ per mark, 2.5 s
life, hundreds sustained); if the profile says they cost, that is where a pool
is argued for, with the number attached.

## Done when

- Each case runs from `probe run` and prints a worst-frame number.
- A recorded budget per case that a regression would break.
- A ranked list of what actually costs, which is what the rest of the epic is
  scheduled against.
