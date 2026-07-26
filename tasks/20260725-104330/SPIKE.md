# Spike: should v0.9.0 turn the Tab drawer into a command terminal?

- DATE: 20260726-105322
- STATUS: RECOMMENDED
- TAGS: spike, v0.9.0, ui, hud, ux, epic

## Question

The v0.9.0 drawer already exists as a paused ship-computer overlay, with a left
Flight Log and a right Objectives panel. Owner feedback after playtest pushes the
idea further: the drawer should feel like a terminal-style spaceship computer,
where the player types commands into one "Nova OS" screen. Some commands print
directly in the terminal; others launch an app that temporarily swallows the
terminal within the same cockpit monitor and returns to it on exit.

A good answer decides whether this belongs in the v0.9.0 sprint, what the first
shippable version should actually do, what should stay deferred, and how the
existing drawer tasks should change before `/plan` turns this epic into smaller
tatr work.

## Context

The current drawer foundation is real and usable. `crates/nova_gameplay/src/hud/drawer.rs`
owns the Tab toggle, the `PauseStates::Drawer` modal state, real-time slide
animation, backdrop, two sliding side panels, scrollable left/right lists, the
derived `DrawerFlightLog`, and active objective rendering. The accepted state
decision in `tasks/20260724-102304/DECISION.md` should stand: the drawer is the
third variant of the single pause/freeze axis, not a second independent freeze
state. The old drawer spike, `tasks/20260721-211512/SPIKE.md`, correctly solved
the initial "how does Tab open a frozen information surface?" question, but its
content model is now stale because the owner wants a commandable ship computer,
not just side panels with logs and objectives.

The current v0.9.0 queue also matters. Most cockpit drawer foundation tasks are
already closed: shell, z-order, objective reveal, minimalist status hint,
HUD-hide/backdrop rework, status tier, comms stack, styled objectives, left
Flight Log, final objective retention, scrolling and scroll clamping. The only
open drawer feature task is `20260724-102320`, the center 3D minimap stretch.
That means this spike is not starting from zero. It is deciding how to evolve the
landed drawer into an interactive computer without throwing away the shipped
foundation.

The visual target also changed. The current shared UI theme is a dark
navy/cyan/amber industrial HUD (`nova_ui::theme`, mirrored from
`web/src/style.css`), but the owner finds it washed out and too gray-blue. The
local PoC at `examples/ui/nova_os_terminal_poc.html` is now the strongest visual
reference: a green phosphor terminal inside a dark blue-black cockpit monitor,
with scanlines, a physical bezel, sharp computer-window chrome, orange/yellow
accent states, and only diagnostic FPS/version floating above it. The drawer
should be the first implementation of that stronger HUD language; the main menu
and other UI can follow later once the drawer proves the style.

## Options considered

- **A. Keep the current drawer and only restyle it as terminal chrome.** This
  would change typography, palette, borders, row text, and headings while
  keeping left Flight Log plus right Objectives as mostly static lists. It is the
  smallest change and would address part of the owner's "terminal TUI" taste,
  but it misses the stronger vision: typing commands and seeing the ship
  computer respond. It risks shipping a skin when the desired interaction is the
  point.

- **B. Replace the drawer with one terminal screen plus launchable apps.** The
  drawer becomes one screen. Commands either print terminal output inline
  (`help`, `objectives`, `log`) or launch an app that takes over the same screen
  (`map`, `ship viewer`). Exiting the app returns to the previous terminal
  session. This uses all available space, makes the computer feel like an OS, and
  gives GUI-heavy tools room to breathe. The risk is UX complexity around app
  exit, focus, command history, and state persistence.

- **C. Hybrid command terminal plus separate output viewport.** Keep the existing
  drawer state and the left/right panel components, but introduce a terminal
  command model as the primary interaction. The left panel becomes the command
  console and command history. The center/right display area becomes a
  command-selected screen: `log`, `objectives`, `map`, `ship`, `help`, etc. The
  existing Flight Log and Objectives renderers are reused as first output modes
  instead of deleted. This is straightforward to build on the landed two-panel
  code, but it now feels like wasted space next to the stronger single-screen OS
  concept.

