# Fix the hud_range example smoke: the scripted run never reaches its last beat

- PRIORITY: 90
- TAGS: v0.10.0, bug, examples, testing
- KIND: TASK
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -

## Story

`tests/examples_smoke.rs::ui_reach_playing_without_panic` fails on CI. The
`hud_range` example's backstop fires:

```
thread 'main' panicked at examples/ui/hud_range.rs:340:9:
hud range: the scripted run never finished (ring=true lock=true goto=true drop=false)
```

Pre-existing and NOT caused by the `nova_autopilot` migration: the identical
failure is in run `30768496842` (2026-08-02 21:42), before `8cf34ebf` landed.
It also reproduces on run `30805870861` (2026-08-03).

## Diagnosis (starting point, not confirmed)

Two clocks disagree:

- The script timeline `t` is relative to entering `Playing`, deliberately, so
  a slow load shifts the beats instead of truncating them.
- The backstop at line 338 uses `elapsed`, the autopilot-window clock, and
  fires at `elapsed > 7.5`.

The last beat needs `t > 4.8`. CI logs show the kill beat (`t > 4.4`) firing
and `scenario_elapsed` reaching only ~4.76 when the run ends, so on a loaded
runner the load cost eats the difference between the two clocks and the
window closes before the final beat runs. The assertions themselves never got
a chance to fail - the beat never ran.

## Decision

The fix is NOT a bigger number. The window becomes a RUNWAY and the script owns
completion - the idiom `broadside`, `lifeline`, `menu_scenarios` and
`screenshot_nova_os` already use here (`examples/screenshots/screenshot_nova_os.rs:54-70`).
The two-clock split then stops mattering: the run exits when the last beat
lands, however long the load took.

Rejected: raising `hold` alone (leaves the same fragile arithmetic, one loaded
runner away from the same failure, and pays the full window on every run);
re-basing the backstop on `t` alone (a run that never reaches `Playing` has no
`t`, so the vacuous-pass hole opens).

Loud failure is preserved on three paths:
- runway expiry with the script pending -> `AppExit::error` from the harness
  (`crates/nova_autopilot/src/autopilot.rs:298-306`);
- any premature `AppExit` with the script unfinished -> in-example
  `guard_script_completion` panics naming the unfired beats;
- a beat assertion failing -> panics, unchanged.

Scope: `hud_range` (the failure) and `com_range` (byte-identical defect, latent
at ~3.2 s slack). The four wide-margin fixed-window examples
(`screenshot_juice`, `screenshot_orbit`, `screenshot_combat`, `playable`) are
audited and deliberately left; converting them is a follow-up if the pattern
bites again. Full audit table in `NOTES.md`.

## Steps

- [ ] Reproduce the defect under Xvfb before touching it: run
      `NOVA_AUTOPILOT=1 cargo run --example hud_range --features debug` with the
      load artificially slowed (simplest: temporarily lower `hold` to ~5.5 s, the
      arithmetic equivalent of ~2.5 s extra load) and confirm the
      `never finished (... drop=false)` panic and a non-zero exit. Revert the
      temporary edit. Record the exact method used in `RETRO.md`.
- [ ] `examples/ui/hud_range.rs`: add `.self_completing()` to the
      `AutopilotPlugin` chain and raise `hold(GameStates::Loading, ..)` from
      `8.0` to `30.0` as a runway (comment it as a runway, and note it must stay
      well under `DEFAULT_DEADLINE_SECS` = 120 s so the specific error wins).
      Replace the stale "needs ~4.5s of Playing" comment at :74.
- [ ] `examples/ui/hud_range.rs`: delete the `elapsed > 7.5` backstop (:337-345)
      - the bug itself. In the final beat (`t > 4.8`, :952), after the existing
      assertions, `info!("probe: script complete, exiting")` and
      `world.resource_mut::<harness::HarnessCompletion>().done(harness::AUTOPILOT)`.
      `HudRangeScript.done` keeps its meaning and becomes the guard's flag.
- [ ] `examples/ui/hud_range.rs`: add `guard_script_completion(mut exits:
      MessageReader<AppExit>, script: Option<Res<HudRangeScript>>)` in `Last`,
      registered only under `NOVA_AUTOPILOT`, panicking on an exit with
      `!script.done` and naming `ring/lock/goto/drop` - the detail the deleted
      backstop carried. Mirror `screenshot_nova_os.rs:160-177`.
- [ ] `examples/sections/com_range.rs`: same four edits - `.self_completing()`,
      `hold` `8.0 -> 30.0` (:64), delete the `elapsed > 7.5` backstop (:350-352),
      report done in the `t > 4.3` assert beat (:379), add the guard over
      `ComRangeScript.asserted`.
- [ ] Update both examples' module-header smoke docs: the "exits non-zero ... if
      the script never finishes (e.g. loading ate the window)" line now describes
      the runway/guard contract, not a window.
- [ ] Verify by RUNNING both examples under Xvfb (`cargo check` misses runtime
      panics): each must print `probe: script complete, exiting`, exit 0, and
      finish in roughly load + 5 s rather than idling 8 s.
- [ ] Falsify the guard deliberately: temporarily gate the final beat off (e.g.
      `t > 999.0`), run, confirm a non-zero exit naming the stall rather than a
      vacuous pass; revert. Record in `RETRO.md`.

## Definition of Done

- The example smoke suite passes.
  (cmd: `nix develop --command cargo test --test examples_smoke`)
- Neither converted example still keys a backstop off the autopilot clock.
  (cmd: `! rg -n 'elapsed > 7\.5' examples/ui/hud_range.rs examples/sections/com_range.rs`)
- Both converted examples self-end through the harness rather than idling the
  window.
  (cmd: `rg -n 'self_completing|probe: script complete' examples/ui/hud_range.rs examples/sections/com_range.rs`)
- A run whose script never finishes still fails loudly - proven by the
  deliberate falsification in the last Step, recorded in `RETRO.md`.
  (manual: falsification transcript in `tasks/20260803-143141/RETRO.md`)
- Workspace stays clean.
  (cmd: `nix develop --command cargo fmt --check && nix develop --command cargo check --examples --features debug`)

## Notes

- Found while closing epic `20260802-120019`; it kept that epic's fourth DoD
  command red. The epic closed anyway because the failure predates its work.
- Proof state on base: the smoke DoD command is RED on CI (runs `30768496842`,
  `30805870861`) but GREEN locally on a fast load - the defect is
  load-dependent by construction. Step 1 makes that redness reproducible on
  demand instead of argued from logs; it is the honest local red-on-base.
- No crate changes. Every API used already exists: `self_completing`
  (`autopilot.rs:131`), `HarnessCompletion::done`, `harness::AUTOPILOT`.
- `tests/examples_smoke.rs:296` already accepts the
  `probe: script complete, exiting` sentinel as an alternative to
  `autopilot: cycle complete, no panic`. No test-file edit needed.
- Runway 30 s vs deadline 120 s (`completion.rs:85`): the runway must expire
  first so the error names the script, not the generic collector laggard.
