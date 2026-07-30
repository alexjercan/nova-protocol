# RETRO - nova_ui slider track: live re-skin + hardware fill follows the value

- TASK: 20260729-211155
- DATE: 2026-07-30
- ROUNDS: 4 (round 1 APPROVE with 2 MINOR + 4 NIT; round 2 REQUEST_CHANGES with
  1 MAJOR + 1 MINOR + 3 NIT; round 3 REQUEST_CHANGES with 2 MAJOR + 2 MINOR +
  3 NIT; round 4 confirmation pass, APPROVE with 1 NIT)

## What shipped

Two owner-reported defects in `nova_ui`'s `slider_track`, plus the sibling half
of a gap task 20260729-121847 had explicitly parked:

- `SliderFill` marks the hardware track's solid fill, and `sync_slider_meters`
  became `sync_slider_tracks` - ONE system lights the phosphor block-meter AND
  moves the hardware fill, watching `SliderRange` as well as `SliderValue`.
- `SliderTrackSkin(f32)` + `reconcile_slider_track_skins` rebuild the track live
  on a `UiSkin` flip. The slider is the first reconciler that REBUILDS rather
  than repaints, because its two skins are structurally different widgets. The
  rebuild is one silenced entity command so it cannot panic on a track a caller
  despawned in the same frame.
- `SegmentedSkin` + `reconcile_segmented_skins` repaint the segmented container,
  whose options were already `ThemedButton`s - which is exactly what made the
  row read as half-reskinned.

## What went well

- **Falsification, applied relentlessly, is what made this cycle honest.** Every
  test was run against the pre-fix code. That caught three separate cases where
  a test proved nothing, each of which would otherwise have shipped as a green
  checkmark over an unpinned fix.
- **Pinning the SHIPPED caller, not just the widget** (`pin-each-caller-not-just-
  shared-core`): the nova_menu test on the actual volume slider is the one that
  maps to what the owner played.
- **The render eyeball found a free win.** The widget zoo's top-bar segmented
  control is spawned once in `setup` and never rebuilt, so the two Xvfb captures
  are direct in-engine proof of `reconcile_segmented_skins` - no new rig needed,
  just noticing which widget the example does NOT rebuild.
- **Three independent out-of-context review rounds each found something real**,
  and rounds 2 and 3 each found a defect in the PREVIOUS round's fix. The value
  was in re-reviewing the fixes, not just the original diff.

## What went wrong

Four misses, in escalating order of how close each came to shipping:

- **A reviewer's suggested fix was wrong, and only a test caught it.** Round 1
  reported that a display-only track empties itself on a skin flip, with a patch
  attached: read `Option<&SliderValue>` in the reconciler. But a display-only
  track has no `SliderValue` by definition, so `map_or(0.0, ..)` zeroed it
  exactly as before. Writing the test for the reported failure mode is what
  exposed it. The real fix was to remember the fraction on `SliderTrackSkin`.
- **A fix that guarded the wrong call.** Round 1's other MINOR was fixed with
  `try_insert` - but the `despawn_related::<Children>()` immediately before it
  still queued through the default panicking handler, so the guard bought
  nothing and the pair still crashed in exactly the scenario its own comment
  described. It reads as fixed; it was not. Round 2 caught it.
- **Two false pins written for that fix, one after another.** First an app-level
  test with a despawning system ordered `.before` the reconciler - it passes
  with the broken code, because the `.before` edge auto-inserts an
  `ApplyDeferred`, so the despawn lands first and the reconciler never matches
  the entity. Then a command-level test calling the fixed command directly -
  tautological, since being one silenced command IS the fix. Both were caught by
  falsifying, and both were deleted.
- **I then declared the thing untestable, in a doc comment, and was wrong.**
  Round 3 wrote the test in a few lines: set `auto_insert_apply_deferred: false`
  on the schedule and the ordering accident becomes deterministic. Two failed
  attempts had made "impossible" feel earned; it was just the third idea.
- **The lesson I wrote from all this was itself factually wrong** - see below.

## Lessons

- `verify-a-new-lesson-before-it-enters-the-ledger` (NEW, and the sharpest one
  here): I recorded `cargo check --all-targets does not compile a lib's
  #[cfg(test)] unit tests`, having "confirmed" it by re-breaking the import and
  watching the check stay green. The observation was real; the CAUSE was
  invented. `--all-targets` sees `#[cfg(test)]` fine. The truth is that this
  repo's root Cargo.toml is a PACKAGE with deliberately no `default-members`
  (documented at Cargo.toml:274), so a BARE `cargo check --all-targets` scopes
  to the root package and never builds member crates' test targets;
  `--workspace --all-targets` (what CI runs) catches it immediately. A ledger
  entry is a claim future sessions act on without re-deriving, so the bar for
  writing one is a tested mechanism, not a reproduced symptom plus a plausible
  story. Distinguish "I reproduced the SYMPTOM" from "I verified the CAUSE".
- `use-workspace-all-targets` (NEW, replaces the wrong entry): a bare
  `cargo check --all-targets` at this repo root only covers the root package.
  Any DoD naming it as a compile gate should name `cargo check --workspace
  --all-targets` instead, plus `cargo test -p <crate> --lib` for touched crates.
- `applied-fix-still-needs-its-own-test` (NEW): a fix suggested by a reviewer,
  another agent, or a plan is a hypothesis like any other - it gets the same
  red-first test as a fix you invented. The authority of the source is not
  evidence. Here a well-argued reviewer patch with code attached did not address
  its own reported failure mode.
- `guard-every-command-in-the-chain` (NEW): silencing/guarding one call in a
  command chain does not protect the others. `despawn_related().try_insert(..)`
  looks guarded and is not - `try_insert` is silenced, `despawn_related` queues
  through the default handler. When the guard matters, make the whole operation
  ONE silenced command rather than decorating the last call in the chain.
- `untestable-is-a-claim-that-needs-the-same-scrutiny-as-a-fix` (NEW): "this
  cannot be pinned" written into a doc comment is an assertion future readers
  will trust and stop questioning. Before writing it, name the specific
  mechanism that blocks the test - and check whether that mechanism is
  CONFIGURABLE. Here the blocker was `auto_insert_apply_deferred`, which is a
  `ScheduleBuildSettings` field you can turn off.
- `grep-the-old-symbol-after-a-rename` (recurrence, x2 this task): renaming a
  system leaves its old name in prose that compiles fine. Two stale
  `sync_slider_meters` references survived into review, and a third was still
  live at round 3. Grep the old identifier across `crates/`, `examples/`,
  `tests/` and docs as the last step of a rename.
- `rebuilt-view-writes-go-to-state-not-the-entity` (confirming use): the lesson
  reads as "not on the rebuilt children". State on the surviving PARENT is the
  correct home, and is what made the display-only case work.

## Do differently next time

- Write the test for a review finding BEFORE applying the suggested patch - the
  ordering is what makes it a red-first pin instead of a confirmation of
  whatever the patch happens to do.
- When a DoD's compile gate is `cargo check`, write `--workspace --all-targets`
  at PLAN time, and add `cargo test -p <crate> --lib` for each crate touched.
- Before writing a ledger entry, re-derive the mechanism one level deeper than
  the symptom, and try the counter-experiment that would disprove it. Two of
  this task's four write-ups were wrong on first attempt.
