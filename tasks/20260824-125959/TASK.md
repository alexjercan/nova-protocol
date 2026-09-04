# Reboot Nova Protocol with block ships and move the Kenney fleet into The Ledger

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.13.0,content,scenario

Detailed implementation plan: [PLAN_01.md](PLAN_01.md).

Promoted 2026-08-31 from ideation into v0.13.0. Rescoped by owner direction:
the built-in campaign gets a new story and a block-ship identity, while The
Ledger becomes the deliberately different Kenney-GLB showcase mod.

## Decisions

- Scrap the current Nova Protocol plot and dialogue. Keep useful maps,
  encounters, and proven scenario structures only where they serve the new
  campaign.
- The owner supplies the final dialogue and exact story events after the
  campaign beat sheet is agreed.
- Built-in campaign and menu-backdrop ships use fixed, hand-authored section
  layouts in the visual language proven by `wfc_arena`. They are reproducible
  authored ships, not runtime WFC output.
- The campaign starts with a new small, simple ship and advances the player's
  ship technology across its scenarios.
- Move the Kenney Racer, CargoA, and CargoB parts, assembled ships, and GLB
  resources from base into The Ledger. The mod demonstrates custom GLB assets,
  custom section prototypes, a custom fleet, and a visual identity unlike the
  base game.
- The Ledger may later contribute its own `menu_backdrop` scenarios. A major
  Ledger story rewrite is not part of the first pass.
- Gauntlet stays independent of The Ledger. Replace its Kenney Racer with a
  base block racer instead of adding a campaign-mod dependency.
- Use the v0.12.0 scenario primitives such as `Sequence` and `once`, not new
  hand-rolled handler cascades.
- Define the complete mainline story, actors, faction motives, revelations, and
  ending before authoring later chapters or final dialogue.
- Keep research summaries, story diagnosis, and the revisable campaign bible
  under this task. Record techniques and citations, never copied source text.
- Publish story work from the start under the website's `/story/` section.
  Its index lists campaigns like the News archive. Each campaign opens a
  full-screen digital-HUD comic reader. Each responsive page fits completely
  inside the available screen. Wheel, touch, arrow/Page keys, previous/next
  controls, and contents links all move by one snapped page rather than
  scrolling through part of a page. The reader also carries a page counter, a
  contents drawer, and one Exit Campaign link back to the archive. The ordinary site header and footer stay outside the reader.
  Keep story separate from the reference
  wiki, then link accepted lore into campaign-specific wiki pages as the full
  storyline stabilizes.
- Treat HTML and SVG as web presentation, not base-mod runtime assets. A future
  in-game encyclopedia should use campaign-owned lore data rather than parse the
  website.
- Give each comic one `web/src/comics/<path>/comic.json`; its directory path is
  its catalog key and URL, with no global registry or duplicated slug. The JSON
  owns metadata, chapters, page order, and TypeScript source modules.
- Author pages through a typed function DSL (`comicPage`, `panel`, `speech`,
  `svg`, `svgText`, and related helpers). Page modules never name CSS classes.
  A shared renderer turns their safe node trees into DOM, and a campaign-agnostic
  `ComicPlayer` owns fitting, playback, controls, contents, deep links, and
  resize alignment.

## Story design workspace

- [`research/STORY_BIBLE.md`](research/STORY_BIBLE.md): accepted facts, cast,
  setting, and unresolved decisions.
- [`research/CAMPAIGN_OUTLINE.md`](research/CAMPAIGN_OUTLINE.md): the complete
  chapter spine as it develops.
- [`research/SCENE_DIAGNOSIS.md`](research/SCENE_DIAGNOSIS.md): what the two
  implemented chapters currently deliver and where they are weak.
- [`research/INSPIRATION.md`](research/INSPIRATION.md): focused source reviews,
  techniques, and originality boundaries.

## Phase 1: campaign design

- Choose the player's starting role and the campaign's central premise.
- Write a short beat sheet for a small number of scenarios. Introduce the core
  flight, navigation, ship-management, combat, and environment mechanics across
  the whole arc instead of front-loading them.
- Define a fixed player-ship progression from the small starter craft to the
  campaign's most advanced ship. Define only the supporting ships each beat
  needs.
- Reuse the opening Shakedown Run map unless the new beat sheet exposes a
  conflict; its route, debris field, pickups, autopilot leg, and gravity well
  are a good starting stage.
- Treat all current story names, factions, comms lines, and chapter links as
  replaceable.

### Campaign foundation

- The player is an engineer aboard a very large Earth industrial carrier.
  Earth fields clean civilian, industrial, and military fleets. Runaways and
  belt pirates usually survive in small, poorly equipped salvage ships.
- A pirate group has stolen a major Earth military warship. It destroys the
  carrier as a demonstration against a large, recognizable, defenseless Earth
  target. The player is only in the wrong place at the wrong time.
- Chapter one does not explain the attackers, their motive, or how they stole
  the ship. There is no special cargo or central mystery: the attack should be
  sudden, disproportionate, and initially incomprehensible.
- The campaign can reveal the political context later, but its first story is
  the engineer surviving the loss of home and crew.

### Scenario 1: first shift

Use the current Shakedown Run stage as the starting layout, then adapt it to
this spine:

1. The engineer leaves the stationary industrial carrier in a small, unarmed
   maintenance cutter. The carrier is much larger than the `wfc_arena` ships
   and remains visible behind the player as home and return destination.
2. Fly manually to the first beacon. Introduce steering, speed control, and
   freelook.
3. Follow a weak signal into the asteroid cluster. Introduce RCS as the natural
   way to make careful movements while recovering several crates. Improve the
   pickup and interaction mechanics only where play exposes a need.
4. Lock the inspected planetoid, use GOTO to approach it, then enter ORBIT.
5. Receive the order to return and use GOTO toward the carrier.
6. A stolen military warship emerges from behind a second, much larger
   planetoid. The second body must fully conceal the attacker during the
   inspection orbit and must not disturb the tutorial body's intended gravity
   or navigation geometry.
7. The warship aligns on the stationary carrier. Its multiple railguns fire
   deliberate shots past the cutter and disable the carrier. Bastion torpedoes
   cross the scene and complete the destruction. The industrial carrier has no
   meaningful defenses.
8. The cutter stays unnoticed or visually masked against the planetoid. The
   military ship leaves.
9. End the scenario when normal comms collapse into silence and the first weak
   distress signal appears from the wreck. Chapter two owns returning to the
   carrier and finding what can be saved; the player gains no weapon in chapter
   one.

The attack is a real simulated set piece, not damage hidden off screen. Keep
its exact geometry authored and reproducible. The large carrier may be built as
several scenario set-piece assemblies if one enormous physical ship is too
costly or fragile, but it must read as one industrial vessel and break visibly.

### Scenario 2: wreck search and escape

Reuse the first-shift map shortly after the attack. Replace the intact carrier
with reproducible derelict hull assemblies, lit debris, and collectible evidence.
The cutter searches several carrier fragments for survivors and evidence before
a five-ship cleanup group enters from behind the large planetoid. Two searchers
are unarmed salvage craft, two carry one simple PDC each, and the leader carries
one PDC plus one standard Serpent torpedo bay. They are accomplices of the stolen
warship rather than unrelated opportunists.

The unarmed cutter must escape through the asteroid field to an extraction or
communications point. Detection should change the escape into a pursuit rather
than immediately fail the scenario. The intended stealth model uses authored AI
profiles composed into runtime components: perception accumulates and forgets
contacts, thrust and weapon emissions affect visibility, large solid bodies can
occlude sensors, and searchers can investigate, search last-known positions, and
share contacts. Default AI configuration is inert; explicit constructors and
named profiles provide fighter and cleanup-searcher behavior.

AI controllers eventually contain only an `AIProfileSource`. Map assignments
use the shared ship-order actions also accepted by `None` ships; AI-specific
constraints such as leashes remain independent components with paired set/clear
actions. Shared orders reject Player ships, preserve a durable directive across
allowed AI interruption, and distinguish completion, interruption, resumption,
cancellation, and failure. Patrol orders execute one loop; scenarios reissue
them when they need a continuous patrol.

### Scripted ship-control prerequisites

Keep autonomous AI and scenario direction separate:

- A fully scripted ship uses `SpaceshipController::None`. Allegiance remains
  independent, so the chapter-one warship is visibly Enemy but never acquires
  or attacks the player unless the scenario explicitly orders a weapon.
  Scripted helm actions accept only `None` ships and refuse normal AI or Player
  controllers rather than stealing control from their owner.
- Add `non_combatant: bool` to `AIControllerConfig`. It applies the existing
  `AINonCombatant` behavior even to an armed ship, allowing autonomous patrol,
  orbit, avoidance, and station-keeping without target acquisition or
  offensive fire. Do not emulate this with a long grace or tiny engage range.
- Do not add a `ScriptedAI` controller. Scenario actions own scripted actors;
  the existing AI controller owns autonomous behavior.
- `MoveShipTo { order, ship, position, arrival_standoff }` engages the real
  `GotoPos` autopilot on a `None` ship. It must fly with real thrust and physics,
  not set the transform. The authored standoff is needed for precise staging
  because the normal 500 m arrival distance is too broad for a cinematic move.
- `ForceAlign { order, ship, look_at, tolerance_degrees }` physically turns the
  whole ship toward an authored world position without translating. It reports
  completion once the aim is within the authored angular tolerance, then holds
  that facing until another helm order replaces it, so several spinal weapons
  can charge while the carrier remains under the bore.
- Add `StopShip { order, ship }` for a real STOP maneuver and
  `ClearShipOrder { ship }` to release scripted helm authority and permit drift.
- Move, align, and stop are one mutually exclusive HELM-order family. Starting
  any one cancels the current helm order before installing itself. Clearing or
  replacing an order does not report the canceled order as complete. Weapon
  orders are independent and can run while an alignment order holds.
- Every completable helm action carries an authored `order` key. On successful
  arrival, alignment, or stop it emits one `OnShipOrderComplete` event carrying
  the order key, ship id, and order kind. Add a dedicated `ShipOrder` event
  filter that can match the key, ship, and kind; do not overload entity ids or
  timer keys with order identity. Scenario handlers use that exact event instead
  of guessed delays, so repeated commands for one ship cannot satisfy the wrong
  continuation. Define completion against the existing autopilot arrival and
  stop thresholds, and alignment against both angular tolerance and settled
  angular velocity.
- Destruction, neutralization, or explicit cancellation retires a pending order
  without a completion event. A replacement starts cleanly in the same update.

Resolved 2026-09-03 by owner direction, after reading the safety chain:

- Every scripted action, helm and weapon alike, refuses anything but a `None`
  controller. The guard keys on the driver markers (`PlayerSpaceshipMarker` /
  `AISpaceshipMarker`), never on `Allegiance`, so the Enemy warship still
  qualifies. Runtime logs an error and skips; content lint makes it an authoring
  error, because the addressed id is already proven to be spawned by this
  scenario and its `SpaceshipController` is in the same config. The guard covers
  only the scripted-authority actions: `SetSpeedCap`, `SetControllerVerb`,
  `SetAllegiance`, `RefillAmmo` and `SetInfiniteAmmo` are retunes the shakedown
  legitimately aims at the player.
- No weapons-hot action. `WeaponsHot` is derived every frame as
  `raised || locked` on any ship carrying a `CombatLock`, and ships without the
  component are unmanaged and fire freely. A `None` ship has no `CombatLock`, so
  scripted fire already works with no safety involved. A scenario write would
  also not stick: both inputs to the derivation are themselves rewritten every
  frame, from held inputs for the player and from the combat mirror for AI, so
  forcing the safety needs an override the whole chain respects. Nothing in
  chapter one needs that.
- Do not guard on the presence of controller SECTIONS. The scripted helm
  requires them: the autopilot disengages without a live `PDController`, and
  `ForceAlign` reads the same PD for its turn rate. The guard is on the authored
  driver enum, not on the hull.
- Derive the alignment settle from the authored tolerance - the aim is settled
  when the ship would not leave tolerance within a second - rather than adding a
  global angular threshold to `FlightSettings`.
- A `ShipOrder` filter with no field set matches every completion. That is an
  authoring choice, not a mistake: it is neither an error nor a warning.

Replace broad all-mount scripted fire with section-addressed actions:

- `ForceRailgunFire { ship, section }` triggers one named railgun section. It
  does not choose or steer toward a target. The railgun keeps its normal charge,
  ammo, reload, projectile, recoil, damage, sound, and visual behavior.
- Replace `ForceTorpedoLaunch { id, target }`, which fires every bay, with
  `ForceTorpedoFire { ship, section, target }`, which orders one named bay at
  one named target and keeps the normal bay gates and guided target lock.
- Validate ship, section, and target references in content lint. Refuse a
  section of the wrong class instead of silently firing another mount.
- Migrate every built-in backdrop and example-mod use of the old torpedo action,
  then update creator documentation. Treat the serialized action rename and
  field change as a format break if the old form shipped in the release
  baseline.
- Cover helm-order replacement and cancellation, keyed completion events,
  scripted movement, held physical alignment, explicit armed non-combatants,
  exact section selection, missing references, wrong section classes, normal
  weapon gates, and one-shot retirement in tests.

## First Shift playtest revision

Approved 2026-09-04 after the first full cinematic playtest:

- Track arrival-standoff consolidation separately in task `20260904-084733`.
  That work starts with code review and design evidence. Do not add another
  independent standoff knob before the current global, per-ship, per-order,
  target-radius, and gravity-parking paths are reconciled.
- Track total-vector speed-limit correctness separately in task
  `20260904-084734`. Manual flight and RCS must not exceed their stated limits
  through diagonal motion, while braking from an overspeed state remains
  available.
- Keep Cutter's manual-flight limiter at 150 m/s for the whole chapter. Do not
  remove it silently after the launch lesson. GOTO keeps its independent flight
  planning.
- Add a player-autopilot completion event. A GOTO target remains alive until the
  maneuver has genuinely settled; only then does the scenario remove its marker
  and despawn it. Use the same completion seam to make the STOP lesson wait for
  a real stop instead of an elapsed dialogue delay. Manual RCS marks can still
  complete on their precise physical intersections.
- Add explicit `SuspendPlayerControl` and `ResumePlayerControl` scenario
  actions. Cinematic camera actions remain camera-only. Suspension disables
  gameplay flight, mouse steering, stance, and weapon input, clears held intent,
  and leaves pause/menu input available. First Shift pairs suspension with every
  active cinematic pose and restores input with camera authority.
- Stationary, gravity-safe cinematic staging is a content-author responsibility,
  not a global engine restriction. First Shift starts its destruction cinematic
  only after Cutter has settled at its clear outer hold. Add scenario proof for
  that contract.
- Keep possible scripted Player helm orders as follow-up architecture:
  `MoveShipTo`, `StopShip`, and related shared orders may accept a Player ship
  only while player control is suspended, and scripted authority must retire
  before control resumes. Do not let an order compete with live player input.
- Replace the close front Cutter shot that hides the event. Frame Meridian from
  a wider Cutter-relative rear or side-quarter view, keeping Cutter in the
  foreground and the destruction readable. Return the normal chase camera
  before the distress aftermath.
- Rework the RCS exercise into four or five precise beacons in open space. Spawn
  the route together during a safe Cutter-relative briefing pose so the player
  can identify it before control returns. Keep only the current step as the
  active objective, preserve tight intersections, and despawn each completed
  mark.
- Keep salvage collection precise. Tune its pickup volume toward visible
  contact with the crate rather than enlarging it, while retaining reliable
  sensor overlap and avoiding an accidental destructive collision lesson.
- Move every prescribed GOTO route clear of asteroid clutter. Add conservative
  segment-clearance tests using authored worst-case rock geometry, Cutter size,
  and a safety margin. This is scenario correctness; generic pathfinding and
  obstacle avoidance remain separate work.
- Route mandatory work behind the small planetoid relative to Meridian. The
  crew believes it has slipped out of sight and talks the captain into doing a
  donut. Complete a real orbit lap instead of ending after the current 13-second
  timer, then have Meridian catch the crew as Cutter comes back into view and
  order it to finish the paid asteroid work before departure. Until player lock
  and radio occlusion exist, describe this as visual cover and do not teach
  sensor degradation as a shipped mechanic.
- Make comms approximately half the viewport wide with substantially larger
  body text, a distinct speaker header, larger spacing and icons, and readable
  objective notifications and world-marker labels. Do not impose a mainline
  two-card engine limit; instead author First Shift so only one or two cards are
  normally active.
- Do not make every story message cinematic. Important conversations belong
  mainly at the opening and ending and use explicit safe conversation holds.
  Mid-scenario lines are short and sparse. Orbit dialogue remains ordinary
  comms because Cutter is moving in a gravity well. The destruction remains a
  full set piece with player input suspended.

Implementation order:

1. Add reliable player maneuver completion and fix GOTO target retirement and
   the STOP lesson.
2. Add player-control suspension and apply it to the cinematic.
3. Replace the close Cutter shot and verify the destruction composition.
4. Rebuild the RCS route with four or five visible precise marks and retain the
   150 m/s manual limiter.
5. Validate and repair every direct GOTO corridor and tighten crate contact.
6. Replace the orbit timer with a real lap and stage the visual-cover joke.
7. Enlarge comms, objectives, and world markers, then revise dialogue placement.
8. Run a full human pacing and handling pass before final dialogue is authored.

## Phase 2: built-in block fleet

- Design and pose candidate ships in focused examples first. Review their
  silhouettes, section layouts, scale, style, and movement there before
  promoting any design into generated base content or campaign scenarios.
- Promote a maintainable authored subset of the WFC section vocabulary into
  base content only after the example designs are accepted.
- Add reproducible catalog ships for the campaign roles and technology tiers.
- Recast every built-in campaign actor and all four base menu backdrops onto
  that fleet.
- Recast Gauntlet onto a suitable base block racer while preserving its
  base-only dependency.

## Phase 3: The Ledger owns the Kenney fleet

- Move the 21 Kenney-derived part GLBs and their semantic section prototypes
  out of base and into The Ledger.
- Make Ledger hulls resolve only through resources and content that the mod
  declares, apart from normal engine/base section contracts it intentionally
  depends on.
- Keep the current Ledger campaign playable and preserve its outcomes while
  changing asset ownership.
- Update its README so the shipped mod is the primary custom-GLB and
  visually-distinct-campaign example.
- Consider one Ledger-owned menu backdrop after the fleet migration works; do
  not let it block the ownership move.

## Done when

- The replacement Nova Protocol beat sheet and ship progression are recorded
  here and approved before final dialogue implementation.
- Mutually exclusive, cancelable helm actions can move, physically align, and
  stop a `None`-controlled ship, with exact keyed completion events for
  sequencing. Independent weapon actions can fire one named railgun and one
  named torpedo bay at one target. AI ships can be explicitly non-combatant
  even when armed. Shipped content no longer uses the broad all-bays
  `ForceTorpedoLaunch` action.
- Scenario 1 plays through the engineer's first shift, the visible destruction
  of the industrial carrier, and the first distress signal, then hands the
  wreck-return beat to chapter two.
- The built-in campaign is a complete, playable arc using reproducible block
  ships whose capabilities advance across the campaign.
- Built-in menu backdrops use the same block-fleet identity.
- Gauntlet uses a base block racer and still depends only on base.
- Base no longer owns the Kenney Racer/CargoA/CargoB semantic fleet or its 21
  part GLBs; The Ledger owns and declares them.
- The Ledger remains playable and documents how its custom GLB fleet is built.
- Generated base content, affected target lints, scenario tests, portal output,
  and user-facing campaign/mod documentation are updated and verified.

## Progress

### 2026-09-03: first-shift fleet bench

Added `examples/playable/first_shift_ships.rs`: three fixed, hand-authored
candidate structures in one free-WASD-camera lineup. The enlarged industrial
maintenance cutter is one hull layer thinner and has two basic drives. The
industrial carrier has massive midship cargo shoulders with two vertical
cutter berths, an empty port berth, and a second cutter joined to the starboard
berth by two docking lugs. The docked cutter stands proud of the hull with its
drives exposed and facing aft. The carrier also has an elongated refinery
spine, extended cargo shoulders, a deep dorsal superstructure and ventral keel,
a broad transom, and two 5x5 capital drives. The armoured stolen warship has a
longer, narrower fighting spine and engine transom, two 3x3 vector drives, two
prow-embedded spinal railguns, three flush torpedo bays down each flank, six
dorsal PDCs, and four ventral PDCs. These are visual candidates, not promoted
base content.

Added `examples/playable/first_shift_map.rs` as a separate spatial bench. It
places the accepted cutter, carrier, and warship designs at their story marks.
The cutter starts directly off the carrier's port side. A nearby first beacon
leads into a broad plate-shaped slalom of 40 small asteroids between the carrier
and both planetoids. The former banana's narrow tail is removed and its bowl is
filled to provide several cutter routes. The carrier and cutter move farther
from the field; the large planetoid and hidden warship remain to the right,
leaving a higher attack approach to the carrier clear. The cutter can collect
three crates near the large body through
the tight field that the carrier cannot safely enter. The 20 wider-stage rocks
and both planetoids retain their positions. Detailed objective
markers explain each landmark. `--pilot warship` (the default) and `--pilot
cutter` make either design the real player ship; `--pilot camera` restores the
accelerated free camera and keys 1-5. The warship has ten distributed controller sections for a faster manual turn,
and binds its ten PDCs to LMB, both railguns to R, and all six capital Breaker
torpedo bays to F. The Breakers use the experimental `heavy_torpedo_section`:
700 m/s, 0.22-radian weave, 450 m blast radius, 2,000 damage, 5,000 projectile
health, and a six-round rack that rearms one round after 10 idle seconds. This
bay is deliberately overpowered balancing and scenario kit, not normal play.
The map example inlines a warship-only railgun override: 500 damage, 200x
prototype pierce power (360,000), and a 30 m rake radius instead of 10 m.
The carrier has ten distributed controller sections for later scripted helm
work. Each mode posts its own simple
playtest objectives. This text is review scaffolding, not final campaign
wording or dialogue.

