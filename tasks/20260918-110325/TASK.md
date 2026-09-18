# Remove the manual flight speed governor

- STATUS: CLOSED
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

## Implementation plan (proposed 2026-09-18)

Read against the tree at `cf35fe2a7`. Line numbers are current.

### Corrections to the Delivery list

- **No `flight_speedcap` lesson exists.** `crates/nova_authoring/src/base_content/lessons.rs`
  has `flight_aim`, `flight_momentum`, `flight_stop`, `flight_cancel`,
  `flight_rcs`, `flight_gravity`, `flight_goto`, `flight_arrival`,
  `flight_orbit`, `flight_dock` and no cap lesson. No lesson text names the
  cap. Step 8 has no work in it; the doc obligation is the wiki prose instead.
- **Nothing authors `SetSpeedCap`.** No file under `assets/`, `webmods/` or
  `crates/nova_authoring/src` uses the action. Its only consumers are the
  editor palette, the docs, and three action-table tests that use it as a
  generic "buried injection" fixture.
- **Removing the field is NOT loud on its own.** `PlayerControllerConfig`
  (`crates/nova_scenario/src/objects/spaceship.rs:61`) carries no
  `deny_unknown_fields`, so serde would silently ignore a leftover
  `speed_cap:` key. Removing the `SetSpeedCap` enum variant IS loud (unknown
  RON variant). See decision D1.
- **Action count.** The table has 52 entries (`actions/mod.rs`); after removal
  51. `web/src/create/reference.md:46` says 52 and must become 51.
  `web/src/docs-manifest.js:753` already claims 51 and becomes correct.

### Owning interface: `nova_ship`

| Path:line | Now | Change |
| --- | --- | --- |
| `crates/nova_ship/src/flight/state.rs:42-53` | `FlightSpeedCap(pub f32)` + docs | delete |
| `crates/nova_ship/src/flight/state.rs:110-112` | `RcsSpeedCap` doc cites `FlightSpeedCap` | reword, keep RCS |
| `crates/nova_ship/src/flight/mod.rs:68,95,136` | re-export x2 + `register_type` | delete all three |
| `crates/nova_ship/src/flight/mod.rs:90` | prelude doc "speed caps" | reword |
| `crates/nova_ship/src/flight/manual.rs:38` | `SPEED_CAP_TAPER_FRACTION` | keep (RCS reads it), re-doc |
| `crates/nova_ship/src/flight/manual.rs:41-82` | `speed_budget_scale` | delete |
| `crates/nova_ship/src/flight/manual.rs:122-134` | `step_inside_sphere` | delete (only caller is above) |
| `crates/nova_ship/src/flight/manual.rs:84-120` | `budgeted_rcs_delta_v` | keep, drop the main-drive comparison prose |
| `crates/nova_ship/src/flight/manual.rs:150,180,228-238` | cap in the query, loop binding, taper block | delete; `Rotation`/`LinearVelocity` leave the query too |

After the edit `manual_burn_system` reads
`(Entity, &FlightIntent, Option<&ComputedCenterOfMass>, &ComputedMass)` and
goes straight from `intent.burn.clamp(0.0, 1.0)` to
`demand = burn * authority` -> `balance_throttles` -> `spool_allocated_thrusters`.
`mass`/`rotation`/`velocity` were read only by the taper, so `authority` is the
only surviving use of the allocation sums.

Dying tests (cap-specific, no other subject):

- `manual.rs` inline: `one_two_and_three_axis_pushes_reach_the_same_ceiling`,
  `a_straight_push_keeps_full_authority_below_the_band_and_tapers_inside_it`,
  `braking_keeps_full_authority_at_and_above_the_cap`,
  `a_tangential_push_cannot_carry_the_residual_past_the_budget`,
  `the_approach_to_the_cap_never_overshoots_or_backs_off`,
  `one_huge_step_lands_on_the_cap_instead_of_through_it`.
- `flight/tests/manual.rs:261` `manual_burn_levels_off_at_the_speed_cap`,
  `:540` `manual_burn_spends_one_total_speed_budget_whatever_the_heading`,
  `:582` `manual_burn_brakes_a_ship_from_above_the_cap_back_inside_it`.

Kept: `rcs_at_the_cap_turns_the_velocity_without_growing_it` (reworded), every
thruster-balance, spool, STOP and high-speed no-spin proof.

### Scenario format and action

