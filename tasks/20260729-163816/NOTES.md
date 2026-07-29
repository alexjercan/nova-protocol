# Notes: objective read-notification stack

Design record for the shipped change (task 20260729-163816). The artifact fork
and the owner's call live in `DECISION.md`; this is what was built and why the
details went the way they did.

## What was built

`crates/nova_gameplay/src/hud/objective_stack.rs` - a top-centre column of demo
2 `.obj` chips (amber, bordered, diamond + the objective's own text), one per
posted objective, newest on top. `crates/nova_gameplay/src/hud/objective_hint.rs`
is DELETED: the status bar is fps + version again.

The lifecycle is a read notification with a HANDOVER, held in
`ObjectiveNotifications`. Two clocks per notification: `age_secs` from the
posting (it drives the handover fallback) and `pop_secs` from the handover -
the chip is not rendered, does not pop and does not start its dwell until the
reveal card has tucked into the stack, so the objective's text is never on
screen twice. A re-worded objective gets no card (it is not an "addition"), so
a fallback hands over after `REVEAL_TOTAL_SECS` instead of waiting forever.

- `post_objective_notifications` diffs `GameObjectives` - a new id, or the same
  id with new words, posts a chip unread;
- `age_objective_notifications` ticks it and marks it read at
  `OBJECTIVE_DWELL_SECS`, then fades and drops it;
- `read_on_nova_os` marks EVERY shown chip read the moment `PauseStates::NovaOs`
  is entered - opening the computer is reading them;
- `sync_objective_chips` rebuilds the rendered chips from that list each frame;
- `pop_chip_on_reveal_tuck` takes the handover off `ObjectiveRevealTucked`,
  matching the message's objective ID (a card can outlive its notification, and
  an anonymous tuck then hands over the WRONG objective - review R2.1),
  writing notification STATE rather than the chip's `HudEmphasis` - the chips
  are rebuilt every frame, so a pop written onto a chip entity is overwritten
  before it plays (review R1.1: it was, and the pop was invisible). It is
  ordered BEFORE the render, or every chip appears a frame late;
- `breathe_objective_chips` gives a settled unread chip the 2.4 s breath;
- `update_stack_anchor` publishes `NovaOsTabAnchor` from the stack.

## Decisions and deviations

- **No `// <range>` suffix**, unlike demo 2's mock. An `Objective` carries no
  link to a world entity: `ObjectiveMarkerTarget` has a free-form `label` and no
  objective id, so there is nothing to measure a distance TO. Range stays the
  world-anchored marker chip's job, which has the target. This also resolves the
  planned "does the label appear twice" step: the stack shows the objective's
  SENTENCE, the marker chip shows its own short label plus range - different
  strings, no duplicate.
- **A completed objective drops its chip instead of re-posting.** The plan said
  a change or completion re-posts unread; on completion the objective is gone
  from `GameObjectives`, so a chip for it would be stale text, and the
  completion cue is already `objective_feedback`'s green ghost. Re-wording an
  active objective still re-posts. Seen live: the lifeline walk completes
  `screen_convoy` 1.3 s after posting it, and its chip leaves with it.
- **The stack sits at 96 px, not demo 2's 58 px.** The mock has nothing else up
  there; the game's scenario readout strip (`readout.rs`) is a top-centre column
  at 16 px growing down, and one two-line readout (`RELIEF 01:09.7`) already
  reaches ~65 px. Measured on the lifeline walk - 58 put the chip on the run
  timer. KNOWN LIMIT recorded at the constant: two or more readouts can still
  reach 96. The durable fix is one shared top-centre column both flow inside,
  which restructures a working widget and is out of scope here.
- **The diamond is a rotated bordered SQUARE node, not `\u{25c6}`.** The first
  cut used the character and rendered tofu - the shipped Iosevka has no diamond.
  `objective_markers` already draws the same mark as a rotated square; copying
  that was the fix (`verify-bevy-api-at-callsite` / copy the in-repo callsite).
- **The TAB footer is amber-dim, not phosphor.** The retired hint's TAB was
  phosphor because it lived in the phosphor status bar; under an amber chip a
  green word reads as a second, unrelated element.
- **`chip_paint` is not used on the chip.** It already carries a
  `BackgroundColor` + `BorderColor` pair, and the fade needs alpha-scaled
  versions - taking both is a duplicate-component panic at spawn (which is
  exactly what `cargo check` cannot see; it took running the example).

## Carried in from the previous task

`LOCK_COMPOSED_FIRING_PEAK` (torpedo_target.rs, task 20260728-175747) is only
read by its test, so the non-test build warned `never used`. It is now
`#[cfg(test)]`. The warning landed on master because the previous cycle's
warning grep was `^warning: unused`, which does not match
`warning: constant ... is never used`.

## Verification

- `cargo check --workspace --all-targets --features dev` clean, no warnings;
  `cargo fmt` clean.
- `cargo test -p nova_gameplay --lib -- hud::` 284 passed (10 for the stack,
  the 5 retired hint tests deleted with the module and their two live
  assertions re-homed onto the stack); `-p nova_menu --lib` 73.
- `cargo run -p nova_probe -- run playable` -> OK (5/6 measured;
  `fps_within_baseline` SKIPPED, no baseline).
- GPU eyeball under Xvfb on the `lifeline` chapter walk: the chip renders as
  demo 2's - hollow diamond, amber bordered slab, the objective sentence, the
  amber TAB footer under it - clear of the `RELIEF` readout strip, with the
  status bar back to fps + version. Both defects above (tofu diamond, the 58 px
  collision) were found this way and are fixed; the screenshots are the reason
  they were found at all.
- NOT verified visually, and worth knowing before the next objective-facing
  task: NO harnessed walk keeps an objective alive long enough to see the
  card-to-chip HANDOVER. The lifeline walk completes `screen_convoy` 1.3 s
  after posting it, and the reveal card takes 3.2 s to land, so the chip is
  correctly never reached; broadside posts none at all; menu_newgame's 6 s
  autopilot ends during the opening conversation, which posts no objective by
  design. The chip's RENDERING was eyeballed before the handover gate was added
  (the rendering code is unchanged since), and the handover itself - appear,
  pop to 1.16, settle, breathe, read, fade - is pinned by App-driven tests that
  assert the RENDERED scale and alpha. Two chips stacked at once is likewise
  test-only (`multiple_objectives_stack_newest_first`). The reviewer pointed
  out the cheap fix - the screenshot examples already run a timed
  `at(<secs>, ...)` script, so posting an objective at 1.0 s and capturing at
  4.5 s would show the handover deterministically. Filed as 20260729-182853
  rather than widening this branch.
