# Retro: HUD restyle + on-screen text reduction (icon dock)

- TASK: 20260728-175742
- BRANCH: feat/hud-icon-dock
- ROUNDS: 1 (APPROVE, 8 MINOR/NIT findings, all addressed)

## What went well

- **The demo was a real spec.** `examples/ui/hud_rework_poc.html` gave exact
  fills, borders, radii and per-tone colours, so `nova_ui::hud` could be written
  first and every site then just consumed it. Building the CHIP LANGUAGE before
  touching any site is what kept eleven restyled modules in one family instead
  of eleven near-misses.
- **Copying the established preload pattern paid off immediately.** The UI-SFX
  mapped collection from 20260729-000956 transferred verbatim: explicit
  `paths(...)`, a `#[cfg(test)]` mirror const, and a test pinning the mirror
  against the owning table. The keycap pipeline had no design work left in it -
  only the decision of what to key the map by.
- **The GPU eyeball earned its place in the DoD.** Two defects were invisible to
  every headless test and obvious in one screenshot: the lock readout wrapping
  into ragged rows inside its new chip, and the ship-anchored speed chip landing
  ON the new bottom-centre dock. Neither is expressible as an entity-tree
  assertion; both are one glance.
- **The A/B stash discipline caught three inherited reds.** Two of the three
  failures met on the way were pre-existing on master
  (`objective_hint_shows_the_nova_crt_star_icon`, `hud_range`'s unit-naive
  parser, and the shakedown rehearsal). Running `git stash && cargo test` before
  reaching for the debugger meant zero time spent blaming this branch's diff.

## What went wrong

- **A "fix" that was a no-op, shipped with a confident wrong reason.** Wide
  Ctrl/Shift keycaps looked squashed, so the glyph box moved to height-only
  sizing "so bevy_ui measures width from the image's aspect". Every keycap PNG
  is a square 128x128 canvas, so the change did nothing; the apparent
  improvement was a crop artifact - the before/after crops were resized 150% and
  200%. The reviewer caught it by measuring the assets. Two failures stacked
  here: comparing two images at different zoom levels, and writing a mechanism
  claim into a comment without checking the mechanism.
- **Spawn-time asset lookups keyed by the wrong string, unnoticed by a green
  test.** `keybind_dock_hud` looked keycaps up by VERB name (`"STOP"`) instead
  of key label (`"X"`), so it always resolved `None`. Everything still worked
  because `update_dock` refills the image on the `Added` frame - the dead branch
  was invisible on screen AND under a passing DoD test that asserted the final
  state. A test asserting the OUTCOME cannot see a redundant path that produces
  the same outcome.
- **The doc-surface sweep stopped at the crate boundary.** Renaming
  `ROW_VERBS -> DOCK_VERBS` was swept through `crates/nova_gameplay` and
  `web/src/wiki`, but `nova_scenario`'s rustdoc - the OTHER crate that talks
  about these verbs, and the one scenario authors read - still named the deleted
  symbol. The grep was scoped to where the code changed, not to where the name
  is spoken.
- **Two doc comments got orphaned by insert-before-anchor edits.** Inserting a
  new item just above an existing one, anchored on the existing item's
  ATTRIBUTE rather than its doc block, silently reassigns that doc block to the
  new item. It happened twice in one branch (`nova_assets` tests,
  `comms_panel` consts) and both times the code compiled and tested clean.

## Lessons

- `compare-crops-at-one-zoom` - a before/after visual comparison must be
  rendered at IDENTICAL crop and scale, or the difference you see is the
  resize. Here two eyeball crops at 150% and 200% "proved" a glyph-sizing fix
  that was provably a no-op (all the art is square). Cheap guard: put the two
  crops through the same `magick` command and, when the claim is about an
  asset's shape, `identify` the asset instead of eyeballing it.
- `outcome-test-hides-a-dead-redundant-path` - when a value is written by BOTH a
  spawn-time initializer and a per-frame updater, a test that asserts the final
  state passes even if the initializer is completely broken. Assert the
  initializer's own output before the updater runs, or delete the redundancy
  (this branch deleted it: the updater owns the image, and its `Added` gate
  makes that the same frame).
- `sweep-a-rename-where-the-NAME-is-spoken` - scope a rename sweep to every
  place the symbol is NAMED, not to the crates whose code changed. A
  cross-crate vocabulary (`ROW_VERBS`/`DOCK_VERBS`, spoken by nova_scenario's
  authoring docs) survives a crate-local grep untouched. Sibling of
  `sweep-content-repo-wide-not-just-assets`.
- `anchor-doc-inserts-above-the-doc-block` - inserting an item above an existing
  one must anchor on that item's DOC COMMENT, not its `#[test]`/`const` line, or
  the insert lands between the doc and its owner and silently steals it.
  Compiles and tests clean; only a reader notices. Instance of
  `anchor-edits-in-the-right-scope`.
- `stash-ab-before-blaming-your-diff` (positive, reinforces) - three test
  failures met during this task, three `git stash && cargo test` A/Bs, two
  inherited reds correctly attributed to master and one of them filed as its own
  task. This should stay the reflex on any red met mid-branch.

## Follow-ups

- **20260729-140945** (filed): `nova_assets`
  `an_early_derelict_kill_skips_to_the_fight` is red on master - the rehearsal
  walk no longer leaves the shakedown mid-lesson with RADAR emphasized, so the
  out-of-order-kill skip path is currently unproven.
- **20260728-175747** (already planned) layers the situational emphasis on the
  `DockChipState::Hot` component this task wired and proved; the seam is ready.
- **20260710-231927** (backlog) keeps the remapping/gamepad half: a runtime
  rebind or a gamepad glyph is dynamic content and takes the `server.load`
  exception, falling back to this task's text chip until then.
- Open observation for playtest, not filed: the dock sits over the ship's hull
  at the bottom centre when the chase camera is close. It is readable (the
  chips have their own slab) and it is what demo 2 does, but if the owner finds
  it busy the fix is a dock lift, not a re-home.
