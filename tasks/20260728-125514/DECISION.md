# Decision: The blip defers HP/ammo detail to the panel; it keeps only a status-coloured dot + a contrast-backed label

- DATE: 20260728-125514
- STATUS: ACCEPTED
- TASK: 20260728-125514
- TAGS: decision, nova_os, ship, hud, ui
- Supersedes: tasks/20260728-115435/DECISION.md

## Context

`20260728-115435` moved section STATUS off the block colour and onto the blip as
an integrity bar + ammo pips, because at that point the blip was the only
per-section surface in the 3D view. Since then `20260728-115430` landed the side
inspector panel, which shows the selected section's full detail (integrity %/meter,
HP, status, ammo). Playtest (2026-07-28) on the blip overlay: the labels are "tiny
and green on green," and the ammo pips render one circle per round ("literally 500
circles"). With the panel now holding the detail, the blip overlay is redundant
and cluttered.

This supersedes ONLY the "status rides the blip integrity bar + ammo pips" part of
`20260728-115435`. The other parts of that decision stand: blocks stay uniform
green, kind is shown by a glyph, separation comes from the wireframe outline + gap.

## Decision

The blip becomes a minimal marker:

- The integrity BAR and ammo PIPS are removed from the blip. HP, ammo and the
  word-status live in the inspector panel.
- The blip DOT is coloured by status (green nominal -> amber critical), so a
  damaged section is spottable across the whole ship without selecting each one -
  a single glanceable cue that costs no clutter.
- Every blip keeps a label (kind glyph + section code), made readable with a dark
  backing pill + brighter text, rather than tiny green-on-green.
- Selection stays the amber dot border.

## Alternatives considered

- **Label only the selected/hovered blip** (bare dots otherwise). Most minimal,
  but the owner chose all-labels-with-contrast so every section code is legible at
  a glance without selecting.
- **Neutral dots, status only in the panel.** Rejected: you could not see which
  sections are damaged without clicking through them; the status-coloured dot is a
  cheap glanceable health map.
- **Keep a compact numeric ammo on the blip.** Deferred to the panel (which shows
  `rounds/capacity`); keeping it on the blip re-clutters the thing we are
  minimising.

## Consequences

- The 3D view reads as a clean set of labelled, status-coloured markers; detail is
  one selection away in the panel. Removes the "500 circles" and the green-on-green
  readability problem.
- Status is no longer shown as a precise bar on the blip - only the dot hue. A
  player who wants the exact integrity selects the section and reads the panel.
  Acceptable: the panel is always present beside the view.
- `ShipSectionView::bar_fraction` and `ammo_pips` lose their only callers and are
  removed.
