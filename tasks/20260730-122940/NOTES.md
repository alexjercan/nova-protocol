# Notes - keycap aspect sizing

## Measured art bounds (2026-07-30, re-measured on the branch)

`magick <file> -alpha extract -threshold 0 -format '%@' info:` over
`assets/input-prompts/keyboard/Alt/`. Every canvas is 128x128; the cap drawn
inside it is not.

| file | opaque bbox (WxH+X+Y) | aspect (w:h) |
|---|---|---|
| `T_X_Key_Alt.png` | 96x104+16+16 | 0.92 |
| `T_O_Key_Alt.png` | 96x104+16+16 | 0.92 |
| `T_Tab_Key_Alt.png` | 112x74+8+32 | 1.51 |
| `T_Shift_Key_Alt.png` | 112x74+8+32 | 1.51 |
| `T_Crtl_Key_Alt.png` | 112x74+8+32 | 1.51 |
| `T_Space_Key_Alt.png` | 128x68+0+32 | 1.88 |
| `T_Mouse_Scroll_Key_Dark_Key_Alt.png` | 76x128+28+0 | 0.59 |

These numbers are the INDEPENDENT expectation the rigs assert against
(`MEASURED_CAPS` in `hud::keybind_dock::keycap_sizing_tests`), never recomputed
from the production trim - LESSONS `test-must-not-reuse-the-formula-under-test`.

## Before-numbers (the fail-first run)

The rig `wide_keycaps_render_at_their_art_aspect` was run against the pre-fix
square box (the new `KeyCap::apply` temporarily reduced to `Vec2::splat(height)`
with no `ImageNode.rect`, which is exactly what the three sites did before):

```
CTRL: the rendered box is Vec2(22.0, 22.0) (aspect 1.000), but the art's opaque
cap is Rect { min: Vec2(8.0, 32.0), max: Vec2(120.0, 106.0) } (aspect 1.514)
```

So a Ctrl/Shift/Tab cap drew 22 px wide and, inside it, only 22 * 74/128 = 12.7
px of actual cap - the legend shrank with it. After the fix the same node
measures 33x22 with the cap filling the height.

Every cap grows, because the square box also padded the near-square caps:

| cap | before (box, drawn cap) | after |
|---|---|---|
| X / O @ GLYPH_PX 22 | 22x22 box, 16.5x17.9 cap | 20x22, cap fills it |
| Ctrl / Shift / Tab @ 22 | 22x22 box, 22x12.7 cap | 33x22, cap fills it |
| Space @ 22 | 22x22 box, 22x11.7 cap | 41x22, cap fills it |
| Scroll (mouse) @ 22 | 22x22 box, 13.1x22 cap | 13x22, cap fills it |

The mouse glyph is the only one that does not grow: it is TALLER than it is
wide, so the height pin leaves it where it was and only trims the empty side
bands off its node box.

## Eyeball (LESSONS `render-output-eyeball`, `compare-crops-at-one-zoom`)

`shots/dock-before.png` and `shots/dock-after.png` are the SAME crop of
`feature-hud.png` at the SAME zoom (`-gravity south -crop 900x60+0+10`,
`-resize 200%`), captured by

```
DISPLAY=:99 NOVA_SHOT_DIR=target/reel-<w> BCS_AUTOPILOT=1 BCS_REEL=1 \
  cargo run --example screenshot_combat --features debug
```

with the working tree stashed for the "before". Before: `Ctrl` and `Shift` are
unreadable smudges. After: both read at a glance, and the row still fits
comfortably.

## Inherited red (not this task)

`nova_assets::scenario::shakedown::tests::an_early_derelict_kill_skips_to_the_fight`
fails on the branch AND on its base commit d1460fc5 ("delivery guard: the
rehearsal was mid-lesson"), verified by stashing this task's changes and
re-running it. Filed separately; nothing here touches that scenario.
