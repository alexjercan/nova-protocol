# Notes: Rebuild ui/ to drive real widgets with pointer input and assert the live tree

Goal in one line: `ui/` runs stop asserting AROUND the interface and start
clicking it, then check the live tree so nothing ghosts or duplicates on the
state change the click caused.

## What changes

Before: six runs under `examples/ui/`, plus three `*_poc.html` design sources
sitting in the same directory. `widget_zoo` is fully interactive but nothing
drives it - it is an eyeball, and it sits in `NOT_SMOKED` because it runs its
own `App` with no `GameStates`. `nova_os_rtt_poc` is a shipped-feature POC.

After: five runs, all driven by synthesized pointer input
(`click_at`, `move_cursor`, `press_mouse` from `nova_autopilot::input`, landed
with `20260802-120025`), all asserting the live tree, `examples/ui/` holding
only runnable examples.

| Run | Lines | Change |
|-|-:|-|
| `widget_zoo` | 533 | DRIVE it: hover, press, reskin, segmented select, checkbox/toggle, slider drag |
| `hud_range` | 1030 | KEEP - already predicate-driven, screen-projected indicators |
| `editor` | 281 | deepen into a real build-and-inspect sequence |
| `menu_newgame` | 147 | NARROW to "gameplay state reached" only |
| `menu_scenarios` | 304 | deepen: pointer-driven picker navigation + the pane-width verdict |
| RTT element test | - | NEW, inherits `nova_os_rtt_poc`'s coverage |

## Surfaces

| File | Why |
|-|-|
| `examples/ui/widget_zoo.rs` | The blocker. `fn main` builds a bare `App::new()` with `DefaultPlugins` + `nova_ui::widget::register` - no `AppBuilder`, no `GameStates`. `AutopilotPlugin<S: States>` is generic over a state type this app does not have. |
| `examples/ui/editor.rs` | One editor action today; needs build-and-inspect. |
| `examples/ui/menu_newgame.rs` | Boots `shakedown_run`, a story scenario. Narrow to asserting gameplay state is reached, nothing about scenario internals. |
| `examples/ui/menu_scenarios.rs` | Likely already solves "click a named button" - read it before writing a new helper (see 093934's open question). |
| `examples/ui/nova_os_rtt_poc.rs` (526 lines) | DELETED by `20260804-093910`. Its coverage becomes an element test here. |
| `examples/ui/*_poc.html` (3 files) | Moved by `20260804-003301` (ACTIVITY: PLANNING). This task depends on that for its "only runnable examples" end-state. |
| `crates/nova_autopilot/src/input.rs` | `press_key`, `release_key`, `move_cursor`, `click_at`, `press_mouse`, `release_mouse`. |
| `tests/examples_smoke.rs` | `UI:47` gains `widget_zoo` if the blocker resolves toward `GameStates`; `NOT_SMOKED:74-78`'s widget_zoo justification is rewritten or deleted; `nova_os_rtt_poc` leaves. |

## Data and interfaces

The RTT element test, beside the other widget tests (not an example):

```rust
#[test]
fn rtt_element_renders_its_subtree();
```

Pointer driving, per `AutopilotPlugin<S>`:

```rust
.step("press_skin")
    .on_enter(|world| click_at(world, skin_button_centre(world)))
    .until(resource_where::<UiSkin>(|s| *s == UiSkin::Hardware))
    .add()
```

`click_at` takes coordinates, so every click needs the target node's screen
position. Resolve by `Name` rather than by literal coordinates, so a layout
move is survivable and only a rename breaks a run. Check `menu_scenarios`
before writing the helper - it may already do this.

Owner call 2026-08-04: real pointer input is used HERE and only here. `ui/`'s
subject IS the interface, so reachability, hover, press and hit-testing are
what it must prove. `systems/outcomes` triggers `Activate` directly instead.
Accepted cost: `ui/` runs become coupled to LAYOUT the way the retired story
runs were coupled to CONTENT. That is the price of testing a UI as a UI, but it
is the same shape of fragility the spike claimed to be escaping, and it should
be recognised as such rather than discovered later.

## Sketches

Illustrative only. The blocker, option A:

```diff
 // widget_zoo.rs
-let mut app = App::new();
-app.add_plugins(DefaultPlugins.set(WindowPlugin { ... }));
+let mut app = App::new();
+app.add_plugins(DefaultPlugins.set(WindowPlugin { ... }));
+app.init_state::<GameStates>();   // so AutopilotPlugin<GameStates> applies
```

Option B leaves the zoo alone and teaches the autopilot to drive a stateless
app - a change in `nova_autopilot`, not in the example.

## Shape

```
                 synthesized pointer (nova_autopilot::input)
                 move_cursor -> press_mouse -> release_mouse
                                 |
        +------------+-----------+-----------+--------------+
        v            v           v           v              v
   widget_zoo     editor     menu_newgame  menu_scenarios  hud_range
   (BLOCKED:      build +    narrow to     picker nav +    KEEP
    no GameStates) inspect   "reached      pane width      (already
        |                     Playing")                     predicate-
        v                                                    driven)
   live-tree assertion after every state change
   (duplicate components, TextShadow ghosting - invisible to cargo check)

   nova_os_rtt_poc  --deleted by 093910-->  rtt_element_renders_its_subtree
                                             (a test, not an example)
```

## Consequences and open questions

- RESOLVED (owner, 2026-08-04): option A - add `GameStates` to the zoo's own
  `App`. Simple and local. Option B (teach `nova_autopilot` to drive a
  stateless app) is the more general fix and stays unbuilt: nothing else needs
  it, and it would change the crate the whole sprint sits on. It becomes its
  own task if a second stateless app ever needs driving.
- Deleting the `widget_zoo` `NOT_SMOKED` entry means it starts running in CI on
  every `cargo test` with a display. It is a 533-line interactive app; expect
  to find things.
- The `NOVA_ZOO_CAPTURE=1` two-skin capture path in `widget_zoo` is a
  screenshot producer living in a `ui/` example. Per the category contract that
  is a `screenshots/` job. Now recorded in the task's Notes; flag it if 093855's
  contract test fires rather than pre-emptively moving it.
- This is the only task depending on `20260804-003301`, which is still in
  PLANNING. If 003301 slips, the `! ls examples/ui/*.html` DoD blocks on
  something outside this task's control.
- OPEN: "cover opening the NOVA OS computer and exercising the RTT screen, or
  record explicitly why that coverage lands elsewhere" is written as a
  choose-one Step. It should be decided at planning, not deferred into the work.
- Live-tree assertions are the point: `cargo check` misses duplicate-component
  panics and TextShadow ghosting. Every run here must be RUN under Xvfb :99.
