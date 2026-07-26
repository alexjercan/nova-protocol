# NOVA OS ship viewer app and safe section actions

- STATUS: OPEN
- PRIORITY: 29
- TAGS: v0.9.0,stretch,feature,ui,hud,gameplay

## Story

As a player using NOVA OS, I want a `ship viewer` app that shows my ship's
sections, HP and status with clickable section details, so that the terminal can
grow into a useful ship-management computer. This is v0.9.0 stretch and should
not block the core terminal OS.

## Steps

- [ ] Launch `ship viewer` through the NOVA OS app runtime, replacing terminal
      scrollback in the same monitor until the app exits.
- [ ] Render a schematic or simple spatial layout of the player's ship sections
      with labels, HP/status coloring and selected-section details.
- [ ] Reuse live section/status data from the player ship; do not invent a
      parallel ship model for the app.
- [ ] Add click selection for sections and a read-only inspector first.
- [ ] Decide whether any action belongs in v0.9.0. Prefer `reload` over
      `repair` if one action is included; leave `repair` deferred unless
      resource, combat-lockout and balance rules are explicit.
- [ ] Add tests for app launch, section rendering from live data, selection, and
      any included safe action.
- [ ] Add/update `tasks/20260726-115339/NOTES.md` with the data model, action
      scope decision, and self-reflection.

## Definition of Done

- `ship viewer` launches as a NOVA OS app and exits back to terminal mode.
  (test: `ship_viewer_app_launches_and_exits`)
- The app renders player ship sections with labels, HP/status and selected
  details from live data. (test: `ship_viewer_renders_live_section_status`)
- Section clicks update the inspector without mutating gameplay state. (test:
  `ship_viewer_selects_section_for_inspection`)
- Any mutating action included in this task has explicit rules and test coverage;
  otherwise the app remains read-only. (manual: owner confirms whether read-only
  v0.9.0 stretch is enough)
- Touched drawer/app tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer terminal`)

## Notes

- Depends on: `20260726-115334`.
- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- Stretch. Cut before the core monitor/input/output/app-runtime tasks if v0.9.0
  needs to tighten.
