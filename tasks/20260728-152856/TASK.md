# NOVA OS ship view: show section labels in a column-aligned table

- STATUS: CLOSED
- PRIORITY: 27
- TAGS: v0.9.0, feedback, feature, ui, hud

## Story

As a player running `ship view` in the NOVA OS terminal, I want each section row
to show its LABEL (the short code like `CTL-1` / `HULL-1` / `PDC-1`) instead of
the freeform name (`Controller Cube`), and I want the rows laid out as a
fixed-width column table with a header, so it is easy to read AND easy to copy a
label straight into the `id`-taking commands (`ship repair <id>`,
`ship reload <id>`, `ship section <id>`).

Playtest request (2026-07-28): "instead of showing stuff like `CONTROLLER
Controller Cube (..)` it should show `CONTROLLER CTL-1` - show the labels instead
of the name to have it easier in the `id` required commands, and format it nicer
like a table with <Tab> style spacing (equal spacing, just text, not a markdown
table) - maybe with a header Kind, Label, Info."

## What it should do

- Replace the freeform section name in each `ship view` row with the section
  CODE (its `SectionCode`, the same id `ship <verb> <id>` accepts).
- Lay the section rows out as a monospace column table: a header row
  (`KIND  LABEL  INFO`) followed by one row per section, every column padded to
  the widest cell so columns line up (plain space padding, NOT a markdown table).
- Keep the status signal: rows stay colour-coded by status
  (`section_status_row_kind`), and the non-nominal status word moves INTO the
  INFO column (e.g. `[critical]`) instead of the current separate `  status:` sub-row.

## Notes

- `ship view` rows are built by the pure `terminal_ship_rows(ship_name, &[ShipSectionStatus])`
  in `crates/nova_gameplay/src/hud/nova_os.rs` (currently prints
  `{KIND} {name} - {health}{ammo}` + an indented `  status:` row when not nominal).
- `ShipSectionStatus` (same file) has `name/kind/health/inactive/zero_health/ammo`
  but NO code. The code is `SectionCode(pub String)` (`pub`, defined in
  `nova_os_ship.rs:98`, minted by `assign_section_codes`).
- `player_ship_snapshot` builds the `Vec<ShipSectionStatus>` from an ECS query
  that does NOT currently fetch `SectionCode`.
- Sibling of the just-landed ship-app terminal-polish tasks (`20260728-115430`
  side inspector, `20260728-125510` orbit recenter). Terminal is monospace, so
  space-padding aligns columns.

## Decision

Layout confirmed at the plan gate (2026-07-28): "Full" - keep the `SHIP {name}`
and `Sections: {n}` preamble, a `KIND  LABEL  INFO` header, and fold the
non-nominal status word into the INFO column (`[critical]` / `[neutralized]`),
dropping the separate `  status:` sub-row. Row colour still driven by status.

## Design

Pure-formatting change plus a small wiring extension to carry the code:

- Add `code: String` to `ShipSectionStatus`. Add `Option<&SectionCode>` to
  `player_ship_snapshot`'s section query and populate `code` from it, falling
  back to the uppercase kind label when a section has no code yet (mirrors the
  existing name fallback; keeps it panic-free before `assign_section_codes`).
  `import super::nova_os_ship::SectionCode` into `nova_os.rs`.
- Rewrite `terminal_ship_rows` to emit a table:
  - Keep the identity rows: `SHIP {NAME}` (Info) and `Sections: {n}` (Dim).
  - Compute column widths: `w_kind = max(len("KIND"), max kind-label len)`,
    `w_label = max(len("LABEL"), max code len)`. INFO is the last column (no pad).
  - Emit a Dim header row: `KIND<pad>  LABEL<pad>  INFO` (two-space gutter).
  - One row per section (kept in the existing sort order): the kind label and
    code padded to their column widths, then INFO =
    `{health_text}{ammo_suffix}` with `  [{status}]` appended when status is not
    nominal. Row `TerminalRowKind` stays `section_status_row_kind(section)`.
  - Drop the separate `  status:` sub-row (its information now rides the INFO
    column + the row colour).
- The empty-ship branch (`SHIP {name}` + "No live player ship sections detected.")
  is unchanged.

## Steps

- [x] Add `code: String` to `ShipSectionStatus`; fetch `Option<&SectionCode>` in
      `player_ship_snapshot` and populate `code` (fallback = uppercase kind label);
      import `SectionCode` into `nova_os.rs`.
- [x] Rewrite `terminal_ship_rows` to emit the padded `KIND/LABEL/INFO` table
      (header row + per-section rows, status folded into INFO, row colour kept).
- [x] Update the existing `ship_view_rows_format_section_status` unit test to the
      new table format (assert the header row, code shown, columns aligned, status
      in INFO, name absent).
- [x] Add/extend a LIVE `ship view` test that submits the command through the
      terminal and asserts the scrollback contains the section CODE label (proves
      the code is threaded from the ECS query to the row, not just the pure
      formatter - `test-the-wiring-system-not-just-its-pure-helpers`).
- [x] Run the check suite.

## Definition of Done

1. `ship view` rows show each section's CODE label (e.g. `HULL-1`), not the
   freeform name. (test: updated `ship_view_rows_format_section_status` asserts a
   row contains the code and NOT the name)
2. Rows are column-aligned under a `KIND  LABEL  INFO` header - every data row's
   LABEL and INFO columns start at the same character offset. (test: alignment
   assertion in the formatter test)
3. Non-nominal status is still surfaced (in the INFO column + row colour); the
   separate `  status:` sub-row is gone. (test: formatter test)
4. The section code is threaded from the live ECS snapshot, not just the pure
   helper. (test: live `ship view` submit test asserts the code in scrollback)
5. Check suite green. (cmd: `cargo check -p nova_gameplay`)
6. Playtest: owner runs `ship view` and sees the aligned label table.
   (manual: owner confirms in a run)
