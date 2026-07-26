# Review: NOVA OS terminal output commands

- TASK: 20260726-115330
- BRANCH: feature/nova-os-terminal-output-commands

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [ ] R1.1 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:2673 - the new
  command tests bypass the live ECS submission path by constructing
  `TerminalCommandSnapshot` directly, so they do not prove that commands read
  current data and update after resource changes. Add an App-driven test that
  submits a command through `handle_terminal_keyboard`, mutates live resources
  or components, submits again, and asserts the second output reflects current
  state.
  - Response: fixed by adding
    `terminal_objectives_command_reads_live_resource_updates`, which submits
    through `handle_terminal_keyboard`, changes `GameObjectives`, submits again
    and verifies the second output reflects the changed resource.
- [ ] R1.2 (MAJOR) crates/nova_gameplay/src/hud/drawer.rs:1244 - the live `ship`
  snapshot path is not covered by `terminal_ship_command_prints_section_status`;
  that test feeds prebuilt `ShipSectionStatus` rows and never verifies that a
  player root plus real `SectionMarker` children are discovered and classified.
  Add a test spawning a player ship and real section marker/component entities,
  then assert `ship` output includes the expected live rows.
  - Response: fixed by adding
    `terminal_ship_command_reads_live_player_sections`, which spawns a live
    player ship with real section marker/component children and verifies `ship`
    output through the keyboard submit path, including changed section health.

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 verified: the live objectives command test now submits through
  `handle_terminal_keyboard`, mutates `GameObjectives`, submits again, and
  asserts the changed output.
- [x] R1.2 verified: the live ship command test now spawns real player ship
  sections, submits through the keyboard path, and asserts refreshed output
  after a component mutation.
- Verification: `nix develop --command bash -lc 'cargo test -p nova_gameplay drawer && cargo test -p nova_gameplay terminal'`.
