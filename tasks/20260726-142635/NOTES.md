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
  `NOVA OS v{APP_VERSION}`, BIOS check, display check, and `Hint: type \`help\` and press Enter.`.
- Changed `clear` to restore the welcome block instead of leaving the terminal empty.

## Feedback Round 2

- Switched the NOVA OS terminal font to `assets/fonts/SGr-IosevkaTerm-Regular.ttc`, copied from `~/Downloads`, and registered a small `.ttc` loader because Bevy's built-in font loader only advertises `.ttf` and `.otf`.
- Changed terminal version labels to use `nova_info::APP_VERSION`, the same build-version source used by the status bar.
- Matched the HTML palette more closely: phosphor `#36ff79`, dim `#19a64f`, muted `#0d6e35`, amber `#ffb84a`, orange `#ff7b2d`, blue `#36a3ff`, with ordinary output rendered in the HTML's pale readable text color.
- Tuned the CRT shader toward the web sample's rounded-screen feel: darker perimeter, lower whole-screen tint, soft center/edge glow and subtle scanlines.
- Made the prompt row darker and rendered autocomplete as an inline ghost suffix, e.g. typing `he` shows `lp` after the cursor.
- Forced terminal scrollback to jump to the bottom after rebuilds so command output remains visible when it overflows.
- Removed `HudDrawerExempt` from the scenario readout strip so the top-center timer hides behind NOVA OS; only the actual status bar stays above the drawer.
- Changed drawer close to an animated close request: Escape, Start and right-stick close keep `PauseStates::Drawer` active until the slide reaches zero, then unpause.
- Upper-cased footer hints to better match the HTML PoC.

## Feedback Round 3

- Removed the `DRAWER PAUSED` topbar label; the right side now shows only `SHIP: <NAME>     LINK: LOCAL`.
- Read the ship name from the actual player ship root `Name` component, upper-cased for the NOVA OS topbar, with `UNKNOWN` as the no-name fallback.
- Made the screen bolder and darker: black-green base, stronger terminal panel opacity, stronger vignette, and a shader falloff that darkens monotonically toward the corners instead of forming a visible ring.
- Fixed the invisible typed-input bug by preventing both the prompt text and autocomplete ghost text from flex-shrinking away inside the prompt row.
- Removed NOVA OS text shadows so the terminal reads sharper and closer to the HTML sample.

## Feedback Round 4

- Pushed the CRT effect darker again: the screen base is closer to black, the vignette strength is high enough for almost-black corners, and the shader now uses a smooth corner falloff instead of an edge ring.
- Added subtle square-cell phosphor grain to the CRT shader. It is sparse and low-alpha so it reads like the HTML surface texture without burying text.
- Moved the command prompt into a dedicated full-width input lane, put the invalid-command hint on a separate line underneath, and lifted the prompt strip above the CRT overlay. This targets the remaining typed-text invisibility bug directly.
- Darkened the prompt strip itself so it reads as an input box sitting on top of the CRT screen rather than another washed screen layer.
- Added a faint zero-offset text bloom pass. This keeps the current sharp text shape and avoids the directional shadow that looked strange in the previous attempt.
- Reworked `help` output so it is generated from the registered command metadata with computed column alignment. The executable command registry still contains only `help` and `clear`.

## Feedback Round 5

- Increased terminal text contrast: normal output now uses the same bright phosphor green as the screen border, and dim/muted text moved up so secondary copy remains readable on the darker screen.
- Changed footer hints to current terminal-app keybinds: `TAB: AUTOCOMPLETE`, `ESC: CLOSE COMPUTER`, and `HINT: TYPE HELP`.
- Fixed autocomplete overlap by making the ghost an absolute input-lane overlay with monospace leading spaces before the completion suffix. It no longer depends on Bevy assigning the typed prompt node a usable flex width.
- Made CRT grain squares smaller and denser while reducing grain strength so the texture reads more like the HTML sample without lowering text contrast.
- Started the rounded-screen darkness closer to the center and added a second soft glass falloff term, while preserving the near-black edges from the previous pass.

## Tradeoffs

- The Iosevka Term TTC is large, about 66 MB. It is used directly because the requested font was only available locally as a TTC collection; a future asset pass can subset or replace it with a smaller TTF/WOFF if needed.
- The fallback scanline layer stays in place for headless/widget-tree coverage and non-material contexts, but its alpha is intentionally low because the shader now carries the real CRT treatment.
- The grain lives in the shader instead of hundreds of UI child nodes. That keeps the square texture cheap and keeps hit-testing/widget hierarchy focused on the terminal itself.
- The ghost offset uses monospace spaces instead of per-character measurement. That is appropriate for the Iosevka terminal font and avoids adding a Bevy text-measurement dependency to the prompt hot path.
- This task does not implement `log`, `objectives`, `ship`, `map`, app runtime or ship viewer output. The live backing state remains in `drawer.rs` for those future commands.

## Difficulties

- The first implementation looked structurally close in tests but the screenshot showed the material and fallback overlays stacking too strongly. The fix was to treat the CRT effect as an accent layer, not a screen-wide tint layer.
- Startup UI and terminal state had separate hardcoded boot rows. Changing the spawned scrollback to call the same welcome-row helper prevents another mismatch between initial render and rebuild.
- The prompt visibility bug did not show up in command-model tests because it was a UI flex sizing problem. The prompt-row test now asserts the typed text and ghost completion keep `flex_shrink: 0`.
- The follow-up prompt bug also needed a keyboard-path test. The new regression opens the drawer, sends real `KeyboardInput`, runs the terminal rebuild, and asserts the prompt and ghost `Text` entities show the typed command.

## Verification

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test -p nova_gameplay drawer`
- `nix develop --command cargo test -p nova_gameplay readout`
- `nix develop --command cargo test -p nova_menu escape_does_not_menu_toggle_the_drawer`
- `nix develop --command cargo check`
- `cd web && npm run ci`
- `tatr check --ledger LESSONS.md`

## Feedback Round 3 Verification

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test -p nova_gameplay drawer`
- `git diff --check`
- `nix develop --command cargo check`
- `cd web && npm run ci`
- `tatr check --ledger LESSONS.md`

## Feedback Round 4 Verification

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test -p nova_gameplay drawer`
- `git diff --check`
- `nix develop --command cargo check`
- `cd web && npm run ci`
- `tatr check --ledger LESSONS.md`

## Feedback Round 5 Verification

- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test -p nova_gameplay drawer`
- `git diff --check`
- `nix develop --command cargo check`
- `tatr check --ledger LESSONS.md`

## Self-Reflection

- The previous pass should have visually compared against a real game screenshot before close-out. For terminal/CRT work, a headless hierarchy test proves structure only; contrast, grain and readability need an actual rendered capture.
