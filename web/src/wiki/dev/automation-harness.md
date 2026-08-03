# The automation harness

How Nova Protocol drives itself without a human at the keyboard: a scripted
autopilot walks the game through its states, screenshot drivers capture the
result, and a completion protocol decides when the run is over. It is what
makes headless verification, the web figures and the performance reports
repeatable rather than a manual pass.

All of it lives in the `nova_autopilot` crate, which depends on `bevy` alone -
no `nova_*` crate, no `avian3d`. It ships the drivers and the protocol; it does
not ship anything Nova-specific. The adapters that know about Nova (scenario
presets, camera posing, freezing rigid bodies, hiding the dev overlay) stay in
`nova_debug` and reach in through caller-supplied closures. The drivers are
generic over the app's state type, and that generic is what keeps
`GameStates` - and with it the whole game dependency tree - out of the crate.

**Read this page as the crate's contract.** Nova's own examples and
`nova_probe` all run these drivers, reaching them through the `nova_debug`
prelude and the `nova_debug::harness` presets - the Nova-flavored adapter, not a
second implementation. `tests/examples_smoke.rs` fails any example that names a
driver unqualified, because the shared `bevy_common_systems` prelude still
exports same-named types that would resolve to an inert twin.

## What it drives

| Driver | Does | What Nova uses it for |
| --- | --- | --- |
| `AutopilotPlugin` | Walks a list of named steps, each advancing when its predicate over the world holds | Headless smoke runs, driving a scenario while something else measures |
| `ScreenshotPlugin` | Advances to a target state, settles N frames, captures the primary window to a PNG | One-off visual checks (a phone-width layout regression) |
| `ScreenshotReelPlugin` | Walks an ordered list of beats - apply, settle, capture, wait for the PNG, advance | The web figures and thumbnails, captured at 1920x1080 |
| `completion` | The registration and exit protocol every driver reports to | Ending a run once, when everyone is finished |

`nova_probe` is the layer above: it arms the harness variables, runs an example
and turns the output into a correctness and performance report. It arms the
variables below - including a window-sized `NOVA_AUTOPILOT_DEADLINE` for its fps
pass, which your own value overrides.

## The environment contract

Every driver is inert unless its own variable is set, so an app adds the
plugins unconditionally and a normal run pays nothing for them. Setting the
variable is what arms the driver; the value only matters where the table says
so.

| Variable | Arms | Read by | Value |
| --- | --- | --- | --- |
| `NOVA_AUTOPILOT` | the scripted state driver | `AutopilotPlugin` | any (presence only) |
| `NOVA_SHOT` | the single settled-frame capture - but it is ignored when `NOVA_AUTOPILOT` is also set: both drivers write `NextState`, so the autopilot wins and `ScreenshotPlugin` stands down with a warning | `ScreenshotPlugin` | `WxH` (for example `390x844`) overrides the window size; anything else is a plain toggle |
| `NOVA_REEL` | the multi-shot reel | `ScreenshotReelPlugin` | any (presence only) |
| `NOVA_SHOT_DIR` | nothing on its own | `ScreenshotReelPlugin` and `capture_window` | directory that relative beat paths resolve under; absolute paths ignore it |
| `NOVA_AUTOPILOT_DEADLINE` | nothing on its own | the completion watcher | seconds before the run gives up and error-exits naming the laggards (default 120); the RUN-level backstop under a script's own per-step deadlines |

`NOVA_SHOT` and `NOVA_REEL` are deliberately separate: a reel run and a one-off
capture must never fight over the same window.

The table above is the whole contract. These names replaced an older set from
when the drivers lived in `bevy-common-systems`, and the swap was not purely a
prefix change - the deadline's stem moved too. A scripted run still pinned to
the old names arms nothing and silently does a plain play-through, so check
yours against this table; the CHANGELOG's breaking entry spells out the old
spellings.

## The completion protocol

One run can carry several collectors - an autopilot timeline, a frame capture,
a reel - each finishing on its own clock. Before the protocol, whichever
finished first wrote `AppExit` and discarded everyone else's data; that is how
an 11-frames-short capture silently lost 229 samples. So the exit is negotiated
instead. Two rules:

1. **Register before the run starts.** A collector calls
   `completion::register` from its `Plugin::build`, behind its own armed check.
   Nothing joins later, and an unarmed collector must not join at all - it
   would hold the exit open until the deadline.
2. **The app exits only when every registrant reports done.** A collector calls
   `HarnessCompletion::done` and never writes `AppExit::Success` itself; the
   watcher writes it once the pending set empties.

An error exit is the exception: a collector that genuinely fails (a screenshot
that cannot save, a stalled script) writes `AppExit::error` directly, because
an abort is not a completion and must not wait for anyone. The deadline backstop
does the same, naming the collectors still pending, so a supervisor reads
"capture never completed" in the log instead of watching a silent hang.

## Writing a script

A script is a list of STEPS. A step is a name plus four optional parts and one
required one:

| Part | Meaning |
| --- | --- |
| `name` | what a log line and a stall message call this beat |
| `enter` | the state to set on entry |
| `on_enter` | a world action run once, on entry (a synthesized gesture, a scenario poke) |
| `each` | a world action run every frame, with the IN-STEP elapsed seconds |
| `until` | the predicate that advances the step |
| `deadline` | in-step seconds after which an unsatisfied `until` ABORTS the run, naming the step |