The same fleet bench now includes five light-hulled, salvage-clad searcher
candidates in a second row. The unarmed needle skiff and fork tug have distinct
recovery silhouettes. The armed picket is a low, balanced hull with its kinetic
PDC mounted directly on the forward nose face. The asymmetric claw puts its PDC
on top of the hanging grapple arm and adds one basic drive directly behind that
arm to balance the two small drives on the opposite structure. The heavier
cleanup leader carries one dorsal kinetic PDC, one flank-mounted standard
Serpent bay, and a centered 3x3 vector drive on a matching transom. All five use fixed block layouts
and real section prototypes; they are visual candidates for scenario 2.

Added `examples/playable/second_shift_map.rs` as the chapter-two spatial bench.
The player cutter starts outside the planetoid-side edge of the shared slalom and points
into twenty-eight fixed carrier-wreck assemblies distributed beside 27 of its
rocks and at the carrier's exact former position. Both planetoids,
all 40 plate rocks, and all twenty ambient rocks are shared directly with the
scenario-one map. Three evidence
beacons label candidate search sites. The five accepted cleanup ships stand at
their intended entry formation behind the large planetoid, while quiet-route
and extraction beacons show possible escape geometry. Marker text describes
these candidate beats, but the example deliberately adds no HUD objectives,
events, AI, or mission progression yet. Camera mode has five fixed review views.
`cargo check --example second_shift_map --features debug` passes inside
`nix develop`. Both map examples compile after the shared plate-layout change.
A rendered scenario-one camera smoke passed runtime content lint, loaded all 77
objects, and remained live until the bounded timeout.

Proof: `cargo check --example first_shift_ships --features debug` passes inside
`nix develop`; a 30-second rendered smoke reached `Playing`, passed the example's
real content-lint gate, clad all three ships, and remained live until the bounded
smoke timeout. `cargo check --example first_shift_map --features debug` passes;
its rendered warship and cutter smokes each loaded 51 fixed objects, posted the
role-specific objectives, attached all ten explanatory markers, clad all three
ships, transferred the camera to the selected player ship, and remained live
until the bounded timeout.

### 2026-09-03: scripted ship control

Implemented the Phase 1 prerequisites recorded above. `nova_events` gained
`ShipOrderKind` and `OnShipOrderComplete`; `nova_ship` gained
`flight::scripted` (the `ScriptedHelmOrder` family, the alignment drive and
`SuspendedArrivalStandoff`) and `railgun_section::scripted`; `nova_scenario`
gained `MoveShipTo`, `ForceAlign`, `StopShip`, `ClearShipOrder`,
`ForceRailgunFire` and `ForceTorpedoFire`, a `ShipOrder` filter, the completion
tracker, and `non_combatant` on `AIControllerConfig`. `ForceTorpedoLaunch` is
gone. Content lint now refuses a scripted action on a driven ship, an empty
order key, a `ShipOrder` filter no action can satisfy, and a section that is
missing or of the wrong class. The two shipped backdrops that launched
torpedoes and the creator docs were migrated.

Proof: `cargo fmt --all` and `cargo check --workspace --all-targets` clean;
`content -- lint` reports 0 errors, 0 warnings, 0 findings; the new tests pass
(2 in `nova_events`, 5 in `nova_ship`, 27 in `nova_scenario` actions/objects/
trackers, 35 in `lint::scenario`).

### 2026-09-03: block fleet and menu backdrops

Added `base_content::ships::block`: four hand-authored cube ships on the same
cell grid the `first_shift` bench uses - `block_cutter` (industrial, unarmed,
the accepted 26-cell workboat), `block_hauler` (industrial, unarmed, cargo
shoulders on a 3x3 transom drive), `block_gunship` (armoured, six PDC mounts
covering both hemispheres, one vector drive) and `block_raider` (salvage,
asymmetric outrigger and scrap boom, four bell drives, two bolted-on guns).
They are 70-95 m against the Kenney fleet's 41-48 m and about a third of the
bench warship, which is the "cutter-sized, with guns" scale the owner asked
for. Each wears a derived skin, so `block_ship` is the first catalog builder
to set `skin` and `style`.

All four base menu backdrops now fly them: the waystation's two freighters and
the weave's runner through `backdrop_orbiter`, the gauntlet's stand, and both
duellists (armoured gunship against salvage raider - the fleet's look argument
in one shot). No base backdrop names a Kenney hull, turret or section id any
more. The campaign scenarios still do; that is Phase 2's remaining half.

The gauntlet's magazine had to be re-derived, not carried over. It is per
TURRET, not per ship: splitting the old 800 rounds six ways was tried first and
every bearing mount ran dry three seconds before impact, so the first torpedo
through killed the ship in half a minute with nothing shot down. At 600 the
stand swats four torpedoes, runs dry, and falls to the fifth at ~46 s.

Proof: `cargo check -p nova_authoring --all-targets` clean; the new
`base_content::ships::block` tests and the existing
`every_shipped_ship_has_one_connected_mate_graph` pass (all four derive one
connected mate graph); `content -- gen` regenerated and `content -- lint`
reports 0 errors, 0 warnings, 0 findings. Live under Xvfb with
`NOVA_MENU_BACKDROP` pinned per backdrop: every block ship spawns, clads (54-93
plates in its own style) and FLIES - the weave cutter tracked its loop across
three frames and the waystation's two haulers across four, the gauntlet gunship
held station and its point defense killed four inbound torpedoes, and the duel
ran to the rival's defeat, the finisher's allegiance flip and its siege launch.

### 2026-09-03: the duel arena, and the trigger volume that could not report an exit

The duel backdrop had a framing defect the block recast made obvious: the
winner does not stop when the loser runs. The AI leash is not the answer -
`beyond_leash` is overridden while a ship is `recently_damaged`, which is
exactly the state a running fight holds both hulls in - so the survivor chases
its target off the edge of the shot and stands there trading fire with nothing
the player can see. Observed at 42 s of a live cycle: the gunship pinned to the
left frame edge, tracers going off-camera.

So the duel now fights inside a bounded arena. `duel_arena` is an 1,800 m
`CreateScenarioArea` shell on the frame's center - past the widest frame
half-width at the fight's depth (~1,410 m at 16:9, ~1,060 m at 4:3), inside the
dressing ring's 2,200 m outer edge, twice the patrol triangle's ~870 m reach. A
duelist that crosses it FORFEITS: it is set Neutral, which is invisible to
`update_ai_target` (it re-picks every frame and keeps only hostile candidates),
so the hull still in frame drops its lock, stops taking fire, and the leash
finally walks it home. The rival's forfeit arms the same finisher beat its
defeat would have; the victor's skips act two and runs straight to the hand-off.
A `duel_decided` latch guards both paths and the defeat path, because otherwise
a neutralized wreck coasting out of the arena would re-arm the finisher clock
mid-flight and put a second siege torpedo in the air.

The first cut did nothing at all, and the reason was an engine defect, not the
authoring. `ScenarioAreaPlugin` refcounted overlapping collider pairs per (area,
body), and a ship SHEDS colliders while it fights - a destroyed section
despawns, and its skin plates go with it - for which avian fires no
`CollisionEnd`. The counter could therefore only climb. Measured over one menu
duel: 270 starts against 149 ends for the gunship, a tally stuck at 121, and no
`OnExit` for either duellist in 100 s. Occupancy is now the SET of overlapping
colliders, and `forget_collider_occupancy` drops a dead collider from every set
it is in, so the surviving sections still drive the set to empty. Every trigger
volume in the game gains this: the shakedown coast ring had the same hole.

