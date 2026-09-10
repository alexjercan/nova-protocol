# Systems ranges: the session loop, settings, mods and the picker

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.14.0, testing, examples, ui

## Goal

Live proof for the flows a store player hits before the first shot: the
session loop, settings that survive a restart, a broken mod, the pause
menu, and the picker. From the 2026-09-09 coverage map: `system_menu_boot`
asserts one direction only, `system_headless_rebind` asserts the settings
store is INERT, and nothing proves the picker starts what was clicked.

Owner (2026-09-09): "add more systems examples tests ... create systems
examples for all edge cases and bugs we find."

Rules: `examples/systems/README.md`. Each range gets its `[[example]]`
block, its slugs on the roster in
`crates/nova_probe_cli/tests/catalog_drift.rs`, and lands in the `ui` shard
of `20260909-213100`. A range that finds a defect fixes it in the same lane
and records the fix on this task.

## Ranges

- [ ] `system_session_loop`: main menu -> New Game -> ESC -> Back to Main
      Menu -> New Game again -> ESC -> Retry, by real pointer clicks over
      `editor_app`. Invariants: new game reaches gameplay twice; back to
      menu tears the scenario down; the ambience backdrop comes back; the
      second run carries nothing from the first (entity count, resources,
      HUD tiers); retry reloads without a menu round trip; the HUD is built
      once per run. The double-teardown command error is the way a store
      build hard-crashes.
- [ ] `system_settings_persist`: rebind `main_drive`, move the master volume,
      change the graphics preset, rebuild the app on the same
      `SettingsStoreRoot`. Needs an explicit `SettingsStoreAccess::ReadWrite`
      on a temp root, because `harness_env_active()` pins the store inert
      under `NOVA_AUTOPILOT`. Invariants: the panel writes the store; a
      relaunched app boots on the saved keybind; the rebound key drives the
      verb after the reload; an older store loads on serde defaults; reset
      clears the override.
- [ ] `system_broken_mod`: install a bundle with a dangling section id,
      enable it in the Mods screen, start its scenario. Invariants: the lint
      names the reference; the start raises FAILED TO START, not a panic;
      the base game still starts afterwards; disabling clears the gate; a
      mod with a missing dependency will not enable.
- [ ] `system_pause_settings`: pause -> Settings -> change a tab -> Back ->
      Resume, and pause -> Retry over a live scenario. Invariants: the modal
      shares the main-menu body; Escape out of the modal returns to the
      overlay, not to flight; the clock stays frozen through the modal;
      retry reloads the live scenario; the cursor is given back on resume.
- [ ] `system_scenario_launch`: pick a row and start it, then let a campaign
      chain to its next scenario. Invariants: the picked row launches its
      own scenario; a hidden scenario has no row; a campaign header groups
      its scenarios; the picker returns with the same selection.

## Done when

Five ranges green in `probe run ui` three times in a row, roster count
updated, every defect they found fixed and listed here.
