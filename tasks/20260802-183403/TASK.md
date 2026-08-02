# Migrate nova_debug, nova_probe, and the example fleet onto nova_autopilot

- PRIORITY: 92
- TAGS: v0.10.0, tooling, autopilot, examples
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183349, 20260802-183352

## Story

Switch Nova onto `nova_autopilot`. `nova_debug::harness` keeps only the
Nova-shaped layer - the `GameStates` presets, the scenario-loaded smoke
assertion, and the reel hooks (camera pose, body freeze, HUD and overlay
hiding) - over the crate's drivers. `nova_probe` registers its capture
collector against the crate directly instead of reaching through
`nova_gameplay::bevy_common_systems`. One atomic migration, because the
activation envs rename with no compatibility aliases.

## Steps

- [ ] Rewire `nova_debug::harness` and `nova_probe` (`capture.rs`, `native/env.rs`,
      `native/spec.rs`) onto `nova_autopilot`, supplying the Nova hooks the ports
      left to the caller.
- [ ] Rename the activation contract everywhere it is read or written -
      `NOVA_AUTOPILOT`, `NOVA_SHOT`, `NOVA_REEL`, `NOVA_AUTOPILOT_DEADLINE` -
      across the example fleet, `tests/examples_smoke.rs`, the
      `nova_gameplay` harness-mute list, and `scripts/gen-web-screenshots.py`.

## Definition of Done

- No repository-owned code names a BCS activation env or harness path.
  (cmd: `! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|debug::harness" crates examples scripts tests --glob '*.rs' --glob '*.py'`)
- The workspace builds with the debug feature and all targets.
  (cmd: `nix develop --command cargo check --workspace --all-targets --features debug`)
- The example fleet still smokes under the renamed env.
  (cmd: `nix develop --command cargo test --test examples_smoke`)
- A probe run still produces a report with the capture collector negotiating
  the exit. (cmd: `nix develop --command cargo run -p nova_probe -- run playable --fps`)

## Notes

- Parent: `20260802-120019`. Depends on the driver ports.
- A harness run mutes audio off the env list in
  `crates/nova_gameplay/src/settings.rs`; missing it makes probe runs audible.
- `nova_probe` gains a direct `nova_autopilot` dependency; the direction is
  probe -> autopilot, never the reverse.
