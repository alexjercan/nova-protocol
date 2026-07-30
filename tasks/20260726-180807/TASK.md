# NOVA OS computer: real HTML-fidelity pass (contrast, input box, inline completion, command parity, CRT)

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: v0.9.0,feature,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Work Record

- Added `examples/screenshots/screenshot_nova_os.rs` (+ `Cargo.toml` entry): a
  capture example that opens the computer, drives commands via the real keyboard
  path, and captures `shots/nova-os-welcome.png` + `shots/nova-os-active.png`.
  `shots/before-game.png` and `shots/reference-html.png` are the comparison.
- Fixed the CRT wash: removed the centre glow, made the vignette edge-only, cut
  tint/vignette/grain alphas, and stopped the shader + UI-node fallback from
  double-stacking (`assets/shaders/nova_os_crt.wgsl`, `drawer.rs`).
- Bright HTML palette (neon phosphor prompt, pale-mint body, blue info).
- Dark input box with a phosphor top border; fish-style inline completion built
  from before/caret/after/ghost pieces, all `LineBreak::NoWrap` (fixes the
  below-line wrap bug).
- Command parity: HTML set + order `help,log,objectives,ship,clear,exit`; `exit`
  drives the animated close; two-line `command not found` / `did you mean`.
- Updated CHANGELOG + wiki hud.md; 4 new DoD tests + updated existing tests.
- Verify: `cargo test -p nova_gameplay drawer` (46 ok), `cargo fmt --check`,
  `cargo check`, `web npm run ci` - all green. AFTER shots inspected and match
  the HTML PoC.

## Story

As a player opening the Tab ship computer, I want NOVA OS to actually look and
behave like `examples/ui/nova_os_terminal_poc.html`: bright readable phosphor
text on a near-black CRT, a dark input box that sits above the screen, fish-style
inline autocomplete, and commands that match the HTML wording/order. Right now
the in-game screen is a pale green wash with unreadable text (see
`/home/alex/Downloads/1785077386771.png` vs the HTML
`/home/alex/Pictures/Screenshots/20260726_142306.png`).

This is the seventh pass on this screen. Tasks `20260726-134738` (structure) and
`20260726-142635` (contrast, 6 feedback rounds) matched the widget tree and
tweaked CRT numbers but landed every round WITHOUT capturing a real in-game
render. That is the recorded miss in `tasks/20260726-142635/RETRO.md`: for a
CRT/readability task, headless tree tests prove structure only; contrast and
readability need an actual rendered capture. This task MUST iterate against a
real screenshot of the running game before it is allowed to close.

Keep the CONTENT and the FONT as-is (both are good). Keep the version/BIOS
welcome block and the footer hint row shape as-is - only their styling/command
functionality changes.

## Root cause (already traced)

The CRT overlay (`assets/shaders/nova_os_crt.wgsl` + fallback nodes) draws ABOVE
the terminal text (overlay `ZIndex=1` > content `ZIndex=0`). Its `glow` term
(0.13, peaking dead-center) plus the tint film puts a ~0.19-alpha green haze over
exactly the region where all the text lives, washing it out. The HTML keeps its
center glow subtle and BEHIND the text, with crisp bright text on near-black. The
fix is to stop the CRT layer from filming the text: kill/relocate the central
glow, drop overlay alpha hard, and/or move the tint behind the content, verified
against a real capture - not another blind number tweak.

## Command parity decision (user-confirmed at plan gate)

- Executable set becomes: `help`, `clear`, `log`, `objectives`, `ship`, `exit`.
- `exit` closes the computer (drives the existing animated close transition).
- Do NOT add `map` / `ship viewer` (those stay in stretch tasks
  `20260724-102320` / `20260726-115339`); they remain unknown commands. `help`
  lists only the executable set.
- `log` / `objectives` / `ship` keep pulling REAL game data, formatted to look
  like the HTML sample (spacing, casing, colour kinds).
- Unknown-command + did-you-mean wording matches the HTML as close as practical.

## Steps