Proof: `a_compound_body_that_loses_a_collider_can_still_leave` is a genuine
regression - it fails with the new observer unregistered and passes with it -
and the three older area tests still pass. `content -- gen` regenerated,
`content -- lint` reports 0 errors, 0 warnings, 0 findings. Live under Xvfb at
the shipped 1,800 m: the forfeit fires at 33 s (`SetAllegiance: 'duel_rival' ->
Neutral`), `duel_decided` is written exactly once, the finisher launches 4 s
later, and the siege torpedo's kill lands just right of frame center at +23 s -
where the old cycle had the survivor pinned to the edge. A 2,500 m probe was
tried and rejected: it forfeits 5 s later for 700 m more off-frame chase.

### 2026-09-03: one helm-order family, shared by the scenario and the AI

The scripted helm actions landed as a `None`-only feature, which forced every
directed ship to be an inert hull. Chapter one wants the other thing: an
ordinary patrol craft that thinks for itself, is told where to be for one beat,
and goes back to thinking. So the family is now SHARED. `MoveShipTo`,
`ForceAlign`, `StopShip`, the new `PatrolShip` and `OrbitShip`, and
`ClearShipOrder` accept a `None`- or an AI-controller ship and refuse only the
player's - a player's stick drops any autopilot on the next frame, so that one
still cannot work. Forced fire stays `None`-only: a bot picks its own targets
and rewrites the same weapon seams every frame. No duplicate `MoveAITo` family.

Two writers on one helm is the whole problem, and the answer is a marker, not
an ordering rule. `ShipOrderHelmAuthority` sits on the ship while an order
runs, and all three AI flight writers (`update_passive_flight`,
`update_maneuver_flight`, `update_avoidance`) carry `Without<...>` for it.
Perception and weapons are untouched, so an ordered ship still looks around and
still shoots. "Missions outrank constraints" then falls out for free: a leash
crossing changes the behavior state, and the state has nowhere to write.

The order is a durable directive, not an installed maneuver.
`ShipHelmOrder { key, directive }` holds `Move`, `Align`, `Stop`,
`Patrol { waypoints, leg }` or `Orbit { well }` - the orbit by AUTHORED ID,
re-resolved every tick, because an interrupted order can outlive the entity
lookup that started it. `ShipOrderEngaged` marks the execution installed, so a
resume re-engages from the leg it was on instead of from the top, and the
driver cannot re-engage a maneuver it just watched finish.

Completion had one genuine ambiguity: the autopilot self-removes both when it
ARRIVES and when it loses the capability to continue (no live controller, no
live thrusters, no stable band). Same signal, opposite meanings. The driver
resolves it by checking the capability at the moment the autopilot lets go -
still able to turn and burn means arrival, otherwise the order failed. That is
what `OnShipOrderFailed` is for, and it is what keeps a `Sequence` gate from
waiting forever on a wreck.

`ShipOrderReports` is the layering seam. `nova_ship` decides WHAT happened and
queues it; `nova_scenario`'s tracker drains the queue and names it in the
authored vocabulary as one of five events. `nova_ship` never learns the event
types, and two outcomes landing in one tick keep their order.

Patrol is ONE loop, ending where it started: `n` waypoints is `n + 1` legs, one
point is out-and-home, and an empty route is a lint error rather than an order
that completes instantly. A standing patrol is still the AI's own `patrol`
field. Orbit completes on `AutopilotPhase::Hold` - the established-ring signal
the ORBIT telemetry already had - and then keeps holding, the way `ForceAlign`
holds its bearing. Move, stop and patrol release the helm as they report;
align and orbit keep it. That is `holds_after_completion`, and it is what lets
an AI ship go straight back to its routine after an errand.

Interruption is a ship property, not an order property, so one authored
`MoveShipTo` means the same thing everywhere.
`AIControllerConfig::order_interruption` inserts `AIOrderInterruption`
(`OnHostileContact` / `OnDamage`); absent means never, and a never-interrupted
ship carries no component at all.

Two deliberate deviations from the handoff. The AI constraints ship as three
actions with an optional payload (`SetAILeash`, `SetAIEngageRange`,
`SetAIPointDefenseRange`, omitted payload clears) rather than six paired
Set/Clear actions - `SetSpeedCap { cap: Option<..> }` already set that
convention here, and six actions would have been the only pairs in the
vocabulary. And there is no destruction-triggered `OnShipOrderFailed`: scenario
teardown despawns every scoped entity, so a `Despawn` observer would fire a
burst of spurious failures at the end of every run, and `OnDestroyed` /
`OnDefeated` already cover the hull. Capability loss on a LIVING ship is
covered, which is the case a gate can actually be stuck on.

Proof: `cargo test -p nova_ship --lib flight::order` - 10 pass, including the
patrol loop-closing count, the holds-the-helm split, an interrupted order
resuming from its own directive, a move that loses its engines failing instead
of reporting an arrival, and an orbit around a missing well failing.
`cargo test -p nova_scenario --lib` - 272 pass, including one event per outcome
under its own key, a player ship refusing every ship action, an AI ship taking
a helm order but not a forced shot, the empty-patrol lint error, and
`order_interruption` mapping to its component at spawn.
`cargo check --workspace --all-targets` clean.

### 2026-09-03: the mainline is First Shift and Second Shift

The old campaign is gone, not deprecated: `shakedown_run`, `broadside`,
`broadside_gunship`, `lifeline` and `final_tally`, their builders, their
thumbnails, their generated RON and the three integration tests that only
covered them. `nova_protocol` now has two members, both picker-visible, and
`base.bundle.ron` starts New Game on `first_shift`.

Both chapters share one `stage` module: the two planetoids, the 40-rock plate,
the 20 ambient rocks and the beacon/rock/planetoid helpers, lifted metre for
metre from the accepted `first_shift_map` bench. A chapter adds only its own
marks to it, so chapter two IS chapter one's belt rather than a copy of it that
can drift.

First Shift follows the approved spine. Four verbs are withheld at spawn as
`DisableVerb` modifications on the cutter's bridge and handed back one per
beat: RCS at the work mark, radar with the survey order, GOTO on the first
lock, ORBIT at the ring. The attack is a real set piece flown by name. Each
step hangs off the PREVIOUS one's completion event - `MoveShipTo` reports, then
`ForceAlign` on the carrier to two degrees reports, then the salvo sequence
runs - so it stages identically at any frame rate. The six siege bays fire 1.2 s
apart, which puts the whole rack in the air at +6 s against a ~9.5 s crossing:
the last one launches before the first one lands.

Second Shift is a search and a run. Each of the five searchers flies its own
`PatrolShip` lane under its own order key, with a matching `OnShipOrderComplete`
handler that sends it round again, because one patrol is one loop. Detection is
`order_interruption: OnHostileContact` plus a deliberately short 900 m
`engage_range`: a searcher that acquires the cutter breaks its own lane, which
fires `OnShipOrderInterrupted` filtered on `Patrol`, and the escalation widens
the three ARMED hulls to 6 km with `SetAIEngageRange` and releases their lanes
with `ClearShipOrder`. The two unarmed hulls keep sweeping - an unarmed AI ship
never acquires, so it can neither see the cutter nor raise the interruption.
Being seen re-posts the escape objective and changes the closing line; it never
declares an outcome. This is NOT the accumulating-perception `AIProfileSource`
model in the design above, which stays future work; it is the same stealth read
built out of shipped parts.

Four content bugs the pins and the live runs found, all fixed:

