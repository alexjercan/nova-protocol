# Spike: Should flight verbs be gated by the controller section and per-verb flags on it?

- DATE: 20260712-143551
- STATUS: RECOMMENDED
- TAGS: spike, input, verbs, controller, scenario, shakedown

## Question

The flight verbs (STOP / GOTO / ORBIT / CANCEL) are today always-on maneuvers
that any ship can invoke as long as it is `flyable`. The request is to make them
a *capability the controller section grants*: a verb is available only if the
ship has a live controller (flight computer), and on top of that the controller
carries per-verb enable/disable flags. The first scenario then disables GOTO
initially and re-enables it once the first objective is complete.

Two uncertainties this spike reduces:

1. **Where do the per-verb flags live** - on the controller *section* entity
   (the thing that logically "provides" the verbs), on the ship root (a cheap
   capability component like `FlightSpeedCap`), or nowhere (gate GOTO with a raw
   scenario variable)?
2. **How does a scenario flip one verb at runtime** and author the initial
   off-state, reusing existing patterns rather than inventing a new mechanism?

A good answer names one home for the flags that matches the user's mental model
("flags on the spaceship controller"), shows the concrete gate points in both
the hint pass and the execution observers, and slices the work into
direction-level tasks a later `/flow` can plan.

## Context

Traced in this session on 2026-07-12; file:line references current as of commit
`b934960`.

**Two things are named "controller" - do not conflate them.**

- `SpaceshipController` enum (`nova_scenario/src/objects/spaceship.rs:16-21`):
  `None | Player(..) | AI(..)` - *who drives the ship*, a component on the ship
  root. NOT what this spike touches.
- `ControllerSectionMarker` (`nova_gameplay/src/sections/controller_section.rs:77`):
  the physical PD-rotation flight-computer *section*, built by
  `controller_section(ControllerSectionConfig)` (controller_section.rs:45-58),
  carrying `PDController`. THIS is the "spaceship controller" the user means:
  it is already what the verb gate keys on.

**How verbs work today.** The four autopilot verbs are `bevy_enhanced_input`
actions in `nova_gameplay/src/input/player.rs`:
`AutopilotStopInput` / `GotoInput` / `OrbitInput` / `OffInput` (player.rs:444-473),
bound to X / G / O / Z (player.rs:517-561). Each has:

- a **hint** field in `update_flight_verb_hints` (player.rs:134-239) that lights
  the on-screen key prompt, and
- an **execution observer** (`on_autopilot_*_input`, player.rs:684-819) that
  inserts/removes the `Autopilot` component when the key is pressed.

Availability is computed from `flyable` (player.rs:175-178):

```rust
let flyable = ship.is_some_and(|ship| {
    q_computer.iter().any(|&ChildOf(parent)| parent == ship)   // live controller section
        && q_thruster.iter().any(|&ChildOf(parent)| parent == ship) // live thruster
});
```

`q_computer` (player.rs:141-148) already filters
`With<ControllerSectionMarker>, With<PDController>, Without<SectionInactiveMarker>`.
So **"available only if the ship has a spaceship controller" is half-built** -
`flyable` already dies when the controller section is destroyed. What is missing
is (a) per-verb granularity and (b) the ability to disable a verb while the
controller is alive. Hints gate on `flyable && <verb condition>` (GOTO also needs
`lock.is_some()`, ORBIT a `dominant` well); the execution observers re-check the
verb-specific condition (GOTO needs a lock, player.rs:743) but do NOT currently
re-check `flyable` - the hint darkening is the only "controller present" signal
today, so gating must be added in BOTH places or a dark key still fires.

