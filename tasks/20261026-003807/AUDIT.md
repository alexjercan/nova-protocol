# Audit for #124 (first-session New Game pilot)

Source: working tree, dirty with other lanes work (not HEAD). Read-only. No cargo, game or tests run. Everything below is source-derived unless it says otherwise.

## 1. RCS legibility

Claim: A New Game pilot has RCS on from the first frame. The only RCS display is a pip row with no words, hidden while idle and full. The dock chip does not show an empty magazine. Wiki copy is stale for New Game.

Evidence:
- New Game player has every capability on: `crates/nova_scenario/src/actions/spawn.rs:333` (`capabilities: default()`), default is all true at `crates/nova_ship/src/flight/capability.rs:82-93`.
- Pip chip, `crates/nova_hud/src/flight_status.rs:538-571` (working tree; lane edits present, not HEAD). Hidden when `RcsIntent` is zero AND `budget.spent <= 0` AND `budget.applied <= 0` (`:553-557`). Otherwise shown with 10 pips; lit count = `ceil(fraction * 10)` (`:560`), spent pips drawn at alpha 0.2 (`:108`, `:525-532`). No text, no tooltip, no capability test.
- Magazine: `crates/nova_ship/src/flight/state.rs:171-268`. Capacity 300 m/s base, +5 m/s per section above 65 (`:218-230`). Refill only after `rcs_recovery_delay` with no nonzero command (`:257-267`). Refill is automatic; there is no station service (see also `crates/nova_world_base/src/rcs_reserve.rs:1-10` for the thaw catch-up).
- States and what the pilot sees:
  - Full and idle: no chip at all.
  - Held or pushing: chip, pips light by tenths.
  - Short of full, idle (refilling): chip stays, pips shrink back as it fills. This also covers an EMPTY idle magazine: `spent > 0` keeps the chip up with ten dim pips (`:554`). TASK.md D4 says "Empty shows in the pips once held"; source says it shows whenever `spent > 0`, held or not. Minor correction, source-derived.
  - Empty: ten dim pips. No word says empty. The Shift push just does nothing (`state.rs:176-179` doc: held command on empty keeps it empty).
  - Capability off: Shift press logs at debug only and returns (`crates/nova_ship/src/input/player/flight_rig.rs:812-815`). Pilot sees nothing. The keybind dock RCS chip draws dark: `hints.rs:324-328` (`helm_gate(capabilities.rcs_enabled)`).
  - Keybind dock RCS chip does NOT look at the magazine; only capability and helm gate (`hints.rs:324-328`). So an empty magazine leaves the RCS chip lit.
  - MATCH (Shift+X) is the only place a reason exists: `flight_rig.rs:520-523` writes `MatchRefusal::RcsDisabled` (`:522`), and `:530` writes `RcsEmpty` when `budget.is_empty()`. The dock STOP/MATCH chip goes dark by the same rule (`hints.rs:239-248`). Reason text is a HUD notice (`MATCH: NO RCS`, `MATCH: RCS EMPTY`; see section 3). Plain Shift translation has no reason.
- Wiki:
  - `web/src/wiki/hud.md:62` describes the pips, the 300 m/s base and when the row shows. Accurate to code. Does not say empty looks like ten dim pips.
  - `web/src/wiki/flight-autopilot.md:90,95` give the magazine, 2 s delay, 100 m/s refill, empty behaviour. Accurate to the doc comments; numbers not re-checked against `FlightSettings` (unverified).
  - STALE for New Game: `flight-autopilot.md:99` ("a scenario may withhold it ... chip stays dark until a scenario grants it"), `keybinds.md:148` ("appears only when a scenario grants it"), `keybinds.md:98` ("when a scenario grants it"), `glossary.md:22` ("a trim a scenario grants rather than standard flight"), `flight-autopilot.md:3`. In New Game the hull has RCS from the start. Only Basic Training withholds it (`crates/nova_authoring/src/base_content/scenarios/tutorial/range.rs:162`).
  - Inconsistent: `glossary.md:92` says a chip "appears only when the ship has the verb"; `hud.md:65` says each verb has a FIXED slot and draws dark.
  - `web/src/wiki/getting-started.md:117` is the Basic Training beat. It says pips show RCS left and refill on release. True there. It does not say the row is hidden when full.
- Does any text give a reason for RCS being unavailable? Only the MATCH notices. None for plain Shift, none for empty magazine, none for withheld capability.
- TASK.md D4 notes, checked:
  - "RCS is a capability, not a section. On by default (`capability.rs:89`)": verified.
  - "off for parked hulls and scripted drills": parked: `spaceship.rs:754-764`, used at `crates/nova_scenario/src/objects/parked_hulls.rs:264`. Drills: `crates/nova_authoring/src/base_content/scenarios/drills.rs:228`. Tutorial range: `tutorial/range.rs:162`. Verified. Stations get `rcs_enabled: living` (`crates/nova_scenario/src/objects/station.rs:531`); not a pilot concern.
  - "no base hull lacks it": hull-level no switch exists; capability is a root component, not hull data. Verified for New Game by `spawn.rs:333`.
  - "Capability restore on Board is unverified": now resolved by source. A Board does not copy the parked record's capabilities. `rehull_ship` destructures `FrozenShip { design, mut state, .. }` and drops `capabilities` (`crates/nova_scenario/src/objects/spaceship.rs:788-790`); its doc says "The root keeps its identity, side, capabilities and credits" (`:768`). The station swap calls it at `crates/nova_scenario/src/objects/station_hulls.rs:180`, and the apply system is `crates/nova_interface/src/station/shipyard.rs:372-461`. The player root (default caps, all on) therefore keeps RCS after Board or Buy. The all-off record applies only to the parked copy (`spaceship.rs:754`). Not run live; UNVERIFIED at runtime. Rehull also resets the magazine to full for the new hull (`spaceship.rs:769-771`).
  - "Disabled RCS is silent on Shift (`flight_rig.rs:812`)": verified, debug log only (`:813`).

