# Actions

Everything a handler can DO. Actions run in authored order once every filter
passes; each is a newtype variant - `Name((field: value, ...))`, double
parens even for one field. Failures warn and continue (a missing target id
never panics a scenario). All 25, grouped by what they touch:

| group | actions |
|---|---|
| [Spawning & the world](#spawning-the-world) | [`SpawnScenarioObject`](#spawnscenarioobject), [`ScatterObjects`](#scatterobjects), [`DespawnScenarioObject`](#despawnscenarioobject), [`CreateScenarioArea`](#createscenarioarea) |
| [Mission & story](#mission-story) | [`Objective`](#objective), [`ObjectiveComplete`](#objectivecomplete), [`ObjectiveMarkerAttach`](#objectivemarkerattach), [`ObjectiveMarkerDetach`](#objectivemarkerdetach), [`StoryMessage`](#storymessage), [`HudReadout`](#hudreadout), [`HintEmphasisSet`](#hintemphasisset), [`HintEmphasisClear`](#hintemphasisclear) |
| [Flow: outcomes & transitions](#flow-outcomes-transitions) | [`Outcome`](#outcome), [`NextScenario`](#nextscenario) |
| [Ship state](#ship-state) | [`SetSpeedCap`](#setspeedcap), [`SetControllerVerb`](#setcontrollerverb), [`SetAllegiance`](#setallegiance), [`ForceTorpedoLaunch`](#forcetorpedolaunch) |
| [Variables, timers & debugging](#variables-timers-debugging) | [`VariableSet`](#variableset), [`TimerStart`](#timerstart), [`TimerCancel`](#timercancel), [`DebugMessage`](#debugmessage) |
| [Camera & photo mode](#camera-photo-mode) | [`SetCamera`](#setcamera), [`Screenshot`](#screenshot), [`SetSkybox`](#setskybox) |

**Scoped targets.** Every by-id action resolves its id ONLY among
scenario-scoped entities (things this scenario spawned); the ship actions
additionally require the id to be a ship ROOT. This is deliberate: ship
SECTIONS carry per-ship ids like `"controller"` too, and an unscoped lookup
would hit every ship's section. Referencing an id before it is spawned warns
and does nothing - spawn first.

## Spawning & the world

### SpawnScenarioObject

Spawn one object. `base` is the shared identity block; `kind` picks the
object and carries its config - the five kinds are the
[Scenario objects reference](../objects/).

| field | type | default | meaning |
|---|---|---|---|
| `base` | object base | required | identity + pose (below) |
| `kind` | object kind | required | `Asteroid((..))` / `Spaceship((..))` / `Beacon((..))` / `SalvageCrate((..))` / `Light(..)` |

The `base` block:

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | the object's scenario id - the address every event, filter and by-id action uses |
| `name` | string | required | display name |
| `position` | 3-tuple | required | world position |
| `rotation` | 4-tuple | required | world rotation quaternion `(x, y, z, w)` |

```ron
SpawnScenarioObject((
    base: (id: "rock_1", name: "Rock", position: (10.0, 0.0, -40.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Asteroid((radius: 5.0, texture: "dep://base/textures/asteroid.png", invulnerable: false)),
)),
```

The same spawn id twice in ONE handler is a lint Error; the same id across
two handlers (an either-or fork) is a Warn.

### ScatterObjects

Spawn `count` copies of a template at deterministic random positions - the
declarative asteroid-field primitive. Copy `i` gets id `"{id_prefix}{i}"`.

| field | type | default | meaning |
|---|---|---|---|
| `id_prefix` | string | required | id prefix for the copies (a filter id starting with this prefix lints clean) |
| `count` | integer | required | copies; runtime cap 4096, absurd counts are a lint Error |
| `seed` | integer | required | RNG seed - the same seed gives the same layout every load. Asteroid templates without an authored `seed` also get deterministic per-rock silhouette seeds derived from it, so the field's shapes are stable too |
| `region` | region | required | sampling volume (below) |
| `template` | object config | required | the object each copy clones (any kind; same `base`/`kind` shape as `SpawnScenarioObject`) |
| `asteroid_radius` | `Option` (lo, hi) | `None` | Asteroid templates only: randomize each rock's radius in `[lo, hi)` |
| `min_separation` | `Option` number | `None` | minimum centre-to-centre distance (world units) against EVERY body scattered so far this scenario, earlier scatters included; 64 placement tries per copy, unplaceable copies are DROPPED, never overlapped |

`region` variants (struct variants - single parens, named fields):

| variant | fields | meaning |
|---|---|---|
| `Box(min: (..), max: (..))` | both required | uniform per axis in `[min, max]` |
| `Ring(center: (..), inner: .., outer: .., y_min: .., y_max: ..)` | `center` defaults to the origin | horizontal annulus: uniform angle, radius in `[inner, outer]`, y in `[y_min, y_max]` |

```ron
ScatterObjects((
    id_prefix: "asteroid_",
    count: 20,
    seed: 433757350076153856,
    region: Box(min: (-100.0, -20.0, -100.0), max: (100.0, 20.0, 100.0)),
    template: (
        base: (id: "asteroid_", name: "Asteroid", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
        kind: Asteroid((radius: 1.0, texture: "dep://base/textures/asteroid.png", invulnerable: false)),
    ),
    asteroid_radius: Some((1.0, 3.0)),
    min_separation: Some(32.0),
)),
```

Set `min_separation` on any field of SOLID bodies: uniform sampling WILL
nest rocks inside each other, and two overlapping dynamic bodies shove apart
violently on the first physics step. Size it as the two widest bodies side
by side - for asteroids that is NOT `radius`: the noise mesh reaches up to
6x the nominal radius. Scatter results are gameplay content and are never
thinned by graphics quality.

### DespawnScenarioObject

Remove the scoped object whose id matches (recursively, whole child
hierarchy). The classic pairing is a salvage crate on pickup.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped object to despawn; missing id warns |

```ron
DespawnScenarioObject((id: "crate_1")),
```

A body despawned while inside a trigger area fires NO `OnExit` for itself.

### CreateScenarioArea

Spawn an invisible spherical SENSOR zone that drives
[`OnEnter`](../events/#onenter) / [`OnExit`](../events/#onexit) under its
id. Works mid-scenario, and works even when created AROUND a body already
inside (the entry still fires).

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | the id `OnEnter`/`OnExit` report as the area |
| `name` | string | required | display name |
| `position` | 3-tuple | required | sphere centre |
| `rotation` | 4-tuple | required | rotation (cosmetic for a sphere) |
| `radius` | number | required | sensor radius, world units |

```ron
CreateScenarioArea((id: "safe_zone", name: "Safe Zone",
    position: (0.0, 0.0, -100.0), rotation: (0.0, 0.0, 0.0, 1.0), radius: 10.0)),
```

Beacons with `area_radius` and salvage crates are their OWN areas - no
separate `CreateScenarioArea` needed for those.

## Mission & story

### Objective

Post a HUD objective. Objectives state goals; comms lines
([`StoryMessage`](#storymessage)) carry voice.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | opaque handle `ObjectiveComplete` uses |
| `message` | string | required | the objective text on the HUD |

```ron
Objective((id: "destroy_asteroids", message: "Objective: Destroy 5 asteroids!")),
```

Re-posting the same id with new text updates the entry in place (the
"recovered N/3" tally trick). A duplicate post otherwise warns.

### ObjectiveComplete

Complete (remove) the HUD objective with this id.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | the objective to complete; a never-posted id warns (and lints) |

```ron
ObjectiveComplete((id: "destroy_asteroids")),
```

### ObjectiveMarkerAttach

Pin the gold HUD marker chip (label + live distance) on a scoped object.
Re-attaching updates the label in place. A despawned target detaches
implicitly.

| field | type | default | meaning |
|---|---|---|---|
| `target_id` | string | required | scoped object to mark; spawn it first |
| `label` | string | required | short chip text ("BEACON 1") |

```ron
ObjectiveMarkerAttach((target_id: "beacon_1", label: "BEACON 1")),
```

### ObjectiveMarkerDetach

Remove that marker.

| field | type | default | meaning |
|---|---|---|---|
| `target_id` | string | required | scoped object to strip; a missing id is quietly fine (detach-after-despawn is legitimate) |

```ron
ObjectiveMarkerDetach((target_id: "beacon_1")),
```

### StoryMessage

A speaker-attributed line for the HUD comms stack (bottom-left, arrival
order, ~8 s hold each, at most three visible). One line per beat is the
style; the queue is the safety net. Scenario-scoped: teardown clears the
log.

| field | type | default | meaning |
|---|---|---|---|
| `speaker` | string | required | the `SPEAKER >` prefix |
| `text` | string | required | the line |
| `dwell` | `Option` number | `None` | per-line hold override in seconds, clamped to [3, 30] (lint warns outside); `Some(12.0)` |
| `icon` | `Option` asset ref | `None` | speaker portrait (`Some("self://icons/okono.png")`); omitted = the cockpit fallback tile |

```ron
StoryMessage((speaker: "Foreman Okono", text: "Strip it clean, Kestrel.", dwell: Some(12.0))),
```

Two story lines in one handler is a lint Warn (unreadable); a story line
beside an [`Outcome`](#outcome) in one handler is a Warn (frozen behind the
overlay). Let the outcome's own `message` carry the closing line.

### HudReadout

Bind a live HUD readout to a scenario variable - the DISPLAY half of the
variable vocabulary (a run clock, a score, a countdown). One fire is
enough; the readout tracks the variable's current value every frame
thereafter. It freezes under pause and behind the outcome overlay because
the variable does - a time-trial's final time simply holds through the
banner. Cleared at teardown.

| field | type | default | meaning |
|---|---|---|---|
| `slot` | string | required | stable readout id; several slots run side by side; fire again to update or clear |
| `variable` | string | required | the variable shown (e.g. `"scenario_elapsed"`); undefined or non-numeric reads 0.0 |
| `format` | format | `Number` | `Number` (one decimal), `Integer` (rounded), `Time` (`mm:ss.s`) |
| `label` | `Option` string | `None` | caption before the value (`Some("TIME")`) |
| `visible` | bool | `true` | `true` shows/updates; `false` clears exactly this slot |

```ron
HudReadout((slot: "run_timer", variable: "scenario_elapsed", format: Time, label: Some("TIME"))),
```

### HintEmphasisSet

Pulse one keybind-dock chip gold. The dock normally hides verbs the player
cannot use yet, so emphasizing an unavailable verb REVEALS its chip dimmed
and pulses it - how a tutorial points at a key before granting it. Emphasis
never grants the verb ([`SetControllerVerb`](#setcontrollerverb) does).

| field | type | default | meaning |
|---|---|---|---|
| `verb` | string | required | one of `"STOP"`, `"GOTO"`, `"ORBIT"`, `"CANCEL"`, `"RADAR"`, `"COMPONENT"`, `"RCS"`; unknown verbs warn and do nothing |

```ron
HintEmphasisSet((verb: "RADAR")),
```

### HintEmphasisClear

Drop the gold emphasis on one chip (teardown clears all emphasis anyway).

| field | type | default | meaning |
|---|---|---|---|
| `verb` | string | required | the chip to un-pulse |

```ron
HintEmphasisClear((verb: "RADAR")),
```

## Flow: outcomes & transitions

### Outcome

Declare the scenario's win or lose: the gold VICTORY / red DEFEAT banner,
an optional message, and buttons - and it freezes the simulation behind it
exactly like the pause menu until it clears. PRESENTATION ONLY: compose the
consequence beside it. Queue a lingering [`NextScenario`](#nextscenario)
and the overlay offers Continue (Victory) or Retry (Defeat); queue nothing
and it offers only Main Menu.

| field | type | default | meaning |
|---|---|---|---|
| `outcome` | kind | required | `Victory` or `Defeat` |
| `message` | `Option` string | `None` | the line under the banner; strict RON `Some("...")` |
| `auto_advance_secs` | `Option` number | `None` | timed banner: advance the queued LINGERING switch after this many REAL seconds (the player can still click sooner); cap 300 s; meaningless without a lingering switch queued |

```ron
Outcome((outcome: Defeat, message: Some("The convoy is lost."))),
NextScenario((scenario_id: "lifeline", linger: true)),
```

The paired switch must LINGER: an instant switch tears the scenario down
the same frame and SWALLOWS the banner (lint warns on that composition).

### NextScenario

Queue a switch to another scenario by id. Three gears:

- **Hard cut** - `linger: false`, no delay: switches on the next sync.
  Menu-scene plumbing.
- **Delayed cut** - `linger: false, delay: Some(4.0)`: the world keeps
  playing for the delay (a story line can land), then cuts. Ticks on
  pause-frozen time; Enter skips the wait.
- **Modal hold** - `linger: true`: waits for the scenario-advance input
  (Enter / DPadDown) or the outcome overlay's Continue/Retry.

| field | type | default | meaning |
|---|---|---|---|
| `scenario_id` | string | required | target scenario; a dangling id is a lint Error |
| `linger` | bool | required | `true` defers until released; `false` switches now (or after `delay`) |
| `delay` | `Option` number | `None` | delayed non-lingering cut, seconds; cap 300 s, lint warns above 60; meaningless with `linger: true` |

```ron
NextScenario((scenario_id: "asteroid_next", linger: true)),
```

The last request wins wholesale - a fresh `NextScenario` replaces a queued
one and resets its clock.

## Ship state

### SetSpeedCap

Install, update or remove the soft manual-speed governor on a scoped ship.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root |
| `cap` | `Option` number | `None` | `Some(25.0)` installs/updates the cap (u/s); `None` or omitted REMOVES it |

```ron
SetSpeedCap((id: "player_spaceship", cap: Some(25.0))),
SetSpeedCap((id: "player_spaceship"))              // release the governor
```

### SetControllerVerb

Grant or withhold one flight verb on a scoped ship's controller - the
tutorial-progression primitive (the Shakedown Run starts with `Goto`
withheld and grants it at the beacon).

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root |
| `verb` | verb | required | `Stop` / `Goto` / `Orbit` / `Lock` / `Rcs` / `PointDefense` (bare enum, no quotes). Convention: never withhold `Stop` - an engaged autopilot should always be cancelable |
| `enabled` | bool | required | `true` grants, `false` withholds |

```ron
SetControllerVerb((id: "player_spaceship", verb: Goto, enabled: true)),
// The battery answers a salvo by itself - until this takes it away.
SetControllerVerb((id: "player_spaceship", verb: PointDefense, enabled: false)),
```

The spawn-time twin is the `DisableVerb` section modification (see
[Spaceship](../objects/#spaceship)); this action is its runtime mirror.

### SetAllegiance

Overwrite a scoped ship's side at runtime - the neutral-until-provoked
primitive: spawn a bystander `Neutral`, flip it `Enemy` on a trigger.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root; a dangling id is a lint Error |
| `allegiance` | side | required | `Player` / `Enemy` / `Neutral` (bare enum) |

```ron
SetAllegiance((id: "magpie", allegiance: Enemy)),
```

### ForceTorpedoLaunch

Order a scoped ship's torpedo bays to launch at a named target - the
scripted counterpart of the AI's launch decision, for controller-less
emplacements fired by timers ("the battery shoots every N seconds"). Every
torpedo bay on the ship gets a one-shot order; each bay's own cooldown and
ammo still time the actual launch, and the ordnance is committed to the
target like an AI launch (so hostile point defense can engage it). The AI's
launch gates (range envelope, hull-forward cone, line of sight, the 10 s AI
cadence) do NOT apply - the script is the decision.

A missing target skips the launch entirely (no dumb-fire duds while the
target is mid-respawn). On an AI-controlled ship the AI rewrites the bay
trigger every frame and wins; give scripted batteries
`controller: None`.

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root whose bays launch; a dangling id is a lint Error |
| `target` | string | required | scoped ship root the ordnance homes on; a dangling id is a lint Error |

```ron
ForceTorpedoLaunch((id: "battery_west", target: "patrol_ship")),
```

## Variables, timers & debugging

### TimerStart

Start a keyed scenario timer. Starting an existing key restarts its deadline.
The duration is a numeric expression and must evaluate to a positive finite
number. Invalid values log an error and leave an existing timer unchanged.

| field | type | default | meaning |
|---|---|---|---|
| `key` | string | required | scenario-local timer key |
| `seconds` | numeric expression | required | live, unpaused seconds until `OnTimerEnd` |

```ron
TimerStart((
    key: "orbit_hold",
    seconds: Term(Factor(Literal(Number(8.0)))),
)),
```

Timers freeze under pause and clear on retry or teardown. Use an
[`OnTimerEnd`](../events/#ontimerend) handler with a
[`Timer`](../filters/#timer) filter to react once.

### TimerCancel

Cancel a running timer. A missing key is a no-op.

```ron
TimerCancel((key: "orbit_hold")),
```

### VariableSet

Evaluate an expression against the CURRENT variables and store the result -
the write half of the whole
[variable vocabulary](../expressions/). Re-evaluated per event, so
`n = n + 1` accumulates.

| field | type | default | meaning |
|---|---|---|---|
| `key` | string | required | mutable variable to write (overwrites); writing a watched variable is a lint ERROR |
| `expression` | expression node | required | see the [expression grammar](../expressions/); an evaluation error skips the write |

```ron
VariableSet((
    key: "asteroids_destroyed",
    expression: Add(Factor(Name("asteroids_destroyed")), Term(Factor(Literal(Number(1.0))))),
)),
```

### DebugMessage

Log a line (debug builds; run with `--features dev` while iterating). No
game effect - sprinkle these to watch handlers fire.

| field | type | default | meaning |
|---|---|---|---|
| `message` | string | required | the log line |

```ron
DebugMessage((message: "gate 3 armed")),
```

## Camera & photo mode

### SetCamera

Pin the scenario camera at a pose (drops free-fly control; the pose is
re-enforced every frame). Part of the screenshot/photo surface; no scenario
camera present is a warn no-op.

| field | type | default | meaning |
|---|---|---|---|
| `position` | 3-tuple | required | world camera position |
| `look_at` | 3-tuple | required | world point to face (up is +Y) |

```ron
SetCamera((position: (0.0, 30.0, 80.0), look_at: (0.0, 0.0, 0.0))),
```

### Screenshot

Capture the primary window to a PNG. Relative paths land under the
`NOVA_SHOT_DIR` env var when set; parent directories are created. A dev
tool - pair `SetCamera` + settle frames + `Screenshot` to script a framed
shot.

| field | type | default | meaning |
|---|---|---|---|
| `path` | string | required | output PNG path |

```ron
Screenshot((path: "shots/my_scene.png")),
```

### SetSkybox

Swap the scenario's skybox mid-scenario. The install is deferred until the
new image has loaded; a failed load leaves the sky unchanged (warned).

| field | type | default | meaning |
|---|---|---|---|
| `cubemap` | asset ref | required | the new cubemap path (`"self://textures/nebula.png"`) |
| `brightness` | `Option` number | `None` | multiplier; `None` keeps the current brightness (initial scenario default 1000) |

```ron
SetSkybox((cubemap: "self://textures/nebula.png", brightness: Some(700.0))),
```
