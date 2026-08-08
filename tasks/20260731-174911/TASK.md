# NOVA OS: the objectives/flight-log row lists are dead in production

- STATUS: OPEN
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

- [ ] Confirm with the owner that the panes are gone for good, not parked.
- [ ] Delete the dead row-spawn path and the components only it used.
- [ ] Delete or rewrite the tests that only covered the dead path.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `grep -rn "NovaOsObjectivesListMarker\|NovaOsFlightLogListMarker" crates/` - no hits.
3. test: the `objectives` and `log` terminal commands still print their rows.

## Notes

Do not delete `NovaOsFlightLog` or its sync system - the terminal commands and
the boot banner read it.
