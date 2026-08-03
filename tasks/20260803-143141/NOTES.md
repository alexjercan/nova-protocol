# Notes: fix the hud_range example smoke - the scripted run never reaches its last beat

## What changes

Before: `hud_range` runs on a fixed 8 s autopilot window while its beats are
timed from entering `Playing`. On a loaded CI runner the load eats the
difference, the window closes before the `t > 4.8` beat, and the hand-rolled
backstop at `elapsed > 7.5` panics:

```
hud range: the scripted run never finished (ring=true lock=true goto=true drop=false)
```

The assertions never ran; the run failed on arithmetic, not on a HUD
regression. `tests/examples_smoke.rs::ui_reach_playing_without_panic` fails.

After: the window becomes a RUNWAY, not the finish line. The script owns
completion - it reports done when its last beat lands, and the run exits then,
however long the load took. A run that never reaches `Playing`, or stalls
mid-script, still fails loudly: the runway expiry is an `AppExit::error` naming
the stall, and an in-example guard panics on any premature exit with the script
unfinished.

Net effect on the smoke suite: hud_range passes regardless of load cost, and a
finishing run gets SHORTER (it exits at its last beat, ~5 s of Playing, instead
of idling out the window).

This is not a new mechanism. It is the idiom four examples in this repo already
use - `broadside`, `lifeline`, `menu_scenarios`, `screenshot_nova_os` - applied
to the two examples still on the fixed-window pattern.

## Surfaces

| File | Why |
| --- | --- |
| `examples/ui/hud_range.rs` | The failing example. Convert to `self_completing()`; drop the `elapsed`-clock backstop; add the completion report + guard. |
| `examples/sections/com_range.rs` | Byte-identical bug (Step 3 of the task): `hold(Loading, 8.0)`, beats to `t > 4.3`, backstop `elapsed > 7.5`. Same conversion. |
| `tasks/20260803-143141/RETRO.md` | Records the deliberate falsification the DoD demands. |

Read-only, for the audit line below (no edits expected):
`examples/gameplay/playable.rs`, `examples/screenshots/screenshot_combat.rs`,
`examples/screenshots/screenshot_juice.rs`,
`examples/screenshots/screenshot_orbit.rs`.

No crate changes. `nova_autopilot` already provides everything needed:
`AutopilotPlugin::self_completing`, `HarnessCompletion::done`, and the
runway-expiry error exit (`crates/nova_autopilot/src/autopilot.rs:298`).

## Data and interfaces

Nothing new is added to any crate. The example uses existing API:

- `nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::self_completing(self) -> Self`
  - marks the timeline a runway; expiry with the script pending writes
    `AppExit::error` (`autopilot: timeline expired but the self-completing
    script never reported done`).
- `nova_protocol::nova_debug::harness::HarnessCompletion::done(&mut self, who: &str)`
  - called with `harness::AUTOPILOT` from the final beat.
- New example-local system, mirroring
  `screenshot_nova_os::guard_script_completion`:

```rust
fn guard_script_completion(mut exits: MessageReader<AppExit>, script: Option<Res<HudRangeScript>>)
```

`HudRangeScript` keeps every field it has; `done` becomes the completion flag
the guard and the report both read. No signature in the example's helper
functions (`player_root`, `indicator_state`, `readout_value`, ...) changes.

Smoke contract: `tests/examples_smoke.rs` already accepts either sentinel -
`autopilot: cycle complete, no panic` OR `probe: script complete, exiting`
(`tests/examples_smoke.rs:296`). The converted examples emit the second. No
test file edit is required for the sentinel; the suite is the proof.

## Sketches

Illustrative, not the patch.

`examples/ui/hud_range.rs`, plugin wiring:

```diff
-        // Not the stock nova_autopilot(): the scripted timeline needs ~4.5s
-        // of Playing, so hold a longer total window than the 6s preset.
         app.add_plugins(
             harness::AutopilotPlugin::<GameStates>::new()
-                .hold(GameStates::Loading, 8.0)
+                // Script-owned completion: the runway only has to outlast the
+                // slowest plausible load plus the ~5 s script, and costs
+                // nothing when the script finishes early.
+                .self_completing()
+                .hold(GameStates::Loading, RANGE_RUNWAY_SECS)   // 30.0
                 .input(autopilot_script),
         );
+        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
+            app.add_systems(Last, guard_script_completion);
+        }
```

