# Assist relative speed and attitude while docking stays manual

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.17.0

## User facts
- Speed/rotation align assist with another ship, no auto-docking. Keep manual approach and capture decision; specification only.
## Agent findings
- `MatchVelocity` already supports held world velocity, optional facing and RCS for AI (`crates/nova_ship/src/flight/state.rs:346-380`), not a player docking contract. Starter RCS 300 m/s with 2 s idle then refill (`:158-244,667-668`). Dock envelope 10 m gap, 15 degrees facing, 5 m/s relative speed, 5 deg/s angular speed (`crates/nova_ship/src/sections/docking_section/mod.rs:117-176`). Sight exists (`crates/nova_hud/src/docking_sight.rs:210-275`); roll capture is not required (`crates/nova_ship/src/sections/docking_section/tests.rs:273`). One manual dock succeeded (`tasks/20261025-214714/TASK.md:45-47`), not ease evidence.
## Decisions needed
- Target/port selection, reference frame and axis, pilot authority/cancel, RCS expenditure and exhaustion, lost-target handling, binding and feedback. No new action, default or alignment algorithm approved. Never trigger D automatically.
## Delivery
- End: assist holds approved relative motion/attitude only while available; pilot closes and docks manually. Dies: replaced control copy only. May break: helm ownership, AI, RCS, docked assemblies and bindings. Invalid target/capability refuses visibly, no hidden snap.
- Owner/entry/state/order: input -> `nova_ship::flight` authority before controller sync and RCS spend (`crates/nova_ship/src/flight/mod.rs:170-238`) -> existing `DockingPorts` -> HUD. Audit lock/AI/action callers, save if persistent, UI and wiki/changelog.
## Verification / Done when
- Assert no auto-dock, cancel/override, empty reserve, lost target; compare manual vs assisted seeded `nova-bench`, inspect telemetry and rendered sight, report observed skill/consequences. Approve permanent test first.
