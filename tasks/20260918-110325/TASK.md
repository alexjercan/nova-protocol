# Remove the manual flight speed governor

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.14.0, gameplay, flight, breaking

## User facts

- Manual flight must be Newtonian: ships have no absolute speed cap, engines limit acceleration, releasing thrust preserves velocity, and turning alone does not change velocity.
- The current cap makes the ship hard to control because the main drive stops answering near the velocity ceiling.
- Players can use the existing `STOP` order. Do not add stopping-distance telemetry, a stopping marker, or another HUD instrument.
- Keep the existing input and throttle behavior.
- Leave RCS unchanged. Reconsider it only with a future fuel design.
- Realistic finite-motor torpedoes are approved as a direction but are a separate task. Do not change torpedoes here.

## Decisions

- Delete the manual speed governor end to end. Do not retain a compatibility component, ignored field, alias, fallback, replacement clamp, or renamed governor.
- `manual_burn_system` always delivers the clamped `FlightIntent::burn` demand through the existing balanced physical-thruster path. Current velocity does not gate main-drive throttle.
- Remove the player-facing rated-speed display. The speed chip shows current speed only.
- Remove the scenario object field, scenario action, console command, lessons, creator documentation, and generated/authored content for the governor.
- This is a shipped content-format break. Removed `speed_cap` and `SetSpeedCap` syntax must fail loudly as unknown content rather than load as a no-op.
- Do not rewrite historical release news. It records behavior shipped by old releases.

## Agent findings

- `FlightSpeedCap` is owned by `crates/nova_ship/src/flight/state.rs:42-53`.
- `manual_burn_system` reads it in `crates/nova_ship/src/flight/manual.rs`; `speed_budget_scale` suppresses any burn that would grow total velocity. At the cap, a burn across the velocity vector is therefore reduced to zero. This is the reported control failure.
- Scenario authoring exposes `PlayerControllerConfig::speed_cap` in `crates/nova_scenario/src/objects/spaceship.rs:79` and inserts the component near line 695.
- Runtime mutation is exposed as `SetSpeedCapActionConfig` in `crates/nova_scenario/src/actions/ship.rs` and `EventActionConfig::SetSpeedCap` in `crates/nova_scenario/src/actions/mod.rs`.
- The console inspects and mutates the cap in `crates/nova_console/src/inspect.rs` and `crates/nova_console/src/cheats.rs`.
- The HUD prints `current / rated` in `crates/nova_hud/src/flight_status.rs:392-419`; `nova_ui::units::speed_rated` exists for that surface.
- Probe uses the cap only as a 10x absurd-speed bound in `crates/nova_probe/src/capabilities/invariants.rs`. Finite velocity remains the valid invariant after removal.
- Base builders author 150 m/s for Basic Training and its drills in `crates/nova_authoring/src/base_content/scenarios/tutorial/range.rs`, and 120 m/s for season one in `crates/nova_authoring/src/base_content/scenarios/season_one/stage.rs`.
- Authored non-base consumers include Gauntlet and the Ledger campaign. Many examples and bench fixtures explicitly serialize `speed_cap: None`; all consumers must be migrated.
- The current cap began as a training-range containment guard. It is now persistent across the main shipped player flows. Removing it can expose objective overshoot, obstacle-lane bypass, high-speed collision, and current capped-torpedo balance assumptions. Observe these; do not restore a governor to hide them.

## Delivery

1. Preserve a before artifact that demonstrates the current failure: a ship at the cap, moving across its nose, receives no meaningful main-drive delta-v under a held burn.
2. Change the owning flight interface first:
   - delete `FlightSpeedCap` and its exports/reflection registration;
   - remove `Option<&FlightSpeedCap>` and the cap branch from `manual_burn_system`;
   - delete main-drive-only speed-budget helpers and cap-specific tests;
   - retain the balanced thruster allocator, spool behavior, braking behavior, and high-speed no-spin proofs.
3. Remove the scenario format and runtime action:
   - delete `PlayerControllerConfig::speed_cap` and spawn insertion;
   - delete `SetSpeedCapActionConfig`, `EventActionConfig::SetSpeedCap`, action tags, lint handling, and focused tests;
   - require removed syntax to fail load/lint.
4. Remove console and HUD surfaces:
   - delete the `speed-cap` command, help/dispatch wiring, inspection output, and tests;
   - make the speed chip read current speed only;
   - delete `speed_rated` if no consumer remains.
5. Remove the cap-derived probe invariant while preserving finite-velocity checks and their evidence fields.
6. Edit Rust base-content builders, then regenerate base RON. Never hand-edit generated `assets/base/**/*.content.ron`.
7. Remove `speed_cap` from every loose benchmark, example, bundled-mod, and webmod RON consumer. Run a repository-wide search after generation; only historical task/release records may still name the old mechanic.
8. Remove the `flight_speedcap` lesson and update lesson ordering/coverage and its screenshot producer as required. Do not replace it with a stopping-distance lesson.
9. Update current documentation:
   - `web/src/wiki/flight-autopilot.md`
   - `web/src/wiki/glossary.md`
   - `web/src/wiki/getting-started.md`
   - `web/src/wiki/commands.md`
   - `web/src/create/objects.md`
   - `web/src/create/actions.md`
   - `web/src/create/reference.md`
   - affected Rust API documentation and code-backed widgets
   - `CHANGELOG.md` with one concise `**(breaking)**` entry based on v0.13.2.
10. Do not change RCS, `arrival_speed_limit`, GOTO/STOP planning, torpedo propulsion, torpedo balance, or historical `web/src/news/*` posts.

## Before and after call graph

```text
Before
Scenario speed_cap / SetSpeedCap / console speed-cap
  -> FlightSpeedCap
  -> manual_burn_system
  -> speed_budget_scale
  -> reduced or zero balanced thruster demand

After
FlightBurnInput.value
  -> FlightIntent::burn
  -> manual_burn_system
  -> balance_throttles
  -> spool_allocated_thrusters
  -> physical acceleration from live thrust and mass
```

## Verification

- Unit/App proof: a ship carrying cross velocity receives main-drive acceleration along its nose; the centered drive adds no artificial spin; releasing burn preserves the resulting velocity.
- Existing behavior proof: `STOP` still converges to rest and GOTO remains independent of the deleted manual governor.
- Content proof: regenerate and lint base content; load affected bundled mods/fixtures; assert removed syntax is rejected, not ignored.
- Player-flow proof: run Basic Training and `system_chapter_one` through their real outcomes. Judge objectives, final state, assertions, logs, and frames, not exit zero alone.
- Control-feel proof: use `nova-bench` for a matched uncapped accelerate, turn, cross-burn, and STOP flow. Record whether the drive remains responsive and whether mission geometry now fails.
- HUD proof: inspect a rendered frame and confirm the speed chip shows current speed without a rated denominator. Headless output cannot prove this visual claim.
- Run affected checks only through `nix develop --command ...`; include content generation/lint, focused crate tests, the two affected scenario flows, formatting, and relevant web/docs checks. Do not run workspace tests or Clippy unless requested.

## Done when

- No runtime system limits manual main-drive thrust by ship speed.
- The main drive remains responsive at every finite velocity.
- No live code, current content, command, HUD, lesson, or current documentation exposes a manual ship speed cap.
- Old authoring syntax fails loudly.
- Basic Training and season one complete without the governor.
- `STOP`, GOTO, RCS, and torpedoes retain their existing behavior.
- Generated content, affected tests, probes, docs, and rendered HUD evidence are reviewed and recorded with the task.
