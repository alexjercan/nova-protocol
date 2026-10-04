# Cargo: recover, move and trade stock

`me.cargo` shows `capacity_g`, `used_g` and `free_g` in grams, `credits`,
and `items` with debug item names and counts. These are hold values, not
ship-centre distances or kilograms. Compare stock and credits before and
after every pickup or transfer. A command gesture is not a cargo action.

## Pick up canisters

A working cargo intake automatically takes a drifting canister when the
canister physically enters its trigger and the hold has enough free mass
for the WHOLE canister. No pickup wire or command is needed. The agent view
has no canister or intake position. It cannot steer accurately into the
trigger from a snapshot alone. Make bounded collection attempts, then
compare `me.cargo.items` before and after. Only an increase in the relevant
item count proves pickup; proximity to a rock or a canister does not.

## Use the Inventory pane

Tap `system.interface_toggle` to open the TAB interface. While `ui.pause`
is `Interface`, `ui.targets` lists visible named nodes and logical-pixel
rectangles. Use the names in that CURRENT list with pointer gestures; move,
press and release in separate acts. Select the Inventory tab and a stock
row. The inspector opens an action draft with a default quantity of one.
Use that default or the named `InventoryDraftAll` target in `ui.targets`,
then click `InventoryDraftConfirm` when enabled. The numeric field and
wheel have no target name; setting an arbitrary quantity needs measured
pointer coordinates and is not discoverable by target name alone. Close
the interface with `system.interface_toggle` when finished.

- Undocked, selecting your own row with a working intake offers Jettison.
  It queues cargo canisters to leave through the intake. A docked ship
  cannot jettison.
- Docked, your own row offers Give to the other ship, or Sell if that
  partner trades. The partner's row offers Take for a lootable or neutralized
  ship, or Buy from a trader. These actions are contextual; a row is not a
  promise that every action is available. Selling changes both stock and
  credits. Check `me.cargo.items` and `me.cargo.credits` after Confirm.

A disabled Confirm or an unchanged hold means the move was not proven.
Inspect the visible draft and its refusal note; do not invent a jettison,
mining or transfer command.
