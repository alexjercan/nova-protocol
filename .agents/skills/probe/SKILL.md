---
name: probe
description: >-
  Add or run Nova examples and scenario probes. Inspect correctness reports
  and performance evidence. Verify selects checks; Probe runs them.
---

# Probe

Follow AGENTS.md. Read [Verify](../verify/SKILL.md) before selecting checks.
Read [systems guidance](../../../examples/systems/README.md) before authoring.
Read the probe sections of [development](../../../docs/development.md) and
[performance](../../../docs/performance.md) for harness or timing work.

- Reuse an affected example. Use `examples/systems/` for asserted behavior,
  `examples/playable/` for human interaction, and `examples/screenshots/`
  for documentation capture.
- Use `bug_` for one confirmed defect, `system_` for subsystem behavior, and
  `stress_` for load. Build with `AppBuilder`; conversion is a behavior change.
- Register examples in `Cargo.toml`; auto-discovery is off. Pair each assertion
  with an `outcome: <slug>` marker and add the slug to
  `crates/nova_probe_cli/tests/catalog_drift.rs`. Check the affected roster.
- `probe scenario` takes an ID or loose RON file without an example entry.
  A clean boot does not prove a player goal; use assertions or agent play.
- Never assert timing. Compare repeat sets against a named reference under
  matching conditions, with no concurrent GPU runs.

Run only affected checks from the repository root through Nix:
```bash
nix develop --command cargo run --features dev probe run <name> \
  --correctness-only
nix develop --command cargo run --features dev probe scenario <id-or-file> \
  --correctness-only
nix develop --command cargo run --features dev probe run <name> --repeat 5
```

`dev` enables `debug`; both are valid. Preserve failures with separate
`--out <base-dir>` values: the default path is reused at a commit. Inspect
`checks.json` and `report.html`; `SKIPPED` is unmeasured. Inspect rendered
output for visual claims. Run the whole fleet only when requested.
