# Review: weapon auto-reload/regen mechanic

- TASK: 20260717-085640
- BRANCH: feature/ammo-reload

## Round 1

- VERDICT: APPROVE

Verified independently (implementer == reviewer, so load-bearing claims were
re-derived, not read):

- **Add-only ordering claim holds.** `tick_section_reload` and both
  `shoot_spawn_projectile` systems all take `&mut SectionAmmo`, so Bevy cannot
  parallelize them - they serialize each `FixedUpdate` tick. Reload only grows
  `rounds` (`(rounds + n).min(capacity)`), fire only shrinks it (`try_consume`,
  guarded at 0). The final count is therefore order-independent; the only
  cross-effect is which tick a discrete reload begins, a sub-frame timing nuance,
  not a miscount. The "no ordering needed" comment is correct.
- **Unlimited / infinite_ammo never reloads.** `SectionReload` is inserted only
  inside the existing `if let Some(capacity) = config.ammo_capacity` block at both
  build sites (turret_section.rs, torpedo_section/mod.rs); `infinite_ammo` sets
  `ammo_capacity = None`, so no `SectionAmmo` and no `SectionReload`. Invariant
  intact.
- **Refill logic.** `advance()` loops so a long `dt` completes multiple cycles
  exactly; clamps at capacity; holds `elapsed = 0` at rest. `is_reloading()`
  gates discrete (only-when-empty) vs continuous correctly. `progress()` is
  clamped and zero at rest - a clean single source for the readout task.
- **Catalog/RON parity + reflection.** `content_ron_parity` and
  `content_lint_gate` green, so `sections.rs` and `base.content.ron` agree on the
  new `reload` fields. `SectionReloadConfig` derives `Reflect` but neither config
  type is `register_type`'d, so nothing walks it at runtime - no
  unregistered-nested-type panic risk. Only the `SectionReload` component is
  registered, which is correct.
- **Shakedown flip.** `infinite_ammo: false` in both the RON and the Rust builder;
  the guard test is inverted to pin finite ammo (`the_new_game_player_has_finite_reloading_ammo`),
  so the flag cannot silently flip back.
- **Tests are meaningful and can fail.** The two integration tests are true A/B
  partners of existing "caps at magazine, forever" no-reload tests - deleting the
  reload wiring makes them fail. `cargo fmt --check` clean; `sections::ammo` 9/9;
  both integration tests green; content + shakedown tests green. Full suite in CI
  (per the repo's standing skip-local-full-suite instruction).

- [x] R1.1 (NIT) crates/nova_gameplay/src/sections/ammo.rs:161 - a misconfigured
  `reload_time <= 0` is a silent no-op: `advance()`'s `while` guard
  (`reload_time > 0.0`) means the magazine never refills while `is_reloading()`
  stays true, so `elapsed` accumulates unbounded (harmless - `progress()` is
  guarded to 0, no crash). Optional: `debug_assert!(reload_time > 0.0)` in
  `from_config`, or a one-line doc note that `reload_time` must be positive.
  Authored content always sets it positive, so this is defensive only.
  - Response: fixed - added a `debug_assert!(reload_time > 0.0)` in `from_config`
    and a doc note on the field that a non-positive value never refills.