| Path:line | Change |
| --- | --- |
| `crates/nova_scenario/src/objects/spaceship.rs:79-87` | delete `speed_cap` field |
| `crates/nova_scenario/src/objects/spaceship.rs:690-696` | delete the spawn insertion |
| `crates/nova_scenario/src/actions/ship.rs:13-57` | delete `SetSpeedCapActionConfig` and its `EventAction` impl |
| `crates/nova_scenario/src/actions/ship.rs:63,135` | doc cross-references "same rule as SetSpeedCap" -> `DespawnScenarioObject` |
| `crates/nova_scenario/src/actions/mod.rs:255-261` | delete the table row (`stem: "cap"`) |
| `crates/nova_scenario/src/actions/mod.rs:720,788,1020,1075,1083,1102` | fixture uses; see D2 |
| `crates/nova_scenario/src/lint/scenario.rs:2019` | doc comment cross-reference |
| `crates/nova_scenario/src/loader/mod.rs:836,947` | fixture field + assertion |

`ActionTag`/`ActionChoice` are the same enum (`nova_editor/src/event.rs:562`),
so the editor arm at `event.rs:694-697` falls out as a compiler error.

### Console, HUD, probe

| Path:line | Change |
| --- | --- |
| `crates/nova_os/src/commands.rs:455-467` | delete the `speed-cap` `CommandSpec` |
| `crates/nova_console/src/dispatch.rs:60` | delete the dispatch arm |
| `crates/nova_console/src/cheats.rs:139-184,299-309` | delete `speed_cap` and its test |
| `crates/nova_console/src/inspect.rs:86-89,105,383-388` | drop the `SPEED CAP` row and `speed_cap_line` |
| `crates/nova_console/src/units.rs` + `lib.rs:28` | delete the module (see D4) |
| `crates/nova_hud/src/flight_status.rs:392-419` | chip reads `nova_ui::units::speed` only |
| `crates/nova_hud/src/flight_status.rs:549-567` | delete the rated-chip test; the plain-chip test above it stays |
| `crates/nova_ui/src/units.rs:50-66` | delete `speed_rated` and its doctest |
| `crates/nova_probe/src/capabilities/invariants.rs:24-28,44,54,65-69,274,288-301` | delete the `speed_sanity` violation and `SPEED_SANITY_MULTIPLIER`; keep `velocity_finite` and `velocity_subjects` |
| `crates/nova_probe/src/capabilities/invariants.rs:508-521` | test becomes NaN-only |

### Content

- `crates/nova_authoring/src/base_content/scenarios/tutorial/range.rs:76-78,132`
  delete `TRAINER_SPEED_CAP`; regenerates `tutorial`, `drill_stop`,
  `drill_momentum`, `drill_gunnery`, `drill_autopilot` base RON.
- `crates/nova_authoring/src/base_content/scenarios/season_one/stage.rs:27-31,64`
  delete `KAVERI_SPEED_CAP`; regenerates `season_one_chapter_one`.
- Then `content -- gen`; never hand-edit `assets/base/**/*.content.ron`.
- Hand-authored RON to migrate: `assets/mods/example/example.content.ron:335`,
  `webmods/gauntlet/gauntlet.content.ron:135`, the six
  `webmods/the-ledger/ledger_0*.content.ron`, and the five
  `crates/nova_bench/scenarios/*.content.ron`.
- `crates/nova_authoring/tests/ledger_campaign.rs:249-277` see D6.
- 34 example files carry `speed_cap: None` in a `PlayerControllerConfig`
  literal (40 lines). Mechanical deletion; `cargo check --examples` is the
  sweep.
- Repository-wide `grep -rn speed_cap` after generation; only `tasks/**` and
  `web/src/news/**` may still name it.

### Docs

`web/src/wiki/flight-autopilot.md:26`, `web/src/wiki/glossary.md:17,103`,
`web/src/wiki/getting-started.md:53`, `web/src/wiki/commands.md:88,120`,
`web/src/create/objects.md:355`, `web/src/create/actions.md:29,785-800`,
`web/src/create/reference.md:46,186`, `web/src/docs-manifest.js:775`,
`docs/sections.md:376`, `docs/development.md:913`,
`crates/nova_authoring/src/base_content/sections/torpedo_bay.rs:26`
(the Lance/Serpent table is quoted against "the player's 150 m/s speed cap" -
restate the reference speed without calling it a cap), plus one
`**(breaking)**` CHANGELOG line under Gameplay & Flight based on v0.13.2.
`web/src/news/*` is history and stays.

