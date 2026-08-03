# Notes: Make nova_autopilot predicate-driven

## What changes

Before: an automation script is `hold(state, secs)` steps plus one per-frame
closure. The closure re-derives everything itself - a `Script` resource of
booleans, a `playing_since` offset so beats are relative to `Playing`, `if t >
0.5 && !script.raised` guards, a `panic!` near the window's end listing the
booleans, an `AutopilotLoop` reader plus a reload-gate poll for looped capture
runs, and a `hold(GameStates::Loading, 30.0)` "runway" that means "give me 30
seconds", not "be in Loading". The driver knows nothing about the beats, so a
stall reports a boolean tuple from the example, not a step name.

After: a script is a named step list. A step optionally enters a state, runs an
entry action, runs a per-frame action, and ADVANCES when its predicate over the
world holds. `hold(state, secs)` still exists and is sugar for
`step().enter(state).until(elapsed(secs))`. A step that never satisfies its
predicate inside its deadline aborts naming the step, its elapsed time, and the
last observed state. Looping is a declared loop point plus a reset hook.

Player-visible behavior is unchanged - this is dev/CI tooling. What changes is
that a smoke run is timing-independent (llvmpipe frame collapse stops
truncating beats) and a failure says which beat stalled.

## Surfaces

| File | Why |
| --- | --- |
| `crates/nova_autopilot/src/autopilot.rs` | The driver. Step list, predicates, per-step deadlines, loop point, diagnostics. All the change lands here. |
| `crates/nova_autopilot/src/lib.rs` | Prelude re-exports for the new public items; crate docs describe the step model. |
| `crates/nova_autopilot/examples/driven_app.rs` | The crate's own readable end-to-end example; becomes the reference script. |
| `crates/nova_autopilot/tests/autopilot_example.rs` | Headless assertion on exit status + log lines; new stall diagnostics assert here. |
| `crates/nova_debug/src/harness/` | The Nova adapter. Nova-typed predicate helpers (scenario variable, state, entity present) live here, not in the crate. |
| `examples/**` | Callers. Migrated in this task only where `20260802-120029` does not rebuild them. |
| `crates/nova_probe/src/bin/probe/native/env.rs` | Owns `NOVA_AUTOPILOT_DEADLINE` sizing; per-step deadlines must not fight the run-level one. |

## Data and interfaces

```rust
/// Advance condition: pure observation over the world.
pub type Predicate = dyn Fn(&World) -> bool + Send + Sync;

pub struct Step<S> {
    name: &'static str,
    enter: Option<S>,          // set NextState on entry (None: stay)
    on_enter: Option<Arc<ActionFn>>,   // fn(&mut World)
    each: Option<Arc<FrameFn>>,        // fn(&mut World, f32) - elapsed IN STEP
    until: Arc<Predicate>,
    deadline: Option<f32>,     // seconds in step; None -> plugin default
}

impl<S: States + FreelyMutableState> AutopilotPlugin<S> {
    pub fn step(self, name: &'static str) -> StepBuilder<S>; // -> .enter/.on_enter/.each/.until/.deadline/.add()
    pub fn hold(self, state: S, seconds: f32) -> Self;       // kept: sugar over elapsed()
    pub fn loop_from(self, name: &'static str) -> Self;      // declared loop point
    pub fn on_loop(self, f: impl Fn(&mut World)) -> Self;    // reset hook
    pub fn observe(self, f: impl Fn(&World) -> String) -> Self; // state line for diagnostics
}

// Predicate vocabulary (crate-level, Nova-free):
pub fn elapsed(seconds: f32) -> Arc<Predicate>;
pub fn in_state<S: States>(state: S) -> Arc<Predicate>;
pub fn resource_where<R: Resource>(f: impl Fn(&R) -> bool) -> Arc<Predicate>;
pub fn any_entity<F: QueryFilter>() -> Arc<Predicate>;
pub fn frames(n: u32) -> Arc<Predicate>;          // settle
pub fn and/or/not(..) -> Arc<Predicate>;

// Input synthesis actions (generic Bevy input; keyboard + mouse only)
pub fn press(key: KeyCode) -> impl Fn(&mut World);
pub fn move_cursor(pos: Vec2) -> impl Fn(&mut World);   // window cursor + CursorMoved
pub fn click_at(pos: Vec2, button: MouseButton) -> impl Fn(&mut World);
pub fn drag(from: Vec2, to: Vec2, button: MouseButton) -> impl Fn(&mut World);
```

