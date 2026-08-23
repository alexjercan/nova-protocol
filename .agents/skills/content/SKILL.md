---
name: content
description: Change Nova authored content, runtime IDs, generated RON, and content generation or lint behavior.
---

# Content

- Edit Rust content builders, then regenerate. Never hand-edit generated files
  under `assets/base/**/*.content.ron`.
- Treat prototype, scenario, style, asset, and other authored IDs as runtime
  strings. Grep every renamed ID and run affected content.
- Put cross-crate IDs in the lowest crate already shared by all consumers. Do
  not add a dependency edge only for a constant.
- Keep test and example IDs local.
- Use the `docs` skill when a format, authoring contract, balance value, or
  user-visible catalog changes.

Run Cargo through the Nix development shell:

```bash
nix develop --command cargo run content gen
nix develop --command cargo run content lint
nix develop --command cargo run content lint --target <mod>
```

Review generated diffs and lint output. Run only the commands relevant to the
change.