- [x] Run the game, open NOVA OS (Tab), and capture a BEFORE screenshot into
      this task folder. This capture-first-then-tune loop is the whole point of
      the task (see `20260726-142635` RETRO). Record the capture command used.
- [x] Text contrast: make terminal text read bright and crisp on near-black like
      the HTML. Audit the per-row-kind colours against the HTML palette
      (`--phosphor #36ff79`, `--text #b9ffc9` body, `--phosphor-dim`, amber,
      blue, red) and the text-bloom pass so nothing greys out.
- [x] CRT background: fix the wash. Stop the CRT overlay from filming the text -
      remove/relocate the central `glow`, cut overlay tint/vignette alpha, and/or
      render the tint behind the content. Match the HTML's near-black screen with
      a subtle contained vignette and faint scanlines, verified on a real render.
- [x] Input box: make the prompt strip darker and read as a box sitting ABOVE the
      screen (like the HTML `.prompt-row`), clearly above the CRT overlay.
- [x] Inline completion: fix the fish-style continuation bug. Typing `help`
      letter by letter must show the completion as a dim inline continuation
      directly after the typed text on the SAME line (e.g. `hel` + dim `p`), with
      a real caret, never wrapping to a line below and never a stray `|`/space
      artifact. Match the HTML ghost behaviour.
- [x] Command parity: register `exit` (closes the computer), keep `help`, `clear`,
      `log`, `objectives`, `ship`. Reformat `help`, `log`, `objectives`, `ship`
      output and the unknown/did-you-mean wording to match the HTML as close as
      practical, keeping real game data. Do not add `map`/`ship viewer`.
- [x] Rename user-facing "drawer"/"DRAWER" wording to "computer" where it is
      shown to the player (footer hint already says CLOSE COMPUTER; sweep for
      other visible strings). Internal module/type names may stay `drawer`.
- [x] Update/extend headless drawer tests for: the new executable set incl.
      `exit`, the reformatted help/unknown wording, the inline-completion
      contract (ghost on same line), and any structure assertions that change.
- [x] Capture an AFTER screenshot with the same command, put it beside the BEFORE
      in this task folder, and confirm readability + input box + completion +
      CRT match the HTML. Read the produced image before close-out.
- [x] Update live docs (`CHANGELOG.md`, `web/src/wiki/hud.md`, any other non-task
      surface found by grep) only if their NOVA OS description becomes stale.
- [x] Write NOTES.md: what changed, why, the capture loop, tradeoffs,
      difficulties, and self-reflection.

## Definition of Done

- The executable command set is exactly `help`, `clear`, `log`, `objectives`,
  `ship`, `exit`; `map`/`ship viewer` still return unknown-command behaviour.
  (test: `nova_os_registered_commands_match_html_set`)
- `exit` closes the computer via the animated close transition. (test:
  `nova_os_exit_closes_computer`)
- Typing a command prefix shows the completion as a dim inline continuation on
  the same line as the typed text (no below-line wrap, no stray cursor glyph).
  (test: `nova_os_inline_completion_is_same_line_continuation`)
- `help` lists exactly the executable set in HTML order and the unknown-command
  path uses the HTML-style wording. (test: `nova_os_help_lists_html_command_set`)
- Touched drawer tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer`)
- Formatting and build checks pass. (cmd:
  `nix develop --command cargo fmt --check` and cmd:
  `nix develop --command cargo check`)
- manual: BEFORE and AFTER in-game screenshots are captured into this task
  folder and the AFTER matches the HTML PoC on readability, input box, inline
  completion and CRT feel. This is the hard gate the previous two tasks skipped.

## Notes

- Epic: `tasks/20260725-104330/TASK.md`. Follow-up to `20260726-134738` and
  `20260726-142635`.
- Keep the font (`assets/fonts/SGr-IosevkaTerm-Regular.ttc`) and the welcome/
  version block content as-is. Footer stays a per-app help-key row; only its
  command functionality/style changes.
- HTML reference: `examples/ui/nova_os_terminal_poc.html`. HTML screenshot:
  `/home/alex/Pictures/Screenshots/20260726_142306.png`. Current game
  screenshot: `/home/alex/Downloads/1785077386771.png`.
