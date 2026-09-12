---
name: verify
description: >-
  Ground Nova bugs, features, and refactors in code and test or play evidence.
  Required before proposing, implementing, or reviewing behavior changes.
---

# Verify

Use AGENTS.md's proposal fields. Cite code and sketch proposed APIs.
Read [the bench guide](../../../docs/agent-bench.md) before agent play.
Inspect a nearby fixture under `crates/nova_bench/scenarios/`.

- Reuse a sufficient check: unit test for local logic; asserted example for
  ECS interactions; scenario plus agent play for player flows. Explain skips.
- Read [Probe](../probe/SKILL.md) and [Content](../content/SKILL.md) as needed.
- Keep test scenes easy: one behavior, nearby targets, suitable equipment,
  generous time, no unrelated hazards. Keep the actual failure preconditions.
- Before play, name the fixture, revision, seed, budgets, pilot/model, and
  output path. Give the pilot steps: action -> expected result -> evidence.
  State pass/stop conditions. Honor approved budgets and model settings.
- Use separate output paths. Preserve and replay the first failing audit.
  A replay match reproduces the outcome, not a pass; close is not identical.

Run from the repository root through Nix:
```bash
nix develop --command cargo run --features dev bench play <scene> \
  --agent pi --seed <seed> --ticks <ticks> --turns <turns> \
  --deadline <seconds> --goal "<steps and pass conditions>" --out <run>
nix develop --command cargo run --features dev bench replay \
  <run>/audit.jsonl --out <replay>
```

- Inspect `score.json`, `audit.jsonl`, and logs, not exit zero or pilot prose.
  For visuals, use `--record <dir>` and inspect frames; it needs GPU and X.
- Distinguish game defects from pilot, fixture, and harness errors using the
  audit. A loss is not itself a bug. Unreached steps and unclear causes remain
  untested. The baseline pilot is a combat hunter, not a general test driver.
- Add an assertion that fails without the fix; rerun it and the affected flow.
  Cite before/after artifacts and gaps. Reviews propose fixes, never edit.
