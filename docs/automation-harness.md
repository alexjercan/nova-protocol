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

**Read this page as the crate's contract.** Nova's own examples run these
drivers, reaching them through the `nova_debug` prelude and the
`nova_debug::harness` presets - the Nova-flavored adapter, not a second
implementation - while `nova_probe` names `nova_autopilot::completion`
directly.

## What it drives

| Driver | Does | What Nova uses it for |
| --- | --- | --- |
| `AutopilotPlugin` | Walks a list of named steps, each advancing when its predicate over the world holds | Headless smoke runs, driving a scenario while something else measures |
| `capture_window` | Writes the primary window to a PNG and acks it into `CaptureLog`. Not a driver - the primitive a script's shot step calls | The web figures and thumbnails, captured at 1920x1080 |
| `completion` | The registration and exit protocol every driver reports to | Ending a run once, when everyone is finished |

`nova_probe_cli` (the game binary's `probe` subcommand, debug feature only) is the host layer above: it arms the
harness variables, spawns a subject as a child process and turns the output
into a correctness and performance report. It arms the variables below -
including a window-sized `NOVA_AUTOPILOT_DEADLINE` for its fps pass, which your
own value overrides. Its in-game half, `nova_probe`, is what the subject
wires to collect the evidence.

A harness run is headless, which for these drivers means a SOFTWARE X server,
and that is not free: presenting a window under Xvfb costs a CPU copy of every
pixel, charged to the frame. Correctness is unaffected and so is any ratio, but
an absolute millisecond off a headless run is the game plus the display server.
[Measuring performance](performance.md) has the size of it and what survives.

The subject is usually an example, which wires the collectors itself. For
`probe scenario` it is the GAME BINARY: `src/main.rs` adds
`nova_probe::NovaProbePlugin` and the `nova_autopilot()` preset under the same
`debug` feature that carries the `probe` subcommand. Both are inert without
their variables, so a plain `cargo run --features debug` behaves exactly as
before - and a scenario becomes measurable without an example file existing for
it.

## The environment contract

Every driver is inert unless its own variable is set, so an app adds the
plugins unconditionally and a normal run pays nothing for them. Setting the
variable is what arms the driver; the value only matters where the table says
so.

| Variable | Arms | Read by | Value |
| --- | --- | --- | --- |
| `NOVA_AUTOPILOT` | the scripted state driver | `AutopilotPlugin` | any (presence only) |
| `NOVA_CAPTURE` | the CAPTURE path of a script that has one: its shot steps write PNGs instead of driving straight through | `capturing()`, which a script reads while building its steps | any (presence only) |
| `NOVA_CAPTURE_DIR` | nothing on its own | `capture_window`, and the scenario `Screenshot` action (`nova_scenario/src/actions/view.rs`) reads it independently | directory that relative capture paths resolve under; absolute paths ignore it |
| `NOVA_AUTOPILOT_DEADLINE` | nothing on its own | the completion watcher | seconds before the run gives up and error-exits naming the laggards (default 120); the RUN-level backstop under a script's own per-step deadlines |

`NOVA_CAPTURE` arms the SHOTS, never a driver. A capturing run therefore sets
`NOVA_AUTOPILOT` too, and one script owns the window: there is no second driver
to fight it over `NextState`.

That is the DRIVER contract in full - four variables. It is not every `NOVA_*`
variable the workspace reads: [Environment variables](environment-variables.md)
indexes the whole set and says which crate owns each, and the measurement
knobs' own values and defaults are tabulated once in `nova_probe`'s crate
rustdoc - `cargo doc --open -p nova_probe` - because the same table serves the
wasm build as URL query parameters. [Measuring performance](performance.md)
covers what they are FOR. A variable that arms nothing is silent, so a run
pinned to a name that is on none of those pages does a plain play-through and
reports nothing wrong.

## Reading the world instead of looking at it

The timeline says what HAPPENED. The world-state snapshot
(`nova_probe::capabilities::snapshot`) says what the world LOOKS like: one JSON
object holding every ship - identity, pose, velocity, aggregate health, mass,
the collapse/defeat flags and its weapon locks - each ship's sections with their
class, pose, health, modifications and magazine state, each section's fixtures
(the skin plates and decor bolted to it), and every torpedo and round in flight
with its owner, damage, remaining lifetime and - for a torpedo - which ordnance
TYPE it is, since two bays on one hull can load different torpedoes that are
identical in every other field.

Use it when a defect would otherwise be judged from a render. A skin bug, a
section that took damage it should not have, a turret that never reloaded: all
of them are one `jq` query away instead of a picture to squint at.

```text
Xvfb :95 -screen 0 1280x720x24 &
NOVA_AUTOPILOT=1 NOVA_PROBE_SNAPSHOT=/tmp/snap.jsonl \
  NOVA_PROBE_SNAPSHOT_FRAMES=600,600 BEVY_ASSET_ROOT="$PWD" DISPLAY=:95 \
  cargo run --example system_turret_gunnery --features debug
jq -S '.ships[0].sections[] | {id, class, health}' /tmp/snap.jsonl
```

Two rules make it a DIFFABLE artifact rather than a dump. Every list is sorted
by a value-derived key, never by entity id or query order, so a respawn that
renumbers entities does not churn the diff. Every float is rounded to four
decimals with `-0.0` normalized, so the last bit of an `f32` does not either.
Two snapshots of one frozen frame are byte-identical, which is what the repeated
frame number above checks.

It is read-only, deliberately. There is no restore: a scenario is already
replayable and is its own checkpoint.

## The completion protocol

One run can carry several collectors - an autopilot timeline, a frame capture -
each finishing on its own clock. A collector that writes `AppExit` on its own
clock discards every other collector's data, and does it silently: a capture
cut short by another collector's exit still writes a plausible file, just a
shorter one. So the exit is NEGOTIATED. Two rules:

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
gestures in `nova_autopilot::input` (`press_key`, `release_key`, `type_text`,
`press_mouse`, `release_mouse`, `move_cursor`, `click_at`), and the Nova-typed
predicates in `nova_debug::harness` (`scenario_variable_is`, `section_gone`,
`player_ship_present`). Anything the vocabulary cannot express is a plain
closure: `Arc::new(|world: &World| ...)`.

Typing is its own gesture because a key has two halves. `press_key` writes the
HELD state (`ButtonInput<KeyCode>`), which is what flight code polls;
`type_text` writes the keyboard MESSAGE carrying the text a keypress produced,
which is what a text field reads. A run that drives a filter or a name field
wants the second - pressing `KeyR` fills nothing in.

**A driven run owns the pointer.** The examples run on a real display, so a
real cursor event - the window manager's enter/motion pair, a developer nudging
the mouse, the echo of the OS-level warp the driver itself performs - lands in
the same stream the synthesized one does. One of those between a press beat and
its release beat CANCELS the click silently: `bevy_picking` dispatches
`Pointer<Click>` from the PREVIOUS frame's hover map, so a pointer that moved
off the widget in between produces a release, no click, and no `Activate` - a
90-second stall on the beat AFTER the one that actually went wrong. So the
autopilot pins its pointer: whenever the window disagrees with the last
position a gesture set, `nova_autopilot::input` puts the pinned position back
in `First`, before the picking backend reads the frame's events. Nothing at a
call site changes, and a script needs no defensive re-hover. The pin holds the
REAL cursor too, since that is what `Window::cursor_position` moves - so a
driven run on your own desktop pulls the mouse back whenever you move it off,
until the run ends.

**A settle predicate over a physics quantity belongs on the physics schedule.**
"The value held still for N frames" is a common way to write "the solve is
done", and on `Update` it does not mean that: avian runs in `FixedPostUpdate`,
so above the fixed rate most `Update` frames carry no solve pass at all and N
unchanged frames can all precede the recompute the beat is waiting for. Sample
in `FixedPostUpdate` after `PhysicsSystems::Prepare`, so one sample is one
pass, and carry any second fact the beat needs (an entity count, say) in the
same sample rather than reading it live off the world. `system_hull_damage`'s
`ComSettle` is the worked example.

The rule is "sample where the quantity is WRITTEN", not "sample on the fixed
schedule" - `system_turret_gunnery`'s `AimSettle` stays on `Update` precisely
because its aim error is produced by `SmoothLookRotationPlugin` in
`PostUpdate`, so consecutive fixed ticks inside one frame would read the same
value and saturate
the streak mid-slew. What that costs is framerate independence, which you buy
back separately: a per-frame delta threshold means a different physical
threshold at every framerate, so compare a RATE against `Time::delta_secs()`.
`AimSettle` is the worked example for that half.

Related, and the other half of the same mistake: **a beat must be strictly
weaker than the assert that follows it.** If the beat's predicate and the next
step's assert share a constant, the assert cannot fail and a real regression
surfaces as a deadline stall on the beat's name instead of the message that
explains it. Gate on the stimulus and the world having stopped changing; assert
on where it ended up. When the two must read the same quantity, give the beat a
margin - the driver enters the next step on the FOLLOWING frame, so a beat that
opened at exactly the assert's threshold on a falling value hands the assert a
failing one (`system_attitude_hold`'s `OFFSET_BEAT_MARGIN_SECS`, and
`system_turret_gunnery`'s `GATE_TRAVEL_BEAT_MARGIN` over a sweep that is not
even monotonic).

Where the invariant has no separate stimulus-side observable - "the torpedo
detonated", "the burn accelerated" - the beat is the stimulus plus a bounded
settle sized off the mechanism (`LAUNCH_SETTLE_SECS`, `BURN_WINDOW_SECS`,
`HIT_SETTLE_SECS`), never the outcome itself. A settle is a runway, so it owes
a derivation on its constant; what it must not do is read the quantity the
assert decides.

### The runway anti-pattern

The shape to avoid is a wall-clock RUNWAY plus one closure that re-derives a
step machine from booleans:

```rust
app.add_plugins(
    AutopilotPlugin::<GameStates>::new()
        .self_completing()
        .hold(GameStates::Loading, 30.0)   // a runway, unrelated to Loading
        .input(script),                    // every frame, in every state
);
app.add_systems(Last, guard_script_completion);

fn script(world: &mut World, elapsed: f32) {
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

Everything in it is a symptom: the `playing_since` offset exists because the
closure's clock is the RUN's and not the beat's, the booleans exist because a
closure has no notion of having advanced, and the hand-rolled guard exists
because nothing else knows the runway expired with beats unplayed. Each is
carried by the step list for free.

Write the beats themselves, each waiting on the world
(`examples/systems/system_hull_damage.rs`):

```rust
app.add_plugins(
    AutopilotPlugin::<GameStates>::new()
        .step("load the rig")
        .enter(GameStates::Loading)
        .until(player_ship_present())   // not "30 seconds should do it"
        .deadline(15.0)
        .add()
        .step("spin the ship")
        .on_enter(apply_spin)
        .add()
        .step("kill the controller section")
        .on_enter(kill_frontmost_section)
        .until(section_gone("controller"))  // the real despawn, not a guess
        .deadline(6.0)
        .add()
        // Last beat: the driver reports done after it, so the run ends on the
        // assertion instead of idling out the rest of a runway.
        .step("assert the com follows the surviving sections")
        .on_enter(assert_com_follows_sections)
        .add(),
);
```

There is no runway, and a stalled step is the driver's job to report, by name.

### Capturing: one idiom

There is one way to take a screenshot from a script, and it is a step. A shot
step calls `shoot(world, "name.png")` from `on_enter`, so the act, the framing
and the shot it produces read top-to-bottom in the step list:

```rust
.step("frame the planetoid")
.on_enter(|world| pose_camera(world, EYE, LOOK))
.until(frames(SETTLE_FRAMES))
.add()
.step("shoot wiki-gravity.png")
.on_enter(|world| shoot(world, "wiki-gravity.png"))
.until(shot_written("wiki-gravity.png"))
.deadline(SHOT_DEADLINE_SECS)
.add()
```

`shoot` is its own gate: it captures only when `NOVA_CAPTURE` is set, so the
SAME script is both the capture run and the smoke run.

`shoot` is asynchronous (the PNG lands at the end of a later frame), which is
why the shot step holds instead of ending immediately: move the camera in the
same frame and the pending capture renders the NEXT framing. It holds on the
ACK, not on a guessed number of frames - `capture_window` records the path in
`CaptureLog` once the write completes, and `shot_written` reads that. On the
smoke path, which shoots nothing, the same predicate holds immediately, so a
script never branches its step timing on `capturing()`.

That leaves ONE settle in a capture script, the same on both paths:
`SETTLE_FRAMES`, the frames a beat needs to come to rest before its shot. The
per-example splits that preceded it were each carrying the write latency on top
of the stillness, which is why they disagreed.

The scene dressing is separate from the steps and lives in `nova_debug::harness`
too: `force_capture_resolution` (a known 16:9 for every shot in the fleet),
`hide_dev_overlays` / `hide_hud`, and `freeze_bodies` for a posed set that must
not drift between framings.

Do not add a second capture idiom beside it. A driver that walks its own list
of shots builds that list away from the script producing the state each shot
frames, which puts timing and framing in different files - and they drift. A
range that only wants ONE settled picture of itself does not get a driver
either: `nova_screenshot(script)` appends the settle-and-shoot beat to the
script it already has.

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
NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=target/shots \
  cargo run --example system_scenario_grammar --features debug
NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=target/shots \
  cargo run --example screenshot_gravity --features debug
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
`nova_autopilot::prelude`.

## Find it in the code

- Driver: `AutopilotPlugin` - `crates/nova_autopilot/src/autopilot.rs`;
  `capture_window` - `crates/nova_autopilot/src/capture.rs`.
- Protocol and vocabulary: completion -
  `crates/nova_autopilot/src/completion.rs`; predicates -
  `crates/nova_autopilot/src/predicate.rs`; gestures -
  `crates/nova_autopilot/src/input.rs`.
- Nova adapter: the `nova_autopilot()` and `nova_screenshot()` presets,
  `shoot`, scene dressing - `crates/nova_debug/src/harness.rs`.
- Host layer: the `probe` subcommand - `crates/nova_probe_cli/src/native.rs`;
  in-game half: `NovaProbePlugin` -
  `crates/nova_probe/src/capabilities/mod.rs`. What the capture measures and
  how to read it: [Measuring performance](performance.md).
- End-to-end example: `crates/nova_autopilot/examples/driven_app.rs`.
- API detail: `cargo doc --open -p nova_autopilot`.
