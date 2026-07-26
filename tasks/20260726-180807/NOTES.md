# Notes: NOVA OS computer HTML-fidelity pass

- TASK: 20260726-180807

## The capture loop (the whole point)

The two prior tasks (`20260726-134738`, `20260726-142635`) landed six feedback
rounds of blind CRT number tweaks WITHOUT ever rendering the game, and the screen
still read as a pale-green wash. This task fixed that process first: I added
`examples/screenshots/screenshot_nova_os.rs` (registered in `Cargo.toml`), a
capture example that boots a one-ship range, presses Tab to open the computer,
drives `help` / `ship` through the real keyboard path and leaves `lo` in the
input to show the inline completion, then captures two PNGs.

Run:

```
DISPLAY=:0 NOVA_SHOT_DIR="$PWD/tasks/20260726-180807/shots" \
  BCS_AUTOPILOT=1 BCS_REEL=1 \
  nix develop --command cargo run --example screenshot_nova_os --features debug
```

BEFORE (`/home/alex/Downloads/1785077386771.png`, user-supplied) vs AFTER
(`tasks/20260726-180807/shots/nova-os-welcome.png` + `nova-os-active.png`). The
AFTER shots are the manual-gate evidence.

## Root cause of the wash (traced, then fixed)

The CRT overlay draws ABOVE the terminal text (overlay `ZIndex=1` > content
`ZIndex=0`). The old shader added a `glow` term peaking at the screen CENTRE
(0.13) plus a whole-screen green tint film, so a ~0.19-alpha green haze sat over
exactly where the text lives. On top of that, render-capable apps spawned BOTH
the shader overlay AND the UI-node scanline+vignette fallback, doubling the film.

Fixes in `assets/shaders/nova_os_crt.wgsl` + `crates/nova_gameplay/src/hud/drawer.rs`:

- Removed the centre `glow` term entirely (dropped `glow_strength` from the WGSL
  struct and `NovaOsCrtUniform`).
- Vignette is now edge-only: `smoothstep(0.46, 0.98, dist)` on a slightly
  elliptical distance, fully transparent through the readable centre (matches the
  HTML `radial-gradient(..., transparent 56%, rgba(0,0,0,0.42) 100%)`).
- Cut the tint alpha 0.06 -> 0.03, scanline 0.07 -> 0.06, vignette scale
  1.28 -> 0.55, grain 0.014 -> 0.01.
- The UI-node scanline/vignette fallback now spawns ONLY when there is no shader
  material (`spawn_nova_os_screen_overlays` returns early after the material), so
  render apps never stack two CRT layers.

## Text contrast

Aligned the palette to the HTML PoC: prompt/borders/lamp use the hot neon
phosphor `#36ff79`; ordinary body text uses the pale mint `--text #b9ffc9`
(brighter and higher-contrast on near-black than the old all-one-green); info
rows use the HTML blue `#36a3ff`.

## Input box + inline completion

- The prompt strip is now a near-opaque black-green box (`srgba(0,0.016,0.008,0.97)`)
  with a brighter phosphor top border, so it reads as a dark input box sitting
  above the screen (HTML `.prompt-row`).
- Rebuilt the prompt as four inline pieces in the input wrap: typed-before-cursor
  text | a 2px amber block caret | typed-after-cursor text | the dim completion
  ghost. All three text pieces use `LineBreak::NoWrap` (new
  `nova_os_prompt_text_layout`), which is the actual fix for the reported
  "completion appears below the line" bug - a width-starved ghost text node was
  wrapping. The ghost is now the raw suffix (no leading space) and there is no
  `|` glyph baked into the prompt text, so `log` reads as `lo` + caret + dim `g`
  on ONE line, fish-style.

## Command parity (user-confirmed at the plan gate)

- Executable set + order now mirror the HTML: `help`, `log`, `objectives`,
  `ship`, `clear`, `exit`. `map` / `ship viewer` stay unknown (their stretch
  tasks own them).
- `exit` sets `NovaOsTerminal::pending_close`, which `handle_terminal_keyboard`
  consumes to drive the existing animated close (same path as Esc/Start).
- Unknown commands now print two HTML-style rows: an error `command not found: x`
  and a warn `did you mean y?`.
- `help` summaries reworded to the HTML text; `ship` header recased `Ship:` ->
  `SHIP` while keeping the real live section data (user chose "real data, HTML
  formatting"). `log` / `objectives` keep their real-data rows.

## Tests

- `nova_os_registered_commands_match_html_set` - the set + order, `exit` parses,
  `map`/`ship viewer` stay unknown.
- `nova_os_help_lists_html_command_set` - `help` lists exactly the set in order.
- `nova_os_exit_closes_computer` - `exit` flips the close transition.
- `nova_os_inline_completion_is_same_line_continuation` - before/after/ghost text
  all carry `LineBreak::NoWrap`; ghost is the raw suffix.
- Updated the existing prompt/ghost/help/unknown tests to the new contract.

## Verification

- `nix develop --command cargo test -p nova_gameplay drawer` (46 passed)
- `nix develop --command cargo fmt --check`
- `nix develop --command cargo check`
- `cd web && npm ci && npm run ci`
- Captured + inspected the AFTER shots (readability, input box, inline
  completion and CRT all match the HTML PoC).

## Tradeoffs / difficulties

- `log` / `objectives` / `ship` keep the game's real data rather than the HTML's
  mock lines (the user picked "real data, HTML formatting"), so the wording is
  HTML-styled but the content stays live and more useful. A deeper HTML-exact
  reformat (percent bars per section, etc.) was intentionally left out to respect
  "content is good" and avoid churn; flag it in review if more parity is wanted.
- The input box sits under the (now near-transparent) CRT overlay in the node
  tree; since the overlay is a sibling of the content parent, a child prompt row
  cannot truly render above it. This is fine because the overlay is faint enough
  that the dark box still reads as raised, and it keeps the HTML-faithful
  "overlays above content" ordering.

## Self-reflection

The durable lesson from `142635`'s RETRO held: build the capture harness FIRST,
then tune against a real render. Every change here was validated on a screenshot,
not guessed. The one-shot fix (kill the centre glow + stop double-stacking the
overlay) did more than six rounds of alpha nudging because it addressed the
mechanism, not the symptom.
