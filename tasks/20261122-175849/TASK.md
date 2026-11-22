# HUD notice: centre the box above the keybind dock

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.19.0, hud, ui

## User facts
- Owner 2026-11-22: "now the notification above the keybinds is not
  centered, it's anchor point is left side and it looks like shit, center
  it please".

## Agent findings
- The HUD notice box is placed with its left edge on the screen midline
  (`web/src/wiki/hud.md`, HUD notice bullet; `crates/nova_hud/src/hud_notice.rs`).

## Delivery
- Centre the HUD notice box on the screen midline above the keybind dock,
  for every notice length and both themes.

## Verification
- Asserted layout: the box centre sits on the midline; frames of a short
  and a long notice.

## Done when
- The notice is centred above the keybinds and the owner has seen it.
