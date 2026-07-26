# NOVA OS Contrast And Welcome Notes

## Screenshot Comparison

- HTML reference `/home/alex/Pictures/Screenshots/20260726_142306.png`: dark blue-black page, very dark green terminal surface, readable 16px-ish monospace text, saturated green/amber/blue rows, subtle scanlines, and a four-line boot welcome block.
- Game screenshot `/home/alex/Downloads/1785065030199.png`: readable layout shape, but the whole screen had a pale green wash, dense scanlines over most of the monitor, smaller text, low contrast against the screen, and only the old two-line `NOVA OS READY` boot text.

## Changes

- Darkened the drawer backdrop from the generic HUD backdrop to a NOVA OS black-blue color, with higher full-open opacity so the frozen scene does not wash the monitor out.
- Increased terminal line font from 13px to 16px, topbar font from 12px to 14px, and prompt row height from 42px to 48px.
- Shifted screen/text colors toward the HTML reference: darker screen base, brighter saturated phosphor, stronger amber/orange, and a blue info color for the welcome header.
- Added `Dim`, `Info` and `Warn` terminal row kinds so the welcome copy can match the HTML row hierarchy instead of rendering every boot line as the same green output.
- Reduced the CRT material tint alpha, scanline strength, vignette strength and glow. Also reduced the fallback scanline/vignette UI node alphas so render-capable apps do not compound into the pale green film seen in the game screenshot.
- Replaced startup scrollback with the HTML-style welcome block:
  `NOVA OS 0.9.0-dev`, BIOS check, display check, and `Hint: type \`help\` and press Enter.`.
- Changed `clear` to restore the welcome block instead of leaving the terminal empty.

## Tradeoffs

- The drawer still uses Bevy's default font for now, per the original scope. The stronger font size/color pass is the practical readability fix until a dedicated font asset task lands.
- The fallback scanline layer stays in place for headless/widget-tree coverage and non-material contexts, but its alpha is intentionally low because the shader now carries the real CRT treatment.
- This task does not implement `log`, `objectives`, `ship`, `map`, app runtime or ship viewer output. The live backing state remains in `drawer.rs` for those future commands.

## Difficulties

- The first implementation looked structurally close in tests but the screenshot showed the material and fallback overlays stacking too strongly. The fix was to treat the CRT effect as an accent layer, not a screen-wide tint layer.
- Startup UI and terminal state had separate hardcoded boot rows. Changing the spawned scrollback to call the same welcome-row helper prevents another mismatch between initial render and rebuild.

## Verification

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test -p nova_gameplay drawer`
- `nix develop --command cargo check`
- `cd web && npm run ci`
- `tatr check --ledger LESSONS.md`

## Self-Reflection

- The previous pass should have visually compared against a real game screenshot before close-out. For terminal/CRT work, a headless hierarchy test proves structure only; contrast, grain and readability need an actual rendered capture.
