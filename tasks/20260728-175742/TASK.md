# HUD restyle + on-screen text reduction (icon dock)

- PRIORITY: 36
- TAGS: v0.9.0, ui, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

The flight HUD chrome still speaks the old flat language and carries the
bulk of the on-screen text, most of it in the lower-left 7-row `[KEY] VERB`
cluster. Restyle the chrome to the phosphor chip language of demo 2
(`examples/ui/hud_rework_poc.html`) and replace the text cluster with the
contextual icon-chip dock using the relocated Alt key glyphs. This is the
HUD LOOK; the automatic show/emphasis BEHAVIOUR is the sibling
20260728-175747, layered on top after this lands. Folds the icon half of
backlog 20260710-231927 (remapping half stays there). HUD chrome is
phosphor-only per the spike's per-surface table - no hardware variant here.

## Steps (re-planned 2026-07-28 from the implemented demo 2 + HUD map)

- [x] Restyle the chip family to demo 2's base: dark translucent fill
      `rgba(2,14,8,~0.6)`, 1px phosphor border ~0.34 alpha, phosphor text,
      dim unit suffixes; semantic variants keep their family (amber
      objective/mode, red lock/threat, blue comms). Sites (from the HUD
      map): flight_status speed + mode chips, torpedo_target DST/CLS
      readout, maneuver_instruments destination/flip chips,
      edge_indicators labels, objective_markers chips, beacon_chips,
      readout.rs top strip, objective_hint block, target_inset frame +
      stat grid (TARGET/HOSTILE header, DST/CLS/KIND/HULL cells, hp bar
      per demo). Keep the velocity-direction shader and the target inset
      as-is functionally (owner KEEPs).
- [x] Status bar: restyle in place. Conscious divergence: demo 2 drew it
      top-LEFT; the game's status bar + objective hint live top-RIGHT -
      keep the game placement, adopt only the visual language (recorded
      here so review does not flag it).
- [x] Key-glyph mapping: a `KeyCode -> Handle<Image>` lookup for
      `assets/input-prompts/keyboard/Alt/` (from 20260728-233707), covering
      the live bindings: X stop, G goto, O orbit, Z cancel, Ctrl radar
      (file is `T_Crtl_Key_Alt.png`, upstream typo), `[`/`]` component
      (`T_Brackets_L/R_Key_Alt.png`), Shift RCS, Space burn, Tab NOVA OS,
      Tilde HUD level. The glyph HANDLES come from a `collection(mapped, typed)`
      glyph collection (keyed by `AssetFileStem`) added to a bevy_asset_loader
      collection with an explicit `paths(...)` list of exactly the mapping
      table's used glyphs - so the glyphs preload/load-gate like every other
      static asset (task 20260729-000956 established this pattern; a FOLDER
      collection is not usable because folder collections do not work on wasm).
      The mapping table owns that path list. Fall back to a text chip for any
      unmapped key so a rebind never breaks the dock; the remapping/gamepad
      follow-up (20260710-231927) may `server.load` a glyph for an unmapped or
      runtime-rebound key (the dynamic-content exception - it cannot sit behind
      the one-shot collection).
- [x] Replace the 7-row cluster (keybind_hints.rs) with the icon-chip DOCK,
      bottom-center per demo 2: one chip per verb from `FlightVerbHints`
      (STOP GOTO ORBIT CANCEL RADAR COMPONENT RCS - the REAL set; demo's
      six were illustrative), each = glyph ImageNode + short verb label.
      Three visual states from availability: dim (~0.45 alpha, muted
      border), available (full phosphor), hot (inverted fill + slight
      scale) - hot wiring is minimal here (static mapping), 175747 drives
      it from live situations. PRESERVE the scenario HintEmphasis
      pulse machinery (HintEmphasisSet/Clear actions must still visibly
      pulse a dock chip; the tutorial scenarios depend on it).
- [x] Anchored cues: the projected `[O]` orbit / `[G]` goto cues
      (keybind_hints.rs) become glyph chips (demo 2 `.cue` shape); the
      objective hint's `TAB` text (objective_hint.rs:117 area) becomes the
      Tab glyph.
- [x] Text reduction: slim objective/beacon chips to glyph + name + range
      (drop any extra prose), comms cards capped tight per demo (~320px,
      speaker line + message only). Verify nothing on the flight screen
      duplicates NOVA OS detail (ship/objectives/map/log live in the
      computer; the map confirms this already holds - re-check after
      restyle).
- [x] Ammo readout: restyle pips to demo 2's group shape (label + pip row
      per weapon - already per-weapon in ammo_readout.rs) and add the
      low-ammo state (amber pips + warn pulse on near-empty groups).
