# Nova technical specification for story planning

Implementation baseline: `91f67d8d12aed96f04414240dda671ae7435ab8c`.

This is a name-free capability reference for a new story and playable campaign.
It establishes no setting, characters, factions, places, history, or plot.
Existing content supplies hardware examples, not fictional canon.

## How to use this specification

- **Implemented:** a mechanic exists in the current runtime.
- **Configured:** a value in the current base content or runtime defaults.
  Other content can use different values.
- **Derived:** arithmetic from those values, under stated assumptions.
  It is not a new flight measurement or a guarantee of combat performance.
- **Gap:** do not depend on this as playable behavior without new work.

Damage and penetration values are game quantities, not joules, explosive yields,
or armor thicknesses. Neither an effect nor a code comment establishes a drive
technology, warhead composition, crew arrangement, or material science.

Source paths below are repository-relative. Line references describe this
baseline. Recheck the implementation if the revision changes.

## 1. Scale, hulls, and propulsion

### Geometry and mass

The simulation is three-dimensional. A build-grid cell is **10 m** across.
The physics boundary converts one engine world unit to 10 m. Story and mission
figures must use meters, meters per second, and meters per second squared.

Ships consist of connected, individually damageable sections. Their colliders
supply mass and inertia. Section density is fixed at 1 in engine coordinates;
there is no separately authored cargo load or physical material density in this
section model. Do not turn its mass values into fictional tonnes or thrust in
newtons without first defining a separate physical mass convention.

<!-- Sources: crates/nova_events/src/units.rs:32;
crates/nova_ship/src/sections/base_section.rs:455;
crates/nova_scenario/src/objects/spaceship.rs:335. -->

### Main-drive acceleration

A thruster's `magnitude` is an **impulse per fixed tick**, not an acceleration.
The current default tick rate is **64 Hz**. For aligned, fully spooled engines:

```text
J = sum of forward thruster impulses per tick
M = sum of section collider volumes at engine density 1
forward acceleration = 10 * 64 * J / M m/s^2
acceleration in g = forward acceleration / 9.81
```

This is an engine-to-SI derivation, not a physical engine specification.
Changing the tick rate changes main-drive acceleration unless thrust is retuned.

| Configured drive size | Collider dimensions | Impulse per tick | Section health |
| --- | --- | ---: | ---: |
| Small | 10 x 10 x 10 m | 1 | 70 |
| Medium | 30 x 30 x 20 m | 9 | 480 |
| Large | 50 x 50 x 30 m | 25 | 1,250 |

The following are **derived intact-hull reference values**, not ship classes.
Rows follow the first six ship entries in the generated base ship catalog.
Names are deliberately omitted. Mass and impulse columns are internal arithmetic
inputs, not player-facing physical units.

| Reference configuration | Sections | Engine mass M | Forward impulse J | Acceleration | g |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1: unarmed, two small drives | 26 | 26 | 2 | 49.23 m/s^2 | 5.02 |
| 2: unarmed, one medium drive | 51 | 68 | 9 | 84.71 m/s^2 | 8.63 |
| 3: six turrets, one medium drive | 53 | 64.75 | 9 | 88.96 m/s^2 | 9.07 |
| 4: two turrets, four small drives | 35 | 33.25 | 4 | 76.99 m/s^2 | 7.85 |
| 5: unarmed, two large and two small drives | 2,081 | 2,229 | 52 | 14.93 m/s^2 | 1.52 |
| 6: mixed weapons, two medium drives | 341 | 376.25 | 18 | 30.62 m/s^2 | 3.12 |

Assumptions: catalog geometry, no spawn modifications, intact sections, full
forward thrust, no speed-cap taper, no external acceleration, and no spool or
attitude transient. These values come from summing the generated sections and
forward-facing engines. They are not timed acceleration runs.

Actual authority changes with the build and damage. Losing hull can reduce mass;
losing an engine reduces thrust. Losing asymmetric sections shifts the center of
mass. The allocator balances differential thrust and can recruit off-axis drives
to reduce torque, with bounded lateral drift. A damaged ship need not simply be
slower in every respect.

