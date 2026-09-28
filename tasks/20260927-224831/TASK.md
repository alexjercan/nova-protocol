# Replace RCS speed cap with recoverable duty budget and HUD gauge

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.15.0, flight, rcs

## User facts

- TOP priority: RCS must be able to accelerate at any ship speed, especially to match a fast docking target. Remove the current speed cap as a limit on acceleration.
- Replace it with a cooldown or draining/recovering budget similar in feel to PDC rounds. Show the available RCS budget in a fuel-style HUD element like the PDC ammo display.
- For now use a self-recovering cooldown, not inventory fuel, consumable cargo, or a propulsion economy. Preserve independent main-drive behavior.

## Agent findings

- Before: `crates/nova_ship/src/flight/manual.rs` spent a vector speed budget on RCS against an optional reference, with a 100 m/s default cap in `flight/state.rs`. Braking and turning still acted above the cap. Reproduced in `proof/rcs-repro-before.txt`: a 300 m/s hull gained 0.00 m/s from 1 s of forward RCS, and a hull chasing 150 m/s stalled at 100.0 m/s.

## Decisions

- Owner: every ship root has a required `RcsBudget { spent, idle, applied }`. A docked driver pays once for the pair; the partner pays nothing.
- Budget: 300 m/s of delivered delta-v (`FlightSettings::rcs_budget`). Only delivered delta-v is charged; a refused push (no capability, missing assembly, bad mass) costs nothing.
- Recovery: 2 s after the last command (`rcs_recovery_delay`), refill at 100 m/s/s (`rcs_recovery_rate`). A held command on an empty magazine keeps it empty.
- No speed cap: `RcsSpeedCap`, `RcsReference`, and `budgeted_rcs_delta_v` are deleted. Main drive is unchanged.
- Autopilot: STOP/GOTO settle hands to RCS when `|v| < rcs_handoff_speed` (100 m/s); ORBIT and velocity-hold trim use the velocity error. Both need delta-v in the magazine, else the main drive keeps the goal.
- HUD: ten violet pips 24 px under the speed chip, one per tenth left; spent pips dim. Shown while commanded, pushing, or not full; hidden when full and idle.
- Audio: the RCS hiss follows `RcsBudget::applied` (delivered thrust), not the command.

## Delivery

- Reproduce failure to match a fast moving ship and preserve before/after artifacts.
- Remove obsolete cap/taper paths and update callers, tests, content, HUD, and documentation as needed. No compatibility alias for unshipped settings.
- Prove that RCS accelerates at speeds above the old cap, drains during sustained use, recovers while idle, has consistent player/autopilot/docked behavior, and displays its live budget; inspect rendered HUD and a real docking approach.

## Verification

- Before: `proof/rcs-repro-before.txt` (disposable test at fb744c880, removed).
- Unit: `cargo test -p nova_ship --lib -- flight:: docking ship_audio` 236 passed; `cargo test -p nova_hud --lib` 294 passed; `cargo check --workspace` clean; web `widgets.test.ts` ran; Prettier clean on changed web files.
- Rendered fixture (lavapipe, disposable): tender at 100 m/s matched a 250 m/s target on RCS alone (252.5 m/s, 49% left); pips drained, refilled after the delay, and hid when full. Frames inspected.

## Limitations

- Tutorial RCS leg and Season 1 docking feel with the magazine are not playtested. The fixture matched speed but did not dock; a real docking approach is not inspected.
- Full workspace tests and Clippy not run locally.
- Fixture source, log, and frames stay local and untracked for owner review.
- STATUS stays OPEN until owner playtest and merge of PR #85.

## Done when

- A reviewed design and proof exist; the cap no longer prevents speed matching, the budget limits sustained RCS use with readable HUD feedback, and affected checks and rendered inspection pass.
