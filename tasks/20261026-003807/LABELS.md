# Dense-label inventory for #124 `Dense labels die`

Read-only pass. No code, git, cargo or game run. Date 2026-11-06, HEAD 51397ef7a.
Output of this file: a named list with proposed verdicts. The owner decides.

## Sources and method

- TASK.md `Dense labels die` (line 62) and the chart-label hierarchy line (37). Audit section 4 is the input.
- Frames, judged by opening each PNG:
  - R2 = `/tmp/nova-onboard/d2/bench/rec2/` (frames 913, 921, 924, 930, 933, 936, 939, 942, 946).
  - R1 = `/tmp/nova-onboard/d2/bench/rec/` (frame 216, local Map with contacts).
  - `/tmp/nova-onboard/bench/rec` (82 frames): menu, loading, Jobs board only. No new screen.
- Frame sizes are 1280x720. Seed 7, world 20261026, Mining Tug, docked at Pragar station.
- `no frame` = no frame in these sets shows it. I did not guess how it looks.
- Ship pane: no frame at all. Flight HUD: only frame 913 (docked, idle). Ammo, RCS pips, target inset, torpedo, notices: no frame.
- `SHARED` = the source file has uncommitted edits from another lane (`git status --short`, 2026-11-06).
- Wiki = `web/src/wiki/`. Both `interface.md` and `hud.md` are SHARED.

SHARED files that carry labels in this list: `nova_hud/src/{keybind_dock,flight_status,objective_stack}.rs`, `nova_interface/src/{jobs,pane}.rs`, `map/scene.rs`, `station/{mod,station_buy,hull_info}.rs`, `inventory/{app,market}.rs`, `ship/app.rs`, `open_world.rs`, wiki `interface.md`, `hud.md`, `getting-started.md`.
Clean files: `map/chart.rs`, `map/stance.rs`, `map/contacts.rs`, `map/app.rs`, `station/{escorts,shipyard}.rs`, `nova_core`, `nova_ui/src/status_bar.rs`, `nova_gameplay/src/chart.rs`.

Frame findings that matter:
- Frame 939 reads `STN-1 (2)`. Frame 930 (after accept) lists one offer on the board. The count and the list disagree. Cause not checked (stale count, or offers the starter board hides).
- Frame 936: the job panel reads `JOB-1`, then `JOB`, then `Cell (6, 0, 2)`, then `SECTOR (6, 0, 2)`. The kind and the cell are said twice.
- Frame 216: local Map labels collide. `ALLY-7` is under `ALLY-8`, `PLN-3` under `PLN-4`.
- Audit said the job panel names the hand-in station by code. Frame 936 shows the name (`Pragar station`), not `STN-n`.

## 1. Flight HUD (docked, idle)

| Label | Source | Frame | Player must know | Explained | Verdict |
| --- | --- | --- | --- | --- | --- |
| `TAKE A JOB` chip | `open_world.rs:48` (SHARED) | R2 frame_000913 | Open the interface and take a job | on screen (the footer) | KEEP (D1/D2 approved) |
| `Tab INTERFACE` footer | `objective_stack.rs:109,484` (SHARED) | R2 frame_000913 | Tab opens the interface | on screen; `getting-started.md:17`, `hud.md:170` | KEEP (D2 approved) |
| `STATION HELM` mode chip | `flight_status.rs:683` (SHARED) | R2 frame_000913 | The station flies the hull while docked | wiki `hud.md:65`; nowhere on screen | KEEP. State word, no duplicate. |
| `0.0 m/s` speed chip | `nova_ui/src/units.rs:47` | R2 frame_000913 | Own speed | self-evident | KEEP |
| Dock: `X STOP` `G GOTO` `O ORBIT` `Z CANCEL` `Ctrl RADAR` `Shift RCS` `D DOCK` `H HELM` | `keybind_dock.rs:90` (SHARED) | R2 frame_000913 | Which verbs exist and which key runs them | `hud.md:65` (fixed slots, dark = cannot press) | KEEP. Eight chips, but each has a fixed slot and a key. `D DOCK` is lit while docked; pressing it undocks. Not proposed: a state word is a redesign. |
| `60 fps` and `v 0.16.0+51397ef7a` | `nova_core/src/lib.rs:1078-1090` | R2 frame_000913 | Frame rate, build id | `hud.md:25,51` (Status kind) | KEEP. Fixed design; the build id is the bug-report key. |
| Ammo gauge, RCS pips, target inset, notices | `ammo_readout.rs`, `flight_status.rs`, `target_inset.rs` (SHARED) | no frame | n/a | n/a | KEEP, not judged: no frame |