<!-- Sources: crates/nova_ship/src/sections/thruster_section.rs:481;
crates/nova_ship/src/flight/manual.rs:219;
crates/nova_ship/src/flight/thrusters.rs;
crates/nova_authoring/src/base_content/sections/standard.rs:623,642,660-685;
assets/base/sections/base.content.ron;
assets/base/ships/base.content.ron:2,403,1179,1985,2521,33747. -->

### Attitude, RCS, and speed

**The 8g constant is not a universal forward-acceleration cap or crew limit.**
It is a structural budget for attitude control: **78.48 m/s^2** at the hull arm.
Turning authority uses the lower of controller torque divided by inertia and
structural load divided by arm length. Existing spin consumes structural margin.
More controllers add torque, but cannot remove the structural ceiling.

RCS applies translation at the center of mass without requiring fitted side
thrusters. Its default acceleration is **49.05 m/s^2 (5g)**, independent of hull
mass, with a **100 m/s** speed budget. Both numbers matter: despite its fine-adjust
role, RCS is not a weak, millimeter-per-second system. Orbit control can measure
this budget relative to a reference velocity; manual RCS normally uses zero.
A live controller with the RCS capability is required.

Main-drive flight preserves momentum. Turning does not rotate the existing
velocity vector. Braking must change that velocity. The autopilot can turn the
ship and burn to stop; RCS also provides capped translation.

There is **no universal main-drive top speed**. A scenario can install a soft
manual-burn speed cap. This tapers acceleration, rather than clamping all motion,
and does not define the autopilot's cruise speed or a technology-wide limit.

No finite propellant supply, drive power budget, crew acceleration injury, or
thermal endurance limit is part of the inspected flight model. Sustained burns
therefore do not establish any particular fuel, reactor, or inertial protection.

<!-- Sources: crates/nova_events/src/scale.rs:16;
crates/nova_ship/src/physics/attitude.rs:72-143;
crates/nova_ship/src/sections/controller_section.rs:440-469;
crates/nova_ship/src/flight/state.rs:40-52,408-446;
crates/nova_ship/src/flight/manual.rs:265-362. -->

### Travel and gravity

- Autopilot supports stop, travel to a body or position, and orbit insertion and
  station-keeping. Maneuvers use physical flight, not teleportation.
- Default arrival clearance is **500 m between hull surfaces**, overridable per
  ship. Reaching a navigation mark is not a docking handshake.
- GOTO tracks a moving destination's position, but is not a general collision-
  avoidance system. Do not assume it can route safely through arbitrary clutter.
- Gravity uses authored, one-way inverse-square wells. Pull is surface-clamped,
  fades near a sphere-of-influence boundary, and uses one dominant well where
  wells overlap. This is not mutual N-body gravity or a full astronomical model.
- There is no verified seamless interplanetary travel contract. Campaign scene
  transitions can represent travel without implementing the entire journey.

For an ideal constant-acceleration estimate, `delta-v = a * t` and stopping
distance is `v^2 / (2 * a)`. A symmetric rest-to-rest trip over distance `D` takes
`2 * sqrt(D / a)`. These exclude turning, spool time, gravity, speed caps, and
arrival clearance. Use them as planning bounds, not exact mission timers.

<!-- Sources: crates/nova_ship/src/flight/state.rs:175-208,414;
crates/nova_ship/src/flight/autopilot.rs;
crates/nova_gameplay/src/gravity.rs:1-27;
crates/nova_scenario/src/actions/flow.rs. -->

## 2. Weapons and ammunition

Ranges below are **nominal travel distances**, not guaranteed engagement ranges.
Projectile lifetime, inherited launch motion, closing speed, guidance, cover,
weapon arcs, and AI policy all affect a real shot.

### Turrets and point defense

| Configured property | Current base value |
| --- | --- |
| Muzzle speed | 1,000 m/s |
| Projectile lifetime | 2 s |
| Derived nominal travel | 2,000 m |
| Total cadence per mount | 100 rounds/s |
| Magazine | 500 rounds |
| Idle reload | 200 rounds after each 3 s without a successful shot |
| Kinetic damage | 4 per hit at 1,000 m/s closing speed |
| Pierce damage | 2 per crossed layer, before any travel-budget limit |
| Yaw and pitch slew | 180 degrees/s |
| Local pitch arc | -10 to +90 degrees |

