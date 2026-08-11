# Events

Everything that can fire a handler. A handler's `name:` field names one of
the NINE event kinds below, written bare (they are unit variants):
`name: OnStart`, `name: OnEnter`, and so on. When the event fires, the
handler's [filters](../filters/) gate it and its [actions](../actions/) run.

The whole vocabulary at a glance:

| event | payload | fires when |
|---|---|---|
| [`OnStart`](#onstart) | none | once, right after the scenario loads |
| [`OnUpdate`](#onupdate) | none | every frame while live and unpaused |
| [`OnDestroyed`](#ondestroyed) | `id`, `type_name` | a scenario object is destroyed |
| [`OnNeutralized`](#onneutralized) | `id`, `type_name` | an armed ship loses ALL weapons AND thrusters |
| [`OnEnter`](#onenter) | `id`, `other_id`, `other_type_name` | a body enters a trigger area |
| [`OnExit`](#onexit) | `id`, `other_id`, `other_type_name` | a body leaves a trigger area |
| [`OnOrbit`](#onorbit) | `id`, `other_id`, `other_type_name` | a ship holds an autopilot ORBIT (recurs) |
| [`OnTravelLock`](#ontravellock) | `id`, `other_id`, `other_type_name` | the player's travel lock lands (recurs) |
| [`OnCombatLock`](#oncombatlock) | `id`, `other_id`, `other_type_name` | the player's combat lock lands (recurs) |

The payload is what an `Entity` filter can match on: `id` / `type_name` name
the event's SUBJECT, `other_id` / `other_type_name` its other party. Which
entity is which is per-event and listed under each kind below. A filter field
the event does not fill NEVER matches - `other_id` on an `OnDestroyed`
handler can never pass.

## OnStart

Fires exactly once, right after the scenario loads - after every handler
entity exists, so no handler can miss it. Carries no payload; an `Entity`
filter never matches it. This is where a scenario seeds its world: spawn
objects and lights, seed every variable its expression filters will read
(they [fail closed](../filters/#traps-for-the-unwary) on unset variables),
and post the first objective.

```ron
(
    name: OnStart,
    actions: [
        VariableSet((key: "beat", expression: Term(Factor(Literal(Number(0.0)))))),
        // ... spawns, lights, first objective ...
    ],
),
```

## OnUpdate

Fires every frame while the scenario is live and UNPAUSED (frozen behind the
pause menu and the outcome overlay). Carries no payload. The chain order is
guaranteed: the scenario clock ticks, then the player speed updates, then
`OnUpdate` fires - so a time or speed gate always sees this frame's values.

An unfiltered `OnUpdate` handler runs its actions EVERY frame. Always gate it
with `Expression` filters plus a one-shot flag (the
[count-gate idiom](../expressions/#recipes)); this is the workhorse for
clock-driven beats and count thresholds that must not depend on handler
order. Seed `briefing_sent` to `0` in `OnStart`, then write the handler as:

```ron
(
    name: OnUpdate,
    filters: [
        Expression((GreaterThan(
            Term(Factor(Name("scenario_elapsed"))),
            Term(Factor(Literal(Number(10.0)))),
        ))),
        Expression((Equal(
            Term(Factor(Name("briefing_sent"))),
            Term(Factor(Literal(Number(0.0)))),
        ))),
    ],
    actions: [
        VariableSet((
            key: "briefing_sent",
            expression: Term(Factor(Literal(Number(1.0)))),
        )),
        StoryMessage((
            speaker: "Control",
            text: "Ten seconds elapsed.",
        )),
    ],
),
```

## OnDestroyed

Fires when a scenario object is destroyed: an asteroid breaks, or a ship dies
through the section-explosion pipeline. Payload: `id` and `type_name` of the
DESTROYED object; there is no other party.

```ron
(
    name: OnDestroyed,
    filters: [ Entity((type_name: Some("asteroid"))) ],
    actions: [ /* bump a counter */ ],
),
```

Type names are the object-kind constants: `"asteroid"`, `"spaceship"`,
`"beacon"`, `"salvage_crate"`, `"light"` - see
[Scenario objects](../objects/).

## OnNeutralized

Fires when a ship that was ARMED loses all working weapons AND all working
thrusters - combat-dead, hull possibly intact, still in the world (it is NOT
despawned, so no `OnDestroyed` fires with it). Payload: `id`, `type_name` of
the neutralized ship; no other party.

Author kill objectives on BOTH events - `OnDestroyed` beside
`OnNeutralized` - so a beaten ship counts as beaten whether or not the hull
finally cracks.

## OnEnter

Fires when a body's FIRST collider makes contact with a trigger area
(occupancy is refcounted per body, so a multi-section ship fires it once, on
the 0-to-1 transition). Payload: `id` is the AREA; `other_id` /
`other_type_name` are the ENTERING body.

Three things produce trigger areas: the
[`CreateScenarioArea`](../actions/#createscenarioarea) action, a
[`Beacon`](../objects/#beacon) with `area_radius` set, and every
[`SalvageCrate`](../objects/#salvagecrate) (its `area_radius` is the pickup
sensor). All three report under their own id:

Match one area and one specific entering ship:

```ron
(
    name: OnEnter,
    filters: [
        Entity((
            id: Some("safe_zone"),
            other_id: Some("player_spaceship"),
        )),
    ],
    actions: [ /* player arrived */ ],
),
```

Or accept any spaceship that enters that area by filtering the other party's
type:

```ron
(
    name: OnEnter,
    filters: [
        Entity((
            id: Some("repair_zone"),
            other_type_name: Some("spaceship"),
        )),
    ],
    actions: [ /* a ship entered */ ],
),
```

Notes:

- The entering body must itself be a scenario object (carry an id and type
  name) to be reported.
- An area created AROUND a body already inside it still fires `OnEnter` -
  the fresh overlapping pair counts as an entry. So arming a zone late, or
  spawning it on top of the player, works. (This was once not true; do not
  design around the old behavior.)
- `OnEnter` is a plain event, not a state: if the handler's variable gate is
  not open yet when the body enters, the entry is consumed and will NOT
  re-fire when the gate opens later. Place areas so the entry happens after
  the gate opens, or gate on a variable the repeat pulses can re-check
  (`OnUpdate` + occupancy variables you maintain yourself).

## OnExit

The complement: fires when a body's LAST collider leaves the area (the
1-to-0 transition). Same payload shape as `OnEnter`. A body despawned while
inside an area fires NO `OnExit` for itself - its occupancy rows are pruned
silently.

```ron
(
    name: OnExit,
    filters: [
        Entity((
            id: Some("repair_zone"),
            other_id: Some("player_spaceship"),
        )),
    ],
    actions: [ /* player left the repair zone */ ],
),
```

Use `other_type_name: Some("spaceship")` instead of `other_id` when every ship
leaving the area should match.

## OnOrbit

Fires when a ship has HELD an engaged autopilot ORBIT around one gravity
well for the hold window - then RECURS every further window while the hold
continues. Payload: `id` is the WELL being orbited; `other_id` /
`other_type_name` the orbiting ship.

The default window is 5 seconds. Override it per AI ship with
`orbit_hold_secs: Some(8.0)` on the AI controller (only meaningful with its
`orbit` directive); non-positive or non-finite values are a lint error and
fall back to the default. The window is measured on the scenario clock, so
it freezes under pause and resets on retry.

Seed `orbit_confirmed` to `0` in `OnStart` before using this recurring handler:

```ron
(
    name: OnOrbit,
    filters: [
        Entity((
            id: Some("planetoid"),
            other_id: Some("player_spaceship"),
        )),
        Expression((Equal(
            Term(Factor(Name("orbit_confirmed"))),
            Term(Factor(Literal(Number(0.0)))),
        ))),
    ],
    actions: [
        VariableSet((
            key: "orbit_confirmed",
            expression: Term(Factor(Literal(Number(1.0)))),
        )),
        ObjectiveComplete((id: "hold_orbit")),
    ],
),
```

Use `other_type_name: Some("spaceship")` when any ship orbiting that well
should match. Keep the one-shot expression because `OnOrbit` recurs.

## OnTravelLock

Fires when the PLAYER's travel (white, navigation) lock lands on a scenario
object - once on acquisition, then RECURRING every re-fire period while the
lock is held. Payload: `id` is the LOCKED target; `other_id` /
`other_type_name` the locking (player) ship. AI locks never fire it.

The default re-fire period is 5 seconds; override per player ship with
`lock_refire_secs: Some(8.0)` on the Player controller. This example assumes
`surveyed` was seeded to `0` in `OnStart`.

```ron
(
    name: OnTravelLock,
    filters: [
        Entity((
            id: Some("anchorage"),
            other_id: Some("player_spaceship"),
        )),
        Expression((Equal(Term(Factor(Name("surveyed"))), Term(Factor(Literal(Number(0.0))))))),
    ],
    actions: [
        VariableSet((key: "surveyed", expression: Term(Factor(Literal(Number(1.0)))))),
        // ... the survey beat ...
    ],
),
```

## OnCombatLock

Identical contract to `OnTravelLock`, for the player's combat (red) lock -
its own event so a scenario can distinguish "looked at" from "targeted". Seed
`flagship_called_out` to `0` in `OnStart` before using this handler.

```ron
(
    name: OnCombatLock,
    filters: [
        Entity((
            id: Some("enemy_flagship"),
            other_id: Some("player_spaceship"),
        )),
        Expression((Equal(
            Term(Factor(Name("flagship_called_out"))),
            Term(Factor(Literal(Number(0.0)))),
        ))),
    ],
    actions: [
        VariableSet((
            key: "flagship_called_out",
            expression: Term(Factor(Literal(Number(1.0)))),
        )),
        StoryMessage((
            speaker: "Gunner",
            text: "Flagship targeted.",
        )),
    ],
),
```

To react to any combat-locked target, omit `id`; keep `other_id` when the
locking ship must be `player_spaceship`.

## Recurrence is deliberate: gate everything

`OnOrbit` and the two lock events REPEAT while their condition holds (5 s
windows by default). This is by design: a one-shot event consumed
while a beat guard rejects it would be gone for good and soft-lock the
script; recurring events make a rejected pulse harmless - the next pulse
re-checks. The cost: every handler on a recurring event MUST be gated on a
variable it flips (or advances), or its actions re-fire every window. The
one-shot flag pattern is worked in
[Variables & expressions](../expressions/#recipes).

## Dispatch order (what you can rely on)

- Events queue and drain FIFO within a frame; for each event, handlers run
  in AUTHORED order; within a handler, actions run in authored order.
- Actions mutate the event world immediately (a later filter in the same
  frame sees the new variable values), but their WORLD effects (spawns,
  despawns) land together at the end-of-frame sync - and an
  `ObjectiveMarkerAttach` ordered after a `SpawnScenarioObject` in the same
  handler does see the fresh object.
- Never depend on handler order across DIFFERENT events; gate on variables.
