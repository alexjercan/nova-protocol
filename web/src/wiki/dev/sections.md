# Spaceship sections and integrity

> To add a new section kind, follow the guide
> [Add a ship section](../guide-add-section/).

Ships are assembled from modular **sections**. Each section is a child entity of
the ship root with its own collider, mass, and health, and contributes one
behavior (structure, thrust, steering, guns). The **integrity** system tracks
how sections connect and handles damage, disabling, and cascading destruction.

## Sections (`nova_ship::sections`)

A section is a `SectionConfig { base: BaseSectionConfig, kind: SectionKind }`.
`BaseSectionConfig` is shared by all kinds: `id`, `name`, `description`, `mass`,
`health`, optional `collider`, structural `link_points`, and `hide_in_editor`.

`SectionKind` variants (one module per kind under `crates/nova_ship/src/sections/`;
`turret_section/` and `torpedo_section/` are directories, not single files):

| Kind         | What it does |
|--------------|--------------|
| `Hull`       | Passive structure/armor. Just a `render_mesh`. |
| `Thruster`   | Forward thrust (`magnitude`); drives the exhaust visual. |
| `Controller` | Attitude controller (`steering_lag`, `max_angular_acceleration`); lag derives the internal PD gains. Also grants flight `verbs` (STOP/GOTO/ORBIT maneuvers plus LOCK targeting and RCS fine-translation). A ship needs one to be drivable; several SHARE one attitude loop (see below). |
| `Turret`     | Aims and fires bullets. An authored joint tree (hinges + muzzles, each joint with its own `offset`/`axis`/`speed`/limits/`render_mesh`), section-wide `muzzle_speed` + authored `bullet_damage` + `bullet_kind`, per-muzzle `fire_rate`, optional `ammo_capacity`. |
| `Torpedo`    | Torpedo bay. Fires guided torpedoes of an authored `torpedo_type` (name, tint, `max_speed`, `weave_angle`, `weave_rate`) that detonate an Explosive area blast (`blast_radius`, `blast_damage`), optional `ammo_capacity`. The TYPE is the run-in - how fast and how evasively; everything else on the config is the tube. |