Single-stream and twin-stream mounts exist. The twin fires 50 rounds/s from each
muzzle, sharing the same magazine; it does not double total damage output.

Derived full-magazine firing time is about **5 s**. Kinetic output is nominally
**400 damage/s** if every round hits at the reference closing speed. Do not use
this as actual time-to-kill: misses, penetration, armor placement, mount arcs,
reloads, and damage geometry matter.

Point defense assigns individual turrets to engageable torpedoes and retains
assignments to account for slew time. It is not an invulnerable defensive bubble.
Multiple approach directions, occlusion, armored ordnance, and salvo saturation
matter. Guns also need deployment and a clear firing solution.

<!-- Sources: crates/nova_authoring/src/base_content/sections/standard.rs:30-78,
264-310,474-500;
crates/nova_ship/src/input/point_defense/assignment.rs;
crates/nova_ship/src/sections/turret_section/stow.rs. -->

### Guided torpedoes

| Configured property | Standard straight | Standard evasive | Heavy |
| --- | ---: | ---: | ---: |
| Cruise cap | 350 m/s | 320 m/s | 700 m/s |
| Weave half-angle | 0 rad | 0.44 rad | 0.22 rad |
| Lifetime | 100 s | 100 s | 60 s |
| Blast radius | 300 m | 300 m | 450 m |
| Center blast damage | 750 | 750 | 2,000 |
| Projectile health | 10 | 10 | 5,000 |
| Bay magazine | 6 | 6 | 6 |
| Bay launch cadence | 1/s | 1/s | 1/s |
| Idle reload | 1 per 10 s | 1 per 10 s | 1 per 10 s |

All three eject at **80 m/s**, ignite after **0.6 s**, and configure arming gates
of **0.5 s** and **50 m** separation. Door opening, ignition, acceleration, and
turning mean cruise cap times lifetime is not measured range.

The two standard types differ in flight, not warhead strength. Evasion makes
interception harder at a lower configured cruise cap. Source commentary reports
about **9.10 s** and **9.78 s** over a **3,000 m** approach. These are previously
reported results, not measurements repeated for this specification. Do not
convert their point-defense ammunition costs into guaranteed interception odds.

A normal player launch without a combat target wastes the round: it does not
acquire a target later and cannot trigger its proximity blast. Scripted launch
orders can supply a target. Guidance and proximity fuzing exist; damage does not
require a cinematic hit to be faked.

Heavy ordnance is much tougher than standard torpedoes. A normal point-defense
mount is not a reliable answer to it. This is configured combat asymmetry, not
proof that the projectile is physically indestructible.

<!-- Sources: crates/nova_authoring/src/base_content/sections/ordnance.rs:12-21,
46,106-117;
crates/nova_authoring/src/base_content/sections/standard.rs:983-1015,1237-1289;
crates/nova_ship/src/input/player/intent.rs:194-234;
crates/nova_ship/src/sections/torpedo_section/projectile.rs:190-209. -->

### Spinal railguns

| Configured property | Standard | Heavy |
| --- | ---: | ---: |
| Charge time | 1.5 s | 1.5 s |
| Slug speed | 15,000 m/s | 15,000 m/s |
| Lifetime | 1.2 s | 1.2 s |
| Derived nominal travel | 18,000 m | 18,000 m |
| Damage per crossed layer | 300 | 500 |
| Penetration power | 1,800 | 360,000 |
| Rake radius | 10 m | 30 m |
| Magazine | 1 | 1 |
| Idle reload | 1 per 12 s | 1 per 12 s |

The mount does not traverse: the hull aims the bore. Triggering commits the
charge; releasing the trigger or moving off target does not cancel it. Switching
weapons safe can cancel the charge and retain the shell. A held trigger produces
roughly one shot per **13.5 s**, not continuous fire.

A shot applies recoil at the muzzle, so an off-center installation can rotate
the firing ship. Configured recoil is **45 engine impulse units** per shot.
Its linear velocity change is `450 / M m/s` for engine mass `M`, opposite the
bore; rotational response also depends on placement and inertia.

Penetration power buys travel through layers, not damage per layer. A standard
shot can destroy a 200-health hull section but does not remove a fresh
1,250-health drive in one hit. The heavy variant is intentionally far more
powerful in penetration and swept width; do not assume all encounters are
balanced around it.

