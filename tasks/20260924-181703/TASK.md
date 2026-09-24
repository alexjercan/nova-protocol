# Add docked-assembly helm control toggle

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.15.0,gameplay,docking

## User facts

- Docking starts in a **neutral** mode. The player has no thrust or rotation authority over the docked pair; the other ship or station may continue to act.
- A dedicated key toggles **Take control** and **Relinquish control** while docked. Taking control gives the player thrust and rotation authority over the coupled assembly. Relinquishing returns to neutral without undocking.
- While the player holds control, the other ship must not fight the player's movement or steering. For the initial player-owned interaction, the request to take control is accepted; no negotiation UI is required.
- Keep the two ships coupled. Do not treat relinquishing control as a request to release the docking joint.

## Agent findings (not approved implementation)

- Docking currently creates one `FixedJoint` and tags **both** roots with `DockedShip`: `crates/nova_ship/src/sections/docking_section/connection.rs:121-181`. It describes docking as modal with neither hull in charge (`:9-21,41-46`).
- `DockedShip` excludes manual drive and RCS (`crates/nova_ship/src/flight/manual.rs:114-120,247-250`); the section thrust pass and attitude control also stop on docked roots (`crates/nova_ship/src/sections/thruster_section.rs:584-592`, `crates/nova_ship/src/sections/docking_section/mod.rs:10-24`). An engaged autopilot currently **breaks the joint** (`connection.rs:261-293`).
- The existing <kbd>D</kbd> action releases the joint when docked (`crates/nova_ship/src/input/player/flight_rig.rs:530-568`). Assigning the new toggle to D would change undocking semantics. Wiki docking and keybind pages currently promise the modal behavior (`web/src/wiki/sections/docking.md`, `web/src/wiki/keybinds.md`).

## Decisions needed before implementation

1. Name the control states and choose a key/action distinct from undock; specify whether neutral permits the partner's autopilot and AI to move the coupled pair while the player's helm stays inactive.
2. Define exactly what gets suppressed on the other hull while the player has control: propulsion/attitude only, or also weapons, targeting, and non-movement actions. Define what happens with an already active autopilot, especially because it currently releases the joint.
3. Define effective mass, thrust, steering and joint stress for a controlled pair (especially a massive or immobile station). State whether docked ship + station and ship + ship differ, and whether either endpoint may take control.
4. Define how control is lost on undock, damage, endpoint death, scenario unload, player change, or conflicting commands. Decide how the UI indicates the active helm and refusals.

## Delivery

- Replace the all-helms-disabled dock rule with explicit docked control state and one authority owner, while keeping the existing fixed-joint capture/release lifetime.
- Route player input, AI/autopilot intent, thrust, RCS, and attitude through that owner; do not leave a competing controller applying forces to the joint.
- Update docking controls, HUD hints, player wiki and examples when the interface is approved. Delete stale modal/no-authority claims rather than retaining aliases.
- No implementation is authorized by this task record. Scout a code-backed design and stop for the decisions above before edits.

## Verification / done when

- An asserted ECS example docks two movable ships: neutral starts with no player authority while the partner can act; take control lets the player steer the connected assembly without partner thrust/torque; relinquish returns control without joint teardown; undock still releases.
- Cover toggles, partner maneuver conflicts, damaged/removed port and cleanup, no stale helm on release, and the chosen HUD/key behavior. Inspect a rendered player flow if a visible control indicator is claimed.
