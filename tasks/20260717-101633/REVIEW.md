# Review: Controller section sounds (lock/radar/safety)

- TASK: 20260717-101633
- BRANCH: task-20260717-101633-controller-sounds

Reviewed the committed diff (905bc4b3) fresh + independent out-of-context pass.
Load-bearing claims verified twice (mine + reviewer's independently):

- The player-controller lookup matches the shipped topology: sections are
  DIRECT children of the ship entity (production input routing filters
  `ChildOf(parent) == spaceship`, player.rs:322/355/366; spaceship assembly
  parents via with_children, nova_scenario spaceship.rs:211). Every shipped
  playable ship uses the `basic_controller_section` prototype (reviewer swept
  shakedown/broadside/gauntlet/ledger/example) - which authors all five cues in
  regenerated base content. No inline controllers; no silent regression.
- Message readers drain BEFORE the controller check - the
  no-stale-replay test proves a message sent controller-less never replays.
- The torpedo's guidance controller (None sounds) can never match the lookup
  (parent is the projectile).
- Suites: nova_gameplay lib 538, gates 4/4, workspace all-targets clean.

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) assets/base/sounds/README.md "Required files" section - the
  intro still claims WORLD_SFX_FILES is "the full set"/single source of truth
  and the tables list the 8 MIGRATED sounds (turret_fire, torpedo_launch,
  dry_fire, lock_on, lock_off, safety_on, radar_deny, radar_retarget) as if
  bank-loaded; only 4 keys remain (ThrusterLoop, Explosion, Impact,
  SalvagePickup). A mod author would misread the migration state.
  - Fix: split the tables into "Section-authored defaults" (the 8, referenced
    from base content) vs "WorldSfx bank (transitional, 4 keys)"; reword the
    intro accordingly.
  - Response:

## Round 2

- VERDICT: APPROVE

R1.1 verified: the README now separates section-authored defaults from the 4
remaining bank keys, each authored file naming its owning config field. No new
findings; all round-1 code checks were already green.