<!-- Sources: crates/nova_authoring/src/base_content/sections/standard.rs:908-950,
1150-1180;
crates/nova_ship/src/sections/railgun_section/firing.rs:84-108,159-234. -->

### Reload is not logistics

Every successful shot resets that weapon's reload clock. An uninterrupted delay
restores an authored batch, repeatedly, up to magazine capacity. No ammunition
factory, carrier resupply, stockpile, or cargo transfer is required by this
mechanic. A story about running out permanently needs deliberate content or code
work; it is not the stock reload behavior.

Weapon configurations can also omit a magazine for unlimited ammunition, or omit
reload for a finite non-replenishing magazine. Scenarios can refill magazines or
enable unlimited ammunition. These are authoring controls, not established
in-world technologies.

Source: `crates/nova_ship/src/sections/ammo.rs`;
`crates/nova_scenario/src/actions/mod.rs`.

## 3. Damage, cover, and defeat

| Representative configured section | Health |
| --- | ---: |
| Reinforced hull | 200 |
| Controller | 100 |
| Turret | 130 |
| Torpedo bay | 100 |
| Spinal railgun | 180 |

- **Kinetic:** damage scales with closing speed, from 0.25 to 2 times authored
  damage, with 1 at 1,000 m/s. A round continues only through layers it destroys,
  spending its remaining damage budget.
- **Pierce:** full authored damage at every crossed layer. Closing speed changes
  penetration power, from 0.5 to 3 times its reference effectiveness. Standard
  turret penetrators start with power 300 and a six-layer limit. Railguns use
  their own power and no practical layer-count cap.
- **Explosive:** linear radial falloff to zero at blast radius. A surviving
  structural section blocks pressure behind it. Each destroyed structural layer
  transmits 65 percent of the remaining pressure.
- Section class is not a hidden resistance table. Current travel and health
  rules take precedence over older catalog comments about class multipliers.
- Structural connections matter. Severed portions can become inert wreck
  fragments. Asteroids can be carved and broken; their remaining material,
  rather than a conventional whole-rock health pool, determines durability.
- An armed ship is **neutralized** if all working weapons are lost, or if it
  loses its last controller after having one. Thruster loss alone does not
  satisfy that rule. Empty ammunition is not the same as a destroyed weapon.
- Neutralization leaves a wreck. It is distinct from physical destruction.
  The unified defeat event fires once, not again when that wreck later breaks.
  An always-unarmed ship does not satisfy the armed-ship neutralization rule.

This supports disabling shots, vulnerable components, ambushes using physical
cover, and wrecks that remain after combat. It does not implement boarding,
prisoner capture, crew casualties, pressure loss, or salvage ownership.

<!-- Sources: crates/nova_authoring/src/base_content/sections/standard.rs:30-40,596;
crates/nova_gameplay/src/damage.rs:183-266,418-452,519-579;
crates/nova_gameplay/src/integrity/neutralize.rs:1-21;
crates/nova_ship/src/sections/integrity.rs;
crates/nova_gameplay/src/integrity/carve.rs. -->

## 4. Sensors, targeting, and AI

Travel and combat designations are separate. New acquisition requires sight;
a held travel designation can survive intervening radar cover, while a combat
lock requires a clear radar path. Physical occluders matter. This is not a full
passive-emissions, electronic-warfare, or light-delay model.

Configured player acquisition gates are:

- Ships and gravity-well bodies: **200,000 m**.
- Committed torpedoes: **25,000 m**.
- Unsigned debris: **50 m**.
- Other signed bodies: signature scaled by the configured range ratio.
- Incumbent designations have a **1.15** range multiplier to avoid boundary
  flicker. These are game gates, not absolute real sensor specifications.

Component targeting unlocks after **1.5 s** of continuous combat lock. This is
separate from initial acquisition dwell. Combat lock decays after **30 s** without
combat activity; holding a weapon trigger or raised weapon stance resets that
clock. Allegiance changes can also clear a hostile lock.

AI supports engagement, idle behavior, patrol routes, orbit directives, and
under-fire evasion. Territorial leashes and scripted orders can constrain it.
**Autonomous low-integrity retreat is not implemented:** the `Retreat` state
currently uses engagement behavior.

