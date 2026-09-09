# Write the v0.13.0 news post and capture its media

- STATUS: OPEN
- PRIORITY: 15
- TAGS: v0.13.0, docs, web, capture, release

## Goal

Write `web/src/news/0.13.0.md` to the standard set by `0.12.0.md` and
`0.11.0.md`: a narrative lead, `##` chapters, live `data-widget` explainers,
and a figure for every claim worth seeing - **every figure resolving to a real
captured asset**.

Owner's ask (2026-09-09): design the news page in the v0.12.0 / v0.11.0 style,
with graphical widgets that show how things work and before/after loops where
the change is a change. Shots of a shipped version may come from
`~/personal/content-machine`, which pins a capsule per Nova tag and can shoot
v0.12.0 and earlier. Those before-shots are not tracked in this repo.

Owner's cut: the lore encyclopedia and the Story archive ship in the tag but
are NOT featured in the post. At most one line in "More in v0.13.0".

## The release

164 `[Unreleased]` entries against the v0.12.0 baseline, 11 of them breaking -
the biggest cycle so far (v0.12.0 shipped 130). The heaviest groups are Modding
(35), Internals & Tooling (35) and Interface & HUD (19).

The epic (`20260831-145934`) calls it "v0.13.0 puts the game on screen": every
section a real machine, a new spinal weapon, the first audio pass, a console
that reaches the world by name, and ships the game builds itself.

## The chapter list

Ranked. `[HERO]` chapters carry the post; `[FOLD]` merges into its neighbour if
the post runs long.

### 0. Lead

Three paragraphs plus the release callout and one lead loop. The frame: v0.12.0
made the editor the star; v0.13.0 is about what the editor now has to place -
sections that are machines, hulls the game draws itself, rock that is a
material, and a soundtrack for all of it.

### 1. The railgun `[HERO]`

The release's flagship weapon. Subsections:

- **Commit and release.** No traverse, so the ship aims it. Tapping the trigger
  commits; the bolt walks the bore; the shot leaves whether or not the nose is
  still on target. Twelve-second idle reload, one shell.
- **What a slug takes.** A three-cell corridor through everything in line.
  Power, not a layer count or the bore, bounds the take. Siege lance at capital
  grade: 500 damage, 360,000 power, a 30 m rake.
- **The gun shoves the ship.** Impulse at the muzzle, so an off-axis mount yaws
  as well as pushes.
- **The bore and the wake.** Charge glow climbing the rails, sparks off the
  brake, camera kick; the ionized wake - cyan haze, violet filaments, a blue
  light riding the slug - gone in half a second, and not drawn on Low.
- **Reading it.** Pierce-blue ammo gauge, and the line of fire ringing every
  section the shot would destroy, dimmed through the reload.
- **Cover breaks a lock.** A lock is a radio link and rock stops radio: a
  contact behind an asteroid or a planetoid cannot be locked, and cover drifting
  across a held lock breaks it. Hostiles lose you the same way.

### 2. Every section is a machine `[HERO]`

The clearest before/after in the release: placeholder cubes became authored
models, and three of them move.

- Every section at the thruster's standard: hull cells, controller core, PDC
  mounts, torpedo bay.
- The PDC stows. Out of combat it sinks into a housed pit under two lids;
  weapons hot, a tracked target or a PD assignment deploys it, and it cannot
  track or fire until fully up.
- The bay's muzzle iris winds open before ejection; held fire keeps the doors
  gaping. **(breaking)** The bay is now a 1x1x2 tube with an unlinkable muzzle
  face and a half-cell seat.
- A twin PDC mount in kinetic and pierce: two muzzles at half rate each, so the
  choice costs the same ammo.
- Hull in three same-stat builds - personnel, cargo, tank - so a flank can say
  what the ship carries.
- Cladding shot off a hull flies: the plate comes away where it stood, takes
  its greebles and tumbles on the ship's own motion.

### 3. The game builds its own ships `[HERO]`

- **(breaking)** The base game ships BLOCK hulls only. The modelled Racer,
  CargoA and CargoB fleet and its prototypes moved into The Ledger, which now
  carries their meshes. Eleven block ships join the catalog - cutter, hauler,
  gunship, raider, industrial carrier, stolen warship, five cleanup craft -
  plus four wreck fragments.
- A grammar says WHERE a part stands: `zone` pins a prototype to the bow,
  amidships, stern, dorsal, ventral or flank third, for every cell of it.
  `bow_gun` seats a spinal gun on the keel and the mirror makes a pair.
- The collapse places multi-cell parts whole - a torpedo bay, a 5x5x3 capital
  drive - instead of skipping anything wider than a cell.
