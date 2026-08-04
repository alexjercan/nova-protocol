# Notes: Retire the mainline and POC example runs, reduce screenshots to capture-only

Goal in one line: delete what the roster spike retired and strip the eight
screenshot runs back to producing images - all mechanical, no new content, but
gated on the composed coverage existing first.

## What changes

Before: 26 cataloged examples. Three of them are retired by decision -
`broadside` (352 lines) and `lifeline` (446) assert story wave timings and
object ids against shipped campaign scenarios; `nova_os_rtt_poc` (526) is a
feasibility prototype for a pipeline that shipped. Eight `screenshots/` runs
carry assertions, fps wiring, probe enrollment and hand-rolled elapsed-time
offsets.

After: those three are gone, `examples/gameplay/` is gone, and every
`screenshots/` run is enter -> wait on a predicate -> shoot -> exit.

## Surfaces

| File | Why |
|-|-|
| `examples/gameplay/broadside.rs`, `lifeline.rs` | DELETED with their catalog and smoke entries. |
| `examples/gameplay/` | Directory removed once `20260804-093934` has moved `scenario.rs` and `playable.rs` out. |
| `examples/ui/nova_os_rtt_poc.rs` | DELETED. Its coverage becomes an element test owned by `20260804-094021`, NOT by this task. |
| `examples/screenshots/screenshot_orbit.rs:151,169-173` | `playing_since: Option<f32>` + the `elapsed - playing_since` runway. Deleted. |
| `examples/screenshots/screenshot_juice.rs:205,224-228` | Same pattern. |
| `examples/screenshots/screenshot_combat.rs:231,268-272` | Same pattern. |
| `screenshot_reel`, `screenshot_ui`, `screenshot_sections`, `screenshot_nova_os`, `render_scale_shot` | Reduced to capture producers; no assertions, no fps wiring, no probe enrollment. |
| `Cargo.toml` | Three `[[example]]` blocks deleted; `fps_exempt = ["broadside"]` (:34-35) deleted - `broadside` is its only entry. |
| `tests/examples_smoke.rs` | `GAMEPLAY:43` loses two; `NOT_SMOKED:78` loses `nova_os_rtt_poc`; the `SCREENSHOTS:51` list must survive the reduction. |
| `scripts/gen-web-screenshots.py` | The consumer of every shot. SEE OPEN QUESTIONS - its CLI does not have the flag this task's DoD calls. |

## Data and interfaces

Nothing added. Removed:

```rust
// three copies of this, in orbit/juice/combat
playing_since: Option<f32>,
let playing_since = *script.playing_since.get_or_insert(elapsed);
let t = elapsed - playing_since;
```

and the `fps_exempt` manifest key. The shape a reduced producer keeps:

```rust
nova_screenshot()   // nova_debug::harness:196
// enter -> until(<predicate>) -> shoot -> exit. No assert, no invariants,
// no nova_frametime.
```

## Sketches

Illustrative only.

```diff
-# gameplay/ - full autopilot scenario runs (the timeline/invariant-wired ones).
-[[example]]
-name = "broadside"
-path = "examples/gameplay/broadside.rs"
-
-[[example]]
-name = "lifeline"
-path = "examples/gameplay/lifeline.rs"
```

```diff
-    let playing_since = {
-        let elapsed = world.resource::<Time>().elapsed_secs();
-        *script.playing_since.get_or_insert(elapsed)
-    };
-    let t = elapsed - playing_since;
-    if t > 4.0 { pose_camera(world); }
+// the driver owns the beat: a step whose `until` is the thing the shot
+// needs to be true, not a wall-clock offset from an ad-hoc origin
```

## Shape

```
must land AFTER 20260804-093934 (systems/outcomes exists)
                        |
                        v
  DELETE  broadside, lifeline  ------ their SYSTEM coverage is not dropped:
          nova_os_rtt_poc       ----- chaining/Defeat/Retry/Victory are already
          examples/gameplay/           pinned in nova_menu, nova_scenario and
          fps_exempt                   nova_assets tests; the COMPOSED path
                                       moved to systems/outcomes

  REDUCE  8 screenshots/ runs:
          [enter] -> [until(predicate)] -> [shoot] -> [exit]
          minus assertions, fps wiring, probe enrollment, playing_since

  NOT this task:  *_poc.html move (20260804-003301)
                  the RTT element test (20260804-094021)
```

## Consequences and open questions

- The sequencing is the whole risk. `DEPENDS ON 20260804-093934` exists so the
  tree is never briefly without the composed outcome path. The four SYSTEMS
  themselves stay covered either way - they are already pinned headlessly.
- RESOLVED (owner, 2026-08-04): the flag is `--report`, one name, built and
  owned by `20260724-082856` (already in its Steps, and in the epic's Done
  Means). `--check` was this task's invention and is gone. Neither flag exists
  today - `scripts/gen-web-screenshots.py:568-574` has only `--stage-dir`,
  `--no-icons`, `--self-test` - so this DoD depends on 082856 shipping it.
  Because `--report` is advisory and always exits 0, the criterion is met by
  the absence of a NEW `capturable` gap, not by exit status.
- OPEN: with `screenshots/` out of probe's `--all` (per 093855), the smoke test
  `screenshots_reach_playing_without_panic` becomes the ONLY automated exercise
  of these eight runs. Stripping their assertions is therefore fine, but
  stripping their harness is not - they must still reach Playing and exit
  cleanly, or they go completely unexercised.
- OPEN: `render_scale_shot` is already in both `NOT_SMOKED` and probe's
  `NOT_PROBED` (`spec.rs:8-13`). Once `screenshots/` leaves `--all` wholesale,
  the `NOT_PROBED` entry is redundant. Harmless, but worth deleting with the
  rest rather than leaving a stale exclusion with a stale reason.
- The Story section of TASK.md still says story scenarios are "the most
  volatile content in the repo". The spike's review measured that and it is
  false - `broadside.rs` has 11 commits ever, `lifeline.rs` 6. The retirement
  rationale that survives is the one in the DECISION: an assisted win over 8000
  lines of story RON proves little, and story is tested by players. Worth
  correcting the Story line at planning so the task does not carry a refuted
  premise into implementation.