Recommendation (text only, owner decision):
- Fix the stale "scenario grants it" lines in the wiki so they say Basic Training withholds it and New Game does not. No code change.
- Optional, already named in D4: one word `RCS: OFF` on a hull with the capability withheld. Not needed for New Game because no New Game hull lacks it.

## 2. Ammo legibility

Claim: A pilot with empty ammo sees only an unlabelled gauge, and only in combat mode. No text names "out of ammo" or "hold has none". The reload rule is explained in text only on the wiki and in two item descriptions.

Evidence:
- Lane edits: `crates/nova_hud/src/ammo_readout.rs` and `situation.rs` carry uncommitted diffs (about 189 added lines in `ammo_readout.rs`). Everything below is the working tree. At HEAD the gate also opened for low ammo and active reload; the lane removes that.
- Gauge gate, working tree: `HudSituations::ammo_relevant` = `weapons_hot || combat_lock` only (`crates/nova_hud/src/situation.rs:66-68`, diff against HEAD confirms the change). `sync_ammo_gate` applies it (`ammo_readout.rs:238-245`). In normal flight, including with a travel lock, no ammo gauge is visible at all. A pilot who has not raised weapons never sees an empty magazine.
- Gauge look: turret ring of 8 pips (`ammo_readout.rs:11-16`, `:251-258`); torpedo bay one pip per round; railgun one blue pip. No number in a release build; the `rounds/capacity` text exists only under the `debug` feature (module doc `:38-43`). The only text on the gauge is the `xN` mount-count badge for clustered mounts (`:690`).
- Low and empty: a group at or under 25 percent (`LOW_AMMO_FRACTION`, `:716`), zero included, breathes amber (`:746-751`). The doc comment says why: "an empty gauge with a dark track is easy to mistake for 'no weapon', and the amber tells you it is a weapon out of ammo". Only visible while the gate is open.
- Reload previews: an incoming batch pulses above the live pips while the weapon rests and the hold can supply it; with no matching item the gauge shows no reload (wiki `combat-weapons.md:164`; code `ammo.rs` doc `:12-21`).
- Empty trigger pull: `SectionAmmo::try_consume` suppresses the shot (`crates/nova_ship/src/sections/ammo.rs:178-181` region). I found no notice, text or message in `hud_notice.rs` for an empty weapon (grep: only the MATCH reasons). Sound or visual cue on a dry pull: not checked, UNVERIFIED.
- Railgun: `crates/nova_hud/src/bore_sight.rs:397` dims the sight line to 30 percent alpha while the magazine is empty. Nothing else.
- How reload spends hold ammo (`ammo.rs:12-21`): each uninterrupted delay moves one batch of the weapon's item from the ship `ShipInventory` into the magazine, never more than the magazine misses or the hold carries. No matching item means no reload progress. Every shot resets the timer. Installed magazines start full without drawing on the hold. No manual reload exists. A station's PDC draws a private reserve, not the hold (`:15-20`).
- Inventory pane: Ammo is a filter and a category colour (`crates/nova_interface/src/inventory/app.rs:345,356`; filter list at `:90`). The inspector shows category, description, count, unit weight (`app.rs:768-`). Item descriptions that say where ammo goes: PDC round "Point-defense round. Kinetic and Pierce mounts reload from it." and Torpedo "Torpedo. Every torpedo bay reloads from it." (`crates/nova_authoring/src/base_content/items.rs:61-62,81-82`). A pilot who clicks the item reads the reload rule. The rows themselves carry no reload note.
- Ship pane: no per-section rounds or capacity anywhere. `grep SectionAmmo` over `crates/nova_interface` finds nothing. `crates/nova_interface/src/ship/scene.rs:134` has a comment "HP and ammo live in the section panel", which is stale: the panel shows no ammo (source grep only; the panel was not read line by line, so the claim "no ammo field" is a grep result, not a full read).
- Station pane: no ammo service and no ammo text (`grep -i ammo crates/nova_interface/src/station/*.rs` found nothing). Ammo is bought or sold on the Inventory market, not Station (see section 5).
- Start hull stock: a PDC-armed start role carries 1,000 PDC rounds (`crates/nova_authoring/src/base_content/start_roles.rs:6-7,60,96,116,135`). So a fresh New Game pilot has hold ammo and will not hit empty soon. The empty case needs sustained fire or a sold hold.
- Wiki, consistent with code:
  - `web/src/wiki/combat-weapons.md:156-164` Magazines (hold draw, no reload with none in stock, station reserve, gauge shows while hot).
  - `web/src/wiki/hud.md:37-38` (gauges only hot; low and reload pulse only while shown).
  - `web/src/wiki/interface.md:175` (no manual reload; empty stays empty until restocked).
  - `web/src/wiki/getting-started.md:175` (reload spends ammunition; Tab to Inventory).
  - Nothing in the wiki says what the pilot sees when BOTH magazine and hold are empty beyond "stays empty".