Pointer synthesis is what lets a `ui/` example drive real widgets - the NOVA OS
computer, the RTT screen, buttons - instead of asserting a tree it never
touched. It writes the same window cursor position + `CursorMoved` +
`ButtonInput<MouseButton>` a real pointer produces, in the same
post-`InputSystems` slot the keyboard poking already uses. Gamepad and touch
are deliberately absent until a caller needs them.

`self_completing()` disappears as a mode: a script ends when its last step
finishes, and a stalled step is the abort. `AutopilotLoop` stays as the
message (probe/examples read it) but the reset hook covers the common case.

## Sketches

Illustrative only.

```diff
-app.add_plugins(AutopilotPlugin::<GameStates>::new()
-    .self_completing()
-    .hold(GameStates::Loading, 30.0)
-    .input(com_range_script));
+app.add_plugins(AutopilotPlugin::<GameStates>::new()
+    .step("await scenario").until(scenario_loaded()).add()
+    .step("spin").on_enter(press(KeyCode::KeyO)).until(spinning()).add()
+    .step("kill controller").on_enter(press(KeyCode::KeyK))
+        .until(section_gone("controller")).deadline(10.0).add()
+    .step("assert com").on_enter(assert_com_tracks_centroid).until(frames(1)).add());
```

```diff
-if st.state_elapsed >= hold { st.index += 1; ... }
+if (step.until)(world) { advance }
+else if step_elapsed > deadline {
+    error!("autopilot: step `{}` stalled after {:.1}s; observed: {}", step.name, step_elapsed, observed);
+    world.write_message(AppExit::error());
+}
```

## Shape

```
AutopilotPlugin<S>            (nova_autopilot, bevy-only)
  steps: [Step { name, enter, on_enter, each, until, deadline }]
  loop_from / on_loop / observe
        |
        v  PreUpdate, after InputSystems
  autopilot_drive::<S>  --- advance? ---> NextState<S> + probe marker
        |                     |no
        |                     +-- deadline? --> error!(step) + AppExit::error
        v last step
  completion::HarnessCompletion.done(AUTOPILOT)
        |                                  ^
        +-- others_pending? -> loop_from ---+
                                            |
                              watcher: all done -> AppExit::Success

Nova-typed predicates (scenario var, GameStates, EntityId) -> nova_debug::harness
```

## Consequences and open questions

- Cost: one rewrite of the only automation driver plus every caller. The old
  `(state, secs)` timeline survives as `hold`, so callers that only held states
  are a no-op migration; the scripted examples are a real rewrite. That is why
  `20260802-120029` rebuilds them rather than porting them line by line.
- Forecloses: a serialized DSL. Predicates are closures with `&World`; they
  cannot be authored in RON without a whole expression language.
- Per-step deadlines interact with `NOVA_AUTOPILOT_DEADLINE`: the run-level
  watcher deadline must stay above the sum of step deadlines, or the generic
  hang detector wins the race and the diagnostic is lost. Probe sizes the
  run-level one from the fps window today.
- Open: whether `each` needs step-relative or run-relative elapsed (sketch says
  step-relative; scripts that ramp input over the whole run may want both).
- Open: how much predicate vocabulary belongs in the crate. `resource_where`
  and `any_entity` are generic; anything that names a Nova component must live
  in the `nova_debug` adapter to keep the crate `bevy`-only.
- Open: whether the reset hook can fully replace the reload-gate poll in
  `playable` (`capture_reloading`/`capture_reload_end` in `nova_probe`) or
  whether a "scene is live again" predicate is still the caller's job.
