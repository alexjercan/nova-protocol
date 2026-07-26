# Decision: NOVA OS app runtime shape and app-exit input

- TASK: 20260726-115334
- STATUS: ACCEPTED
- DATE: 20260726-115334

## Context

The app runtime is the seam the future `map` (`20260724-102320`) and `ship
viewer` (`20260726-115339`) tasks plug into. Two load-bearing forks had to be
settled before building: (1) how apps register and render, and (2) what returns
from an app to the terminal. Both were surfaced to the owner at the flow plan
gate because the candidates are mutually exclusive in maintainability terms and
the exit binding conflicts with the existing Escape=close-drawer route.

## Decision 1: app-as-plugin (trait objects)

Each NOVA OS app is its own runtime object implementing a
`NovaOsAppRuntime` trait (`id`, `title`, `summary`, `spawn_body`, and optional
per-app input), registered into a `NovaOsAppRegistry` resource as
`Box<dyn NovaOsAppRuntime>`. The drawer owns the generic parts: the
`TerminalMode::App { .. }` transition, input ownership gating, uniform app exit
(chrome close + Escape), and the app chrome (title bar + close control + body
slot). Future apps register their own runtime and spawn arbitrary UI (including
a 3D viewport for `map`) into the body slot; they never edit a central match arm
in `drawer.rs`.

Rejected alternatives:

- Enum + match in `drawer.rs`: simplest now, but every future app edits the
  enum and every match arm, coupling `map`/`ship viewer` back into `drawer.rs`.
- Resource registry of plain descriptors + generic chrome: decoupled for
  text-body apps, but a descriptor cannot express a `map` 3D viewport without
  growing back toward the trait seam anyway.

The owner chose the most decoupled seam; the extra scaffolding is accepted per
the "correct and maintainable over faster" rule.

## Decision 2: Escape is the app-exit key, context-sensitive

Escape becomes context-sensitive rather than always closing the drawer:

- In app mode, Escape (and the on-screen chrome close control) exits the app
  and returns to the terminal with scrollback and prompt state intact. The
  drawer does NOT close.
- In terminal/prompt mode, Escape closes the whole NOVA OS computer, exactly as
  today.

This overrides the spike's recommendation ("keep Escape as drawer close; give
apps a `Ctrl+C`/`Ctrl+[` chord") and exercises the task's own escape hatch in
Step 4 ("unless implementation uncovers a hard input conflict"). The owner
directed it explicitly. No `Ctrl+C`/`Ctrl+[` chord is added; `Ctrl+[` was also
ruled out independently because it is the ANSI escape sequence for Escape and
can be indistinguishable from the drawer-close key.

Feasibility: Escape in `PauseStates::Drawer` is handled only by
`close_drawer_from_menu_keys` in `drawer.rs` (nova_menu's `toggle_pause`
no-ops in `Drawer`). Guarding that system on `active_mode == Prompt` and adding
an app-mode Escape->exit path keeps the whole context switch inside `drawer.rs`
with no cross-crate input contention.

## Consequence for the Definition of Done

The DoD test that asserted "Escape still closes the whole drawer" in app mode is
re-scoped: in app mode Escape exits the app to the terminal; the drawer-close
route is proven from terminal mode instead.
