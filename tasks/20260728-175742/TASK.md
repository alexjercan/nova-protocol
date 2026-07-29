# HUD restyle + on-screen text reduction (icon dock)

- STATUS: OPEN
- PRIORITY: 36
- TAGS: v0.9.0,ui,hud

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

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

- [ ] Restyle the chip family to demo 2's base: dark translucent fill
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
- [ ] Status bar: restyle in place. Conscious divergence: demo 2 drew it
      top-LEFT; the game's status bar + objective hint live top-RIGHT -
      keep the game placement, adopt only the visual language (recorded
      here so review does not flag it).
- [ ] Key-glyph mapping: a `KeyCode -> Handle<Image>` lookup for
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
- [ ] Replace the 7-row cluster (keybind_hints.rs) with the icon-chip DOCK,
      bottom-center per demo 2: one chip per verb from `FlightVerbHints`
      (STOP GOTO ORBIT CANCEL RADAR COMPONENT RCS - the REAL set; demo's
      six were illustrative), each = glyph ImageNode + short verb label.
      Three visual states from availability: dim (~0.45 alpha, muted
      border), available (full phosphor), hot (inverted fill + slight
      scale) - hot wiring is minimal here (static mapping), 175747 drives
      it from live situations. PRESERVE the scenario HintEmphasis
      pulse machinery (HintEmphasisSet/Clear actions must still visibly
      pulse a dock chip; the tutorial scenarios depend on it).
- [ ] Anchored cues: the projected `[O]` orbit / `[G]` goto cues
      (keybind_hints.rs) become glyph chips (demo 2 `.cue` shape); the
      objective hint's `TAB` text (objective_hint.rs:117 area) becomes the
      Tab glyph.
- [ ] Text reduction: slim objective/beacon chips to glyph + name + range
      (drop any extra prose), comms cards capped tight per demo (~320px,
      speaker line + message only). Verify nothing on the flight screen
      duplicates NOVA OS detail (ship/objectives/map/log live in the
      computer; the map confirms this already holds - re-check after
      restyle).
- [ ] Ammo readout: restyle pips to demo 2's group shape (label + pip row
      per weapon - already per-weapon in ammo_readout.rs) and add the
      low-ammo state (amber pips + warn pulse on near-empty groups).
- [ ] Docs sweep (keep-docs-in-sync): wiki hud.md (keybind cluster ->
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