- Wiki working tree diffs: `combat-weapons.md`, `hud.md`, `interface.md`, `getting-started.md` are modified; read as written, not HEAD.

Recommendation (text only):
- One line in `combat-weapons.md#magazines` or `hud.md`: an empty weapon with an empty hold shows a dark amber gauge only while weapons are raised, and nothing in normal flight.
- Fix the stale `ship/scene.rs:134` comment (code comment, not player text).
- Whether to surface "OUT OF AMMO" or "HOLD EMPTY" on screen is an owner product decision, not a text fix.

## 3. Port readiness and docking constraints

Claim: A DOCK press never explains itself. The DOCK chip is lit or dark and nothing else. Only two station refusals post a sentence, and both are about the station, never about the geometry or the pilot's own hull. Docking sight and wiki cover the geometry.

Evidence (working tree; `hints.rs` has lane edits, +370 lines vs HEAD, none of them add a no-port line):
- Free port rule: a port is a candidate only if it is a live docking section with no `DockedPort` and no `SectionInactiveMarker` (`crates/nova_ship/src/sections/docking_section/port.rs:126-143`). `graded_pairs` returns nothing when either ship already holds a connection or both ships are the same (`port.rs:277-283`). `best_candidate` keeps only pairs that pass every gate (`port.rs:248-253`). Gates: face gap 10 m, facing 15 degrees, relative speed 5 m/s, relative spin 5 deg/s, roll ignored, graded on the stricter port (wiki table `web/src/wiki/sections/docking.md:32-41`; numbers from authored content, not re-read from code here).
- Target admission: `station_admission` (`crates/nova_ship/src/sections/docking_section/admission.rs:57-72`) returns `StationOccupied` when every live port of the station is held, even for a station you own; then admits owned or dead stations; `EnemyStation` on the Enemy side; `HostileStation` when hostile. Ships: `Combat` when hostile, Enemy, answering fire, or AI fighting; a neutralized ship always admits (`admission.rs:147-188`).
- DOCK chip: lit when docked (it is the undock key) or when `dock_enabled` and the lock is admitted and `best_candidate` exists (`hints.rs:311-322`). Chip state is Hot when docked, else Dim when unavailable, else Available (`crates/nova_hud/src/keybind_dock.rs:500-523`). Three states, no text.
- The press: `on_dock_input` (`crates/nova_ship/src/input/player/flight_rig.rs:739-769`) returns silently with no travel lock or with `dock_enabled` off (debug log only). It only asks. The request refuses with a debug log, not a player message: `crates/nova_ship/src/sections/docking_section/connection.rs:352` (target refuses now), `:356` (no eligible port pair), `:375` (two stations never dock). The docking.md HTML comment cites `connection.rs:193` for this; the lines are 352/356 now (invisible citation drift, not player text).
- Explicit reasons that exist:
  - `<NAME> REFUSES DOCKING` for a living Enemy or Hostile station (`hints.rs:119-129`).
  - `<NAME> DOCK OCCUPIED` for a station with every live port held (`hints.rs:126`).
  - Both come from `FlightVerbHints.dock_refusal` (`hints.rs:255-266`) and post once on the edge through `post_dock_refusal_notice` (`crates/nova_hud/src/hud_notice.rs:165-177`) into the one-slot notice line, 4 s of game time (wiki `web/src/wiki/hud.md:66`).
  - MATCH reasons: `MATCH: NO TARGET`, `NO RCS`, `RCS EMPTY`, `TARGET LOST`, `NO FLIGHT COMPUTER` (`hud_notice.rs:188-198`).
  - Mode chip words for a docked pair: `NEUTRAL`, `HELM FAULT`, `STATION HELM` (`crates/nova_hud/src/flight_status.rs:1-9,635-645`).
  - `JOB-n done: hand in at STN-m` (`hud_notice.rs:200-222`).
- Dock refusals with NO reason (dark chip or silent press only):
  - Ship target in `Combat` state: `dock_refusal_line` returns `None` on purpose (`hints.rs:120-122`, doc `:115-119` "a ship under fire is a situation the player can read off the world").
  - No travel lock: silent (`flight_rig.rs:761-764`). The wiki says the dock is offered against the travel lock (`docking.md:46`).
  - Target is a station or ship but no free pair passes the envelope (gap, facing, speed, spin): dark chip. The docking sight draws only inside 80 m (wiki `docking.md:49`) and colours plates and line green per gate (`crates/nova_hud/src/docking_sight.rs:1-50`, colour logic by wiki). Beyond 80 m, nothing says why.
  - The player's own hull has no live free docking port (destroyed or inactive section), or the station's free ports are all geometrically ineligible: dark chip, no text. This is TASK.md D3's gap. D3 proposes `NO DOCK PORT` when the travel lock is a dock target and the hull has no free live port; not in the tree.
  - `dock_enabled` withdrawn (parked hull, drills): dark chip, silent press (`flight_rig.rs:757-760`).
  - Docked pair, partner hull not driving: DOCK stays lit (undock) but RCS, STOP and so on go dark; the mode chip says `NEUTRAL` (`hints.rs:201` `docked_without_helm`).
  - Press while the game is paused or player control is suspended: silent by design (`flight_rig.rs:747-749`).