The step advances the first frame `until` holds. **Name a step after what it is
waiting FOR, not after what it pokes** - the name is what a stall message
carries, and "stalled on `lock the prey`" is a diagnosis where "stalled after 30
seconds" is a shrug.

Elapsed time is one predicate among many, so `hold(state, secs)` - enter a
state, wait N seconds - is sugar for
`step("hold:<state>").enter(state).until(elapsed(secs))` rather than a second
mechanism. The vocabulary is in `nova_autopilot::predicate` (`elapsed`,
`frames`, `state_is`, `resource_where`, `any_entity`, `and`, `not`), the
gestures in `nova_autopilot::input` (`press_key`, `release_key`, `press_mouse`,
`release_mouse`, `move_cursor`, `click_at`), and the Nova-typed predicates in
`nova_debug::harness` (`scenario_variable_is`, `section_gone`,
`player_ship_present`). Anything the vocabulary cannot express is a plain
closure: `Arc::new(|world: &World| ...)`.

### Before and after: the `com_range` script

The old shape was a wall-clock runway plus one closure that re-derived a
step machine from booleans:

```rust
app.add_plugins(
    AutopilotPlugin::<GameStates>::new()
        .self_completing()
        .hold(GameStates::Loading, 30.0)   // a runway, unrelated to Loading
        .input(com_range_script),          // every frame, in every state
);
app.add_systems(Last, guard_script_completion);

fn com_range_script(world: &mut World, elapsed: f32) {
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    // Re-derive a step-relative clock by hand, because `elapsed` is the run's.
    let playing_since = { /* get_or_insert into a script resource */ };
    let t = elapsed - playing_since;
    if t > 1.0 && !script.spun { script.spun = true; apply_spin(world); }
    if t > 2.0 && !script.killed_controller { /* ... */ }
    // ... and a hand-rolled panic if the runway expires with beats unplayed.
}
```

The new shape is the beats themselves, each waiting on the world:

```rust
app.add_plugins(
    AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())   // not "30 seconds should do it"
        .deadline(30.0)
        .add()
        .step("spin the ship")
        .on_enter(apply_spin)
        .until(elapsed(1.0))
        .add()
        .step("kill the controller section")
        .on_enter(kill_frontmost_section)
        .until(section_gone("controller"))  // the real despawn, not a guess
        .deadline(10.0)
        .add()
        .step("settle after the losses")
        .until(elapsed(1.5))
        .add()
        // Last beat: the driver reports done after it, so the run ends on the
        // assertion instead of idling out the rest of a runway.
        .step("assert the com follows the surviving sections")
        .on_enter(assert_com_follows_sections)
        .add(),
);
```

What the rewrite deleted, in every migrated script: the `playing_since` offset,
the beat booleans, the per-example `AppExit` guard that panicked when the runway
expired with beats unplayed, and the runway itself. A step that stalls is now
the driver's job to report, by name.

### Deadlines

A step's `deadline` is IN-STEP seconds and is unset by default, leaving
`NOVA_AUTOPILOT_DEADLINE` as the run-level backstop for a script that hangs
somewhere without one. Set a deadline where a stall is worth NAMING, and **keep
the sum of a script's deadlines under the run-level value** - otherwise the
generic hang detector wins the race and the named-step diagnostic is lost. That
ordering is documented, not enforced: the run-level value comes from the harness
that launches the process, which the crate cannot see.

### Do not `enter` a state something else owns

`enter` force-sets `NextState`, which is why a Nova script does not enter
`GameStates::Playing`: the `Loading -> Playing` transition is asset-gated by the
loader, so forcing `Playing` either fires before the `GameAssets` resource
exists (panicking the scene setup that reads it) or re-enters a state the loader
already entered, double-running `OnEnter(Playing)`. Enter the state BEFORE the
gate and let the loader do its own transition, then wait on something the load
produced - `player_ship_present()`, a seeded scenario variable. Nova's
`nova_autopilot()` preset (`crates/nova_debug/src/harness.rs`) is the wall-clock
fallback for examples with nothing observable to wait on.

Then arm it from the shell - `driven_app` is the crate's own example, the
`scenario` forms are Nova's:

```sh
NOVA_AUTOPILOT=1 cargo run -p nova_autopilot --example driven_app
NOVA_SHOT=390x844 cargo run --example scenario
NOVA_REEL=1 NOVA_SHOT_DIR=web/figures cargo run --example scenario
```

`crates/nova_autopilot/examples/driven_app.rs` is the end-to-end read: a
self-contained `DefaultPlugins` app with its own state machine, driven through
named predicate steps and exited by the completion protocol, importing no
`nova_*` crate but `nova_autopilot`. Run it with
`NOVA_AUTOPILOT=1 cargo run -p nova_autopilot --example driven_app`;
`crates/nova_autopilot/tests/autopilot_example.rs` runs the same thing headless
and asserts on the exit status and the log lines.

The full API reference is the crate's rustdoc
(`cargo doc -p nova_autopilot --open`); every public item is reachable through
`nova_autopilot::prelude`, and `crates/nova_autopilot/tests/prelude.rs` fails if
a new one is not.