Default AI policy is narrower than theoretical weapon travel:

- Turret fire gate: **1,800 m** for the base turrets.
- Point-defense selection range: **1,500 m**, overridable by scenario.
- Torpedo launch band: three blast radii to **10,000 m**, with other targeting
  and alignment gates. Thus the standard lower bound is **900 m**, heavy
  **1,350 m**.

A script can order a withdrawal or change allegiance. That does not establish
an autonomous surrender, negotiation, strategic command, or fleet logistics AI.

<!-- Sources: crates/nova_ship/src/input/targeting/contacts.rs:12-29,149-171,264-310;
crates/nova_ship/src/input/targeting/state.rs:78-89;
crates/nova_ship/src/input/ai/behavior.rs:22-55;
crates/nova_ship/src/input/ai/guns.rs:65;
crates/nova_ship/src/input/ai/acquisition.rs:268;
crates/nova_ship/src/input/ai/torpedo.rs:26-32,63-72. -->

## 5. Playable campaign capabilities

The scenario system is event-driven. Current actions and events can compose:

| Campaign need | Available mechanism | Boundary |
| --- | --- | --- |
| Objectives and progress | Objectives, markers, variables, expressions, timers | The author supplies the mission rules |
| Briefings and dialogue presentation | Narrative cues, HUD readouts, sounds, cinematic titles | Presentation is not a conversation or social simulation |
| Staged encounters | Spawn, scatter, despawn, areas, allegiance changes | No persistent simulated economy or strategic war is implied |
| Navigation and escort staging | Move, stop, patrol, orbit, order-completion events | An escort objective needs authored success and failure rules |
| Combat outcomes | Defeat, neutralization, destruction events | Select the right event; a wreck need not disappear |
| Cinematic action | Camera anchors, control suspension, sequences, forced weapon orders | A forced weapon order still uses the weapon's mechanics |
| Recovery objectives | Proximity salvage pickups and entry events | Collection is scenario state, not a cargo inventory |
| Mission branching | Conditions, variables, explicit next-scenario selection | State does not automatically carry to the next scene |
| Endings and retries | Victory/defeat overlay, delayed or lingering transitions | Outcomes and continuation must be authored |

Campaign content groups an ordered list of scenarios, including hidden entries.
That list is not itself a mission-completion or persistent progression system.
Transitions are explicit actions. Loading or unloading clears the scenario event
world, variables, objectives, and scenario-scoped entities.

Do not assume that hull damage, ammunition, collected cargo, choices, or a player
ship survive a scene transition. Conditional transitions can choose a branch
before teardown. General cross-mission state and save/checkpoint continuity need
a separate implementation contract.

Sources:

- `crates/nova_scenario/src/actions/mod.rs` and `actions/flow.rs`.
- `crates/nova_scenario/src/events.rs` and `queries.rs`.
- `crates/nova_scenario/src/objects/salvage.rs`.
- `crates/nova_scenario/src/loader/mod.rs`, `loader/lifecycle.rs`, and `world.rs`.

## 6. Things the story must not silently assume

The inspected runtime and content do not establish playable systems for:

- Fuel consumption, reactor management, overheating, radiation, life support,
  crew endurance, or an engineering explanation for the drive.
- Docking and boarding, crew combat, interior exploration, repair operations,
  towing, or cargo transfer.
- Resource extraction, refining, trade, a persistent inventory, or an economy.
  Destructible rocks and proximity pickups are not that complete loop.
- Energy shields, beam weapons, cloaking, jamming, or faster-than-light travel.
- Real astronomical distances and travel times, communications delay, or
  physically calibrated explosive yields.
- General persistent campaign state, autonomous surrender, or strategic fleet AI.

These are **implementation gaps, not declarations that the future setting
forbids them**. A new story may keep such activity off-screen, represent a result
with an explicit script, or request a new mechanic. Mark which choice is needed.
Do not describe a scripted substitute as a simulated capability.

Before accepting a campaign beat, identify the player's action, the runtime
mechanic it uses, its spatial and timing assumptions, the event that advances
it, and its failure condition. Mark any unsupported dependency before treating
the beat as playable.
