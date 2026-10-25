# Give manual ship roll physical RCS control

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.17.0

## User facts
- Pilot can roll ship via RCS; retain skill and physical ship behavior. This is not a requirement for dock capture. Specification only.
## Agent findings
- RCS currently applies a linear impulse through COM and never rotates hull (`crates/nova_ship/src/flight/manual.rs:210-245,403-429`); controller copies rotation command after flight (`crates/nova_ship/src/flight/mod.rs:170-238`; `crates/nova_ship/src/sections/controller_section.rs:467-481`). Existing RCS binding is Shift + mouse/scroll or stick (`crates/nova_ship/src/input/bindings.rs:88-105`). Dock capture ignores roll (`crates/nova_ship/src/sections/docking_section/tests.rs:273`); `web/src/wiki/sections/docking.md:50` falsely says roll alignment required.
## Decisions needed
- Pilot binding/controller arbitration, roll axis and authority, RCS angular budget accounting, capability/empty state, concurrent translation, camera/HUD cues and what ship sections actuate it. No new torque, bind, default or auto-align approved.
## Delivery
- End: pilot issues and visibly observes controlled physical roll with finite RCS, no hidden capture gate. Dies: stale 'RCS never rotates' and false docking wiki text after implementation. May break: controller PD, aim/camera, other input bindings, reserve and AI order precedence. Invalid capability/binding authoring fails lint/load; exhausted RCS cannot fake torque.
- Owner/entry/state/order: `nova_ship::input` -> flight rotation authority/RCS spend -> controller sync -> physics; HUD and keybinds. Audit every RCS caller, docs/changelog and docking-facing help; keep manual docking intact.
## Verification / Done when
- Assert rotation and correct reserve spending, translation overlap and exhaustion without changing docking capture; `nova-bench` pilot roll with inspected frames and RCS feedback. Approve new permanent tests before writing.