- `cleanup_picket`'s second sweep mark was a copy of a rock's own centre. A
  sweep lane is flown by the real autopilot with no avoidance, so the hull would
  have ground against that rock for the rest of the chapter. Every sweep mark is
  now pinned clear of both planetoids AND every plate rock, worst-case mesh
  radius plus a hull pad.
- The engineering evidence sat 76 m inside a rock's worst-case surface, where
  its pickup volume could never be entered.
- The player spawned 673 m from the approach mark, inside its 700 m trigger
  volume: the arrival handler fired on frame one, completed an objective that
  had not posted yet, and left the real post orphaned. The mark moved out to
  2.4 km, and `no_scenario_starts_the_player_inside_one_of_its_own_trigger_volumes`
  now walks every OnStart-spawned beacon area, crate area and `CreateScenarioArea`
  in every mainline chapter. It was verified to FAIL on the old position.
- The salvage objective told the player `[X]` for thrusters. `X` is STOP; RCS
  is Shift plus mouse.

Documentation is on the new campaign: the wiki's first flight walks First Shift
beat by beat and no longer teaches a live-fire beat the unarmed cutter cannot
fly, the scenario list carries both chapters, and the glossary, gravity-well
figures, the two-well widget and the create/dev references name the shipped
bodies instead of the deleted ones. The Kenney cast is documented as the catalog
The Ledger flies rather than as the campaign's own ships.

Proof: `cargo test -p nova_authoring --lib` - 93 pass, including 7 first-shift
pins (beat chaining, the salvo firing every gun exactly once, the rack away
before the first arrival, both warship marks clear of the large planetoid, the
approach ring containing every park point and orbit, crates clear of rocks,
every withheld verb granted back), 6 second-shift pins (every sweep relapped,
detection arming only the armed hulls without declaring an outcome, the leader
entering outside its own 10 km launch envelope, no sweep mark inside anything
solid, evidence in open space, 28 distinct fragments) and the 5 cross-chapter
pacing pins. `cargo test -p nova_assets --test example_scenario --test
mod_cache_install --test neutralized_ships` and `-p nova_authoring --test
campaign_membership` pass. `content lint`: 0 errors, 0 warnings, 0 findings.
`cargo check --workspace --all-targets` clean. `npm run format:check`, `lint`
and `test` clean in `web/`. Live under Xvfb: both chapters boot, the opening
conversation runs and the first objective posts after it, and a throwaway
scenario spawning all 11 promoted block hulls at once clads every one of them
with no panic.

### 2026-09-04: First Shift playtest revision, step 3

Reframed the destruction's final Cutter-relative shot as a 500 m
rear-quarter view. Cutter now sits at a controlled edge while Meridian stays
central and the ordnance enters from the opposite edge. The shot aims at the
fixed carrier berth rather than the carrier entity, so carrier despawn cannot
make the camera fall back to its Cutter anchor and swing away from the kill.
The warship is 109.7 degrees from Meridian at the lens and cannot enter the
72.7-degree horizontal frame.

The camera review is now reproducible in
`examples/playable/first_shift_attack.rs`. Its CLI accepts a death-shot offset,
resolution, capture switch and label, runs the real three-ship salvo on the
authored marks, and writes default captures under `target/shots/`. It and the
spatial map bench assemble the same complete belt from
`examples/playable/shared/first_shift_stage.rs`, including both planetoids, the
40-rock salvage plate and all 20 ambient rocks. The scene also mirrors
mainline's three-point light rig, three salvo camera poses, cumulative cut
cadence and player-control suspension, so its review follows mainline rather
than the earlier abbreviated sequence. Its harness can also record the two
railgun hits and their damage tail as `first-shift-railgun-hits.webm`, with the
recording window keyed to scenario variables inside that same attack sequence.
Both spinal railguns now fire in the same action step in mainline and the scene:
the second hit no longer lands after the first hit's debris has hidden its
impact. The former inter-shot time remains as quiet room before dialogue, so
the later camera and story cadence does not move. Rendered reviews at 1280x720
and 1920x1080 accepted the shipped pose.

The route audit also found TRANSIT 2 inside the inspection planetoid's gravity
well: it was 2.72 km from the centre against a 3.29 km sphere of influence, so
GOTO could not settle and its real completion event could never fire. The mark
now stands 4.01 km out with 721 m gravity clearance. Both changed corridors
clear conservative stage geometry by 851 m and 1,186 m with Cutter's hull
included. Structural pins now require every arrival mark to stand outside both
wells and the kill shot to use a persistent point aim.

Proof: 16 focused First Shift structural tests pass; both probe catalog gates
pass after restoring the three `system_turn_limit` outcome slugs omitted on
master; `content gen` and `content lint` report 0 errors, 0 warnings and 0
findings; `git diff --check` is clean; rendered 1280x720 and 1920x1080 captures
were inspected from the attack scene. The shared-map revision compiles both
examples, loads the complete scene in a rendered run, clads all three ships,
and writes all four 1280x720 beat captures. The armed loop walk exits cleanly
and encodes 154 deterministic frames as a 5.1-second, 1280x720, 30 fps VP9 webm;
four sampled frames confirm that both lance hits and the physical hull damage
are visible.

### 2026-09-04: First Shift playtest revision, step 4

Rebuilt the two-mark RCS nudge as a four-mark box around the stopped work mark:
300 m across to TRIM A, 220 m up to B, back across to C, then down to D at the
starting line. All four 100 m intersections spawn together after Cutter's real
STOP. A wide Cutter-relative pose frames the complete route while control is
suspended and the copilot explains short taps, translation without turning and
the violet velocity display. The camera and control return together before RCS
is granted and only TRIM A becomes the active objective; the active marker then
advances one corner at a time while completed beacons despawn.

Added `examples/playable/first_shift_rcs.rs` as the focused handling and framing
bench. It uses the shared complete belt, mainline Cutter and carrier prototypes,
150 m/s cap, light rig, route coordinates and briefing pose. Its live route
moves the active marker A-B-C-D after the camera returns. A rendered 1280x720
review loaded all 71 objects: all four labels and Cutter fit in the briefing
frame, only A is gold after release, and the later marks remain visible.

Proof: 17 focused First Shift structural tests pass, including the complete-box
spawn, geometry, camera and control-return pin; `cargo check --example
first_shift_rcs --features debug` passes; generated content lint reports 0
errors, 0 warnings and 0 findings; web CI passes; `git diff --check` is clean.

### 2026-09-04: Reusable First Shift scenes

Partitioned the unchanged First Shift event graph into nine named production
scenes. Mainline concatenates those fragments in order. A narrow
`nova_authoring::prelude::first_shift_scene` API builds standalone reviews from
the same world spawns, cast, story, objectives, cameras, actions and handlers.
Preview prerequisite state is assembled around the production boundaries, but
ship poses remain constants in each example and the reusable scenes never move
the player.

Replaced the two duplicated benches with numbered scene examples and added the
remaining seven. The salvo example keeps its still and rail-hit instrumentation
by modifying the production sequence after construction. Every standalone
scene now closes with a PREVIEW comms line that says where that scene would end;
the campaign graph does not receive these preview-only lines.

Proof: 18 focused First Shift structural tests pass, including production-stage,
production-pose and explicit-end-message checks for all nine scenes. All nine
examples compile with debug features. Eight thin scenes complete rendered
runtime smoke checks; the salvo captures a 466,806-byte rail-hit loop. Generated
First Shift content is unchanged, and content lint reports 0 errors, 0 warnings
and 0 findings.

### 2026-09-04: Silent terminal cinematic

