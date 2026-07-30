# Wide keycaps (Tab/Shift/Space) render at their art aspect, height-constrained

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.9.0,ui,hud,feedback

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

- [ ] Reproduce first: a rig asserting the rendered node for a WIDE key is
      wider than it is tall and that its cap fills the box height, alongside a
      near-square key that stays near-square. Watch it fail on today's square
      box; record the measured before-numbers in NOTES.md.
- [ ] Compute the trimmed bbox + aspect per glyph once the glyph collection
      finishes loading, into a resource beside `NovaHudAssets::key_glyphs`.
      Handle the not-yet-loaded frame the same way the dock already handles it
      (`assets.is_changed()` re-paint).
- [ ] Teach `paint_key_visual` to size from that resource: height pinned to the
      site's constant, width derived, `ImageNode.rect` set to the cap bbox.
      Cover all three sites - dock chips, anchored cues, objective-stack TAB
      footer - rather than only the dock.
- [ ] Re-check the chip layouts a wider keycap now sits in: the dock row's
      `column_gap`/centring, the cue chips over the world, and the TAB footer.
      Adjust the constants if a wide cap now crowds its verb word.
- [ ] Correct the `GLYPH_PX` doc paragraph (it currently asserts the wide caps
      are small by construction and that this is not a squash).
- [ ] Sweep for any other square-boxed key glyph site before closing.
- [ ] Screenshot and LOOK at a dock with a wide cap in it (ledger: eyeball the
      rendered output).

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

## Flow State

- FLOW STEP: PLANNED
