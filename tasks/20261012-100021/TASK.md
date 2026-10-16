# Salvage: tow a derelict and sell it by delivery radius

- STATUS: CLOSED
- PRIORITY: 67
- TAGS: v0.16.0, salvage

## User facts
- Epic `20261012-100018`. Decision 6b (2026-10-12): selling a derelict is a delivery into a station's service radius, never a dock, since the player's one port is busy with the towed hull; a whole hull must be worth more than its parts or nobody tows.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Towing is docking with a controller-less hull (`DockedAssembly`); a docked ship at retirement is a fault (`crates/nova_world/src/frozen.rs:442-451`), so a towed pair across a sector edge needs a rule.

## Delivery gate
- A `Sell derelict` action available inside the station radius while docked to a lootable hull; the price is the sum of recipe values at bid plus a hull premium that lint checks against the strip total. Decide the towed-pair crossing rule with the owner before code.

## Verification
- Asserted example: tow into the radius, sell, the hull despawns, credits rise by the stated sum; a tow across a sector edge does not fault.

## Done when
- Towing and selling work in a live run and the premium rule is a permanent lint.

## Orchestrator close (2026-10-16, not done)
- The owner dropped tow-and-sell on 2026-10-16: the whole-ship strip (100019) covers salvage and the owner prefers it. No code landed for this task.
