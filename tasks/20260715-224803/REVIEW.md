# Review: Make Gauntlet Run a playable sequential slalom race

- TASK: 20260715-224803
- BRANCH: feat/gauntlet-race

## Round 1

- VERDICT: REQUEST_CHANGES

Diff reviewed against master (93604cd4): the RON gameplay in
`webmods/gauntlet/gauntlet.content.ron`, the bundle meta, the new
`crates/nova_assets/tests/gauntlet_race.rs`, and the `bevy_common_systems`
dev-dep. Ran the two relevant suites in the worktree devshell: `gauntlet_race`
2/2 and `webmods_validation` 1/1 (green). The gating design is sound and the
behavior test genuinely drives the real area->body bridge on the shipped RON -
good. One coverage gap blocks approval.

- [ ] R1.1 (MAJOR) crates/nova_assets/tests/gauntlet_race.rs:50,78 - the
  load-bearing "playable" wiring lives in `OnStart` (spawn `player_spaceship`,
  seed `gate=1`, first objective/marker), and NOTHING tests it. `race_app`
  injects `gate=1` itself and `register_on_enter_handlers` filters to
  `OnEnter`, deliberately skipping `OnStart`. This matters because a missing or
  broken OnStart `VariableSet(gate=1)` makes every gate's `Expression(gate==N)`
  filter evaluate to `Err(UndefinedVariable)` -> `false`
  (crates/nova_scenario/src/filters.rs:164-171, verified), so GATE 1 never
  fires and the whole race soft-locks - and `webmods_validation` only proves
  the file LOADS, not that OnStart is wired. Likewise a dropped player-ship
  spawn (the single change that makes this scenario playable at all) would ship
  green. Suggest: add a cheap structural assertion over the parsed shipped RON
  (reuse `gauntlet_scenario()`), asserting the `OnStart` event's actions
  contain (a) a `SpawnScenarioObject` whose base id is `player_spaceship` with a
  `Spaceship` kind, and (b) a `VariableSet` on key `gate` seeding `1.0`. That
  pins the wiring without an asset-heavy ship spawn.
  - Response: Fixed. Added `onstart_spawns_the_player_and_seeds_the_race` to
    gauntlet_race.rs: pulls the OnStart event from the parsed shipped RON and
    asserts (a) an action spawns a `Spaceship` with base id `player_spaceship`
    and (b) a `VariableSet` on key `gate` whose expression evaluates to
    `Number(1.0)`. Green (3/3).
- [ ] R1.2 (NIT) webmods/gauntlet/gauntlet.content.ron:43 - the sequential
  design silently depends on the gate trigger areas NOT overlapping: after a
  gate advances `gate`, the pilot must produce a fresh `OnEnter` on the next
  gate's area. Today that holds (gate1<->gate2 centres ~98.7u apart vs 25+25u
  radii, so a pilot cannot be inside gate 2's area at the instant they clear
  gate 1). But a future position/radius edit that lets two gate areas overlap
  could soft-lock a pilot who is already loitering inside the next area when it
  becomes live (no re-enter event fires). Worth a one-line comment near the
  beacon positions, or a note in TASK.md's Notes, so the invariant is explicit.
  - Response: Fixed. Added an INVARIANT comment block to the RON header stating
    the gate areas must not overlap and why, with the current ~90-140u spacing.
- [ ] R1.3 (NIT) crates/nova_assets/tests/gauntlet_race.rs:153 - the test
  asserts only the `gate` variable, not the `ObjectiveComplete` / marker
  progression the handlers also perform. Acceptable (the variable is the
  ordering proof and objectives are cosmetic HUD), but a one-line comment
  stating the test is intentionally scoped to the gating variable would save
  the next reader from assuming objective coverage exists.
  - Response: Fixed. Added a comment above the two bridge tests stating they are
    scoped to the `gate` ordering variable and do not assert the cosmetic
    objective/marker actions.

## Round 2

- VERDICT: APPROVE

Verified the round-1 responses against the new diff and re-ran the suite in the
worktree devshell: `gauntlet_race` 3/3 (new `onstart_spawns_the_player_and_seeds_the_race`
included), `webmods_validation` still 1/1.

- [x] R1.1 - `onstart_spawns_the_player_and_seeds_the_race` pulls the OnStart
  event from the parsed shipped RON and asserts the `player_spaceship` Spaceship
  spawn plus the `gate` seed evaluating to `Number(1.0)`. Falsifiable: dropping
  the spawn makes the `any(...)` false and dropping the seed panics the
  `.expect`. Closes the load-bearing gap.
- [x] R1.2 - INVARIANT comment added to the RON header (gate areas must not
  overlap, current ~90-140u spacing).
- [x] R1.3 - scoping comment added above the two bridge tests.

No new issues introduced by the fixes. Approving.
