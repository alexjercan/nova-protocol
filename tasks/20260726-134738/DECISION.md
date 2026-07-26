# Decision: match the PoC in the drawer UI before command work

- DATE: 20260726-134738
- STATUS: ACCEPTED
- TASK: 20260726-134738
- TAGS: decision, ui, hud, drawer

## Context

The closed monitor shell task `20260726-115320` deliberately adapted
`examples/ui/nova_os_terminal_poc.html` with ordinary Bevy UI nodes and left a
manual visual comparison item. The owner now wants the current in-game
`drawer.rs` NOVA OS to look exactly like that HTML example before the command
tasks proceed, while keeping only `help` and `clear` executable for now.

## Decision

Run a focused UI-fidelity task on the existing drawer-owned Bevy monitor tree.
The task will reshape the screen to the PoC's single terminal surface, prompt
row, topbar, footer hints, casing, bezel, scanline/glass treatment and palette.
It will not implement planned command output or apps. The existing objective and
flight-log state stays as backing logic for future command tasks, but it is not
rendered as permanent panes in the open drawer.

The CRT pass should use a drawer-specific Bevy UI material/WGSL overlay when it
is practical in the current UI stack. If that path is blocked by Bevy UI
constraints, the task must keep a UI-node fallback and record the blocker plus a
follow-up task.

## Alternatives considered

- **Reopen `20260726-115320`** - rejected because that task is already closed,
  reviewed and retroed. A new task preserves the historical record and owns the
  stricter fidelity pass.
- **Fold the visual work into command tasks** - rejected because command output
  and app runtime should not be built on a layout that is still being reshaped.
- **Implement the POC commands now** - rejected because `log`, `objectives`,
  `ship`, `map`, `ship viewer`, `exit`, `reload` and `repair` already have
  separate task scope or are explicitly deferred by the owner.

## Consequences

The command tasks should wait behind this visual pass. Tests should assert the
spawned widget tree and command registry because pixel-perfect Bevy rendering is
not a stable local gate on software GPU. Human visual comparison remains a
manual acceptance item, and the produced artifact must be opened before
close-out.
