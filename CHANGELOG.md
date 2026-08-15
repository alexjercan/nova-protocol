# Changelog

All notable changes to this project are documented here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
but groups each release by subsystem (Gameplay & Flight, Combat & Weapons,
Ships & Sections, Scenarios & Objectives, Modding & Mod Portal, Interface & HUD,
Web & Platform, Audio & Visuals, Performance, Fixes, Internals & Tooling) rather
than by Added/Changed/Fixed. Entries are kept SHORT - one commit-title line each;
the per-release News post (`web/src/news/<version>.md`) is where the detail and
narrative live. This project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Breaking changes are
tagged **(breaking)**.

## [Unreleased]

### Combat & Weapons

- Gunfights happen at 1-2 km instead of 4-5. A PDC round lives 2.0 s (reach
  200 u) and the scavenger turret 3.0 s (180 u), and the AI ranges moved with
  them: ships settle at a 100 u standoff (+-25 u), open fire inside 180 u,
  answer inbound torpedoes at 150 u, and commit to a fight at 400 u. Muzzle
  speed is unchanged, so no weapon's damage moved.
- **(breaking)** The per-section damage-resistance table is GONE. Damage is one
  number: a round deals the same amount to a hull, a thruster or a turret, and
  a blast deals the same to everything in its radius. A damage type's identity
  is now how the round TRAVELS, which is something a player can watch happen,
  rather than an invisible multiplier.
- **(breaking)** The damage roster is cut to `Kinetic`, `Pierce` and
  `Explosive`. `Emp` is removed (no shipped content authored it) and
  `ArmorPiercing` is renamed `Pierce`. No compatibility aliases: a mod naming
  `Emp` or `ArmorPiercing` must be re-authored.
- **(breaking)** The two bullet types are a punch and a rake. Kinetic keeps the
  hardest single hit and carries on only through what it DESTROYS, spending its
  damage as the budget, so its total can never exceed what it was fired with.
  Pierce deals lower damage but deals it IN FULL to every section it crosses,
  alive or dead and undiminished by depth, so its total legitimately exceeds one
  round's worth. Nothing of either type gets through an indestructible obstacle.
