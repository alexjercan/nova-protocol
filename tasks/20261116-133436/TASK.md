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

## Map (2026-11-20; `MAP.md`, frames `before/flow-sheet-{1,2,3}.png`)
- Correction to the finding above: TAB does nothing while the catalogue
  is open. TAB closes the interface when the catalogue is closed, and a
  part in hand is lost with no cue (frames 22-23).
- Every Place or Swap closes the catalogue; a second part needs the footer
  button again, and the filter is reset (frames 10, 14, 20).
- The catalogue has no close of its own; the top bar Close shuts the whole
  interface (frame 24).
- The editor opens and closes the same gallery with TAB
  (`crates/nova_editor/src/gallery.rs:26-31,193-207`); in game TAB closes
  the interface. Likely owner path: Place, then TAB to get back to the
  catalogue, and the interface closes.

## Decisions (owner approved option A, 2026-11-20)
Keep the full-cover catalogue (owner: the elements look decent; the
editor shares it). Fix how it opens, returns and closes:
1. TAB, Ship pane at a refit station: opens and closes the catalogue, as
   in the editor. With a part in hand, TAB puts it away and opens the
   catalogue. Every other pane: TAB closes the interface, as now.
   (Option B, rejected: TAB always closes the interface, and the open
   catalogue and the part in hand come back on reopen.)
2. Place and Swap: the catalogue hides and the part is in hand, as now.
   After an add the part stays in hand. TAB returns to the catalogue on
   the same category, filter and scroll.
3. Esc steps back one layer: filter caret, card, catalogue, part in hand,
   then the interface. No layer is skipped.
4. The footer Catalogue button carries its key chip. The top bar Close
   chip names the key that closes the interface in the current state.
5. The wheel over the grid scrolls it.
6. No refit: the Catalogue button stays hidden (owner rule: a verb that
   cannot work is hidden).
7. Out of scope: a side drawer beside the ship view; Swap target change
   inside the catalogue; the Craft view keys.
- Proof: ECS asserted example for TAB and Esc with the catalogue open,
  closed and with a part in hand; before/after flow frames.

## Delivery record (2026-11-20)
- Landed 2d0bbd800: one TAB decision in `toggle_interface`
  (`crates/nova_interface/src/pane.rs`), ordered before the gallery input;
  `GalleryState::resumed()` keeps category, filter, Available chip and
  page in game (the editor keeps a fresh unfiltered browse); wheel pages
  the grid (game and editor); Catalogue key chip; Close chip names Esc at
  a refit dock. The Catalogue button click also puts a held part away.
  Esc chain needed no code change; now proven.
- Proof: 6 ECS tests (`terminal/tests/catalogue_keys.rs`), button chip,
  wheel and resume tests; asserted walk `system_catalogue_keys` at
  1280x720 and 1024x768; `system_station_{add,swap,swap_intake,rebuild,
  round_trip}` and `news_0160_refit_catalogue` pass. After frames:
  `~/Pictures/nova-catalogue-after/` (not in the repo).
- `news_0160_refit_catalogue` failed before this change: the start kit
  now holds section panels the PDC recipe takes; the capture stages an
  empty hold (separate commit).
- Not verified: hand play; `station_docked_sandbox` live (stalls under
  lavapipe, compile only). Stale picture: `wiki-interface-ship.png`.
- Open until the owner hand-checks the flow.
