---
name: nova-probe
description: Run deterministic Nova correctness and performance probes.
---

# Nova Probe

Use this skill for the `probe` command and probe examples. Read
[systems guidance](../../../examples/systems/README.md) before authoring. Read
the probe sections of [development](../../../docs/development.md) and
[performance](../../../docs/performance.md) for harness or timing work.

- Reuse an affected example before adding one. A new range needs one named,
  important behavior, failure, or invariant; never add it for coverage.
- Use `examples/systems/` for asserted behavior, `examples/playable/` for human
  interaction, and `examples/screenshots/` for documentation capture.
- Use `bug_` for one reproduced defect, `system_` for subsystem behavior, and
  `stress_` for load. Build apps with `AppBuilder`.
- Register examples in `Cargo.toml`; auto-discovery is off. Put an
  `outcome: <slug>` marker beside each range assertion and register the slug in
  `crates/nova_probe_cli/tests/catalog_drift.rs`.
- `probe scenario` accepts an ID or loose RON file. A clean boot does not prove
  a player goal. Use assertions or `nova-bench` for that claim.
- Never assert timing. Compare repeat sets with a named matched reference. Run
  one GPU measurement at a time.

Run only affected checks from the repository root through Nix:

```bash
nix develop --command cargo run --features dev probe run <name> \
  --correctness-only
nix develop --command cargo run --features dev probe scenario <id-or-file> \
  --correctness-only
nix develop --command cargo run --features dev probe run <name> --repeat 5
```

Use separate `--out` paths when preserving failures. Inspect `checks.json`,
`report.html`, logs, and frames. `SKIPPED` means unmeasured. Headless output
cannot prove appearance. Run the whole fleet only when the user requests it.
