# Charters: per-role end goals as saved facts

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.16.0,charters

## User facts
- Epic `20261012-100030`. Decision 13 (2026-10-12): six roles, one finite goal each: industrial, a mining empire (owned stations with stock targets met); scavenger, an own yard; transport, a station network; bounty hunter, a kill list; military, the Line Warship; pirate, turn a civilization Enemy and outlive it (provisional until the stance graph exists).

## Delivery gate
- Goals are content on the start role; progress is a saved fact; completion applies a refresh at thaw, never by regeneration. Stop for the owner on each goal's exact counts.

## Verification
- Asserted example per goal where feasible; the completion is a saved fact that survives a reload.

## Done when
- Six goals ship as content with progress on the Map and a permanent proof each.

## Phases
- Phase 1 (2026-10-17, landed): `StartRole.goal: Option<RoleGoal>` with kinds `StationNetwork`, `KillList`, `Flagship`; transport = station network (8 jobs, 4 stations, 2 rings), bounty hunter = kill list (6 hulls, 2 rings); progress derived from `Jobs::completed` each frame in `nova_world_base::goal`, `GoalRecord.met_at` stamped once and saved (format 19); CHARTER block in the Map's chart view; `goal: None` on industrial, scavenger, military, pirate.
- Phase 2 (2026-10-17, landed): scavenger = `OwnYard` (one owned station whose store holds the full recipe of a loaded section of every one of the seven part families, read live and off the frozen ledger); military = `Flagship { hull: "line_warship" }` (64,300 cr at the recipes, reachable from generated Armored hulls near 2,000 km or by refit); industrial = `MiningEmpire { stations: 2, escorts: 2 }` (owned stations live and frozen, escorts in the player's fleet, no stock clause). Proofs: `goal.rs` unit tests, `system_role_goal_yard`, `system_role_goal_flagship`, `system_role_goal_empire`.
- Phase 3 (open): pirate after the stance graph (`20261012-100036`) can turn a civilization Enemy. Completion refresh at thaw (see the 2026-10-17 owner direction: a charter is a milestone, not an ending). Then `goal` turns required and `None` dies.

## Agent decisions
- 2026-10-17: `goal` is `Option<RoleGoal>` for this phase (owner picked option a); `None` is documented as "no goal yet", no shim, no default; the field turns required when all six land.
- 2026-10-17: goals are evaluated from saved facts only: `Jobs.completed` turned from `Vec<JobId>` into `Vec<Job>` so the kind and the poster are readable after the hand-in; no new event or counter.
- 2026-10-17: a ring is `floor(|cell centre| / 240 km)` (`civilizations::ring_of`); a station stands in one ring and a hand-in serves one station, so lint refuses more rings than stations or hulls and more stations than jobs (review finding: such a goal is unmeetable).
- 2026-10-17: military stays `None`. Measured: the Line Warship is worth 64,300 cr at the build recipes (153 sections); generated Armored hulls reach at most 20,600 cr at advancement 0.01 (ring 1), 54,020 at 0.15 (ring 4), 60,680 at 0.30 (ring 6); every seed clears it only from advancement 0.50, about 2,000 km out. The `Flagship` kind is implemented, linted and unit-tested but no base role uses it.
- 2026-10-17: a met goal changes nothing in the world; the "extinct drones wake" refresh at thaw is not designed and not implemented. The record stamps once and the Map reads it; this phase does not meet the task's refresh gate.
- 2026-10-17: the save refuses a `met_at` ahead of the saved clock and a met record for a role with no goal, as `NeutralizedCivilizations` refuses a stamp ahead of the clock.
- 2026-10-17: seed invariance is proven at app level (`system_start_roles` streams the same station ids for every role on seed 10); the generator takes no role, so a generator unit test would be a tautology.
- 2026-10-17: the CHARTER block's hide path (a role with `goal: None`) is proven through `system_start_roles`: each Create asserts the charter progress is the role's goal title or empty, and the block hides on empty progress; no range opens the Map for a no-goal role.
- 2026-10-17 (phase 2): `OwnYard` takes no count and no lint: the seven families are the closed `ShipPartFamilyType`, and the base catalog, which every mod loads, holds a recipe-bearing section of each. A yard is the best single owned station (two half-stocked stations do not add up); a recipe with no parts stocks nothing (lint does not refuse an empty `parts` map).
- 2026-10-17 (phase 2): owned stations are read live (`OwnedStationMarker` + `StationStorage`) and frozen (`FrozenSectors` -> `FrozenStation::owned` + `storage`), so the yard reads met while its cell is off the window; no new saved fact, save format stays 19.
- 2026-10-17 (phase 2): military ships the phase 1 `Flagship` kind at the Line Warship bar per the owner's direction (expensive but reachable); no placement or job-mix change, the map stays seed-determined.
- 2026-10-17 (phase 2): "Stock targets met" is satisfied from the buy on: a bought station keeps its `TraderMarket.target` and hold, and `restock_thawed_traders` refills owned stations too (`economy.rs:36-47`). Options put to the owner: (a) drop the clause, `MiningEmpire { stations: 2, escorts: 2 }`; (b) restock skips owned stations so the player stocks them by trade; (c) keep the clause as is. Recommended (a).
- 2026-10-17 (phase 2, owner): option (a). Automatic restock stays as it is, kept simple; industrial = `MiningEmpire { stations: 2, escorts: 2 }`, two owned stations and the full fleet (`FLEET_SIDES`), no stock clause. Lint refuses more escorts than fleet slots.
- 2026-10-17 (phase 2, owner): the yard proof found a generated station's store (100 kg at advancement 0, 400 kg at 1) under the lightest full yard (540 kg: hull 20, thruster 30, intake 75, mining 75, docking 110, controller 115, turret 115), so `OwnYard` was unreachable everywhere. Options put: (a) raise the generated store to 1 t / 4 t; (b) sum every owned store; (c) one item per family; (d) grow the store on the buy. Owner picked (a): `STORAGE_CAP_G` is (1 t, 4 t), one stocked station stays the yard, placement untouched. The cap never shipped (absent at v0.15.0), so no changelog entry.
- 2026-10-17 (phase 2, owner): a charter is a finite optional milestone, like a first station buy, never an ending: the game is an endless sandbox, a met charter stamps the save and changes nothing else, and there is no completion or end-of-game screen. Docs and code say "milestone", not "end goal". The "refresh at thaw" gate in this task is read under that direction: any refresh is a world change, not a game end.
