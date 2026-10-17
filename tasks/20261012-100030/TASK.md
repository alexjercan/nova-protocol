# Epic: charters, discovery log and end goals

- STATUS: OPEN
- PRIORITY: 54
- TAGS: v0.16.0,epic,charters

## User facts
- Stage 6 of the game expansion. Decisions 2026-10-12: each of the six start roles has one finite end goal (mining empire, own yard, station network, kill list, Line Warship, outlive a civilization; decision 13); a discovery log and charters point the player at the first station; a capstone build flips the rings (D4 refresh).
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Charters:` title prefix and the `charters` tag.
1. Discovery log and charter 0.
2. Per-role end goals as saved facts.

## Done when
- A role's goal is visible from the start, progresses across reloads, and completing it is a saved fact the world reacts to.

## Children
- `20261012-100031` Charters: discovery log and charter 0
- `20261012-100033` Charters: per-role end goals as saved facts

## Owner feedback 2026-10-16
- The chart map (landed under `20261012-100007`) feels underwhelming: it does not show what is available in a visited cell in a readable way, and the legend carries too much text. Revisit it with the charter work here.

## Owner feedback 2026-10-17 (UI, ideas only, nothing implemented)
The chart view, the CHARTER block and the discovery log are text blocks, and
the menu screens that feed a charter (New Game, Load) are text too. Ideas:

### Chart view (`crates/nova_interface/src/map/chart.rs`)
1. Cells as tiles: a visited cell shows up to four small glyphs for what it
   holds (asteroids, rocks, planets, derelicts, station, wreck, job) instead
   of the `ASTEROIDS` / `ROCKS` word tag. Add the glyphs to `icons.rs` and the
   legend becomes one icon row, no sentences.
2. Station marks as icon variants, not colour alone: living, dead, owned,
   refusing (economy window), posting jobs. A small count on the mark for the
   jobs posted there, read off the frozen board.
3. Hover or select a cell and the side panel lists its contents as rows with
   glyphs and counts, the play time entered, and the finds, instead of one
   paragraph.
4. Route hint: the sector offset from the player to the selected station or
   job mark ("3 sectors east, 1 north"), since the chart has no GOTO.

### CHARTER block
5. Each count as a labelled meter (`slider_track`) with `have / need` at the
   end, the title as a `panel_head`, and `STANDING` / `MET` as a `badge`.
   The met stamp stays as a line under it.
6. Charter also on the HUD as a collapsed objective chip ("Station network
   3/8 jobs") that expands on hover; and on the Load page row (see below).

### Discovery log
7. Rows with the find's glyph, the code, the sector and a relative play time
   ("4 min ago"), grouped by session; the newest find pulses once when the
   chart opens so the player sees what changed.

### Load page (`crates/nova_menu/src/load_screen.rs`)
8. Rows carry the start role's hull glyph, the charter progress as a small
   meter, the play time and a relative saved time ("2 h ago"); sort by last
   played; a `Continue` button on the main menu for the most recent world.
9. Details pane as a card: the role hull in the `hull_preview` 3D view, then
   Seed, Sector, Credits, Play time, Charter `have / need`, Saved; the version
   only as a badge when it differs from the running build.

### New Game (`crates/nova_menu/src/world_setup.rs`)
10. The role step as cards, one per role, with the hull in the `hull_preview`
    3D view, the charter title and its counts ("Station network: 8 jobs, 4
    stations, 2 rings"), credits and kit as chips; the lore paragraph behind
    the card, in the details pane, not on it.
11. World name and seed on one line with Randomize as an icon button; the
    profile paragraph folds behind a `What is a world?` disclosure. One
    screen, no Next step, Create at the bottom.
12. A seed preview: the ring-0 chart of the seed (four or five cells around
    the start) drawn with the chart tiles, so the seed field means something.

### Across the interface
13. One pattern: list rows with glyphs and badges on the left, an inspector
    with the prose on the right, as the Inventory pane already does. The
    Station pane, the chart view and the Load page each do their own thing
    today.
14. Numbers align in fixed columns; sentences move to tooltips or the
    inspector; the Danger colour is for refusals that need action now, not
    for every reason a button is off.

## Review 2026-10-17 (unpushed range to 0763909fe, read at HEAD)
- nit (counts): `KillList` counts completed Neutralize jobs, not distinct
  hulls (`crates/nova_world_base/src/goal.rs:122-128`); two boards with
  overlapping cubes can name one hull, so one kill reads as two. Two Recover
  jobs over one stack pay twice (`jobs.rs:687`). Decide: distinct hulls, or
  cross-board exclusion of unheld offers.
- nit (ordering): `lib.rs:253-256` says the goal stamps before the save
  snapshots, but `save_systems()` is only after `chart_docked_station`
  (`lib.rs:283-291`), so a `met_at` can land in the next save. The clock
  invariant still holds.
- Sound: `GoalRecord::validate`, the met-with-no-goal refusal, no stamp on the
  Load frame, owned stations consumed once on materialize, `ring_of` and the
  lint bounds, charter 0 on `OnStart` only with the living-station filter,
  `OwnYard` best single station, `MiningEmpire` without a stock clause.