- In the editor: Generate a HULL into the ship you are inside. Name a seed,
  tick the sections, press a row's zone chip to cycle where it may stand. The
  biggest ticked drive is seeded as the main engine and the biggest railgun as
  the bow gun; the grid grows to hold both, and a HULL PLAN line names what the
  ticks chose. The Generate block picks a HULL LINE - one row per grammar in
  the merged content.
- A weapon takes its own key when placed or generated, so a generated ship set
  to Player flies without rebinding.
- A grammar the collapse cannot run is refused by id at lint AND at load.

### 4. The world has bodies `[HERO]`

- **(breaking)** A scenario places a `Planet`: a type and a seed draw a world
  with terrain, biomes and a cap. A planetoid authored as a big `Asteroid` is
  now a rock - re-author it at its true radius. A `Planet` must author
  `invulnerable: true` or it refuses to load.
- **(breaking)** An asteroid's `material` names its KIND and is required: rock,
  metal, ice, carbon or plain. Each gets its own palette, strata and specular
  over a triplanar surface, so a belt reads as several materials.
- `ScatterObjects` takes `asteroid_kinds`, a weighted mix drawn from the
  scatter's own seed.
- Shooting a rock throws rock. The body states its material while the crater is
  cut into the mesh beneath it; every chip used to come off as white-hot ship
  plate.
- Nav beacons acquirable to 12 km, up from 6 km.

### 5. The game has a voice `[HERO]`