- Wiki:
  - `docking.md:46-60` Flying the approach: travel lock, chip is the readout, sight inside 80 m, gap, facing, speed and spin. `docking.md:62-68` Who lets you dock: refusal rules and the `DOCK OCCUPIED` notice.
  - Roll claim: `docking.md:32,41,53` say roll is ignored. Matches the sight module doc (`docking_sight.rs:19-25`: roll is not drawn). TASK.md says the false roll-required text is gone from HEAD; the working tree agrees (no diff in `web/src/wiki/sections/docking.md`).
  - `docking.md:62-64` does NOT mention that `<NAME> REFUSES DOCKING` posts for an Enemy/Hostile station; `hud.md:66` does. A ship in combat gives a dark chip and no line; docking.md says "the DOCK chip also goes dark" without noting it is silent for ships.
  - Not explained anywhere I found on screen or in docking.md: the 80 m figure appears on the page; the 10 m capture and 5 m/s figures appear in a table. A pilot with a dark DOCK chip at 300 m and a lock sees no hint that they are too far. UNVERIFIED whether the keybind dock tooltip says anything; I found no tooltip in `keybind_dock.rs`.
  - Home-station start: New Game starts docked (`open_world.rs:6-8`). The first undock is `D`. Whether any text tells a New Game pilot that `D` undocks: the objective cue names only the interface key (TASK.md D1). `getting-started.md` has no word `undock` (grep). The only on-screen sign is the DOCK chip drawn Hot (inverted) while docked (`keybind_dock.rs:500-523`, `situations.docked`), and the wiki says so (`docking.md:22,105`: `D` again lets go; `hud.md:65` describes the inverted chips). Docked, nothing in the New Game flow links the inverted DOCK chip to leaving.
- Station-dock path inside the interface: the Station and Jobs panes read the docked state; their refusal texts are in section 5.

Recommendation (text only, owner decision):
- Add one sentence to `docking.md#who-lets-you-dock` listing which refusals post a line and which only darken the chip (Combat ship, no port, geometry, no lock).
- D3 (`NO DOCK PORT`) is a code change plus a new string; it is an owner decision, as TASK.md already says. Not recommended here beyond restating it.

## 4. Chart labels

Claim: The chart uses short codes and words on marks and in the side panel. The wiki explains most of them. On screen, the legend names only icons. The `(n)` job count, the red ring, the sector coordinate and the route words have no on-screen key. There is one tooltip on the chart pane. Not audited before; this is the first list. `crates/nova_interface/src/map/scene.rs` has a one-line colour diff from another lane (`:1218`), not label text.

Evidence, label strings in `crates/nova_interface/src/map/chart.rs` unless noted:
- Mark labels (projected over the viewport, 12 px unless noted):
  - `STN-n`: a found station, n = 1-based place in the chart's found order (`crates/nova_gameplay/src/chart.rs:225-230`). With open jobs the label reads `STN-n (k)` where k is the count of jobs the board posts that you neither hold nor completed (`chart.rs:20-21` doc, `:720-725`, `ChartStationLive.posting` `:104-113`).
  - `WRK-n`: a found derelict hull, n in found order (`:169`).
  - `JOB-n`: a held job, n in accept order (`crates/nova_gameplay/src/jobs.rs:1141-1146`). The number is the position in the held list, so the code can change when an earlier job leaves. Source suggests this; not run.
  - `YOU`: own ship (`:744-745`, 11 px).
  - Cell tag `(x, y, z)`: the sector coordinate of a visited cell, 10 px, at the tile corner (`:606-615`; `SectorCoord` Display `crates/nova_events/src/generated.rs:105-108`). No unit or axis note on screen.
