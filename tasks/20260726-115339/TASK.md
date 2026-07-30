# NOVA OS ship computer: 3D ship app + `ship` CLI verbs on section codes

- STATUS: CLOSED
- PRIORITY: 29
- TAGS: v0.9.0,stretch,feature,ui,hud,gameplay
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

As a player using NOVA OS, I want `ship` to open an interactive ship computer
that shows my ship as a 3D green-phosphor schematic with selectable, labelled
sections, and a `ship` CLI with `view`/`section`/`reload`/`repair` verbs that
target sections by a short code - so the terminal becomes a real
ship-management computer. v0.9.0 stretch; must not block the core terminal OS.

This is a re-scope of the original "ship viewer" task. The concrete build shape
(command surface, 3D render approach, section-code IDs, and arcade-now /
queued-later action semantics) is fixed in `DECISION.md` - read it first.

## Steps

### A. Section codes (the human/CLI/label handle)

- [x] Add a lightweight `SectionCode` component (e.g. `HULL-3`, `THR-1`,
      `PDC-1`, `TRB-1`, `CTL-1`) and a gameplay system that assigns codes
      stably per session to the live player-ship sections from their
      `SectionKind` + a stable index. Underlying `EntityId` is unchanged.
- [x] Expose a resolver from a typed code back to the section entity (used by
      both the CLI handler and Tab completion). Case-insensitive input.

### B. `ship` command surface (nova_os)

- [x] Replace the builtin `ship` snapshot command: `ship` becomes an APP
      command (launch word `ship`), and the status summary moves to a
      `ship view` `CliOutput::Snapshot` subcommand. Register the app + subs via
      the plugin, mirroring `map` / `map view`.
- [x] Add `ship section <id>`, `ship reload <id>`, `ship repair <id>`
      subcommands with `CommandArity::UpTo(1)`.
- [x] Deliver parsed arguments to gameplay: add a command-invocation pathway so
      an arg-bearing command yields a structured request the gameplay layer
      handles (arg-bearing reads like `ship section <id>` produce their result
      rows in gameplay; mutators raise a `ShipSectionCommand`). No inline
      mutation or dynamic-row logic inside `nova_os`.
- [x] Per-command dynamic Tab completion for the `<id>` argument: completing
      `ship repair <stem>` offers the live `SectionCode` set (extends the
      completion system, which today only completes command names + universal
      verbs).

### C. Deferred action handler (arcade now, queued-ready)

- [x] A gameplay handler system drains `ShipSectionCommand { target, action }`
      requests and applies them: `Reload` refills the target weapon section's
      `SectionAmmo` to capacity; `Repair` restores the section `Health` to max.
      Instant and free for now. Structure it as the seam a future queued/
      resource-costed job will plug into (see DECISION follow-ups). Unknown or
      inapplicable targets (e.g. `reload` on a hull) report a friendly error row.

### D. 3D ship app (nova_gameplay), map-app pattern

- [x] `ShipApp` `NovaOsAppRuntime` launched by `ship`, rendering into `<main>`
      via a dedicated `Camera3d` + `RenderTarget::Image` on an isolated
      `RenderLayers`, composited through the existing CRT shader (reuse the
      `nova_os_map.rs` RTT/orbit scaffolding; factor shared helpers if clean).
- [x] Build unlit green-phosphor proxy blocks from each live section's
      `SectionCollider` + local `Transform`; orbit camera (drag / Q-E-R-F /
      wheel-zoom / T-reset) matching the map app's controls and `hints()`.
- [x] Projected UI blips: `world_to_viewport` each section to an
      absolutely-positioned `Button` label showing its `SectionCode`; click or
      cycle (`[`/`]`) to select. Selected block/blip highlights amber.
- [x] Inspector readout for the selected section: code, kind, integrity % +
      meter, a word status line (nominal/scored/degraded/critical/inactive),
      and ammo for weapon sections - from live data.
- [x] In-app actions on the selected section drive the SAME
      `ShipSectionCommand` path as the CLI: a `reload` key (weapon sections)
      and a `repair` key, with disabled actions explaining why in a note line.
- [x] Destroyed sections: despawned leaf sections do not render; 0-HP
      `SectionInactiveMarker` sections render dim/dashed and read "inactive".

### E. Tests, example, notes

- [x] Tests: app launch/exit; code assignment + resolver; `ship view` rows from
      live data; `ship section <id>` detail; `reload`/`repair` mutate live
      section state via the handler; id Tab completion; block build from
      colliders; selection (click + cycle).
- [x] Refresh the `screenshot_nova_os` example (or add a ship-app variant) so
      the 3D schematic is exercised end to end and catalog smoke-list passes.
- [x] Update `tasks/20260726-115339/NOTES.md`: final data model, the
      command-invocation pathway, the queued/resource extension seam, and
      self-reflection.

## Definition of Done

- `ship` launches the ship computer app and exits back to the terminal.
  (test: `ship_app_launches_and_exits`)
- Sections get stable short codes; a code resolves to the right section.
  (test: `section_codes_assigned_and_resolve`)
- `ship view` prints the live section status summary; `ship section <id>`
  prints that section's detail from live data.
  (test: `ship_view_and_section_detail_rows_from_live_data`)
- The app renders proxy blocks from live section colliders and selects a
  section by click and by cycle, updating the inspector without mutating
  gameplay state. (test: `ship_app_renders_blocks_and_selects_section`)
- `ship reload <id>` / `ship repair <id>` (and the in-app action keys) mutate
  the target section's ammo / integrity through the `ShipSectionCommand`
  handler; the pathway carries the parsed argument. (test:
  `ship_reload_and_repair_apply_through_command_handler`)
- Tab completing a `<id>` argument offers the live section codes. (test:
  `ship_verb_id_tab_completion`)
- Touched nova_os + nova_gameplay tests pass. (cmd:
  `nix develop --command cargo test -p nova_os -p nova_gameplay ship nova_os`)
- The ship 3D schematic is exercised by an example and the catalog smoke-list
  passes. (manual: owner confirms the in-game look/feel of the 3D schematic and
  actions)

## Deferred to follow-up tasks (see DECISION.md)

- Queued / over-time action execution while the drawer is closed.
- Hull-stored resource model, action costs, combat lockout, "why disabled"
  notes.
- Ship inventory panel in the app.

## Notes

- Depends on: `20260726-115334` (done), builds on the unified `TerminalCommand`
  model (`20260727-231546`), the 3D `map` app (`20260724-102320`), and the
  persistent layout (`20260728-085741`).
- Epic: `tasks/20260725-104330/TASK.md`. Spike: same folder `SPIKE.md`.
- Concrete build-shape forks fixed in `tasks/20260726-115339/DECISION.md`.
- Stretch: cut before the core monitor/input/output/app-runtime tasks if
  v0.9.0 needs to tighten.
