# RCS: refill while Shift is held, mouse cancels Shift+X

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.19.0, flight, input

## User facts
- Owner 2026-11-22: "very big annoyance : if I hold Shift and not move
  mouse (aka not using RCS) the RCS does not 'reload' it should because
  otherwise it's very very annoying and frustrating to use; also when in
  'Shift+X' stop mode we should allow the play to cancel it by moving the
  mouse AKA apply manual RCS, so basically if I press Shift+X and then move
  the mouse it overrides it back to manual RCS".

## Agent findings
- `RcsBudget::recover` (`crates/nova_ship/src/flight/state.rs:256-268`)
  runs only when `RcsIntent == Vec3::ZERO`; holding Shift with a still
  mouse does not reach it.

## Delivery
- RCS refills while Shift is held and no RCS input is applied.
- Mouse input (manual RCS) cancels the Shift+X stop and returns manual RCS.
- Build running (2026-11-22).

## Verification
- Unit or ECS asserts for both rules; a playable example walk.

## Done when
- Both rules hold in play and the owner has played them.