- **D. Build a full in-game command VM or scripting console.** The terminal
  accepts a richer language, can run scripts/macros, and exposes broad game
  control. This is powerful, but it is a different feature class. It introduces
  parser, permissions, state mutation, save/load and modding questions before the
  player-facing loop has proven itself.

- **E. Defer command input and finish the minimap first.** This keeps the old
  v0.9.0 sequence intact. It avoids changing direction mid-sprint, but it would
  make the center minimap the next thing built even though the owner's updated
  feedback says the terminal interaction is now the more important identity of
  the drawer.

### Visual shell options

- **V1. Current blue industrial HUD, only darker.** Keep the existing palette
  family and deepen the panel/background colors. This preserves Nova's current
  brand direction and is cheap, but it does not reach the physical retro
  terminal feeling by itself.
- **V2. Pure green phosphor terminal.** Make the drawer monochrome green with a
  CRT shader, scanlines and terminal text. This strongly matches the reference,
  but it risks losing Nova's blue-space identity and removes useful semantic
  accents.
- **V3. Dark blue-black Nova computer with green phosphor as the active screen
  language and orange/yellow accents.** Keep the outer system darker and colder
  than today, but make the active terminal/app screen read green-phosphor. Use
  orange/yellow for warnings, selections, command highlights and objective
  moments. This preserves the "Nova in space" blue base while making the
  computer feel retro, physical and readable. This is the direction shown by
  `examples/ui/nova_os_terminal_poc.html`.

## Recommendation

Use option B: evolve the landed drawer into a one-screen terminal OS in v0.9.0,
but ship it as a tight vertical slice rather than trying to expose every future
computer function at once.

Use visual option V3 for the drawer: darker blue-black outer chrome, green
phosphor terminal surface, and orange/yellow accents. Treat the drawer as the
style pilot for a later menu/HUD refresh, not as a one-off skin. The first
planning pass should use `examples/ui/nova_os_terminal_poc.html` as the visual
reference for screen fill, bezel proportions, phosphor contrast, scanlines,
diagnostic FPS/version placement, and the terminal/app takeover model.

The drawer should keep `PauseStates::Drawer`, Tab close/open behavior, the
backdrop, and the current HUD suppression rules. The change is inside the
drawer's content model:

- The whole drawer becomes one fixed-size **NOVA OS** screen.
- It should feel like a physical cockpit monitor, not a floating web panel:
  inset from the viewport edges, framed by a hard bezel, with the active screen
  inside that frame.
- Opening the drawer shows a boot/welcome terminal unless a previous app should
  be restored.
- Terminal-mode commands print inline output in the screen's own scrollback:
  `help`, `objectives`, `log`, `ship`, maybe `clear`.
- App-mode commands launch a GUI app that replaces the terminal on the same
  screen: `map` for the minimap, `ship viewer` for a clickable section viewer,
  later richer tools.
- Exiting an app returns to the terminal with the prior scrollback and prompt
  intact.
- Existing drawer views become command outputs or apps, not discarded work:
  `log` prints the combined Flight Log; `objectives` prints active objectives;
  `map` launches the placeholder minimap app; `ship` can start as a CLI summary
  before `ship viewer` exists.
- Ship modification commands should be introduced cautiously. For v0.9.0, prefer
  read-only `ship` first, then one explicit verb with readable rules, such as
  `reload`, if it maps cleanly to existing ammo/weapon state. Keep `repair` for
  a follow-up unless its resource and combat-lockout rules are designed.

This lets v0.9.0 keep the drawer "in one go" as a coherent product beat: not
side panels first and command computer later, but one terminal-style ship
computer that can run small in-game apps. The important constraint is that
"terminal" means interaction and response, not only monospace styling.

## Proposed v0.9.0 shape

First screen after opening Tab:

- One fixed terminal screen with a prompt such as `nova>`, recent output, and
  input focus.
- The first open in a scenario shows a short **NOVA OS** boot/welcome block and
  suggests `help`.
- Later opens restore the last terminal state or the last app, depending on the
  chosen persistence rule.
- Ordinary flight HUD and key hints hide while NOVA OS is open. The only
  intentional overlay exception is diagnostic screenshot chrome: FPS/version may
  stay visible above the computer screen.

