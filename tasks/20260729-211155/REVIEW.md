# REVIEW - nova_ui slider track: live re-skin + hardware fill follows the value

- TASK: 20260729-211155
- VERDICT: APPROVE (round 4; rounds 1 APPROVE, 2 REQUEST_CHANGES, 3 REQUEST_CHANGES)

## Round 1 - out-of-context reviewer, fresh context (2026-07-30)

Verdict: APPROVE. No BLOCKER/MAJOR. The reviewer independently verified the
load-bearing claims rather than taking them on trust:

- The system-ordering claim holds AND is load-bearing.
  `ScheduleBuildSettings::auto_insert_apply_deferred` defaults to `true`, so the
  explicit `.after(reconcile_slider_track_skins)` edge auto-inserts an
  `ApplyDeferred`: the `despawn_related` + child spawn ARE applied before
  `sync_slider_tracks` runs, same frame. Falsified by flipping `.after` ->
  `.before`, which turns both new widget tests red.
- The `slider_fraction` / `SliderRange::thumb_position` change is
  behaviour-neutral for every existing caller. Only two sites spawn
  `slider_track` (`nova_menu:2746`, `widget_zoo:310`) and both pair it with
  `SliderRange::new(0.0, 1.0)`, where `thumb_position(v) == v`.
  `examples/sections/turret_section/slider.rs` uses a non-`[0,1]` range but has
  its own thumb widget and never touches `slider_track`.
- `*node = slider_track_node(skin)` cannot clobber caller fields: `slider_track`
  already contains `Node`, so no caller can add one without a duplicate-component
  panic. `reconcile_segmented_skins` correctly touches only radius/bg/border,
  which it must, since that container's `Node` is co-spawned with a caller `Name`.
- `despawn_related::<Children>()` is the right Bevy 0.19 API; it collects sources
  before removal so hooks/observers still see the data.
- Re-ran the DoD: nova_ui 18 passed, nova_menu 74 passed, `check --all-targets`
  clean, `fmt --check` clean. Clippy's 4 warnings are pre-existing in `hud.rs`.
  Ran the widget zoo under Xvfb on the real GPU: no panic, clean exit, and
  confirmed the zoo's top-bar segmented control is spawned once in `setup` and
  never rebuilt - so it is fair in-engine evidence for `reconcile_segmented_skins`.

### Findings (2 MINOR, 4 NIT) - all applied

1. MINOR, `widget.rs` `reconcile_slider_track_skins`: a display-only
   `slider_track(0.7, skin)` (no `SliderValue`, which the signature invites)
   silently emptied itself on a skin flip, because the rebuild spawned children
   at a hardcoded `0.0` and delegated the whole "show the value" job to
   `sync_slider_tracks` - which cannot see a track with no value to sync from.
2. MINOR, same system: bare `commands.entity(..)` where the sibling
   `reconcile_panel_skins` deliberately uses `try_insert`. Latent, not live: the
   widget zoo despawns its whole body on the SAME frame this reconciler fires,
   and only nova_ui's earlier registration order keeps it from panicking.
3. NIT: the `SpawnChild` trait (18 lines + two impls) is unnecessary - the
   reconciler can reuse the factory's `SpawnWith`/`RelatedSpawner` shape.
4. NIT: `sync_slider_meters_lights_blocks_from_value` still named the renamed
   system.
5. NIT: the module header credited only `reconcile_button_skins`; there are now
   five reconcilers, and the slider is the only one that REBUILDS.
6. NIT: `SLIDER_TRACK_PHOSPHOR_H`/`_HARDWARE_H` were `pub` but not in the
   prelude and not needed outside the module.
7. Doc accuracy: no overclaims; both cited LESSONS anchors exist. The recorded
   `cargo check --all-targets` caveat is worth promoting to LESSONS.

## Round 2 - the fixes (2026-07-30)

All six applied. One of them did not survive contact:

- **Finding 1's suggested fix was wrong, and the test written to pin it caught
  that.** Reading `Option<&SliderValue>` off the track still yields `None` for a
  display-only track, so `map_or(0.0, ..)` zeroed it exactly as before. The
  fraction had to be remembered somewhere that survives the rebuild, so
  `SliderTrackSkin` became `SliderTrackSkin(pub f32)`: `slider_track` seeds it
  from its `fraction` argument, `sync_slider_tracks` refreshes it whenever a real
  `SliderValue` changes, and the reconciler reads it. This is consistent with
  `rebuilt-view-writes-go-to-state-not-the-entity`, which forbids state on the
  REBUILT CHILDREN - the track is the surviving parent. The `if track.0 !=
  fraction` guard keeps the write from re-triggering the system's own
  `Changed` filter every frame.