### Decisions

**D1. How a leftover `speed_cap:` key fails.**
- (a) Add `#[serde(deny_unknown_fields)]` to `PlayerControllerConfig`.
  Precedent and the same argument already written at
  `crates/nova_scenario/src/loader/mod.rs:245-252` for `ScenarioConfig`. Also
  makes every future typo in that struct a refusal.
- (b) Leave it: a stale `speed_cap:` loads and is ignored - contradicts the
  task's "fail loudly" decision.
- **Recommend (a).**

**D2. Replacement fixture for the action-table tests.**
`SetSpeedCap` is the stand-in "buried injection" in three tests
(`nested_effect_is_the_loudest_beat`, the RON-tag sample list, and the
`walk`/`walk_mut`/`collect_injections` sweep).
- (a) `SetAllegiance` - same scoped-ship-by-id shape, same `Injection` effect,
  no new imports. **Recommend.**
- (b) `SetShipCapabilityStop` - also fine, but its config is macro-generated,
  which makes the fixture read as a test of the macro.

**D3. New permanent proofs (three).**
- (a) `flight/tests/manual.rs`:
  `manual_burn_accelerates_along_the_nose_at_any_speed` - the reproduced
  failure inverted. Ship carrying `LinearVelocity(Vec3::X * 20.0)`, nose on
  -Z, held burn: assert -Z velocity grows, assert `angular_velocity ~ 0`
  (centered drive adds no spin), then release burn and assert the velocity is
  preserved. Replaces the three deleted App tests.
- (b) `nova_scenario`: one parse test asserting both removed syntaxes are
  rejected - `Player((input_mapping: {}, speed_cap: Some(150.0)))` and
  `SetSpeedCap((id: "player"))`. This is what makes D1 observable.
- (c) Port two RCS properties off the dying helper: "one ceiling whatever the
  axis count" and "braking is free at and above the cap" are currently proven
  only through `speed_budget_scale`, which dies. Rewrite them against
  `budgeted_rcs_delta_v` rather than lose RCS coverage.
- **Recommend all three.** (c) is a rewrite of existing tests, not new
  coverage; (a) and (b) are the reproduced failure and the format break.

**D4. `nova_console::units`.** `cap_label` is the module's only function and
has exactly two callers, both deleted. Delete the module and its `mod units;`
line. Alternative: keep an empty module for the next formatter. **Recommend
deletion.**

**D5. Probe absurd-speed bound.** Delete `speed_sanity` and
`SPEED_SANITY_MULTIPLIER` outright; `velocity_finite` and the
`velocity_subjects` evidence field stay. Alternative: re-derive an absolute
bound from `FlightSettings` - a new invariant nobody asked for.
**Recommend deletion.**

**D6. `every_player_ship_uses_the_raised_speed_governor`
(`crates/nova_authoring/tests/ledger_campaign.rs:249`).** Its whole subject is
the governor.
- (a) Delete the test. **Recommend.**
- (b) Repurpose it into "every activity spawns a player ship" - weaker than
  what the file's other tests already assert.
Also add one line to `webmods/the-ledger/CHANGELOG.md` (the mod's own record
of a format migration); `webmods/the-ledger/README.md` names the governor too.

### Order of work

1. Capture the before artifact (D3a's rig, run against current code, cap
   installed, cross velocity): record zero delta-v under a held burn into
   `tasks/20260918-110325/proof/before.txt`.
2. `nova_ship`: delete the component, the helper, the branch, the tests.
3. `nova_scenario`: field, spawn insertion, action config, table row, fixtures,
   D1's attribute, D3b's test.
4. Compiler sweep: `nova_editor`, `nova_console`, `nova_os`, `nova_hud`,
   `nova_ui`, `nova_probe`, `examples/**`.
5. Builders, then `content -- gen`, then `content -- lint`.
6. Hand-authored RON: example mod, gauntlet, the-ledger, bench scenarios.
7. Docs, CHANGELOG, `docs-manifest.js`.
8. Proofs (below), then a repository-wide `speed_cap` sweep.

### Verification to run

- `nix develop --command cargo check -p nova_ship -p nova_scenario -p nova_console -p nova_os -p nova_hud -p nova_ui -p nova_probe -p nova_authoring -p nova_editor`
  and `--all-targets --keep-going` for the examples sweep.