Initial commands:

- `help` - show available commands and one-line syntax.
- `log` - print comms plus objective event stream in the terminal.
- `objectives` - print active objectives in the terminal.
- `map` - launch the placeholder 3D/schematic map app if that task lands;
  otherwise print `module unavailable` with a useful hint.
- `ship` - print ship sections, critical status, weapons, thrusters, cargo or
  salvage count as available from existing state.
- `ship viewer` - later GUI app that takes over the screen, labels each section,
  shows HP/status, and allows click-driven actions on sections.
- `reload` - candidate first mutating command, only if it maps cleanly to
  existing ammo/weapon state.
- `repair` - candidate later mutating command, only if the rules are designed
  around resources, combat lockout, and section state. This likely wants its own
  planning decision.

Input behavior:

- Tab opens the drawer from flight, but once NOVA OS owns the keyboard, Tab is
  terminal completion. Do not require Tab to close the drawer from inside the
  prompt; that would fight autocomplete.
- Escape closes the drawer from terminal mode. In app mode, Escape should still
  close the drawer unless playtest proves that is too destructive.
- Enter submits the focused command line.
- Up/Down navigate command history.
- Backspace/Delete/Left/Right edit the command line.
- Tab completion suggests commands and completes the common prefix.
- While typing, valid prefixes render in the normal input color, unknown tokens
  render red, and ghost text suggests the most likely completion in gray/green.
- Close command typos get a helpful suggestion, e.g. `did you mean ship?`.
- Mouse wheel scrolls terminal scrollback or the active app's scroll area.
- When an app is active, keyboard/mouse ownership belongs to the app until the
  user exits it.
- App exit should not be Escape if Escape remains "close the whole drawer".
  Preferred first design: reserve an on-screen close control in the app chrome
  plus a terminal-like command chord such as `Ctrl+C` or `Ctrl+[` to return to
  the terminal. Avoid RMB as the default app-exit gesture because future apps may
  need RMB for their own actions.

Visual direction:

- Move the drawer UI toward a realistic terminal spaceship HUD: sharper window
  frames, fixed window sizing, monospace terminal font, denser rows, command
  prefixes, timestamps or sequence ids where useful, and stronger line contrast.
- Palette: keep a much darker blue-black Nova base, use green phosphor as the
  primary active screen/text language, and keep orange/yellow accents for
  warnings, objectives, selections and important command output. The current
  blue/cyan feels too gray and washed out; if it remains, deepen it and reduce
  the gray cast.
- CRT treatment: add scanlines, subtle phosphor glow/bloom, vignette, slight
  curvature or screen glass impression, and maybe a boot flicker. Prefer a
  shader/material layer if Bevy UI supports it cleanly; otherwise build a
  composited overlay first and defer heavier shader work.
- Framing: do not let the terminal touch the screen edges. Leave a deliberate
  margin and use a recessed border/bezel, with the question still open whether
  the surrounding area is solid computer casing, dimmed world, or a stylized
  backdrop.
- Avoid only reskinning the drawer. The terminal should show typed commands and
  command responses, so the interaction sells the fantasy.
- Keep the palette centralized in `nova_ui::theme`; do not hand-tune drawer-only
  colors unless they are semantic accents.
- The one-screen layout should use the full drawer area. No separate permanent
  side panels, because they waste space once app-mode exists.
- When NOVA OS is active, hide all ordinary flight HUD/UI behind it. The only
  exception should be screenshot/diagnostic chrome that exists for bug reports:
  FPS/version may remain visible above the screen when the HUD visibility mode
  allows it. Everything else should yield to the computer screen.

## Flow planning split

Flow planning created the gate package below. `20260725-104330` remains the epic
container; implementation should not start until the owner approves the package.

- `20260726-115320` - NOVA OS monitor shell and visual treatment. Builds the
  physical one-screen monitor, ports the PoC's visual language into Bevy UI, and
  hides ordinary flight HUD while preserving diagnostic FPS/version chrome.
