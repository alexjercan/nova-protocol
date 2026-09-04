# Events

Everything that can fire a handler. A handler's `name:` field names one of
the TWENTY-THREE event kinds below, written bare (they are unit variants):
`name: OnStart`, `name: OnEnter`, and so on. When the event fires, the
handler's [filters](../filters/) gate it and its [actions](../actions/) run.

A handler that describes a beat happening ONCE says
[`once: true`](../scenarios/#once-a-beat-that-happens-one-time) beside its
name, and the engine retires it the first time its filters pass. Every
repeating event below composes with it.

An event name also appears inside a
[`Sequence`](../actions/#sequence) step's `until` gate, where it names the
event a paced beat waits for. The filters that qualify it are the same ones.

The whole vocabulary at a glance:

| event | payload | fires when |
|---|---|---|
| [`OnStart`](#onstart) | none | once, right after the scenario loads |
| [`OnUpdate`](#onupdate) | none | every frame while live, unpaused and fully spawned |
| [`OnTimerEnd`](#ontimerend) | `key` | a keyed scenario timer ends |
| [`OnDefeated`](#ondefeated) | `id`, `type_name` | a ship is neutralized or directly destroyed |
| [`OnDestroyed`](#ondestroyed) | `id`, `type_name` | a scenario object is physically destroyed |
| [`OnNeutralized`](#onneutralized) | `id`, `type_name` | an armed ship loses ALL weapons, or the flight computer it had |
| [`OnEnter`](#onenter) | `id`, `other_id`, `other_type_name` | a body enters a trigger area |
| [`OnExit`](#onexit) | `id`, `other_id`, `other_type_name` | a body leaves a trigger area |
| [`OnGotoComplete`](#player-maneuver-completion) | `id`, `other_id`, `other_type_name` | the player's GOTO reaches its target and stops |
| [`OnStopComplete`](#player-maneuver-completion) | `id`, `type_name` | the player's STOP comes to rest |
| [`OnOrbitStart`](#orbit-lifecycle) | `id`, `other_id`, `other_type_name` | an ORBIT maneuver starts |
| [`OnOrbitStable`](#orbit-lifecycle) | `id`, `other_id`, `other_type_name` | ORBIT enters stable station-keeping |
| [`OnOrbitLap`](#orbit-lifecycle) | `id`, `other_id`, `other_type_name` | one net stable revolution completes |
| [`OnOrbitUnstable`](#orbit-lifecycle) | `id`, `other_id`, `other_type_name` | stable station-keeping is lost |
| [`OnOrbitEnd`](#orbit-lifecycle) | `id`, `other_id`, `other_type_name` | a surviving ship ends ORBIT |
| [`OnTravelLockStart`](#lock-lifecycle) | `id`, `other_id`, `other_type_name` | the player's travel lock lands |
| [`OnTravelLockEnd`](#lock-lifecycle) | `id`, `other_id`, `other_type_name` | the player's travel lock leaves |
| [`OnCombatLockStart`](#lock-lifecycle) | `id`, `other_id`, `other_type_name` | the player's combat lock lands |
| [`OnCombatLockEnd`](#lock-lifecycle) | `id`, `other_id`, `other_type_name` | the player's combat lock leaves |
| [`OnShipOrderComplete`](#onshipordercomplete) | `order`, `kind`, `id`, `type_name` | a helm order reaches what it was told to reach |
| [`OnShipOrderInterrupted`](#onshiporderinterrupted) | `order`, `kind`, `id`, `type_name` | an AI ship breaks off its order to fight |
| [`OnShipOrderResumed`](#onshiporderresumed) | `order`, `kind`, `id`, `type_name` | that ship picks the same order back up |
| [`OnShipOrderCanceled`](#onshipordercanceled) | `order`, `kind`, `id`, `type_name` | an unfinished order is cleared or replaced |
| [`OnShipOrderFailed`](#onshiporderfailed) | `order`, `kind`, `id`, `type_name` | an order can no longer be flown |

Entity payload fields are what an `Entity` filter can match: `id` /
`type_name` name the event's SUBJECT, `other_id` / `other_type_name` its other
party. `OnTimerEnd` instead carries `key`, matched by a `Timer` filter, and the five
ship-order events carry `order` / `kind` for a
[`ShipOrder`](../filters/#shiporder) filter beside the ship's own `id` /
`type_name` - the same four fields on all five, so one filter matches
whichever of them you hang it on. Which entity is which is per-event and listed below. A filter
field the event does not fill NEVER matches - `other_id` on an `OnDestroyed`
handler can never pass.

## OnStart

Fires exactly once, right after the scenario loads - after every handler
entity exists, so no handler can miss it. Carries no payload; this is where a
scenario seeds its world: spawns, lights, variable seeds, the first objective.

```ron
(
    name: OnStart,
    actions: [
        VariableSet((key: "beat", expression: Term(Factor(Literal(Number(0.0)))))),
        // ... spawns, lights, first objective ...
    ],
),
```

<details class="explain">
<summary>Show explanation</summary>

An `Entity` filter never matches `OnStart` (no payload). Seed every variable
your expression filters will read (they
[fail closed](../filters/#traps-for-the-unwary) on unset variables), and post
the first objective here.

Its spawns arrive over the next few frames, and the scenario is HELD until they
have: no other handler runs, the scenario clock does not tick, and the LOADING
panel stays up. So the first `OnUpdate` after `OnStart` already sees every
object `OnStart` asked for - a count gate cannot read a half-built world.

</details>

## OnUpdate

Fires every frame while the scenario is live and UNPAUSED (frozen behind the
pause menu and the outcome overlay) and every object it asked for exists.
Carries no payload - and an unfiltered `OnUpdate` handler runs its actions
EVERY frame, so always gate it.

<details class="explain">
<summary>Show explanation</summary>

The chain order is guaranteed: the scenario clock ticks, typed queries and
watches update, ended timers fire, then `OnUpdate` fires. Query-backed gates
see one coherent frame snapshot.

This is the workhorse for clock-driven beats and count thresholds that must
not depend on handler order. Gate it with `Expression` filters, and add
[`once: true`](../scenarios/#once-a-beat-that-happens-one-time) when the beat
happens one time - then the only filter left is the one about the game:

```ron
(
    name: OnUpdate,
    once: true,
    filters: [
        Expression((GreaterThan(
            Term(Factor(Name("scenario_elapsed"))),
            Term(Factor(Literal(Number(10.0)))),
        ))),
    ],
    actions: [
        StoryMessage((
            speaker: "Control",
            text: "Ten seconds elapsed.",
        )),
    ],
),
```

A beat that genuinely repeats leaves `once` off and re-arms itself in its own
actions - see the [recipes](../expressions/#recipes).

</details>

## OnTimerEnd

Fires exactly once when a keyed scenario timer reaches its deadline. Payload:
`key` is the scenario-local timer key - match it with a
[`Timer`](../filters/#timer) filter.

```ron
(
    name: OnTimerEnd,
    filters: [Timer((key: "briefing_delay"))],
    actions: [StoryMessage((speaker: "Control", text: "Proceed."))],
),
```

<details class="explain">
<summary>Show explanation</summary>

Timer-end events queue before that frame's `OnUpdate` pulse.

Start or restart the delay with [`TimerStart`](../actions/#timerstart). Cancel
it with [`TimerCancel`](../actions/#timercancel). Timers use live, unpaused
scenario time and clear on retry or teardown.

</details>

## Ship order lifecycle

The five events below report what became of a
[helm order](../actions/#helm-orders). They carry the SAME four fields -
`order` is the key the action named the order, `kind` is which of the five it
was (`Move` / `Align` / `Stop` / `Patrol` / `Orbit`), and `id` / `type_name`
are the ship - so one [`ShipOrder`](../filters/#shiporder) filter matches
whichever of them a handler listens for, and an `Entity` filter still works on
all of them.

```ron
(
    name: OnShipOrderComplete,
    filters: [ShipOrder((order: Some("close_the_gap")))],
    actions: [ForceRailgunFire((ship: "warship", section: "spinal"))],
),
```

<details class="explain">
<summary>Show explanation</summary>

An order that is REFUSED when it is issued - a dangling ship id, a player's
ship, an empty patrol route, an impossible tolerance - fires nothing at all.
It was never installed, so there is nothing to report on. The lint catches
each of those before the scenario runs.

A key is reusable: give the same order again and the lifecycle runs again.

</details>

## OnShipOrderComplete

Fires exactly once when an order reaches what it was told to reach.

<details class="explain">
<summary>Show explanation</summary>

What "reached" means is per order:

- [`MoveShipTo`](../actions/#moveshipto) and
  [`StopShip`](../actions/#stopship) complete when the ship's autopilot lets
  go - inside the standoff, or at rest.
- [`ForceAlign`](../actions/#forcealign) completes when the aim is inside its
  authored tolerance and steady there.
- [`PatrolShip`](../actions/#patrolship) completes at the END of its one
  loop, back where it started - not at each waypoint.
- [`OrbitShip`](../actions/#orbitship) completes when the ring is
  ESTABLISHED, and then keeps holding it.

Completion says the CONDITION was met, not that the behavior stopped. `Move`,
`Stop` and `Patrol` release the helm as they report; `Align` and `Orbit` keep
holding until something else takes it.

</details>

## OnShipOrderInterrupted

Fires when an AI ship breaks off its order to fight. Only an AI ship can, and
only one whose `order_interruption` policy says so (see
[Spaceship](../objects/#spaceship)); the default policy is never, and a
`None`-controller ship has no bot to break off in the first place.

```ron
(
    name: OnShipOrderInterrupted,
    filters: [ShipOrder((ship: Some("picket")))],
    actions: [StoryMessage((speaker: "Picket", text: "Contact. Breaking off."))],
),
```

<details class="explain">
<summary>Show explanation</summary>

The order is NOT lost. It is parked: the ship keeps the key, the kind and its
place in the route, the AI takes the helm back and fights, and when the signal
clears the same order picks up from where it stopped - see
[`OnShipOrderResumed`](#onshiporderresumed).

A beat gated on the COMPLETION still waits, correctly: an interrupted patrol
has not swept the belt yet.

</details>

## OnShipOrderResumed

Fires when an interrupted order is picked back up - the hostile is gone, or
the damage has stopped, depending on the ship's policy.

<details class="explain">
<summary>Show explanation</summary>

Pairs one-for-one with
[`OnShipOrderInterrupted`](#onshiporderinterrupted), and can pair with it
several times over one order's life. The resumed order flies its OWN
directive, from the leg it was on - not a fresh copy from the top.

</details>

## OnShipOrderCanceled

Fires when an unfinished order is called off: either
[`ClearShipOrder`](../actions/#clearshiporder), or a second helm order
replacing it.

<details class="explain">
<summary>Show explanation</summary>

Cancellation is not completion, and a beat gated on the completion correctly
never runs. This is the event for the OTHER side of that: cleaning up a
readout, or telling the player the approach was called off.

An order that had already reported a terminal outcome says nothing more, so
clearing a ship that is holding a finished
[`ForceAlign`](../actions/#forcealign) is silent - the align already
completed.

</details>

## OnShipOrderFailed

Fires when an order can no longer be flown. The hull lost the flight computer
or the drives the order runs on, or the gravity well an
[`OrbitShip`](../actions/#orbitship) named is gone or will not hold a ring.

<details class="explain">
<summary>Show explanation</summary>

This is the event that keeps a `Sequence` gate from waiting forever on a beat
that can never land. A wreck does not arrive.

It reports the ORDER, not the ship: a hull that is destroyed outright is
[`OnDestroyed`](#ondestroyed) and [`OnDefeated`](#ondefeated), which is what a
kill objective should be waiting on. Failure is for the ship that is still
there and can no longer do the job.

Weapon actions have no lifecycle at all - what a shot produces is the weapon's
own event chain.

</details>

## OnDefeated

Fires exactly once when a ship leaves combat through neutralization or direct
physical destruction. Payload: `id` and `type_name` of the defeated ship - the
event for kill objectives and encounter progression that do not care whether a
wreck remains.

```ron
(
    name: OnDefeated,
    filters: [Entity((id: Some("raider")))],
    actions: [ /* complete the encounter once */ ],
),
```

<details class="explain">
<summary>Show explanation</summary>

Ordering is fixed:

- Neutralization: `OnDefeated`, then `OnNeutralized`.
- Direct ship destruction: `OnDefeated`, then `OnDestroyed`.
- Later destruction of an already-neutralized wreck: `OnDestroyed` only.

Scripted despawn, scenario teardown, and boundary cleanup fire none of these
edges.

</details>

## OnDestroyed

Fires when a scenario object is physically destroyed: an asteroid breaks, or a
ship dies through the section-explosion pipeline. Payload: `id` and
`type_name` of the DESTROYED object; there is no other party.

```ron
(
    name: OnDestroyed,
    filters: [ Entity((type_name: Some("asteroid"))) ],
    actions: [ /* bump a counter */ ],
),
```

<details class="explain">
<summary>Show explanation</summary>

Type names are the object-kind constants: `"anchor"`, `"asteroid"`,
`"spaceship"`, `"beacon"`, `"salvage_crate"`, `"light"` - see
[Scenario objects](../objects/).

</details>

## OnNeutralized

Fires when a ship that was ARMED loses all working weapons, or loses the
flight computer it once had - combat-dead, hull possibly intact, still in the
world. Payload: `id`, `type_name` of the neutralized ship; no other party.

```ron
(
    name: OnNeutralized,
    filters: [Entity((id: Some("derelict_gunship")))],
    actions: [
        ObjectiveComplete((id: "disarm_gunship")),
        StoryMessage((speaker: "Control", text: "Guns down. The wreck is yours.")),
    ],
),
```

<details class="explain">
<summary>Show explanation</summary>

A brain-dead ship cannot aim or fly, whatever else survives; thrusters play
no part in the rule. The ship is NOT despawned, so no `OnDestroyed` fires
with it. A ship that never had a computer (a bare emplacement) only
neutralizes by losing its guns.

Use `OnNeutralized` only when the persistent-wreck distinction matters. Use
`OnDefeated` for the shared combat outcome.

</details>

## OnEnter

Fires when a body's FIRST collider makes contact with a trigger area
(occupancy is refcounted per body, so a multi-section ship fires it once, on
the 0-to-1 transition). Payload: `id` is the AREA; `other_id` /
`other_type_name` are the ENTERING body.

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

<details class="explain">
<summary>Show explanation</summary>

Three things produce trigger areas: the
[`CreateScenarioArea`](../actions/#createscenarioarea) action, a
[`Beacon`](../objects/#beacon) with `area_radius` set, and every
[`SalvageCrate`](../objects/#salvagecrate) (its `area_radius` is the pickup
sensor). All three report under their own id.

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

</details>

## OnExit

The complement: fires when a body's LAST collider leaves the area (the
1-to-0 transition). Same payload shape as `OnEnter`.

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

<details class="explain">
<summary>Show explanation</summary>

A body despawned while inside an area fires NO `OnExit` for itself - its
occupancy rows are pruned silently.

Use `other_type_name: Some("spaceship")` instead of `other_id` when every ship
leaving the area should match.

</details>

## Player maneuver completion

`OnGotoComplete` and `OnStopComplete` report successful terminal conditions
from the player's real autopilot. They do not fire when manual input cancels a
maneuver, a target disappears, or the ship loses the ability to continue.
Scripted and AI ship orders use the separate
[`OnShipOrderComplete`](#onshipordercomplete) lifecycle.

`OnGotoComplete` carries the reached target as `id` and the player ship as
`other_id` / `other_type_name`. The target still exists while the handler runs,
so this is the safe event on which to remove a temporary navigation beacon:

```ron
(
    name: OnGotoComplete,
    once: true,
    filters: [Entity((
        id: Some("transit_mark"),
        other_id: Some("cutter"),
    ))],
    actions: [
        ObjectiveComplete((id: "transit")),
        DespawnScenarioObject((id: "transit_mark")),
    ],
),
```

`OnStopComplete` carries the stopped player ship as `id` / `type_name`:

```ron
(
    name: OnStopComplete,
    once: true,
    filters: [Entity((id: Some("cutter")))],
    actions: [ObjectiveComplete((id: "come_to_rest"))],
),
```

A GOTO that reaches a gravity well and transitions directly into a viable
ORBIT does not complete as GOTO; use the orbit lifecycle for that maneuver.

## Orbit lifecycle

Five events describe ORBIT without hidden timing: `OnOrbitStart`,
`OnOrbitStable`, `OnOrbitLap`, `OnOrbitUnstable`, `OnOrbitEnd`. All five carry
`id` = well and `other_id` / `other_type_name` = orbiting ship.

<details class="explain">
<summary>Show explanation</summary>

- `OnOrbitStart`: the maneuver engages for a well. The ship may still be
  aligning or burning toward its ring.
- `OnOrbitStable`: velocity error enters the autopilot's stable Hold band. It
  can fire again after stability is recovered.
- `OnOrbitLap`: the ship accumulates one net revolution in the planned travel
  direction while stable. Losing stability resets partial-lap progress. It
  fires again for each later complete lap.
- `OnOrbitUnstable`: velocity error leaves Hold while ORBIT stays engaged.
- `OnOrbitEnd`: a surviving ship cancels ORBIT, changes verb, loses flight
  capability, loses the well, or switches wells. Ship destruction emits only
  `OnDestroyed`, consistent with area despawn not emitting `OnExit`.

Switching wells queues `OnOrbitEnd` for the old well, then `OnOrbitStart` for
the new one. Ending a stable orbit emits only `OnOrbitEnd`, not an unstable
edge first.

A mission that requires one physical lap listens directly:

```ron
(
    name: OnOrbitLap,
    once: true,
    filters: [Entity((id: Some("planetoid"), other_id: Some("player_spaceship")))],
    actions: [ObjectiveComplete((id: "orbit_once"))],
),
```

A continuous eight-second stable hold uses a timer instead:

```ron
(
    name: OnOrbitStable,
    filters: [Entity((id: Some("planetoid"), other_id: Some("player_spaceship")))],
    actions: [TimerStart((key: "orbit_hold", seconds: Term(Factor(Literal(Number(8.0))))))],
),
(
    name: OnOrbitUnstable,
    filters: [Entity((id: Some("planetoid"), other_id: Some("player_spaceship")))],
    actions: [TimerCancel((key: "orbit_hold"))],
),
(
    name: OnOrbitEnd,
    filters: [Entity((id: Some("planetoid"), other_id: Some("player_spaceship")))],
    actions: [TimerCancel((key: "orbit_hold"))],
),
(
    name: OnTimerEnd,
    filters: [Timer((key: "orbit_hold"))],
    actions: [ObjectiveComplete((id: "hold_orbit"))],
),
```

</details>

## Lock lifecycle

Player locks expose four one-shot edges: `OnTravelLockStart` /
`OnTravelLockEnd` for the travel (white, navigation) lock landing on and
leaving its target, `OnCombatLockStart` / `OnCombatLockEnd` for the combat
(red) lock. All four carry the locked target as `id` and the locking player
ship as `other_id` / `other_type_name`.

<details class="explain">
<summary>Show explanation</summary>

A held lock stays quiet. AI locks do not fire scenario events. A direct
target switch queues end for the old target, then start for the new target.

```ron
(
    name: OnTravelLockStart,
    filters: [Entity((
        id: Some("anchorage"),
        other_id: Some("player_spaceship"),
    ))],
    actions: [
        VariableSet((key: "surveyed", expression: Term(Factor(Literal(Number(1.0)))))),
        // ... the survey beat ...
    ],
),
(
    name: OnTravelLockEnd,
    filters: [Entity((
        id: Some("anchorage"),
        other_id: Some("player_spaceship"),
    ))],
    actions: [
        // ... react to losing the survey target ...
    ],
),
```

Use the combat pair for the red lock. To react to any locked target, omit
`id`; keep `other_id` when the locking ship must be `player_spaceship`.

</details>

## Dispatch order (what you can rely on)

- Events queue and drain FIFO within a frame; for each event, handlers run
  in AUTHORED order; within a handler, actions run in authored order.
- Actions mutate the event world immediately (a later filter in the same
  frame sees the new variable values), but their WORLD effects (spawns,
  despawns) land together at the end-of-frame sync - and an
  `ObjectiveMarkerAttach` ordered after a `SpawnScenarioObject` in the same
  handler does see the fresh object.
- Never depend on handler order across DIFFERENT events; gate on variables.
