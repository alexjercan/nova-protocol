# NOVA OS map view: labelled KIND/LABEL/INFO table + map goto <label>

- STATUS: CLOSED
- PRIORITY: 27
- TAGS: v0.9.0,feature,ui,hud

## Story

As a player running `map view` in the NOVA OS terminal, I want the contact list
laid out as the same fixed-width KIND/LABEL/INFO table `ship view` uses, and I
want every contact to carry a short, unique, typeable LABEL (like ship's
`HULL-1` / `PDC-1` section codes) - e.g. `SELF`, `HOST-1`, `AST-2`, `OBJ-1` - so
I can read the list at a glance AND copy a label straight into a new
`map goto <label>` command that flies the ship there.

This mirrors the `ship view` label-table work (task 20260728-125510 /
20260726-115339): ship sections got stable `SectionCode`s and a KIND/LABEL/INFO
table; map contacts get the same treatment plus the `goto` verb the labels exist
to feed.

## Design decisions (confirmed with owner before planning)

- LABEL mechanism: mint a stable `MapContactCode` component on each contact
  entity, once per session, never reassigned - exactly the `assign_section_codes`
  pattern. Labels stay pinned to an entity even as the range-sorted list
  reorders, so `map goto <label>` resolves to a fixed target. (Not ephemeral
  per-frame labels.)
- Scope: labels + KIND/LABEL/INFO table reformat + a `map goto <label>` CLI verb
  (this run, not later). The verb reuses the existing app GOTO seam
  (`Autopilot::engage(AutopilotAction::Goto { target })`).

## Label scheme

Per-kind prefix + stable index, assigned deterministically (sorted by each
contact's `EntityId` when present, else entity bits, so indices are stable within
a session):

- OwnShip  -> `SELF` (exactly one; no index)
- Ally     -> `ALLY-n`
- Hostile  -> `HOST-n`
- Objective-> `OBJ-n`
- Terrain  -> `AST-n`

## `map view` output shape (mirror `ship view`)

```
LOCAL SPACE - contacts
Contacts: N
KIND       LABEL   INFO
OWN SHIP   SELF    range 0 u
HOSTILE    HOST-1  120 u  045 mark +10
TERRAIN    AST-1   60 u  180 mark +00
```

- Header row `KIND  LABEL  INFO`, columns padded to the widest cell (header
  included), monospace-aligned with a two-space gutter - identical mechanism to
  `terminal_ship_rows`.
- INFO carries what the old RANGE/BEARING columns did (`range 0 u` for own ship;
  `<range> u  <bearing> mark <mark>` for others).
- Own ship first, then ascending range (unchanged ordering).
- Empty state (`no contacts in local space`) preserved.

## Cross-app CLI dispatch

`NovaOsTerminal` has a single `pending_invocation` slot and
`apply_ship_cli_commands` takes it unconditionally, erroring on unknown verbs.
Adding `map goto` as a second gameplay-verb app collides. Fix: add a peek
(`peek_pending_invocation`) so each handler only `take`s an invocation whose name
it owns; update the ship handler to peek-then-take too. No behavior change for
ship.

## Steps

- [x] Add `MapContactCode(pub String)` component + prelude export; add an
      `assign_map_contact_codes` system that mints codes for contact entities
      lacking one (per-kind index, stable sort key, never reassign), mirroring
      `assign_section_codes`.
- [x] Thread the code into the contact model: `MapContact` gains `code: String`;
      `MapContacts` reads `Option<&MapContactCode>` and fills it (fallback to the
      uppercased name / kind when unminted).
- [x] Reformat `map_rows_from_contacts` into the KIND/LABEL/INFO table (header +
      padded columns + INFO range/bearing), keeping own-ship-first + range sort
      and the empty state. Update the app blip label to show the code.
- [x] Add `peek_pending_invocation` to `NovaOsTerminal`; make
      `apply_ship_cli_commands` peek-then-take only its own verbs.
- [x] Register `map goto` (`CommandArity::UpTo(1)`) in the map command tree; add
      `apply_map_cli_commands` that resolves the label (case-insensitive) to a
      contact and sets the flight `Autopilot` GOTO on the player ship, with
      unknown-label + can't-goto-self result rows.
- [x] Add `sync_map_arg_completions` so `map goto <TAB>` offers the live codes.
- [x] Tests: `map view` renders a KIND/LABEL/INFO header + aligned columns;
      contact codes are unique per kind; `map goto <label>` sets the autopilot on
      the player ship and rejects `SELF` / unknown labels.
- [x] Update docs (`web/src/wiki/hud.md`) for the new `map view` table + `map goto`.

## Definition of Done

1. `map view` prints a KIND/LABEL/INFO table with aligned columns and a unique
   code per contact (cmd: `cargo test -p nova_gameplay map_view` covers the
   format + uniqueness).
2. `map goto <label>` engages the flight autopilot toward the labelled contact
   and rejects `SELF`/unknown labels (cmd: `cargo test -p nova_gameplay map_goto`).
3. Ship CLI verbs still work unchanged after the peek-then-take refactor
   (cmd: existing `ship view` / `ship section` tests stay green).
4. `cargo check` + `cargo fmt --check` clean; blip labels show the codes
   (manual: open the `map` app, confirm each blip reads its code).

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED
