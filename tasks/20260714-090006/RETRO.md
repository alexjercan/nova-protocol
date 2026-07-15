# Retro: UI/feedback SFX pass (20260714-090006)

Closed 2026-07-15. Verdict APPROVE, no review rounds needed (self-review found
only design decisions, no bugs).

## What shipped

Four placeholder cues wiring the silent feedback moments the audio audit found:

- **MenuSelect** - one global `On<Activate>` observer clicks for every
  `MenuButton`, so all menu/pause/mods buttons sound without touching each
  handler.
- **UiToggle** - ESC pause-overlay open/close blip in `toggle_pause`.
- **DryFire** - dull click on the rising edge of a player turret pulling an
  empty magazine; edge-latched per turret, player-only.
- **RadarRetarget** - subtle tick on a held gesture's re-designation, via a new
  `RadarRetargeted` message, distinct from the once-per-gesture LockOn acquire.

## What went well

- The one-global-observer approach for menu clicks scaled to every button for
  free, instead of editing ~10 `on_X` handlers. Filtering on the `MenuButton`
  the `button()` helper already attaches made it a two-function change.
- Modelling the retarget as a message (mirroring `RadarLockAcquired`) kept the
  audio module the single home for cue playback and the targeting module free of
  audio concerns - the existing seam did the work.
- Reading the shoot system carefully showed the empty-mag `continue` is
  per-tick, so a naive cue there would machine-gun; moving detection to an
  edge-latched Update system avoided that from the start.

## Difficulties

- Adding a `MessageWriter<RadarRetargeted>` to `update_radar_search` broke two
  headless test rigs that build the system without the full plugin (they
  register messages by hand). Both needed the new `Messages<RadarRetargeted>` /
  `add_message` - the same `messagereader-needs-resource-guard-in-tests` shape
  from LESSONS.md, but for a writer: a new message param panics any minimal test
  app that runs the system without registering the message. Caught by grepping
  for the manual `add_message` sites before running tests.

## Lessons for next time

- **A new `MessageWriter<T>` param has the same test-rig cost as a reader.**
  Before adding one to a shared system, grep for every test that runs that
  system on a minimal app and add `add_message::<T>()` / the `Messages<T>`
  resource there, or it panics. (Reinforces the existing reader lesson.)
- **Prefer one marker-filtered global observer over N per-handler edits** for a
  cross-cutting cue - it is fewer edits and cannot miss a handler.

## Left out / follow-ups

- Settings/mods "expand/collapse" rides the menu-button click on their button
  rather than a separate panel-visibility cue (avoids double-sounding the same
  gesture) - a design choice, recorded in REVIEW.md D1.
- The dry-fire per-turret latch HashMap is not pruned for despawned turrets
  (negligible growth); README sound table refresh still pending (shared with
  20260714-090002).
