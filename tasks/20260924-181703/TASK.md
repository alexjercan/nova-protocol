# Add docked-assembly helm control toggle

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.15.0, gameplay, docking

## User facts

- Docking starts in a **neutral** mode. The player has no thrust or rotation authority over the docked pair; the other ship or station may continue to act.
- A dedicated key toggles **Take control** and **Relinquish control** while docked. Taking control gives the player thrust and rotation authority over the coupled assembly. Relinquishing returns to neutral without undocking.
- While the player holds control, the other ship must not fight the player's movement or steering. For the initial player-owned interaction, the request to take control is accepted; no negotiation UI is required.
- Keep the two ships coupled. Do not treat relinquishing control as a request to release the docking joint.

## Agent findings (before implementation)

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
- The owner-approved implementation merged in PR #76 as `a4b856654`. The example proof was completed on master after the merge. H toggles helm authority; D still undocks.

## Verification / done when

- An asserted ECS example docks two movable ships: neutral starts with no player authority while the partner can act; take control lets the player steer the connected assembly without partner thrust/torque; relinquish returns control without joint teardown; undock still releases.
- Cover toggles, partner maneuver conflicts, damaged/removed port and cleanup, no stale helm on release, and the chosen HUD/key behavior. Inspect a rendered player flow if a visible control indicator is claimed.

## Closure evidence, 2026-09-25

- `examples/systems/system_docking_ports.rs` now asserts a neutral pair with a
  player and two movable driven hulls; the neutral partner first drives both
  roots. H's request takes the helm without releasing the joint, both roots
  move on the player's burn, and the partner's competing throttle is zero.
  Relinquishing returns to neutral, makes the player's held burn inert, and
  keeps the joint; existing stages still assert undock, destroyed-port cleanup,
  and scenario dock/undock events. The example triggers `DockingHelmRequest`
  directly; it does not synthesize a keyboard press.
  `crates/nova_probe_cli/tests/catalog_drift.rs` pins the three new outcomes. Input/key behavior is separately covered by
  `crates/nova_ship/src/input/player/flight_rig.rs` tests.
- `nix develop --command cargo run --features dev probe run
  system_docking_ports --correctness-only --out
  /tmp/task181703-helm-complete` passed six applicable checks and emitted
  all fourteen outcomes (thirteen asserted, one recorded geometry); 0 invariant
  violations and 0 log offences. The marker at frame 176 records both hulls
  moving at 8.81 m/s under the neutral partner's burn. At frame 236 both hulls
  move at 8.14 m/s with the player's throttle near 1, the partner's at 0, and
  one joint. At frame 296 the pair remains jointed and near rest after handing
  the helm back. The two capture
  and frame-rate checks were not applicable, not passes.
- The existing `crates/nova_ship/src/sections/docking_section/tests.rs`
  covers helm changes, partner movement/order suppression, port destruction,
  missing player, and measurement-fault refusal/recovery. The earlier PR #76
  CI run passed all seven reported checks; after this proof-only change,
  focused `catalog_drift` tests, example Clippy, formatting and diff checks
  passed. No workspace-wide rerun was made.
- A recorded rendered bench run on the merged implementation used
  `crates/nova_bench/scenarios/docking.content.ron` with seed 1. Its audit at
  ticks 3386, 3391 and 3406 shows neutral -> held -> neutral and a still-live
  dock; inspected frames `/tmp/task181703-bench/f{3384,3390,3400,3405}.png`
  show matching NEUTRAL, H TAKE/RELEASE HELM, and disabled/enabled STOP/RCS
  hints. HELM FAULT was asserted in HUD/flight unit tests but was not
  rendered in this fixture. The bench's partner has no controller; the
  asserted example and unit tests, not this capture, prove suppression.
  Artifacts are local to this host and not committed.
