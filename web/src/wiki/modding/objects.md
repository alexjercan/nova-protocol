# Scenario objects

Everything a scenario can place in the world. An object is spawned by
[`SpawnScenarioObject`](../actions/#spawnscenarioobject) (or in bulk by
[`ScatterObjects`](../actions/#scatterobjects)): a shared `base` block plus a
`kind` that picks one of the SIX kinds below. Every object gets the base's
id, name and pose, is scenario-scoped (teardown removes it), and carries a
type name the `type_name` filters match:

| kind | type name | body | what it is |
|---|---|---|---|
| [`Anchor`](#anchor) | `"anchor"` | static | invisible authored gravity well (framing/orbit target) |
| [`Asteroid`](#asteroid) | `"asteroid"` | dynamic | destructible rock, optional gravity well |
| [`Spaceship`](#spaceship) | `"spaceship"` | dynamic | a multi-section ship, player- or AI-flown |
| [`Beacon`](#beacon) | `"beacon"` | static | lockable nav marker with a HUD chip |
| [`SalvageCrate`](#salvagecrate) | `"salvage_crate"` | static | fly-through pickup |
| [`Light`](#light) | `"light"` | static | the scene's own lighting |

(Trigger AREAS are spawned by the
[`CreateScenarioArea`](../actions/#createscenarioarea) action rather than as
an object kind - and beacons and crates can be their own areas, below.)

## Anchor

An invisible authored point that publishes a [gravity
well](#asteroid) with an AUTHORED radius: no mesh, no collider, and no
geometric extent for AI [obstacle avoidance](#the-controller) to steer
around. Use it where a contract needs a position plus well geometry but the
scene does not want a rock there - an orbit directive's target, or a real
gravity source with no body. Because the radius is authored (an asteroid's
is derived from its generated mesh), everything reading the well sees the
same geometry on every load.

| field | type | default | meaning |
|---|---|---|---|
| `body_radius` | number | required | the well's published body radius, world units |
| `mass` | `Option` number | `None` | the gravity parameter mu (u^3/s^2), same unit as an asteroid's `mass`. `None` = a zero-strength well: it frames and anchors but never pulls |

```ron
SpawnScenarioObject((
    base: (id: "patrol_anchor", name: "Patrol Anchor", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Anchor((
        body_radius: 80.0,
        mass: Some(30000.0),
    )),
)),
```

An anchor is indestructible (there is nothing to hit) and static; it never
fires destruction events.

## Asteroid

A noise-generated destructible rock. `radius` drives the mesh, collider,
default mass and radar signature together.

| field | type | default | meaning |
|---|---|---|---|
| `radius` | number | required | nominal radius in world units. The true mesh extent reaches up to 6x this (matters for [`min_separation`](../actions/#scatterobjects)) |
| `texture` | asset ref | required | surface texture (`dep://base/textures/asteroid.png` is the stock rock) |
| `health` | number | required | hit points; ignored when `invulnerable` |
| `invulnerable` | bool | required | `true` = no health node at all: can never be destroyed or disabled, so its gravity well can never die mid-scenario |
| `mass` | `Option` number | `None` | the gravity parameter mu (u^3/s^2). `Some` ALWAYS makes this rock a well: pull is `a = mu / r^2`; size it by the sphere of influence you want (`mu = soi_cutoff_accel * soi^2`). `None` = the global rule (a default mass only if the radius qualifies it as a well) |
| `impact_sound` | `Option` asset ref | `None` | played on hit (`Some("dep://base/sounds/impact.wav")`); omitted = silent |
| `destroy_sound` | `Option` asset ref | `None` | played on destruction; omitted = silent |
| `lock_signature` | `Option` number | `None` | radar signature override; `None` = the radius (big rocks lock far) |
| `seed` | `Option` number | `None` | silhouette seed. `Some` pins the generated shape (and the derived geometric extent) across runs; `None` = a fresh silhouette per spawn. [`ScatterObjects`](../actions/#scatterobjects) fills it deterministically from its own seed |

```ron
SpawnScenarioObject((
    base: (id: "planetoid", name: "Planetoid", position: (250.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Asteroid((
        radius: 20.0,
        texture: "dep://base/textures/asteroid.png",
        health: 2000.0,
        mass: Some(45000.0),
        invulnerable: true,
    )),
)),
```

Destruction fires [`OnDestroyed`](../events/#ondestroyed) with the rock's
id and `"asteroid"`.

## Spaceship

A spawn of a SHIP: where it sits, who flies it, which side it is on. What it
IS - its section layout, its cladding - is a [ship](../ships/), named here by
id or authored inline.

| field | type | default | meaning |
|---|---|---|---|
| `hull` | hull source | required | `Prototype("cargoa")` names a [ship](../ships/) by id; `Inline((..))` carries a one-off hull (below) |
| `controller` | controller | required | who flies it (below) |
| `allegiance` | `Option` side | `None` | side override, strict RON `Some(Neutral)`. Omitted = the controller default: Player ships fight for the player, AI ships are hostile |
| `modifications` | list | `[]` | per-spawn deltas over the shared hull: `(section: "fuselage", modifications: [SetHealth(500.0)])`. Applied AFTER the section's own list, so the spawn wins. A section id the hull does not carry is a lint error |

`hull: Inline((..))` carries the same fields a [ship](../ships/)'s own `hull`
does - `sections`, `collapse_threshold`, `skin`, `style`. Author one for a
genuine one-off (a scripted battery that is a single torpedo tube); anything a
second scenario would spawn belongs in the ship catalog.

An abbreviated player ship:

```ron
SpawnScenarioObject((
    base: (
        id: "player_spaceship",
        name: "Player Ship",
        position: (0.0, 0.0, 0.0),
        rotation: (0.0, 0.0, 0.0, 1.0),
    ),
    kind: Spaceship((
        controller: Player((
            input_mapping: {
                "turret_port": [Mouse(Left)],
            },
            infinite_ammo: false,
        )),
        // The shipped corvette, by id.
        hull: Prototype("cargoa"),
        // This spawn's own flight computer is hardened; every other cargoa
        // is untouched.
        modifications: [
            (section: "fuselage", modifications: [SetHealth(500.0)]),
        ],
    )),
)),
```

A one-off hull, authored inline:

```ron
kind: Spaceship((
    controller: None,
    hull: Inline((
        sections: [
            (
                id: "bay",
                position: (0.0, 0.0, 0.0),
                rotation: (0.0, 0.0, 0.0, 1.0),
                source: Prototype("torpedo_section"),
            ),
        ],
    )),
)),
```

An abbreviated AI ship:

```ron
SpawnScenarioObject((
    base: (
        id: "raider_1",
        name: "Raider",
        position: (0.0, 0.0, -300.0),
        rotation: (0.0, 0.0, 0.0, 1.0),
    ),
    kind: Spaceship((
        controller: AI((
            patrol: [(0.0, 0.0, -300.0), (80.0, 0.0, -220.0)],
            engage_delay: Some(8.0),
        )),
        hull: Prototype("cargoa_raider"),
    )),
)),
```

A ship id nothing authored spawns an empty root and logs an error rather than
crashing, so a missing dependency is visible instead of fatal.

### The controller

| variant | meaning |
|---|---|
| `None` | nobody drives; the ship station-keeps |
| `Player((..))` | human-driven |
| `AI((..))` | bot-driven |

`Player((..))` fields:

| field | type | default | meaning |
|---|---|---|---|
| `input_mapping` | map | `{}` | per-SECTION bindings, keyed by section id: `{ "turret_port": [ Mouse(Left) ] }`. Values are `Keyboard(<KeyCode>)` / `Mouse(<MouseButton>)` / `Gamepad(<GamepadButton>)` - modifier-free buttons only |
| `speed_cap` | `Option` number | `None` | soft manual-speed cap in u/s (the Shakedown starts at `Some(25.0)`); `None` = unbounded. Runtime mirror: [`SetSpeedCap`](../actions/#setspeedcap) |
| `infinite_ammo` | bool | required in shipped RON | DEBUG-ONLY CHEAT: weapons built without magazines - never run dry. Only a `debug` build honors it; the shipped game warns and keeps the authored magazines, so author `false` and balance the scenario around real ammunition |

`AI((..))` fields:

| field | type | default | meaning |
|---|---|---|---|
| `patrol` | list of 3-tuples | `[]` | waypoint loop while nothing hostile is detected; empty = station-keep. Legs blocked by a sized body (an asteroid's geometric radius) are flown around automatically, so routes need not measure every rock |
| `orbit` | `Option` string | `None` | id of a gravity-well object to orbit passively. Precedence: orbit > patrol > idle |
| `engage_range` | `Option` number | `None` | hostile-detection override (world units): a passive ship leaves its routine for a hostile inside this range instead of the 400 u default. Wide = a long-watch emplacement that wakes for targets nothing else detects; short = a ship that ignores a nearby brawl |
| `pd_range` | `Option` number | `None` | point-defense override (world units): the guns hold fire until an inbound hostile torpedo is inside this range instead of the 150 u default. Short = staged close-in intercepts; past the turret's ~180 u reach it just wastes the opening shots |
| `waypoint_slack` | `Option` number | `None` | patrol arrival slack override (world units) on top of the arrival standoff; the default is 25. Small = the ship turns onto the next leg closer to each waypoint. Below ~2 risks stalling outside the advance gate - author small, not zero |
| `arrival_standoff` | `Option` number | `None` | how far from a GOTO goal this ship's computer comes to rest, instead of the engine's 50 u default. Pair a small standoff with a small `waypoint_slack` so a nav ship visibly REACHES its marks (the patrol turns at `standoff + slack`) |
| `leash` | `Option` number | `None` | territorial tether radius; combat breaks off beyond it; `None` = chases freely |
| `engage_delay` | `Option` number | `None` | arrival grace in seconds: flies its passive routine and refuses to engage until it elapses; being SHOT ends the grace instantly and permanently. The telegraphed-arrival tool |

An UNARMED AI ship (no turret or torpedo section) is automatically a
NON-COMBATANT: it flies its routine and never acquires, chases or shoots -
no field to set. Keep its `allegiance` on the side you want hunted and it
becomes something to defend (the Lifeline convoy).

Allegiance values: `Player` / `Enemy` / `Neutral`. Player and Enemy are
mutually hostile; Neutral relates neutrally to everyone (stray blast damage
still hurts it). Runtime flip:
[`SetAllegiance`](../actions/#setallegiance).

### The sections list

Each entry places one section in continuous ship-root space:

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scenario-local section id; keys `input_mapping` (shipped ships use semantic ids such as `"turret_port"`) |
| `position` | 3-tuple | required | continuous offset from the ship root |
| `rotation` | 4-tuple | required | rotation relative to the root; structural link points rotate with the section |
| `source` | source | required | `Prototype("<id>")` - a [catalog id](../base-content/#section-prototypes), the compact reusable form - or `Inline((..))` with a full section config ([Ship sections for mods](../sections/)) |
| `modifications` | list | `[]` | spawn-time deltas (below) |

Section modifications - closed, data-only deltas applied at spawn:

| variant | payload | meaning |
|---|---|---|
| `DisableVerb(<verb>)` | `Stop`/`Goto`/`Orbit`/`Lock`/`Rcs`/`PointDefense` | withhold a flight verb from birth (controller sections; multiple accumulate). Runtime mirror: [`SetControllerVerb`](../actions/#setcontrollerverb) |
| `SetHealth(<number>)` | starting health | override the section's health (current and max) |
| `Rename(<string>)` | new name | rename the section entity |
| `SetAmmo(<number>)` | rounds | HARD magazine: override the weapon's rounds AND strip its auto-reload - when they are gone the section is dry for good. Inert on a section with no magazine |

```ron
(id: "fuselage", position: (0.0, 0.7, 0.1), rotation: (0.0, 0.0, 0.0, 1.0),
 source: Prototype("racer_fuselage"),
 modifications: [ DisableVerb(Goto), DisableVerb(Orbit) ]),
```

Ship structure is linted from authoritative link-point mates. A multi-section
ship must form one graph. Collider AABB overlap is an Error unless the two
sections directly mate, which permits intentional interlocking parts while
still catching accidental embedding.

## Beacon

A static, lockable, blinking nav marker with an automatic HUD chip (label,
live distance, edge-clamped direction cue).

| field | type | default | meaning |
|---|---|---|---|
| `label` | string | required | HUD chip text ("BEACON 1") |
| `radius` | number | required | visual orb radius, world units |
| `color` | color | required | orb + emissive tint, tagged: `Srgba((red: 0.3, green: 0.9, blue: 1.0, alpha: 1.0))` |
| `area_radius` | `Option` number | `None` | when set, the beacon IS its own trigger area of this radius - [`OnEnter`](../events/#onenter)/`OnExit` fire under the beacon's id, no `CreateScenarioArea` needed |
| `lock_signature` | `Option` number | `None` | radar signature override; default 20 (about a 600 u lock range) - author bigger for longer GOTO legs |

```ron
SpawnScenarioObject((
    base: (id: "beacon_1", name: "BEACON 1", position: (0.0, 0.0, -350.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Beacon((
        label: "BEACON 1",
        radius: 2.0,
        color: Srgba((red: 0.3, green: 0.9, blue: 1.0, alpha: 1.0)),
        area_radius: Some(70.0),
    )),
)),
```

## SalvageCrate

A minimal fly-through pickup: a static tumbling prop that is its own
trigger area. There is no inventory system - "collected" is scenario state
you author: an `OnEnter` handler under the crate's id, paired with
[`DespawnScenarioObject`](../actions/#despawnscenarioobject) and a counter
[`VariableSet`](../actions/#variableset). The HUD brackets it
automatically.

| field | type | default | meaning |
|---|---|---|---|
| `size` | number | required | visible box edge length, world units |
| `area_radius` | number | required | the pickup sensor sphere ("collected" distance) |
| `pickup_sound` | `Option` asset ref | `None` | the collection ding, player pickups only (`Some("dep://base/sounds/salvage_pickup.wav")` is the stock one); omitted = silent |

```ron
SpawnScenarioObject((
    base: (id: "crate_1", name: "Supply Pod", position: (40.0, 5.0, -60.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: SalvageCrate((size: 1.5, area_radius: 8.0, pickup_sound: Some("dep://base/sounds/salvage_pickup.wav"))),
)),
```

## Light

The scene's own lighting - and it is load-bearing: the engine spawns NO
default light, so **a scenario with no `Light` object renders black.**
`Light` takes one of two methods as an inner enum with NAMED fields in
single parens (not the newtype double-paren shape).

`Directional` - a sun: parallel rays, direction only. The key/rim/fill
workhorse.

| field | type | default | meaning |
|---|---|---|---|
| `illuminance` | number | required | lux (the shipped key lights run ~11000) |
| `color` | color | required | tagged `Srgba((..))` |
| `shadows` | bool | required | shadow casting; convention: exactly ONE caster per scene |
| `aim` | `Option` 3-tuple | `None` | point the light AT this world position, ignoring `base.rotation` (hand-authoring an aim quaternion is impractical); `None` uses the rotation |

`Point` - a positional lamp with falloff: a star, a floodlight, a nebula
glow.

| field | type | default | meaning |
|---|---|---|---|
| `intensity` | number | required | lumens; needs tuning by eye against your scene scale (~2.5M for a 200 u yard lamp) |
| `range` | number | required | contribution cutoff distance, world units |
| `radius` | number | required | source radius (softens the terminator) |
| `color` | color | required | tagged `Srgba((..))` |
| `shadows` | bool | required | shadow casting |

```ron
SpawnScenarioObject((
    base: (id: "key", name: "Key Light", position: (-60.0, 50.0, 60.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Light(Directional(
        illuminance: 11000.0,
        color: Srgba((red: 1.0, green: 0.96, blue: 0.9, alpha: 1.0)),
        shadows: true,
        aim: Some((0.0, 0.0, 0.0)),
    )),
)),
```

The shipped scenes all use the same three-point rig - an 11000 lux warm key
(the only shadow caster), a 16000 lux cold rim from behind, and a 2600 lux cool
fill from the shadow side. Copy the full light blocks from
`assets/mods/example/example.content.ron`.

## Traps for the unwary

- A despawned object fires no `OnExit` for itself, and a beacon/crate area
  dies with its object.
- Dynamic bodies (asteroids, ships) spawned overlapping shove apart
  violently on the first physics step - keep spawns separated (see
  `min_separation` under
  [`ScatterObjects`](../actions/#scatterobjects)).
- Every object id must be unique among live scoped entities; duplicate
  spawn ids in one handler are a lint Error.
- Ships are verbose; scatter is seeded. If you are typing a 400-line ship
  by hand, stop and reference [prototypes](../base-content/) instead.
