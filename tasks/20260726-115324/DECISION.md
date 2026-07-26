# Decision: Keep the terminal shell inside the drawer plugin

- DATE: 20260726-120000
- STATUS: ACCEPTED
- TASK: 20260726-115324
- TAGS: decision, ui, hud, input

## Context

Task `20260726-115320` replaced the old drawer panels with one NOVA OS monitor
owned by `crates/nova_gameplay/src/hud/drawer.rs`. That same module already owns
Tab opening, drawer scroll viewports, the live `DrawerFlightLog`, active
objective rows and the prompt placeholder. `nova_menu` owns Escape and the
pause/free-cursor hooks through `PauseStates::Drawer`.

## Decision

Implement the first terminal input shell as private drawer-plugin state and
systems in `hud/drawer.rs`. Tab from flight still opens the drawer, but Tab
inside `PauseStates::Drawer` is consumed by terminal completion. Escape remains
the existing `nova_menu` drawer-close path. The command registry starts with
read-only shell mechanics, `help` and `clear`; gameplay commands stay out of
this task.

## Alternatives considered

- **New pause/menu mode** - rejected because the accepted drawer architecture is
  one shared freeze axis, and a second mode would duplicate the freeze/cursor
  lifecycle already wired for `PauseStates::Drawer`.
- **Shared `nova_ui` terminal widget** - rejected for this first slice because
  no other surface consumes a terminal yet, and the implementation needs drawer
  resources, scrollback and command state tied to the monitor shell.
- **Gameplay command bus now** - rejected because later tasks own live output
  commands and app runtime; this task should prove shell input behavior without
  widening the gameplay surface.

## Consequences

The first implementation stays close to the code that owns the monitor and can
test the Tab/Escape split directly against `PauseStates::Drawer`. If later menu
or modding surfaces need a terminal, this drawer-local model may need extraction
after the behavior is proven.
