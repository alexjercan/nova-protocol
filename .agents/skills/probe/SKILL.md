---
name: probe
description: Add or run Nova examples and probe checks, inspect correctness reports, and evaluate performance evidence.
---

# Probe

Read `examples/systems/README.md` before changing a systems range. Read the
probe sections of `docs/development.md` and `docs/performance.md` before changing
the harness or making performance claims.

- Reproduce a bug before fixing it with an affected `examples/systems/` range.
- Use `systems/` for asserted behavior, `playable/` for a real human affordance,
  and `screenshots/` for documentation capture.
- Build examples with `AppBuilder`. Treat conversion to it as behavioral work.
- Put an `outcome: <slug>` marker beside every range assertion and add the same
  slug to `crates/nova_probe_cli/tests/catalog_drift.rs`.
- Never assert timing. Record it and judge repeat-set evidence against a named
  reference.

Run Cargo through the Nix development shell:

```bash
nix develop --command cargo run --features debug probe run <name> --correctness-only
nix develop --command cargo run --features debug probe run <name>
```

Run only affected ranges unless the user requests a fleet sweep. Inspect
`checks.json` and `report.html`; `SKIPPED` means unmeasured. Use a rendered run
when rendering behavior can change.
