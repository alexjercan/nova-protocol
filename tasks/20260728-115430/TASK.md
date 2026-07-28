# NOVA OS ship app: side inspector panel with section detail + repair/reload buttons

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.9.0,feedback,feature,ui,hud,gameplay

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

## Notes

- Reuse `section_detail_rows` content and `apply_action_to_section` /
  `ShipSectionCommand` from `crates/nova_gameplay/src/hud/nova_os_ship.rs`.
- Buttons inside the RTT composite must use the `Activate` observer, not
  `Interaction` polling (`rtt-ui-select-via-activate-not-interaction`).
- Follows `20260726-115339` (the ship app). Sibling of the section-rendering
  legibility task.