- `20260726-115324` - NOVA OS terminal input and command shell. Adds prompt
  editing, history, autocomplete, command registry plumbing, `help`, `clear`,
  typo suggestions and the Tab/Escape input split.
- `20260726-115330` - NOVA OS terminal output commands. Implements read-only
  `log`, `objectives` and `ship` output from live game state.
- `20260726-115334` - NOVA OS app runtime. Adds terminal mode vs active app mode,
  app registration, input ownership, app exit and lifecycle tests.
- `20260724-102320` - NOVA OS `map` app stretch. Keeps the 3D/schematic minimap
  idea, but launches it from the terminal and lets it swallow the same monitor.
- `20260726-115339` - NOVA OS `ship viewer` stretch. Adds clickable ship-section
  inspection and only includes mutating actions if their rules are explicit.

Wider UI restyle remains a follow-up after the drawer proves the style. The main
menu, settings, mod explorer and scenario picker can inherit the palette/font
later, but the v0.9.0 gate keeps the implementation centered on the in-game
ship-computer drawer.

## Open questions

- What is the minimum mutating command that makes the terminal feel real without
  inventing a half-designed management system? `reload` is probably safer than
  `repair`; `repair` touches resources, combat lockout, balance and scenario
  pacing.
- What exits an app without conflicting with drawer close? Recommendation for
  planning: Escape keeps closing the drawer; apps get an explicit chrome close
  button plus a keyboard chord such as `Ctrl+C` or `Ctrl+[`. Confirm in
  playtest before giving RMB to app exit.
- Should Tab close the drawer while NOVA OS is open? Recommendation: no. Tab
  should become autocomplete inside the terminal; Escape and the OS close chrome
  close the drawer.
- Should typing consume all text/navigation keys while the terminal is visible,
  and should app mode own WASD/mouse until app exit? The map app and ship viewer
  need explicit input ownership.
- Does the OS restore the last active app when Tab reopens, or does closing the
  drawer always return to the terminal? Recommendation: persist the last app
  within one scenario, but show the NOVA OS boot/welcome once per scenario and
  reset on player/scenario teardown.
- Should the terminal have in-fiction command names (`scan`, `nav`, `status`) or
  plain readable names (`map`, `objectives`, `ship`)? Recommendation: start with
  readable aliases and let flavor appear in output text.
- Should the first command output include timestamps? The prior Flight Log spike
  deferred timestamps because no stable scenario clock was defined. A simple
  sequence counter may be enough for v0.9.0.
- Should this stay confined to the in-game drawer for v0.9.0, or should the main
  menu/settings/scenario picker restyle happen in the same sprint? Recommendation:
  keep the command terminal in the drawer first; file the wider UI restyle as a
  separate v0.9.0 or backlog track once the drawer visual language is proven.
- How much CRT shader work belongs in v0.9.0? Recommendation: build the bezel,
  palette, typography, scanline/vignette overlay and boot flicker first. Add a
  real shader only if the Bevy UI/render path is clean enough to test with
  screenshots.
- What exactly surrounds the inset screen: opaque computer casing, dimmed frozen
  world, or a hybrid? Recommendation: start with a dark physical casing/bezel so
  the computer reads as an object in the cockpit, then playtest whether the
  dimmed world should remain visible outside it.

## What changed from the earlier drawer direction

- The old direction: Tab opens a paused drawer with side panels for information.
- The new direction: Tab opens one NOVA OS terminal screen.
- Some commands print directly in the terminal; other commands launch apps that
  swallow the terminal until exited.
- The old left/right panel work is still useful as data/rendering behavior, but
  the permanent two-panel layout should be replaced.
- The minimap remains useful, but it should be launched by `map`, not treated as
  mandatory always-on center content.
- Ship status/damage is no longer just a passive future section; it becomes the
  gateway to both CLI commands and a GUI `ship viewer` app.
- The drawer is also now the first pass at the future UI art direction: darker
  Nova blue-black, green phosphor terminal surface, orange/yellow accents,
  CRT/bezel treatment, and ordinary flight HUD hidden while the computer is open.

## Fix record

This is the epic-level spike for the terminal drawer family. Implementing child
tasks should append short entries here as they land: task id, what shipped, proof
headline, and any direction changes.