Kept player and camera authority suspended after the warship reveal. The
approach no longer returns to the chase rig for a flight segment that gives the
player nothing useful to do. Recut the destruction as a silent sequence:
warship launch, Meridian railgun strike, then Cutter holding the torpedo
impacts, the warship starting away and the aftermath without another cut. The distress act opens on
that final shot and scenario teardown restores authority.

Standalone post-training scenes now explicitly enable RCS and Lock instead of
inheriting the production Cutter's fresh-spawn gates. Later scenes also receive
GOTO and ORBIT when their campaign prerequisites would already have granted
them. Mainline capability teaching is unchanged.

Proof: 18 focused tests pass, including silent-salvo, shot-order, terminal
control-authority and preview-capability pins. Scenes 07-09 compile. Content
lint reports 0 errors, 0 warnings and 0 findings. A rendered five-still review
confirmed there is no camera cut between Cutter's torpedo and aftermath views;
the 575,077-byte rail-hit loop and complete 34.5-second walk both exit cleanly.

### 2026-09-04: First Shift playtest revision, step 6

Reduced each maintenance crate's pickup radius from 80 m to 15 m. The visible
15 m tumbling cube has a 13 m half-diagonal, leaving 2 m of sensor tolerance.
Cutter already contributes its compound section colliders to the overlap, so
the crate sensor no longer adds another ship-sized standoff. The sensor remains
non-solid, so pickup does not require or cause a destructive physical impact.

Moved the standalone salvage preview to a 70 m abeam fixture. A rendered
1920x1080 review showed the complete crate and Cutter side by side before
contact. Driving the real Shift-plus-mouse RCS input laterally completed the
first crate objective and revealed the second; no scripted repositioning or
example-side scenario transition was used. The three crate centres remain
clear of every worst-case plate rock.

Proof: 18 focused First Shift tests pass, including a pin that the pickup sphere
encloses the rotating crate with at most 2.1 m tolerance. The salvage example
compiles, generated content lint reports 0 errors, 0 warnings and 0 findings,
and `git diff --check` is clean.

### 2026-09-04: First Shift playtest revision, step 7 fixed corridors

Added a conservative corridor check for every GOTO leg with an authored start:
second crate to TRANSIT 1, TRANSIT 1 to TRANSIT 2, and the last crate to the
Meridian hold. Each segment must clear both planetoids and every shared-stage
rock by the worst-case mesh radius, Cutter's 55 m hull sphere, and another 100 m
of flight margin.

The new check failed on the first leg: it passed 354 m from the nearby plate
rock against a required 365 m envelope. Moving TRANSIT 1 southwest from
(-1600, 100, -3600) m to (-1400, 100, -4000) m clears that rock and preserves
the next corridor. All temporary marks also remain outside both gravity wells.
The planetoid approach is manual rather than a GOTO. The orbit-to-work leg has
no fixed start until step 8 gives the full orbit an authored departure angle;
that integration remains with the orbit revision rather than pretending an
arbitrary orbit point is a proved route.

### 2026-09-04: First Shift playtest revision, step 8

Added reusable `OnOrbitLap` scenario vocabulary. The orbit tracker now sums
signed radial-angle changes around the maneuver's sticky plane while ORBIT is
stable, emits once per net revolution, and resets partial progress when
station-keeping becomes unstable. Backtracking does not count as progress.

First Shift no longer completes the detour on a 13-second timer. Its mandatory
GOTO route now passes around the inspection body and ends 4.00 km behind it
from Meridian. Cutter must complete a physical lap, then continue to a fixed
near-side gate before Meridian calls. The resulting gate-to-work GOTO corridor
is included in the conservative shared-stage clearance proof. The work site
moved to preserve that direct corridor.

The standalone orbit preview was rendered at 1280x720 from the new covered
start: Cutter remained settled 3.99 km from the survey body while the detour
prompt opened. A synthetic-input attempt did not acquire the off-screen travel
lock, so the full rendered lap still needs the next hands-on playtest; angular
completion, event dispatch, cover geometry, departure gating, and route safety
are covered by focused tests.

Proof: all 294 `nova_scenario` unit tests and all 19 focused First Shift tests
pass. The affected examples and editor compile, content lint reports 0 errors,
0 warnings and 0 findings, mdBook builds, and web CI passes.

### 2026-09-04: First Shift playtest revision, step 9

Rebuilt the comms card as a screen-relative subtitle panel: 48% of the viewport
with a 960 px ultrawide ceiling, 20 px message text, a separate 14 px speaker
header, 48 px portrait, and larger spacing and padding. The visible stack still
paces three cards, but its pending queue is now lossless; a creator-authored
burst no longer silently drops old lines.

Raised posted objective text from 13 px to 17 px. Raised world-objective labels
from 12 px to 16 px, their diamond from 8 px to 12 px, and their edge chevron
from 16 px to 22 px. Objective notifications can wrap inside 80% of the
viewport instead of forcing one unbounded line.

Rendered review:

- `/tmp/comms-window-1280x720.png`: one broad card remains clear over Meridian
  and uses two lines without covering the flight centre.
- `/tmp/comms-window-1920x1080.png`: the same line has comfortable subtitle
  width and remains anchored to the lower-left safe area.
- `/tmp/comms-window-1920x720.png`: at a 2.67:1 ultrawide ratio the card remains
  bounded rather than stretching across the display.
- `/tmp/objective-window-1280x720.png` and
  `/tmp/objective-window-1920x1080.png`: both the posted objective and the
  CRATE world marker remain legible over dense, bright asteroid clutter.

Proof: all 238 `nova_hud` unit tests pass, including screen-relative card,
distinct text-scale, lossless queue, live objective-chip layout, and chevron
alignment checks. mdBook builds and web CI passes.

### 2026-09-04: STOP prompt wording correction

Corrected First Shift's STOP bark and objective from "hold [X]" to "press
[X]". STOP is a toggle, so the old story text could make a player keep the key
down and conclude that the maneuver was not working. The getting-started guide
now says to press X once and let STOP finish.

Proof: all 19 focused First Shift tests pass, generated content lint reports 0
errors, 0 warnings and 0 findings, and web CI passes.

### 2026-09-04: First Shift dialogue pass, scene 1

Replaced the three-line departure scaffold with the owner-reviewed Cutter One
briefing. The experienced crew receives three Plate Seven recoveries: two
manifested crates and one loose, unweighed return. Meridian's departure clock
and cabin-channel banter establish the carrier as both workplace and home
before the ordinary shift begins.

The opening now uses an explicit Cutter-relative conversation hold. Player and
camera authority remain suspended while Cutter One sits against Meridian in the
establishing frame, then return together when the work mark and first flight
bark appear. The two dense briefing lines get six seconds to land, ordinary
lines four, normal replies three, and terse exchanges two.

The campaign notes record the approved post-maintenance spine: the four-way RCS
box checks repaired translation, later GOTO work checks integrated guidance and
braking, and the unauthorized orbit extends that checkout under gravity.

Proof: all 19 focused First Shift tests pass and generated content lint reports
0 errors, 0 warnings and 0 findings. A rendered 1280x720 review at 10, 25 and 43
seconds keeps Cutter One, Meridian and up to three dialogue cards readable
through the full conversation.

### 2026-09-04: First Shift dialogue pass, scene 2

Recast the four-mark RCS lesson as Cutter One's live acceptance check after a
scheduled port-manifold replacement. The engineer challenges the copilot's
caution; the answer is Prospector Six, another company's cutter whose green
computer report preceded a locked-open manifold and the cutter's loss in the
belt. The flight recorder cleared its dead crew after news blamed pilot error.