- Finding 2: now `try_insert` with `despawn_related` +
  `Children::spawn(SpawnWith(..))`, commented with the widget-zoo constraint.
- Findings 3-6 applied as suggested.
- New test `a_display_only_track_keeps_its_fraction_across_a_flip`.

Re-verified after the fixes: nova_ui **19** passed, nova_menu **74** passed,
`check --all-targets` clean with no new warnings, `fmt --check` clean, and the
widget_zoo Xvfb capture re-run renders identically in both skins.

Round 2 verdict: **REQUEST_CHANGES** (1 MAJOR, 1 MINOR, 3 NIT).

## Round 2 - out-of-context reviewer, fresh context (2026-07-30)

The reviewer re-verified round 1's two load-bearing conclusions against the
EDITED code rather than re-reading them, including a probe that flips the skin
and changes the value in the same frame (the rebuild reads a one-frame-old
marker, but `sync_slider_tracks` repaints in the same frame, so nothing stale
renders). It also confirmed the display-only design is consistent with
`rebuilt-view-writes-go-to-state-not-the-entity` (state on the surviving parent,
not the discarded children).

### 1. MAJOR - the round-1 `try_insert` fix protected the wrong command

`try_insert` is silenced, but the `despawn_related::<Children>()` one call
earlier queues through the DEFAULT handler. So the guard bought nothing: the
pair still hard-crashes in exactly the scenario its own comment described.

I verified the underlying fact directly rather than taking it on trust: queueing
`despawn_related::<Children>()` at an already-despawned entity under
`FallbackErrorHandler(bevy::ecs::error::panic)` panics in bevy_ecs's error
handler. That handler is not hypothetical - the game installs it under
`BCS_AUTOPILOT` (examples/ui/menu_newgame.rs:57), which is how the smoke suite
runs, so this is a crash in CI's configuration, not just in theory.

Fixed by extracting the rebuild into `rebuild_slider_track_children(fraction,
skin) -> impl EntityCommand` and queueing it as ONE `queue_silenced` command, so
despawn and respawn are atomic and both no-op on a dead entity.

**Not covered by a regression test, deliberately, and this is recorded in the
function's doc comment.** Two attempts were made and both were thrown away as
false pins:

- An app-level test with a despawning system ordered `.before` the reconciler
  passes with the BROKEN code too. The `.before` edge auto-inserts an
  `ApplyDeferred`, so the despawn lands before the reconciler runs, the
  reconciler no longer matches the entity, and it queues nothing at all. The
  real race needs NO ordering edge (it is a system-index accident), which an App
  cannot express deterministically.
- A command-level test that calls `rebuild_slider_track_children` through
  `queue_silenced` is tautological: being one silenced command IS the fix, so it
  cannot fail for the old shape.

The guard is therefore the shape of the function, and the doc comment says so.

### 2. MINOR - `sync_slider_tracks` read `SliderRange` but did not watch it

The system newly derives its fraction from `SliderRange::thumb_position`, but
its run filter was still `Or<(Changed<SliderValue>, Changed<Children>)>`, so a
widened range with a steady value never repainted - the value had not changed,
but what it MEANT had. Filter now includes `Changed<SliderRange>`. Pinned by
`a_changed_range_moves_the_fill_even_at_a_steady_value`, which was falsified
against the old filter (goes red).

### 3. NIT - `SliderTrackSkin.0` was a second source of truth

For a value-bearing track the marker was a derived cache that could only ever go
stale. `reconcile_slider_track_skins` now prefers the live `SliderValue` and
falls back to the seed only for a display-only track, which is the case the
field exists for.

### 4. NIT - `pub f32` was wider than needed

Field is now private with a crate-internal `seed()` accessor; the type stays in
the prelude for `With<..>` filters.

### 5. NIT - the ordering comment sat above the wrong system

Moved directly above `reconcile_slider_track_skins`.

### Re-verified after round 2's fixes

nova_ui **20** passed, nova_menu **74** passed, `check --all-targets` clean with
no new warnings, `fmt --check` clean, widget_zoo Xvfb capture renders
identically in both skins.

## Round 3 - out-of-context reviewer, fresh context (2026-07-30)

