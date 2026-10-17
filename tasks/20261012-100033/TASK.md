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
- Phase 2 (open): scavenger own yard, industrial mining empire (stock targets need the restock model), military flagship once a generated hull above the Line Warship's value is reachable in play or the goal is re-authored, pirate after the stance graph (`20261012-100036`) can turn a civilization Enemy. Completion refresh at thaw. Then `goal` turns required and `None` dies.

## Agent decisions
- 2026-10-17: `goal` is `Option<RoleGoal>` for this phase (owner picked option a); `None` is documented as "no goal yet", no shim, no default; the field turns required when all six land.
- 2026-10-17: goals are evaluated from saved facts only: `Jobs.completed` turned from `Vec<JobId>` into `Vec<Job>` so the kind and the poster are readable after the hand-in; no new event or counter.
- 2026-10-17: a ring is `floor(|cell centre| / 240 km)` (`civilizations::ring_of`); a station stands in one ring and a hand-in serves one station, so lint refuses more rings than stations or hulls and more stations than jobs (review finding: such a goal is unmeetable).
- 2026-10-17: military stays `None`. Measured: the Line Warship is worth 64,300 cr at the build recipes (153 sections); generated Armored hulls reach at most 20,600 cr at advancement 0.01 (ring 1), 54,020 at 0.15 (ring 4), 60,680 at 0.30 (ring 6); every seed clears it only from advancement 0.50, about 2,000 km out. The `Flagship` kind is implemented, linted and unit-tested but no base role uses it.
- 2026-10-17: a met goal changes nothing in the world; the "extinct drones wake" refresh at thaw is not designed and not implemented. The record stamps once and the Map reads it; this phase does not meet the task's refresh gate.
- 2026-10-17: the save refuses a `met_at` ahead of the saved clock and a met record for a role with no goal, as `NeutralizedCivilizations` refuses a stamp ahead of the clock.
- 2026-10-17: seed invariance is proven at app level (`system_start_roles` streams the same station ids for every role on seed 10); the generator takes no role, so a generator unit test would be a tautology.
- 2026-10-17: the CHARTER block's hide path (a role with `goal: None`) is proven through `system_start_roles`: each Create asserts the charter progress is the role's goal title or empty, and the block hides on empty progress; no range opens the Map for a no-goal role.
