# Fix ammo-count number showing outside F11/debug mode

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.5.0,hud,bug,playtest

## Goal

Playtest (2026-07-12): the debug ammo-count number (`rounds/capacity` on the
readout) appears when NOT in debug mode; it should only show while F11/debug is
active.

Root cause: the number's visibility mirrors nova_debug's `DebugEnabled` via a
SEPARATE `AmmoReadoutDebug` flag (nova_gameplay cannot depend on nova_debug).
Both default `true` and flip on F11 - but `toggle_debug_mode` (nova_debug) is
UNGATED while `toggle_ammo_readout_debug` (nova_gameplay) is gated
`in_state(GameStates::Playing)`. An F11 press outside Playing (main menu, editor)
flips `DebugEnabled` but not the ammo mirror, so they desync: debug off, ammo
number still on.

## Steps

- [ ] Remove the `.run_if(in_state(GameStates::Playing))` gate from the
  `toggle_ammo_readout_debug` registration (ammo_readout.rs AmmoReadoutPlugin
  build) so it toggles on F11 in every state, exactly like nova_debug's ungated
  `toggle_debug_mode` - keeping the two flags in phase from their shared `true`
  default. Comment the invariant (must stay ungated to match DebugEnabled).
- [ ] Add a test that `toggle_ammo_readout_debug` flips `AmmoReadoutDebug` on an
  F11 press (guards the toggle logic).
- [ ] Verify check + the ammo_readout tests + fmt. CHANGELOG line.

## Notes

- Debug-only feature (`#[cfg(feature = "debug")]`); release builds have no number
  at all, so this only affects dev/debug builds.
- Scope: the ammo readout is the only F11 mirror in nova_gameplay (only user of
  DEBUG_TOGGLE_KEY). Not touching the global `DebugEnabled(true)` default - at
  boot both flags are true (debug on = gizmos + number), which is consistent; the
  bug is purely the desync after an out-of-Playing F11.
- Relevant: crates/nova_gameplay/src/hud/ammo_readout.rs (toggle_ammo_readout_debug
  :422, its registration in AmmoReadoutPlugin build :457-460, drive_ammo_readout_numbers
  :392); crates/nova_debug/src/lib.rs (toggle_debug_mode :68, ungated).