The first dedicated audio pass, and the chapter with the hardest asset problem
(the site's loops are muted - see Open questions).

- The set is re-recorded and twice the size: guns, ordnance, impacts,
  destruction, drives, cockpit, menus, editor - all synthesised from scratch.
  Only the ship computer's family is untouched.
- Sound is positional: a cue out in the world falls off with distance and pans
  with bearing.
- Your own ship is heard through its hull - its guns, drives and the damage
  landing on it never attenuate or pan - while everything else is across the
  gap.
- A hit is named by the round AND by what it struck: slug, penetrator and
  warhead each land differently, and each lands differently again on stone.
- The cockpit talks: lock taken or lost, denied or retargeted radar, the safety,
  a dry magazine, a hostile's lock arriving, the hull under thirty percent.
- Machinery has a voice: the PDC housing, the bay iris, the railgun capacitor
  bank.
- Four volume sliders - master, interface, world, music. Music is reserved.

### 6. Flying it, in meters `[HERO]`

Pairs the unit change with the flight-model change, because both are about
numbers you can feel.

- **(breaking)** Content and code are authored in meters through the
  `nova_events` quantity types. World units appear only at a Bevy, physics,
  rendering or build-grid boundary. One world unit is 10 m; one build-grid cell
  is one world unit. Every player- or creator-facing figure prints in meters -
  the editor inspector, the gallery, the wiki, the scopes.
- The Ledger (1.28.0) and Gauntlet Run (1.12.0) are republished in meters. A
  portal mod installed on 0.12.0 carries the old numbers and this build reads
  them as meters.
- RCS reaches 100 m/s at a mass-independent 5 G: one speed budget and one
  acceleration budget for every direction. STOP below that speed brakes on RCS
  without turning the ship.
- The 8 G turn limit is now a limit: the stick pins a ship at the hardest turn
  its hull survives, and a ship that loses its nose speeds up to the sharper
  turn its shorter body allows.
- GOTO parks a margin off the target's SURFACE, measured from the ship's own
  hull, and sizes every target it can - a beacon's orb, another ship's hull, an
  asteroid's geometry. A bare position still has no size.
- A GOTO at a planetoid arrives on a ring ORBIT accepts. ORBIT's park and the
  AI patrol's advance gate read the ship's own standoff, not the global one.
- A speed cap governs TOTAL speed, so turning and burning again no longer
  stacks a second cap's worth.

### 7. New Game is Basic Training

- New Game starts Basic Training: a Fleet gunnery range where Range Control
  talks a cadet through the helm verbs, GOTO and ORBIT out to a planetoid and
  back, the gun, five hulks and two drones that go live.
- **(breaking)** The game ships no campaign. `nova_protocol` and its five
  chapters are gone and the story is being rewritten. Retarget a mod that named
  one.
- Comms are drawn in their channel: work traffic in transmission blue, the crew
  in phosphor, a guard-channel catch faint, amber and tagged GUARD. Base
  content ships two portraits - the player's label and Range Control's face -
  reachable by any mod as `dep://base/portraits/<name>.png`.
- Comms use screen-relative cards with 20 px text and distinct speaker headers;
  a new line fades in without growing across the screen edge.
- A completed objective no longer ghosts green down the right of the screen.
- Every main-menu backdrop flies the block fleet. A duellist that leaves the
  arena forfeits and the survivor comes back to the middle of the shot.

### 8. NOVA OS gets a second shell

- Press `:` anywhere - menu, editor, flight, pause - and the ship computer
  opens on a second shell. One CRT, two languages: `commands` switches, and
  each keeps its own transcript and history.
- 27 commands in four classes. `help` and Tab completion come from one
  registry, and Tab offers the ship and section ids the world holds now.
- Cheats are refused until `cheats enable`, which marks the run and turns the
  header amber. A fresh scenario clears the mark.

### 9. A scenario can direct `[HERO]`

The modding chapter - 35 entries, the largest group.

- `Cinematic` plays a beat chain as a scene the player may leave, reporting
  `OnCinematicFinished` on every path out and `OnCinematicSkipped` first on a
  skip. A `Cinematic` filter matches by scene key; an unplayed key is a lint
  error.
- `SetCameraAnchor` rides the camera on an object at an offset, facing a point
  or another object; `SetCamera`/`SetCameraAnchor` take a `blend` of seconds and
  an easing, so a cut becomes a tracking move. `CinematicTitle` posts a title
  card in an authored corner and expires on the scenario clock.
- Six actions take a ship's helm - `MoveShipTo`, `ForceAlign`, `StopShip`,
  `PatrolShip`, `OrbitShip`, `ClearShipOrder` - one order at a time under a
  named key. An order outranks the bot: the AI stops flying and keeps shooting,
  and `order_interruption` lets it break off and resume where it left off. Five
  `OnShipOrder*` events report by key, ship and kind.
- `SetAILeash`, `SetAIEngageRange`, `SetAIPointDefenseRange` retune at runtime;
  `non_combatant: true` flies the routine and never fires.
- **(breaking)** `StoryMessage` is `NarrativeCue` and every line authors a
  `channel`. **(breaking)** `ForceTorpedoLaunch` is `ForceTorpedoFire` and names
  ONE bay. **(breaking)** A player controller's `infinite_ammo` is gone -
  `SetInfiniteAmmo` and `RefillAmmo` grant it from content.
- `ForceRailgunFire`, `PlaySound`, `Railgun` as a section kind, `Grammar` as
  content, `enabled_by_default` on a catalog entry.
- `content lint` checks helm orders, patrol routes, orbit wells, AI constraints
  and forced-shot sections, and reports each scenario's creative map as context.
- One TABLE declares the action vocabulary: a row generates the enum arm, the
  dispatch, the RON name, the menu label, the id stem and the injection class.
  A new action is five edits, not fifteen.
- The editor writes as many mods as you name ranges, each an `editor_`-prefixed
  bundle. File > Save As names the file and File > Open lists what there is; a
  document remembers its file.

### 10. Death is an event

- A death burns: a white flash, then incandescent fragments that outlive it and
  keep travelling, and light on the hulls nearby. A section going up is a
  compartment; a hull letting go covers the wreck.
- A torpedo's detonation is twice the size and burns three times as long:
  bigger core, ejecta past 100 m, a flash reaching 1.4 km.
- Debris turns about its own shape (a piece used to spin about the section
  origin, swing wide, then snap straight).

### 11. An agent flies it `[HERO]`

The surprise chapter, and the one with a ready-made asset (`--record` already
stitches an mp4).

- `bench play` seats an agent at the process channel - a scripted baseline, `pi`
  with any model, or any process on a unix socket - and scores the run from the
  game's own state.
- The snapshot is condensed into a pilot's view in meters: bearings from the
  nose, contacts, beacons, mounts. Near and in-the-way bodies carry in full;
  the rest is one line per band and quarter, and `observe {"expand"}` reopens a
  group. Every line is logged both ways to `audit.jsonl`.
- Five game pages - targeting, weapons, travel, orbit, fighting - read on demand
  through a `page` tool instead of riding in every prompt.
- Scoring: a run that armed cheats is cheated; unparsable wire lines are counted
  rather than refusals; an objective completed between two samples is credited
  from the flight log.
- `bench replay` grades a replay `match`, `close` or `mismatch`.
- `--record <dir>` draws every tick offscreen with the full HUD and stitches
  `<dir>.mp4` at 60 fps: the agent's run as a real-time movie, however long it
  thought.

### 12. Performance

Four measured wins, all of them ceilings rather than trims.

- A collapsing hull measures its pre-cut centre of mass once per frame, not
  once per destroyed section: 55 ms of one impact down to under 1 ms.
- A frame throws at most 128 carve chips: a capital hull raked open threw about
  10,000 and now throws about 2,000. One crater is untouched.
- Wreck pieces go physical two dozen a frame, so 700 sections shed at once no
  longer grow 700 colliders and their contacts on one frame.
- The editor stops re-walking the document each frame.

### 13. More in v0.13.0 `[FOLD]`

Bullet list, v0.12.0 style. Candidates: mouse sensitivity sliders and the
two-thirds default gain; Settings > Controls MOUSE group; the scenario spawn
queue taking a fifth of a frame; `skybox`/narrative band ids; only the pane
under the pointer scrolls; the trigger-volume leave fix; the severed flight
computer; the sandbox picket that mounts a railgun; the Gauntlet's course ship;
`SuspendPlayerControl`/`ResumePlayerControl`; `OnGotoComplete`/`OnStopComplete`/
`OnOrbitLap`; the debug build's commit stamp. Plus ONE line for the site work:
the wiki reads as a book, every figure in meters, a Railgun page with a corridor
scope and a combat range ladder, a Commands page - and one clause that the lore
encyclopedia and Story archive are up but not the subject of this post.

### 14. Modding and compatibility

All 11 breaking changes with their migrations, then the `content lint` line and
the `/create/` pointer. Ordered: units, Planet, asteroid material, torpedo bay
geometry, base fleet moved to The Ledger, campaign removed, `NarrativeCue`,
`ForceTorpedoFire`, `infinite_ammo`, input-ack schema 2, portal mods
republished.

### 15. Closing callout

## What the post needs: assets and visuals

### A. Already captured, alias only (no new work)

`scripts/capture-web-media.sh` ALIASES and `gen-web-screenshots.py` ALIASES
already carry this pattern. These exist at HEAD and show v0.13.0 behaviour:

| Chapter | Asset | Source |
| --- | --- | --- |
| Railgun | `loop-section-railgun.webm`, `loop-section-railgun-live.webm` | `screenshot_railgun` |
| Railgun | `wiki-section-railgun.png`, `-corridor.png`, `-sight.png`, `wiki-combat-railgun.png` | `screenshot_railgun` / `screenshot_section_weapons` |
| Sections | `loop-section-turret-stow.webm` (PDC stow) | `loop_turret_stow` |
| Sections | `loop-section-torpedo-bay.webm` (bay iris) | `system_torpedo_launch` |
| Sections | `loop-section-turret.webm` (4 mounts, lanes) | `stress_point_defense` |
| Sections | `wiki-section-turret-twin.png`, `wiki-section-hull{,-cargo,-tank}.png`, `wiki-section-controller.png`, `wiki-sections.png` | `screenshot_section_gallery` |
| Sections | 12 `catalog-*.png` cards incl. lance, siege lance, twin PDC | catalog producer |
| Block ships | `hero-wfc-duel.webm`, `landing-wfc-2v2.webm` | `wfc_arena` |
| NOVA OS | `command-shell-open.webm`, `wiki-nova-os-terminal.png` | `loop_command_shell` |
| Flight | `goto-arrival.webm`, `landing-player-flight.webm`, `wiki-gravity.png` | `loop_goto_arrival`, `loop_player_flight` |
| Death | `landing-damage-sequence.webm`, `torpedo-blast.webm` | `loop_damage_sequence`, `loop_torpedo_blast` |
| Lock | `lock-dwell.webm`, `wiki-radar.png` | `screenshot_radar_lock` |
| Editor | `landing-editor-build.webm`, `wiki-sandbox-range.png` | `screenshot_editor` |

Every one still needs a `news-0130-` alias entry so the post's copy is frozen
under its own namespace (`FROZEN_NAMESPACE` in `gen-web-screenshots.py`, and
the news-name-is-a-leaf rule in `capture-web-media.sh`).

### B. New captures needed in THIS repo

New loop producers or new poses in existing examples. Ordered by how much the
chapter needs them.

1. **`news-0130-railgun-lance.webm`** - one lance shot end to end at speed: the
   bore charging, the discharge and kick, the wake, and the corridor opening
   through the target. `screenshot_railgun` has the pose; needs a longer
   aftermath cut than the two shipped variants.
2. **`news-0130-asteroid-kinds.png`** - the four kinds side by side under one
   light. `examples/playable/asteroid_kinds.rs` and `compare_asteroids.rs`
   exist; need a screenshot producer wired into `gen-web-screenshots.py`.
3. **`news-0130-planet.png`** - a planet at true radius with a ship for scale,
   plus a second frame of the biome/terrain draw. `compare_planets.rs` and
   `planet_types.rs` exist as playable examples; same wiring gap.
4. **`news-0130-block-fleet.png`** - the eleven block ships on one bench.
   `wfc_ships.rs` / `block_bench.rs` are playable; need a framed gallery pose.
5. **`news-0130-hull-generate.webm`** - the editor generating a hull: seed
   named, rows ticked, a zone chip cycled, Generate, the HULL PLAN line, the
   ship replaced. New loop in `screenshot_editor`.
6. **`news-0130-basic-training.webm`** - Range Control talking a cadet through
   one beat: a comms card in transmission blue with a portrait, an objective
   chip, and the verb being flown. New producer, or a pose on the shipped
   scenario.
7. **`news-0130-comms-channels.png`** - the three channel styles on one frame
   (comms blue, crew phosphor, guard amber+tagged). Static; a widget could
   replace this if the frame is hard to stage.
8. **`news-0130-cinematic.webm`** - a title card and a camera blend tracking an
   anchored ship. `examples/systems/system_cinematic.rs` plays a scene live and
   is the obvious source.
9. **`news-0130-cladding.webm`** - skin plates coming off a hull under fire and
   tumbling on the ship's motion. Likely a pose on `loop_damage_sequence`.
10. **`news-0130-death-pyre.webm`** - a section going up vs a hull letting go,
    with the light on the neighbours. Same producer family.
11. **`news-0130-occlusion.webm`** - a held lock broken by a rock drifting
    across it. `examples/systems/system_lock_line_of_sight.rs` exists.
12. **`news-0130-agent-run.webm/mp4`** - a cut from `bench play --record`. The
    only asset that already renders itself; needs a 3 MB cut.
13. **`thumb-news-0.13.0.png`** - the news index card. Alias of whichever
    figure ends up carrying the post (railgun corridor is the strongest
    candidate).
14. **`news-0130-release-lead.webm`** - the lead loop. Recommend a block-fleet
    gun fight with a lance in it: it shows the new fleet, the new weapon, the
    new sound (silently) and the new death in one shot. Probably an alias of a
    re-cut `hero-wfc-duel`.

Budget note: `capture-web-media.sh` FAILS a loop over 3 MB, and every still
lands through `gen-web-screenshots.py`.

### C. Before/after, via `~/personal/content-machine`

These need the v0.12.0 capsule (`capture/nova-protocol/v0.12.0/`, which today
holds only `drive-scales.rs` and `vacuum-combat.rs`). Each pair needs a NEW bin
in that capsule plus a matched pose at HEAD, so the two frames differ only in
the version. Recommend three; more is a lot of staging for one post.

1. **Sections: cube vs machine.** The hull cell, the controller core, the PDC
   mount and the torpedo bay, same camera, same light. This is the release's
   single clearest picture. `[recommend]`
2. **A belt: one rock vs four kinds.** The same scatter seed shot on both
   builds. `[recommend]`
3. **Death: pop vs pyre.** The same section destroyed on both builds.
   `[recommend]`
4. Base fleet: modelled Racer/CargoA sandbox vs the block fleet. `[optional]`
5. Menu backdrop: v0.12.0 vs the block-fleet backdrop. `[optional]`
6. Comms: the v0.12.0 story bar vs the channel cards. `[optional]`

Constraints to respect: `docs/capture-capsules.md` says scenes are added only
in the CURRENT capsule, which is still v0.12.0 until this release tags - so the
window for adding before-scenes is now. Output lands under
`~/personal/content-machine/media/<slug>/` and is untracked; the cut webm is
copied into `web/src/assets/loops/` by hand as a frozen `news-0130-` asset,
which is the same freeze every news asset already carries.

### D. Widgets

Eight is the v0.11.0 standard. Existing reusable: `lance-corridor`,
`weapon-reach`, `damage-levels`, `point-defense`, `lock-sweep`, `goto-verb`,
`gravity-well`, `hull-armour`, `thruster-mass`, `ammo-rhythm`, `turret-arc`.

New for this post, ranked:

1. **`rcs-budget`** (ch. 6) - one speed budget and one acceleration budget for
   every direction: drag a heading and a mass and watch 100 m/s at 5 G hold.
   Best single explainer of the flight change. `[recommend]`
2. **`arrival-standoff`** (ch. 6) - GOTO parks off the target's SURFACE measured
   from your own hull. Drag ship size and target size; a warship and a shuttle
   stop the same distance clear. `[recommend]`
3. **`railgun-recoil`** (ch. 1) - the impulse at the muzzle, and the yaw an
   off-axis mount adds. Pairs with the existing `lance-corridor`. `[recommend]`
4. **`lock-occlusion`** (ch. 1) - drag a rock across the line and watch the lock
   break. `[recommend]`
5. **`asteroid-kinds`** (ch. 4) - the four kinds with palette, strata and the
   weighted mix a scatter draws. `[recommend]`
6. **`hull-zones`** (ch. 3) - the six zones of a hull and which cells a ticked
   part may stand in. `[recommend]`
7. **`collapse-budget`** (ch. 12) - 128 chips a frame, 24 wreck bodies a frame,
   one centre-of-mass walk: the three ceilings against the old counts.
   `[recommend]`
8. **`sound-map`** (ch. 5) - the cue families, their routes (interface, world,
   hull) and the positional falloff. Carries the audio chapter if a sounded
   clip does not land. `[recommend]`
9. `command-catalog` (ch. 8) - 27 commands in four classes. `[optional]`
10. `planet-scale` (ch. 4) - planet, planetoid, asteroid and ship at true radius
    in meters. `[optional]`
11. `channel-cards` (ch. 7) - the three comms channel styles, live. `[optional]`
    Replaces `news-0130-comms-channels.png` if that frame is hard to stage.

Every widget needs a source comment naming the game file the numbers come from,
per the pattern in `0.12.0.md`.

## Open questions

- **Audio has no route to the site.** `site.ts` upgrades a figure into a MUTED
  autoplay `<video>`, and `capture-web-media.sh` records video only through
  Xvfb. The audio chapter therefore has no way to play a sound today. Options:
  (a) carry it with the `sound-map` widget and prose only; (b) add a
  click-to-play sounded clip component to `site.ts` and an audio route to the
  capture script; (c) put an A/B on YouTube and embed. Needs a decision before
  the chapter is written.
- **Chapter count.** 15 sections against v0.12.0's 9. The `[FOLD]` marks and the
  `[HERO]` ranking are the cut list if the post runs long.
- **Lead loop.** Confirm the block-fleet-with-a-lance framing.

## Draft status (2026-09-09)

`web/src/news/0.13.0.md` is written with every figure as a placeholder,
registered in `webpack.config.js` `NEWS_POSTS` and carded in `news.html`
(date `2026-09-XX` until the tag). `npm run build`, `tests/assets.test.js`
and prettier are green. The owner reviews the placeholders next.

### Figures planned (35)

- `loops/news-0130-release-lead.webm` - Loop planned
- `loops/news-0130-railgun-commit.webm` - Loop planned
- `news-0130-railgun-sight.png` - Image planned
- `loops/news-0130-railgun-corridor.webm` - Loop planned
- `loops/news-0130-railgun-wake.webm` - Loop planned
- `loops/news-0130-lock-occlusion.webm` - Loop planned
- `loops/news-0130-sections-before-after.webm` - Comparison planned
- `loops/news-0130-pdc-stow.webm` - Loop planned
- `loops/news-0130-bay-iris.webm` - Loop planned
- `news-0130-hull-builds.png` - Image planned
- `loops/news-0130-cladding.webm` - Loop planned
- `news-0130-block-fleet.png` - Image planned
- `loops/news-0130-hull-generate.webm` - Loop planned
- `news-0130-hull-plan.png` - Image planned
- `loops/news-0130-block-duel.webm` - Loop planned
- `news-0130-planet-types.png` - Image planned
- `loops/news-0130-belt-before-after.webm` - Comparison planned
- `news-0130-asteroid-kinds.png` - Image planned
- `loops/news-0130-sound-a-b.webm` - Sounded clip planned
- `news-0130-volume-sliders.png` - Image planned
- `news-0130-editor-meters.png` - Image planned
- `loops/news-0130-rcs-box.webm` - Loop planned
- `loops/news-0130-goto-standoff.webm` - captured (11.2 s, 167 KB)
- `loops/news-0130-basic-training.webm` - Loop planned
- `news-0130-comms-channels.png` - Image planned
- `news-0130-menu-backdrop.png` - Image planned
- `loops/news-0130-command-shell.webm` - Loop planned
- `loops/news-0130-cinematic.webm` - Loop planned
- `loops/news-0130-helm-orders.webm` - Loop planned
- `news-0130-editor-save-as.png` - Image planned
- `loops/news-0130-death-before-after.webm` - Comparison planned
- `loops/news-0130-torpedo-blast.webm` - Loop planned
- `news-0130-agent-view.png` - Image planned
- `loops/news-0130-agent-run.webm` - Loop planned
- `loops/news-0130-hull-collapse.webm` - Loop planned

### Widgets declared (10)

- `lance-corridor` - registered, renders live
- `railgun-recoil` - NEW - prose fallback until written
- `lock-occlusion` - NEW - prose fallback until written
- `hull-zones` - NEW - prose fallback until written
- `asteroid-kinds` - NEW - prose fallback until written
- `sound-map` - NEW - prose fallback until written
- `rcs-budget` - NEW - prose fallback until written
- `arrival-standoff` - NEW - prose fallback until written
- `command-catalog` - NEW - prose fallback until written
- `collapse-budget` - NEW - prose fallback until written

`thumb-news-0.13.0.png` is referenced by the index card and does not exist
yet; alias it to the figure that ends up carrying the post.

## Media status (2026-09-09, second pass)

The sound board (`sound-board` widget, five families, 44 WAVs copied verbatim
to `web/src/assets/sounds/`) plays the game's own cues through Web Audio with
drawn peak envelopes. Nine pure-TS widgets are being written in a lane
(`railgun-recoil`, `lock-occlusion`, `hull-zones`, `asteroid-kinds`,
`sound-map`, `rcs-budget`, `arrival-standoff`, `command-catalog`,
`collapse-budget`); proof screenshots land under `proof/widgets/`.

### Captured

Loops, packaged from the loop stage with `NOVA_REUSE_STAGE=1` (nine
`news-0130-*` aliases in `scripts/capture-web-media.sh`): release-lead and
block-duel (both `hero-wfc-duel`, a bespoke lead is still owed),
railgun-commit, railgun-corridor, pdc-stow, bay-iris, cladding,
command-shell, torpedo-blast. The reuse re-copied four LIVING loops from a
stage older than their committed cuts (`command-shell-open`, `goto-arrival`,
`landing-cockpit`, `landing-damage-sequence`); those were restored from git
and the two news aliases cut from them re-copied off the committed files, so
the manifest rows for `news-0130-cladding` and `news-0130-command-shell`
carry the committed sizes. A full loop sweep at release re-cuts the stage.

Stills, packaged by `scripts/gen-web-screenshots.py`: railgun-sight,
menu-backdrop, volume-sliders, editor-meters and the post card by ALIASES;
hull-builds by TRIPTYCHS; asteroid-kinds (`asteroid_kinds` grid, five kinds by
three seeds with the plain control), planet-types (`planet_types` lineup, the
bench readout cropped off) and the new hull-row (`wfc_ships`, three collapsed
hulls in the industrial skin) by the new CUTS table - a 16:9 window of a
staged bench frame under a news name, cut from `target/news-shots/`.

`news-0130-agent-view.png` is no longer a figure: the section quotes one real
turn of `bench-runs/395a8f1b/tutorial/pi-gpt-5.6-sol-medium-1/audit.jsonl`
as a code block (view, agent text, act, echo).

### In flight

- `news-0130-railgun-wake.webm`: `railgun_wake_bench` gained a recorded cut
  (`NOVA_WAKE_LOOP=1`, LOOPS row in `capture-web-media.sh`): the close pose,
  one volley slowed to a twentieth as the middle lane's slug crosses, real
  time again once it has left. First take slowed the clock a frame too late
  and recorded the crossing at full speed; re-cut with the slowdown two real
  frames before the window.
- `news-0130-block-fleet.png`: `first_shift_ships` gained a shot
  (`first-shift-ships.png`, cut whole by CUTS); reframed from 620 m after the
  parking pose put the carrier alone in the frame.
- `news-0130-hull-generate.webm`, `news-0130-hull-plan.png`,
  `news-0130-editor-save-as.png`: an editor-producer lane is extending
  `screenshot_editor` (or a sibling) with the Generate and Save As walks.

### Still pending a producer

Loops: lock-occlusion, rcs-box, goto-standoff, basic-training, cinematic,
helm-orders, agent-run, hull-collapse; the three content-machine
before/after pairs (sections, belt, death); sound-a-b (a sounded clip - the
sound board may make it redundant). Stills: comms-channels (a HUD pose with
three channels' cards up).

## Media status (2026-09-09, third pass)

Every still the post names exists under `web/src/assets/`, the post card
included. Of the 22 loops the post names, 18 exist; four rows in
`web/src/assets/loops/manifest.txt` are still `pending`.

### Captured this pass

- `news-0130-helm-orders.webm` (13.1 s, 447 KB): `loop_helm_orders`, new
  under `examples/screenshots/`. A gunship on a GOTO leg east, a salvage
  skiff crossing north of the route declared hostile as the gunship passes
  x = -250 m; the helm breaks off, takes the skiff apart, and, with the sky
  clear, resumes the same leg. Three takes: a fixed 950 m lens made the
  ships specks and the skiff at 70 m/s drew the fight off frame; a fixed
  650 m lens lost the attack runs; the shipped take follows from above
  (`FightLens`: the midpoint of the gunship and the live skiff, the height
  fitting the pair with a 120 m margin, exp-lerped at 2.5/s, held through
  `pose_camera`).
- `news-0130-hull-collapse.webm` (7.5 s, 2.55 MB): `stress_hull_collapse`
  gained a loop mode (`NOVA_COLLAPSE_LOOP=1`) that records the volley and
  6 s of aftermath from a fixed pose with the clock pinned. The range still
  measures nothing in that mode; its module docs say so. The first cut was
  3.98 MB at the default CRF, so the loop profile runs at CRF 40. The
  harness needed one change: `nova_debug::harness` now re-exports the
  `completion` module, so a range with no autopilot script can register its
  own collector - without one `LoopCapturePlugin` reported done at startup
  and the app exited before the first frame.
- Sound board A/B: sixteen rows (the twelve base cues and the four
  interface cues whose file existed at v0.12.0) carry an amber `v0.12.0` key
  that plays the recording this release replaced, drawn in amber while it
  runs (`web/src/assets/sounds/v0120/`, `soundCueUrl` in `widgets.ts`).
  This replaces the planned `news-0130-sound-a-b.webm`: the site's loops are
  muted, so a sounded clip was never makeable there, and the board plays
  both sets. The row is four named grid lanes - keys, wave, length, words -
  so the words can never run under the wave; under 560 px the words drop
  to a second line. Proofs: `proof/widgets/sound-board-ab-playing.png` (the
  drives board with the old thruster loop up), `sound-board-ab-rest.png`
  (the interface board's four amber keys) and `sound-board-ab-narrow.png`
  (400 px).
- Page width: the quoted agent turn (a `text` code block with 76-char
  lines) made the whole post 611 px wide under 860 px - the mobile
  `.news__body { margin: 0 auto }` shrink-wrapped the article to its widest
  code line. `width: 100%` on that rule (and `min-width: 0` on both grid
  items) lets the pre scroll instead; the 0.12.0 post never hit it because
  its code blocks are narrow.
- Widgets: all nine pure-TS widgets landed and their proofs under
  `proof/widgets/` were reviewed; the post's hull-plan, editor-save-as,
  goto-standoff, helm-orders and hull-collapse notes were rewritten to say
  what the media actually shows.

- The three before/after pairs landed from the lane: `news-0130-sections-`,
  `-belt-` and `-death-before-after.webm` (8.1 / 9.0 / 6.6 s; 824 KB /
  1.26 MB / 177 KB), each the middle 640x720 of the content-machine
  `v0.12.0` capsule take on the left and the same crop of the v0.13.0
  producer (`loop_sections_compare`, `loop_belt_compare`,
  `loop_death_compare`, new under `examples/screenshots/`) on the right,
  trimmed to equal frame counts. Verified by tiling both halves; the death
  pair was tiled around the kill (frames 44-59) to see the flash. The belt
  and death scripts wait for the scenario spawn queue to drain before the
  loop opens - the first v0.12.0 belt take opened before the rig's lights
  had spawned. The v0.12.0 producers live in the content-machine capsule
  (`capture/nova-protocol/v0.12.0/src/bin/*-compare.rs`, projects
  `v012-*-compare`), uncommitted there. The "Death is an event" prose was
  aligned with the footage (a few sparks and the cell cut loose, not a
  puff).

### Still pending

- `news-0130-goto-standoff.webm` landed once the arrival creep was fixed in
  the flight computer (the RCS settle scaled to the manual cap, the spool
  tail counted as a linear wind-down, the leg flown from the body origin
  instead of the centre of mass, the flip slewed at crumb urgency, and the
  hull's settle onto the brake attitude left out of the flip lead). The
  range's park log now reads the block warship at 146 m and the block
  gunship at 143 m off the orb's face for a 150 m margin, from +86 / -15 m
  before; the residual is the crumb the RCS settles at the standoff.
- A bespoke release lead: `news-0130-release-lead` is still the
  `hero-wfc-duel` alias shared with `news-0130-block-duel`.

### Observations for the owner

- In the collapse loop the shell drifts after the volley (5.2 m/s, 0.00
  rad/s in the shipped run) although the railgun applies no impulse to the
  struck body: the 720 detached pieces go solid inside the shell and shove
  it. The footage keeps it; the caption calls the loop a picture and the
  range's log the measurement.
- The editor lane reported that scrolled Scene rows stay pickable under the
  top bar; not fixed here.
- The post's date is still `2026-09-XX`.
