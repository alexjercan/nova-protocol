# Clear the stress/ round-1 review findings (R1.1-R1.4)

- PRIORITY: 0
- TAGS: backlog,examples,testing
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- DEPENDS ON: 20260804-094006

## Story

Round 1 of `20260804-094006` APPROVEd with four unfixed findings. They are all
small, none blocked the land, and they are collected here rather than reopening
an approved branch.

## Done when

- [ ] `cmd:` (R1.1, MINOR) `examples/stress/many_bodies.rs` `swarm_count()`
      ends `.max(1)` with a one-line rationale, matching
      `many_sections.rs`/`many_projectiles.rs`. Today `NOVA_STRESS_COUNT=0`
      makes `many_bodies` spawn nothing and pass its returned-to-baseline
      assertion vacuously, while the same variable clamps its two siblings.
- [ ] `cmd:` (R1.2, NIT) `crates/nova_probe/src/bin/probe/native/env.rs:66`
      no longer says "non-`perf/` window" - that category is gone. Reword to
      "non-frame-time window".
- [ ] `cmd:` (R1.3, NIT) `crates/nova_probe/src/fixtures.rs`'s module doc
      distinguishes it from the bin-side
      `crates/nova_probe/src/bin/probe/native/fixtures.rs` (probe's synthetic
      catalog), or the lib module is renamed `scenario_fixtures`.
- [ ] `cmd:` (R1.4, NIT) the Fibonacci-shell duplication between
      `many_bodies.rs::rock_position` and
      `many_projectiles.rs::target_position` is either commented as deliberate
      example self-containment at both sites, or hoisted into
      `nova_probe::fixtures`.
