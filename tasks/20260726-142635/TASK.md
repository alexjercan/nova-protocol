# Tune NOVA OS terminal contrast and welcome

- PRIORITY: 51
- TAGS: v0.9.0, feature, ui, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a player opening NOVA OS, I want the in-game terminal to be readable and visually close to the HTML reference screenshot, so the drawer feels like a dark, sharp cockpit CRT instead of a pale green wash.

This is a UI-only follow-up to `20260726-134738`. Keep the executable terminal commands limited to `help` and `clear`; do not implement the pending command/app tasks `20260726-115330`, `20260726-115334` or `20260726-115339`.

## Steps

- [x] Compare the HTML screenshot `/home/alex/Pictures/Screenshots/20260726_142306.png` with the game screenshot `/home/alex/Downloads/1785065030199.png` and record the visible deltas.
- [x] Darken the drawer backdrop/screen treatment so the monitor reads closer to the HTML PoC.
- [x] Increase terminal text size and contrast, including brighter saturated phosphor, amber, dim and info rows.
- [x] Reduce the aggressive CRT grain/scanline wash while preserving the shader and fallback overlay details.
- [x] Replace the minimal startup rows with the HTML-style welcome block, and make `clear` restore that welcome block as the first scrollback content.
- [x] Update focused drawer tests for welcome rows and `clear` behavior.
- [x] Document what changed, tradeoffs, verification and self-reflection in this task folder.

## Definition of Done

- The default NOVA OS scrollback starts with the HTML-style welcome block. (test: `drawer_matches_nova_os_terminal_poc_structure`)
- The `clear` command restores the welcome block instead of leaving an empty terminal. (test: `nova_os_clear_restores_welcome_block`)
- Only `help` and `clear` remain executable. (test: `nova_os_only_help_and_clear_are_registered`)
- Touched drawer tests pass. (cmd: `nix develop --command cargo test -p nova_gameplay drawer`)
- Formatting and build checks pass. (cmd: `nix develop --command cargo fmt --check` and cmd: `nix develop --command cargo check`)

## Notes

- User visual feedback: current game screenshot is hard to read, too pale, too desaturated, has too much grain/scanline wash, smaller text than the HTML, and lacks the welcome text.
- Keep objectives and logs backing state in place for future commands, but do not render or implement the future command outputs here.