- **(breaking)** Both bullet types read the round's CLOSING SPEED against what
  it hits, anchored so closing at 100 u/s (a stock PDC's muzzle speed)
  multiplies by exactly 1.0: Kinetic turns speed into damage per hit (clamped
  0.25x..2.0x), Pierce into POWER - how much thickness it rakes through
  (clamped 0.5x..3.0x) - and never into damage. Charging hits harder, fleeing
  hits less.
- **(breaking)** A Pierce round pays for travel out of a separate power budget,
  spent on each layer's FULL health rating rather than its remaining health, so
  light plating is nearly free to cross while a heavy block is expensive and
  softening a section first cannot open a cheaper hole through it. A hard
  six-layer cap bounds the chain.
- **(breaking)** `SectionDamageClass` is renamed `SectionClass`: with the table
  gone it is the ship computer's section LABEL, not a damage key.
- **(breaking)** `pdc_turret_section` splits into `pdc_kinetic_turret_section`
  ("PDC Turret (Kinetic)", 4.0/hit) and `pdc_pierce_turret_section` ("PDC Turret
  (Pierce)", 2.0/hit): the same mount, fire rate and magazine, so mounting one
  of each is a straight punch-versus-rake comparison.
- **(breaking)** Fixes a live double-damage bug: avian raises one
  `CollisionStart` per event-enabled collider, so a production round hitting a
  health-bearing section reported the contact twice with the sides swapped and
  paid out on both - 20 authored dealt 40. Real turret damage therefore halves
  at unchanged authored values, which is the number those values always meant.
- **(breaking)** Torpedo ordnance stops dying to one bullet: the default
  `projectile_health` goes 1.0 -> 10.0, above the hardest single round a stock
  PDC can land (4.0 authored x the 2.0 kinetic speed ceiling), so an intercept
  costs two or three rounds instead of one lucky tap. Bays gain the authorable
  field; the siege bay's armoured ordnance is unchanged at 5000.
- **(breaking)** Standard torpedoes hit like torpedoes now: blast damage
  100 -> 750 EVERYWHERE, so a connecting torpedo all but decides a
  small-craft fight (Expanse-style). That is the baseline for every bay -
  the catalog section, the gunship's tubes, and an un-authored bay's
  default - with only the capital-grade siege bay above it. The counter is
  point defense, not armor - the ordnance is still shot down in a burst.
- Turrets no longer aim into the hull they stand on: the pitch hinge's
  depression floor tightens from 30 to 10 degrees below level (elevation
  still reaches 90 for the point-defense arc).
- **(breaking)** Neutralization is "no weapons OR no flight computer" now
  (was "no weapons AND no thrusters"): a brain-dead ship cannot aim or fly,
  so it is out of the fight whatever else survives, and a disarmed runner
  is beaten even at full burn. Ships that never had a computer (bare
  emplacements) still only neutralize by losing their guns.
- Retires the "guns and thrusters gone" defeat copy that outlived the rule
  above: every neutralized-defeat banner now reads "Nothing left to fight
  with", which is true whichever half went.
- New `heavy_torpedo_section` prototype: a capital-grade siege bay with a
  ship-killing blast and armored ordnance, hidden from the editor gallery
  (scene dressing, not player kit).
- New `ForceTorpedoLaunch` event action: script a controller-less ship's
  torpedo bays to launch at a named target on timers - dumb emplacements
  with authored cadence instead of full AI.
- **(breaking)** Unlimited player ammunition is a DEBUG-ONLY cheat. The
  `infinite_ammo` player flag is honored only by a build carrying the `debug`
  feature (examples, the probe harness); the shipped game logs a warning and
  keeps the authored magazines. Every scenario, base or mod, is played with
  real ammunition now, so holding the trigger costs a reload and point defense
  is a decision instead of a reflex.

### Gameplay & Flight

- **(breaking)** A hull may carry several flight computers and steers better
  for it, on a curve with a real ceiling: the stack's torque budget tops out
  at twice its strongest computer (so no ship snaps around, whatever it bolts
  on) while its precision - how early it starts arresting a turn - keeps
  improving. Half the gain arrives with the second computer; the tenth is
  worth under a percent. Measured on a 15-section barge: overshoot falls
  9.3 -> 1.4 degrees and the flip finishes 23% sooner while peak turn rate
  moves under 4%. Single-computer ships fly exactly as before; multi-computer
  builds no longer stack torque linearly (two computers were worth 2.0x, they
  are worth 1.5x now) and no longer risk the damping instability linear
  stacking walked into.
- Losing one computer on a stacked hull degrades its handling to the smaller
  stack instead of casting the ship adrift. The last one is still the ship's
  brain: autopilot and neutralization both key on "no computer LEFT".
- Sliced mesh fragments (asteroid debris) now despawn after 30 seconds instead
  of persisting until scenario teardown, so long-lived scenes no longer
  accumulate physics bodies.
- AI patrol legs steer around sized bodies: a leg blocked by an asteroid's
  geometric radius detours past it instead of flying the GOTO straight
  through the rock.
- AI ships gain an authorable `engage_range` (default 800): the
  hostile-detection distance a passive ship leaves its routine for, so
  long-watch emplacements can wake for targets that cannot be pulled toward
  them in return.
- AI ships gain an authorable `pd_range` (default 400): the distance point
  defense starts engaging inbound torpedoes, so a scene can stage its
  intercepts close-in instead of at the edge of turret reach.
- AI ships gain an authorable `waypoint_slack` (default 25) and
  `arrival_standoff` (default 50): together they set how close a patrol
  presses to its waypoints - a nav-drill ship can park nearly ON its
  beacons instead of turning 75 u out.
- New `SetAmmo` section modification: a hard magazine (rounds overridden,
  auto-reload stripped) for ships whose ammunition is the scene's clock.

### Ships & Sections

- **(breaking)** Ships collapse structurally: a hull carrying less than a
  fraction of the health it was BUILT with (0.25 by default) comes apart and
  is destroyed, instead of having to be dismantled section by section.
  Authorable per ship as `collapse_threshold` - `Some(0.1)` for a capital that
  must be taken further apart, `Some(0.0)` for the old strip-every-section
  rule. Neutralization is untouched: a disarmed ship with a sound hull is
  still a live derelict, not a wreck.
- Ships can wear a DERIVED skin: `skin: true` on a spaceship and the game clads
  the whole hull at spawn from the structure alone. Nothing places a plate,
  nothing authors one and no id names one - every plate's shape follows from the
  eight boundary samples its cell shares with its neighbours, so neighbours meet
  exactly and a run of plates is one continuous hull. Cladding is
  DESTRUCTIBLE: a plate carries its own health and mass, comes off when it is
  shot out and leaves a hole a piercing round carries on through, and it never
  counts toward the ship's own health or holds a severed hull together. A
  destroyed section takes its own cladding with it. Off by default, and only for
  hulls built out of the unit-cell sections.
- `basic_thruster_section` carries ONE socket now, on the forward face it
  bolts by - the rest of the part is barrel, nozzle and plume, so nothing
  mounts on the drive or plates over its exhaust.
- A collapsing ship TEARS ITSELF APART instead of popping out of existence:
  every section still standing is disabled at the collapse, the outermost ones
  blow off first with their own debris burst, and the wreck peels inward over
  the following frames until the root dies with it. A remnant with no loose end
  to start on (sections mated in a ring) is forced apart anyway, so nothing can
  hang there as an indestructible hulk.
- A ship coming apart keeps fighting until the sections carrying its guns blow
  off, and its sections now fire their own `OnDestroyed` on the way out - the
  same events a ship dismantled by gunfire already fired. The ship's own
  `OnDefeated` and `OnDestroyed` still fire exactly once each, a few frames
  later than before (at the real death, not at the moment of collapse).
- The hull cast swapped: the cargoa is the campaign's armed corvette now
  (PDC turrets on new nose-cheek mounts, player and raider grades), and
  the racer flies unarmed as the civilian the story protects - the Ceres
  Queen is a yacht, the Lifeline convoy a yacht and a courier. The
  `racer_turret_*` prototypes stay in the catalog for mods that want them.
- The editor places parts by MATING link points instead of stepping one unit
  along the surface it hit: it snaps the part's chosen socket onto the socket
  nearest the pointer, opposes their normals, and leaves the builder the roll
  and which of the part's sockets does the mating. A placement that would take
  an occupied socket, leave a socket with two suitors, or bury the part inside
  a section it does not mate with is refused, and the status line says which.
  The ghost is the part's real mesh.
- The semantic Racer, CargoA and CargoB parts join the editor palette - they
  were hidden while placement was a grid step, because a nose only fits a
  fuselage where its authored sockets say.
- A part now mates the same way up on ANY socket, so parts from different
  craft fit each other. A socket's roll zero is derived from its normal and
  the two socket FRAMES are mated (a shortest-arc rotation left the roll to
  whatever axis it happened to sweep about), and authored normals derived from
  part geometry snap to an axis - the cargob's pod faced its fuselage 36
  degrees off -X, and everything mated onto that socket arrived tilted by
  exactly that much.
- New `pdc_turret_section` ("PDC Turret"): one compact point-defense mount that
  fits any hull face, replacing ten per-craft copies of the same gun in the
  editor's Weapons tab. Its sockets follow its own 0.3 size, which is what lets
  a small mount sit ON a unit-cube hull instead of standing in for one.
- The per-craft `*_turret_*` prototypes are catalog-only now: ships and mods
  still name them, the editor offers the shared PDC instead.
- A turret's base stands on the face of the section it mounts through. The
  joint tree hardcoded the unit cube's -0.5, so a mount authored at its own size
  planted its base below its own underside and sank the gun into the hull it was
  bolted to. (The per-craft modules keep the old offset their ships were framed
  with.)
- `render_mesh_transform` takes a `scale` alongside its position and rotation,
  so art can be resized without touching the collider, the sockets or the mass.
  It also sizes the DEFAULT primitive an unmeshed turret joint gets - a plate a
  full unit across, which is why a turret on a small mount used to wear a
  hull-sized base plate whatever else was scaled.
- The shared PDC is assembled at HALF a section now, one number driving its
  collider, its sockets and its art, so it reads as a gun bolted to a hull
  rather than a full-size turret balanced on a small box. A whole assembly
  scales by its joint offsets AND its joint art together; either alone comes
  apart.
- Semantic parts are named for the craft they came off - `CargoB // Nose`
  rather than a third part called `Nose`.

### Scenarios & Objectives

- Chapters no longer cut from the killing blow straight to the modal
  Victory banner: a win now posts the beat it earned, then plays two
  timer-paced outro comms beats over the live world before the overlay and
  the queued next chapter. Built on the first-class scenario timers
  (`TimerStart` + `OnTimerEnd`) rather than clock-mark expression trees, and
  the win locks the instant it lands, so a death during the outro cannot
  overwrite it.
- New `Anchor` scenario object: an invisible authored gravity well (radius +
  optional mass, no mesh or collider) for orbit targets and bodiless gravity
  in scenes that do not want a rock at the anchor point.
- **(breaking)** Menu backdrops pose their own camera: a `SetCamera` in
  OnStart is the contract (lint Error without one), replacing the old
  derive-from-`menu_planetoid`-well framing.
- The main menu is a backdrop CAROUSEL now: four scenes hand off to each
  other Factorio-style (menu entry starts the ring at a random one). Three
  combat scenes are new - Torpedo Gauntlet (a station-keeping corvette's doomed
  point-defense stand against scripted batteries on both flanks; its hard
  magazines run dry and the stream wins), Asteroid Weave (a ten-waypoint run
  hugging its beacons through a dense rock band), and Duel Cycle (a
  center-frame duel whose winner is erased by a siege torpedo) - joining
  Waystation Traffic. Menu Ambience and Scrapyard Drift retire (three
  planetoid-and-orbiter scenes were one too many... two too many). Each
  hand-off is a full scenario switch that clears wrecks and ordnance, the
  endless scenes carry a rotation time limit, and stall watchdogs guarantee
  the ring always turns.
- Asteroids gain an optional `seed` pinning the generated silhouette (and the
  derived geometric radius) across runs; `ScatterObjects` fills it
  deterministically from the scatter seed, so authored fields keep the same
  shapes every load.
- The editor's Play sandbox becomes a RANGE rather than an empty field: two
  seeded rock belts (64 rocks, up from 20), a corridor of five inert target
  hulks to shoot, three DORMANT pickets that stay neutral until you paint them
  with a combat lock or fly into their trip sphere, and two beacons that swap
  the skybox as you pass through. It still has no ending - the standing
  objective just names F1 as the way back to the editor - but dying now offers
  a Retry of the same range instead of a silent restart.

### Interface & HUD

- The editor SHOWS the ship's derived skin while you build it. **Ship Skin** in
  the Tools block clads the build from its own structure, and the cladding
  re-derives whenever that structure moves - including around the part still
  under your pointer, so a hull is dragged about UNDER the skin and the plating
  closes over it before the click that commits it. A refused placement stays
  bare, since it will not be built. The toggle rides Play: a ship built clad is
  a ship flown clad.
- The editor grows a **parts gallery**: a full-screen browser of the section
  catalog with a live 3D preview per tile, a category row, a text filter (just
  type) and a focus card that turntables the part beside its stats. Picking
  from it arms the placement tool, so a part is found by LOOKING at it.
- **(breaking)** The Components drawer is gone; the gallery replaces it. A text
  card cannot say what a part looks like, which is the one thing a parts list
  owes a builder.
- The focus card takes direction: drag to turn the part, wheel to zoom, and the
  turntable picks up again from wherever you left it once you stop.
- Link points are visible while a part is armed - every free socket draws a ring
  and a stub along its normal, the one under the pointer draws bright, and the
  armed part's mating socket draws on the ghost.
- Placement pose control is reversible: the wheel rolls the ghost and
  Shift+wheel cycles its socket, so overshooting costs one notch back instead of
  five more forward. `R` and `F` still step forward.
- `Q` picks up whatever part is under the cursor, Factorio-style, and the
  editor's bottom-left legend lists the keys that apply to what you are holding.
- `Tab` opens and closes the parts gallery from the build view, and `Q` over a
  tile takes that part and hands you straight back to placing it - no focus
  card, no Place button, no click.
- The gallery's search takes the keyboard only once the field has the caret
  (`/`, or click it), so letters are free to be shortcuts; `Enter` opens the top
  hit and `Esc` backs out of the field before it backs out of the gallery.
- The socket cycle moved from Shift+wheel to **Ctrl**+wheel: Shift is the
  free-fly rig's descend key, so cycling a socket also sank the camera.
- The preview ship draws its forward direction, which a pile of boxes otherwise
  has no way to show.
- The editor lights its scene with a key and a rim (it had one light pointing
  straight down, leaving every vertical face flat), and a gallery tile is
  fitted to what it DRAWS rather than to its collider - a turret used to spill
  its barrel four cells wide.

### Audio & Visuals

- Turret rounds are modelled per DAMAGE TYPE instead of one shared box: a
  Kinetic slug is a stubby 1.2 m tracer, a Pierce round a 2.2 m needle a third
  the thickness, an Explosive shell squat and wide. All three are shorter than
  the 3 m box they replace, and they burn in the damage type's own HUD colour,
  so a round in flight matches the ammo pip that loaded it. A turret's authored
  `projectile_render_mesh` still overrides all of it.
- Torpedoes fly nose-first: the warhead is a coned body instead of a
  flat-ended pipe, and one shared mesh serves every launch.

### Fixes

- **(breaking)** A ship's HP bar no longer FILLS UP as the ship is shot apart.
  A root's maximum health is pinned to the hull it was built with (a running
  maximum) instead of being re-summed from the surviving sections, so
  destroying a 1000-hp section no longer takes that 1000 out of the
  denominator as well: 150/1100 used to read 100/100.
- Weapons no longer offer a mating surface where their business end is. The
  shared PDC mount carried the full six-socket cube, so a builder could stand
  a second turret on the first one's barrel or plate a slab across its
  traverse; it now sockets its base plate and nothing else. Both torpedo bays
  drop the socket on the face they fire through, so a section can no longer be
  bolted over a muzzle.
- **(breaking)** Turret rounds deal their authored damage once, not twice.
  Avian raises a collision event per event-enabled collider, so a round (which
  carries its own) hitting a health-bearing section (which the integrity hook
  enables) reported the same contact twice with the sides swapped, and the hit
  observer paid out on both. Every turret has been hitting for 2x its authored
  number against ships; the authored values may want re-tuning.
- The editor sandbox no longer spawns you on top of the planetoid. An
  asteroid's drawn surface is 3.5-6x its authored radius, so the old 55u
  planetoid was a ~250u ball of rock parked 314u away, with a well reaching
  1095u - you started ~60u off its surface and fell in. It is now a smaller,
  seed-pinned body far outside its own reach of the spawn.
- ESC in the editor backs out (closing the parts gallery, then putting the
  armed part down) instead of stacking the pause overlay on top of it.
- TAB no longer arms the NOVA OS where there is no ship to fly: the editor's
  build mode is inside `Playing`, so a TAB there set the freeze state
  invisibly - and pressing Play dropped straight into a NOVA OS nobody opened.
- The section keybind chips no longer hang over the parts gallery. They were
  positioned by a system keyed on the free-fly camera controller, which the
  gallery removes when it parks the camera, so the chips froze where they were.
- A gallery rebuild no longer flashes its parts across the middle of the screen:
  the preview bundle carried a `Visibility` that overwrote the tile's own
  hidden one, so every tile drew a frame at the stage origin.

- A mid-menu backdrop reload (the self-resetting backdrops) no longer
  crashes the UI layout: the menu interface renders through its OWN camera
  now, so the scenario camera being torn down and respawned can never yank
  the UI's render target mid-frame; the backdrop view itself holds its last
  scripted pose through the swap instead of blinking.
- The menu's interface camera no longer re-renders world-space particle
  effects over the finished frame (blast VFX ghosted across the menu): the
  overlay draws on an empty render layer, UI only.

### Internals & Tooling

- The base mod generates its own decoration art: `scripts/gen-greebles.py`
  builds one `.glb` per JSON recipe in `scripts/greeble-recipes/` out of five
  primitives (box, cylinder, taper, ribs, disc), byte-deterministically and
  gated by `--check`. Four magenta placeholders ship to prove the pipeline.
- The three mesh scripts share one stdlib-only glTF writer,
  `scripts/nova_glb.py`, instead of a copy each; the 21 shipped ship-part
  meshes re-cut byte-identical from their recipes.
- New `wfc_ships` screenshot producer: wave function collapse over the section
  catalog, where the adjacency rules ARE the link points - a socket must meet a
  socket, so mounts land on the skin and bays keep their muzzles clear without
  anything in the generator knowing what either part is. Every generated hull
  goes through the real `lint_scenario` before it is posed.
- `wfc_ships` collapses STRUCTURE and nothing else. It builds no skin, names no
  plate and knows nothing about cladding: a ship asks for a skin with one flag
  and the game derives the whole of it at spawn, so a clad row is evidence that
  the derivation reads structure rather than a picture of what the generator
  already decided. `--bare`, and `C` in a hand-run, turn the flag off on the
  same seeds, which is what makes the pair a before and after.
- `wfc_ships` states its rule as: a socket may never press into a face that has
  none. Two sockets meeting is a mate; a socket against a blank face is what must
  never be built. Where NEITHER face has a socket the two may touch: a drive's
  flank is the side of a cylinder and a mount's is its housing, so two of those
  resting against each other is an engine cluster and not a fault. What this
  does NOT model is CLEARANCE - a muzzle or a nozzle wants the cell in front of
  it empty - because a socket set cannot tell the mouth of a barrel from its
  side; the content already knows where a plume and a muzzle point and
  clearance should be read off that.
- `wfc_ships` no longer masks a face off the drive. It carried one assertion
  over the catalog, because a six-socket thruster called its own nozzle a mating
  surface. The drive says that itself now, so the generator only reads link
  points.
- `NOVA_MENU_BACKDROP=<scenario id>` pins the menu's backdrop draw for
  capture and authoring runs; unknown ids fall back to the random pick.
- The autopilot gains a `type_text` gesture: a driven run can type into a text
  field (the editor gallery's filter), which `press_key` cannot do - it writes
  only the held-key state, not the text a keypress produces.
- `cut-obj-into-parts.py` proposes link-point candidates from recipe seams -
  one socket per shared face, written into each part manifest - and a recipe
  part can author its own list instead. Judgement stays with whoever promotes
  a candidate; shipped gameplay sockets are still hand-authored in Rust.
- Built-in authoring content now has an explicit `base_content` inventory grouped into Nova Protocol chapters, private main-menu backdrops, sandboxes, standard section prototypes, and per-craft semantic parts; generated RON is unchanged.

## [0.10.0] - 2026-08-13

### Ships & Sections

- **(breaking)** Ship integrity now uses explicit section link-point mates. Collider contact and one-unit center spacing no longer create structural edges; multi-section mods must author one connected link-point graph. NOVA OS toggles a `MATES` structure overlay with `G`.
- **(breaking)** Racer, CargoA, and CargoB now use semantic parts such as `fuselage`, `engine_port`, and `turret_starboard`. Coordinate-named cube prototypes and meshes are removed; bundled mods use the new ids.

### Gameplay & Flight

- **(breaking)** Gravity wells are authored by MASS, not surface gravity:
  `AsteroidConfig::surface_gravity` becomes `mass` (the `mu` in `a = mu / r^2`)
  and the sphere of influence is where that pull decays to the new
  `GravitySettings::soi_cutoff_accel`, so a body's reach no longer multiplies
  its noise-mesh radius - the same rock is the same well on every seed instead
  of swinging 1.7x. `soi_factor` and `default_surface_gravity` are gone; a mod
  authoring `surface_gravity` loses that setting silently.
- The chase camera takes the SHORT way around the rear (-Z) seam: an orbit that
  crossed it used to swing the long way round as the angle wrapped.

### Combat & Weapons

- Neutralized wrecks leave player threat tracking and AI target acquisition but
  remain combat-lockable; their solid allegiance triangle becomes a hollow
  wreck chevron, target details show `NEUTRALIZED`, and the target inset gives
  a brief defeat confirmation while preserving allegiance color. Physical
  destruction shows an amber `DESTROYED` ribbon over the final two-second kill
  cam; cleanup despawns no longer trigger that kill cam.
- AI burst cadence ticks on the fixed clock, so AI damage output no longer
  varies with framerate.

### Scenarios & Objectives

- **(breaking)** Scenario world reads now use typed queries and declared watched variables. `Scenario(Elapsed)` replaces the implicit `scenario_elapsed` value, `Entity(... Speed)` replaces implicit player speed, and inline `Query(...)` factors support one-shot snapshots.
- Rust scenario authors gain one public `nova_authoring::scenario_helpers` catalog for common expression, filter, watch, and action constructors; built-in scenarios and examples no longer import generic helpers from Shakedown.
- Scenario handlers gain `OnDefeated`, an exact-once ship outcome edge shared
  by neutralization and direct destruction. It precedes `OnNeutralized` or
  `OnDestroyed`; later destruction of a neutralized wreck emits only
  `OnDestroyed`. Cleanup and teardown emit none of these edges.
- **(breaking)** Scenario lighting is authored content: a new `Light` scenario
  object (`Directional` and `Point`) replaces the engine's hardcoded top-down
  key light, which is deleted. A scenario that authors no light now renders
  black - every shipped scene, mod scene, example and the editor sandbox spawns
  its own three-point key/rim/fill rig. Third-party mods must add a light to
  each scenario.

- Shakedown Run is dressed: the planetoid moves in to ~760u of the spawn (mass
  27 000, keeping the crate beat gravity-free) and a 78-rock slalom belt bends
  around it - five near `BELT_KNOTS` plus a far parallax ring, every knot
  clear of the beat pockets.
- `ScatterRegion::Ring` gains an optional `center`, so a belt can circle a body
  that does not sit at the world origin. Omitted, it is the origin as before.
- `ScatterObjects` gains an optional `min_separation`: scattered bodies are kept
  that far apart from every body scattered so far in the scenario, across
  sibling scatters, instead of being sampled on top of each other.

### Modding & Mod Portal

- **(breaking)** Recurring `OnOrbit` and per-ship `orbit_hold_secs` are replaced
  by one-shot `OnOrbitStart`, `OnOrbitStable`, `OnOrbitUnstable`, and
  `OnOrbitEnd` edges; continuous holds compose these events with scenario timers.
- **(breaking)** Recurring `OnTravelLock` / `OnCombatLock` and per-player
  `lock_refire_secs` are replaced by one-shot start/end lock lifecycle edges.
- Scenarios gain keyed, pause-frozen timers: `TimerStart`, `TimerCancel`, and a
  one-shot `OnTimerEnd` event with a timer-key filter.
- The scenario lint warns on a zero delay written as a value:
  `auto_advance_secs: Some(0.0)` and a `NextScenario` `delay: Some(0.0)` now
  report - both mean "no delay", which is spelled `None`. Third-party content
  authoring `0.0` starts emitting warnings.
- Portal republish so installed copies actually get the relit scenarios and the
  mass-authored gravity wells: Gauntlet Run `1.3.0 -> 1.5.0`, The Ledger
  `1.14.0 -> 1.16.0`. The Mods screen offers an Update on a version-string
  mismatch, so without the bump an installed copy would keep its unlit content
  and render black, and its wells would silently lose their authored gravity.

### Interface & HUD

- Setting buttons commit on RELEASE over the button, not on mouse-down: press
  a wrong option, drag off and release, and nothing changes.
- Every button variant now has its own pressed face on BOTH skins. `Ghost` (the
  segmented Graphics-preset and UI-skin rows) and `Primary` had no press
  feedback at all; the hardware Exit/Danger face gains a sunk one.
- The phosphor slider track drops its 2px inset, so the lit edge lands on the
  value the click actually commits instead of ~3px off it.
- The block meter no longer rounds a near-full value to full or a near-empty
  one to empty: 98% reads as 98%, 2% reads as 2%.

### Internals & Tooling

- The dev CLIs are subcommands of the game binary: `cargo run content gen|lint`
  and `cargo run --features debug probe run|report` replace the standalone
  `-p nova_authoring --bin content` and `-p nova_probe_cli` bins, which are
  gone.
- Probe runs declared frame-time capture and native tracing automatically;
  `--fps` and `--profile` are removed, while slow `--samply` stays opt-in.
- All probe examples use the unified `NovaProbePlugin`; `probe run --correctness-only` keeps behavioral evidence while omitting frame-time and profiling passes, and is the CI windowed gate.
- **(breaking)** `ScenarioConfig` no longer derives `Default`; build one with
  `ScenarioConfig::new(id, name, cubemap)` plus struct-update syntax. A
  defaulted `cubemap` was never a valid scenario.
- `examples/sections/`: five ranges, one per section family, each walking a
  NAMED roster of invariants over several predicate-gated rounds, across as
  many scenes or rig layouts as its invariants need. `com_range` folds into
  `hull_section` and `torpedo_guidance` into `torpedo_section`, whose PN lead
  angle is now asserted rather than logged;
  `sections_assert_their_invariant_roster` pins all 27 invariant names so one
  cannot be deleted into a still-green run.
- `scripts/serve-web.sh`: one-command live web preview - site, game and mod
  portal on free 7XXX ports, proxied onto one origin, all watched.
- `scripts/serve-mods.sh`: builds and serves the mod portal, regenerating on
  every `webmods/` edit.
- Web dev server picks a free port in 7000-7999 instead of `:8090`, and proxies
  `/mods` alongside `/play`.
- `Trunk.toml`: the dev `[[proxy]]` moved to `TRUNK_SERVE_PROXY_BACKEND`; a
  config-file entry conflicts with the env one and panics Trunk.
- Dev shell: added `watchexec`.
- New `nova_autopilot` crate: the automation drivers (autopilot timeline,
  screenshot), the `capture` primitive and the run-completion protocol,
  depending on `bevy` alone. `nova_debug`, `nova_probe` and every example now
  drive it instead of the `bevy_common_systems` harness copy.
- Harness environment variables renamed `BCS_* -> NOVA_*`: `BCS_AUTOPILOT ->
  NOVA_AUTOPILOT`, `BCS_SHOT -> NOVA_SHOT`, `BCS_REEL -> NOVA_CAPTURE`, and
  `BCS_HARNESS_DEADLINE -> NOVA_AUTOPILOT_DEADLINE` (the deadline's stem moved
  too, so it is not a pure prefix swap). `NOVA_SHOT_DIR` was always spelled
  that way and is unchanged. Any scripted run pinned to the old names arms
  nothing and silently does a plain play-through. **(breaking)**
- One capture idiom: the screenshot reel is deleted
  (`ScreenshotReelPlugin`, `ReelBeat`, `nova_reel`, `reel_beat`, `ReelCamera`,
  `completion::REEL`). A capturing example is now an ordinary autopilot script
  whose steps call `nova_debug::harness::shoot`, so the act, the framing and the
  shot read in step order instead of the beat list being built away from the
  script that produced the state it framed. `screenshot_sections` and
  `screenshot_scene` were rewritten onto steps; `shoot`,
  `force_capture_resolution`, `hide_hud`, `freeze_bodies`, `pose_camera` (was
  `reel_pose_camera`) and the `scenario_camera_present` predicate are the shared
  pieces. `NOVA_CAPTURE` arms the capture path of a script that has one, so the
  same file is also the smoke run. `capture_window` stays as the primitive - and
  `widget_zoo` now shoots through it instead of resolving `NOVA_SHOT_DIR` a
  second time. **(breaking)**
- Every shot ACKS: `capture_window` records the path in a new `CaptureLog` once
  the PNG is on disk, and a shot step holds on `until(shot_written(name))`
  instead of a guessed frame count. That deletes the save-latency settle
  outright and collapses the per-example scene settles (90/6, 40/6, 30/2, 20/2)
  onto one `SETTLE_FRAMES`, the same on the capture and the smoke path - no
  example branches its step timing on `capturing()` any more. A shot that never
  lands is now an error exit naming the step (`SHOT_DEADLINE_SECS`) rather than
  a missing file. `menu_scenarios` and `widget_zoo` wait on the ack too.
- Dev wiki: "Automation harness" page for the `nova_autopilot` drivers.
- `nova_autopilot`: curated prelude, crate-level env contract table, and a
  `completion` doc example.
- `nova_autopilot` is predicate-driven: a script is a list of NAMED STEPS, each
  advancing when its predicate over the world holds (`elapsed`, `frames`,
  `state_is`, `resource_where`, `any_entity`, `and`, `not`), with per-step
  entry/per-frame actions, per-step deadlines and a loop point. A stalled beat
  error-exits naming the step instead of timing out generically. `hold(state,
  secs)` and `input(f)` survive as constructors over the step model;
  `self_completing` (replaced by `deadline`) and `loop_while_pending` (replaced
  by `loop_from` + `on_loop`) are removed. **(breaking)**
- `nova_autopilot::input`: synthesized keyboard and pointer gestures
  (`press_key`, `release_key`, `press_mouse`, `release_mouse`, `move_cursor`,
  `click_at`) that leave the world in the state a real device would - window
  `cursor_position`, a `CursorMoved` message and a fresh `just_pressed`.
- `nova_debug::harness`: Nova-typed predicates `scenario_variable_is`,
  `section_gone` and `player_ship_present`, so a script waits on what the game
  agreed happened rather than on a guessed duration. `hull_section`,
  `hud_range` and `player_path` are rewritten onto them, dropping their beat
  offsets, boolean stage trackers and per-example completion guards.
- **(breaking)** Probe coverage is a HANDSHAKE, not a table: every probe plugin
  declares its capability into `probe-contract.json` (`NOVA_PERF_CONTRACT`), and
  each check resolves against that declaration instead of against the example's
  category. A check whose capability was never wired reports
  `N/A (not declared)` rather than passing on absent evidence, and a run that
  declares nothing at all grades `UNPROBEABLE` instead of `OK`. The launch-side
  `CategoryPolicy { probed, frame_time }` table, the per-example `NOT_PROBED`
  opt-out, the `NOT_PROBED_CATEGORIES` list and the `checks.json` `fps_exempt` /
  `fps_skipped` manifest field are all gone with no shim - there is no
  launch-side way to exempt an example from a check any more.
- New `examples/systems/` category: code-built `ScenarioConfig` fixtures for
  the cross-cutting systems, reaching no shipped story content. `scenario`
  becomes `systems/scenario_grammar` and `playable` becomes
  `systems/player_path`, both deepened into repeated rounds gated on scenario
  variables; new `systems/outcomes` walks the whole outcome arc in one live run
  (die -> Defeat overlay -> Retry -> clean reload -> Victory + CHECKPOINT ->
  chained scenario). `cargo run --example scenario` / `playable` and
  `probe run scenario` / `playable` are gone - use the new names.
  **(breaking)**
- `nova_probe` invariants: a registered monotonic is one-way within a SCENARIO
  LIFE, not for the process - the memory is forgotten on `ScenarioLoaded`.
  A replaying example previously took a false `monotonic_regression` at every
  round boundary, because a reload overwrites its variables in place and never
  leaves the vanished-key gap the old reset waited for.
- `examples/ui/` rebuilt around real pointer input: four of the five runs
  (`widget_zoo`, `editor`, `menu_newgame`, `menu_scenarios`) now DRIVE the
  interface - hover, press, segmented select, check/toggle, slider drag,
  placement clicks on the ship - instead of triggering widget observers, and
  check the live tree after every rebuild. `hud_range` stays predicate-driven;
  its subject is where an indicator lands on screen, not what a pointer does to
  it. `menu_newgame` narrowed to the
  boot flow (`NOVA_MENU_PATH=editorplay` is gone - `editor` owns that
  sequence).
- `nova_autopilot::input`: `click_named` / `hover_named` / `ui_node_centre` /
  `ui_node_rect` resolve a click target by `Name`, so a layout move is
  survivable and only a rename breaks a run. `ui_node_rect` is the single home
  of the physical-to-logical pixel conversion, and it warns when two laid-out
  nodes share a name rather than pointing at an arbitrary one silently.
- `nova_debug::harness::REACHED_PLAYING`: the smoke sentinel is a const, named
  by its two emitters.
- **(breaking)** `tests/examples_smoke.rs` is deleted and CI's windowed smoke
  step becomes the probe correctness sweep (`probe run --all` under Xvfb), which
  asserts a superset of it. Its two drift guards moved to
  `crates/nova_probe/tests/catalog_drift.rs`.
- One camera-authority chain (`CameraAuthority { Solve, Additive, Override }`)
  declares who writes the camera `Transform` and in what order, folding bcs's
  chase, WASD and shake writers into it. The missing edges used to be filled by
  executor readiness, which is what made a scripted pose flicker between runs.
- Nova owns its health pool, damage typing and destruction pipeline
  (`nova_gameplay::integrity`) rather than importing them. Impact (ram) damage
  now routes through the typed path instead of bypassing it. No numbers move
  today - Kinetic is the 1.0 reference column on every section - but a ram is
  now subject to the same table every other weapon meets.
- One persistence store (`nova_assets::persist`) replaces the two hand-rolled
  copies behind the mod set and the settings menu. Storage locations are
  unchanged, so saved mods and settings survive the swap.
- `AppBuilder::with_main_menu` and `nova_ui`'s `debug` feature are deleted; the
  menu fronts the default app and nothing else. **(breaking)**

## [0.9.1] - 2026-08-02

### Web & Platform

- Web release builds compile after the pause-menu split stopped importing the native-only Exit handler on wasm.

## [0.9.0] - 2026-08-01

### Scenarios & Objectives

- Scenarios tab groups campaigns under collapsible `[-]`/`[+]` headers listing each campaign's ordered chapters - including hidden mid-story chapters - so any chapter is launchable directly for replay.
- Ships are now combat-dead when out of the fight: an armed ship that loses ALL working weapons AND thrusters is NEUTRALIZED - a distinct drifting-wreck state (AI switched off, not despawned) that fires a new `OnNeutralized` scenario event, so a beaten enemy counts as beaten without grinding its hull to zero and the player loses when they can no longer fight. Shipped enemy kill-objectives and player-defeat handlers accept neutralize as well as destroy.

### Modding & Mod Portal

- New `Campaign` content kind: a bundle declares an ordered campaign->scenario mapping (`Campaign((id, name, scenarios: [...]))`) loaded into `GameCampaigns`; the content lint flags a member scenario no bundle provides.
- The Ledger (1.12.0 -> 1.13.0): its six chapters group under a "The Ledger" collapsible campaign header; the reward finale (The Raid) is re-hidden from the flat picker and now replayed from the header, and chapter titles drop their redundant "The Ledger N:" prefixes.

### Interface & HUD

- Objectives now post as a **notification stack** at the top of the flight HUD: one amber chip per posting carrying the objective itself (not a count), arriving like a chat notification the moment the objective posts - popping on the posting frame, then breathing. Each chip is read like a notification - it leaves after a dwell or the instant you open NOVA OS - so idle cruise stays clean; the standing list is `objectives` in NOVA OS and the gold markers on the targets. Replaces the top-right status-bar objective hint (so the status bar is fps + version again) and the small gold posting flash; completions still ghost green.
- The flight HUD becomes CONTEXTUAL: idle cruise is near-empty and each element surfaces while its situation is live - ammo gauges only while the weapons are hot or a group runs low, the speed chip grown during an autopilot burn, the lock readout grown while the weapons are hot, the reticle pulsing while the trigger is down, the dock chip of the verb the ship is doing inverted and grown, a posted objective popping its chip as it posts and then breathing slowly, and a fresh comms card arriving grown. The `~` HUD cycle collapses from All/Minimal/None to **On / Cinematic** (breaking for anything that named the old levels): contextual visibility replaces the manual detail dial, and Cinematic still clears the screen.
- The flight HUD adopts the phosphor chip language and drops most of its on-screen text: the lower-left seven-row `[KEY] VERB` cluster is replaced by a bottom-centre **icon dock** of the flight verbs you can use right now, each showing the real keycap picture plus the verb word, inverted while that verb is what the ship is doing. A verb that would do nothing at this moment is off the dock entirely rather than shown greyed out, so the row tracks your actual options; a scenario spotlight still pulses a chip gold and can reveal one that has not lit up yet. The anchored ORBIT and GOTO cues and the objective stack's TAB affordance become keycaps too. Every keycap draws at the proportions its ART carries - each glyph is trimmed to the cap it actually paints and pinned by HEIGHT, so the wide Tab/Shift/Ctrl/Space caps read at a glance instead of being squeezed into a square. And every readout - speed, autopilot mode, destination, beacon, objective, lock, scenario readout, comms card - is now a bordered translucent chip in its meaning colour. Comms cards cap tighter, and a nearly-dry weapon group warns in amber.
- Scenarios picker holds its layout: the list pane keeps a fixed share of the screen instead of resizing with the selected scenario's details, and a campaign's chapters are indented under their `[-]` header.
- The menus and editor chrome adopt the NOVA OS visual language: the flat navy/cyan theme is retired in favour of the green-phosphor terminal palette and skin-aware widgets (buttons, segmented controls, panels, badges, rows). Settings gains an `Interface` section with a `UI skin` choice - Phosphor (the default CLI-terminal look) or Hardware (the light-3D casing) - persisted across restarts. The main menu grows a primary New Game / danger Exit emphasis, a glowing title and a version footer; the scenarios list now scrolls.
- Player-facing distances and speeds now read at 1 world unit = 10 m everywhere (metres below 1 km, kilometres above; speed and closing speed in `m/s`); the `u`/`u/s` unit retires from the HUD, the NOVA OS map and the wiki glossary. Display-only - physics, content and AI tuning are unchanged.
- Comms panel becomes a bottom-left stacked chat surface with optional authored speaker icons, timeout/dismiss controls and skip-to-next backlog control.
- Tab ship-computer drawer: one inset NOVA OS monitor opens on Tab (or the gamepad right-stick click), pauses the game and frees the cursor, with a PoC-matched green phosphor terminal screen, topbar, footer hints and CRT treatment.
- NOVA OS screen is now a real CRT: the terminal renders to an offscreen image displayed through one sampling shader, so the bright green glyphs bloom into a soft halo and the whole screen bows with barrel curvature (the crisp content curve a CSS overlay could only fake), with scanlines/grain/vignette folded into the same pass; opening blooms the raster on from a scan line and closing collapses it to a dying dot. Terminal scrolling and app clicks are preserved through the image via a forwarded pointer.
- NOVA OS terminal readability pass: bright high-contrast phosphor text on a near-black CRT (the overlay no longer films the text), an input box that reads as a dark strip above the screen, fish-style inline completion that continues on the same line, subtle square grain, Iosevka terminal font, live ship-name status, auto-scroll command output and an HTML-style welcome block that returns after `clear`.
- NOVA OS terminal prompt: Tab completes commands once the monitor is open, Esc closes it, and shell commands include `help`, `log`, `objectives`, read-only `ship`, `clear` and `exit` (which suspends the computer), with history, cursor editing, inline completion and `command not found` / `did you mean` suggestions.
- NOVA OS help/usage now reads like a real shell: `help` opens with a `Usage:` synopsis and a `Commands:` list, `<command> help` prints a capital `Usage:` line naming the real argument (`map goto <label>`) plus an aligned `Subcommands:` section, an unknown command points at `help`, and a wrong argument reports `command: reason` (naming a bad subcommand) then the command's usage.
- Flight objective surface: the always-on compact objectives panel is gone; a posting is a chip in the objective stack and the detailed objective output lives in the NOVA OS `objectives` command.
- NOVA OS command output prints the combined Flight Log, active objectives and live player-ship section status without restoring permanent drawer panes.
- Allegiance markers: a small filled triangle floats above every ship, coloured by side (green ally, red threat, grey neutral) so a mixed brawl reads friend-from-foe at a glance; your own ship shows none, and a ship provoked mid-fight flips its marker red.
- Startup now shows a phosphor NOVA OS loading screen (near-black CRT screen, green "NOVA OS" mark and a "LOADING" line with an indeterminate blinking-cursor + marching-dots animation) while the game's assets preload, instead of a blank native window; it hands off to the menu once loading finishes.

### Web & Platform

- The website adopts NOVA OS, so it stops advertising a look the game no longer has: the navy/cyan industrial-HUD palette is retired for green phosphor on a near-black field, and the site wears the game's PHOSPHOR skin - a page is a CRT surface, so headers, cards and sidebars are lit screen panels behind phosphor hairlines, buttons are flat bordered CLI elements that invert to solid phosphor when primary, keycaps are amber bracketed tokens, and code, tables and placeholders are screen recesses. Nothing is bevelled or moulded; the light-3D HARDWARE skin is the game's alternate look and is used nowhere on the site. Typography goes terminal-first: display, body and code all render in JetBrains Mono (Rajdhani and Inter are dropped), with the leading and measure retuned for a mono face. Site and game now mirror the SAME `:root` block (`web/design/nova_ui_rework_poc.html`), and `npm test` fails if the two drift or if hardware material creeps into the site.
- New `scripts/shoot-web-pages.sh`: builds and serves the site, then headless-captures the six page kinds at desktop and mobile widths so a styling change can actually be looked at. `npm run ci` now runs `npm test` as well.

### Fixes

- The combat lock no longer lets go mid-fight: FIRING now counts as combat activity for the 30 s idle decay, not just holding the weapons raised, so a player who locks a hostile and shoots it with the stance lowered keeps the lock however long the fight runs. The 30 s rule itself is unchanged but is now VISIBLE - over the last five seconds the red reticle dims and pulses faster, snapping back to full the instant anything counts as combat - and every automatic lock drop (idle decay, out of range, target gone, allegiance flip) now names its own cause.
- World-anchored nav chips (the objective marker and the beacon chip) now show a full background: their fill and border wrap the whole label instead of collapsing to a slab in its top-left corner, and the objective chip's diamond rides inside the pill.
- Clicks inside NOVA OS land where the picture is. The mouse was mapped onto the CRT's offscreen image through the barrel warp's inverse rather than the warp itself, and skipped the shader's overscan entirely, so the further from screen centre you clicked the further the click landed from the thing under the cursor - up to 27 px at the corners. Worst in the `map` app, whose contacts spread across the whole viewport, while the `ship` app's mid-screen sections were barely affected. Contact and section labels are also one unbroken target with their dot now, instead of sitting a few dead pixels clear of it.
- Tab drawer scrolling now clamps at the content bottom, so wheel-up responds immediately after reaching the end.
- Web build: NOVA OS text is visible. The UI font shipped with no `.meta` sidecar, so under `AssetMetaCheck::Always` the missing-meta fetch failed on the web and every NOVA OS glyph rendered invisible; the `nova_meta_gen` sidecar generator now registers the font loader and emits the sidecar (the font is the built-in-loader `.ttf` per the preload change above).

### Internals & Tooling

- `nova_probe` runs are profile-sandboxed: each native child run gets an empty, probe-owned profile under its run dir (`NOVA_MOD_CACHE_ROOT`, `XDG_DATA_HOME`, `XDG_CONFIG_HOME`), so a locally cached mod, saved enabled-mod set or saved settings can no longer decide a run's result; export any of the three to keep your own.
- The Iosevka Term terminal font is now credited in `credits/` under its SIL Open Font License 1.1 (copyright and full license text bundled with every build, as the OFL requires).
- Input-prompt key glyphs (JulioCacko's FREE Input Prompts, CC0) move into the game asset tree at `assets/input-prompts/keyboard/Alt/` and are credited in `credits/`; only the Alt style ships and the unused Dark/White styles are dropped.
- Static assets (UI font, shared HUD art, UI sound effects, textures and meshes) now preload through `bevy_asset_loader` collections and are load-gated before gameplay starts, instead of being fetched lazily at first use; scenario-authored `AssetRef`s and downloaded `mods://` bundles remain the dynamic exceptions. The UI font ships as the single Iosevka Term Regular face (`.ttf`, ~11 MB) extracted from the former 66 MB `.ttc`, so the bespoke `.ttc` font loader is retired for Bevy's built-in one (the web `.meta` generator follows suit) and the first NOVA OS open no longer triggers a 66 MB download on the web build.

## [0.8.1] - 2026-07-24

### Web & Platform

- Landing hero gains per-OS download buttons (Windows/macOS/Linux) that deep-link the matching release asset, with a fallback to the releases page.

## [0.8.0] - 2026-07-23

### Gameplay & Flight

- Unarmed AI ships are now non-combatants: no target, no chase, no fire - still targetable, so an escort/convoy must be defended.
- Gravity wells pull only PILOTED ships; an unpiloted bystander floats where it sits instead of being dragged in.
- Lifeline's convoy haulers now fly (unarmed non-combatant AI) instead of drifting off when shoved.

### Scenarios & Objectives

- Base-campaign pacing pass: an objective posts a beat AFTER its comms line, never on the same frame, with a breather between objectives (shared scenario-pacing toolbox).
- Shakedown Run opens slower: Capt. Halloran briefs you over comms (~40s) before objective 1; objective texts shrank to the bare goal.
- New scenario **Lifeline** (ch3 pt1): screen a two-hauler convoy through three raider waves to a live `RELIEF mm:ss` countdown - the first shipped ally content.
- New scenario **Final Tally** (ch3 pt2, finale): survey + orbital picket + capital-escort fight in the base chain's first combat gravity well; the campaign now ends properly. New Game arc is five scenarios.
- Broadside found its voice: story moved to the comms panel, objectives shrank to imperative goals, Victory banners track the Ceres Queen's fate.
- The **Asteroid Field** sandbox is back in the Scenarios picker (it had been wrongly hidden as unreachable).
- New `HudReadout` action: show a scenario variable on the HUD (`Number`/`Integer`/`Time`), an Instrument-tier readout, pause- and teardown-safe.
- New `SetAllegiance` action: flip a ship's allegiance mid-scenario - the neutral-until-provoked primitive.
- New reserved `player_speed` variable: the player's live speed, engine-written and read-only, to gate beats on how fast you fly.

### Modding & Mod Portal

- Gauntlet Run is now a TIME-TRIAL (1.3.0): a live `mm:ss.s` clock and a clean-run bonus, built on `HudReadout`.
- The Ledger grew 1.5.0 -> 1.12.0: a campaign-wide pacing pass, a ch3 stealth run (neutral pickets, warn-then-trip overspeed), a forking ch4 finale, and a fifth reward chapter (The Raid); re-published to the portal.

### Interface & HUD

- The Scenarios picker groups the base storyline as a campaign: scenarios declare an optional `campaign` (name + order); mods can group their own chapters the same way.

### Fixes

- Destroying a sectioned ship no longer crashes: the damage-tint tolerates a section the explosion destroyed the same frame.
- The mouse cursor is hidden while flying, dev builds too; the `--features debug` layer now boots OFF and F11 toggles all of it as one.

### Internals & Tooling

- A pre-commit `cargo fmt --check` hook makes rustfmt drift impossible to LAND (arm once with `scripts/setup-hooks.sh`).
- `content lint` is the single content command: `audit` folded in, plus a flight-rig input-overlap check; `lint --target <mod> --report <path>` writes a per-mod Markdown/HTML report pinpointing each finding.
- The `nova_perf` crate became `nova_probe`, the run-harness crate (bin names, `NOVA_PERF_*` env vars and output formats unchanged).
- One front door: `probe run <example>` runs a clean pass + optional `--profile` trace and `--samply` flamegraph, then a self-contained `report.html` + machine-readable `checks.json` with an OK/WARN/FAIL/NO_DATA verdict.
- New run-timeline recorder (`NOVA_PERF_TIMELINE`): ordered JSONL of state transitions, fired events, variable changes and beat markers, flushed per entry.
- Continuous invariant checks (`NOVA_PERF_INVARIANTS`): per-frame health/velocity/variable/entity-count assertions that land on the timeline.
- Probe timelines emit outcome markers at their assertion sites, so the timeline shows the FEATURE working, not just the process surviving.
- The whole example fleet is probe-evaluable: every cataloged example carries the recorder, invariants and frame-time capture (all inert without probe's env).
- Fleet runs: `probe run <list|category|--all>` run sequentially with an aggregated `index.html`/`index.json` status index and worst-row verdict.
- `--fps` is a dedicated capture-only pass; narrative one-shots are fps-EXEMPT via `Cargo.toml` metadata; the capture window and completion deadline are sized to the request.
- `--baseline` works across a group: regression-check a whole fleet against a prior `--all` run (missing captures are SKIPPED, not errors).
- Harness runs are silent: any bcs harness env zeroes the audio output (`NOVA_MUTE` overrides); the volume setting and menu are untouched.
- Examples moved into purpose directories (`sections|gameplay|ui|screenshots|perf/`) with the root `Cargo.toml` `[[example]]` catalog as the single source of truth; per-category smoke tests.
- Profiled pass renders a top-N costliest-systems table (Perfetto chrome trace via a new `trace` feature); `--samply` adds a flamegraph on a dedicated profile.
- Frame-time rows record run metadata (backend, GPU, resolution, preset, git SHA, host) and build profile; pre-metadata files still load.
- Run-harness hardened for unattended use: a cleaned run dir + `probe-run.json` manifest, so a hung or crashed run still produces a FAILing report.
- bevy_common_systems 0.19.1 -> 0.19.2: `GameEvent` gained public `name()`/`info()` read accessors for external observers.
- `docs/` wipes to only its README at a release (guard-enforced).

## [0.7.0] - 2026-07-18

### Gameplay & Flight

- RCS fine docking thrusters: hold SHIFT to translate along the ship's own axes under a ~2 u/s cap (no rotation) - a per-ship controller verb, withheld in the mainline.
- GOTO/STOP autopilot now settle their arrival on RCS, easing to a stop instead of pulsing on the spot.

### Combat & Weapons

- Turrets fire and aim every muzzle they have: twin-barrel PDCs throw two streams at once, sharing one magazine.
- Hostiles respect line of fire: hold fire (and torpedoes) when hard cover blocks the shot; intangible volumes and PDC-vs-torpedo exempt.
- Weapons auto-reload: a dry magazine refills on its own, so magazine size is a fire-pacing beat and finite ammo is on by default.
- Locking is no longer instant: a lock-on dwell (longer at range) you must hold the target through before it commits.

### Ships & Sections

- Enemy ships show battle damage: a destroyed hostile section reads burnt-black (no intermediate red on the enemy hull).

### Scenarios & Objectives

- Scenarios can declare a win/lose: new `Outcome` action shows a VICTORY/DEFEAT overlay with real buttons and freezes the sim behind it.
- **Broadside**, chapter two of the base storyline: a distress call, a two-corvette ambush, then screen a gunship's torpedoes - chained from Shakedown's victory.
- Broadside plays as two scenarios (checkpoint after the ambush; the gunship retries itself); invulnerable boulders anchor the big fights.
- Campaign + Ledger storytelling pass: fights announce themselves on an arrival grace, comms beats spaced on the scenario clock, closing lines moved into the banner; beat-sheet convention documented and lint-enforced.
- Scenario transitions got a middle gear: `NextScenario` gained a `delay` and Outcome banners an `auto_advance_secs`; lint warns on pairing an Outcome with a non-lingering switch.
- The Ledger chapter two rebuilt around fair fights (single lane, mook guns, a cover field) and split into two checkpointed acts.
- The Ledger finale's Auditor traded its top-tier turret for the light mook gun on both ending branches.
- Every combat scenario now flies real auto-reloading ammo (unlimited stays an authoring/testing option).
- Ships in scenario RON can author an `allegiance` (e.g. `Some(Neutral)` for bystanders).
- The Asteroid Field sandbox joined the outcome frame (Victory on arrival, Defeat + Retry on death).
- The ship-less Demo Scenario was removed; the example mod's arena is the worked hand-authored RON example now.

### Fixes

- The Rust Tally's side mounts now roll to seat against the spine and face outboard (firing arcs fixed, port/starboard ids un-swapped).
- The Ledger Auditor's torpedo bay comes out of the hull (flush on the bow); a new overlap lint catches the whole class at build time.
- Skybox `.meta` sidecars are honored for every asset (base and mod, shipped or downloaded), fixing the WebGL2 oversized-upload crash.
- Retry no longer blanks the objectives panel (the objectives HUD resets on teardown).
- A ship that loses its last section off the damage path no longer lingers as a 0-HP ghost (structural death backstop).
- Scenario `OnUpdate` handlers now freeze while the game is paused.

### Modding & Mod Portal

- Turret mounts are an arbitrary joint tree **(breaking)**: `root` + recursive `children` (offset/axis/render_mesh/muzzle) replace the fixed yaw/pitch/barrel fields; `fire_rate` is per-muzzle; stock content migrated, the lint checks the tree.
- Scenarios can tell time **(breaking)**: reserved `scenario_elapsed` variable (live unpaused seconds) readable from any filter; writing it is a lint error.
- `orbit_hold_secs` / `lock_refire_secs` are author-tunable per ship (default 5s; a non-positive value is a lint error).
- Mods can ship their own art: a `resources` list + `self://` paths that resolve against the mod's own folder.
- Asset paths always carry a scheme **(breaking)**: `self://`, `dep://<id>/`, `dep://base/`; the base game is a normal mod now and a bare path is an error.
- One `example` mod is the single copy-me tutorial (section overlay, new section, arena, shipped art, comms, Outcomes, menu backdrop).
- New `menu_backdrop` flag: the menu picks a flagged scenario at random; New Game moved to the base bundle's `new_game_scenario`.
- Two new menu backdrops: Waystation Traffic and Scrapyard Drift.
- New `StoryMessage` action: speaker-attributed dialog in a HUD comms panel.
- Broken mod content fails loud: reference errors give a FAILED TO START report; the backdrop rotation skips them.
- `content -- lint` lints every scenario for reference bugs (CI-enforced); `--target <mod>` scopes to one mod.
- **The Ledger**, the first campaign mod on the portal: a four-chapter salvage arc with a two-ending finale. Install from Mods > Explore.
- Every world sound is content-owned: weapon/controller/thruster/crate sound fields and per-target `impact_sound`/`destroy_sound` are authorable refs; base sounds ship under `assets/base/sounds/`.

### Interface & HUD

- Comms chatter is readable: lines queue in arrival order, fade in, hold a dwell and yield; newly posted objectives flash gold.
- The diegetic ammo gauge shows reload as a dimmer sweep filling the pips back up.
- The pause menu gained a Retry button (offered only while a scenario is live).
- The Settings menu is real: a master volume slider, a Low/Medium/High graphics preset, and a keybind reference, remembered across restarts.

### Performance

- Low/Medium presets skip heavy visuals for weak machines: Low is spawn-less (no particle bursts), Medium keeps particles.
- Low also renders the world at ~70% internal resolution and upscales (HUD stays crisp); Medium/High untouched.

### Internals & Tooling

- Content tools are one CLI: `content -- <gen|lint|audit>` replaces the three former binaries.
- The content lint checks mount seating (a base face must seat against an occupied neighbor cell), at build time and in the in-game gate.
- The two 5s scenario-event windows now measure against `scenario_elapsed` (one place freezes and resets them on pause/teardown/retry).
- The balance audit learned acknowledgments (`balance_acks.ron`): intended drama acked with a reason + task; errors can't be acked, a stale ack fails CI.
- `content -- audit` grades each combat scenario's fairness sheet and fails CI on an armed hostile that spawns inside its own range of the player.
- Firing no longer spams an avian "no mass or inertia" warning (bullets carry an explicit angular inertia).
- Debug builds bind F12 to a screenshot saved to Downloads.

## [0.6.0] - 2026-07-16

### Scenarios & Objectives

- Scenarios picker on the main menu: two-pane overlay listing every base and mod-added scenario, with a details pane and Play button; scenarios gained optional `thumbnail` and `hidden` fields.
- New `SetSkybox` action swaps the skybox cubemap mid-scenario, deferred until the image loads so a bad path leaves the sky unchanged.

### Modding & Mod Portal

- Mod dependencies resolve end to end: installs auto-pull missing deps, enabling a mod auto-enables its transitive deps, disabling a still-depended-on mod is refused, and merge order is dependency-respecting topological (ids only, no version constraints yet).
- Static mod portal now publishes on every deploy: validated, sha256-hashed bundles under `/mods/<id>/<version>/` with a generated `catalog.json` (first mod: Gauntlet Run).
- Local mod cache foundation: a `mods://` asset source, a RON installed index, and downloaded bundles that load and merge through the same pipeline as shipped ones.
- Portal client fetches `catalog.json` and installs/uninstalls over the wire on native and web, staged installs verified against size + sha256 and committed only once every file checks out.
- Explore online tab is real: browse the portal in-game and install/update/uninstall on native and web, with per-file progress, offline catalog fallback, and enabled-state-preserving updates.
- Mods menu is a two-pane Factorio-style screen: `Installed` | `Explore online` tabs over the scrollable list with per-row enable checkboxes and a details panel from bundle meta.
- Mods menu hides dev/tooling mods: `hidden: true` entries stay installed and code-enableable by id but drop out of the player list.
- **(breaking)** Mod metadata moved into each `*.bundle.ron` as a `meta` block; `assets/mods.catalog.ron` slimmed to a thin pointer list (catalog and out-of-tree bundle format break).

### Web & Platform

- Particle effects (muzzle flash, projectile trail, torpedo launch/detonation bursts) now render in the web build after moving from WebGL2 to WebGPU.
- Browsers without WebGPU get a clear "WebGPU required" message instead of a black canvas, plus a heads-up under the landing page's "Play in browser" button.

### Performance

- Modding event dispatch is indexed by event name (upstreamed to bevy-common-systems rev 4c81117): 17-24% faster under bursts of 500-5000 handlers, neutral at the realistic one-event-per-frame rate.
- Added a criterion benchmark for the scenario-dispatch hot path (`cargo bench -p nova_scenario --bench scenario_dispatch`); the measure-first gate is documented in `tasks/20260714-083331/modding-perf-report.md`.
- Sibling filter-key-interning and condition-eval-compile optimizations were measured and deferred: at realistic event rates their per-handler cost is noise, kept as documented insurance.

### Fixes

- Scenarios picker no longer crashes the renderer on a non-2D thumbnail: such thumbnails are skipped with a warning, and images mount only once loaded.
- Local mod-portal web testing no longer needs a cross-origin `?portal=` override: `scripts/preview-web.sh` serves the portal same-origin as the game, matching production.

### Internals & Tooling

- Screenshot Reel capture set no longer ships in the game assets: its scenario moved into the example that films it, so players and the web build stop downloading a capture tool.

## [0.5.2] - 2026-07-14

### Gameplay & Flight

- Enemies can be authored to ARRIVE instead of appearing: a scenario ship with an `engage_delay` grace flies its patrol and holds its fire for those seconds before going hot - shoot it and the courtesy ends instantly and permanently, and its point-defense never stops watching for torpedoes. Paired with a warning comms beat, a spawn now reads as an approach you were told about, not an ambush from nowhere.
- Gamepad bindings rounded out: ORBIT -> South, scenario-advance confirm -> DPadDown, and HUD cycle / pause / back-to-editor gained buttons.

### Web & Platform

- The site grew a full wiki (gameplay, ship sections, keybinds, world and meta pages), two new devlogs, and a tutorial trimmed to first-scenario onboarding.

### Fixes

- Thruster hum now attenuates with distance per ship, so another ship's or torpedo's burn no longer plays at full volume from anywhere (your own ship stays exempt).
- Scenario teardown no longer logs an "Entity despawned" command error when a ship despawns with autopilot engaged: the telemetry-cleanup observer uses the fallible remove variant, pinned by regression tests.
- Debug inspector's window-camera placement fix moved upstream into bevy-common-systems (rev 4a743b2), deleting nova's local workaround.

### Internals & Tooling

- CI's example smoke suite is BLOCKING again: the GitHub-runner-only taffy panic is gone, so the 12-example suite gates every push.
- Examples are a testable curriculum: twelve numbered examples from controller PD through the boot flow, each self-driving under BCS_AUTOPILOT with behavior assertions and a completion backstop, all on the CI smoke list.
- Example smoke now fails on ANY logged command error, closing the gap where handled remove/despawn warnings (the stale-entity teardown race) sailed past the panic gate.
- Weapon test ranges fire again: the weapons safety had silently disarmed both, so their scripts raise the stance first, with new assertions pinning the fire -> hit chain.

## [0.5.1] - 2026-07-13

### Fixes

- Web build no longer quits with a fatal render validation error on New Game / editor Play: the target inset dropped its `view_formats` override (unsupported on WebGL2) for a plain sRGB target.
- Skybox cubemap's `.meta` loader settings now actually apply in the shipped app: `AssetMetaCheck::Never` had silently ignored them, resurrecting the oversized-upload race on GPUs with a 16384 texture limit.

## [0.5.0] - 2026-07-13

### Gameplay & Flight

- Diegetic flight readouts replace the bottom-left status text: speed and engaged-mode chips beside the velocity sphere, an ORBIT radius spoke, and a nav-cyan tint while the autopilot flies.
- Keybind cluster is contextual: rows appear only while their verb can act; scenario-emphasized keys show early, pulsing gold.

### Combat & Weapons

- Deliberate radar locking replaces all passive targeting: hold CTRL to sweep and live-lock what you look at, stance picking the slot (lowered NAV vs raised combat) and a tap to clear; locks stick until the target dies, leaves range, or goes cold.
- Typed damage (Kinetic / AP / EMP / Explosive) against per-section resistance tables; each turret carries a loaded-ammo slot setting its rounds' type, with a color-coded ammo readout.
- Lock language is slot-colored: RED bracket = combat lock, WHITE = nav lock; relation tint and reticle pips retired, turrets hold the combat lock even during manual aim.
- Turret rounds curve through gravity wells, like ships and torpedoes.
- PDC retuned to point defense: per-hit damage 20 -> 4, so the stream chips targets down over a visible burst instead of one-shotting them.

### Ships & Sections

- Editor: placed sections show their bound key as a chip and can be clicked to rebind (keys or mouse buttons); the build panel scrolls.
- Editor play-test ship is now a passive target instead of an AI combatant.

### Scenarios & Objectives

- Shakedown Run: New Game starts a ~12-beat tutorial (burn, freelook, stop, salvage, GOTO, gravity coast, ORBIT, radar lock, live-fire rehearsal, scavenger fight); each beat teaches one gesture and completes the instant it lands.
- Objective conveyance: gold marker chip with live distance to the current target, salvage-crate glow and brackets, keybind emphasis pulses, completion chime and posting blip.
- Scenario primitives: nav beacons and salvage crates (authorable radar signatures), despawn-by-id, and new events/actions (OnOrbit, OnTravelLock/OnCombatLock, SetSpeedCap, objective markers, hint emphasis).

### Interface & HUD

- Target viewfinder: a corner inset renders a live magnified 3D view of the combat lock via a second camera; red armed frame while hot, NO-SIGNAL for non-scopeable bodies, and a ~2 s freeze-frame kill cam on death, the fine-locked section glowing in both views.
- Main menu (live ambient scene: an AI ship flying a thruster-driven orbit) and ESC pause menu; the game now boots into the menu (new `nova_menu` crate).
- HUD visibility levels: grave/tilde cycles ALL -> MINIMAL -> NONE.

### Web & Platform

- Web landing site (`web/`: TypeScript + Webpack + Tailwind); the Pages deploy serves it at the root with the game under `/play/`.

### Fixes

- Debug builds no longer crash when a scenario advances (the smoke assertion now covers only the boot load), and teardown no longer warns about despawned entities.
- Asteroid kills emit OnDestroyed under the scenario id so scripts can react (this had soft-locked the derelict beat).
- Target inset zooms combat bodies only (ships, torpedoes, asteroids), not beacons, and frames section-less bodies by collider bounds.
- Debug inspector stays on a window camera instead of leaking into render-to-texture cameras.
- HUD apparent-size elements (reticles, brackets) measure real colliders, not invisible trigger volumes.
- Turret bullets are sensor projectiles: same damage, no physical shove, expended on first contact.
- Many Shakedown playtest fixes: park points inside beacon triggers, orbit-hold completion, scavenger spawn timing and a 150u combat leash, invulnerable planetoids, speed-cap teaching, objective text wrapping and sound pacing, readable gold pulses.
- F1 back-to-editor is sandbox-only; the debug ammo readout no longer lingers when debug mode is off.

## [0.4.1] - 2026-07-10

### Internals & Tooling

- release-flow: install the `x86_64-apple-darwin` std for the pinned nightly, fixing the macOS universal build.
- CI: one `--features debug` feature set across clippy/tests/examples (one Bevy build), cache saved on failure, windowed examples smoke as a separate non-blocking step.

## [0.4.0] - 2026-07-10

### Gameplay & Flight

- Flight-assist overhaul: assisted velocity-hold mode (WASDQE nudges, X brake latch, soft cap), Z direct Newtonian mode, RCS budget, FA/speed readout.
- Flight computer balances thrust through the live center of mass: differential throttle nulls burn torque, off-axis thrusters recruited for counter-torque.
- Mass-legible handling: turn rate derived from the torque budget and live inertia (stripped ships snap, heavy builds lumber); max_torque 100 -> 40.
- Chase camera: smoothing on all gameplay modes plus a burn push-back lean.

### Combat & Weapons

- Torpedo guidance: proportional navigation, angular lock-on aim-assist, arming gate, blast-radius visual, launch particle burst.
- Player targeting arc: signature auto-acquisition (550 m), focus dwell, per-section fine-lock with aim-snap and cycling, HUD lock markers and focus meter.
- Turret auto-aim with true intercept lead, fed by fine-locked section, else live structure, else camera ray.
- Faction/relation model (hostile/neutral/own) drives acquisition, projectile allegiance, and reticle tint.
- AI combat wave: behavior state machine (Idle/Patrol/Engage/Evade/Retreat), fire discipline, point-defense priority on inbound torpedoes, standoff orbit/strafe, patrol routes, threat-memory evasion, enveloped torpedo launches.
- Player lock range 2 km -> 20 km (AI sensor scan unchanged).

### Interface & HUD

- HUD substrate: screen-projected indicators (entity/point anchors, apparent-size sizing, clamp-to-edge arrows), turret lead pips, and a locked-target readout (range, closing speed, health).

### Audio & Visuals

- First audio: placeholder SFX (explosions, impacts, turret fire, torpedo launch, throttle-tracking thruster loop) with distance attenuation and throttling.
- Combat juice: trauma-model camera shake and expanding hit/impact flash rings, distance-attenuated and throttled.
- The SFX/juice listener is an explicit marker on the gameplay camera, not "any Camera3d".

### Fixes

- Skybox cubemap reinterpreted into a 6-layer array at load time; the raw 24576 px image exceeded smaller GPUs' texture limit and killed the app.
- Blast damage reaches every body overlapping the blast, not just one.
- Ships, asteroids and torpedoes interpolate between physics ticks (no camera twitch); the chase camera anchors on the live center of mass.
- Projectiles no longer collide with their shooter; shot-down torpedoes die whole and blast-free; destroyed asteroids no longer leave rigid-body husks.
- Section overkill is absorbed instead of propagated; a disabled controller stops torquing the hull.
- Bullet lead solved in the shooter's frame so a moving shooter's rounds land; the AI helm writes slewed absolute rotation commands.
- One hit plays one cue: audio/juice observers ignore propagation re-entry.
- Misc: editor preview controller made inert, turret resting position, one-frame origin snap on camera-mode switch.

### Internals & Tooling

- Example test ranges (`06_torpedo_range`, `08_turret_range`, `10_gameplay`, `11_com_range`) with live tuning sliders, FPS/version overlay, and a headless autopilot + screenshot smoke harness that asserts scenario init.
- CI workflow: fmt, clippy, and the workspace test suite (windowed examples under Xvfb) on every PR and push to master.
- Integrity, health, blast and mesh-slicer systems consumed from `bevy-common-systems` instead of in-tree copies; torpedo section split into its own module.

## [0.3.1] - 2026-07-07

### Audio & Visuals

- Post-processing camera component.

### Internals & Tooling

- **(breaking)** Upgrade to Bevy 0.19 (avian3d 0.7, bevy_rand 0.15, bevy_enhanced_input 0.26, rand 0.10); anything built against nova, including from-source mods, must move to the matching versions.
- `bevy_common_systems` externalized as a git dependency; vendored copy removed.
- Added `AGENTS.md` and a `docs/` folder (architecture, scenario system, sections, development, migration notes).

## [0.3.0] - 2025-11-29

### Combat & Weapons

- Torpedo bay section and blast damage.

### Ships & Sections

- Per-section health system.

### Scenarios & Objectives

- OnEnter/OnExit zone events.

### Audio & Visuals

- Improved directional and thruster shaders.

## [0.2.1] - 2025-11-15

### Modding & Mod Portal

- Modding documentation and examples; event system refactor.

## [0.2.0] - 2025-11-08

### Modding & Mod Portal

- Game events and a queue system; scenario and modding capabilities.

### Scenarios & Objectives

- Asteroids with procedural mesh and dynamic destruction.

## [0.1.0] - 2025-10-21

### Ships & Sections

- Basic spaceship sections.

### Scenarios & Objectives

- Editor and simulation scenes.

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.9.1...HEAD
[0.9.1]: https://github.com/alexjercan/nova-protocol/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/alexjercan/nova-protocol/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/alexjercan/nova-protocol/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/alexjercan/nova-protocol/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/alexjercan/nova-protocol/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/alexjercan/nova-protocol/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/alexjercan/nova-protocol/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/alexjercan/nova-protocol/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/alexjercan/nova-protocol/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/alexjercan/nova-protocol/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/alexjercan/nova-protocol/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/alexjercan/nova-protocol/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/alexjercan/nova-protocol/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/alexjercan/nova-protocol/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/alexjercan/nova-protocol/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alexjercan/nova-protocol/releases/tag/v0.1.0
