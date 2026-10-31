# Stop a neutralized drifting ship with its own RCS when capable

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.17.0

## User facts
- Neutralized ship should be able to stop with RCS so it can be approached; no auto-dock. Specification only.
## Agent findings
- `AutopilotAction::Stop` exists, with player RCS settle (`crates/nova_ship/src/flight/state.rs:320-328`; `crates/nova_ship/src/flight/autopilot.rs:528,819`). `rcs_burn_system` already checks ship capability and shared budget (`crates/nova_ship/src/flight/manual.rs:245-380`). A neutralized target may dock (`crates/nova_ship/src/sections/docking_section/tests.rs:173,1820`).
## Decisions
- Trigger: `NeutralizedMarker` on a non-player, non-station ship root. The brake owns the helm: patrol, follow, combat flight and orders stop; an order fails. Player STOP and controls do not change.
- Brake: world rest on the hull's own finite `RcsBudget`, no engine needed. It needs a live flight computer, `rcs_enabled`, delta-v left, no dock and a well the RCS out-pushes (`RCS_GRAVITY_AUTHORITY`).
- The first missing need ends the brake for good as `NeutralizedBrakeType` (`NoFlightComputer`, `RcsDisabled`, `RcsEmpty`, `Docked`, `Gravity`), logged once; the hull coasts. A refill does not restart it.
- Persistence (owner, option A): `FrozenShip` owns RCS spent/idle and the brake state on every ship. `SavedRcs`/`SavedControls.rcs` are deleted. Save and Load refuse non-finite, negative or overspent RCS and an unloaded design.
- A thaw catches up an idle magazine by `ThawedAfter` play time; a magazine frozen mid-command (`idle == 0`) gets nothing.
- Save version stays `v0.16.0` (RELEASE.md policy). World saves from earlier development builds lack the new `FrozenShip` fields and are refused at Load. Released v0.16 saves are not promised to load.
## Delivery
- End state: a reproducible capable neutralized target demonstrably brakes using its own finite RCS before a player manual approach; an incapable one does not fake it. Dies: superseded neutralized drift behavior only if chosen. May break: physics, damage/neutralization order, AI/autopilot control, budget, salvage and docking eligibility. Owner/entry/order: neutralization event/AI -> `nova_ship::flight` STOP or approved command -> `rcs_burn_system` before physics (`crates/nova_ship/src/flight/mod.rs:194-238`) -> velocity/docking HUD. Audit save/thaw, capability, content and wiki; report unable-to-stop visibly, invalid authored capability at lint/load.
## Verification / Done when
- First preserve drifting-target reproduction; asserted scenario measures velocity and spent reserve on a real neutralized ship including empty/capability-loss paths, then manual `nova-bench` dock. Do not infer success from exit zero; approve any new permanent test first.
## Implementation
- Brake: `crates/nova_ship/src/flight/neutralized_brake.rs` (`on_neutralized_engage_brake`, `neutralized_brake_system`, chained after `station_keeping_system`, before `rcs_burn_system`). `RcsBudget::recover` is exact for any `dt`.
- Authority: `drive_ship_orders` fails a neutralized hull's order; `update_passive_flight`, `update_follow_flight` and `update_combat_flight` skip `NeutralizedMarker`.
- Persistence: `FrozenRcs` and `brake` in `FrozenShipState` (`crates/nova_scenario/src/objects/spaceship.rs`); `check_saved_rcs` in `crates/nova_world_base/src/save/mod.rs`; `recover_thawed_rcs_budgets` in `crates/nova_world_base/src/rcs_reserve.rs`. `open_world`/`check_world` take `&GameShipDesigns`.
- Limits: the reason is a log line and a reflected component, no HUD text. Every end is terminal, `Rested` too: a later bump (the docking player's hull in the bench play) sets the hull drifting and the brake does not restart; an undock does not restart it. A resumed interrupted order reports `Resumed` then `Failed`.
- A refit that deletes a section holds RCS spent at the new capacity (`a_refit_remove_under_the_spent_delta_v_holds_spent_at_the_new_capacity`). Save and Load refuse a neutralized sector ship with no brake state, which a thaw would restart at `Braking`. A parked hull's magazine does not catch up while its station is off the window.
## Result
- Units: meters. Raw `RcsBudget` values are engine u/s (1 u/s = 10 m/s); a hull of 65 sections or fewer holds a 300 m/s magazine.
- Before/after (disposable fixture; `/tmp/nova-85/before/run2/summary.txt` vs `/tmp/nova-86/after/REPORT.md:55-99`): before, neutralized patrol ships drift at 94-153 m/s with 0 RCS spent, three still on their patrol GOTO. After, they rest at 0.18-0.60 m/s on 50-157 m/s of their own RCS; the computerless one coasts at 150.8 m/s as `NoFlightComputer`, 0 spent. A thaw keeps 30.66 m/s spent (before: reset to 0).
- Mid-brake freeze (`/tmp/nova-87/after/REPORT.md:66-71`): a ship frozen at `Braking` thaws `Braking` with spent exact (30.66 m/s) and rests at 0.000 m/s on 90 m/s of RCS from 89.2 m/s. A `Rested` ship re-frozen stays `Rested` with spent unchanged (150.8 m/s). Its `FrozenShip` RON round trip is bit-exact.
- Live save/load (open world, real Load path; `/tmp/nova-87/e2e/run.log:223,287-288`): a neutralized NPC saves and loads at 150.00 m/s spent and `Rested`; the brake does not restart.
- Catch-up (real sector retire and thaw; `/tmp/nova-87/e2e/run.log:143,182-183`): an idle NPC frozen at 250 of 300 m/s spent thaws after 122.7 s of play at 0, which equals `RcsBudget::recover` for that time. The proof hits the zero clamp: a lawful 300 m/s magazine refills within 5 s of idle (2 s delay, then 100 m/s^2), and the round trip takes more than 90 s under lavapipe. A partial catch-up is proved only by `an_idle_thawed_magazine_catches_up_and_a_held_command_gets_nothing`. An earlier run seeded an overspent magazine; live Save refused it (`run-savefail.log:151`), so that run is not evidence of catch-up.
- Bench dock (`/tmp/nova-87/bench/REPORT.md`, play1, seed 8787): the drifter is neutralized while it cruises at 40 m/s (`game.log:20`), brakes to `Rested` at 0.09 m/s with 147.5 m/s of RCS left (`game.log:25`), holds rest until the player's approach, and the player docks (`score.json` `docked: true`). The player's hull pushes it to about 6.8 m/s before the dock.
- Checks (`/tmp/nova-87/checks/checks.log`): fmt; `cargo check` of the five changed crates with tests and the example subset; focused lib tests in `nova_ship` (11 + 57), `nova_scenario` (8), `nova_world_base` (57), `nova_menu` (22) and `nova_core --test world_resume_refusal` (1), all pass.
- Release checks (`/tmp/nova-88/REPORT.md`): `git diff --check`, `cargo fmt --all --check`, `mdbook build`, web `npm run ci` and wasm32 Clippy `-D warnings` on the five changed crates with `CLIPPY_CONF_DIR=ci/wasm-clippy`, all pass.
## Limitations and skipped proof
- `system_world_resume` probe: FAIL, skipped as stale (`/tmp/nova-87/checks/probe_world_resume/2c8157f74/system_world_resume/run.log:100-101`). It panics at `examples/systems/system_world_resume.rs:1187`, "the player ship has a torpedo bay", before any RCS step: its `bounty_hunter` start hull is the interceptor with two PDC mounts and no torpedo bay (`crates/nova_authoring/src/base_content/start_roles.rs:88-92`). This task changes one argument line in the example. The fixture needs its own fix.
- Bench replay: `replay_mismatch` from tick 91-406 (`/tmp/nova-87/bench/replay1`); the drifter is never defeated in the replay. It reproduces nothing. In the same ticks the player's hull lost sections and plates (health 1420 -> 1126.6) while the unarmed drifter was more than 900 m away. Not diagnosed; no claim rests on it.
- The player ship's RCS through a real Load has no live proof; `a_saved_ships_rcs_magazine_overspent_negative_non_finite_or_brakeless_is_refused_on_open` covers the record round trip and refusals.
- Each live proof is one run.
