# Review: HUD restyle + on-screen text reduction (icon dock)

- TASK: 20260728-175742
- BRANCH: feat/hud-icon-dock

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/keybind_dock.rs:237 - the
  spawn-time keycap lookup is keyed by the VERB name
  (`glyphs.get(DOCK_VERBS[index])`, i.e. `"STOP"`, `"GOTO"`) instead of the
  verb's key label, so it always resolves `None` and the doc's claim that the
  chips "render a picture from their first frame" is false - the picture only
  ever comes from `update_dock`. Either drop the argument and spawn with
  `Handle::default()` (documenting that `update_dock` owns the image), or
  resolve the default key label per verb here so the claim holds.
  - Response: CONFIRMED and fixed the first way. `keybind_dock_hud()` now takes
    no arguments and spawns an empty `ImageNode`; its doc says `update_dock`
    owns the image and that the `Added<DockChip>` gate fills it on the same
    frame the chips appear. The second option was rejected on purpose: the
    binding is only knowable from the live `FlightVerbHints`, so resolving art
    at spawn would have meant hardcoding a guess. Re-eyeballed on the GPU after
    the change - the keycaps still render on the capture frame.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/keybind_dock.rs:344 - the
  anchored cues hardcode their keycaps at spawn (`cue("ORBIT", "O")`,
  `cue("GOTO", "G")`), a regression in generality from the code they replace:
  master's `drive_orbit_cue`/`drive_goto_cue` rendered
  `format!("[{}] ORBIT", hints.orbit.key)` from the live binding every update.
  The cue also has no key-text fallback child, so a cue spawned before the glyph
  collection finishes loading silently loses its key forever. Drive the cue
  glyph (and a text fallback) from `hints.orbit.key`/`hints.goto.key` inside the
  two drive systems, as the dock does.
  - Response: CONFIRMED - the real regression here is the silent-loss one, and
    both are fixed. The cues now carry a `ChipKeyText` fallback child, and both
    drive systems read `hints.<verb>.key` and repaint through a new shared
    `paint_key_visual` helper (generic over the two systems' query filters), so
    the dock and the cues cannot drift apart. Their change-detection gate now
    also wakes on `NovaHudAssets` changing, which is what fixes the
    spawned-before-the-collection-landed case.
    `orbit_cue_is_a_keycap_chip_following_the_resolvers_offer` was extended to
    prove both: it feeds the resource a REMAPPED orbit key and asserts the
    keycap follows, then an unmapped `F13` and asserts the text fallback shows.
- [x] R1.3 (MINOR) crates/nova_gameplay/src/hud/keybind_dock.rs:41 - `GLYPH_PX`'s
  doc (and TASK.md's delivery note "the wide Ctrl/Shift keycaps were squashed by
  a fixed square glyph box ... width auto from the image's aspect") states a
  cause that is not true of the shipped art: every PNG under
  `assets/input-prompts/keyboard/Alt/` is square (128x128, a few 512x512), so
  height-only sizing is exactly equivalent to the square box it replaced.
  Correct the comment to whatever the eyeball actually showed (e.g. the cap is
  drawn inside transparent padding), and note that `objective_hint.rs:155-156`
  still sets a fixed square box for the TAB keycap, which contradicts the stated
  rule as written.
  - Response: CONFIRMED by measurement (`magick identify` on the five keycaps
    the dock uses: all 128x128). The height-only change was a no-op and the
    apparent improvement was a crop artifact - the two eyeball crops were
    resized 150% and 200%. Reverted to an explicit square box in both the dock
    and the cue, and the comment now says the true thing: the art is a square
    canvas with the wide caps drawn smaller INSIDE it, so a wide cap's legend is
    small by construction and the verb word beside it carries the meaning.
    `objective_hint`'s square TAB box is now consistent, not contradictory. The
    stale delivery note in TASK.md is corrected too.
- [x] R1.4 (MINOR) crates/nova_gameplay/src/hud/key_glyphs.rs:144 -
  `every_bound_key_maps_to_an_existing_glyph_asset` reimplements the production
  label function as a local closure instead of calling
  `input::player::keyboard_label`, so a change to the real labelling would leave
  this DoD test green while the runtime lookup missed the table. Make
  `keyboard_label` `pub(crate)` and call it from the test.
  - Response: CONFIRMED and done exactly as suggested - `keyboard_label` is
    `pub(crate)` and the test calls it, so the DoD proof is now anchored to the
    production labeller.
- [x] R1.5 (MINOR) crates/nova_gameplay/src/hud/screen_indicator.rs:69 - the new
  `ScreenIndicatorSize::Content` branch (skip the width/height write, centre on
  `ComputedNode::size() * inverse_scale_factor()`) ships with no test in this
  module, even though every restyled chip now depends on it; a regression that
  re-enabled the width/height write would only show up as a squashed chip on a
  screenshot. Add a small test that a `Content` indicator's `Node.width/height`
  stay `Val::Auto` after `update_screen_indicators` while `left/top` are centred
  on the computed size.
  - Response: CONFIRMED and added -
    `content_sizing_positions_without_writing_the_box` spawns a `Content`
    indicator with a stand-in `ComputedNode` and asserts `width`/`height` stay
    `Val::Auto` while `left`/`top` land on centre-minus-half-the-computed-size.
- [x] R1.6 (MINOR) crates/nova_assets/src/lib.rs:1388 - the new test's doc
  comment was inserted INSIDE the existing
  `ui_sfx_collection_matches_ui_sfx_files` doc block, so that test is now
  undocumented and `key_glyph_collection_matches_mapping_table`'s rustdoc opens
  with five lines of UI-SFX prose. Split the block: leave the UI-SFX paragraph
  on the UI-SFX test and keep only the "DoD 2, half two" paragraph on the new
  one.
  - Response: CONFIRMED and split; each test carries its own doc again.
- [x] R1.7 (MINOR) crates/nova_scenario/src/actions.rs:953 - doc-sweep miss:
  `HintEmphasisSet`/`HintEmphasisClear` rustdoc still says "keybind-hint row"
  and "one of `ROW_VERBS`" at lines 953, 958 and 995, but `ROW_VERBS` was
  deleted by this branch (it is `DOCK_VERBS` now). Rename in all three places,
  matching the wiki edits already made.
  - Response: CONFIRMED - a real miss, the sweep only covered `web/` and the
    HUD crate. Renamed in all three places; `grep -rn "ROW_VERBS" crates/` is
    now 0 (the surviving `keybind-hints` hits are the SPIKE FILENAME
    `docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md`, which
    is history and correctly untouched).
- [x] R1.8 (NIT) crates/nova_gameplay/src/hud/comms_panel.rs:93 - same
  doc-comment displacement as R1.6: `/// Square speaker icon size inside a comms
  card.` now documents the new `COMMS_BODY` colour, and `COMMS_ICON_SIZE_PX` is
  left undocumented. Move the line back down onto the const it describes.
  - Response: CONFIRMED and moved.

The reviewer ran the DoD proofs independently: `cargo test -p nova_gameplay
--lib hud::` (258 passed), `keybind_dock::` (10, incl. DoD 1 and DoD 3),
`key_glyphs::` (3, incl. DoD 2 half one), `-p nova_assets --lib key_glyph` (DoD
2 half two), `-p nova_ui --lib hud::` (3), `cargo fmt --check` clean, the DoD 4
greps at 0 hits, and DoD 5's `hud_range` rig green on a real GPU under Xvfb. It
independently confirmed that the pre-existing `nova_assets`
`an_early_derelict_kill_skips_to_the_fight` red belongs to master and not to
this branch (filed as 20260729-140945), and judged all four declared
divergences from demo 2 justified rather than dropped scope.

After the responses above: `cargo test -p nova_gameplay --lib` 730 passed / 0
failed, `screen_indicator` 25 passed, `keybind_dock` 10 passed, `cargo check
--workspace --all-targets` clean, `cargo fmt --check` clean, and the GPU eyeball
re-run after the spawn-path change (`screenshot_combat` under Xvfb) shows the
dock's keycaps and the anchored GOTO cue unchanged - the `Added` gate does fill
them on the frame they spawn. The committed web captures were regenerated from
that final run.

Pending user checks: DoD 6, the owner playtest verdict ("on-screen text density
dropped and the HUD reads in the phosphor language"), is a `manual:` item and
stays open for the flow Finish. The reviewer also suggests the owner glance at
`web/src/assets/wiki-hud.png` and `feature-hud.png` alongside that playtest.
