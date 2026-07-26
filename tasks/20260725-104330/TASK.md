# Epic: NOVA OS terminal drawer for v0.9.0

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.9.0,feedback,gameplay,ui,ux,epic

## Context

This task captures the v0.9.0 feedback pass that turned the Tab drawer from a
simple information overlay into the candidate centerpiece for the release. The
raw feedback started before the drawer existed, then sharpened after seeing the
landed implementation in game. The durable design recommendation is in
`SPIKE.md`; this file is the source context that explains why that spike exists.

The broad UI feedback is that Nova's current interface still reads like a
prototype. The lines are weak, the color scheme feels washed out, and the current
blue/cyan direction is too gray. The preferred direction is a sharper terminal
spaceship UI: fixed-size windows, square borders, a good monospace terminal
font, denser layouts, scrollable lists where needed, and a darker palette that
can still keep Nova's blue-space identity. The visual PoC at
`examples/ui/nova_os_terminal_poc.html` tightened that into a Nova-owned look:
darker blue-black casing, green phosphor terminal screen, orange/yellow accents,
CRT scanlines, a physical bezel, and a diagnostic FPS/version overlay. The
current main menu, settings, mod explorer and scenario picker can inherit this
direction later, but the drawer should prove it first.

The gameplay feedback around comms and objectives points in the same direction.
Comms cards should feel less like small chat bubbles and more like terminal/API
traffic: clearer speaker treatment, useful log history, skip/dismiss behavior,
sound/typing feedback, and a Flight Log that records the conversation plus
important mission events. Objective presentation should move away from large
center-screen interruption: new objectives should be readable, tucked toward the
drawer affordance, and visible through the ship computer. Salvage pickup and
objective completion need better feedback, and multiple simultaneous objectives
need layout that does not collide.

The initial drawer idea was two surfaces: a ship-computer terminal on one side,
and a second screen that changes based on commands. After discussing it, the
preferred direction changed to one screen. Opening Tab should feel like sitting
at a cockpit computer: a single inset **NOVA OS** terminal monitor appears, boots
or restores its last state, and owns the drawer. Commands such as `help`,
`objectives`, `log` and `ship` can print directly in the terminal. Commands such
as `map` or `ship viewer` can launch GUI apps that temporarily swallow that same
monitor until the user exits the app. This avoids wasted side-panel space and
lets the game grow both CLI-style and GUI-style interactions.

The command UX should borrow from familiar shells. `help` should be suggested on
first boot. Tab should become autocomplete while NOVA OS owns the keyboard, with
colored hints for valid prefixes, invalid tokens, and likely completions. Typos
should get friendly suggestions like a Git subcommand typo. Escape should remain
the drawer close affordance; app exit needs its own control, probably explicit
app chrome plus a terminal-like chord such as `Ctrl+C` or `Ctrl+[`. RMB should
not be the default app-exit gesture because future GUI apps may need it.

The first useful app candidates are `map` and `ship viewer`. The map can begin
as the existing v0.9.0 minimap stretch idea, but launched from the terminal
instead of permanently occupying drawer space. The ship viewer should eventually
show the player's ship with labeled sections, HP and status, then allow actions
on individual sections. A CLI version should exist too, starting with read-only
`ship`; mutating commands such as `reload` or `repair` should be added only once
their rules are clear. `reload` is likely safer than `repair`; `repair` touches
resources, combat lockout, balance and scenario pacing.

Separate gameplay ideas from the same feedback pass remain useful future
context, but they are not the terminal drawer's first scope. The campaign could
use a stronger opening with a space station, docking or undocking, shorter or
skippable intro comms, typewriter-style delivery, and possibly puppeted cutscene
movement. Some scenarios may need spatial/story polish: Scenario 2 enemies feel
like they appear from nowhere, Scenario 3 may work better if spawn positions are
swapped, Scenario 4 could use space-station structures instead of beacons, and
Scenario 5 may need a slower investigation setup before the fight. Map-boundary
feedback and occasional combat-radar focus loss are also separate follow-ups.

## Epic notes

- This task is the epic container for the post-playtest terminal drawer vision.
  The researched direction lives in `SPIKE.md`; flow planning has now created
  child tasks, but no implementation work starts until the owner approves this
  gate.
- Current recommendation in `SPIKE.md`: replace the landed two-panel drawer with
  one inset **NOVA OS** terminal monitor. Commands either print inline terminal
  output (`help`, `log`, `objectives`, `ship`) or launch apps that swallow the
  same monitor until exited (`map`, later `ship viewer`).
