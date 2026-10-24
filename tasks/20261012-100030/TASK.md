# Epic: charters, discovery log and end goals

- STATUS: OPEN
- PRIORITY: 54
- TAGS: v0.16.0,epic,charters

## User facts
- Stage 6 of the game expansion. Decisions 2026-10-12: each of the six start roles has one finite end goal (mining empire, own yard, station network, kill list, Line Warship, outlive a civilization; decision 13); a discovery log and charters point the player at the first station; a capstone build flips the rings (D4 refresh).
- 2026-10-18 (owner, option "No world refresh"): the six role charters are optional sandbox milestones, saved and displayed; a met charter triggers no thaw refresh and no ending. This replaces the capstone ring flip (D4 refresh) above for the charters.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Charters:` title prefix and the `charters` tag.
1. Discovery log and charter 0.
2. Per-role end goals as saved facts.

## Done when
- A role's goal is visible from the start, progresses across reloads, and completing it is a saved fact shown on the Map; the world does not react to it.

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

## Owner feedback 2026-10-17 (round 2, chart, load, new game, goals)
- The chart already looks good in the game. Improve it, do not replace it:
  keep the 3D orbit view, show more useful things in chart mode, and draw
  generated images for cells instead of coloured squares, as the local map
  draws its contacts.
- Load: show the saved world's chart as the same 3D view, with zoom and
  orbit, beside the play time and the saved-ago time.
- New Game: the seed panel shows the world preview as the chart (zoom,
  orbit), as Factorio previews a map seed; then the role step.
- The `3 / 8` charter counts read as nonsense to the owner. Show the
  current objectives (held jobs, the next milestone) instead of goal counts,
  in the chart panel, on Load and on the HUD.
- Role descriptions are bad; the owner rewrites them. Do not spend time on
  them.
- Goals must be balanced across roles. "Own 4 stations" against "own 2" is
  not acceptable. One owned station is the shared first milestone for every
  role (the "launch the rocket" moment). After it each role gets one simple
  flavour goal: industrial something about mining, military something about
  clearing enemies, and so on. Keep them simple. Revisit decision 13 and the
  phase 2 counts in `20261012-100033` under this direction.
- The `ui_gallery` example (`examples/playable/ui_gallery.rs`) is the mock
  the owner judges; round 2 rework in progress.

## Owner feedback 2026-10-17 (round 3) and next up in the game
- Chart is approved as the gallery shows it. Load shows only what the save
  has (ship, owned ships, escorts, owned stations when present) and only
  OPEN objectives, never met ones. New Game keeps the game's two modal
  panels: the world preview on the first panel so the player sees the world
  first, the ship details when a class is selected on the second.
- Owner idea: autopilot travel between sectors. GOTO a station on the chart
  in another sector even with no live target: a sector destination for the
  autopilot, the streaming window follows, arrival when the station
  materializes. Owners: `nova_ship` autopilot, `nova_world` streaming, the
  Map's GOTO.
- Next up (game changes this epic owns):
  - `crates/nova_interface/src/map/chart.rs`: textured cell tiles generated
    per content in place of cuboid stripes; station mark variants (living,
    dead, owned, refusing, posting n jobs); the sector panel lists contents
    as icon rows with counts, the entered time, the finds, the last-seen
    stock with prices and the dock time, the jobs posted there, a route
    hint; the OBJECTIVES block (held jobs with `have / need`, the unmet
    milestone) replaces the CHARTER block; the discovery log gets icons and
    relative times; the legend becomes an icon row. Icons in `icons.rs`.
  - `crates/nova_menu/src/load_screen.rs`: rows with role icon, play time,
    saved-ago and the open objectives count; details with the save's 3D
    chart preview, a FLEET block (ship, owned ships, escorts, owned stations,
    only when present) and open objectives only; a Continue button for the
    most recent save in `menu_ui.rs`. Needs the save header to carry the
    play time, the hull name, the fleet and station counts and a chart
    summary (`crates/nova_world_base/src/save/mod.rs`, format bump, not
    shipped so no migration).
  - `crates/nova_menu/src/world_setup.rs`: step 1 adds the 3D chart preview
    of the seed (a preview call on the generator that draws the cells around
    the start without creating the world); step 2 adds the 3D hull preview
    in the details (move `hull_preview` out of `nova_interface` into a shared
    place) and the milestones; the owner rewrites the role texts.
  - Goals rebalanced per the round 2 direction (landed 2026-10-24 under
    `20261012-100033`): one owned station is the shared first line of every
    role's goal, then one flavour line each (ore deliveries, recoveries, a
    station network, a kill list, patrols, an outlaw's name). HUD objective
    chips stay undone: the owner dropped them from the rebalance scope.
  - Docs: `wiki/interface.md`, `wiki/getting-started.md`, `create/start-roles.md`,
    changelog entries under the subsystem headings.

## Progress 2026-10-18 (Map objectives)
- Claim: the chart view's OBJECTIVES block replaces the CHARTER block. With
  nothing selected it lists charter 0's open step, each held job (kind icon,
  title, `have / need` or `done`, hand-in `STN-n`) and the role's next unmet
  milestone, or one `Met at <play time>` row. Met milestones and goal counts
  are gone. A held job's row selects its JOB mark.
- Evidence: `crates/nova_interface/src/map/chart.rs:1330` (`ObjectiveRowType`),
  `:1359` (`objective_rows`, the pure list), `:1411` (the system, shown at
  `:1430` only with nothing selected, as the gallery shows it), `:921`
  (`spawn_icon_row_breaking`: the charter line wraps instead of clipping);
  `crates/nova_interface/src/jobs.rs:386` (`progress`, now `pub(crate)`,
  shared with the Jobs pane); `crates/nova_interface/src/icons.rs:595`
  (`ChartIconType::Milestone`, the gallery's pennant).
- Change: `charter_block_lines` and `MapChartCharterMarker` are deleted. The
  system keeps the name `update_map_chart_charter` because
  `crates/nova_interface/src/map/mod.rs:181` registers it and another lane
  held that file; rename it to `update_map_chart_objectives` in both places.
  The goal's `title` no longer shows on the Map; `web/src/create/start-roles.md`
  still names the CHARTER block (lines 63, 70, 101, 132) and needs the
  goals-rebalance rewrite.
- Verification: `the_objectives_list_the_open_charter_step_held_jobs_and_only_the_next_unmet_milestone`
  (31 map tests pass). `system_world_chart` claim 10 under lavapipe: `world
  chart: PASS the objectives read [... "Bring 7 Water ice", "0 / 7", "STN-1",
  "Charter 0: undock from your starting station", "Stations owned", "0 / 1"]`,
  `PASS the loaded chart holds 3 cells`, `autopilot: cycle complete, no panic`.
  `map-objectives.png` shows the block; the first capture clipped the charter
  line, fixed by the wrap.

## Progress 2026-10-18 (Load and New Game)
- Claim: the save header (format 22) carries play time, hull name, fleet
  counts, open objectives and a chart summary. Evidence:
  `crates/nova_world_base/src/save/mod.rs:85`, `:124`, `:142`, `:238`;
  `save/session.rs:667` (owned ships), `:692` (open objectives only, a met
  goal is skipped). Verification: `cargo test -p nova_world_base --lib save`
  43 passed, including
  `a_save_header_carries_the_play_time_hull_fleet_open_objectives_and_chart`
  and `a_met_goal_is_not_listed_among_a_save_header_s_open_objectives`.
- Claim: New Game step 1 previews the seed without creating the world.
  Evidence: `crates/nova_world_base/src/start.rs:165` (`start_preview`, 5 by 5
  cells round the allied start), `crates/nova_menu/src/world_setup.rs:826`.
  Step 2 shows the hull turning and the milestones (`world_setup.rs:652`,
  `:679`). Verification: `start` tests 2 passed; nova_menu
  `new_game_previews_the_seed_s_start_and_lists_the_role_s_milestones`.
- Claim: Load rows read play time, saved-ago and open count; the details draw
  the save's chart, a FLEET block (counts only when present) and the open
  objectives; Continue heads the menu. Evidence:
  `crates/nova_menu/src/load_screen.rs:702`, `:820`, `:1092`, `:365`, `:388`;
  `menu_ui.rs:133`, `:718`. Verification: nova_menu 220 passed, including
  `a_load_row_reads_play_time_save_age_and_open_count_and_its_details_the_fleet_and_objectives`
  and `continue_names_the_newest_save_and_resumes_it`.
- Change: the chart scene and the hull view live in `crates/nova_hud/src/preview/`
  (`PreviewPlugin`, `spawn_chart_scene` at `chart.rs:227`,
  `spawn_hull_preview_scene` at `hull.rs:147`); the Map, the Station cards and
  the menu (`crates/nova_menu/src/preview.rs`) draw through them.
- Fix found in the live run: the menu backdrop's `stage_menu_camera` posed
  every `Camera3d`, preview cameras included, and the Station cards'
  system switched off every hull view camera, the menu's too. Now the first
  takes the scenario camera only (`crates/nova_menu/src/ambience.rs:306`,
  `:320`) and the second its own cameras only
  (`crates/nova_interface/src/station/hull_preview.rs:165`). A hidden preview
  stops rendering (`crates/nova_menu/src/preview.rs:329`).
- Live proof: `screenshot_menu` (`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`, lavapipe)
  walks the world step, the role step, Create, a leave to the menu, Continue
  and Load, and asserts the Load row's open count, the FLEET ship line and an
  open objective. Frames: `news-0150-world-setup.png`, `menu-role-step.png`,
  `menu-continue.png`, `menu-load-screen.png`.
- Claim (a94bbc96c): each start role draws its own icon on the New Game role
  rows and on the Load rows. `StartRole` takes a required
  `icon: RoleIconType` (Drill, Claw, Container, Star, Shield, Blades); a
  missing or unknown icon fails lint and load. Evidence:
  `crates/nova_gameplay/src/role_icon.rs:19`,
  `crates/nova_scenario/src/objects/start_role.rs:186`;
  `crates/nova_interface/src/icons.rs:112` (`InterfaceIcons::role`), `:801`
  (`ROLE_ICONS`), `:826` (one procedural mask per shape);
  `crates/nova_menu/src/world_setup.rs:72` (18 px), `:462`;
  `crates/nova_menu/src/load_screen.rs:763`. A Load row whose role a mod
  reload dropped keeps an empty icon slot. Verification:
  `cargo test -p nova_menu --lib` filtered to
  `a_load_row_reads_play_time_save_age_and_open_count_and_its_details_the_fleet_and_objectives`
  and `new_game_previews_the_seed_s_start_and_lists_the_role_s_milestones`,
  2 passed (2026-10-19).

## Design 2026-10-19: cross-sector station GOTO (owner review pending)
- Agent design for the round 3 owner idea, not approved: `STATION-GOTO.md`.
- Finding: the window already follows the player ship (`WorldObserver`), so
  the work is a far leg to the station's cell and a handoff to the existing
  entity GOTO when the station materializes.
- Stop: gates G1-G10 in the note (owner crate, names, aim point, arrival,
  failure, save, speed cap, targets, proof, G key). No code before the answers.
