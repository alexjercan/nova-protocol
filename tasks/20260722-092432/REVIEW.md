# Review - convoy loiter / non-combatant AI (task 20260722-092432)

- VERDICT: APPROVE

Out-of-context review of branch `content/convoy-loiter` vs `master`. The feature
is correct, complete against the task's DoD, and green. Findings below are all
LOW / informational - none blocks the merge.

## Verification

- `cargo test -p nova_gameplay --lib a_non_combatant` - 1 passed.
- `cargo test -p nova_scenario --lib an_unarmed` - 1 passed.
- `cargo test -p nova_assets --test lifeline_convoy` - 8 passed.
- `cargo check -p nova_gameplay -p nova_scenario -p nova_assets` - clean.
- (Full suite + clippy skipped per repo memory; CI runs them.)

## Focus 1 - NON-ENGAGEMENT COMPLETENESS: complete, guaranteed

Traced every AI system for a targetless / non-combatant ship. There is NO path
where a non-combatant aims, fires, chases, or panics.

- `update_ai_target` (ai.rs:288-296): skips the non-combatant, clears `AITarget`
  every frame (defensively, in case it was armed when it last acquired). Good.
- `update_behavior_state` (ai.rs:918-921): `hostile_distance` is derived from
  `**target` (always None here), so it is `None`. `next_behavior_state`
  (ai.rs:811-813) returns the passive state on `None` hostile_distance BEFORE the
  `recently_damaged` engage branch (line 815) is ever reached. So a non-combatant
  holds Patrol/Idle/Orbit EVEN WHEN SHOT - the strongest possible guarantee, and
  exactly the "held under fire" behavior the owner wants. Pinned by the new unit
  test's `is_passive()` assertion.
- `update_turret_target_input` (ai.rs:1440-1446): iterates turret sections
  filtered to this parent; a non-combatant has none, so the loop body never runs
  - no aim, no fire, no panic. Firing is moot (no guns), confirmed structurally.
- `update_passive_flight`: driven purely by `AIBehaviorState` (passive), so it
  flies the loiter loop. Correct.
- No system indexes/unwraps in a way that a targetless AI ship could panic.

## Focus 2 - AUTO-DETECT CORRECTNESS: correct

- `has_weapon` (spaceship.rs:368, 387) is set in the Turret/Torpedo match arms,
  which run on the RESOLVED `config.kind` (spaceship.rs:346) AFTER the source is
  resolved (Inline used as-is, Prototype looked up). So prototype-sourced weapons
  (e.g. the racer's turrets) DO set the flag - an armed prototype ship is not
  mis-tagged. An unarmed hull (cargoa) never hits those arms -> tagged. The unit
  test covers unarmed-AI (tagged), armed-AI (not), player (never).
- INFO (not a bug): a weapon disabled by a `SectionModification` still sets
  `has_weapon` (the section is still `SectionKind::Turret`), so such a ship would
  NOT be tagged non-combatant. No shipped content does this, and the task
  explicitly defers "weapons destroyed => non-combatant" to the dynamic
  critical-damage backlog. Acceptable, documented boundary.

## Focus 3 - LOITER GEOMETRY: sound

Computed from the consts:
- QUEEN legs 137.5 / 106.8 / 124.5; MERIDIAN legs 95.5 / 80.6 / 110.6 - ALL
  exceed the ~75u arrival radius (arrival_standoff 50 + AI_WAYPOINT_SLACK 25), so
  the haulers FLY the loop, they do not collapse to station-keeping. Good.
- Centroids match the holding stations: QUEEN 1.7u off QUEEN_POS, MERIDIAN 17.1u
  off MERIDIAN_POS. Loops stay centred in the belt where the player defends.
- No waypoint is within 30u of any lane boulder (radii 3.5-5.0); min inter-hauler
  waypoint distance is 65u. Clear of cover and of each other.
- Max waypoint-to-centroid radius is 79.9u (Queen), well under the 200u probe
  bound.
- cargoa hull has two rear thrusters (cubes (1,1,2)/(-1,1,2)) + a controller
  (craft.rs:559-601), so the AI can actually fly the plan; the probe confirms it.
- No `leash` on the haulers - fine: a non-combatant never engages, so it always
  flies the patrol; a leash would be inert. (The task mentioned a leash as one
  option; the auto non-combatant tag makes it unnecessary.)

## Focus 4 - DEFENSE SCENARIO INTACT: yes

- Haulers stay `allegiance: Some(Player)` (lifeline.rs:219), so raider AI still
  hunts them (pinned in lifeline_convoy.rs on_start_stages_the_lane).
- Loss is keyed on `destroyed(ID_QUEEN)` / `destroyed(ID_MERIDIAN)`
  (lifeline.rs:641,650) - entity destruction, unaffected by movement.
- Objective markers use `mark(ID_QUEEN, ...)` / `mark(ID_MERIDIAN, ...)`
  (lifeline.rs:469-470) keyed to the hauler entity, so the gold marker follows
  the moving ship. `the_relief_bell_wins_and_the_banner_tracks_the_convoy`
  passes.
- Generated RON (lifeline.content.ron) matches the builder waypoints exactly -
  regenerated, not hand-edited.

## Focus 5 - TEST ADEQUACY: adequate

- `a_non_combatant_never_targets_or_engages` pins target-None + is_passive AND an
  armed control that DOES acquire - it would fail before the change (the old
  weaponless AI would engage).
- `an_unarmed_ai_ship_is_flagged_non_combatant` pins unarmed/armed/player - fails
  before the tag existed.
- The in-region probe (examples/gameplay/lifeline.rs:298-317) asserts drift <
  200u from the loiter centre after two waves. The bound is meaningful: the loops
  span < 80u from centre, so a drifted-off None hauler blows well past 200u
  (Fix note claims fails-first; trusted). NOTE: the assertion is inside
  `if let Some(pos)`, so a destroyed hauler silently skips - acceptable
  (destruction is the defeat path's concern, not this probe's).

## Focus 6 - REFLECTION / PRELUDE

- `AINonCombatant` IS in the ai prelude (ai.rs:24). Good.
- LOW: `AINonCombatant` derives `Reflect` + `#[reflect(Component)]` but is NOT
  `register_type`'d in `SpaceshipAIInputPlugin::build`. However this exactly
  matches the existing precedent of `AILeash` and `AISpaceshipMarker` (same
  derives, also unregistered) - only the FSM/state components are registered. So
  it is consistent with the file's convention, not a regression this task
  introduces. If AI components ever need inspector/scene visibility, register
  `AILeash` + `AISpaceshipMarker` + `AINonCombatant` together as a follow-up.

## Other LOW findings

- Stale doc: `mirror_ai_combat_state` sets `WeaponsRaised(true)` / a `CombatLock`
  on a non-combatant whenever `update_point_defense_target` gives it an
  `AIPointDefenseTarget` (an inbound hostile torpedo) - it computes a PDC target
  even for non-combatants (ai.rs:379-418 has no non-combatant skip). Harmless: no
  turret sections consume it, so nothing fires and flight stays passive. Purely
  cosmetic (a "weapons hot" flag on a ship with no weapons). Optional cleanup:
  skip non-combatants in `update_point_defense_target` too, for tidiness.
- Stale module doc: `lifeline_convoy.rs:9` still describes the haulers as
  "`controller: None` ... cannot chase". The test BODY was updated to assert the
  AI controller (line 243), but the header comment was not. Cosmetic doc rot.
