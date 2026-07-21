# Review: hide cursor while flying (20260721-211500)

## Round 1 (out-of-context reviewer)

Reviewer examined `git diff master...HEAD` on `bug/hide-cursor-flight`.

### Findings

- **R1.1 (MAJOR, resolved as non-bug + doc fix):** "`toggle_debug_mode` does
  not toggle `InspectorEnabled`, so F11 never raises the inspector." Verified
  against source: `bevy_common_systems`'s `InspectorDebugPlugin` runs its OWN
  F11 `toggle_debug_mode` over the same `DebugEnabled` resource
  (`~/personal/bevy-common-systems/src/debug/inspector.rs:183`). So F11 does
  toggle the inspector - nova's `toggle_debug_mode` and bcs's run in lockstep
  off the same key. Not a bug. Fixed the misleading comment in
  `nova_debug/src/lib.rs` to name bcs as the toggle owner.
- **R1.2 (MAJOR, resolved as non-bug):** "tests do not verify the F11 toggle of
  `InspectorEnabled`." The toggle lives in bcs (third-party), not this diff;
  testing it would test the dependency, not our change. Our tests correctly pin
  the behavior our code owns (default-off + `sync_inspector_cursor`
  free/grab/yield). No change warranted.
- **R1.3 (MINOR, accepted with rationale):** the "flying" predicate is
  duplicated between `nova_menu` and `nova_debug`. Kept separate: the two live
  in crates with no dependency edge between them and carry slightly different
  guard sets (the observer early-returns on paused; `restore_cursor` checks
  `GameStates`), so a shared helper would be a thin, awkward wrapper. Added a
  mirror comment on the `nova_debug` predicate flagging the drift risk as low.
- **R1.4 (MINOR, addressed):** `nova_menu`'s `restore_cursor` /
  `regrab_cursor_on_player_spawn` had no tests. Added two observer tests in
  `nova_menu`: `player_spawn_hides_cursor_while_flying` (grab on spawn in
  flight) and `player_spawn_yields_to_pause` (no grab while paused).
- **R1.5 (NIT, addressed):** comment on the inspector default did not explain
  the F11 mechanism. Rewritten (same edit as R1.1).
- **R1.6..R1.8 (verification, no action):** idempotent change-detection guards,
  all three cfg gates removed, ammo-readout split preserved - reviewer
  confirmed correct.

### Response

R1.1/R1.2 were the only MAJORs and both resolved to non-bugs (verified F11 is
owned by bcs); R1.3-R1.5 addressed with a comment + two new tests. Tests green:
nova_debug 10, nova_editor 13, nova_menu 64.

## Verdict: APPROVE

The core fix (un-gating the three grabs + inspector-reclaims-cursor
reconciliation) is correct. The `manual:` DoD item (owner replays a dev build
and sees no cursor while flying) remains for the Finish checkpoint - the
headless harness has no real window to observe.