- Focused tests only, per standing instruction (no workspace test run, no
  Clippy): the new/edited tests in `nova_ship::flight`, `nova_scenario`,
  `nova_hud`, `nova_probe`, plus `nova_authoring`'s ledger suite.
- `content -- gen` then `content -- lint`; diff the generated RON.
- Load proof: the-ledger, gauntlet and the example mod load clean; the two
  removed syntaxes are refused with the key/variant named.
- Flow proof: Basic Training and `system_chapter_one` to their real outcomes,
  judged on objectives, final state and logs.
- Control-feel proof: `nova-bench` accelerate -> turn -> cross-burn -> STOP on
  one fixture; record whether the drive answers and whether range geometry now
  overshoots.
- HUD proof: one rendered frame under Xvfb showing the chip with no
  denominator.
- `cargo fmt`, and the web checks for the edited pages.

### Blast radius and what may break loudly

- Basic Training and season one lose their containment. Objective overshoot,
  obstacle-lane bypass and high-speed collisions are the expected exposures;
  they get recorded, not patched with a new governor.
- Torpedo balance prose is quoted against a 150 m/s runner. The numbers stay
  true as a stated reference speed; the wording changes.
- Any third-party mod RON carrying `speed_cap` stops loading once D1 lands.
  That is the intended break.

## Delivered 2026-09-18

Commits: `77785cab1` before artifact, `94a96a0f2` code and content,
`30b9b8220` docs, `c50ec6f0a` flown evidence. Evidence in `proof/`.

- No runtime system limits manual thrust by speed. `manual_burn_system` reads
  `(Entity, &FlightIntent, Option<&ComputedCenterOfMass>)` and goes straight to
  the balanced thruster path. `speed_budget_scale` and `step_inside_sphere` are
  gone; RCS keeps its own budget unchanged.
- The format break is loud. `PlayerControllerConfig` is `deny_unknown_fields`:
  a stale `speed_cap:` is refused naming the key and the span, and
  `SetSpeedCap(...)` is refused as an unknown variant. Proven by
  `removed_governor_syntax_is_refused_not_ignored` and by the ledger campaign
  failing 0/9 on the stale key before migration.
- Shipped content migrated: six generated base scenarios, the example mod,
  Gauntlet (1.12.0 -> 1.13.0), the six Ledger activities, five bench fixtures.
  The two mods record the break in their own changelogs.
- Action table 52 -> 51, console catalog 27 -> 26 commands. Both counts were
  restated in the code-backed widgets and the creator reference.
- Checks: `cargo check --workspace` green; `--all-targets` clean under
  `--features debug` (the ~22 default-feature failures are the pre-existing
  `hollow.rs` debug-gate mismatch, present at `cf35fe2a7`); `content gen` diff
  is exactly the six scenarios; `content lint` 0 errors, 14 scenarios audited;
  focused tests green in nova_ship (149), nova_scenario (436), nova_hud,
  nova_probe, nova_console, nova_os, nova_ui, nova_authoring; `web npm run ci`
  and `mdbook build` pass. Workspace tests and Clippy were NOT run locally, per
  standing instruction; CI covers them.

### Exposures found, not fixed

1. **Basic Training's pattern beat overshoots.** The trainer tops out at
   210 m/s (the 600 m to the gate is the real ceiling, not a tail) and the beat
   ends at rest 1 890 m past mark ALPHA. The next card is an RCS slide to BRAVO
   and RCS caps at 100 m/s, so the authored geometry no longer matches where
   the ship stops. The card is still reachable. Evidence:
   `proof/bench-tutorial.txt`.
2. **High-speed collision is now reachable and the damage model does not
   answer it.** Twelve seconds of held throttle reaches 1 004 m/s; the hull
   strikes the planetoid at that speed and bounces with 44/44 plates and full
   health. Evidence: `proof/bench-straight-burn.txt`.

Both are consequences the task predicted and told the delivery to observe
rather than hide. Neither is a reason to restore a governor.

### Not measured

- The full Basic Training card past BRAVO (radar, gun, five hulks, two drones).
  Those beats read no ship speed.
- Obstacle-lane bypass in season one. `system_chapter_one` teleports Kaveri
  between gates rather than flying the rock lane, so nothing in the repo flies
  that lane by hand.
