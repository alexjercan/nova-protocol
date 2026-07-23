# REVIEW - player_speed reserved scenario variable

- Round 1
- Reviewer: out-of-context agent
- Date: 2026-07-23
- Branch: feat/player-speed-var (commit 5a70a774)
- Verdict basis: read of the full diff + surrounding source, plus a live run of
  the two new tests (`cargo test -p nova_scenario --lib player_speed`: 2 passed).

## Summary

The change adds a second engine-owned reserved scenario variable `player_speed`
that mirrors the `scenario_elapsed` clock pattern point for point: a const with
a rustdoc contract, a tracker system (`track_player_speed`) chained ahead of
`fire_on_update` under the single existing `scenario_is_live && Unpaused` gate, a
shared `is_reserved_engine_var` predicate driving BOTH lint sites, and two tests.
I checked each of the six scrutiny points below and found no defects.

## Findings

1. Player-scoping (BLOCKER candidate) - CLEAN.
   `track_player_speed` (loader.rs:450-459) queries
   `Query<&LinearVelocity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>`,
   which is byte-for-byte the scoping `track_player_locks` uses (loader.rs:688).
   An AI ship carrying only `SpaceshipRootMarker` cannot match. The test proves
   this with a co-resident AI ship burning at |v|=50 the entire run; the readout
   never leaks it.

2. Registration gate/chain/order - CLEAN.
   Registered as `(tick_scenario_clock, track_player_speed, fire_on_update).chain()`
   in the single `register_clock_and_pulse` (loader.rs:479-486), the same function
   the plugin and the test rigs share, so plugin/rig cannot drift. Ordering is
   correct and load-bearing: the tracker is chained AHEAD of `fire_on_update`, so
   the OnUpdate pulse that evaluates speed-gated filters sees THIS frame's value,
   not last frame's. Under the `Unpaused` run condition the whole chain is skipped
   while paused, which is what freezes the readout.

3. Fail-closed 0.0 + insert-every-frame - CLEAN and faithful.
   `.iter().next().map_or(0.0, |v| v.length() as f64)` publishes 0.0 with no
   player, exactly mirroring `scenario_elapsed`'s `_ => 0.0` (loader.rs:468-473).
   Inserting each frame (vs seeding once) matches the clock precisely:
   `insert_variable` overwrites (world.rs:352), and `world.clear()` at teardown
   drops `variables` (world.rs:259) so the readout resets for free. A pre-first-tick
   read of the var returns `Err(UndefinedVariable)` (variables.rs:70-73) - but that
   is the identical convention `scenario_elapsed` already has, and because the
   tracker is chained ahead of the pulse under the same gate, the first live pulse
   always sees a freshly-inserted value. No new hole. The lint undefined-variable
   exemption (below) means authored reads still lint clean regardless.

4. `is_reserved_engine_var` generalizes both lint sites - CLEAN.
   The predicate (loader.rs:428-430) is called at BOTH lint.rs:304 (the
   undefined-variable exemption, `continue`) and lint.rs:385 (the VariableSet
   write-error). The write-error message changed from "reserved engine clock" to
   "reserved engine variable '{}'" and now interpolates `config.key` instead of
   the hardcoded clock const. I verified no test pins the old phrase: the existing
   clock test (lint.rs:1608) asserts `message.contains(SCENARIO_ELAPSED_VAR)` (the
   var NAME, still present) and `errors(&issues).len() == 1` - both still hold. The
   message-string change breaks nothing.

5. Test quality - STRONG.
   `player_speed_var_tracks_live_velocity_and_fails_closed` (loader.rs:1930+) is
   built on the REAL `register_clock_and_pulse`, and covers: live tracking
   (5 -> 0 -> 10), player-scoping (AI at 50 never leaks), pause-freeze (readout
   latches 10.0 across 4 paused frames while the velocity is driven to 100),
   unpause-resume, and no-player fail-closed (0.0, not the AI's 50). The fail-first
   claim holds by construction: every non-zero `assert_eq!(speed, N)` reads via the
   `Some(Number) else 0.0` helper, so if the tracker were unregistered the var stays
   absent, `speed` reads 0.0, and the very first `== 5.0` trips. The lint test
   asserts both the clean read and the error write and pins the var name. Meaningful.
   (Note: the pause assertion depends on Bevy applying the `NextState` transition in
   the same `app.update()` before the gated `Update` chain runs, so the tracker is
   already skipped that frame - verified this is the correct Bevy semantics and the
   test passes accordingly.)

6. Multiplicity / NaN / teardown / doc-vs-code - CLEAN.
   `.iter().next()` is multiplicity-safe: if two player ships ever coexist it takes
   the first deterministically rather than panicking (`single()` would panic), a
   strictly safer choice than the query alternative; the game spawns one player.
   `Vec3::length()` is a plain sqrt of a sum of squares - no NaN/precision concern
   for realistic velocities, and `as f64` widens without loss. Teardown reset is
   handled by the shared `world.clear()`. The rustdoc on `PLAYER_SPEED_VAR`
   (loader.rs:409-418) and `track_player_speed` accurately describe the code
   (engine-written, fail-closed 0.0, player-scoped, chained ahead of the pulse,
   pause-freezing) - no claim the code doesn't honor. The prelude re-export was
   added (loader.rs:19).

## Verdict

Faithful mirror of the established `scenario_elapsed` reserved-clock pattern with
no correctness, scoping, ordering, lint, or teardown defects; tests are
production-faithful and would fail if the system were removed. Nothing to change.

VERDICT: APPROVE