`GameSections(Vec<SectionConfig>)` is the resource of section blueprints.
Generic prototypes are authored in
`crates/nova_authoring/src/base_content/sections/standard.rs`; semantic craft
parts live under `base_content/ships/`. Their explicit `section_catalog()` is
generated into `assets/base/sections/base.content.ron` by `content -- gen` and
merged into the resource by
`crates/nova_assets/src/merge.rs`. The outer-skin cladding is not a prototype at
all: a ship's skin is DERIVED from the structure it wraps by `nova_ship`'s
`shell_skin` ([below](#the-derived-skin)), so no id names a plate. Look a
section up with
`sections.get_section("basic_thruster_section")`.

### Stacked controllers share one loop

Every live controller torques the hull in parallel, so a hull with several of
them would multiply both its gains and acceleration authority by the section
count.
`update_controller_stack_tuning`
(`crates/nova_ship/src/sections/controller_section.rs`) prevents that: it runs
first in `FixedUpdate` (`ControllerSectionSystems::SyncStack`), derives ONE
ship-level attitude loop per root, and writes each live controller a share of
it into its `PDController`. The authored numbers stay put in
`ControllerSectionTuning`, which is what the pass re-derives from when a
controller dies. The smallest live `steering_lag` supplies the stack's base
response; acceleration authority is ranked independently.

The ship-level loop, for `n` live controllers on the curve
`stack_curve(n, limit) = limit - (limit - 1) / n`:

- acceleration authority: each controller's authored
  `max_angular_acceleration` at its rank weight, summing to
  `stack_curve(n, 2.0)` of the strongest - 1.00 / 1.50 / 1.75 / 1.90 at n = 1
  / 2 / 4 / 10, with a hard ceiling of 2x. The PD converts each principal-axis
  acceleration into the torque required by the live inertia, so hull size does
  not change handling by default.
- P gain: DIVIDED by `stack_curve(n, 1.5)`, which increases the effective
  steering lag so the stack brakes earlier and lands on the commanded attitude
  instead of sailing past it.
- D gain: held at exactly one fastest computer's worth. This is not tuning: `kd * dt`
  crosses 2 at two controllers on the shipped tuning, and past that the PD
  limit-cycles instead of parking (the corkscrew that used to follow a
  released maneuver).

`ship_turn_rate` (`flight/guidance.rs`) then sums the live acceleration shares,
which is why the flight layer is ordered after `SyncStack`. `n = 1` is the
identity case.

### Meshes and colliders (authorable)

Two authorable knobs decouple a section's look and physics from the default unit
cube (`crates/nova_ship/src/sections/base_section.rs`). Unset content still uses
the unit-cube defaults:

- `render_mesh_transform` (optional, on every mesh-bearing kind; for turrets
  it sits per JOINT in the joint tree) - an offset /
  rotation / scale applied to the section's render mesh child ONLY, so a model
  can be re-seated visually without moving the collider or (for turrets) the
  joint tree. Type `RenderMeshTransform`.
- `collider` on `BaseSectionConfig` (optional) - the physics shape:
  `Cuboid { size }`, `Sphere { radius }`, `Capsule { radius, length }`, or
  `Cylinder { radius, height }` (the last three along local Y). `None` resolves
  to the unit cube (`Cuboid { size: (1,1,1) }`) - the shape every section had
  before colliders were authorable. Section mass is `density * collider_volume`,
  so a larger collider is also heavier.

## Building a ship

A `SpaceshipConfig` (`crates/nova_scenario/src/objects/spaceship.rs`) has a
`controller` (`None`, `Player`, or `AI`), an `allegiance`, an optional
`collapse_threshold` (below), a `skin` flag (the
[derived cladding](#the-derived-skin)), and a list of
`SpaceshipSectionConfig`, each placing one section at a `position` + `rotation`
relative to the ship root (world units), with a `source` (`Inline` /
`Prototype`) and optional `modifications`. The player
config carries the input mapping (section id -> key/gamepad bindings) plus
`speed_cap` and `infinite_ammo`; the AI config carries `patrol`/`orbit`/`leash`/`engage_delay`.

Spawning: the base scenario bundle gives the root `RigidBody::Dynamic`; the
spaceship object adds `SpaceshipRootMarker`, and an observer
(`insert_spaceship_sections`) spawns each section as a direct child. Every
section gets `SectionMarker`, its `Collider` (the authored `collider` shape, or
a unit cube by default), `SectionLinkPoints`, `ConnectedTo`, and `Health` (`base_section` in
`sections/base_section.rs`), so the ship is one rigid body whose child colliders
each carry their own health.

See the semantic Racer, CargoA, and CargoB builders under
`crates/nova_authoring/src/base_content/ships/` for complete generated examples.
The editor (`crates/nova_editor`) assembles ships interactively using
`preview_section`, which has no health or rigid body and never enters the
damage pipeline.

### The derived skin

`skin: true` on a ship's hull asks the game to CLAD it. Nothing authors a plate:
`spawn_ship_skin` (`sections/shell_skin.rs`) reads the finished section batch on
the same `Added<SectionLinkPoints>` edge the integrity graph is built off,
buckets the sections into cells (the lattice is read off the sections, so a hull
mirrored about its centreline is not cut in two), and derives one plate per cell
of outer surface from the eight boundary samples that cell shares with its
neighbours. The same structure always gives the same skin.

- A plate is a `SectionFixture` (`sections/fixture.rs`): `Collider`, `Health`,
  density and `HealthIsolated`, but no `SectionMarker`. So it never joins the
  integrity graph, never counts toward the ship's health, never takes a damage
  tint, and never reaches the palette. Losing one costs the ship nothing it can
  DO - which is the line between a fixture and a section.
- Each plate is a CHILD of the section it clads, so a destroyed section takes
  its own cladding down with it and nothing has to hunt the plates of a part
  that no longer exists.
- On a LIVE ship, derivation runs at spawn and nowhere else. The skin is a pure
  function of the structure, so re-running it would grow back whatever combat
  blew off; `despawn_dead_fixtures` takes a dead plate away and nothing puts it
  back.
- `ShipSkinPlugin { render }` is split at the render line, not at the look line:
  the derivation and the sweep are gameplay and run headless, and `render` gates
  only the meshes hung on each plate by the `dress_skin_plate` observer.

Cladding is OPT IN. The derivation reads a hull as unit cells, which the
catalog's cube sections are and the modelled semantic parts (the racer, the
haulers) are not.

#### The editor's live preview

The build view clads the ship being ASSEMBLED, and it re-derives rather than
spawning once: `sync_editor_skin` (`nova_editor/src/skin.rs`) runs after
`sync_placement_ghost`, hashes the structure it is about to read, and respawns
the whole skin when that hash moves. Nothing is patched and nothing is
diffed - the derivation is a pure function, so throwing the plates away and
asking again is both the simplest answer and the one that cannot drift. On a
384-plate ship a reflow costs about 2 ms and an unchanged frame about 0.1 ms;
a real build is an order of magnitude smaller than that, and the ghost only
travels in whole cells (placement mates sockets), so dragging a part does not
re-derive per frame.

Two things it does differently from the spawner:

- The part UNDER THE POINTER is structure, while its placement is legal. That
  is the feature: a hull is dragged about under the skin and the cladding
  closes around it before the click. A REFUSED ghost contributes nothing - it
  will not be built, so cladding it would draw a ship that cannot exist.
- A preview plate is DISPLAY ONLY: `ShipSkinMarker` and a pose, so the shared
  `dress_skin_plate` observer still draws it, but no `SectionMarker`, no
  `Collider` and no health. The placement solver never counts one as a part,
  the pointer never hits one, and the `Q` pipette cannot arm one.

Both readings go through `read_structure` (`shell_skin.rs`), so the lattice the
editor clads on and the lattice the flown ship clads on cannot drift - and the
build state carries the toggle into the `SpaceshipConfig` the scenario spawns,
so what you see in the editor is what you fly.

#### The plate vocabulary and skin styles

The derivation works out far more than it keeps. `read_plates`
(`sections/skin_reading.rs`) is a SECOND PASS over the finished plates that reads
it back out as a `PlateReading` each: which way the plate faces, what its top is
shaped like (`Flat` / `Step` / `Ridge` / `Peak` / `Bevel` / `Brink` / `Spur`),
which way it falls away, how enclosed its cell is, how long the run of like plate
through it is and which way that run points, how far it is from the end of that
run, how much of its cell it fills, how deep the structure under it goes, and how
close the mouth of a fitting is. The plates are the whole input - their cells are
the clad set, `cell - anchor` is the face each shows - so a reading cannot drift
from the skin it describes. It costs about 0.6 ms on a 384-plate ship, against
1.6 ms to derive the skin itself.

The FALLING PLATE is three reliefs and not one, which matters because it is four
fifths of every ship. A corner sample dies to the cell floor for exactly one
reason - open space stands at it - so counting the dead corners says how many
ways a plate falls: one corner is a `Bevel` (a panel with a corner taken off),
two on one side is a `Brink` (the straight edge of a hull), and anything more is
a `Spur` (a tip, an outer corner, a saddle). Summing those corner directions and
turning them by the plate's own rotation gives `PlateReading::fall`, the
OUTBOARD direction - a cardinal on a `Brink`, a diagonal on a corner, and zero
where a plate falls two ways and cancels. It is the second alignment axis: a
piece turned to `along` lies down an edge, and one turned to `fall` leans out
over it.

A `ShipStyleConfig` (`sections/skin_style.rs`) is CONTENT resolved by id out of
`GameStyles`, exactly as a section prototype resolves out of `GameSections`. It
carries a material per surface role and a list of decoration fixtures, each with
a model `AssetRef` and a `ScatterRule` written in the vocabulary above. The mod
merge routes `Content::Style` into `GameStyles` with the same last-wins overlay
every other kind gets, so a mod restyles a base look by declaring its id.

`scatter_decor` (`sections/skin_decor.rs`) turns plates plus readings plus a
style into placements. It takes the READINGS, not the structure, so the scatter
cannot reach past the vocabulary into the derivation. Two properties are
load-bearing:

- DETERMINISM. There is no RNG. A plate's claim is a hand-written FNV-1a hash of
  its cell, its out face and the fixture's id - hand-written because
  `DefaultHasher` is not promised to be stable across releases of the standard
  library, and a ship that comes back wearing different antennae after a
  toolchain bump would break the same promise the derived skin exists to keep.
  The editor re-derives and re-scatters on every structure change, so anything
  less would flicker while a hull is dragged.
- GRID CLAIMING, not blue noise. A rule claims cells on its own `stride` and a
  piece is yawed to `PlateReading::along` or to `PlateReading::fall`. Poisson
  sampling deliberately destroys alignment, and alignment is the difference
  between decoration that reads as bolted on and decoration that reads as
  confetti.

One thing is decided by a BLOCK of hull rather than by a cell, and it is the
DENSITY NORMALISATION. Every other knob is per plate and they multiply, so a rule
tuned on a 150-plate generated hull put one visible piece on a 20-plate editor
build. `ScatterRule::patch` is a floor: within each block of `patch` cubed cells,
keyed by the out face, a rule that the share left with nothing claims its lowest
hashing eligible plate. A block is a fixed division of the ship's own cells, so a
hull that grows by one cell keeps every piece outside the block it grew into; the
floor never displaces another rule's piece, so priority still means what it says.
With `chance: 0.0` the share picks nothing and the rule is purely "one piece per
block", which is a density that reads the same at any hull size.

A decoration is a `SectionFixture` like a plate, and a child of the PLATE, one
level further out - so a plate shot off takes its greebles and the `damage_tint`
ancestor walk stops at the first fixture it meets whichever of the two it started
under. The base game generates its greeble models from committed JSON recipes
(`scripts/gen-greebles.py`), and the mod-facing format is documented in
[Ship skin styles](../../modding/styles/).

What a hull actually OFFERS is worth knowing before writing a rule, and a
GENERATED hull and a HAND-BUILT one offer almost opposite things. Measured, per
ship:

| subject | plates | flat | step | ridge | peak | bevel | brink | spur |
|---|---|---|---|---|---|---|---|---|
| `wfc_ships` row | 132-162 | 6-22 | 18-22 | 0-4 | 0 | 10-14 | 48-66 | 34-42 |
| editor build | 19-27 | 0 | 2-3 | 3-8 | 1-3 | 0 | 0 | 9-17 |

A hand-built ship has NO flat plate, no bevel and no brink at all: it is spurs,
ridges and studs, because almost every cell of it is one cell wide. So a rule
written for flat panels lands nowhere on the thing the owner actually builds, and
no density normalisation can rescue it - a floor over an empty eligible set is
still empty.

Both `spawn_ship_skin` and the editor's `sync_editor_skin` log that histogram plus
a per-rule `taken of reach` tally at debug, where REACH is everything the rule's
filter and lattice admit before the share and before priority. The two zeroes
mean opposite things: `x0 of 78` is a rule starved by one above it or thinned away
by its own share, and `x0 of 0` is a filter that matches nothing this hull has.

## Integrity: damage -> disable -> destroy

The destruction stack is nova's own, in
`crates/nova_gameplay/src/integrity/`, with the ship adapter in
`crates/nova_ship/src/sections/integrity.rs`. `NovaIntegrityPlugin` composes five
generic pieces:

- `health.rs` - the hit-point store: `Health`, `HealthApplyDamage` and the
  `HealthZeroMarker` its observer adds at zero.
- `core.rs` (`IntegrityCorePlugin`) - the generic disable/destroy core, plus
  the mass-times-velocity impact damage.
- Ship-owned `ShipIntegrityPlugin` - derives the section graph, handles disabled
  sections, rolls section health up to the ship root, and collapses a root that
  falls below its `StructuralCollapseThreshold`.
- `explode.rs` - reacts to destruction: debris, mesh fragments, `OnDestroyedEvent`.
  Destructibility is SEMANTIC (`ExplodableEntity` plus the destroy marker), never
  where a `Mesh3d` sits: a section keeps its gameplay components on a root and
  draws through `SectionRenderOf` descendants, so the geometry walk in
  `mesh/explode.rs` collects the whole subtree. That walk is also the only thing
  that decides whether a body HAS geometry - it reports an empty
  `ExplodeFragments` when it finds none, and the finale reads that one answer to
  choose between real fragments and the generic cube burst, so one death can
  never emit both. The fragment budget is per BODY
  (`BODY_FRAGMENT_BUDGET`), so a multi-part turret costs no more than a hull
  cube. A spaceship root is excluded from fragmenting: its descendants are whole
  sections, each of which bursts as itself.
- `neutralize.rs` - combat-death: fires `OnNeutralized` when a ship stops
  being a threat.

Graph build: every section prototype authors local `link_points` with an id,
position, and outward unit normal. When avian links a collider to its body
(`ColliderOf`), `ShipIntegrityPlugin` transforms those points into ship-root
space. Coincident points with opposed normals become symmetric `ConnectedTo`
neighbor edges. IDs are for diagnostics and UI, not compatibility. A malformed,
ambiguous, or disconnected graph is rejected as a whole; collider contact and
center distance never create fallback edges. `SpaceshipRootMarker` declares the
body as `IntegrityRoot`. Asteroids declare the same root role and give their lone
collider node an empty list, so it is a leaf.

Editor placement mates the same sockets, so the editor cannot build a ship the
graph would reject. `snap_placement` (`nova_ship::sections::link_points`) poses a
part from one mate: the two sockets become coincident and their normals opposed,
which leaves only the ROLL about that axis free - the builder's choice, alongside
which of the part's own sockets does the mating.

That roll has a defined ZERO, and it is what makes one part usable on every
other part. Each socket carries an implied up vector, `link_point_up(normal)`:
ship up (+Y) projected onto the socket's plane, or forward (+Z) on the sockets
that face +-Y. It is DERIVED from the normal rather than authored, so two parts
that never met agree on it without anyone writing a second vector, and
`snap_placement` mates the two socket FRAMES rather than just the two normals.
Aligning normals alone leaves the roll to whichever axis a shortest-arc rotation
happened to sweep about - and for a socket facing exactly opposite the part's
own, to an arbitrary perpendicular.

The other half is the normals themselves. An authoring tool that derives a
socket from part GEOMETRY gets whatever angle the neighbour happened to sit at,
so `cardinal_axis` snaps the derived normal to the nearest axis. It is
antisymmetric (`cardinal_axis(-d) == -cardinal_axis(d)`), so both ends of one
authored edge stay exactly opposed and no existing mate is lost. Without it the
cargob's pod faced its fuselage 36 degrees off -X and anything mated onto that
socket arrived tilted by exactly that much - which is what made parts look like
they only fit the craft they were cut from.

`box_link_points(size)` is the general face-socket helper
(`unit_cube_link_points` is `box_link_points(Vec3::ONE)`). A part authored at its
own size mates against a part of any other size, because the sockets meet face to
face and the roll comes from the axis alone; `pdc_kinetic_turret_section` (and
its `pdc_pierce_turret_section` twin, the same gun with a different round) is the
shipped example - one compact mount that fits every hull, replacing ten per-craft
copies of the same gun.

`candidate_link_point_mates` is
the same pairing WITHOUT the ambiguity and connectivity gates, because a ship
under assembly is legitimately disconnected; the editor uses it to see which
sockets are taken and to refuse a placement that would leave one with two
suitors. Collider bounds enter only as the overlap refusal, under the ship lint's
rule: interpenetration is allowed exactly where a mate says the interface is
intentional.

`scripts/cut-obj-into-parts.py` proposes candidates for freshly cut parts: two
parts whose bounds meet at a seam and overlap across it get one socket each at
the centre of that shared face, written into the part manifest as `link_points`.
A recipe part can author its own list instead (in ship space, like every other
recipe coordinate), which replaces the generated one. They are candidates for a
human to judge - shipped gameplay sockets stay hand-authored in `nova_authoring`.

Damage flow:

1. A hit triggers `HealthApplyDamage` (`nova_gameplay::integrity::health`);
   its observer subtracts the amount and adds `HealthZeroMarker` at zero. The amount also bubbles up `ChildOf`, clamped to
   what the section actually had left - so overkill on one section cannot kill
   the ship (a 1000 hit on a 100 hp section costs the root 100).
2. Zero health -> `IntegrityDisabledMarker`. A depleted ship section is destroyed
   at any graph degree. The leaf rule remains for healthy sections disabled by
   final structural collapse.
3. Destruction prunes the node from its neighbors' lists. If surviving structure
   becomes disconnected, the controller-bearing component keeps ship identity
   and every other component becomes an inert dynamic wreck body.
4. `aggregate_ship_health` keeps the root's `current` equal to the sum of its
   living sections, over a `max` that is PINNED - a running maximum, never
   re-derived from the survivors. A destroyed section despawns, so a live
   denominator would make the HP bar fill up as a ship is shot apart (150/1100
   reading 100/100) and would make any fraction of it rebound. It is a running
   maximum rather than a set-once pin because a ship's sections can land across
   several frames.
5. At or below its `StructuralCollapseThreshold` (`collapse_threshold` on the ship,
   default 0.05) the root gets `StructuralCollapseMarker` and the ship starts
   TEARING ITSELF APART, rather than dying on the spot: `cascade_structural_collapse`
   disables every section still standing and hands them to steps 2 and 3 above.
   The extremities are leaves, so they go first and burst their debris; the
   prune turns their neighbors into leaves, and those go on the next frames.
   The wreck peels from the outside in instead of vanishing - how long that
   takes is the remnant's DEPTH, so a chain peels from both ends over several
   frames while a shallow remnant whose sections are all already leaves goes in
   one. Every section's own debris burst fires either way.
6. The ROOT dies last, and of the same rule. Each destroyed section leaves the
   sum, so step 4 walks `current` down to zero on its own; with no structure
   left the recompute marks the root `HealthZeroMarker` and the ordinary chain
   destroys it, which is what fires `OnDefeated`/`OnDestroyed`. That is also
   the standalone backstop for a last section removed WITHOUT a damage bubble
   (a direct destroy, a detach), which nothing else would mark, and it is what
   threshold `0.0` reduces the whole rule to.

**The no-progress override.** Disabling a section costs it no health - only
DESTRUCTION takes it out of the root's sum - so a remnant with no leaf never
drains. Four hulls mated in a ring each keep two neighbors, none ever becomes a
leaf, nothing is destroyed, `current` never falls and the root never dies: an
immortal disabled hulk. So the leaf rule is treated as a preference for the
ORDER a wreck comes apart in, not a correctness requirement. A cascade tick
that disables nothing new AND does not see the standing-section count fall is a
stall, and the most leaf-like survivor is destroyed whatever its neighbors.
Breaking one node out of a ring leaves a chain, so the ordinary cascade
resumes and the peel is kept everywhere it is possible. Progress is measured as
that count FALLING rather than against a frame budget, because the cascade's own
gaps are irregular while a count that fell is direct evidence a section died.

A severed wreck fragment is persistent until scenario teardown. Its healthy
sections remain damageable, but `SectionInactiveMarker` disconnects every
controller, thruster and weapon from the lost command bus. Fragments inherit
rigid point velocity and receive a momentum-balanced 1 u/s kick away from the
cut. They are unsigned debris, not ships: no allegiance, control, defeat event,
or scenario identity.

A ship is disabled progressively, so a collapsing ship can keep shooting for a
few frames; its weapons stop as their own sections go. That also means the
unified defeat edge usually comes from `neutralize.rs` partway through the
peel, and the root's later destruction fires only `OnDestroyed` - `DefeatedMarker`
is what keeps `OnDefeated` to exactly one.

Structural collapse is a MATERIAL test and stands apart from neutralization
(`neutralize.rs`), which is a CAPABILITY test: a ship can be out of the fight
with a sound hull (a derelict to board, salvage or let limp away), and a ship
can collapse while its guns still work.

The cascade a single section walks through:

```mermaid
flowchart TD
    A[Section takes damage] --> B[Integrity drops]
    B --> C{Zero health?}
    C -->|No| A
    C -->|Yes| F[Destroyed]
    F --> G[Pruned from neighbors]
    G --> H{Graph still connected?}
    H -->|No| P[Detached components become wreck bodies]
    F --> I[Root current re-aggregated over a pinned max]
    I --> J{At or below the collapse threshold?}
    J -->|Yes| K[Every standing section disabled]
    K --> Q{Leaf?}
    Q -->|Yes| F
    Q -->|No| E[Inactive until pruning makes it a leaf]
    K --> L{Nothing destroyed this tick?}
    L -->|Stalled| M[Destroy the most leaf-like anyway]
    M --> G
    I --> N{No sections left?}
    N -->|Yes| O[Ship dead]
```

## Typed damage (`crates/nova_gameplay/src/damage.rs`)

Weapon damage is authored, not emergent from bullet physics, and it is ONE
number: there is no resistance table and no per-section multiplier anywhere in
the damage path. A projectile carries
`ProjectileDamage { amount, power, layers, kind }` with a `DamageType`:
`Kinetic`, `Pierce`, or `Explosive`. `apply_damage` is the single point at which
any weapon enters the health store, and it is a plain `HealthApplyDamage`
trigger - nothing between the weapon and `on_damage` reinterprets the number.

A type is a way of TRAVELLING, not a multiplier. That was the point of dropping
the table: a round visibly crossing three sections is legible from the cockpit,
a 1.5x is not. `SectionClass` survives the table as the ship computer's section
LABEL (`nova_os_ui` reads it for codes, glyphs and descriptions); nothing in the
damage path branches on it.

Turret bullets are given a near-zero physical mass (`NEUTRALIZED_BULLET_MASS`)
so the impact path's mass-times-velocity damage
(`on_impact_collision_deal_damage`, `integrity/core.rs`) is negligible and the
authored amount is the only weapon damage. Torpedoes detonate a `NovaBlast`.
`damage.rs` computes linear falloff from each collider's world centre. For a
target it then walks the centre ray through closer live ship sections: a
survivor stops pressure, while a destroyed section transmits 65 percent. All
blasts collected in one fixed tick read one pre-damage health snapshot.

### Closing speed

Both BULLET types are speed-driven, and the term is computed at the hit, not at
the muzzle: `closing_speed(round_velocity, target_velocity)` projects the same
relative velocity `on_impact_collision_deal_damage` uses onto the round's own
line of flight (projecting onto the line BETWEEN the bodies is unusable - at
contact they are touching, so that direction is noise). Both curves are the
speed ratio against `REFERENCE_CLOSING_SPEED` (100 u/s, the shipped PDC's
`muzzle_speed`), clamped:

- `kinetic_damage_multiplier` scales what a hit DEALS, clamped to `[0.25, 2.0]`;
- `pierce_power_multiplier` scales how far the round GETS - it divides what a
  layer costs - clamped to `[0.5, 3.0]`.

Linear, not the ram model's own curve: `impact_damage` is impulse plus absorbed
energy, and at bullet speeds the quadratic energy half reads ~3.9x at twice the
reference, which would turn a ~400 DPS PDC into ~1600. Both read exactly 1.0 at
the reference, so authored `bullet_damage` values keep the feel they were tuned
for. Speed scaling is deliberately NOT in `apply_damage`: a ram already carries
its velocity in the amount, and a blast has no line of flight.

### The travel rule

`pierce_remainder` (`damage.rs`) is the whole rule, one branch per type;
`spend_piercing_damage` deals `hit_bite` through `apply_damage` and then calls
it.

- KINETIC spends its DAMAGE. `amount` doubles as the budget: a hit that fails to
  destroy the target has by definition put the whole bite into it, so the round
  dies; a hit that destroys it costs only the health that was there, priced back
  through the speed curve that scaled the bite, and the rest flies on. A slug
  can never deal more in total than it was fired with.
- PIERCE spends POWER, never damage. `amount` is flat - the same bite into every
  layer, no speed term and no decay with depth. Crossing a layer costs that
  layer's `Health.max` divided by `pierce_power_multiplier`. MAX, not remaining,
  for two reasons: light plating stays nearly free while a heavy block is
  expensive (the spaced-armour intuition), and softening a section with other
  fire cannot open a cheaper hole through it. A rake's TOTAL damage therefore
  exceeds what it was fired with, which is intended. `PIERCE_BASE_POWER` (300 hp
  of thickness) is the budget and `MAX_PIERCE_LAYERS` (6) the backstop under it,
  because cheap plating alone would not bound the chain.

A target with no `Health` on the hit collider (an asteroid, a planetoid, a pool
that lives on an ancestor) has no thickness to price and nothing provably
destroyed, so it is a wall to both types at any speed. Nothing in the rule knows
what it hit, so destructible cover needs no special case. Torpedoes do not use
it - they detonate on a proximity fuze.

One avian trap the hit callsite has to handle: `CollisionStart` is raised once
per EVENT-ENABLED collider, so a contact with events on both sides arrives
twice with `collider1`/`collider2` swapped. An asymmetric rule must act on one
ordering only (`resolve_bullet_hit` keys on the round being named first;
`on_nova_blast_collision` on the blast being `body1`), or it pays out twice
per contact. A symmetric rule - ram damage - wants both.

## Ammo

- `SectionAmmo` (`sections/ammo.rs`): optional magazine on a weapon section.
  Absent = unlimited fire; `ammo_capacity` in the turret/torpedo config opts in.
  The player `infinite_ammo` flag builds that ship's weapons without magazines,
  but only under the `debug` feature: a shipped build logs a warning and keeps
  the authored magazines, so unlimited fire is a dev cheat, never a player state.
- `SectionReload` (`sections/ammo.rs`): optional idle batch reload on a
  magazine, from the turret/torpedo config `reload: Some((delay, amount))`.
  Every successful shot resets progress; every completed quiet delay restores
  one batch until full. Fire runs before `tick_section_reload` in FixedUpdate so
  a shot wins an exact completion tick. Unlimited weapons never reload. The HUD
  reads `progress()` and `incoming_rounds()` to pulse only the next batch.
- `LoadedBullet` (`sections/turret_section/mod.rs`): the turret's loaded-round slot
  (damage type + amount), seeded from the config. Fired bullets and the HUD ammo
  readout colors read this slot, so swapping ammo types is one component write.
- `DefaultProjectileRender` (`sections/turret_section/render.rs`): the built-in
  round art, ONE mesh + material per `DamageType`, built in `FromWorld`. The
  render observer reads the round's own `ProjectileDamage.kind` and hands out
  clones, because a turret's authored `projectile_render_mesh` is per-TURRET
  while the fired type comes from `LoadedBullet` at runtime. Every shipped
  turret leaves that field `None`, so this IS the shipped path at 100 rounds/s
  per muzzle: it must never allocate per shot, and
  `default_projectile_render_allocates_no_assets_per_shot` pins that. Its meshes
  come from `sections::nose_cone_mesh` (a cylinder and a cone, merged), which
  the torpedo warhead's `DefaultTorpedoRender` shares. The warhead colours ITS copy of that
  mesh from the launched `TorpedoType`'s tint - the material was already per
  projectile (`SectionDamageTint` clones it per section), so per-type colour
  costs nothing the shared mesh handle protects.
