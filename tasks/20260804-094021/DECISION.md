# Decision: NOVA OS coverage lands in systems/, Name-clicks live in nova_autopilot, and the zoo drives its own state

- DATE: 20260804-094021
- STATUS: ACCEPTED
- TASK: 20260804-094021
- TAGS: decision, examples, testing, ui

## Context

Three questions had to be settled before the `ui/` rebuild could be
implemented top to bottom. The task's own Steps carried the first as an
explicit choose-one fork ("cover the NOVA OS computer ... or record why that
coverage lands elsewhere"), and the NOTES flagged it as something to decide at
planning rather than defer into the work. The other two are the shape
questions the five runs share: how a beat names the widget it clicks, and how a
bare `App` with no `GameStates` satisfies a smoke contract written for the
game.

## Decision

### D1 - Where the NOVA OS / RTT live coverage lands (the Step's open fork)

The Step offered "cover opening the NOVA OS computer and exercising the RTT
screen, or record explicitly why that coverage lands elsewhere". It lands
ELSEWHERE, and the reasons are the category contract and the evidence already
on disk.

What `nova_os_rtt_poc` actually proved, and where each half goes:

| POC claim | Where it lives after this task |
|-|-|
| (c) a subtree behind an image camera is hoverable/clickable | ALREADY covered: `mirror_hover_serves_content_but_never_clobbers_window_ui` and `nova_os_pointer_mapping_matches_the_crt_shader_across_the_screen` (`crates/nova_gameplay/src/hud/nova_os/tests/crt.rs:246,324`) |
| the sampled image, its uniforms, the content-root routing | ALREADY covered: `nova_os_screen_samples_offscreen_image` (`tests/crt.rs:6`) |
| (a) the content subtree is RENDERED through the image camera at all | NOT covered - no test names `NovaOsImageCameraMarker`. This is `rtt_element_renders_its_subtree`, this task's new test |
| (b) the bloom/warp cost is affordable on a real GPU | a pixels-and-eyes claim; `screenshot_nova_os` captures it |

The remaining gap is a LIVE one: "press Tab in a real run, the computer opens,
its RTT screen takes a click". That does not belong in `ui/`:

- The category contract says `ui/` proves "a staged UI flow - layout,
  navigation, real text measure", and disqualifies an example whose "subject is
  the simulation, not the interface over it". The NOVA OS computer is a SYSTEM
  (terminal model, shell, app runtime, the CRT pipeline over it), not a staged
  flow. Its contract row is `systems/`.
- The roster spike (`20260804-003244`) fixed `ui/` at five runs. Adding a sixth
  here would re-open a settled roster, and folding NOVA OS beats into
  `hud_range` or `widget_zoo` would give a run two subjects - the exact drift
  the spike existed to stop.
- The path is not unwatched in the meantime: `screenshot_nova_os` still opens
  the computer with Tab and drives commands, and `20260804-093910`'s own DoD
  keeps it under `screenshots_reach_playing_without_panic`. A panic in the RTT
  path on open still fails CI; what is missing is assertions, which that run
  never carried anyway.

So the live claim is seeded as its own `systems/` task rather than smuggled in
here. Recorded so the gap reads as a decision, not an oversight.

### D2 - `Name`-resolved click actions live in `nova_autopilot::input`

`click_at` takes COORDINATES, and the task requires resolving targets by `Name`
so a layout move is survivable. Three callers in this task need the same
resolve-then-click (`widget_zoo`, `editor`, `menu_scenarios`), and examples
cannot share a helper module across category roots - a deeper file under
`examples/ui/` is a module of its SIBLING root, not of the category.

So the resolve step goes where `click_at` already lives:

```rust
pub fn ui_node_centre(world: &mut World, name: &str) -> Option<Vec2>;
pub fn click_named(name: impl Into<String>) -> impl Fn(&mut World) + ...;
pub fn hover_named(name: impl Into<String>) -> impl Fn(&mut World) + ...;
```

`ui_node_centre` is public because the slider drag needs the rect to compute
its drag leg, and assertions need to project a target. `click_named` /
`hover_named` are the `Fn(&mut World)` constructors `on_enter` takes, matching
the module's existing idiom, and warn-and-continue on a missing name exactly as
`move_cursor` does without a window.

Sizing: `ComputedNode::size()` is PHYSICAL px; the logical centre is
`GlobalTransform.translation().xy()` (already logical, the UI transform origin
is the node centre). `menu_scenarios::width_by_name` already documents the
physical-vs-logical trap (`review R1.2`) - the helper inherits that note.

### D3 - `widget_zoo` reaches `Playing` on its own, and the sentinel becomes a const

Option A (owner call) adds `GameStates` to the zoo's bare `App`. That alone is
not enough for the smoke contract: `smoke()` greps stderr for
`nova harness: reached Playing`, and that line is emitted by `nova_debug`'s
`DebugPlugin` (`crates/nova_debug/src/lib.rs:131`), which the zoo does not add
and should not - it wants an inspector, a wireframe toggle and gameplay state.

So the zoo drives the transition itself: `init_state::<GameStates>()` plus a
`Startup` system setting `NextState` to `Playing`. One unconditional
transition, identical in interactive and harnessed runs, so the autopilot's
first step needs no `.enter(...)` and the interactive run is not stuck in a
`Loading` it never leaves.

The sentinel string is then needed in three places (`nova_debug`'s plugin, the
zoo, and `tests/examples_smoke.rs`, which greps for it). It becomes
`pub const REACHED_PLAYING: &str` in `crates/nova_debug/src/harness.rs` and all
three name it. A magic string duplicated across two crates and a test is drift
waiting to happen; one const with three real callers is not a speculative
abstraction.

"Reached Playing" means, for the zoo, "the widget library is up" - not
gameplay. Said plainly in the zoo's module docs and in the `UI` smoke-list
comment, so the next reader does not take the line for more than it claims.

## Alternatives considered

- **A sixth `ui/` run for NOVA OS.** Rejected: it re-opens a roster the spike
  `20260804-003244` settled, and the contract puts a whole system in
  `systems/`, not `ui/`.
- **NOVA OS beats folded into `hud_range` or `widget_zoo`.** Rejected: both
  runs have a stated single subject, and a run with two subjects is the drift
  the spike existed to stop.
- **A `NamedClick` component, or a per-example `button_by_name` helper.**
  Rejected: the first is a concept with no requirement behind it; the second
  duplicates the same twelve lines a fourth time and lets the copies drift
  (three already exist, in `editor`, `menu_newgame` and `menu_scenarios`).
- **Teaching `nova_autopilot` to drive a stateless app (the zoo blocker's
  option B).** Already rejected by the owner on 2026-08-04 and recorded in the
  task NOTES; D3 is what option A costs.
- **Adding `nova_debug`'s `DebugPlugin` to `widget_zoo`** so the sentinel comes
  for free. Rejected: it brings an inspector, a wireframe toggle and gameplay
  state into a widget showcase.

## Consequences

- The `ui/` category stays at five runs and each keeps one subject.
- A live "the computer opens and its RTT screen takes a click" claim does not
  exist until `20260804-134347` lands. Until then the path is watched only by
  `screenshot_nova_os` exiting clean - a panic gate, not an assertion. Stated
  plainly so the gap is not mistaken for coverage.
- `nova_autopilot` gains three public items. That is a change to the crate the
  whole sprint sits on, but a read-only resolve plus two constructors in the
  module that already owns synthesized input - no behavior change to the
  driver.
- `widget_zoo` starts running in CI on every `cargo test` with a display: 533
  lines of interactive app, newly asserted to reach `Playing` and exit clean.
  Expect to find things.
- `nova harness: reached Playing` now means slightly different things in two
  apps. The const makes them impossible to drift apart; the docs make the
  difference explicit.