- Marks without text: a dead station fades to 30 percent alpha (`:5-9` doc, const `MAP_CHART_DEAD_STATION_ALPHA`, `chart.rs:208`); a station that refuses you a dock gets a ring under it ("a ring under one hostile enough to refuse the player a dock, whatever its ports hold", `:9-11`). Own-ship cell has a stronger fill (`chart.rs:201`).
- Cluster words (cell rows, panel, legend), `chart_cluster_word` `:210-218`: `ASTEROIDS`, `ROCKS`, `PLANETS`, `DERELICTS`, joined by ` + `, or `EMPTY` (`:220-229`). Legend words are the title-case set `Asteroids`, `Rocks`, `Planet`, `Derelicts` (`crates/nova_interface/src/icons.rs:477-486`). Plural `PLANETS` vs legend `Planet`: small mismatch, source only.
- Station kind line: `DEAD STATION`, `OWNED STATION`, `LIVING STATION` (`:241-249`).
- Sector block (`:1129-1330`): `SECTOR (x, y, z)`, `CONTENTS`, `Empty space.`, `Not entered yet: contents unknown.`, `Entered ...`, `FINDS`, facts `dead`, `yours`, `refuses docking`, `living`, `found <time>`, `<NAME> LAST SEEN`, `Dead. Nothing trades here.`, `Services` (lowercased service enum names), `No stock seen.`, `STORED`, `JOBS HERE` with `done` or `to do`, `<code> posts <n> jobs` + `open`, stock rows `xN`.
- Cell panel: `VISITED CELL`, `Cell (x, y, z)`, `Entered <time>`; route hint `2 cells +X, 1 cell -Z` or `This cell` (`:273-292`). The +X/-Y/+Z axes have no on-screen key. Whether the Map pane draws axis arrows: UNVERIFIED.
- Job panel: `JOB`, `Cell (...)`, `Reward ... cr`, `Posted by <station>. <state> Hand in at <STN> while docked.` (`:1853-1880`).
- OBJECTIVES block (`:1425-1520`): heading `OBJECTIVES  Y<layer>`, rows `JOB-n title have / need` or `done`, hand-in station code, role heading and next milestone, `Goal reached at <time>`, `EMPTY` (`:1488`). The only tooltip in the chart: the heading, "N visited cells on layer L" (`:1438-1444`). The wiki says so (`interface.md:95`). The `Y6` means the Y layer number; nothing on screen says "layer" except this tooltip.
- Discovery log: `DISCOVERY LOG`, `Nothing found yet.`, rows with a code or cluster words, a cell and `just now`, `N min ago`, `N h ago` (`:1647-1670`, `:263-267`).
- Stance block labels (`crates/nova_interface/src/map/stance.rs`): `STANCES`, `ALLY`/`NEUTRAL`/`ENEMY` (`:91-93`), `HOSTILE <time>`, `CLEAR <time>`, `SEEDED`, `FINE <cr> @ STN-n`, `FINE NONE`, `OFFENCES a/b`, `MERITS a/b` (`:159-194`), `ADV 15%` with a tooltip (`:484`), per-row hover hints (`:498-513`), button line `DOCK STN-n` or `STN-n REFUSED` (`:305-307`). These are the densest codes. They carry tooltips.
- Local map labels (`crates/nova_interface/src/map/contacts.rs:91-105`): `SELF`, `ALLY-n`, `HOST-n`, `OBJ-n`, `AST-n`, `NEU-n`, `PLN-n`; a station reads its `STN-n`. Panel notes (`:79-89`): `That is you.`, `Friendly contact.`, `Hostile contact.`, `Mission objective.`, `Asteroid mass.`, `Ship on no side.`, `Planetary body.`. Local legend words: `Own ship`, `Ally ship`, `Hostile ship`, `Objective ship`, `Neutral ship`, `Ally station`, ... `Asteroid`, `Planet`, `Objective` (`scene.rs:1341-1362`). Footer line: `Click a contact to select it.` (`scene.rs:1218`). Local-view action words: `GOTO`, `GOTO SET: <name>`, `GOTO CANCELLED`, `GOTO <code>  <range>`, `Range <m>`, and `NO CHART: NO OPEN WORLD` (`scene.rs:426,473,995,1180,1245`; `chart.rs:334`).
- Chart legend on screen (`chart.rs:1930-1938`, `icons.rs:454-463`): eight icon plus word entries only: Asteroids, Rocks, Planet, Derelicts, Station, Wreck, Job, You. Nothing for `STN-n`, `(k)`, the red ring, a dead-station fade, tile layers, or the `(x, y, z)` tags.
- Wiki coverage, `web/src/wiki/interface.md`:
  - Local codes table `:75-83` (SELF, ALLY-1, HOST-1, OBJ-1, NEU-1, PLN-1, AST-1; "AST code shows only while selected" is wiki text; code check of that rule not done, UNVERIFIED).
  - Chart paragraph `:95` defines `STN-1`, `WRK-1`, `STN-1 (3)`, the red ring, `living/dead/yours/refuses docking`, `Not docked: no stock seen.`, `OWNED STATION`, `JOB-1`, `OBJECTIVES  Y6`, the tooltip, `EMPTY`, `Goal reached at`, the discovery log times, and the stance badges. It does not explain the `(x, y, z)` tag as sector cell coordinates, nor `+X/-Z` in the route line, nor that JOB numbers shift when a job leaves (unverified in play).
  - `hud.md:66` and `interface.md:279` use `JOB-1` and `STN-1` in notices and chips.
  - Glossary: not checked for `STN`.
- On-screen explanation summary: the codes `STN-n`, `WRK-n`, `JOB-n` are explained only in the wiki. On screen a pilot can click a mark and read `LIVING STATION`, `Posted by ...`, `Hand in at STN-n while docked.` in the side panel, which does name the code's meaning by example.
- TASK.md "dense labels die" has no named labels. This list is the input for that decision.

Recommendation (text only, owner decision):
- Legend: add short words for the red ring (`refuses docking`) and for `(k)` (`open jobs`). Two legend entries; owner choice because it touches the shared legend widget.
- Wiki: add one sentence that `(x, y, z)` is a sector cell and what `+X`, `-Z` mean, and that JOB numbers follow the held list.
- Not text-only: removing or merging codes, or adding tooltips to marks, is a product decision.

## 5. Station, Inventory market, wiki parity, pad parity

