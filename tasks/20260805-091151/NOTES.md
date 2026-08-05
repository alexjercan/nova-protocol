# NOTES: `click_named` presses in the same frame it warps the cursor

## Problem Statement

A `click_named` beat intermittently produces no `Activate`, so the following
release beat's `until(state_is(...))` never holds and the example dies on its
deadline. Seen roughly 1 in 3 full `examples_smoke` runs, in more than one
example, so it is the shared `click_named` mechanism and not one script.

Confirmed with the owner 2026-08-05.

It is explicitly NOT:

- one example's script (`menu_newgame` and `editor` both failed),
- the game's button or picking code (a real player click works),
- a fix that is only evidenced by "it passed once" - the failure is
  intermittent, so the record must say what was run and how many times.

## Context

### The click path, as the tree has it

| Fact | Source |
| --- | --- |
| `click_named` warps the cursor and presses in the SAME call | `crates/nova_autopilot/src/input.rs:151-160` |
| `set_cursor` writes `Window::set_cursor_position` AND both `CursorMoved` message halves | `crates/nova_autopilot/src/input.rs:186-214` |
| The driver runs `on_enter` in `PreUpdate`, `.after(InputSystems)` | `crates/nova_autopilot/src/autopilot.rs:384` |
| Every clicking beat advances on `frames(SETTLE)`, never on an observed widget state | `examples/ui/menu_newgame.rs:104`, `examples/ui/editor.rs:107,120,205,227`, `examples/ui/widget_zoo.rs:721,743,761,769` |
| `widget_zoo` already asserts `Hovered` and `bevy::ui::Pressed` in its own beats - precedent for an observed precondition | `examples/ui/widget_zoo.rs:672-700` |
| `nova_autopilot` depends on `bevy` alone, so `bevy::ui::Pressed` / `bevy::picking::hover::Hovered` predicates CAN live in it | `crates/nova_autopilot/Cargo.toml` |

### The bevy 0.19 pointer pipeline, read at the source

Checked 2026-08-05 against the vendored crates in `~/.cargo/registry`.

| Claim | Source |
| --- | --- |
| Window events become `PointerInput` in the **`First`** schedule, one frame AFTER the driver wrote them in `PreUpdate` - and the Move in the same batch updates `cursor_last` BEFORE the Press is stamped with it | `bevy_picking-0.19.0/src/input.rs:99-106,121-160` |
| Within `PreUpdate`, `ProcessInput -> Backend -> Hover` is `.chain()`ed, so the raycast and the hover map are CURRENT when the press dispatches | `bevy_picking-0.19.0/src/lib.rs:401-410` |
| `Pointer<Press>` is dispatched from the CURRENT `hover_map` | `bevy_picking-0.19.0/src/events.rs:921-932` |
| `Pointer<Click>` is dispatched from the **`previous_hover_map`**, and only for entities this pointer is recorded as pressing | `bevy_picking-0.19.0/src/events.rs:955-975` |
| The button emits `Activate` from `Pointer<Click>`, and only while it still has `Pressed` | `bevy_ui_widgets-0.19.0/src/button.rs:58-71` |
| `Window::set_cursor_position` makes `bevy_winit` warp the REAL OS pointer, which echoes back as a real `CursorMoved` at an arbitrary later frame | `bevy_winit-0.19.0/src/system.rs:433-440` |