## 2. Map, local view

| Label | Source | Frame | Player must know | Explained | Verdict |
| --- | --- | --- | --- | --- | --- |
| Legend `Own ship` `Ally ship` `Neutral ship` `Ally station` `Asteroid` `Planet` | `scene.rs:1344` (SHARED) | R1 frame_000216 | Icon meaning | on screen (icon beside word) | KEEP |
| Contact code `ALLY-n` `NEU-n` | `contacts.rs:94-103` | R1 frame_000216 | Which contact is which | `interface.md:75-83` | KEEP. Allies and neutrals are the ships a pilot acts on. |
| Contact code `PLN-n` | `contacts.rs:94-103`; always shown by `scene.rs:583` (SHARED) | R1 frame_000216 | A planet's id | `interface.md:82`; the legend `Planet` and the ringed icon | REMOVE (see shortlist 6) |
| Asteroid `AST-n` | `scene.rs:583` | hidden until selected; no frame of a selected rock | Terrain id | `interface.md:83` | KEEP |
| Panel `No contact`, `Click a contact to select it.`, `No GOTO set` | `scene.rs:1248,1219,1185` (SHARED) | R2 frame_000921 | How to pick a contact and set GOTO | on screen | KEEP |
| Buttons `GOTO`, `CANCEL`, `Reframe`, `Chart` | `map/app.rs:194,202`; `pane.rs:1563`; `chart.rs:1942` | R2 frame_000921 | Mouse routes for the same actions | on screen | KEEP. |
| Footer `Q/E Turn R/F Tilt W/A/S/D Pan Z/X Layer T Reframe [/] Select G GOTO C Chart` + dim `Click select Right-drag look Wheel zoom` | `pane.rs:1133-1168,1217-1218` (SHARED) | R2 frame_000921, frame_000936 | The view keys | `interface.md:85-91` | KEEP. Dense, but it is the only on-screen key list. `T Reframe` and `C Chart` repeat the buttons; the keys appear nowhere else, so no removal. |
| Panel fields for a selected contact (`Range`, `Bearing`, note) | `scene.rs:1244-1247` | no frame (no contact selected in any frame) | | `interface.md:100` | KEEP, not judged: no frame |

## 3. Map, chart view