Scope note: `station/mod.rs`, `station/station_buy.rs`, `inventory/*`, `jobs.rs` (856 lines changed), `pane.rs`, `objective_stack.rs`, `mission.rs` and the wiki pages `interface.md`, `getting-started.md`, `hud.md` all carry other lanes' uncommitted edits. The wiki below matches the working tree, not HEAD. The tree also moved while I read: `open_world.rs` gained an `ObjectiveChipHold` action between two of my reads.

### 5a. What the Station pane sells and refuses

Claim: The Station pane sells only things a pilot buys with credits: the station, ships, escorts. Every refused button keeps a greyed button whose reason is a hover tooltip, and every result or refusal also posts to the pane notice line for 4 s. The pane never mentions ammo, plates, trade goods or repair, and the pane shows no pointer to Inventory for goods.

Evidence:
- Pane purpose and blocks: `crates/nova_interface/src/station/mod.rs:1-35`: This station, Your ships, Ships for sale, Escorts for hire. "A refused action keeps its button, greyed, with the reason as its tooltip" (`:19-21`). Notice line, 4 s, green or red, with click or deny cue (`:23-29`). Refit stays in Ship pane; wreck strip is in Inventory; job board is in Jobs (`:31-35`).
- Empty state: `This station sells no ships, hires no escorts and is not for sale.` (`mod.rs:187`). No pointer to Inventory or Jobs in that line.
- Greyed-with-reason buttons: `refusable_button` inserts `InteractionDisabled` and a `Tooltip` with the reason (`crates/nova_interface/src/pane.rs:1031-1043`).
- Reason strings:
  - Station buy (`station/station_buy.rs:162-176`): `buy: dock at a living station`, `buy: no civilization sells <name>`, `buy: <name> is yours already`, `buy: <name>'s civilization sells to its allies only`, `buy: <name> costs <x>, the ship holds <y>`. An `ALLIES ONLY` badge sits beside the name. Working tree adds a two-step Buy: the first press arms, `Confirm` plus `Cancel` appear over `Buy <station> for <price>?` (lane diff in `station_buy.rs`).
  - Hull buy and board (`station/shipyard.rs:111-123`): `buy: dock at a station that refits`, `buy: this dock offers no refit`, `buy: <name> costs <x>; you have <y>`, `buy: a balance after buying <name> does not fit`. Result strings: `bought <name> for <x>; your last ship is parked here.` and `boarded <name>; your last ship is parked here.` (`shipyard.rs:445-448`).
  - Escort hire (`station/escorts.rs:79-99`): `hire: dock at a station that refits`, `hire: this dock offers no refit`, `hire: this station's civilization is no ally of yours`, `hire: the fleet holds its <cap> escorts`, `hire: <name> costs <x>, the ship holds <y>`, overflow.
  - Settle (Map pane notice, `station/settle.rs:76-83`): `settle: <name> asks <x>, the ship holds <y>`, `settle: dock at a station a civilization keeps`.
  - Price shortfall shows in red as `(missing <x>)` beside the price (`mod.rs:261-279`).
