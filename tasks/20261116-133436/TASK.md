# Catalogue UX: open, browse and return without leaving the interface

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.19.0, ui, shipyard

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- The section catalogue in the ship viewer is "a bit meh to use".
- The catalogue UI elements look decent. The problem is how you open it and
  navigate back: it feels awkward.
- "pressing tab takes you out of the UI", which is low UX.
- Owner rule on record: editor and interface density stays low; one top
  bar, context menus, no big buttons or extra bars.

## Agent findings
- The catalogue opens only from the Ship pane footer Catalogue button,
  while docked at a station that refits
  (`crates/nova_interface/src/ship/catalogue.rs:1-8,300-346`).
- When open, a gallery host covers the view, the panel and the footer
  (`catalogue.rs:4-8,64`). It closes with Esc (`catalogue.rs:424`), on a
  tab switch, or when the interface closes (`catalogue.rs:329-342`).
- TAB toggles the whole interface (`crates/nova_interface/src/bindings.rs:52`,
  `crates/nova_interface/src/pane.rs:132-158`). TAB inside the catalogue
  closes the catalogue and the interface together.
- Place puts the part in hand (`HeldPart`); the ghost shows where a click
  adds it. Swap raises `SectionSwapCommand` (`catalogue.rs:20-23`).
- Unverified: exact flow from catalogue to Place and back to the catalogue
  for a second part; map this in a live run before the design.

## Delivery
1. Live walk of the current flow (open, browse, place, swap, return, close)
   with frames. Record each step and each awkward step.
2. Short design: how the catalogue opens, where it sits beside the ship
   view, how Esc and TAB behave (TAB must not drop the player out of the
   catalogue without intent), and how the player returns after a Place.
   Owner approves.
3. Implement. The crafting UI (task 20261116-132140) reuses the result.

## Verification
- ECS asserted example for the key rules (Esc and TAB behaviour while the
  catalogue is open).
- Before/after frames of the flow, inspected.

## Done when
- The owner can open the catalogue, place or swap several parts and get back
  to the ship view without leaving the interface by accident.