- [x] Docs sweep (keep-docs-in-sync): wiki hud.md (keybind cluster ->
      dock), targeting-radar.md / flight-autopilot.md if they show `[KEY]`
      hints, tutorial.html key mentions, CHANGELOG [Unreleased]; regenerate
      committed HUD captures (`scripts/gen-web-screenshots.py` set).

## Definition of Done (re-planned 2026-07-28)

1. test: `dock_renders_glyph_chip_per_verb_with_availability` - a live-tree
   test that FlightVerbHints availability drives per-verb chips with the
   right glyph asset path + state marker (fails if the dock is a no-op).
2. test: `every_bound_key_maps_to_an_existing_glyph_asset` - the mapping
   table resolves every live binding to a file that exists on disk under
   `assets/input-prompts/keyboard/Alt/` (pins the Crtl/Brackets filenames),
   AND every mapped glyph path appears in the glyph collection's explicit
   `paths(...)` list (pins that every used glyph is actually preloaded/gated,
   parallel to `ui_sfx_collection_matches_ui_sfx_files` in 20260729-000956).
3. test: scenario emphasis still lands - HintEmphasisSet on a verb marks
   its dock chip emphasized (migrated from the row-based test).
4. cmd: `grep -rn "hint_row\|\[KEY\]" crates/nova_gameplay/src/hud` prints 0
   hits for the old row cluster (exact symbols pinned at implementation
   start; recorded here with counts).
5. render eyeball: HUD captures per changed element (status bar, dock,
   chips, comms, readouts, target inset) via the screenshot_combat /
   hud_range rigs; reviewed.
6. manual: owner playtest verdict - on-screen text density dropped and the
   HUD reads in the phosphor language.

## Notes

- Demo 2 (`examples/ui/hud_rework_poc.html`) is the visual reference; its
  `.chip`/`.vchip`/`.tgt-zoom`/`.ammo` CSS is the spec. Real bindings from
  `crates/nova_gameplay/src/input/player.rs:240-266` override the demo's
  key choices (demo had Z STOP / X CANCEL; game is X stop / Z cancel).
- Glyph tinting: Alt keycaps are dark with white glyphs and read well on
  the phosphor HUD per the spike; dim state via ImageNode color alpha (no
  grayscale filter in Bevy UI - accept alpha-dimming).
- The NOVA OS affordance stays embedded in the objective hint per decision
  20260724-134312 (no separate HUD button; demo 2's bottom-right buttons
  were easter-egg-web affordances).
- Units are DONE (20260728-175731 landed 93032f53): all readouts already
  format via `nova_ui::units` - reuse it for any new text, never inline
  format.
- Depends on: 20260728-175734 (theme tokens), 20260728-233707 (glyphs in
  assets/). Lands before 20260728-175747.
- 2026-07-29 alignment (20260729-000956, static-asset preload): the key-glyph
  mapping resolves glyph HANDLES from a `collection(mapped, typed)` glyph
  collection with an explicit `paths(...)` list (keyed by `AssetFileStem`), not
  a lazy per-chip `server.load` - so the Alt glyphs preload/load-gate like the
  UI font, crt mark and UI SFX now do. The CLOSED 20260728-233707's `server.load`
  note stays as history; the correction lands here in the consumer plan. DoD
  test 2 extended to assert every mapped path is in the collection. Runtime
  rebinds/gamepad glyphs (20260710-231927) keep the `server.load` dynamic-content
  exception.

## 2026-07-29 update (post-175734 delivery, owner /flow)

175734 landed the NOVA OS tokens but KEPT the legacy navy/cyan `theme.rs`
consts (per its DECISION.md, owner call) so the HUD/editor kept compiling. This
task owns migrating the HUD-chrome surface off them:

- Migrate the ~23 `theme::{BG,PANEL,PANEL_RAISED,BORDER,BORDER_BRIGHT,CYAN,
  CYAN_BRIGHT,SELECTED_FILL}` references in `crates/nova_gameplay/src/hud/`
  onto the NOVA OS tokens (HUD is phosphor-only, so no skin toggle here). The
  `nova_ui::theme::semantic` accents (NAV/OBJECTIVE/THREAT/ALLY/...) are NOT
  legacy and stay as-is.
- Deletion ordering: the `LEGACY web palette (retiring)` block in `theme.rs` is
  DELETED by whichever of THIS task and 20260728-175738 lands SECOND (that task
  owns the editor + menu refs; deleting the block early breaks its build). If
  this lands first, leave the block; if second, delete it and prove the legacy
  consts are unreferenced (`grep -rn "theme::\(BG\|PANEL\|CYAN\|...\)" crates/`
  = 0).

## Progress (2026-07-29) - PARTIAL

Landed as a self-contained first commit (`refactor(hud): migrate HUD chrome off
legacy palette; delete LEGACY theme block`):

