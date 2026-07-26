# Decision: NOVA OS is one monitor with app takeover, not permanent drawer panels

- DATE: 20260726-120000
- STATUS: ACCEPTED
- TASK: 20260725-104330
- TAGS: decision, ui, hud, terminal

## Context

The first Tab drawer implementation shipped as a paused information overlay with
left/right drawer panels. Playtest feedback changed the desired product shape:
the drawer should feel like a cockpit computer where the player types commands
and sees useful responses. The visual PoC at
`examples/ui/nova_os_terminal_poc.html` showed that one full monitor reads
better than several permanent panes.

The freeze/input decision from `tasks/20260724-102304/DECISION.md` still stands:
the drawer remains `PauseStates::Drawer`, a third variant on the existing pause
axis. This decision only changes the drawer content model and layout.

## Decision

NOVA OS is one inset cockpit monitor. It has two modes:

- Terminal mode: commands such as `help`, `log`, `objectives` and `ship` print
  output into terminal scrollback.
- App mode: commands such as `map` or `ship viewer` launch a GUI app that
  replaces the terminal within the same monitor until the app exits.

There are no permanent left/right/center drawer panels in the new direction.
Existing drawer data sources remain valuable, but they are rendered as terminal
output or app content.

## Alternatives

- Keep the current side panels and restyle them as terminal chrome. This improves
  appearance but misses the interaction goal.
- Use a terminal plus a separate app viewport. This keeps terminal output visible
  but wastes space and makes GUI apps feel cramped.
- Build a full command VM/scripting console now. This is too broad for the first
  player-facing slice and would force parser, permissions and mutation rules
  before the OS interaction is proven.

## Consequences

- `Tab` opens the drawer from flight, but inside NOVA OS it belongs to terminal
  autocomplete.
- `Escape` remains the drawer close affordance. Apps need their own explicit
  return-to-terminal affordance, first planned as app chrome plus a chord such as
  `Ctrl+C` or `Ctrl+[`.
- The `map` minimap work stays useful, but it is a terminal-launched app rather
  than always-on center drawer content.
- The drawer becomes the first implementation of the stronger HUD visual
  language: dark blue-black casing, green phosphor screen, orange/yellow
  accents, CRT scanlines, and ordinary flight HUD hidden while open.
