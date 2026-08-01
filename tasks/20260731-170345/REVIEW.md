# Review: KISS: nova_gameplay flight, camera, audio, juice

- TASK: 20260731-170345
- BRANCH: refactor/kiss-gameplay-flight-camera-audio-juice

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [x] R1.1 (MINOR) crates/nova_gameplay/src/audio/mixing.rs:204 - eight `#[test]`
  fns (204, 218, 238, 248, 261, 274, 308, 336) were widened to `pub(super)` by
  the column-0 visibility sweep; test fns are never called, so drop
  `pub(super)` from all eight to match NOTES.md's "narrowest visibility that
  works" claim.
  - Response: Fixed. Swept every split file for `pub(super) fn` immediately
    preceded by `#[test]` - exactly the eight in `mixing.rs` - and narrowed
    them back to private.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/settings.rs:22 - the re-wrapped
  module-doc paragraph leaves a 124-char line, contradicting NOTES.md's
  "re-wrapped so the deletions do not leave ragged lines"; re-wrap lines 20-24
  to the file's ~80-col width.
  - Response: Fixed. The bullet's continuation lines are back under 80 columns;
    `awk 'length>90 && /^\/\/!/'` over the file now returns nothing.
- [x] R1.3 (MINOR) tasks/20260731-170345/NOTES.md:150 - "Four
  `// --- section ---` separators were deleted (3 in `gravity.rs`, 1 in
  `juice.rs`)" is wrong: 13 were deleted; correct the count and the file list.
  - Response: Fixed. Re-counted against master
    (`git show master:... | grep -cE '^\s*//\s*-{2,}'`): flight.rs 7,
    gravity.rs 3, audio.rs 1, camera_controller.rs 1, juice.rs 1 = 13. NOTES.md
    now says thirteen with the per-file breakdown. The zero-hit claim for the
    post-pass grep was already accurate.
- [x] R1.4 (NIT) crates/nova_gameplay/src/flight/thrusters.rs - `ThrusterGroup`,
  plus `SFX_ROLLOFF_FLOOR`, `SFX_AUDIBLE_THRESHOLD`, `SFX_THROTTLE_PRUNE_WINDOW`
  (audio/mixing.rs), `HumLevels` (audio/loops.rs), `impact_destroy_sounds`
  (audio/combat.rs), `player_controller_sounds` (audio/cues.rs) and
  `unfinished_flight_app` (flight/tests/support.rs) are `pub(super)` but
  referenced only inside their own defining file; make them private.
  - Response: Fixed for seven of the eight. `ThrusterGroup` must stay
    `pub(super)`: it appears in the signatures of `cluster_thrusters` and
    `choose_group`, which the autopilot and manual-burn systems call across the
    module boundary, so making it private is a private-type-in-public-interface
    error. Verified by compiling the change and reverting it.

Re-derived in-session before accepting: R1.1 (8 `pub(super)` test fns confirmed
by `grep -B1 'pub(super) fn' | grep -c '#\[test\]'`), R1.2 (the 124-char line
confirmed by `awk length>90`), R1.3 (separator counts re-counted against
`master` per file), R1.4 (each item grepped across `flight/`,
`camera_controller/` and `audio/` for out-of-file references).

Checks rerun in-session after the fixes: `cargo check --workspace
--all-targets` green (only the pre-existing `nova_os_map`/`nova_os_ship`
ambiguous-import warnings), `cargo fmt --check` clean, `cargo test -p
nova_gameplay --lib` per module - flight 75, camera_controller 14, audio 30,
gravity 18 (+1 pre-existing ignored), juice 21, settings 8, damage 8; 0 failed.

The reviewer's own verification, independently reproduced: the line-multiset
comparison of the three deleted files against their folder modules shows no
behavior-bearing line differs (camera_controller zero master-only lines;
flight's 7 and audio's 5 are rustfmt re-wraps of signatures lengthened by
`pub(super)` or by changed import paths). Outside the three splits, the entire
non-comment diff is one line: the wiki project-tour table cell.

Pending user check (does not block APPROVE):

- DoD 6 `manual:` - owner skims the diff and agrees no behavior changed.
