# Stop a neutralized drifting ship with its own RCS when capable

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.17.0

## User facts
- Neutralized ship should be able to stop with RCS so it can be approached; no auto-dock. Specification only.
## Agent findings
- `AutopilotAction::Stop` exists, with player RCS settle (`crates/nova_ship/src/flight/state.rs:320-328`; `crates/nova_ship/src/flight/autopilot.rs:528,819`). `rcs_burn_system` already checks ship capability and shared budget (`crates/nova_ship/src/flight/manual.rs:245-380`). A neutralized target may dock (`crates/nova_ship/src/sections/docking_section/tests.rs:173,1820`); current evidence does not show a reproduced unbraked neutralization.
## Decisions needed before implementation
- Trigger/ownership (AI, player order or autonomous), neutralized capability after section loss, when it stops, target reference velocity/gravity, budget depletion, despawn/freeze continuity and explicit failure when braking cannot complete. Do not assume unlimited RCS or a new default stop rule.
## Delivery
- End state: a reproducible capable neutralized target demonstrably brakes using its own finite RCS before a player manual approach; an incapable one does not fake it. Dies: superseded neutralized drift behavior only if chosen. May break: physics, damage/neutralization order, AI/autopilot control, budget, salvage and docking eligibility. Owner/entry/order: neutralization event/AI -> `nova_ship::flight` STOP or approved command -> `rcs_burn_system` before physics (`crates/nova_ship/src/flight/mod.rs:194-238`) -> velocity/docking HUD. Audit save/thaw, capability, content and wiki; report unable-to-stop visibly, invalid authored capability at lint/load.
## Verification / Done when
- First preserve drifting-target reproduction; asserted scenario measures velocity and spent reserve on a real neutralized ship including empty/capability-loss paths, then manual `nova-bench` dock. Do not infer success from exit zero; approve any new permanent test first.