| Label | Source | Frame | Player must know | Explained | Verdict |
| --- | --- | --- | --- | --- | --- |
| Legend `Asteroids Rocks Planet Derelicts Station Wreck Job You` | `chart.rs:1930-1938` | R2 frame_000936 | Icon meaning | on screen | KEEP |
| Mark `STN-1` | `nova_gameplay/src/chart.rs:225-230`; `chart.rs:722` | R2 frame_000939 | Found-order code of a station; same code in the log, FINDS and the row tail | `interface.md:95`; nowhere on screen | KEEP. The code links the mark to the list rows. |
| Mark `STN-1 (2)` count `(k)` | `chart.rs:722-725` | R2 frame_000939 | Number of open jobs posted there | `interface.md:95`; nowhere on screen | REMOVE (see shortlist 4) |
| Mark `JOB-1` | `jobs.rs` (gameplay) `mark_code`; `chart.rs:170` | R2 frame_000939, frame_000936 | Accept-order code of a held job | `interface.md:95` | KEEP |
| Mark `YOU` | `chart.rs:744-745` | R2 frame_000939 | Own ship | self-evident, legend `You` | KEEP |
| Cell tag `(6, 0, 2)` at the tile corner | `chart.rs:606-615` | R2 frame_000939 | Sector coordinate; also the click handle for the tile | `interface.md:95` ("click a sector's coordinate"); the axes nowhere | KEEP. Only on-map carrier of the coordinate and a select target. |
| Red ring, dead-station fade | `chart.rs:5-11` | no frame | | `interface.md:95` | KEEP, not judged: no frame |
| Heading `OBJECTIVES  Y0` | `chart.rs:1447` | R2 frame_000939 | `Y0` is the layer number | tooltip on the heading only (`chart.rs:1438`); `interface.md:95` | RENAME to `OBJECTIVES  LAYER 0` (shortlist 1) |
| Objective row `JOB-1 Bring 5 Iron ore  0 / 5  STN-1` | `chart.rs:1472-1490` | R2 frame_000939 | Held job, progress, hand-in station code | `interface.md:95` | KEEP |
| Role rows `Ore to market` over `Stations owned 0 / 1` | `chart.rs:1494-1520`; text from content | R2 frame_000939 | The start role's goal and next milestone | `interface.md:95`; nowhere on screen | KEEP. Text comes from authored content. |
| `DISCOVERY LOG` rows `STN-1 Pragar station  (6, 0, 2)  just now`, `ASTEROIDS  (6, 0, 2)  just now` | `chart.rs:1647-1670` | R2 frame_000939 | Find, sector, age | `interface.md:95` | KEEP. The station name clips at the panel edge in the frame; the clip is not a text issue. |
| `STANCES` row `Pragar [ALLY]` | `stance.rs:331,91-93` | R2 frame_000939 | Civilization and its stance toward you | `interface.md:95` | KEEP |
| Chip `ADV 1%` | `stance.rs:226-230`, chip `:481-490` | R2 frame_000939 | Seeded advancement of the civilization. Its own tooltip says it is not ship tech or progression. | tooltip `stance.rs:484`; `interface.md:95` | MOVE to hover (shortlist 5). It is the only on-screen source of the number, so it cannot be removed. |
| Stance detail lines `HOSTILE`, `CLEAR`, `SEEDED`, `FINE n @ STN-n`, `FINE NONE`, `OFFENCES a/b`, `MERITS a/b`, `DOCK STN-n`, `STN-n REFUSED` | `stance.rs:159-194,305-307` | no frame (the Ally row has no details) | | each has a tooltip (`stance.rs:159-194`); `interface.md:95` | KEEP, not judged: no frame |
| Job panel heading `JOB-1`, then title `Bring 5 Iron ore` | `chart.rs:1847` | R2 frame_000936 | The selected job | `interface.md:95` | KEEP |
| Job panel kind line `JOB` | `chart.rs:1852-1854` | R2 frame_000936 | The mark is a job | said by the `JOB-1` heading and the legend `Job` | REMOVE (shortlist 3) |
| Job panel `Cell (6, 0, 2)` | `chart.rs:1855-1858` | R2 frame_000936 | The job's sector | `SECTOR (6, 0, 2)` sits 6 lines below | REMOVE (shortlist 2) |
| Job panel `Reward 270 cr` | `chart.rs:1860-1863` | R2 frame_000936 | Reward | `interface.md:95` | KEEP. Jobs pane held rows carry no reward. |
| Job panel note `Posted by Pragar station. ... Hand in at Pragar station while docked.` | `chart.rs:1866-1878` | R2 frame_000936 | Who posted it, where to hand in | on screen | KEEP |
| `SECTOR (6, 0, 2)` + `This cell` | `chart.rs:1129`; route hint `:273-292` | R2 frame_000936 | Sector of the selection, route from your cell (`+X/-Z` when far) | `interface.md:95`; axes nowhere | KEEP |
| `CONTENTS`, `Entered 0:00:00, just now`, `FINDS`, `living`, `PRAGAR STATION LAST SEEN`, `Services rebuild, refit, trade, storage, jobs`, `Actuator pack x2` | `chart.rs:1138-1330` | R2 frame_000936 | Sector facts and last-dock stock | `interface.md:95` | KEEP. `0:00:00, just now` states time twice; not proposed (the title clock and the log use the same forms). |
| `VISITED CELL`, `Cell (x, y, z)`, station and wreck panels | `chart.rs:1835-1842,1905` | no frame | | | KEEP, not judged: no frame. The same `Cell` repeat as the job panel exists in source (`SECTOR` shows for every mark with a cell, `chart.rs:1100-1130`). |

