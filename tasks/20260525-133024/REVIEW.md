# Review: Torpedo bay shooting particles

- TASK: 20260525-133024
- BRANCH: feature/torpedo-launch-particles

## Round 1

- VERDICT: APPROVE

The diff delivers the Goal: firing a torpedo now emits a launch particle burst
at the bay spawner. It is a faithful mirror of the already-reviewed turret
muzzle-effect pattern (`insert_turret_barrel_muzzle_effect` +
`on_projectile_marker_effect`): a spawn-on-command hanabi effect
(`with_emit_on_start(false)`) parented to the spawner, triggered from the
projectile `Add` observer via `EffectSpawner::reset()`, wasm-gated like the
existing blast burst. Every ticked step matches the code, and the honesty notes
(visual look is a playtest item) are accurate.

Verification performed by the reviewer:
- `cargo fmt --check` clean; `cargo check -p nova_gameplay` clean (only an
  unrelated upstream `proc-macro-error2` future-incompat warning).
- `06_torpedo_range` autopilot smoke test (built binary, run with
  `BEVY_ASSET_ROOT`/`DISPLAY=:99`/`BCS_AUTOPILOT=1`): reached Playing, repeated
  `range: torpedo fired`, `autopilot: cycle complete, no panic (t=6.0s)`,
  exit 0, and crucially **no** `effect for spawner ... not found` errors, so the
  per-shot effect lookup and `reset()` fire on every launch.
- Full `cargo test` / `clippy` deferred to CI per project policy
  (`.github/workflows/ci.yaml`); no new unit-testable pure functions were added
  (the effect is declarative hanabi config; the smoke test is the integration
  check, matching how the turret muzzle effect ships untested).

Correctness spot-checks that passed:
- Multi-bay safe: each projectile carries its own `TorpedoSectionSpawnerEntity`
  and each spawner owns its effect child, so the `ChildOf(parent) == spawner`
  lookup targets the correct bay.
- World-space `normal`: neither turret, blast, nor this effect sets
  `SimulationSpace`, so all use hanabi's default (Global). Passing the spawner's
  world-space `up` as `normal` is therefore correct (and matches how
  `shoot_spawn_projectile` derives the launch direction from `spawner.up()`).

Findings below are all NIT-level and left to the implementer's discretion; none
block.

- [x] R1.1 (NIT) render.rs:495 - `spawner_transform.up().normalize()` is
  redundant: `up()` already returns a unit `Dir3`. Harmless and it mirrors the
  turret's identical `normal.normalize()`, so leaving it for parity is fine;
  dropping `.normalize()` is equally fine.
  - Response: Fixed. Dropped `.normalize()`, now `let normal =
    spawner_transform.up();` with `Vec3::from(normal)` for the property set.
    Verified confirmed: value is identical to the prior normalized unit vector.
- [x] R1.2 (NIT) render.rs:499 - `base_velocity` is always set to `Vec3::ZERO`
  and the effect adds it into every particle velocity, so it is currently
  vestigial. This is deliberate parity with the turret (which also sets it to
  zero) and leaves a hook for later inheriting ship motion. Fine to keep; worth
  a one-word "(currently always zero)" note if you want to signal intent.
  - Response: Added a comment at the `set("base_velocity", ...)` call noting it
    is currently always zero and is a hook for later riding ship motion.
    Verified confirmed.
- [x] R1.3 (NIT) mod.rs:76 / render.rs - a caller-supplied `launch_effect`
  handle must itself declare the `normal` and `base_velocity` properties, or the
  `properties.set(...)` calls in `on_torpedo_launch_effect` become no-ops (hanabi
  logs and ignores unknown properties). Same coupling exists in the turret's
  `muzzle_effect`, and `launch_effect` is `None` everywhere today, so there is no
  live bug. Consider a doc line on the field noting the required properties for a
  custom effect.
  - Response: Added the property contract (spawn-on-command; `normal` /
    `base_velocity` `Vec3` properties) to the `launch_effect` field doc.
    Verified confirmed; `cargo check -p nova_gameplay` clean after the change.
