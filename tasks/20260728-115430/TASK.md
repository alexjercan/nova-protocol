# NOVA OS ship app: side inspector panel with section detail + repair/reload buttons

- PRIORITY: 30
- TAGS: v0.9.0, feedback, feature, ui, hud, gameplay
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a player using the NOVA OS `ship` app, I want a side panel that shows full
details about the selected section and gives me repair/reload buttons, so I can
inspect and act on a section without leaving the schematic or using the CLI.

Playtest verdict (2026-07-28): the 3D schematic app landed in `20260726-115339`
"looks COOL / AMAZING". This is the owner's follow-up polish request #1 (and the
panel half of request #2: "when I click I get full details in side panel +
buttons for repair/reload").

## What it should do

- Restructure the ship app body (currently one full-bleed viewport + a single
  readout line) into a row: the 3D viewport (flex-grow) beside a fixed-width
  **inspector panel** on the right, CRT/sci-fi styled.
- The panel shows the SELECTED section's full detail from live data: code, name,
  kind + what it does, integrity % + meter, word status, ammo (weapons), and
  headroom for the future queued-jobs/resources view.
- **Repair / Reload buttons** in the panel drive the existing
  `ShipSectionCommand` seam (the same path as the `L`/`P` keys and the CLI
  verbs). Disabled-with-reason when N/A (reload on a hull, nothing to repair),
  echoing the PoC's `actionNote` UX.
- Selecting a section (click a blip or `[`/`]`) updates the panel; keep the mouse
  optional.

## Approach

Restructure `ShipApp::spawn_body` so the body's Column holds a single Row that
grows: `[viewport (flex-grow)  |  fixed-width inspector panel]`. The panel is a
bordered CRT container (matching the case/bezel styling) with a title, a
multi-line live detail block, a Repair/Reload action row, and a note/reason line.
The single bottom `ShipReadoutMarker` line is REMOVED - the panel is the detail
surface now (and it is the home the blip-overlay-minimize task `20260728-125514`
defers detail to, which is why this task comes first).

Detail + action state are derived from the already-live `ShipSectionView` via two
pure helpers so they unit-test cleanly:
- `panel_detail_text(view) -> String` (multi-line: kind + what it does, integrity
  % + meter, status, HP, ammo).
- `panel_action_state(view) -> PanelActions { repair_enabled, reload_enabled, reason }`
  keyed off the SAME validity `apply_action_to_section` enforces (Reload =
  `Turret`/`Torpedo` with `Some(ammo)`; Repair = `Health` with `max > 0`).

Buttons route through the existing `ShipSectionCommand` seam (same path as `L`/`P`
and the CLI verbs) via `On<Activate>` observers; the observer targets
`runtime.selected` and no-ops when the panel marked that action disabled (enabled
flags cached on `ShipRuntime`). Panel content refreshes each frame in a
`update_ship_panel` system that replaces `update_ship_readout`, mirroring its
selected-view + transient-note pattern.

## Steps

