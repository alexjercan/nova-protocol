# REVIEW - 20260723-000253 Add SetAllegiance scenario action

## Round 1

Out-of-context diff-mechanics review of branch `feat/set-allegiance-action`
(one commit, 6bb1b60c) against master. Deep runtime semantics were verified
out-of-band before this round (see "Semantics verified out-of-band" below).

### Diff-mechanics verification (all confirmed in the diff)

- Config struct `SetAllegianceActionConfig { id: String, allegiance: Allegiance }`
  at `crates/nova_scenario/src/actions.rs:1070`, with the same
  `#[derive(Clone, Debug)]` + serde-gated derives as `SetSpeedCapActionConfig`
  (actions.rs:1019).
- Enum variant `EventActionConfig::SetAllegiance` at actions.rs:65, dispatch arm
  at actions.rs:128, both shaped exactly like SetSpeedCap's (actions.rs:61/122).
- The `EventAction` impl (actions.rs:1077) mirrors SetSpeedCap's apply path
  line-for-line: `push_command` -> `commands.queue` -> `query_filtered::<(Entity,
  &EntityId), (With<ScenarioScopedMarker>, With<SpaceshipRootMarker>)>` ->
  `find(entity_id.0 == id)` -> warn-and-return on miss
  (`"SetAllegiance: no scoped ship with id '{}'"`) ->
  `world.entity_mut(ship).insert(allegiance)` (insert = overwrite).
- Prelude re-export present: `SetAllegianceActionConfig` in the module prelude
  list at actions.rs:26.
- No new crate dependency: `Allegiance` arrives via the pre-existing
  `use nova_gameplay::prelude::*` in actions.rs; `crates/nova_scenario/Cargo.toml:18`
  already declares `nova_gameplay = { path = "../nova_gameplay" }`, and the serde
  feature (Cargo.toml:43) already forwards `nova_gameplay/serde`, so the serde
  derive on the `Allegiance` field is covered.
- Flip test `set_allegiance_flips_the_scoped_ship` (actions.rs:2601 area):
  spawns a real ship (`ScenarioScopedMarker` + `SpaceshipRootMarker` +
  `EntityId::new("magpie")` + `Allegiance::Neutral`), drives the action, flushes
  via `NovaEventWorld::state_to_world_system`, asserts
  `Some(Allegiance::Enemy)`. Fail-first: without the `insert(allegiance)` line
  the component stays `Neutral` and the first assert fails - the test cannot
  pass on a stubbed apply path. The unknown-id case ("nope") drives the action
  and flushes without panicking and asserts the ship is unchanged.
- RON round-trip test `set_allegiance_action_round_trips_through_ron`
  (actions.rs:2564) serializes `SetAllegiance((id: "x", allegiance: Enemy))`
  through ron and asserts id + allegiance survive, mirroring
  `set_skybox_action_round_trips_through_ron`.
- Nothing else changed: the only code diff is actions.rs (a prelude reflow, the
  new variant/arm/struct/impl, two tests); no existing action or behavior was
  touched. Other files are NOTES.md and the two wiki pages.

### Findings

- HIGH (R1.1) `crates/nova_scenario/src/lint.rs:454` (`check_action`) - missed
  exhaustive enumeration: the content lint validates the target `id` of every
  other id-referencing action via `check_target` - `DespawnScenarioObject`,
  `ObjectiveMarkerAttach`/`Detach`, `SetSpeedCap` (lint.rs:472),
  `SetControllerVerb` (lint.rs:475) - but `SetAllegiance` falls into the
  trailing `_ => {}` (lint.rs:505). A scenario authoring
  `SetAllegiance((id: "typo", ...))` lints clean and silently no-ops at runtime
  (just a warn! log), which is exactly the class of authoring slip the lint
  exists to catch - and the ch3 stealth rework (20260723-000320) is about to
  author this action. Suggestion: add
  `EventActionConfig::SetAllegiance(config) => { check_target(&config.id,
  "SetAllegiance", scenario, satisfiable, issues); }` next to the SetSpeedCap
  arm, plus a lint test mirroring whatever covers SetSpeedCap's unknown-id case.
  (The repo-wide grep for `SetControllerVerb` found no other enumeration site -
  lint.rs and the two wiki pages are the complete set; the wiki pages were
  updated.)

No other findings. Docs, tests, and wiring are otherwise clean.

### Semantics verified out-of-band (prior verification run; cited, not re-derived)

- AI targeting reads `Allegiance` via live per-frame queries
  (`update_ai_target`, ai.rs ~262-320); `AINonCombatant` is armament-based, not
  allegiance-based - so a runtime flip re-routes targeting live.
- Targeting already handles `Changed<Allegiance>`
  (`update_contacts_and_locks`, targeting.rs ~596-660: a hostile lock target
  flipping non-hostile clears the lock) - runtime allegiance change is an
  anticipated engine semantic; no consumer caches allegiance at spawn.
- Combat-lock acquisition is stance-gated, not hostility-gated (radar slot from
  `WeaponsRaised`, `radar_pick` has no `is_hostile` filter) - a Neutral ship
  CAN be painted and `OnCombatLock` fires on acquisition.

### Checks run (nix develop, this worktree)

- `cargo test -p nova_scenario --lib set_allegiance`:
  `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 134 filtered out`
  (`set_allegiance_action_round_trips_through_ron` ok,
  `set_allegiance_flips_the_scoped_ship` ok).
- `cargo check -p nova_scenario`: Finished, no warnings from this crate (only
  the pre-existing proc-macro-error2 future-incompat note).
- `cargo fmt --check`: clean.
- `cargo run -p nova_assets --bin content -- lint`:
  `content lint: 0 error(s), 0 warning(s), 0 finding(s), 13 scenario(s)
  balance-audited, 1 acked`.

### Docs

- `web/src/wiki/dev/scenario-system.md:121` - `SetAllegiance` added to the
  action bullet list next to `SetControllerVerb`.
- `web/src/wiki/dev/guide-author-scenario.md:550` - `### SetAllegiance` section
  with a RON example (`SetAllegiance((id: "ship_x", allegiance: Enemy))`),
  correctly slotted between SetControllerVerb and CreateScenarioArea.

### Verdict

One HIGH finding (missed lint enumeration, R1.1); everything else mirrors
SetSpeedCap faithfully and all checks are green.

- VERDICT: REQUEST_CHANGES

## Round 2

R1.1 (HIGH, lint enumeration gap) is fixed exactly as suggested: `check_action`
in `crates/nova_scenario/src/lint.rs` gains the `SetAllegiance` arm calling
`check_target` (same shape as SetSpeedCap), pinned by a new fail-first lint test
`dangling_set_allegiance_target_is_an_error` (a "ghost" id yields exactly one
error naming both the id and the action). Verification re-run from the committed
state: `cargo test -p nova_scenario --lib -- set_allegiance` 3 passed (round-trip,
flip, dangling-target), `cargo check -p nova_scenario` clean, `cargo fmt --check`
clean, full-tree `content lint` 0 errors / 0 warnings. No other R1 findings were
blocking; the out-of-band semantic verification stands.

- VERDICT: APPROVE