- Station tab appears only while docked at a living station: `crates/nova_interface/src/pane.rs:226-250`, `StationDock::docked` (`station/mod.rs:116-139`). Undocked, the tab is hidden and the interface opens on Ship. No on-screen text says why the tab is gone. A derelict or dead station gives no tab either.
- Pad pilots cannot read a tooltip reason: the reason is hover text, and the interface has no pad pointer (see 5d). The notice line shows the reason only after the action is attempted, which a pad pilot cannot do either. Source-derived; not run with a pad.
- No fuel, ammo, repair or reload service anywhere in `station/` (grep for ammo found nothing). TASK.md says the same (`:15`).
- Wiki: `interface.md:227-256` describes the pane, buttons, refusals, `(missing <amount>)`, `ALLIES ONLY`, the two-step Buy (working tree wording; accurate to the lane's code, which is not yet committed). It states `Refit stays in the Ship pane`. It does not say that goods and ammo are bought in Inventory.

### 5b. Inventory market (where ammo and plates are actually bought)

Claim: Ammo, plates and ore trade in the Inventory pane against the docked ship or station, not in Station. The market strip explains price deltas and restock. The wiki has the full price table; the code agrees with it.

Evidence:
- Strip: `crates/nova_interface/src/inventory/market.rs:1-9`: what the docked trader pays most for, sells cheapest, restock gap with play time to fill, and a `REFUSED` badge while its civilization refuses the player. It draws only for a generated trader (`TraderMarket`); an authored trader deals at reference with no strip; a lootable or own partner trades nothing.
- Row prices and `Deal` badge: `inventory/app.rs:320-335` (`RowPrice`, lot of 100 for PDC rounds).
- Refusal strings: `Refused: enter a quantity`, `quantity is zero`, `sale pays 0 cr`, `only <n> <item> in <ship>`, `<ship> has room for <kg> more`, `<buyer> has only <cr>`, `<seller> cannot hold more credits`, `<partner> does not trade` (`app.rs:2785-2830`); storage: `not docked`, `<partner> offers no storage`, `storage full, <kg> free` (`app.rs:2965-2990`); loot: `no room for <n> <item>`, `holds no credits`, `not neutralized or lootable` (`app.rs:2520-2540,2645-2660`).
- Item facts: description text for each item (`crates/nova_authoring/src/base_content/items.rs:50-90`). Reference prices in code equal the wiki table: Hull plate 40/30, PDC round 10/7 per lot of 100, Rail slug 40/30, Torpedo 400/300, Stone 4/3, Iron 16/12, Water ice 12/9, Carbon 8/6 (`items.rs:44-130` first eight rows; Rations and Salvaged parts not read, UNVERIFIED).
- Category word `Ammo` and filter exist (`app.rs:345`). Nothing in Inventory says "your weapons reload from this".  Only the item description does, after a click.

### 5c. Wiki parity, Jobs / Station / Inventory

Checked against working-tree code. Matches:
- Jobs refusal strings quoted in `interface.md:272` exist: `accept: the hull has no weapon` (`crates/nova_interface/src/jobs.rs:432`), `accept: <kg> does not fit the <kg> hold` (`:427`), `dock at <station> to hand in` (`:459`), `the hold carries 2 of 4 Iron ore` (`:466`), `not done yet` (`:470`). Also present and not in the wiki list: `accept: the hull has no docking section`, `accept: dock at a station that posts jobs`, `this station did not post it`, `you took it already`, `you completed it already`, `this offer expired`, `its target is gone`, `hand in: it pays <x>, the station holds <y>`, overflow (`jobs.rs:431-477`). The wiki names some of these in prose (`target is already gone`); the full set is not listed. Not a falsehood.
- Pane strings: `Dock at a station to see its job board.` (`jobs.rs:1515`), `Delete offer?` (`:1375`), `Drop job?` (`:1624`), `HAND IN HERE` (`:1657`), `DONE` badge (`:1752`), Back and Drop and Delete buttons, `Show on Map` (`:1799`).
- Pane order: Map, Ship, Inventory, Jobs, Station while docked, then Map (`pane.rs:97-106`); matches `interface.md:39`.
- Station tab gate matches `interface.md:40`.
- Esc and Start: `close_surface_from_menu_keys` backs out one level, then closes the interface (`crates/nova_interface/src/terminal/input.rs:105-150`). Matches `interface.md:38`.
- Close button and Next hint: the lane adds `InterfaceCloseButton`, `InterfaceNextHint`, `refresh_card_head_keys` in `pane.rs` (diff). Matches `interface.md:37`, but this is uncommitted, so the wiki claims a feature that is not at HEAD.
- Price table in the Inventory market: matches items (see 5b).

Mismatches, stale text, or unverified:
1. `getting-started.md` New Game paragraph says the Take a job chip "stays up until your first accepted job clears it", and `interface.md:56` says the same. TASK.md D1 and D2 say the shipped behaviour is a 12 s dwell or first interface open (`objective_stack.rs:59,335-366`) and D2 is open. In the working tree the hold exists: `ObjectiveChipHold` (`crates/nova_scenario/src/actions/mod.rs:228`, `mission.rs`), `ObjectiveChipHolds` and `keep_held_objective_chips` (`crates/nova_hud/src/objective_stack.rs:205-250,410-450`), and `open_world.rs:41-51` sets it in OnStart and OnResume. So wiki, tree and TASK.md D2 disagree: TASK.md D2 is STALE against the tree. The tree is uncommitted lane work, so the wiki is true only if that lands. Not run.
2. D7 partly addressed in the tree: the chip footer reads `INTERFACE` while a held chip shows, else `OBJECTIVES` (`objective_stack.rs` diff: `TAB_LABEL_INTERFACE`). TASK.md D7 still says the footer word is `OBJECTIVES` only. Chart OBJECTIVES block still lists held jobs and role milestone only, not `Take a job` (chart.rs doc `:26-35`).
3. `flight-autopilot.md:99`, `keybinds.md:98,148`, `glossary.md:22`: "a scenario grants RCS" is stale for New Game (area 1).
4. `docking.md` HTML comment cites `connection.rs:193`; lines are `:352,356,375` now (invisible).
5. Wiki `interface.md:39` says M steps panes "while a text field has the caret, M types instead". Not checked in code. UNVERIFIED.
6. `interface.md:58-75` local map code table: "AST-1 code shows only while selected" not checked. UNVERIFIED.
7. Pad: no wiki line says a gamepad cannot press Accept, Show on Map or Station buttons (see 5d). The gamepad column of the Map key table lists only bound actions.

### 5d. Keyboard versus pad, Accept, Show on Map and Station actions (source only, no proposals)

Claim: Accept, Hand in, Drop, Delete, Show on Map, Buy station, Buy, Board, Hire, Settle and the Chart button are mouse-click buttons only. There is no keyboard key and no pad button for any of them. Keyboard and pad are equal here: both lack a route. TASK.md D5 is right. Runtime unverified.

Evidence:
- `crates/nova_interface/src/bindings.rs:46-160` lists every interface binding: `interface_toggle` (Tab, Right Thumb), `interface_next_tab` (M, North/Y), viewer orbit, pan, reframe, select next and previous (`]`, `[`, bumpers), `map_goto` (G, South/A), `map_chart` (C, keyboard only), `map_layer_down/up` (Z, X, keyboard only), `ship_mates`, `ship_repair`, `ship_rebind`, `ship_delete` (keyboard only), `ship_part_*`, `ship_pick`, `inventory_details` (I, keyboard only). No `jobs_*` or `station_*` binding exists.
- Buttons act on `On<Activate>` observers (`jobs.rs:1808,1826,1852,1885,1917`; Show on Map `jobs.rs:1799-1801`; Station buttons `station_buy.rs`, `shipyard.rs:339-355`, `escorts.rs`). `just_pressed` appears in the interface crate only for `viewer_next/prev` and `inventory_details` (`inventory/app.rs:1508-1545`); none in `jobs.rs` or `station/*`.
- The bindings.rs doc says the pad "runs out before the pane verbs do" and names the keyboard-only list (`bindings.rs:34-39`). It does not list Jobs or Station actions.
- No focus navigation: grep for `InputFocus`, `TabIndex`, `DirectionalNav` over `crates/nova_interface/src` finds only a comment in `pointer_rig.rs:171` (test-only rig). No virtual cursor found in `nova_input` (`device.rs:8` mentions a pinned cursor only for scripted runs).
- What a pad can do in the interface: open and close it (Right Thumb, Start), switch panes (Y), step the selection (bumpers), GOTO (A) on the Map, reset the view, orbit.
- Keyboard-only pane keys: C (chart), Z/X (layer), P, B, Delete, I. Accept, Show on Map and every Station action have no key even on keyboard; a mouse is needed. A keyboard-only player (no mouse) is also locked out. Not stated in the wiki.
- Keybinds wiki (`web/src/wiki/keybinds.md`) lists keyboard and gamepad columns for flight; Jobs and Station are not in it (grep `Accept` not run; UNVERIFIED).

Recommendation (text only, owner decision):
- Add one honest sentence to the wiki Jobs and Station sections: these buttons need a mouse pointer; no key or pad button reaches them. This is a text-only fix if the owner wants the limit stated; the fix itself (focus navigation or binds) is the keybind rework, not text.
- A New Game Station-pane hint "Goods, ammo and plates trade in Inventory; repairs spend plates in Ship" is a text-only line owner can choose. Whether it belongs on the Station empty state or only the wiki is an owner decision.

## Cross-cutting notes

- TASK.md is stale on D2 and D7 against the working tree (see 5c items 1-2). The held-chip action and the `INTERFACE` footer word exist uncommitted; `open_world.rs` sets the hold in OnStart and OnResume. The tree is another lane's active work and changed while I read.
- Nothing in this audit changes bindings, pad, controls, economy, docking, autofill or autodock. D3 (`NO DOCK PORT`) is restated as an owner decision only.
- Not verified at runtime anywhere: no game, test or cargo was run. Every claim is source or wiki text.
- Items I could not confirm: Rations and Salvaged parts prices; `AST-n` code visibility rule; the `M types in a text field` rule; whether a dry-fire gives sound or a visual cue; whether the keybind dock chips have tooltips (none found); `FlightSettings` RCS numbers against the wiki (300 m/s, 2 s, 100 m/s); `keybinds.md` Jobs or Station rows.

## Summary table

| Area | What a pilot can learn on screen | Gap | Fix type |
| --- | --- | --- | --- |
| 1 RCS | Ten violet pips under the speed chip while pushing, short of full, or empty-idle; dock RCS chip lit or dark; `MATCH: NO RCS` / `MATCH: RCS EMPTY` only on Shift+X | No word for empty, withheld or full; plain Shift gives no reason; wiki still says a scenario grants RCS (stale for New Game); Board keeps root capabilities (source-verified, not run) | Wiki text fix for the stale lines; `RCS: OFF` or an empty word is an owner product decision |
| 2 Ammo | Amber-breathing ring or pip row, only in combat mode or under a combat lock (lane edit removes low/reload wake-up); item description says which mounts reload from it | Empty magazine and empty hold show nothing in normal flight; no number in release; no text for a dry pull; Ship pane shows no rounds; stale `ship/scene.rs:134` comment | One wiki sentence is text-only; an on-screen OUT OF AMMO line is an owner decision |
| 3 Docking | Dark or lit DOCK chip, inverted when docked; docking sight inside 80 m; `<NAME> REFUSES DOCKING`, `<NAME> DOCK OCCUPIED`, `MATCH:` lines, `NEUTRAL`/`HELM FAULT`/`STATION HELM` | No reason for: ship in Combat, no travel lock, no free own port, geometry out of envelope beyond 80 m, `dock_enabled` off; `D` to undock not taught in New Game flow | Wiki sentence listing silent cases is text-only; `NO DOCK PORT` (D3) and any new notice need an owner decision |
| 4 Chart labels | Legend of 8 icons; side panel names station kind, job text and hand-in station; wiki defines `STN-n`, `WRK-n`, `JOB-n`, `(k)`, ring, `Y<n>` | No on-screen key for `(k)`, the red ring, `(x, y, z)`, `+X/-Z` route words or `Y6`; one tooltip only (OBJECTIVES heading); JOB number shifts as the held list changes | Two legend words plus one wiki sentence are text-only; pruning codes or adding mark tooltips is an owner decision |
| 5 Station and parity | Greyed buttons with hover reason, 4 s notice line; Jobs reasons; Inventory prices and `Deal`; item descriptions | Reasons are hover-only; Station tab silently vanishes undocked; Station pane never points to Inventory for goods; Accept, Show on Map and all Station actions have no key or pad route; wiki chip-hold text depends on uncommitted lane code | Wiki wording is text-only; any key or pad route for these buttons is the keybind rework (owner decision, not in #124) |