- [x] Add `kind_description(SectionDamageClass) -> &'static str` (a short "what it
      does" phrase per kind) in `nova_os_ship.rs`.
- [x] Add pure helpers `panel_detail_text(&ShipSectionView) -> String` and
      `panel_action_state(&ShipSectionView) -> PanelActions` (struct with
      `repair_enabled: bool`, `reload_enabled: bool`, `reason: Option<String>`),
      deriving validity from kind/health/ammo exactly as `apply_action_to_section`.
- [x] Restructure `ShipApp::spawn_body`: replace the viewport+readout children
      with a flex-grow Row containing the existing `ShipViewportMarker` viewport
      (flex-grow) and a new fixed-width bordered `ShipPanelMarker` panel. Build the
      panel subtree with marked children: `ShipPanelTitleMarker` (code + name),
      `ShipPanelDetailMarker` (multi-line detail Text), a Repair button
      (`ShipRepairButtonMarker`) and Reload button (`ShipReloadButtonMarker`) each
      with a label, and a `ShipPanelNoteMarker` reason/result line. Style with the
      NOVA_OS case/bezel colours + `nova_os_text_font`.
- [x] Wire the buttons: `.observe(on_ship_repair_button)` /
      `.observe(on_ship_reload_button)`, each reading `runtime.selected` + the
      cached enabled flag and writing a `ShipSectionCommand` (Activate, not
      Interaction: `rtt-ui-select-via-activate-not-interaction`).
- [x] Replace `update_ship_readout` with `update_ship_panel`: set the title,
      detail (via `panel_detail_text`), compute `panel_action_state`, cache
      `repair_enabled`/`reload_enabled` on `ShipRuntime`, dim the disabled
      button(s) (opacity/muted colour) and set the note line to the transient
      `runtime.note` (amber) or the disabled reason or a hint. Remove
      `ShipReadoutMarker` and its spawn; update the system chain in the plugin.
- [x] Tests: unit-test `panel_action_state` (hull -> reload disabled with reason,
      repair enabled; critical turret w/ ammo -> both enabled; a no-health section
      -> repair disabled) and `panel_detail_text` (contains code/kind/integrity/
      status/ammo); a live-tree test that builds the body, selects a section, runs
      `update_ship_panel` and asserts the `ShipPanelDetailMarker` text reflects it;
      and an observer test that `on_ship_repair_button`/`on_ship_reload_button`
      writes a `ShipSectionCommand` for the selected section (pin each button path
      at its own boundary - `pin-each-caller-not-just-shared-core`).

## Definition of Done

- The ship app body is a row: the 3D viewport (flex-grow) beside a fixed-width
  bordered inspector panel on the right
  (cmd: `grep -n "ShipPanelMarker" crates/nova_gameplay/src/hud/nova_os_ship.rs`;
  manual: the panel sits to the right of the schematic, CRT-styled).
- The panel shows the selected section's full live detail - code, name, kind +
  what it does, integrity % + meter, status, ammo for weapons
  (test: `panel_detail_text_covers_live_fields`; manual: the panel updates as I
  select different sections via click or `[`/`]`).
- Repair/Reload buttons drive the `ShipSectionCommand` seam and are
  disabled-with-reason when N/A (reload on a hull, repair with no integrity)
  (test: `panel_action_state_gates_repair_and_reload`,
  `panel_buttons_raise_ship_section_command`; manual: clicking Repair/Reload acts
  on the selected section and a disabled button shows why).
- The mouse stays optional: `L`/`P` keys and `[`/`]` selection still work
  (test: existing `ship_action_keys_mutate_through_message_handler` still passes;
  manual: keyboard-only still inspects + acts).
- The CRT/phosphor palette is preserved: no new hue constants
  (cmd: `grep -n "srgb" crates/nova_gameplay/src/hud/nova_os_ship.rs`).

## Notes

- Reuse `section_detail_rows` content and `apply_action_to_section` /
  `ShipSectionCommand` from `crates/nova_gameplay/src/hud/nova_os_ship.rs`.
- Buttons inside the RTT composite must use the `Activate` observer, not
  `Interaction` polling (`rtt-ui-select-via-activate-not-interaction`).
- Follows `20260726-115339` (the ship app). Sibling of the section-rendering
  legibility task.

## Work Log (close-out)

**What changed** (`crates/nova_gameplay/src/hud/nova_os_ship.rs`):

- `ShipApp::spawn_body` now lays a flex-grow Row: the existing `ShipViewportMarker`
  viewport (flex-grow) beside a fixed-width (`SHIP_PANEL_PX` = 232) bordered
  `ShipPanelMarker` column. The single bottom `ShipReadoutMarker` line is REMOVED
  and `update_ship_readout` deleted - the panel is the detail surface now (this is
  the home the blip-minimize task `20260728-125514` defers detail to).
- Panel subtree: a title, a multi-line detail block, a Repair/Reload action row,
  and a note line. The three info texts carry a `ShipPanelField` enum
  (Title/Detail/Note) so one query refreshes them; the two buttons carry a
  `ShipPanelButton` enum (Repair/Reload).
- Pure helpers drive content + validity: `panel_detail_text` (kind + what it does,
  integrity %/meter, HP, status, ammo) and `panel_action_state` -> `PanelActions
  { repair_enabled, reload_enabled, reason }`, keyed off the SAME conditions
  `apply_action_to_section` enforces (Reload = `Turret`/`Torpedo` with `Some(ammo)`;
  Repair = `Health` with `max > 0`), so the buttons never disagree with the handler.
  Added `kind_description` for the "what it does" line.
- `update_ship_panel` (replacing `update_ship_readout`, same slot in the chain)
  sets the title/detail, caches `panel_repair_enabled`/`panel_reload_enabled` on
  `ShipRuntime`, dims disabled buttons, and shows the transient `runtime.note`
  (amber) or the disabled reason or a key hint on the note line.
- `on_ship_repair_button` / `on_ship_reload_button` are `On<Activate>` observers
  (not Interaction) that raise a `ShipSectionCommand` for `runtime.selected`, and
  no-op when the panel marked that action disabled. Same seam as `L`/`P` + CLI.

**Tests.** `panel_action_state_gates_repair_and_reload`,
`panel_detail_text_covers_live_fields`, and `panel_buttons_raise_ship_section_command`
(triggers `Activate` on a button and asserts the disabled path is a no-op and the
enabled path refills ammo through the seam - pins the button entry point per
`pin-each-caller-not-just-shared-core`). All 15 `nova_os_ship` tests pass; `cargo
check` (non-test build) clean; `cargo fmt` clean.

**Visual verification.** `screenshot_nova_os` harness (real GPU, exit 0):
`nova-os-ship.png` shows the bordered panel right of the narrowed viewport with
`CTL-1 Basic Controller Section`, `kind: controller`, the description,
`integrity: 100% [##########]`, `100/100 HP`, `status: nominal`, an enabled bright
`P Repair`, a dimmed `L Reload`, and the reason `reload: CTL-1 is a controller
section, no ammo feed`. The row restructure did not break the RTT viewport.

**Note on scope.** No DECISION.md: the task specified the artifact (row +
fixed-width panel + repair/reload buttons); the only judgement call - removing the
bottom readout line in favour of the panel - was flagged at the plan gate and
accepted, and is recorded here.

**Self-reflection.** Reusing the `apply_action_to_section` validity in a single
`panel_action_state` helper (rather than re-deriving button-enable logic) kept the
buttons and handler in lockstep and made the gate unit-testable. The screenshot
range still has no weapon section, so the ENABLED Reload button + ammo line were
verified by test/ECS, not on screen - the same coverage gap noted last cycle
(`new-render-primitive-verify-on-gpu`); an armed-ship capture fixture would close it.