- Current visual direction in `SPIKE.md`: use the drawer as the first pass at a
  stronger HUD style - darker Nova blue-black casing, green phosphor screen,
  orange/yellow accents, CRT scanline/bezel treatment, ordinary flight HUD
  hidden while open, and only diagnostic FPS/version allowed above it.
- Visual PoC: `examples/ui/nova_os_terminal_poc.html` is the current visual
  target for planning. It is a standalone HTML/CSS/JS mock of the one-screen
  NOVA OS monitor with terminal commands, app takeover, mocked map and ship
  viewer data, CRT scanlines, full-main monitor fill, and diagnostic
  FPS/version.

## Flow State

- FLOW STEP: PLANNING
- Gate status: awaiting owner review. Do not mark `PLAN STATUS: APPROVED`, cut a
  worktree, or start implementation until the owner explicitly approves the
  package below.

## Epic

Turn the v0.9.0 Tab drawer into one terminal-style **NOVA OS** cockpit monitor.
The monitor replaces permanent left/right drawer panels with a single physical
screen. In terminal mode, commands print useful information into scrollback. In
app mode, commands launch a GUI surface that swallows the same screen and later
returns to the terminal.

The v0.9.0 core is the OS shell, terminal input, useful read-only output
commands, and app runtime. `map` and `ship viewer` remain stretch apps after the
core is usable.

## Done Means

- The old permanent side-panel drawer has been replaced by one inset NOVA OS
  monitor that keeps the accepted `PauseStates::Drawer` freeze/cursor behavior.
  (test/manual: drawer tests plus screenshot/run comparison against
  `examples/ui/nova_os_terminal_poc.html`)
- The terminal accepts typed commands, supports editing/history/autocomplete,
  prints helpful errors and suggestions, and keeps Tab/Escape behavior
  unambiguous. (test: terminal input state tests)
- `help`, `log`, `objectives` and read-only `ship` provide useful output from
  live game state without stale scenario leakage. (test: command output tests)
- App takeover exists: a command can launch an app inside the same monitor, app
  input is isolated, and app exit returns to the terminal. (test: app runtime
  lifecycle tests)
- Ordinary flight HUD/key hints are hidden while NOVA OS is open, with only
  diagnostic screenshot chrome such as FPS/version intentionally allowed above
  it. (test/manual: HUD suppression and screenshot review)
- v0.9.0 tracker and stale drawer/minimap notes point at this one-screen NOVA OS
  model, not multiple drawers or permanent side panels. (manual: task review)

## Child Tasks

- [ ] `20260726-115320` (p49) - NOVA OS monitor shell and visual treatment.
- [ ] `20260726-115324` (p48) - NOVA OS terminal input and command shell.
      Depends on `20260726-115320`.
- [ ] `20260726-134738` (p50) - Match NOVA OS drawer to terminal PoC. Depends
      on `20260726-115320` and `20260726-115324`; blocks command/app tasks until
      the UI no longer needs to be reshaped under them.
- [ ] `20260726-115330` (p47) - NOVA OS terminal output commands. Depends on
      `20260726-134738`.
- [ ] `20260726-115334` (p46) - NOVA OS app runtime. Depends on
      `20260726-134738`.
- [ ] `20260724-102320` (p30, stretch) - NOVA OS map app launched from the
      terminal. Depends on `20260726-115334`.
- [ ] `20260726-115339` (p29, stretch) - NOVA OS ship viewer app and safe
      section actions. Depends on `20260726-115334`.

## Decisions

- `tasks/20260725-104330/DECISION.md` records the load-bearing layout decision:
  NOVA OS is one monitor with app takeover, not permanent drawer panels.
- `tasks/20260726-134738/DECISION.md` records the scope split for this follow-up:
  match the PoC UI now while keeping only `help` and `clear` executable.
- `tasks/20260724-102304/DECISION.md` still stands for freeze/input state:
  `PauseStates::Drawer` is the third variant on the existing pause axis.

## Manual Acceptance

Before implementation starts, the owner should confirm:

- One-screen NOVA OS is the accepted v0.9.0 direction.
- The core/stretches split is right: monitor shell, terminal input, read-only
  commands and app runtime first; `map` and `ship viewer` as stretch.
- Escape should remain drawer close, while apps return to terminal through app
  chrome plus a terminal-like chord such as `Ctrl+C` or `Ctrl+[`.

## Review notes

- 2026-07-26 review pass: tightened the spike after the conversation moved from
  "terminal plus separate screen" to "one terminal monitor that launches apps".
  Resolved two UX contradictions: Tab opens the drawer from flight but becomes
  autocomplete inside NOVA OS, and the old lower-left key hints should not remain
  visible over the CRT computer screen.
- Next session should plan from `SPIKE.md` rather than from the raw feedback
  bullets above; the bullets remain source context, not the current design.