## 4. Jobs

| Label | Source | Frame | Player must know | Explained | Verdict |
| --- | --- | --- | --- | --- | --- |
| Heading `Jobs posted here: Pragar station`, heads `Job Reward Time left` | `jobs.rs:1342-1353` (SHARED) | R2 frame_000924 | The board of the docked station | on screen | KEEP |
| Offer row: icon, title, credit figure, time bar + `20 min`, `Accept`, `x` | `jobs.rs:1465` (SHARED) | R2 frame_000924 | Take or delete the offer | `interface.md:273-277`; the `x` has a confirm `Delete offer?` (`jobs.rs:1375`) | KEEP |
| `Your jobs` + `Ore to market: Stations owned 0 / 1` | `jobs.rs:1529-1550` (SHARED); text from content | R2 frame_000924 | The role goal. It shows before any job is taken. | `interface.md:273`; nowhere on screen | KEEP. |
| `No jobs taken. Accept one at a station board.` | `jobs.rs:1609` (SHARED) | R2 frame_000924 | Next step | on screen | KEEP |
| Heads `Job Progress Hand-in` | `jobs.rs:1600-1602` (SHARED) | R2 frame_000930 | Columns of a held row | on screen | KEEP |
| Held row `0 / 5`, `[HAND IN HERE]`, `Hand in`, `x` | `jobs.rs:1657-1664` (SHARED) | R2 frame_000930, 933 | Whether you can hand in now | `interface.md:273` | KEEP. The badge is the only on-screen "docked at hand-in" cue; the button reason is hover text. |
| Footer `accepted: Bring 5 Iron ore` | `jobs.rs` notice line (SHARED) | R2 frame_000930 | Result of Accept | on screen | KEEP |
| Inspector `TASK`, `Target`, `Hand in`, `Reward`, `Progress`, orange `the hold carries 0 of 5 Iron ore`, `Show on Map` | `jobs.rs:1756-1799` (SHARED) | R2 frame_000933 | Details of one job and why Hand in is off | `interface.md:277` | KEEP. `Target` and `Hand in` name the same station for a procurement, but they differ for other kinds. The orange line is the explicit reason the task wants. |
| Empty inspector `Select a job to see its details.` | `jobs.rs:1733` (SHARED) | R2 frame_000924 | | on screen | KEEP |

## 5. Station

| Label | Source | Frame | Player must know | Explained | Verdict |
| --- | --- | --- | --- | --- | --- |
| `This station`, `Pragar station`, price with red `(missing ... )`, `Buy station` | `station_buy.rs:197,326`; `station/mod.rs:261-279` (SHARED) | R2 frame_000942 | The station is for sale and you are short | `interface.md:231`; the refusal is a tooltip | KEEP |
| `Ships for sale`, card name, `100 m`, price, `Buy`, `Info` | `shipyard.rs:270`; `hull_info.rs:45` (SHARED) | R2 frame_000942 | A hull you can buy | `interface.md:231` | KEEP. `100 m` is the hull length; no label says so. Not proposed. |
| `Escorts for hire` + `FLEET` + `STBD --` `PORT --` | `escorts.rs:214,237` | R2 frame_000942 | Two fleet sides, both empty | `interface.md:231` (no mention of `FLEET`, `STBD`, `PORT`) | KEEP. Only source of fleet occupancy. |
| Station tab missing when undocked; no pointer to Inventory | `pane.rs:226-250` (SHARED) | no frame | | | not a label; see audit 5a |

## 6. Inventory

