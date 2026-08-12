# Notes

## What / why

Dead-code removal. The NOVA OS monitor dropped its permanent objective /
flight-log panes in the PoC-terminal rework; the row-building path survived
because only tests spawned its containers. Deleted the path, kept the model.

Deleted:

- `rebuild_nova_os_objectives`, `rebuild_nova_os_flight_log`, both empty-state
  spawners, `spawn_nova_os_objective_row`, `spawn_nova_os_flight_log_row`,
  `spawn_nova_os_flight_log_icon`.
- `nova_os_lists_just_spawned` run condition (shell.rs).
- Components: both list markers, row/text/glyph/empty markers for both lists,
  `NovaOsObjectiveId`, single-variant `NovaOsObjectiveRowStatus`,
  test-only `NovaOsObjectiveStrikeMarker`, `NovaOsFlightLogIconMarker` +
  `NovaOsFlightLogIconKind`.
- Style constants only the spawners used: `DRAWER_ROW_GAP_PX`,
  `DRAWER_ROW_PADDING_X/Y_PX`, `DRAWER_OBJECTIVE_GLYPH_WIDTH_PX`,
  `DRAWER_LOG_ICON_SIZE_PX`.

Kept and renamed: `terminal/lists.rs` -> `terminal/flight_log.rs` with
`sync_nova_os_logs` and `announce_objectives_in_terminal`. `NovaOsFlightLog`
and `NovaOsScrollViewportMarker` stay live (terminal commands, boot banner
unread count, wheel scroll).

Tests: `tests/lists.rs` -> `tests/flight_log.rs`. Row-rendering tests deleted.
Model behavior tests (comms append, objective edit-in-place, interleave)
rewritten to assert on `NovaOsFlightLog` entries via `nova_os_flight_log_text`
instead of UI rows. Scroll-viewport and teardown tests kept as-is.
`objectives_app` rig shrank to `sync_nova_os_logs` only.

## Tradeoffs

- The model tests now assert on the resource, not a UI tree. That is the real
  production surface: the `log` / `objectives` commands and the boot banner
  read the resource. UI-facing coverage for the commands already lives in
  `tests/commands.rs`.
- Rename over keeping the `lists` name: a module named "lists" with no lists
  is the same residue the task removes.

## DoD proofs (2026-08-12)

1. `nix develop --command cargo check --workspace --all-targets` - green,
   no new warnings.
2. `grep -rn "NovaOsObjectivesListMarker\|NovaOsFlightLogListMarker" crates/`
   - no hits.
3. `cargo test --lib -p nova_os_ui` - 99 passed, 0 failed; includes
   `terminal_log_command_prints_flight_log_rows` and
   `terminal_objectives_command_prints_active_objectives`.
4. `cargo fmt --check` - clean.
