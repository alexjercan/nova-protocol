---
name: content
description: >-
  Change Nova content, runtime IDs, generated RON, and generation or lint rules.
---

# Content

[AGENTS.md](../../../AGENTS.md) requires this skill before content changes
and owns the always-on generation and runtime-ID rules. Use
[Verify](../verify/SKILL.md) to plan a reproduction or player-flow check.

- Edit Rust content builders, then regenerate. Never hand-edit generated files
  under `assets/base/**/*.content.ron`. Loose bench fixtures are authored RON,
  not generated base content. Inspect a nearby fixture under
  `crates/nova_bench/scenarios/` before editing.
- Treat prototype, scenario, style, asset, and other authored IDs as runtime
  strings. Grep every renamed ID and run affected content.
- Put cross-crate IDs in the lowest crate already shared by all consumers. Do
  not add a dependency edge only for a constant.
- Keep test and example IDs local.
- Read [Docs](../docs/SKILL.md) when formats, authoring contracts, balance,
  or a user-visible catalog changes.

Run from the repository root through Nix:

```bash
nix develop --command cargo run content gen
nix develop --command cargo run content lint
nix develop --command cargo run content lint --target <mod>
```

`--target` takes a mod ID or directory, not a loose scenario file. Load a
loose fixture through `probe scenario` or `bench play` as planned in Verify.
Review generated diffs and lint output, then run affected content. Run only
the commands relevant to the change.