The safe wide shot now holds a paced eight-line conversation before returning
camera and helm authority. Each mark gets a longer response that reports what
the crew observes and directs the next leg. Objectives explicitly name Shift
and mouse movement, while the first flight bark connects the violet velocity
marker to RCS. Returning to the origin proves no residual drift before Meridian
closes the handling card and releases the first manifested recovery.

Proof: all 19 focused First Shift tests pass; generated content lint reports 0
errors, 0 warnings and 0 findings; web CI passes. A rendered 1280x720 review at
10, 22, 33 and 39 seconds keeps the full trim box visible throughout the
conversation and returns to a readable objective, active mark and flight bark.

### 2026-09-04: First Shift dialogue pass, scene 3

Kept both manifested recoveries deliberately ordinary and unspecified. Contact
with the first gets one short engineer report: sound seal, matching tag, and a
warning that the next mark lies deeper in the plate. Contact with the second
confirms both manifests, then gives the Deck Chief a separate line to introduce
the third crate and Control's route around the survey body. The dialogue remains
normal flight comms with no conversation hold during close asteroid work.

Proof: all 19 focused First Shift tests pass and generated content lint reports
0 errors, 0 warnings and 0 findings. A rendered 1280x720 scene-opening review
keeps the first manifested-crate objective, contact marker, Cutter One and Deck
Chief handoff readable together over the dense plate.

### 2026-09-04: First Shift dialogue pass, scene 4

Made both safe transit legs the guidance half of Cutter One's post-maintenance
release. TRANSIT 1 now checks turnaround, automatic braking and physical
arrival. The engineer asks whether one solution is enough; the copilot requires
the complete lock-and-GOTO operation again at TRANSIT 2. Its arrival closes the
maintenance release before the crew notices its visual cover behind the survey
body.

The exchange remains ordinary comms over low-workload autopilot travel. Control
laid the marks on the real route to the third crate, so no extra training leg or
conversation hold interrupts the assigned work.

Proof: all 19 focused First Shift tests pass; the standalone navigation scene
compiles with debug features; generated content lint reports 0 errors, 0
warnings and 0 findings; web CI passes.

### 2026-09-04: First Shift dialogue pass, scenes 5-9

Carried the established workplace voice through the remaining scenes. The orbit
is now explicitly an unscheduled gravity check that everyone calls a donut. The
stable lap verifies the new manifold under correction, and Meridian catches the
crew after the maintenance release was already filed. The third crate remains
an ordinary unmanifested recovery before the final call home.

The attack approach identifies a dark Earth Navy hull with no fleet code and
names Meridian as an unarmed Earthworks carrier. The Deck Chief recognizes its
bow rail apertures as the hull aligns. Scene 8 remains narratively silent through
weapons launch, impacts, destruction and departure. Scene 9 gives each surviving
crew voice one restrained beat before the automatic distress beacon answers.

Proof: all 19 focused First Shift tests pass; scenes 05-09 compile with debug
features; generated content lint reports 0 errors, 0 warnings and 0 findings;
web CI passes. Rendered 1280x720 captures under `/tmp/first-shift-story-05.png`
through `09.png` verify each preview, with additional scene 07 captures for the
Earth Navy identification and Earthworks challenge. Scene 08 remains free of
narrative cards.

### 2026-09-04: Transit beacons moved clear of gravity

Owner review caught that both transit beacon centres were only about 4 km from
the inspection planetoid's centre. The old test compared only each centre with
the 3.29 km sphere of influence. It ignored the beacon's own 700 m area, leaving
its volume almost touching the well and placing the marker visibly against the
planetoid. That proof was inadequate.

TRANSIT 1 now routes below the plate at 6.00 km from the body. TRANSIT 2 remains
on the covered bearing from Meridian but stands 7.00 km from the body. The
revised test requires every complete beacon volume plus another 500 m visible
buffer to clear every gravity well. A second test proves both GOTO transit
segments also clear the inspection well with Cutter's 55 m hull sphere and 100
m flight margin included. The orbit preview pose moved with TRANSIT 2.

Proof: all 20 focused First Shift tests pass, including complete beacon-volume
and transit-segment gravity clearance. The orbit example compiles, and generated
content lint reports 0 errors, 0 warnings and 0 findings. A rendered scene 05
start at `/tmp/first-shift-transit2-clear.png` shows the survey body 7.00 km
away after TRANSIT 2 despawns, rather than filling the beacon's vicinity.

### 2026-09-04: Campaign portrait style candidates

Added two non-runtime studies of the same provisional industrial commander under
`art/portrait-candidates/`: one hard-pixel CRT treatment and one faceted
low-poly treatment. Both use close-cropped grey hair, a cyan work headset and an
amber industrial uniform so the review compares rendering style rather than two
different character concepts. Editable SVG and 512x512 PNG versions remain in
source art only; no scenario references or shipped assets were added pending
owner selection. Both were also inspected at the comms panel's 48x48 size.

Owner selected the CRT direction for another pass. Added a cleaner green
phosphor palette candidate and a separate exaggerated shader mock showing bloom,
green tint, and tube-edge falloff. The clean pixel portrait remains the source;
the treatment is not baked into runtime content. Both revisions remain readable
at 48x48 and await visual selection before a shared UI material is designed.

Owner accepted the clean green CRT treatment without the shader mock. Promoted
that direction into seven distinct campaign portraits: Meridian Control, Deck
Chief, Copilot, Engineer, the helmeted player, automated beacons and unknown
channels. Added reproducible 32x32 SVG generation, shipped 512x512 PNGs through
the base bundle, and attached portraits recursively to First Shift and Second
Shift story actions. Preview-only speakers retain the HUD fallback. No runtime
shader was added; the accepted clean pixel treatment remains unblurred at 48x48.

Removed the comms card scale pop after owner review found it crossed the left
screen edge and visually intruded into older transcript cards. New messages now
use the existing fade and audio blip only. The flex stack admits each card at
its final layout size, so previous lines move once and retain stable geometry.

Corrected the return route after owner playtesting identified that WORK SITE was
inside Belt Rock 6's default-mass gravity well, where GOTO could not settle.
Moved WORK SITE, its complete 700 m volume, the third crate, and a 500 m visible
buffer outside every authored and radius-designated well in the shared belt.
Expanded the regression proof from the two planetoids to every rock that the
runtime promotes to a well, and included all three salvage objectives. Raised
the default and First Shift beacon signatures from their 6/9 km ranges to 12 km;
every prescribed lock leg now has a range proof from its preceding goal.

### 2026-09-04: Public comic player foundation

Added the public Story archive and a full-screen digital-HUD comic reader. Each
comic now owns one path-indexed `web/src/comics/<path>/comic.json`; the build
discovers those manifests, validates unique ids, TypeScript page sources and
cover assets, generates archive cards and campaign routes, and embeds the
ordered definition in one generic shell.

`ComicPlayer` is campaign-agnostic. It receives rendered page elements and owns
wheel, touch, keyboard, previous/next, contents, progress and deep-link state.
It displays exactly one fitted page at a time, so a page never shares the screen
with either neighbour. Five Nova Protocol page modules exercise the typed
function DSL and shared renderer, including external SVG assets and safe inline
SVG primitives. No page module names a CSS class.

Proof: web format, lint, tests and build pass, including path discovery and
manifest-order tests. The generated cover and First Shift page were inspected at
1440x900 and 390x844; each fills one reader viewport with no adjacent page
visible. All bounded preview servers were stopped by recorded PID.