Backstop removal (the two-clock bug itself):

```diff
-    if elapsed > 7.5 && !world.resource::<HudRangeScript>().done {
-        let script = world.resource::<HudRangeScript>();
-        panic!("hud range: the scripted run never finished (...)");
-    }
     if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
         return;
     }
```

Final beat reports done instead of just flipping a flag:

```diff
         info!("hud range: PASS - indicators track their anchors and hide when they die");
+        info!("probe: script complete, exiting");
+        world.resource_mut::<harness::HarnessCompletion>().done(harness::AUTOPILOT);
     }
```

## Shape

```
  autopilot clock (elapsed)          script clock (t = elapsed - playing_since)
  |                                  |
  0 ---- load ---> Playing ----------0---- beats ----4.8 done
  |<-- varies with runner load -->|  |<-- fixed ~5 s -->|
  |                                                     |
  |                                              BEFORE: window closed at 8.0,
  |                                              backstop panicked at 7.5 when
  |                                              load > ~2.7 s
  |
  '--- AFTER: runway 30 s, exit fires HERE (script done) --------------->

  exit paths after the change
  ---------------------------
  script reports done  -> HarnessCompletion: pending empties -> AppExit::Success
  runway expires first -> autopilot: AppExit::error("script never reported done")
  premature AppExit    -> guard_script_completion panics, naming the unfired beats
  beat assertion fails -> panic, as today (unchanged)
```

## Consequences and open questions

Costs and tradeoffs:

- The runway must outlast the worst plausible load. 30 s is arbitrary-generous
  and free (an early finish exits early), but it is still a number; a load
  pathological beyond it turns a hang into an error exit, which is correct.
- The completion deadline watcher (`NOVA_AUTOPILOT_DEADLINE`, default 120 s)
  stays the outer bound. The runway must stay well under it so the specific
  error message wins over the generic one.
- Panic message quality changes shape: the old backstop named the unfired
  flags in one string. The guard reproduces that; the runway-expiry path gives
  the harness's generic message plus the guard's detail. Keeping the flag
  detail in the guard is deliberate, not incidental.
- `probe run` on these examples now measures a shorter window for hud_range.
  Frametime capture is a registered collector, so the exit still waits for it -
  the protocol handles this, but it is worth an eye during verify.

Audit of the sibling fleet (Step 3), for the record:

| Example | Window | Last beat | Slack | Verdict |
| --- | --- | --- | --- | --- |
| `hud_range` | 8.0 | t > 4.8 | ~2.7 s | BROKEN - fix |
| `com_range` | 8.0 | t > 4.3 | ~3.2 s | same bug, latent - fix |
| `screenshot_juice` | 8.0 | t > 1.3 | ~6.7 s | wide, leave |
| `screenshot_orbit` | 12.0 | t > 6.0 | ~6 s | wide, leave |
| `screenshot_combat` | 14.0 | t > 7.0 | ~7 s | wide, leave |
| `playable` | 24.0 | event-chained, ~1 s | ~22 s | wide, leave |
| `broadside`, `lifeline`, `menu_scenarios`, `screenshot_nova_os` | runway | self-ending | n/a | already the target idiom |

Leaving the four "wide" ones on the fixed window is a scope call, not a claim
that the pattern is sound there. They carry the same two-clock split with a
margin big enough that no run has hit it. Converting them is a follow-up task
if the pattern bites again.

Open questions:

- None blocking. Assumption recorded: the fix converts hud_range AND com_range
  (Step 3 names the sibling sweep, and com_range is the one true copy of the
  defect), and does not touch the four wide-margin examples.
- Falsification method for the DoD's "fails loudly" half is chosen at work
  time: the cheapest is a temporary edit that never fires the last beat (or
  never reaches `Playing`) and confirming a non-zero exit naming the stall.
  Recorded in RETRO.md, not landed.