| Label | Source | Frame | Player must know | Explained | Verdict |
| --- | --- | --- | --- | --- | --- |
| Filters `All Food Ammo Repair Raw Parts` | `inventory/app.rs:343-347` (SHARED) | R2 frame_000946 | Category filter | on screen | KEEP |
| Column heads `Item Qty Price Deal` | `inventory/app.rs:624-653` (SHARED) | R2 frame_000946 | | `interface.md:385` | KEEP |
| Row `Hull plat x15  31  [+3%]`; item names clip (`Hull plat`, `Actuator p`) | `inventory/app.rs:320-335` (SHARED) | R2 frame_000946 | Price per unit and the percent versus reference | `interface.md:385` | KEEP. The clip is a column-width and ellipsis issue, not a label text issue; `nova_ui/src/screen/ellipsis.rs` is another lane's new file. |
| PDC row price `11 / 100` | `inventory/app.rs:320-335` (SHARED) | R2 frame_000946 | Price of a lot of 100 | `interface.md:385` (as `10 cr / 100`); nowhere on screen | KEEP. A rename to `11 per 100` is possible, but the price column is narrow; not shortlisted. |
| Column facts `Cargo 200 kg / 2100 kg`, `Credits 10,240`, `Stored 0 kg / 1023.981 kg` | `inventory/app.rs:549-551`; `kg_text` in `nova_gameplay/src/inventory.rs:281` | R2 frame_000946 | Hold, funds, storage | self-evident | KEEP. `1023.981 kg` is exact grams by design and tested (`inventory.rs:2183`). |
| MARKET strip `Buys [PDC ROUND +14%] ... Sells [...]` | `market.rs:190-195` (SHARED) | R2 frame_000946 | What the trader pays most for or sells cheapest | `interface.md:385` | KEEP. Six badges, but they name items the Deal column does not rank. |
| `Restock [full bar] Stocked` | `market.rs:223-252` (SHARED) | R2 frame_000946 | Trader at target stock | `interface.md:385` | KEEP. Only source of the fact. Null state; owner may want it hidden. |
| `Docked, double-click an item to move its stack, Shift for one.` + `[/] Select` | `inventory/app.rs:791` (SHARED) | R2 frame_000946 | How to trade | on screen | KEEP |
| Top bar: tabs `Map Ship Inventory Jobs Station`, credit figure `2,200`, clock `0:00:15`, `M Next`, `Close Tab` | `pane.rs:668,688,711` (SHARED) | R2 frame_000946 (all panes) | Panes, funds, play time, keys | `interface.md:37-39,385` | KEEP. The clock has no word; `interface.md:385` names it. |

## 7. Ship

No frame of the Ship pane exists in any set. Every label here is `no frame` and not judged.
Source labels (`ship/app.rs`, SHARED): `Control`, `Rebind`, `Salvage`, `Delete`, `Repair`, `All`, `Lost sections`, `Rebuild`, `Placing`. Verdict for all: KEEP, no frame.
Next step if the owner wants this screen judged: one bench frame of the Ship pane.

## 8. Ranked shortlist (6 changes)

All rank by value per risk. Every wiki line named must change with the code.
`D` = documented in the wiki.

1. RENAME the chart heading.
   - Old: `OBJECTIVES  Y0` (format `OBJECTIVES  Y{layer}`). New: `OBJECTIVES  LAYER 0` (format `OBJECTIVES  LAYER {layer}`).
   - File: `crates/nova_interface/src/map/chart.rs:1447` (clean file).
   - Cost and risk: one string. The tooltip already says "layer". Break: `map/tests.rs:3162,3168` assert `OBJECTIVES  Y2`/`Y3`. D: `interface.md:95` (`OBJECTIVES  Y6`, SHARED). Audit noted `job_board_sandbox.rs:1082` already waits for a bare `OBJECTIVES`.

2. REMOVE the `Cell (x, y, z)` line from the Job panel.
   - Old: `Cell (6, 0, 2)` (`format!("Cell {}", job_cell(job))` in the Range field). New: empty string.
   - File: `chart.rs:1855-1858`.
   - Fact kept: `SECTOR (6, 0, 2)` plus the route hint `This cell`, shown for every selected job (`chart.rs:1100-1130`, `chart_mark_cell` `:181`).
   - Cost and risk: tiny. Unverified: whether an empty Range row leaves a visible gap (no frame of it). Not D. The same repeat exists at `chart.rs:1838,1905` (`no frame`); leave until a frame shows it.

