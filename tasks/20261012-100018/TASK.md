# Epic: salvage from derelicts and neutralized hulls

- STATUS: CLOSED
- PRIORITY: 69
- TAGS: v0.16.0, epic, salvage

## User facts
- Stage 3 of the game expansion. Decisions 2026-10-12: salvage yes, blueprints no; strip a docked lootable hull into its sections' recipe parts scaled by health (decision 4); tow a derelict and sell it by delivery into a station's radius, one-port docking stays (6b); scrap value per section is content beside the recipe; lint refuses a profitable strip loop.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Salvage:` title prefix and the `salvage` tag.
1. Strip a docked lootable hull into recipe parts.
2. Tow a derelict and sell it by delivery radius.

## Done when
- A player strips a derelict, sells another by towing, and the wreck stays stripped after retire and thaw.

## Children
- `20261012-100019` Salvage: strip a docked lootable hull into recipe parts
- `20261012-100021` Salvage: tow a derelict and sell it by delivery radius

## Orchestrator close (2026-10-16)
- Strip landed and the owner accepted it (100019). The owner dropped tow-and-sell (100021) in favour of strip, so "sells another by towing" in Done when no longer applies. The wreck stays stripped across retire and thaw (`system_station_strip`).