**How a scenario mutates a ship capability at runtime.**
`SetSpeedCapActionConfig` (`nova_scenario/src/actions.rs:211-252`) is the
template: given a ship's scenario `id`, it looks up the scoped
`SpaceshipRootMarker` entity and inserts/removes `FlightSpeedCap`. The shakedown
uses it to release the training governor at beacon 1
(`nova_assets/src/scenario/shakedown.rs:437-440`). A per-verb toggle is the same
shape, one level deeper (find the ship's child controller section).

**The shakedown today.** GOTO is first *taught* at Beat 4 (`OBJ_B4`,
shakedown.rs:518-521: "Lock BEACON 3 and press [G]"), deliberately after the
pirate-ambush fix (shakedown.rs:508-510). The player ship and the pirate BOTH
reference the same shared `basic_controller_section` catalog entry
(sections.rs:56-77; shakedown.rs:283, 336), so an initial flag baked into that
shared config would hit the pirate too - initial per-player state wants to be
authored in the scenario, not the catalog. "First objective" = `OBJ_B1` (burn to
BEACON 1), completed at the Beat 1 -> 2 handler (shakedown.rs:429-446), which is
exactly where the governor release already lives.

**Existing capability-flag patterns.** `PlayerControllerConfig`
(spaceship.rs:23-38) already carries per-ship gameplay flags (`speed_cap`,
`infinite_ammo`) authored on the ship root. `SectionInactiveMarker` gates section
queries. `ControllerSectionConfig` (controller_section.rs:18-39) is the reflected,
editor-visible config bundle a controller section is built from - the natural
home for authored defaults.

## Options considered

### A. Verb flags on the controller SECTION entity (RECOMMENDED)

A new reflected component - call it `ControllerVerbs { stop, goto, orbit, cancel }`
(all `true` by default) - lives on the controller section entity, seeded from new
fields on `ControllerSectionConfig` and inserted by `controller_section(..)`
alongside `PDController`.

- **Gate (hints):** in `update_flight_verb_hints`, extend `q_computer` to also
  read `&ControllerVerbs`, find the player ship's live controller section, and
  fold its per-verb flag into each `available:` expression
  (`stop: flyable && verbs.stop`, `goto: flyable && verbs.goto && lock.is_some()`,
  etc.). No live controller -> no flags -> everything already dark via `flyable`.
- **Gate (execution):** each `on_autopilot_*_input` observer looks up the player
  ship's live controller section and returns early unless the matching flag is
  set (adding, in passing, the `flyable`/controller-present re-check the
  observers lack today).
- **Runtime toggle:** a new `SetControllerVerbActionConfig { id, verb, enabled }`
  scenario action - same scoped-lookup skeleton as `SetSpeedCap`, but after
  finding the ship root it finds the child `ControllerSectionMarker` entity(s)
  and writes the flag.
- **Initial off-state:** authored in the scenario, not the shared catalog - a
  `SetControllerVerb(disable GOTO)` in the shakedown's opening setup event
  (alongside the `objective(OBJ_B1, ..)` at shakedown.rs:420-424), matched by a
  `SetControllerVerb(enable GOTO)` in the Beat 1 -> 2 handler next to the existing
  `complete(OBJ_B1)` / governor release.

Pros: matches the user's model exactly ("flags ON the spaceship controller");
the capability travels with the component that provides it, so a future
cheap-vs-premium controller tier (STOP-only shuttle vs full-nav flagship) is just
different `ControllerSectionConfig` defaults - no ship-root plumbing; reuses
`flyable`, the `SetSpeedCap` action skeleton, and the exact shakedown seam the
governor already uses; editor-visible via `ControllerSectionConfig` reflection.

Cons / unknowns: the execution observers must now look up the ship's controller
section (an extra query per keypress - negligible, keypresses are rare); the
runtime action is one indirection deeper than `SetSpeedCap` (root -> child
section); if a ship somehow had two controller sections the action must define
"all of them" (spec: write every live controller section; the union is the ship
capability). All mechanical, none blocking.

### B. Verb flags on the ship ROOT (capability component)

Put `ShipVerbFlags { .. }` on the ship root, mirroring `FlightSpeedCap` /
`infinite_ammo`; `SetControllerVerb` targets the root directly (byte-for-byte the
`SetSpeedCap` pattern, no child lookup); the execution observers already
`Single<.. With<PlayerSpaceshipMarker>>` the root, so reading the flag is free.

Pros: least code; zero new indirection; initial state can be a
`PlayerControllerConfig` field exactly like `speed_cap`.

Cons: semantically wrong for this request - the flags are "on the ship", not "on
the controller", so a destroyed-and-replaced controller, or a controller tier
system, would not carry its own capability; it re-introduces the same
conflation the user is trying to undo ("verbs come from the controller"). It is
the pragmatic runner-up, and if A's child-section plumbing proves fiddly this is
the graceful fallback - but A is the design the user described and the one that
scales, so B is not recommended.

### C. Gate GOTO with a raw scenario variable (do-little - REJECTED)

No component at all: the GOTO observer reads a `NovaEventWorld` variable
(e.g. `goto_enabled`) and the shakedown sets it. Rejected: it leaks scenario-engine
state into the input layer (a hard dependency the input crate does not have and
should not grow), does nothing for the "controller provides the verb" semantics,
does not generalize past this one scripted case, and gives the editor/other
scenarios no knob. Documented so a future session does not reach for the
quick variable hack; A is this done properly - the state lives on the controller
as data, and the scenario just writes it through a typed action.

### Do nothing

Leave verbs always-on. Costs the requested capability-gating and the tutorial
pacing lever (disable GOTO until the pilot has flown a controlled leg). The
`flyable` gate already gives coarse "no controller, no verbs", but not
per-verb control. Not chosen - the feature is small and directly asked for.

## Recommendation

**Option A.** Add a `ControllerVerbs` component on the controller section, seeded
from `ControllerSectionConfig` (default all-on), fold it into both the hint pass
and the execution observers, add a `SetControllerVerb` scenario action shaped like
`SetSpeedCap`, and use it in the shakedown to ship GOTO disabled from scenario
start and enable it in the Beat 1 -> 2 handler when `OBJ_B1` completes.

It is the only option that honours "flags on the spaceship controller" literally,
keeps the capability with the component that provides it (so it scales to
controller tiers), and reuses three existing seams (`flyable`, the `SetSpeedCap`
action skeleton, the governor-release event) instead of inventing mechanism.
`flyable` stays as the physical "controller + thruster present" gate; the flags
are the orthogonal per-verb enable layer on top - two legible conditions, not one
conflated one.

Slice, in dependency order:

1. **Controller verb-flag capability + gating.** `ControllerVerbs` component +
   `ControllerSectionConfig` fields; fold into `update_flight_verb_hints` and the
   four `on_autopilot_*_input` observers (including the missing controller-present
   re-check). Ships the mechanism; every verb defaults on, so no behaviour change
   until something writes a flag.
2. **`SetControllerVerb` scenario action.** New `EventActionConfig` variant +
   `SetControllerVerbActionConfig` targeting a ship id -> its controller
   section(s), enable/disable one verb. The runtime lever.
3. **Shakedown: GOTO off until first objective.** Author GOTO disabled at scenario
   start; enable it in the Beat 1 -> 2 handler next to `complete(OBJ_B1)`.

## Open questions

- **Verb enum vs four bools.** `ControllerVerbs { stop, goto, orbit, cancel }`
  (four bools, matches `FlightVerbHints`) vs a `HashSet<FlightVerb>` /
  bitflags keyed by a `FlightVerb` enum. The enum is nicer for the action
  signature (`SetControllerVerb { verb: FlightVerb, .. }`) and grows cleanly if
  verbs multiply; four bools are simpler and mirror the existing `FlightVerbHints`
  struct. Lean enum for the action + a small struct/bitflags for storage; settle
  in phase-1 `/plan`. CANCEL (Z) is "always available while engaged" today
  (player.rs:201-204) - decide whether it is flag-gatable at all or always
  exempt (recommend: exempt, so a disabled verb can never strand an engaged
  autopilot with no way to cancel it).
- **Which objective re-enables GOTO.** User said "until you complete the first
  objective" = `OBJ_B1` (Beat 1 -> 2). GOTO is not actually *taught* until Beat 4,
  so a stricter reading keeps it off until Beat 4. Default to the user's words
  (enable at Beat 1); the flag makes either trivial to tune in playtest.
- **Initial-state authoring: action vs config.** Recommended: a scenario
  `SetControllerVerb` at start (co-locates all GOTO gating in shakedown.rs, leaves
  the shared `basic_controller_section` catalog generic). Alternative: a
  per-controller-section initial-flags field set only on the player's section
  (needs the player to stop sharing the catalog entry, or a per-section override).
  There is a one-frame window where the start action has not yet run and GOTO is
  briefly on; confirm the scenario start event fires before the first player
  input is possible (it runs on scenario load, same as `objective(OBJ_B1)`), else
  author the initial state in config instead.
- **AI ships.** Verbs are player input only, so AI controllers are unaffected by
  the flags; confirm nothing in the AI brain reads the verb path. Expected no-op.

## Next steps

Direction-level tasks seeded from the recommendation (for `/plan` to break into
steps):

- tatr 20260712-143832: controller-provided verb flags - `ControllerVerbs`
  capability on the controller section, gate the hint pass and the four autopilot
  execution observers (add the missing controller-present re-check)
- tatr 20260712-143833: `SetControllerVerb` scenario action - enable/disable one
  flight verb on a ship's controller section by scenario id (shaped like
  `SetSpeedCap`)
- tatr 20260712-143834: shakedown - ship GOTO disabled from scenario start and
  re-enable it when the first objective (`OBJ_B1`) completes

## Fix record

(Appended by each implementing task as it lands.)
