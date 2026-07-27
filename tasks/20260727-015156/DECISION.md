# DECISION - NOVA OS rename + crate extraction shape

- STATUS: ACCEPTED

Note: the task itself is still OPEN, awaiting its /flow plan-gate approval; this
record documents the load-bearing forks already resolved for that plan.

## Context

`drawer` is the legacy internal name for the NOVA OS ship-computer terminal
(`nova_gameplay/src/hud/drawer.rs`, ~5900 lines). Naming is a half-finished
rename to NOVA OS, and the file mixes OS logic with bevy UI. The task asks to
finish the rename and "might want to create a crate nova_os ... and have it
refactored". Two load-bearing forks had to be resolved before planning.

## D1 - Blast radius of the rename: FULL CONSISTENCY

Rename every "drawer" identifier in the NOVA OS subsystem, INCLUDING the
cross-crate public surface: `PauseStates::Drawer` -> `PauseStates::NovaOs`,
`DrawerTabAnchor` -> `NovaOsTabAnchor`, `HudDrawerExempt` -> `HudNovaOsExempt`.
Updates `nova_menu`, `nova_core`, `input/player.rs`, `objective_hint.rs`,
`objective_reveal.rs`.

Alternatives rejected: keeping `PauseStates::Drawer` as a "generic freeze axis"
name - rejected because the owner chose full consistency; the variant is only
ever entered for NOVA OS, so the generic-axis argument does not hold.

## D2 - Crate topology: nova_os = LOGIC only; UI stays in nova_gameplay

Extract ONLY the OS logic (shell command language, terminal model, app
runtime/registry) into a new `nova_os` crate. The bevy UI (casing/CRT/nodes/
slide/keyboard systems), the game-data bridges (objectives/flight-log/ship ->
terminal rows), `PauseStates`, and the plugin STAY in `nova_gameplay`, which
gains a dependency on `nova_os`.

Hard constraint from the owner: `nova_ui` MUST NOT depend on `nova_os`. Only
tiny, model-independent visual helpers (e.g. a screw/vent/recessed-plate
builder) may move into `nova_ui`; `nova_gameplay` then depends on BOTH crates.

Dependency graph (acyclic):

    nova_os  <-- nova_gameplay --> nova_ui
    (logic)      (UI + wiring)      (generic chrome, no nova_os dep)

Alternatives rejected:
- Extracting the WHOLE subsystem (UI included) into `nova_os`: would force
  `nova_os` to depend on `nova_gameplay` for `PauseStates`/`GameStates`/
  `NovaHudSystems` while `nova_gameplay` registers the plugin and reads the
  drawer axis for HUD visibility - a CIRCULAR dependency requiring the plugin
  registration to move to `nova_core`. The owner's logic-only split avoids the
  cycle entirely (single direction `nova_gameplay -> nova_os`).
- Moving the NOVA OS UI into `nova_ui`: rejected by the owner's constraint that
  `nova_ui` must not depend on `nova_os` (the UI reads the terminal model).
- In-place rename with no crate: rejected; the owner wants the logic isolated
  and refactored out of the monolith.
- Renaming the `nova_ui` crate itself: not intended; "renamed" referred to the
  drawer->nova_os identifier rename, not the crate.

## D3 - nova_os MAY depend on bevy

Engine-free is NOT required. `nova_os` depending on `bevy` is consistent with
`nova_ui` and `nova_events`, and is needed for the app-runtime trait, `Key`
input handling, and `Handle<Font>` in row styling. (The only engine-free crate,
`nova_mod_format`, is a wire-schema crate and not a precedent here.)
