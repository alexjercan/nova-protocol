# SW-DOCK: a_docked_pair_saves_both_ships_undocked_at_their_poses

## Scope
Edited only `crates/nova_world/src/tests/frozen.rs` in the `resumable-worlds`
worktree. No other file was hand-edited. `cargo fmt -p nova_world` (run as
instructed) additionally reformatted other already-modified files in that
package - see "Pre-existing worktree state" below.

## Test added
`a_docked_pair_saves_both_ships_undocked_at_their_poses`, gated
`#[cfg(feature = "serde")]` + `#[test]`, placed between
`a_snapshot_of_the_live_world_reads_back_as_the_same_world` and
`a_loose_canister_freezes_with_the_cell_it_drifts_in`.

Flow:
1. Builds the same docked pair as
   `a_docked_partner_moves_to_the_cell_it_stands_in_when_its_home_cell_retires`
   (arm `Nothing` at `home`, a home `SectorRoot`, a player `docking_hull` with
   `WorldObserver` + `PlayerSpaceshipMarker`, a partner `docking_hull` child of
   home), docks them, and settles. No helm, no burn.
2. Asserts the pre-snapshot dock: one `DockingConnection`, `DockedShip` on
   both.
3. Gives both ships the extra components `freeze_ship` requires (see below),
   then calls `nova_scenario::prelude::freeze_ship` directly on the player
   (imported as `freeze_ship`) and asserts it succeeds while docked and that
   the player's `Position` is unchanged (the function takes `&World`, so this
   mostly proves it's reachable and read-only from `nova_world` tests - see
   "Player freeze_ship caveat").
4. Calls `crate::snapshot_sectors::<Nothing>(world)` and asserts the live pair
   is untouched: both entities alive, both still `DockedShip`, one
   `DockingConnection`, the partner still `ChildOf(home_root)`, both
   `Position`s unchanged.
5. Asserts the ledger's `home` record holds the partner as
   `FrozenBodyType::Ship`, under its `EntityId("partner")`, with
   `transform().translation` equal to its live `Position`.
6. Round-trips the ledger through `ron::to_string` / `ron::from_str`, restores
   it into a fresh `ship_app()` armed with `Nothing` at `home`, materializes
   `home` with `materialize::<Nothing>(world, home, far())`, and settles.
7. Asserts the partner comes back exactly once, at its saved pose, `ChildOf`
   the new root, with no `DockedShip` and no `DockingConnection` anywhere in
   the fresh world.

## Components added, and why
`docking_hull` builds a bare hull for docking-mechanics tests only; it carries
none of what `freeze_ship` (`crates/nova_scenario/src/objects/spaceship.rs:633`)
and `freeze_section` (`crates/nova_ship/src/sections/frozen.rs:187`) require.
Tracing both functions' panics/requireds against `docking_hull`'s bundle:

- `DamageMarks`, `ShipInventory`, `ShipCredits` - **not added**: already
  arrive as `SpaceshipRootMarker` required components
  (`crates/nova_gameplay/src/markers.rs:48`).
- `SpaceshipDesign` - added, pointing at a new one-section `Inline` design
  (`docking_hull_design()`) naming `fore` against a new `test_docking`
  section prototype (`load_docking_section()`, pushed onto `GameSections`
  the same way `wreck_home` pushes its turret prototype). A bare
  `SpaceshipDesign::default()` (empty sections) freezes fine, but panics
  on thaw: `insert_spaceship_sections` refuses a frozen section the design
  doesn't name (`crates/nova_scenario/src/objects/spaceship.rs:1142`), and
  the partner's live `fore` port is a real frozen section. Naming it in the
  design was the smallest fix once the round-trip step (assertion 6/7) forced
  the issue; it also means the fresh world's `fore` port now spawns through
  the real `base_section` + `docking_section` bundle, matching how a scenario
  ship actually works.
- `SpaceshipController`, `ShipCapabilities` - added as `default()`; both are
  hard-`required()`-panics in `freeze_ship` with no required-component
  default.
- `EntityId` - added (`"player"` / `"partner"`) per the task's note; `freeze_body`
  reads it as `Option` but the test needs stable ids to find bodies by.
- On the `fore` section: `Health::new(10.0)` and
  `SectionAnimations::new(Vec::new())` - added; both are hard panics in
  `freeze_section` and `docking_section()` alone carries neither (unlike
  `base_section`, which a hand-spawned `docking_hull` never calls).

Two small helpers were extracted since both the player and the partner need
the same insertions, and the design needs the same catalog entry live on both
the first and the restored app: `fore_section` (find the child the hull was
built with), `docking_hull_design` (the one-section design), and
`load_docking_section` (push the `test_docking` prototype onto `GameSections`).

## Pass output
```
cargo test -j 4 -p nova_world --features serde --lib a_docked_pair_saves
running 1 test
test tests::frozen::a_docked_pair_saves_both_ships_undocked_at_their_poses ... ok

cargo test -j 4 -p nova_world --features serde --lib frozen
running 12 tests
... (all 12 pass, including the new one and the two it reused fixtures from)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 29 filtered out
```
`cargo fmt -p nova_world` ran clean; a follow-up `cargo fmt -p nova_world --
--check` reports no diffs.

## Mutation check
Flipped the final assertion to `world.get::<DockedShip>(restored_partner).is_some()`
("the partner comes back undocked"). Reran the single test: it failed with
```
thread '...' panicked at crates/nova_world/src/tests/frozen.rs:1156:5:
the partner comes back undocked
```
Reverted the edit with the same `Edit` tool call in reverse; reran the full
`frozen` module afterward and all 12 tests pass again.

## Player freeze_ship caveat (assertion 4)
`nova_scenario::prelude::freeze_ship` is directly reachable and callable from
`nova_world`'s tests (the crate is already a dev-dependency here, imported for
`SpaceshipPlugin` etc.), so I called it directly on the docked player rather
than through `snapshot_sectors` (which deliberately excludes `WorldObserver`).
It takes `&World` and returns only a `FrozenShip` (design/controller/
capabilities/credits/section state) - it carries no pose field of its own.
So "keeps the player's pose" is proven only as: the call cannot mutate the
world (it borrows `&World`), and the player's live `Position` is the same
before and after. The session-level pose capture around a saved player (a
`ResumedSpaceship`-shaped record) is assembled outside `nova_world`/
`nova_scenario::objects::spaceship`, so a true "same pose after a full
player-save round trip" proof is not reachable from a `nova_world` test;
flagging per the task's own fallback instruction.

## Pre-existing worktree state (not mine)
`git status` showed this worktree already dirty before I touched it: edits in
`crates/nova_world/src/frozen.rs`, `streaming.rs`, `generation.rs`, `lib.rs`,
`Cargo.toml`, `nova_world_base`, and several other crates, plus untracked
`crates/nova_world_base/src/save/` and sibling reports
(`GATE.md`, `REVIEW-S123.md`, `SLICE4.md`, `SW-SERDE.md`) in this same task
folder - evidence of other workers' in-flight, unstaged work in this shared
worktree. I did not edit any of those files. Running the owner-specified
`cargo fmt -p nova_world` reformats the whole package, so it touched those
already-dirty production files too (format-only, since a follow-up
`--check` is clean); I did not inspect or revert their content. Nothing was
staged or committed.
