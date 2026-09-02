# Scenario files

A scenario defines one playable mission, continuation, or menu backdrop. Its
`events` list is the script: events decide when a handler runs, filters decide
whether it applies, and actions change the world.

For a guided first mission, start with
[Create your first scenario](../author-a-scenario/). This page is the scenario
file reference.

## File shape

A scenario can have its own content file:

```ron
[
    Scenario((
        id: "freight_wars_arrival",
        name: "Freight Wars: Arrival",
        description: "Clear the shipping lane.",
        cubemap: "dep://base/textures/cubemap.png",
        events: [
            (
                name: OnStart,
                actions: [
                    Objective((
                        id: "clear_lane",
                        message: "Clear the shipping lane.",
                    )),
                ],
            ),
        ],
    )),
]
```

List the file in the bundle manifest. See [Mod files](../mod-files/) for the
folder and bundle shape.

## Fields

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | Stable scenario key used by campaigns and `NextScenario`. Prefix new ids with your mod id. |
| `name` | string | required | Name shown in the Scenarios menu. |
| `description` | string | required | Details shown for the selected scenario. |
| `cubemap` | asset ref | required | Skybox image, such as `dep://base/textures/cubemap.png` or `self://textures/sky.png`. |
| `thumbnail` | `Option` asset ref | `None` | Plain 2D menu image, written as `Some("self://thumbnails/x.png")`. Do not use a cubemap. |
| `hidden` | bool | `false` | `true` removes the scenario from the flat list. Campaign members remain available under their campaign. |
| `menu_backdrop` | bool | `false` | `true` adds the scenario to the random main-menu backdrop rotation. Backdrops normally also use `hidden: true`. |
| `watches` | list | `[]` | Read-only queries sampled into auto-updating variables, entries of `(variable: "...", query: ...)`. See [Queries and watched variables](../expressions/#queries-and-watched-variables). |
| `events` | list of handlers | `[]` | Scenario script. Empty is valid but does nothing. |

A menu backdrop POSES ITS OWN CAMERA: author a
[`SetCamera`](../actions/#setcamera) in its `OnStart` (the reference shot is
`position: (0, 570, 1920)` in meters, looking at the origin, which frames
about 1,060 m either side of the origin in a 4:3 window). A backdrop without
one is a content Error and never enters the menu rotation - the menu derives
no pose of its own.

## Handler shape

Each event entry is one handler:

| field | type | default | meaning |
|---|---|---|---|
| `name` | event kind | required | Trigger such as `OnStart`, `OnUpdate`, or `OnDestroyed`. |
| `once` | bool | `false` | Retire this handler the first time its filters pass. |
| `filters` | list | `[]` | Every filter must pass. An empty list always passes. |
| `actions` | list | `[]` | Commands run in listed order after the filters pass. |

```ron
(
    name: OnDestroyed,
    once: true,
    filters: [
        Entity((id: Some("lane_blocker"))),
    ],
    actions: [
        ObjectiveComplete((id: "clear_lane")),
        Outcome((
            outcome: Victory,
            message: Some("The shipping lane is open."),
        )),
    ],
),
```

### `once` - a beat that happens one time

A beat that can only happen once says so, and the engine holds the fact.
Without `once` the same handler needs a flag of its own: a `VariableSet` in
`OnStart` to seed it, a filter reading it, and an action writing it - three
lines of ceremony that are about the machine, not about the game.

`once` retires the handler the first time its filters PASS, not the first time
its event fires. A refused event leaves it live, so a beat waiting on a
condition still gets every later chance at it.

<details class="explain">
<summary>Show explanation</summary>

Use it for anything with a single occurrence: a story line, an objective post,
a one-time spawn, an act transition, an outcome. Leave it OFF for anything
that genuinely repeats - a per-frame HUD readout, a re-armable warning, a
cycle a player can ride more than once.

Keep a variable only where it is a SIGNAL another handler reads ("wave two is
on the board", "the convoy lost a ship"). Delete it where its only reader was
its own filter - that is what `once` replaces.

A retired handler is gone: it stops being walked every frame, and it cannot
fire again even if the same event repeats in the same frame.

</details>

### `Sequence` - beats that follow each other

A run of beats that only wait for LATER is one
[`Sequence`](../actions/#sequence) action holding the ordered steps. The
engine holds the cursor, so the chain needs no step counter, no stamped
deadline, and no handler per beat.

```ron
Sequence((
    key: "opening",
    steps: [
        (after: Some(2.0), actions: [ /* first line */ ]),
        (after: Some(8.4), actions: [ /* second line */ ]),
    ],
)),
```

Keep a HANDLER where the beat must still ASK something when it lands - which
of two lines to speak, whether the fight is still live. A step runs when its
wait ends; only a handler re-checks.

## Scenario scripting chapters

- [Events](../events/) - when handlers run and what entity data they carry.
- [Filters](../filters/) - entity matching, expression conditions, and logic.
- [Actions](../actions/) - objectives, spawning, story, flow, and state changes.
- [Scenario objects](../objects/) - asteroids, ships, beacons, salvage, and
  lights.
- [Variables and expressions](../expressions/) - counters, conditions, clocks,
  and state machines.

These constructs apply only inside a scenario's `events` list. Campaign and
section items do not use them.

## Lifecycle

1. The game tears down the previous scenario and loads the selected scenario.
2. All handlers are registered, then `OnStart` fires.
3. Events fire while the scenario is active. Filters gate actions, and a
   `once` handler retires the first time its filters pass.
4. A switch or retry removes all scenario-scoped objects and clears variables,
   objectives, story messages, HUD readouts, and pending transitions.

Nothing persists automatically between scenarios. Put shared progression in
separate content or design each scenario to start from a complete state.

## Add or replace a scenario

A new id adds a scenario. Reusing an existing id replaces the whole scenario;
it is not a field-level patch. Names and filenames do not affect matching.

Leave `hidden` unset for a scenario players should launch directly. Use a
campaign or `NextScenario` for hidden continuation chapters.

## Check it

```sh
nix develop --command cargo run content lint --target path/to/your-mod
nix develop --command cargo run --features dev
```

The lint checks target ids, duplicate object ids, prototype references, ship
geometry, reserved variables, and other errors that valid RON alone cannot
find. In the game, enable the mod, open Scenarios, and play the visible entry.

## Common mistakes

- A scenario with no `Light` object renders black.
- Unknown fields and misspelled enum variants are parse errors.
- Optional values require `Some(...)`.
- Initialize variables before filters read them.
- Spawn an object before an action targets its id.
- `hidden` and `menu_backdrop` are separate flags.
- A mod cannot replace the base New Game selection. Use the Scenarios menu,
  campaigns, or `NextScenario`.
