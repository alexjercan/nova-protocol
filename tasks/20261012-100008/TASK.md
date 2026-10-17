# Epic: New World wizard, start roles and station upgrades

- STATUS: CLOSED
- PRIORITY: 79
- TAGS: v0.16.0, epic, wizard

## User facts
- Stage 2 and 2b of the game expansion. Decisions 2026-10-12: New World is a wizard, seed then a start role; six roles (industrial, scavenger, transport, bounty hunter, military, pirate), each a small hull and kit at inner-ring advancement; roles are content (5); a pirate is a playstyle, not a side (5b); the advancement curve becomes smoothstep to about 4000 km with the offset scaled by the curve (4b); upgrades are a ladder at a station: same-family swap, then add at a free attachment point, then buy a whole ship, all validated by the generator's post-build checks, never free placement (6); fleet and bought stations land right after Stage 2 so the role goals are reachable (13).
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Wizard:` or `Upgrades:` title prefix and the `wizard` tag.
1. Wizard: seed then start role; roles as content.
2. Wizard: advancement curve smoothstep and far radius.
3. Upgrades: same-family swap at a station.
4. Upgrades: add a section at a free attachment point.
5. Upgrades: buy a whole ship from a tiered catalogue.
6. Fleet: hire an escort at a station.
7. Own station: buy a station from a living civilization.

## Done when
- A new player picks a role, starts in a small hull, and can swap, add, buy a ship, hire an escort and buy a station, each only while docked and each surviving a save round trip.

## Children
- `20261012-100010` Wizard: seed then start role; six roles as content
- `20261012-100011` Wizard: advancement curve smoothstep to about 4000 km
- `20261012-100012` Upgrades: same-family section swap at a station
- `20261012-100013` Upgrades: add a section at a free attachment point
- `20261012-100014` Upgrades: buy a whole ship from a tiered station catalogue
- `20261012-100016` Fleet: hire an escort at a station
- `20261012-100017` Own station: buy a station from a living civilization

## Orchestrator close (2026-10-16)
- Every child is closed: start roles, swap, add a section, buy a ship, hire an escort and buy a station. The owner accepted the last three (100013, 100014, and the docked flows) in live runs on 2026-10-16.

## Owner idea 2026-10-17 (start docked at an allied station)
- At New World, find the closest station of an allied civilization to the
  seed's start point and move the start there; the player ship begins docked
  at it. That station is the starting station and the only one that posts
  the class-specific starter jobs (jobs epic `20261012-100022`). Charter 0
  ("dock at a living station", `20261012-100031`) needs a new first step
  under this direction.

## Next up 2026-10-17 (start docked at an allied station)
- Generator: find the closest station of an allied civilization to the seed's
  start point, move the start there and spawn the player docked at it; keep
  the station's id as the starting station in the world save; charter 0
  gets a new first step; only this station posts the gated starter jobs
  (jobs epic `20261012-100022`). The New Game seed preview centres on it.

## Progress 2026-10-17 (start docked)
- Claim: New Game picks the living Ally station nearest the origin as the
  starting station, spawns the player at its berth standoff and docks it there;
  the save carries the station (format 21); charter 0 is two steps; only the
  starting station's board wears `StarterBoardMarker`.
- Evidence: `crates/nova_world_base/src/start.rs:55` (`starting_station`,
  refuses with `SectorFault::Generation { id: "seed <n>", field: "start" }`),
  `start.rs:128` (`start_session`), `start.rs` `berth_at_start` (docks the
  `StartBerthMarker` ship once the station streams in);
  `crates/nova_scenario/src/objects/start_role.rs` (`StartingStation`,
  `OpenWorldSession::is_starting_station`); `crates/nova_world_base/src/save/mod.rs`
  (`WORLD_SAVE_FORMAT = 21`, `WorldSaveState.start`, `check_saved_start`);
  `crates/nova_world_base/src/charter.rs` (`CHARTER_ZERO_UNDOCK_OBJECTIVE_ID`,
  `complete_charter_zero` skips the starting station, `point_charter_zero`
  never marks it); `crates/nova_world_base/src/jobs.rs` (`starter` flag,
  `TODO(20261012-100022)` where the gate goes);
  `crates/nova_authoring/src/base_content/scenarios/open_world.rs`
  (`charter_zero_undocked` handler); `crates/nova_menu/src/world_setup.rs`
  (`on_create_world` refuses a seed with no allied station on the seed field).
- Change: `SpawnStartRole` lost `position`/`rotation` (scenario grammar break,
  changelog `**(breaking)**`); charter 0 completion moved from the authored
  `OnDocked` handler to code, because only the session knows the start id.
- Decisions: allied pick = nearest station (by `pose.position.length()`, ties
  by id) among the living `StanceType::Ally` civilizations of the 3x3x3
  lattice block round the origin, confirmed through `StationClaims::claim`;
  none -> generation error with the seed, surfaced as a `TextFieldError` on
  the seed field, never a fallback. Charter 0 steps: `charter_0_undock`
  ("Charter 0: undock from your starting station") posted on a fresh start,
  then `charter_0` ("Charter 0: dock at another living station") posted by the
  player's first undock from a living station; a dock back at the starting
  station or at a dead station does not complete it. Save: `start` is a
  required field, no migration (format 20 was unshipped).
- Verification: unit `nova_world_base` `start::tests`, `charter::tests`,
  `save::tests`, sync tests; `nova_authoring` `open_world::tests`;
  `nova_menu` world_setup/pause/menu tests; `content gen` + `content lint`
  (0 errors). Proof `system_world_chart` under Xvfb :96 + lavapipe: PASS
  `the loaded chart holds 3 cells`, `autopilot: cycle complete, no panic`;
  markers `outcome: the player starts docked at the allied starting station`,
  `outcome: undocking posts charter 0's dock step`, `outcome: the leave save
  holds the chart and the start`; capture `start-docked-station.png` shows the
  Station pane of `Fenda station` (seed 10, cell (0, 6, 2)) with the player
  hull docked. Stations are a civilization apart, so the walk teleports to the
  pinned waystation cell (1, 0, 0) for its second dock.
- Open: ~20 `examples/systems/` ranges and `examples/playable/world_sectors.rs`
  still wait on a window round `SectorCoord::ORIGIN` or pick stations outside
  the origin cell; they need the same player-cell rework as the chart walk.
