# Filters

A filter gates a handler: when its event fires, EVERY entry in the handler's
`filters` list must pass (logical AND) before the actions run. An empty (or
omitted) list always passes. There are exactly four filter kinds - `Entity` matches
entity payloads, `Timer` matches a timer key, `Expression` tests scenario
variables, and `Conditional` combines other filters with boolean logic.

| filter | tests | typical use |
|---|---|---|
| [`Entity`](#entity) | who the event is about | "this beacon, entered by the player" |
| [`Timer`](#timer) | a timer event key | "the orbit hold timer ended" |
| [`Expression`](#expression) | a variable condition | "the counter is past 4 and the flag is unset" |
| [`Conditional`](#conditional) | other filters, combined | "NOT the player", "picket A down OR picket B down" |

## Entity

Match the identity fields the event carries. Every field is optional; every
SET field must match exactly (string equality); an unset field is
unconstrained.

| field | type | default | matches |
|---|---|---|---|
| `id` | `Option` string | `None` | the event subject's id |
| `type_name` | `Option` string | `None` | the subject's object kind (`"asteroid"`, `"spaceship"`, `"beacon"`, `"salvage_crate"`, `"light"`) |
| `other_id` | `Option` string | `None` | the other party's id |
| `other_type_name` | `Option` string | `None` | the other party's object kind |

Which entity is the subject and which is the other party is per-event - the
payload tables in the [Events reference](../events/) are the source of
truth. The quick map:

| event | `id` / `type_name` (subject) | `other_id` / `other_type_name` |
|---|---|---|
| `OnDestroyed`, `OnNeutralized` | the destroyed / neutralized object | (none) |
| `OnEnter` / `OnExit` | the AREA (zone, beacon, crate) | the body that entered / left |
| `OnOrbit` | the well being orbited | the orbiting ship |
| `OnTravelLock` / `OnCombatLock` | the locked target | the locking (player) ship |
| `OnStart` / `OnUpdate` | (no payload - an Entity filter never matches) | (none) |

```ron
// the beacon_1 area, entered specifically by the player ship
Entity((id: Some("beacon_1"), other_id: Some("player_spaceship")))

// any asteroid
Entity((type_name: Some("asteroid")))

// any event that carries entity data at all
Entity(())
```

Two rules that bite:

- **Absent field = never matches.** A set filter field whose key the event
  does not fill fails the filter - `other_id` can never pass on
  `OnDestroyed`, and any `Entity` filter fails on `OnStart`/`OnUpdate`.
  Constrain only the fields the event actually provides.
- **Filters gate; they do not bind.** The matched ids are never passed to
  the actions. An action cannot say "spawn at whatever entered" - it acts on
  its own configured target id. Use the filter to decide WHETHER the handler
  runs, then address entities by their known scenario ids.

## Timer

Match the `key` carried by [`OnTimerEnd`](../events/#ontimerend). It fails
closed on every other event because those events carry no timer key.

```ron
Timer((key: "orbit_hold"))
```

Timer keys are scenario-local strings. This filter observes the event that
already ended; it does not test whether a timer is currently running.

## Expression

Evaluate a variable condition; pass when it is true. The single tuple field
is a condition node from the
[expression grammar](../expressions/#conditions-the-boolean-root) -
`LessThan`, `GreaterThan` or `Equal` over value expressions:

```ron
Expression((GreaterThan(
    Term(Factor(Name("asteroids_destroyed"))),
    Term(Factor(Literal(Number(4.0)))),
)))
```

An `Expression` filter reads ONLY the variable store - it ignores the event
payload entirely, which is why it composes with any event, including
`OnUpdate`.

## Conditional

Boolean combinators over other filters (any kinds, nestable). Tuple
variants: the inner filters are written positionally.

| variant | arity | passes when |
|---|---|---|
| `Not(<filter>)` | 1 | the inner filter does NOT pass |
| `Or(<filter>, <filter>)` | 2 | either passes |
| `And(<filter>, <filter>)` | 2 | both pass |

```ron
// not the player
Conditional(Not(Entity((id: Some("player_spaceship")))))

// either picket still standing (shipped, Final Tally)
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

The `filters` LIST is already an AND, so a top-level `Conditional(And(..))`
is redundant - `And` exists for composing under `Or` and `Not`. Nesting
depth is bounded by RON's recursion limit (128), far past anything a sane
script needs.

## Traps for the unwary

- **Filters fail CLOSED.** Any evaluation error - an UNDEFINED variable, a
  type mismatch, a division by zero - logs an error and the filter returns
  false. The handler silently never fires. Consequence: seed EVERY variable
  your filters read in `OnStart` (the two
  [reserved engine variables](../expressions/#reserved-engine-variables)
  are exempt). A missing seed is a soft-lock, not a crash - and the content
  lint warns on reads of never-set variables.
- **A payload that fails to serialize also fails closed** (loudly, in the
  log): every `Entity` filter reads "no match" for that event.
- **Recurring events + an ungated filter = repeated actions.** The 5 s
  recurrence on `OnOrbit` and the locks (and every frame on `OnUpdate`)
  passes your filters again each pulse. Pair the condition with a one-shot
  flag the actions flip - the
  [count-gate idiom](../expressions/#recipes).
