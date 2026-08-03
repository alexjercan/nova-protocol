# Decision: The predicate autopilot's API surface

- DATE: 20260804-004646
- STATUS: ACCEPTED
- TASK: 20260802-120025
- TAGS: autopilot, tooling, api

## Context

`AutopilotPlugin` has twelve callers across `examples/`, plus `nova_probe` and
the `nova_debug::harness` adapter. Replacing the `(state, seconds)` timeline
with predicate steps decides three things at once: which parts of the old
surface survive, how much predicate vocabulary the crate ships, and how far
the migration reaches in one commit. The NOTES sketch proposed a wider surface
than any current caller consumes, so each item needs a named caller or a
deferral.

Inventory at plan time (grep over `examples/`): `self_completing` in
broadside, lifeline, com_range, hud_range, screenshot_nova_os,
menu_scenarios; `loop_while_pending` in playable and scenario;
`playing_since` beat offsets in com_range, hud_range, playable and the three
screenshot producers; per-example completion guards in five files. Six callers
use nothing but `hold` + `input`.

## Decision

**`hold` and `input` survive; `self_completing` and `loop_while_pending` do
not.** `hold(state, secs)` is `step("hold:<state>").enter(state)
.until(elapsed(secs))` and `input(f)` attaches `f` as the `each` action of
every step - constructors over the step model, not compatibility shims, and
they leave the six pure-timeline callers untouched. The other two are MODES:
`self_completing` means "runway expiry is an abort", which the ordinary
per-step deadline now covers, and `loop_while_pending` means "loop the whole
script", which `loop_from(name)` generalizes. Both are deleted.

**`each` elapsed is STEP-relative.** The `playing_since` scaffolding exists
solely to make the run clock step-relative by hand, so step-relative is the
useful one. It is also compatible with every current `input(f)` caller: all
twelve are single-step scripts, so the two clocks coincide and no caller's
arithmetic changes. Run-relative elapsed is not additionally exposed.

**Per-step deadline defaults to unset**, leaving the run-level
`NOVA_AUTOPILOT_DEADLINE` watcher as the backstop. A plugin-wide default would
put every `hold` step under a bound it can never hit and would silently change
the failure mode of the six untouched callers.

**`AutopilotLoop` stays alongside `on_loop`.** `crates/nova_probe/src/capture.rs`
reads the message, so it is not internal to the examples. `on_loop(f)` is the
in-driver reset hook; playable's `on_autopilot_loop` reader collapses into it,
and the probe reload-gate calls (`capture_reload_begin`) move into that hook
rather than becoming driver features - they are probe-specific.

**Nova-typed predicates live in `nova_debug::harness`.** `nova_autopilot`
depends on `bevy` alone so it can be extracted later; `scenario_variable_is`,
`section_gone` and `player_ship_present` name Nova types, so they go in the
adapter, built on the crate's generic `resource_where` / `any_entity`.

Deferred, each for want of a caller in this task:

| Item | Why deferred |
| --- | --- |
| `observe(f)` diagnostic hook | The stall message already carries step name, in-step elapsed, run elapsed and the `S` state via `Debug` - what the DoD asks for. |
| `or(a, b)` | `and` and `not` cover every predicate the three rewritten scripts compose. |
| `drag(from, to, button)` | No example drags. `move_cursor` + `press_mouse`/`release_mouse` compose one. |
| Gamepad and touch synthesis | The fleet uses keyboard and mouse only. |
| A serialized script DSL | Predicates are `Fn(&World) -> bool` closures; authoring them as data needs an expression language. |
| Enforcing sum(step deadlines) < run deadline | Documented in rustdoc. The run-level value comes from `nova_probe`'s env sizing, which the crate cannot see. |

## Alternatives considered

**Keep `self_completing` and land the driver alone, migrating callers in
`20260802-120029`.** Rejected: it keeps two ways to end a run in the crate
indefinitely, and the epic's Nova-first decision is rename/replace with no
aliases. The cost is accepted breadth - eight example binaries change in the
same commit as the driver, because deleting the mode leaves no compiling
intermediate tree. The offsetting cut is that only three scripts are
REWRITTEN (com_range, hud_range, playable); the other five change at the
plugin-construction site and `20260802-120029` rebuilds their content.

**Split driver and callers into two tasks.** Rejected for the same reason: the
split needs a throwaway shim or a broken intermediate state.

**Ship the full NOTES vocabulary (`or`, `entity_count`, `drag`, `observe`).**
Rejected: none has a caller, and each is a public item the prelude test then
pins in place.

**Run-relative `each` elapsed, matching today's `input(f)`.** Rejected: it is
the clock every script hand-corrects away, and no caller ramps input across
step boundaries.

## Consequences

- The eight migrated example binaries and the driver land in one commit; a
  bisect across it cannot separate a driver bug from a migration bug.
- Six callers keep `hold`-only timelines, so wall-clock scripts still exist in
  the tree after this task. `20260802-120029` converts them.
- `hold` and `input` remain public, so the "predicate-driven" claim is a
  default rather than an enforcement; a caller can still write a timing
  script.
- Deferring the deadline-sum check means a caller can size per-step deadlines
  above `NOVA_AUTOPILOT_DEADLINE` and lose the named-step diagnostic to the
  generic hang detector. Documented in rustdoc only.
## Resolved during work

**The pointer test drives a real widget; no reduction was needed.** The plan
left open whether `click_at` could be proven against a live `bevy_ui` node or
would have to settle for asserting the three pieces of pointer state. It does
the full thing: `click_at_position_reaches_the_widget_under_it` runs
`examples/driven_app.rs` as a real `DefaultPlugins` process under a virtual
display, and the "click the button" beat waits on the game's own
`Pointer<Press>` flag, so a click that lands where the widget is not stalls the
run. The unit tests in `src/input.rs` keep the state-level assertions as the
cheap headless half.

That depth forced a correction to the gesture synthesis. `bevy_picking`'s
`mouse_pick_events` reads `WindowEvent`, NOT the concrete `CursorMoved` /
`MouseButtonInput` messages, and it tracks the cursor from those events alone.
Writing only the concrete messages left the picking backend believing the
pointer had never moved, so every synthesized click resolved at the origin.
`set_cursor` and `set_mouse_button` now write BOTH, which is what `bevy_winit`
does for a real device. The button state is still written directly to
`ButtonInput` (so it is `just_pressed` on the same frame, as `press_key` is)
and the concrete `MouseButtonInput` message is deliberately NOT written -
`bevy_input`'s own collector reads it and would re-apply the transition a frame
late.

**One predicate beyond the planned three: `script_reports_done()` in
`nova_debug::harness`.** The five callers migrated at the construction site
keep closures that walk their own beats and report `AUTOPILOT` done on the last
one; their single wrapping step needs an `until` that ends exactly there. It
reads a collector's state to decide a step, which is why it is in the adapter
and not in the crate's generic vocabulary, and its rustdoc says plainly that a
script written fresh should not use it. `20260802-120029` retires it when it
rebuilds those five.

**`in_state::<S>(s)` ships as `state_is(s)`.** The planned name collides with
`bevy::prelude::in_state`, the run-condition every example globs.
