# Filters

A filter gates a handler: when its event fires, EVERY entry in the handler's
`filters` list must pass (logical AND) before the actions run. An empty (or
omitted) list always passes. There are exactly five filter kinds - `Entity` matches
entity payloads, `Timer` matches a timer key, `ShipOrder` matches a helm
order's outcome, `Expression` tests scenario variables, and `Conditional`
combines other filters with boolean logic.

| filter | tests | typical use |
|---|---|---|
| [`Entity`](#entity) | who the event is about | "this beacon, entered by the player" |
| [`Timer`](#timer) | a timer event key | "the orbit hold timer ended" |
| [`ShipOrder`](#shiporder) | one helm order's outcome | "the warship reached its firing position" |
| [`Expression`](#expression) | a variable condition | "the counter is past 4 and the flag is unset" |
| [`Conditional`](#conditional) | other filters, combined | "NOT the player", "picket A down OR picket B down" |

## Entity

Match the identity fields the event carries. Every field is optional; every
SET field must match exactly (string equality); an unset field is
unconstrained.

```ron
// the beacon_1 area, entered specifically by the player ship
Entity((id: Some("beacon_1"), other_id: Some("player_spaceship")))

// any asteroid
Entity((type_name: Some("asteroid")))

// any event that carries entity data at all
Entity(())
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | matches |
|---|---|---|---|
| `id` | `Option` string | `None` | the event subject's id |
| `type_name` | `Option` string | `None` | the subject's object kind (`"anchor"`, `"asteroid"`, `"spaceship"`, `"beacon"`, `"salvage_crate"`, `"light"`) |
| `other_id` | `Option` string | `None` | the other party's id |
| `other_type_name` | `Option` string | `None` | the other party's object kind |

Which entity is the subject and which is the other party is per-event - the
payload tables in the [Events reference](../events/) are the source of
truth. The quick map:

| event | `id` / `type_name` (subject) | `other_id` / `other_type_name` |
|---|---|---|
| `OnDefeated`, `OnDestroyed`, `OnNeutralized` | the defeated / destroyed / neutralized object | (none) |
| `OnEnter` / `OnExit` | the AREA (zone, beacon, crate) | the body that entered / left |
| Orbit lifecycle events | the well being orbited | the orbiting ship |
| travel/combat lock start/end | the locked target | the locking player ship |
| `OnStart` / `OnUpdate` | (no payload - an Entity filter never matches) | (none) |

Two rules that bite:

- **Absent field = never matches.** A set filter field whose key the event
  does not fill fails the filter - `other_id` can never pass on
  `OnDestroyed`, and any `Entity` filter fails on `OnStart`/`OnUpdate`.
  Constrain only the fields the event actually provides.
- **Filters gate; they do not bind.** The matched ids are never passed to
  the actions. An action cannot say "spawn at whatever entered" - it acts on
  its own configured target id. Use the filter to decide WHETHER the handler
  runs, then address entities by their known scenario ids.

</details>

## Timer

Match the `key` carried by [`OnTimerEnd`](../events/#ontimerend). It fails
closed on every other event because those events carry no timer key.

```ron
Timer((key: "orbit_hold"))
```

<details class="explain">
<summary>Show explanation</summary>

Timer keys are scenario-local strings. This filter observes the event that
already ended; it does not test whether a timer is currently running.

</details>

## ShipOrder

Match one [helm order](../actions/#helm-orders) outcome, carried by any of the
five [ship order events](../events/#ship-order-lifecycle). Every field is
optional; every SET field must match exactly. It fails closed on every other
event, which carries no order.

```ron
// the order this beat is waiting for
ShipOrder((order: Some("close_the_gap")))

// any order the warship finishes
ShipOrder((ship: Some("warship")))

// any alignment, by any ordered ship
ShipOrder((kind: Some(Align)))
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | matches |
|---|---|---|---|
| `order` | `Option` string | `None` | the key the helm action named the order |
| `ship` | `Option` string | `None` | the ordered ship's id |
| `kind` | `Option` kind | `None` | `Move` / `Align` / `Stop` / `Patrol` / `Orbit` (bare enum, no quotes) |

The same filter reads all five outcomes - completion, interruption, resume,
cancellation, failure - because the EVENT decides which outcome a handler
listens for and the filter only qualifies it. Waiting on a completion and
cleaning up on a failure is two handlers with the same filter.

`ShipOrder(())`, with nothing set, matches EVERY order outcome - a legitimate
way to hang one handler off a whole scripted sequence, and neither an error
nor a warning.

Order keys are minted by the action that installs the order, so the lint
checks them against the [`MoveShipTo`](../actions/#moveshipto),
[`ForceAlign`](../actions/#forcealign),
[`StopShip`](../actions/#stopship),
[`PatrolShip`](../actions/#patrolship) and
[`OrbitShip`](../actions/#orbitship) actions the scenario authors: a filter
waiting on a key nothing ever issues is an Error, because that handler could
never run.

</details>

## Expression

Evaluate a variable condition; pass when it is true. The single tuple field
is a condition node from the
[expression grammar](../expressions/#conditions-the-boolean-root) -
`LessThan`, `GreaterThan` or `Equal` over value expressions.

```ron
Expression((GreaterThan(
    Term(Factor(Name("asteroids_destroyed"))),
    Term(Factor(Literal(Number(4.0)))),
)))
```

<details class="explain">
<summary>Show explanation</summary>

An `Expression` filter reads ONLY the variable store - it ignores the event
payload entirely, which is why it composes with any event, including
`OnUpdate`.

</details>

## Conditional

Boolean combinators over other filters (any kinds, nestable), written
positionally: `Not(<filter>)`, `Or(<filter>, <filter>)`,
`And(<filter>, <filter>)`.

```ron
// not the player
Conditional(Not(Entity((id: Some("player_spaceship")))))

// either picket still standing
Conditional(Or(
    Expression((Equal(
        Term(Factor(Name("picket_a_down"))),
        Term(Factor(Literal(Number(0.0)))),
    ))),
    Expression((Equal(
        Term(Factor(Name("picket_b_down"))),
        Term(Factor(Literal(Number(0.0)))),
    ))),
))
```

<details class="explain">
<summary>Show explanation</summary>

| variant | arity | passes when |
|---|---|---|
| `Not(<filter>)` | 1 | the inner filter does NOT pass |
| `Or(<filter>, <filter>)` | 2 | either passes |
| `And(<filter>, <filter>)` | 2 | both pass |

The `filters` LIST is already an AND, so a top-level `Conditional(And(..))`
is redundant - `And` exists for composing under `Or` and `Not`. Nesting
depth is bounded by RON's recursion limit (128), far past anything a sane
script needs.

</details>

## Traps for the unwary

Three rules catch every new author: filters fail closed, a payload that fails
to serialize reads as no-match, and repeating events re-run ungated actions.

<details class="explain">
<summary>Show explanation</summary>

- **Filters fail CLOSED.** Any evaluation error - an UNDEFINED variable, a
  type mismatch, a division by zero - logs an error and the filter returns
  false. The handler silently never fires. Consequence: seed EVERY variable
  your filters read in `OnStart` (variables a
  [watch](../expressions/#queries-and-watched-variables) maintains are
  exempt). A missing seed is a soft-lock, not a crash - and the content
  lint warns on reads of never-set, unwatched variables.
- **A payload that fails to serialize also fails closed** (loudly, in the
  log): every `Entity` filter reads "no match" for that event.
- **Repeating events + an ungated filter = repeated actions.** `OnUpdate`
  fires every frame, and the lifecycle edges re-fire on every genuine
  transition (a lock landing again, orbit stability recovered). When the beat
  happens one time, mark the handler
  [`once: true`](../scenarios/#once-a-beat-that-happens-one-time) and it
  retires the first time its filters pass. When the beat repeats, re-arm it in
  its own actions - the [recipes](../expressions/#recipes).

</details>