3. REMOVE the `JOB` kind line from the Job panel.
   - Old: `JOB` (Kind field, `MAP_CHART_JOB_COLOR`). New: empty string.
   - File: `chart.rs:1852-1854`.
   - Fact kept: the heading `JOB-1` and the legend `Job`.
   - Cost and risk: tiny. Break: `examples/playable/job_board_sandbox.rs:1045` waits for exact text `JOB` on the Map (the line at `:1095` is on the Jobs pane; I did not check what it matches). Not D.

4. REMOVE the open-job count from a station mark.
   - Old: `STN-1 (2)`. New: `STN-1`.
   - File: `chart.rs:722-725` (drop the `posting` arm; keep `ChartStationLive.posting` for `:1311`).
   - Fact kept: `<code> posts <n> jobs` in the SECTOR block (`chart.rs:1285-1311`) on selection, and the Jobs board when docked.
   - Cost and risk: the pilot loses an at-a-glance "where are jobs" cue on the overview. Evidence for the cut: frame 939 `(2)` against one offer in frame 930. The same `posting` feeds the SECTOR line, so check the cause before trusting either. Alternative if the owner keeps the cue: `STN-1 JOBS 2`. D: `interface.md:95` (`STN-1 (3)`). Break: any chart test on the `(k)` form (none found by grep).

5. MOVE the advancement chip to a hover.
   - Old: chip `ADV 1%` beside the stance badge. New: no chip; the same sentence becomes a `Tooltip` on the civilization name cell (`Advancement 1%: seeded generation potential, not per-ship technology or progression.`).
   - File: `crates/nova_interface/src/map/stance.rs:226-230` (text), `:481-490` (chip), tooltip text exists at `:484`; `Tooltip` is already used in the block at `:498-513`.
   - Cost and risk: small. It is the only on-screen source of the number, so MOVE, not REMOVE. Break: `map/tests.rs:1263-1268` reads the chip text `ADV 13%`. D: `interface.md:95` (`ADV 15%` chip).

6. REMOVE the always-on `PLN-n` label on the local Map.
   - Old: `PLN-3`, `PLN-4` drawn beside every planet. New: planets draw their icon only; the code shows when selected, as for `AST-n`.
   - File: `crates/nova_interface/src/map/scene.rs:583`: `selected || contact.kind != Terrain` becomes `selected || !matches!(contact.kind, Terrain | Planet)`. SHARED file (4-line diff by another lane, `git status` M).
   - Fact kept: the panel code and the legend `Planet` on selection (`scene.rs:1239`).
   - Cost and risk: small, but frame 216 only shows collisions at the start; ally labels collide too (`ALLY-7`/`ALLY-8`) and stay. D: `interface.md:82` (the `PLN-1` row; add "code shows only while selected" as for `AST-1` at `:83`). Coordinate with the other lane before the edit.

## 9. Not proposed, and why

- Dock chip `D DOCK` while docked (undock): a state word needs a redesign of the fixed verb slots (`keybind_dock.rs:90`, `hud.md:65`). Owner decision.
- Legend words for the red ring and `(k)` (audit 4): they add labels; this task kills labels.
- `(x, y, z)` tag, `+X/-Z` route words: wiki-only today. A wiki sentence fixes it; no label change.
- `Hand-in` column, `[HAND IN HERE]` badge, orange `the hold carries ...`: each is the only on-screen source or the explicit reason a button is off.
- `0:00:00, just now` in the SECTOR block: double time form, small gain, not shortlisted.
- `STBD`/`PORT`/`FLEET`, `100 m`: abbreviations with no on-screen key, but the only source of their fact.
- Station and Inventory missing a pointer to each other: a new line, not a removal. Audit 5a and 5d own it.
