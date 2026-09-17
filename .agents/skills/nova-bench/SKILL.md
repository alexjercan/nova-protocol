---
name: nova-bench
description: Run Nova player flows through agent play and deterministic replay.
---

# Nova Bench

Use this skill for the `bench` command. Read the complete
[bench guide](../../../docs/agent-bench.md) and inspect a nearby fixture under
`crates/nova_bench/scenarios/` before planning a play.

## Plan

- Name the fixture, revision, seed, tick and turn budgets, deadline, pilot,
  model, thinking level, and unique output path.
- List each action, expected world result, and evidence field.
- State pass and stop conditions. Keep the scene easy but retain the trigger.
- Use an existing fixture when it can prove the flow. Do not make unrelated
  hazards part of the check.

## Run

```bash
nix develop --command cargo run --features dev bench play <scene> \
  --agent pi --seed <seed> --ticks <ticks> --turns <turns> \
  --deadline <seconds> --goal "<steps and pass conditions>" --out <run>
nix develop --command cargo run --features dev bench replay \
  <run>/audit.jsonl --out <replay>
```

Use `--record <dir>` and inspect frames for a visual claim. Recording needs GPU
and X. Run one game or GPU measurement at a time.

## Judge

- Inspect `score.json`, `audit.jsonl`, game and agent logs, and recordings.
- Judge world state and assertions, not exit zero, outcomes, or pilot prose.
- Separate game, pilot, fixture, and harness failures. Unreached means untested.
- Preserve the first failure. Replay it when deterministic reproduction matters.
  A matching replay reproduces the outcome; it does not prove the fix.
- Treat the baseline pilot as a combat hunter, not a general test driver.
- Record before/after artifacts, skipped checks, and remaining uncertainty.
