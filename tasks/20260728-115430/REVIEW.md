# Review: NOVA OS ship app - side inspector panel

- TASK: 20260728-115430
- BRANCH: feature/ship-inspector-panel

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/nova_os_ship.rs:1737 -
  `update_ship_panel` has zero test coverage. Step 6 asked for a live-tree test
  that runs `update_ship_panel` and asserts the detail text reflects the
  selection; the step was ticked but only the pure helpers + the observer were
  tested. If `update_ship_panel` were reverted to a no-op every test would still
  pass. Add a live-tree test that builds the panel body, selects a section, sets
  `active`, runs `update_ship_panel`, and asserts the `ShipPanelField::Detail`
  text reflects the section and the enabled flags are cached.
  - Response: fixed - added `update_ship_panel_reflects_selection`: it builds the
    panel via `spawn_ship_panel`, selects the hull, runs `update_ship_panel`, and
    asserts the Detail text contains `kind: hull` + `status:`, the Title contains
    `HULL-1`, and `panel_reload_enabled == false` / `panel_repair_enabled == true`
    are cached. A no-op `update_ship_panel` leaves the placeholder detail and
    false flags, so it fails on revert.

- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/nova_os_ship.rs:2596 -
  `panel_buttons_raise_ship_section_command` only exercises the Reload observer;
  `on_ship_repair_button` is a separate caller and is never triggered. Pin both
  per `pin-each-caller-not-just-shared-core`.
  - Response: fixed - extended the test with a Repair-button case: spawn a
    `ShipPanelButton::Repair` observer, select the hull (80/100), toggle
    `panel_repair_enabled` false then true, trigger `Activate`, and assert Health
    stays 80 then restores to 100.

- [x] R1.3 (NIT) crates/nova_gameplay/src/hud/nova_os_ship.rs:1108,1295,1424 -
  three comments (and one test message at 2113) still say "readout" after the
  readout node was removed. Reword to the panel / note line.
  - Response: fixed - reworded all four ("panel note line" / "the panel" /
    "the panel buttons raise").

Verification notes (out-of-context reviewer, re-confirmed in-session):
- `cargo test -p nova_gameplay --lib nova_os_ship` -> 15 pass; `cargo check`
  (non-test build) clean, no dead_code.
- DoD `cmd:` proofs: `ShipPanelMarker` present; `srgb` unchanged from master
  (only `SHIP_VIEW_BG`), no new hue constants.
- Lockstep confirmed: `panel_action_state` derives enabled/reason from the same
  conditions + identical text `apply_action_to_section` uses; no drift.
- Row restructure keeps viewport `flex_grow: 1.0` beside the `flex_shrink: 0.0`
  fixed-width panel; `reconcile_ship_target` still sizes the RTT from the
  viewport `ComputedNode`. Observers use `MessageWriter` + `Activate`, not
  `Interaction`. No query-conflicts in `update_ship_panel`.

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (round-1 findings were test-coverage gaps + comment
  rewording; the fixes are mechanically verifiable and I re-derived the
  load-bearing fail-on-revert claim myself, so an out-of-context re-run was not
  needed for this constrained follow-up)

All three round-1 findings addressed; verified:
- R1.1: `update_ship_panel_reflects_selection` now runs the system and asserts the
  Detail/Title text + cached flags. Re-derived the fail-on-revert: the panel's
  initial Detail is the placeholder "Select a section:..." (no "kind: hull") and
  `panel_repair_enabled` defaults `false`; a no-op `update_ship_panel` leaves both,
  so the `contains("kind: hull")` and `panel_repair_enabled` asserts fail. Genuine
  pin.
- R1.2: the button test now triggers `Activate` on BOTH a Repair and a Reload
  observer, each with a disabled no-op case and an enabled routing case.
- R1.3: `grep -n readout` over the file returns nothing; all four sites reworded.
- Full suite: `cargo test -p nova_gameplay --lib nova_os_ship` -> 16 pass;
  `cargo fmt` clean.

Pending user checks (open `manual:` DoD items, not resolved by APPROVE):
- The panel sits to the right of the schematic, CRT-styled (confirmed in the
  screenshot capture, pending owner playtest).
- The panel updates as sections are selected via click or `[`/`]`.
- Clicking Repair/Reload acts on the selected section; a disabled button shows why
  (Reload disabled + reason confirmed in the screenshot).
- Keyboard-only (`L`/`P`, `[`/`]`) still inspects and acts.
