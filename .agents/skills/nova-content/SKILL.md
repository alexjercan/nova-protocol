---
name: nova-content
description: Change Nova content, runtime IDs, builders, generation, and lint.
---

# Nova Content

Follow [AGENTS.md](../../../AGENTS.md) and the approved `nova-implement` plan.
Read a nearby builder or fixture before editing.

- Edit Rust builders, then regenerate base content. Never hand-edit generated
  files under `assets/base/**/*.content.ron`.
- Treat loose bench fixtures as authored RON. Inspect a nearby fixture under
  `crates/nova_bench/scenarios/` before changing one.
- Require authored values that the game needs. Do not add a Serde default,
  optional field, alias, or legacy parser to make an incomplete refactor load.
- Use `Option` only when absence has explicit behavior in the content contract.
- Treat prototype, scenario, style, asset, and authored IDs as runtime strings.
  Search every consumer when an ID changes; delete the old ID.
- Put shared IDs in the lowest crate already used by every consumer. Do not add
  a dependency edge only for a constant. Keep fixture IDs local.
- Read [Nova Docs](../nova-docs/SKILL.md) when a format, authoring contract,
  balance value, shipped catalog, or player-visible behavior changes.

Run only affected commands from the repository root through Nix:

```bash
nix develop --command cargo run content gen
nix develop --command cargo run content lint
nix develop --command cargo run content lint --target <mod>
```

`--target` accepts a mod ID or directory, not a loose scenario. Load a loose
fixture through `nova-probe` or `nova-bench`. Inspect generated diffs and lint
output, then run the affected content flow.