So the mechanism recorded in `TASK.md` (candidate 1: "the picking backend
raycasts a system later, so the widget never enters `Pressed`") does NOT hold:
warping and pressing in one call is self-consistent, and if it were not, every
run would fail rather than one in three.

### What the instrumented run actually shows

A temporary `NOVA_POINTER_TRACE` plugin (`crates/nova_autopilot/src/pointer_trace.rs`,
NOT for landing) logs window events, `PointerInput`, the hover map, `Pressed`
and the `Pointer<Press|Click|Release>` triples.

`DISPLAY=:99 NOVA_AUTOPILOT=1 NOVA_POINTER_TRACE=1 cargo run --example menu_newgame --features debug`
on a PASSING run:

```
step `menu_newgame: click New Game` begins
TRACE win: CursorMoved Vec2(844.0, 421.0)      <- synthesized, read a frame later
TRACE win: MouseButton Left Pressed
TRACE ptr: Move ... at Vec2(844.0, 421.0)
TRACE ptr: Press(Primary) at Vec2(844.0, 421.0)
TRACE evt: Press on `1242v0`
TRACE state: ... hovered=["1242v0"] pressed=["New Game Button"]
...
step `menu_newgame: release New Game` begins
TRACE evt: Click on `1242v0`
```

Two facts from it: the press lands on the frame after the beat (not a
system later, and not lost), and `Pressed` bubbles to the NAMED button entity
while the hover map holds an unnamed inner node - so a `Pressed`-based
predicate can key off the name a script already spells.

The same run's startup carries a REAL X pointer event the app did not
synthesize:

```
TRACE win: Focused false
TRACE win: CursorEntered
TRACE win: CursorMoved Vec2(640.0, 360.0)      <- the real Xvfb pointer, screen centre
```

### Reproduction attempts, in order

| Experiment | Runs | Result |
| --- | ---: | --- |
| `menu_newgame` alone, sequential, `DISPLAY=:99` | 40 | 40/40 pass |
| `menu_newgame` x5 concurrent on one display | 40 | 40/40 pass |
| All 5 smoke categories in parallel (the suite's own shape), traced, 6 rounds | 138 | 138/138 pass |

None of the three reproduced it. The ambient trigger needs more load than a
bare example fleet on an idle display - the owner's reproduction was a FULL
workspace suite run, which links and runs the rest of the tests alongside.

### The mechanism, reproduced deterministically

Since the ambient trigger would not come out, the hypothesis was injected
instead. `NOVA_POINTER_JITTER=<n>` (in the same temporary module) writes a
`CursorMoved` at `(10, 10)` every `n` frames the way `bevy_winit` writes a REAL
one - a message, no `Window` write - and nothing else changes.

```sh
DISPLAY=:99 NOVA_AUTOPILOT=1 NOVA_POINTER_TRACE=1 NOVA_POINTER_JITTER=7 \
  cargo run --example menu_newgame --features debug
```

3 runs, 3 failures, and the failure is the owner's line to the character:

```
ERROR autopilot: step `menu_newgame: release New Game` stalled after 90.0s (run 91.6s, state MainMenu)   # here
ERROR autopilot: step `menu_newgame: release New Game` stalled after 90.0s (run 91.7s, state MainMenu)   # owner, 2026-08-05
```

The trace names the step that goes wrong, and it is NOT the press:

```
TRACE evt: Press on `1243v0`      <- the press LANDED on the button
TRACE evt: Release on `66v0`      <- the release resolved against the STRAY position
                                  <- and no Click, so no Activate
```

**Root cause.** The press is fine. `Pointer<Click>` - the only event that
produces `Activate` - is dispatched from `previous_hover_map`
(`bevy_picking/src/events.rs:963`), so ANY pointer event that moves the hover
off the widget between the press beat and the release beat silently cancels
the click. `click_named` warping and pressing in one frame is not the defect;
the defect is that the driven pointer is not authoritative - a real X pointer
event lands in the same stream and outvotes it.

Ambient sources of such an event on a shared Xvfb display, all present in the
CI run and none reproducible on demand here: the startup
`CursorEntered`/`CursorMoved(640, 360)` pair the trace shows arriving at t=0
on an idle box (late under load, it lands mid-click); real enter/leave motion
as other example windows map and unmap; and the REAL pointer warp
`Window::set_cursor_position` triggers, which echoes back asynchronously
(`bevy_winit/src/system.rs:433`).

LIMITATION, stated plainly: the injected event proves the FAILURE MODE and
matches the observed signature exactly. It does not prove WHICH ambient event
fires in CI - 218 runs across three shapes never produced one. A fix must
therefore be robust to the class, not to one source.

### The fix shape, prototyped against the rig

`NOVA_POINTER_PIN=1` re-asserts the driver's last synthesized cursor position
in `First`, AFTER `bevy_winit` wrote the frame's real events and BEFORE
`PickingSystems::Input` consumes them, so the last `Move` in every batch is the
driver's.

| Configuration | Runs | Result |
| --- | ---: | --- |
| `menu_newgame`, jitter | 3 | 3/3 FAIL (release beat stalls) |
| `menu_newgame`, jitter + pin | 4 | 4/4 pass, `Click on 1243v0` restored |
| whole `ui/` category, pin, no jitter | 5 | 5/5 pass - the pin breaks nothing, including `widget_zoo`'s hover/press/drag assertions |
| whole `ui/` category, jitter | 5 | 4/5 FAIL (`editor`, `menu_newgame` stall; `menu_scenarios`, `widget_zoo` panic on their own assertions). `hud_range` is the one that never clicks |
| whole `ui/` category, jitter + pin | 5 | 5/5 pass |

The rig and the prototype are kept at `tasks/20260805-091151/prototype/`
(patch plus its run commands); the working tree is back to clean.

## Ideas

Ranked best first.

### 1. Pin the driven pointer (recommended)

While a run is driven, the driver re-asserts its own cursor position every
frame in `First`, between the real events and picking's read of them. A stray
event is outvoted in the same batch, so `hover_map` and `previous_hover_map`
always agree with where the script pointed.

- Closes the CLASS, not one source; the only idea the rig actually clears.
- One place in `nova_autopilot`, ZERO call-site changes, no script churn.
- Prototyped: 4/4 under the rig, 5/5 on the untouched `ui/` category.
- Cost: a driven app stops responding to a real mouse. That is the intent -
  it also fixes a developer nudging the mouse during a local run.

### 2. Observed precondition on the click beats

`hovered_named` / `pressed_named` predicates in `nova_autopilot::predicate`
(the trace confirms `Pressed` lands on the NAMED entity, so the predicate keys
off the name a script already spells), turning
`.on_enter(click_named(x)).until(frames(SETTLE))` into `.until(pressed_named(x))`.

- Does NOT close the class: a stray can still land between the observed press
  and the release beat.
- Does kill the epic's frame-count anti-pattern at every call site, and turns
  a 90 s mystery stall into a beat that names the widget that never took the
  press. `widget_zoo` already asserts exactly these components by hand.
- Complements 1; not a substitute for it.

### 3. Split every call site into hover-then-click beats

Mechanical, with precedent in `editor.rs`. Rejected as the fix: it does not
survive the rig (the gap it closes is before the press; the failure is after
it), it leaves the trap armed for the next `click_named` call site, and it
grows every script for less protection than 1.

### 4. Defer the press one frame inside `click_named`

Rejected by evidence: the rig shows the press landing correctly every time.
It also needs driver machinery, since `on_enter` is a single `Fn(&mut World)`.

### 5. One Xvfb display per smoke example

Narrows one ambient source in CI only. Does nothing for a local run or for the
startup enter/motion pair, and does not close the class. A possible complement,
never the fix.
