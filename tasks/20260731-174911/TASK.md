# NOVA OS: the objectives/flight-log row lists are dead in production

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, refactor, hud

## Story

Found during the v0.9.0 KISS pass (20260731-170322). Nothing in the NOVA OS
shell ever spawns `NovaOsObjectivesListMarker` or `NovaOsFlightLogListMarker`
- only the tests do. The monitor dropped its permanent panes; objectives and
the flight log now reach the player only through the `objectives` / `log`
terminal commands.

That leaves the whole row-building path in `hud/nova_os/lists.rs` reachable
only from tests: `rebuild_nova_os_objectives`, `rebuild_nova_os_flight_log`,
both empty-state row spawners, `spawn_nova_os_objective_row`,
`spawn_nova_os_flight_log_row` and `spawn_nova_os_flight_log_icon`, plus the
`nova_os_lists_just_spawned` run condition that gates them.

The `NovaOsFlightLog` resource itself is NOT dead - it feeds `terminal_log_rows`
and the boot banner's unread count. Only the UI row rendering is.

Two smaller pieces of the same residue:

- `NovaOsObjectiveRowStatus` has one variant (`Active`) and is always set to
  it, so the tests asserting it are tautological.
- `NovaOsObjectiveStrikeMarker` is `#[cfg(test)]`-only and never spawned; the
  test asserting zero of them exist is vacuous.

## Steps

- [x] Confirm with the owner that the panes are gone for good, not parked.
      Owner confirmed 2026-08-12: the tabs are gone from the NOVA OS computer.
- [x] Delete the dead row-spawn path and the components only it used.
- [x] Delete or rewrite the tests that only covered the dead path.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `grep -rn "NovaOsObjectivesListMarker\|NovaOsFlightLogListMarker" crates/` - no hits.
3. test: the `objectives` and `log` terminal commands still print their rows.

## Notes

Do not delete `NovaOsFlightLog` or its sync system - the terminal commands and
the boot banner read it.

2026-08-12: the L9 crate split (20260806-121625) had moved the dead path from
`hud/nova_os/lists.rs` to `crates/nova_os_ui/src/terminal/lists.rs` unchanged.
Done in this task (see NOTES.md for proofs):

- Deleted the row-spawn path, the 14 components only it used, the
  `nova_os_lists_just_spawned` run condition and the 5 row-layout style
  constants.
- Renamed the survivor: `terminal/lists.rs` -> `terminal/flight_log.rs`, now
  only `sync_nova_os_logs` + `announce_objectives_in_terminal`.
- Tests: deleted the row-rendering tests; rewrote the model tests
  (comms append, edit-in-place, interleave) against `NovaOsFlightLog`
  entries; kept the scroll-viewport and teardown tests
  (`tests/lists.rs` -> `tests/flight_log.rs`).