Verdict: **REQUEST_CHANGES** (2 MAJOR, 2 MINOR, 3 NIT). Both MAJORs were defects
in round 2's own fixes and in the record written about them.

### 1. MAJOR - "no honest regression test exists" was wrong

The doc comment on `rebuild_slider_track_children` argued the despawn race could
not be pinned: an app-level test cannot force it because the `.before` edge
auto-inserts an `ApplyDeferred`, and a command-level test is tautological. That
reasoning stopped one step short - **the auto-insert is a schedule build setting
you can turn off**. With `auto_insert_apply_deferred: false`, the despawner and
the reconciler share a block with no flush between them, so the reconciler still
SEES the entity and queues its rebuild while the despawn lands first at the
end-of-schedule flush. The race becomes deterministic.

Added as `rebuild_survives_a_track_despawned_in_the_same_frame`, under the
`FallbackErrorHandler(panic)` the game installs for `BCS_AUTOPILOT`. Falsified
against the old split shape: it panics in bevy_ecs's error handler with
`Entity despawned ... invalid`, out of command-buffer application. The doc
comment now points at the test instead of claiming untestability.

### 2. MAJOR - the recorded lesson was factually wrong

The retro's `check-all-targets-cannot-see-lib-unit-tests` claimed
`cargo check --all-targets` does not compile a lib's `#[cfg(test)]` unit tests.
That is false, and it was about to enter the durable ledger. The symptom was
real; the CAUSE was invented from a plausible story around it.

Verified independently, including the counter-experiment: breaking an import in
a MEMBER crate's test module leaves a bare `cargo check --all-targets` green but
fails `--workspace --all-targets` and `-p nova_menu --all-targets`; breaking the
same import in the ROOT package's test module fails the BARE invocation too. So
`--all-targets` sees `cfg(test)` fine - the variable is package SCOPE. This
repo's root Cargo.toml is a PACKAGE with deliberately no `default-members`
(documented at Cargo.toml:274), so a bare invocation never reaches member
crates' test targets. Corrected in TASK.md and RETRO.md.

### 3. MINOR - Close-out described code not in the diff

It still credited the deleted `SpawnChild` trait, undercounted the tests (3 vs
6), and quoted stale run totals. Rewritten against the final diff, and it now
covers the three load-bearing round-2 changes it had omitted.

### 4. MINOR - the retro recorded round 2 as APPROVE and omitted its MAJOR

Rewritten: round 2 was REQUEST_CHANGES, and the "guarded the wrong call in the
chain" failure is now its own entry - the most instructive miss of the cycle.

### 5-7. NITs

Stale `sync_slider_meters` doc references fixed. The repo-wide sweep found a
FOURTH one in `examples/ui/widget_zoo.rs:414` that three review rounds had
missed - `grep-the-old-symbol-after-a-rename`, earning its recurrence. The zoo's
slider comment crediting `on_slider_change` for lighting the bars is corrected,
and `seed()`'s doc now says what a frozen fill on a slider-backed track means (a
visual spawned on the wrong entity).

## Round 4 - out-of-context confirmation pass (2026-07-30)

Verdict: **APPROVE** (1 NIT, since applied).

Scoped to confirming round 3's five items, all re-derived by experiment rather
than by reading:

- The despawn-race pin is real: reverting to the split shape turns 21 passed
  into 20 passed + 1 FAILED, the failure being exactly that test, by panic out
  of command-buffer application. No other test moves, so the pin is specific.
- The corrected `--all-targets` explanation is true, confirmed with the
  three-way experiment above including the root-package counter-case that
  disproves the old cause. The `Cargo.toml:274` citation is accurate to the line.
- Close-out and RETRO prose match the diff: `#[test]` counts audited at 6 new in
  nova_ui (the 7th added `fn` is the rename, correctly not counted) and 1 new in
  nova_menu; every symbol named in the prose exists with the shape described.
- Stale-symbol sweep clean: zero hits for `sync_slider_meters` / `SpawnChild`
  across crates/, examples/, tests/, web/.
- Gates: nova_ui 21 passed, nova_menu 74 passed, `cargo check --workspace
  --all-targets` clean, `cargo fmt --all --check` clean.

The NIT: this file's header still read "pending round 3", and the retro asserted
"then APPROVE" before this round had issued one - prose written from intent
rather than from what happened, the very convention this repo names. Both fixed.

VERDICT: APPROVE
