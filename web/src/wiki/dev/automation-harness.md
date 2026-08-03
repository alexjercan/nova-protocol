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

**Read this page as the crate's contract, not as today's wiring.**
`nova_autopilot` is the extracted home for these drivers, and it is complete and
tested on its own. Nova's own examples and `nova_probe` still run the older
`bevy-common-systems` copy through `nova_debug::harness`, on the legacy `BCS_*`
variables; moving them onto this crate is task `20260802-183403`. Everything
below describes `nova_autopilot` itself and the shape Nova's callers take after
that migration.

## What it drives

| Driver | Does | What Nova will use it for |
| --- | --- | --- |
| `AutopilotPlugin` | Walks a `(state, seconds)` timeline, running a per-frame input closure with full world access | Headless smoke runs, driving a scenario while something else measures |
| `ScreenshotPlugin` | Advances to a target state, settles N frames, captures the primary window to a PNG | One-off visual checks (a phone-width layout regression) |
| `ScreenshotReelPlugin` | Walks an ordered list of beats - apply, settle, capture, wait for the PNG, advance | The web figures and thumbnails, captured at 1920x1080 |
| `completion` | The registration and exit protocol every driver reports to | Ending a run once, when everyone is finished |

`nova_probe` is the layer above: it arms the harness variables, runs an example
and turns the output into a correctness and performance report. It arms the
legacy `BCS_*` names today and moves onto the `NOVA_*` ones with the rest of the
migration.

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
| `NOVA_AUTOPILOT_DEADLINE` | nothing on its own | the completion watcher | seconds before the run gives up and error-exits naming the laggards (default 120) |

`NOVA_SHOT` and `NOVA_REEL` are deliberately separate: a reel run and a one-off
capture must never fight over the same window.

Nova's own run scripts and `nova_probe` still spell these `BCS_*`, left over
from when the drivers lived in `bevy-common-systems`. It is not a mechanical
prefix swap: the deadline's stem changed too, from `BCS_HARNESS_DEADLINE` to
`NOVA_AUTOPILOT_DEADLINE`. The crate's contract is the table above.

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

## How an example opts in

Add the plugins unconditionally and let the env gate them. The app's own state
type is the only thing the driver needs to know about:

```rust
app.add_plugins(
    AutopilotPlugin::new()
        .hold(DemoState::Boot, 0.5)
        .hold(DemoState::Flying, 2.0)
        .input(|world, elapsed| {
            // Every frame, with the total elapsed seconds. Gate it to the
            // state you mean: it runs in every state, and a stray key press
            // is exactly what trips a menu early.
        }),
);
```

`hold` force-sets `NextState` when the timeline reaches that entry, which is why
a Nova example does not `hold(GameStates::Playing, ...)`: the `Loading ->
Playing` transition is asset-gated by the loader, so forcing `Playing` either
fires before the `GameAssets` resource exists (panicking the scene setup that
reads it) or re-enters a state the loader already entered, double-running
`OnEnter(Playing)`. Nova's preset
holds the state BEFORE the gate and lets the loader do its own transition -
`AutopilotPlugin::new().hold(GameStates::Loading, NOVA_AUTOPILOT_SECS)`
(`crates/nova_debug/src/harness.rs`). Hold a state something else is
responsible for entering and you get the same bug.

Then arm it from the shell. `driven_app` is the crate's own example and runs
today; the `scenario` forms are what Nova's examples take once
`20260802-183403` moves them off `BCS_*`:

```sh
NOVA_AUTOPILOT=1 cargo run -p nova_autopilot --example driven_app
NOVA_SHOT=390x844 cargo run --example scenario
NOVA_REEL=1 NOVA_SHOT_DIR=web/figures cargo run --example scenario
```

`crates/nova_autopilot/examples/driven_app.rs` is the end-to-end read: a
self-contained `DefaultPlugins` app with its own three-state machine, driven
`Boot -> Flying -> Done` and exited by the completion protocol, importing no
`nova_*` crate but `nova_autopilot`. Run it with
`NOVA_AUTOPILOT=1 cargo run -p nova_autopilot --example driven_app`;
`crates/nova_autopilot/tests/autopilot_example.rs` runs the same thing headless
and asserts on the exit status and the log lines.

The full API reference is the crate's rustdoc
(`cargo doc -p nova_autopilot --open`); every public item is reachable through
`nova_autopilot::prelude`, and `crates/nova_autopilot/tests/prelude.rs` fails if
a new one is not.