- DONE: migrated the 23 legacy navy/cyan `theme::*` refs in
  `crates/nova_gameplay/src/hud/` onto the NOVA OS tokens (comms `CYAN -> BLUE`;
  `TEXT_MUTED -> PHOSPHOR_DIM`). semantic accents untouched.
- DONE: deleted the `LEGACY web palette` block from `nova_ui::theme` (this task
  landed second after 175738, so nothing referenced it). Navy/cyan is now fully
  retired from the game: `grep` for the legacy consts across `crates/` = 0. This
  satisfies the epic's Done-Means-3 palette goal (menus/editor/HUD no longer show
  the flat navy/cyan theme) at the token level. nova_ui + nova_gameplay compile;
  nova_gameplay tests compile.

REMAINING (the bulk of this task's own DoD - NOT yet done, task stays OPEN):

- The phosphor CHIP-family restyle across the HUD sites (flight_status, target
  readouts, edge/objective/beacon chips, target inset, status bar) - Step 1/2.
- The key-glyph asset pipeline: a `collection(mapped, typed)` glyph collection
  with an explicit `paths(...)` list + a `KeyCode -> Handle<Image>` mapping table
  (Step 3), following the 20260729-000956 preload pattern.
- The icon-chip DOCK replacing the 755-line `keybind_hints.rs` 7-row cluster,
  with the 3 availability states + preserved `HintEmphasis` pulse (Step 4), the
  anchored cue glyphs (Step 5), text reduction (Step 6), ammo low-state (Step 7).
- DoD tests 1-3 (dock renders per verb, every-bound-key-maps-to-a-glyph,
  emphasis-still-lands), the render eyeball (5) + docs/screenshots (Step 8).

This remaining scope is a large, GPU-eyeball-dependent effort (a new asset
pipeline + a full HUD component rewrite) best taken as its own focused cycle.

## Delivery (2026-07-29) - the remaining scope, complete

The chip family, the key-glyph pipeline, the icon dock, the cue/TAB keycaps,
the text reduction and the ammo warn state all landed in one pass on
`feat/hud-icon-dock`.

### What was built

- `nova_ui::hud` (NEW): the chip language in one place - `CHIP_FILL`,
  `CHIP_RADIUS`, `ChipTone{Phosphor,Amber,Threat,Comms}` (text/unit/border/fill
  per tone) and the `chip_node()`/`chip_paint()`/`text_chip()`/`quiet_chip()`
  builders. Phosphor-only, no `UiSkin` toggle (per the spike's per-surface
  table). 3 tests pin the family invariants.
- `nova_gameplay::hud::key_glyphs` (NEW): the label -> keycap mapping
  (`KEY_GLYPH_FILES`, 18 labels over 13 distinct files), `key_glyph_stem`,
  `key_glyph_asset_paths()` and the `KeyGlyphs` label-keyed handle lookup
  published on `NovaHudAssets`. Keyed by the DISPLAY LABEL the hints already
  carry (`"X"`, `"CTRL"`, `"SCROLL"`), which is what `FlightVerbHints` gives -
  a `KeyCode`-keyed table could not express the wheel/modifier pseudo-labels.
- `nova_assets`: `GameAssets::key_glyphs`, a `collection(mapped, typed)` with
  an explicit 13-path `paths(...)` list (no folder collection - they do not
  work on wasm), fanned out to the label-keyed lookup in
  `update_nova_hud_assets`. The glyphs now preload and load-gate like the UI
  font, CRT mark and UI SFX.
- `hud/keybind_dock.rs` (REPLACES `keybind_hints.rs`): the bottom-centre dock,
  one chip per `DOCK_VERBS` entry = keycap `ImageNode` + verb word, three
  states (`DockChipState::{Dim,Available,Hot}`) driven from availability. The
  scenario `HintEmphasis` spotlight is UNCHANGED in name, API and verb
  vocabulary (nova_scenario, the shipped scenario RON and the wiki all keep
  working) - it now pulses a chip's border + label instead of a text row.
  An unmapped key falls back to a text chip.
- `screen_indicator`: new `ScreenIndicatorSize::Content` (position-only, hug
  the laid-out box) and `screen_indicator_node(config, node)` for an indicator
  that is ALSO a styled box - Bevy refuses two `Node`s in one bundle, which is
  what a bordered chip on an indicator needs.
- Restyled onto the family: flight_status speed (amber mode) chips,
  torpedo_target lock readout (threat), maneuver destination/flip/radius,
  beacon chips, objective marker chips (amber), readout strip rows, comms cards
  (blue, capped 420 -> 320 px), target-inset frame + caption header.
- objective_hint: the `TAB` word becomes the Tab keycap (text fallback kept).
- ammo_readout: the low-ammo warn state - at or below 1/4 capacity a group goes
  amber and breathes (~1.1 Hz), suppressed while it is mid-reload.

### Conscious divergences from demo 2 (so review does not flag them)

1. Status bar placement: the demo drew it top-LEFT; the game's bar + objective
   hint live top-RIGHT and stay there (already recorded in Step 2).
2. `edge_indicators` labels are NOT chipped. Demo 2's own `.edge` rule is
   `border: 0; background: none` - an edge arrow is a pointer, not a readout -
   and its label already carries the semantic (threat/nav) colour. Chipping it
   would have ADDED chrome in a text-reduction pass.
3. The target inset did NOT grow the demo's DST/CLS/KIND/HULL stat grid. That
   data is already on screen at the reticle (`torpedo_target`'s lock readout,
   demo 2's own `.lock-read`); duplicating it into the inset would have added
   four lines of text in the pass whose GOAL is less text. The inset took the
   frame + header-cell restyle only; the owner KEEP on its function holds.
4. `SPEED_CHIP_OFFSET` moved from `(120, 0)` to `(120, -90)` (mode chip with
   it). The render eyeball showed the ship-anchored speed chip sitting ON the
   new bottom-centre dock - the chase camera parks the ship low-centre, which
   is exactly where the dock is. This is demo 2's `.speed` band expressed as a
   ship-relative offset.

### DoD status

1. `dock_renders_glyph_chip_per_verb_with_availability` - PASS (live-tree: one
   chip per verb, each carrying the real keycap ASSET PATH its binding maps to,
   plus per-verb state; engaging a maneuver flips CANCEL to `Hot` and repaints
   it).
2. `every_bound_key_maps_to_an_existing_glyph_asset` (nova_gameplay) - PASS,
   walking the REAL `flight_rig_reserved_sources()` and pinning the upstream
   `T_Crtl_Key_Alt` typo + `T_Brackets_L/R`; plus
   `key_glyph_collection_matches_mapping_table` (nova_assets) - PASS, pinning
   that every mapped path is in the preload collection (the
   `ui_sfx_collection_matches_ui_sfx_files` parallel from 20260729-000956).
3. `scenario_emphasis_marks_the_dock_chip` - PASS (migrated from the row test;
   set -> gold chip, clear -> base restored, quiet frames hold), plus
   `rig_despawn_mid_pulse_restores_the_base_paint`.
4. `grep -rn "hint_row\|\[KEY\]" crates/nova_gameplay/src/hud` = **0 hits**
   (was 38 across `hint_row|HintRow|ROW_VERBS|hint_cluster|[KEY]` at
   implementation start). `grep -rn "keybind_hints" crates/` = 0.
5. Render eyeball - DONE on the real GPU (Xvfb :99, `screenshot_combat`,
   `screenshot_orbit`, `screenshot_reel` + `hud_range` PASS). Two defects were
   found by the eyeball and fixed: the lock readout wrapped into ragged rows
   inside its new chip (added `LineBreak::NoWrap`), and the ship-anchored speed
   chip landed ON the new bottom-centre dock (see divergence 4). The committed
   web captures were regenerated (`scripts/gen-web-screenshots.py`): 15 assets
   updated.

   Correction (review R1.3): a third "fix" in this pass - switching the keycap
   box to height-only sizing so wide caps could keep their aspect - was a NO-OP
   and its rationale was wrong. Every keycap PNG is a square 128x128 canvas
   (`magick identify`), with the wide caps drawn smaller inside it; the apparent
   improvement was a crop artifact (the two eyeball crops were resized 150% vs
   200%). Reverted to an explicit square box, with the true reason recorded at
   `GLYPH_PX`.
6. Owner playtest verdict - PENDING (manual).

### Inherited reds met on the way (both A/B-confirmed against master)

- `examples/ui/hud_range.rs` asserted a raw-metre distance against a readout
  that has printed `1.50 km` since the units task (20260728-175731). FIXED
  here as merge integration: `readout_value` now reads the unit suffix and
  converts back through `nova_ui::units::METRES_PER_UNIT`.
- `objective_hint_shows_the_nova_crt_star_icon` had no `NovaHudAssets` in its
  rig, so the hint took its no-assets fallback and spawned no icon. FIXED here
  (the rig now supplies the resource) and extended to assert the new TAB
  keycap.
- `nova_assets` `an_early_derelict_kill_skips_to_the_fight` is red on master
  and untouched by this work; filed as **20260729-140945**.

### Test evidence

nova_ui 15 passed; nova_gameplay --lib 729 passed (hud:: 258, keybind_dock 10,
key_glyphs 3, ammo_readout 14); nova_scenario --lib 145 passed; nova_assets
--lib 95 passed / 1 pre-existing failure (20260729-140945);
`cargo test --test examples_smoke ui` 1 passed; `cargo check --workspace
--all-targets` clean; `cargo fmt --check` clean. Full suite + clippy left to CI
per AGENTS.md.
