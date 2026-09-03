# Reboot Nova Protocol with block ships and move the Kenney fleet into The Ledger

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.13.0,content,scenario

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
leads forward to a salvage field between the two planetoids, then to the small
inspection body; the large body hides the warship on the opposite side. Three
crates are pinned clear of every salvage rock's worst-case generated surface,
and 20 additional fixed rocks dress the wider stage. Detailed objective
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

Proof: `cargo check --example first_shift_ships --features debug` passes inside
`nix develop`; a 30-second rendered smoke reached `Playing`, passed the example's
real content-lint gate, clad all three ships, and remained live until the bounded
smoke timeout. `cargo check --example first_shift_map --features debug` passes;
its rendered warship and cutter smokes each loaded 51 fixed objects, posted the
role-specific objectives, attached all ten explanatory markers, clad all three
ships, transferred the camera to the selected player ship, and remained live
until the bounded timeout.
