# Wide keycaps (Tab/Shift/Space) render at their art aspect, height-constrained

- STATUS: CLOSED
- PRIORITY: 46
- TAGS: v0.9.0,ui,hud,feedback
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

Owner playtest 2026-07-30 (feedback wave on the v0.9.0 UI rework):

> "Tab", "Shift", "Ctrl" buttons are really small and barely visible what they
> say, maybe we can make them larger or "clearer" somehow -> some buttons have
> specific widths, and I think we are squeezing them to same width, we should
> keep the width:height ratio, and constraint the height only

## Understanding (2026-07-30) - measured, not assumed

Every glyph under `assets/input-prompts/keyboard/Alt/` is a 128x128 canvas (97
files; verified). The wide caps are drawn WIDE INSIDE that square canvas with
transparent bands above and below. Measured opaque bounding boxes:

| file | bbox | aspect (w:h) |
|---|---|---|
| `T_A_Key_Alt.png` | 96x104 | 0.92 |
| `T_Enter_Key_Alt.png` | 100x108 | 0.93 |
| `T_Tab_Key_Alt.png` | 112x74 | 1.51 |
| `T_Shift_Key_Alt.png` | 112x74 | 1.51 |
| `T_CapsLock_Key_Alt.png` | 112x74 | 1.51 |
| `T_Space_Key_Alt.png` | 128x68 | 1.88 |

The HUD draws every glyph into a SQUARE box - `GLYPH_PX: f32 = 22.0` for dock
chips and `CUE_GLYPH_PX: f32 = 20.0` for the anchored cues, both set as
`width == height` in `crates/nova_gameplay/src/hud/keybind_dock.rs` (the
objective stack's TAB footer, `hud/objective_stack.rs`, does the same). So a
Tab cap renders 22 px wide but only ~12.7 px TALL, and its legend shrinks with
it. The current `GLYPH_PX` doc calls this "the art, not a squash" - the
measurements above show the art does carry a real aspect and the square box is
what throws it away. That doc paragraph is wrong and must be corrected.

The owner's rule: constrain the cap HEIGHT, let the width follow the aspect.
A Tab cap then draws 22 px tall x ~33 px wide with a full-size legend.

## Approach

Trim-at-load rather than a hand-maintained aspect table: scan each loaded glyph
image's alpha once, store `(sub-rect, aspect)` keyed the same way
`GameAssets::key_glyphs` is keyed, then render with `ImageNode { rect }` = the
trimmed cap and `Node { height: GLYPH_PX, width: GLYPH_PX * aspect }`. A new or
replaced glyph then just works, and the vertical transparent bands stop eating
the cap's height everywhere at once (including the near-square caps, whose 0.92
bbox is currently padded to 1.0 too).

97 images x 128x128 is a trivial one-time scan and is wasm-safe (no new
dependency, no filesystem access - the images are already loaded via
`bevy_asset_loader`).

## Steps

- [x] Reproduce first: a rig asserting the rendered node for a WIDE key is
      wider than it is tall and that its cap fills the box height, alongside a
      near-square key that stays near-square. Watch it fail on today's square
      box; record the measured before-numbers in NOTES.md.
- [x] Compute the trimmed bbox + aspect per glyph once the glyph collection
      finishes loading, into a resource beside `NovaHudAssets::key_glyphs`.
      Handle the not-yet-loaded frame the same way the dock already handles it
      (`assets.is_changed()` re-paint).
- [x] Teach `paint_key_visual` to size from that resource: height pinned to the
      site's constant, width derived, `ImageNode.rect` set to the cap bbox.
      Cover all three sites - dock chips, anchored cues, objective-stack TAB
      footer - rather than only the dock.
- [x] Re-check the chip layouts a wider keycap now sits in: the dock row's
      `column_gap`/centring, the cue chips over the world, and the TAB footer.
      Adjust the constants if a wide cap now crowds its verb word.
- [x] Correct the `GLYPH_PX` doc paragraph (it currently asserts the wide caps
      are small by construction and that this is not a squash).
- [x] Sweep for any other square-boxed key glyph site before closing.
- [x] Screenshot and LOOK at a dock with a wide cap in it (ledger: eyeball the
      rendered output).

## Close-out (2026-07-30)

The cap's shape comes from an alpha TRIM at load, not a hand-maintained table,
and every site pins the HEIGHT (DECISION.md). `KeyGlyphs` now holds `KeyCap`s
instead of bare handles; `KeyCap::apply`/`node` is the single sizing path, and
`nova_assets::update_nova_hud_assets` scans the caps in
`OnEnter(GameAssetsStates::Processing)` where the collection is loaded.

- Fail-first: `wide_keycaps_render_at_their_art_aspect` measured `Vec2(22.0,
  22.0)` (aspect 1.000) against Tab/Ctrl's real 1.514 art; numbers and the
  per-cap before/after table are in NOTES.md.
- Layout re-check: no constant needed changing. The dock row is
  `justify_content: Center` with a 7 px gap and the chips are content-sized, so
  the wider caps just widen their own chips and the row re-centres; the cue
  chips hug their content (`ScreenIndicatorSize::Content`) and the TAB footer is
  a plain flex row. The eyeball crop confirms nothing crowds its verb word.
- Sweep: `rg -n 'GLYPH_PX|CUE_GLYPH_PX' crates/nova_gameplay/src/hud` shows no
  production site setting `width == height`; `rg -l 'key_glyphs|KEY_GLYPH_DIR'`
  finds no fourth keycap site outside the three fixed here.
- Eyeball: `shots/dock-before.png` vs `shots/dock-after.png`, same crop, same
  zoom. `Ctrl`/`Shift` go from smudges to legible.
- CHANGELOG's (unreleased) dock entry now states the art-proportion rule.
  Backlog 20260728-214929 carries an interim note that it inherits `KeyCap`.
- Inherited red, filed as 20260730-161545 and NOT caused here:
  `nova_assets ... an_early_derelict_kill_skips_to_the_fight` fails on the base
  commit too (verified by stashing).

## Definition of Done

1. A wide cap (Tab/Shift/Space) renders at its art aspect with the cap filling
   the box height, and a near-square cap is unchanged in feel (test: the
   aspect rig from step 1, which failed first).
2. All three keycap sites - dock, anchored cues, objective-stack TAB footer -
   go through one shared sizing path (cmd:
   `rg -n 'GLYPH_PX|CUE_GLYPH_PX' crates/nova_gameplay/src/hud`, reviewed: no
   site sets `width == height` by hand).
3. Every preloaded glyph resolves a trimmed bbox (test: a rig over the whole
   `key_glyphs` collection, so a future glyph with an unexpected canvas fails
   loudly instead of drawing a sliver).
4. A screenshot example shows the dock with a wide cap, eyeballed (cmd:
   `cargo test --test examples_smoke screenshots`).
5. Owner can read TAB / SHIFT / CTRL at a glance in game (manual).

## Notes

Sits under epic 20260728-175719 (UI rework). Backlog 20260728-214929 (adopt the
glyphs across the remaining surfaces: web key-UI, NOVA OS help, editor chips)
inherits whatever sizing path this task lands - note it there when this closes.
