# Let the scenario pulse sleep: wake OnUpdate on writes and scheduled times

- STATUS: CLOSED
- PRIORITY: 68
- TAGS: v0.12.0, scenario, events, performance

Stage 3 of `20260820-223059`, taken as the recommendation that task closed on:
build the two WAKE SOURCES, not chains/branching/tags/custom triggers.

`fire_on_update` queues an `OnUpdate` event every frame, and the dispatcher
then walks the whole bucket re-evaluating filters that cannot have changed
their answer. Nothing in the authored language changes; the pulse just stops
firing when nothing moved. The rigidbody analogy is exact - the scenario
sleeps until something wakes it.

## The two wake sources

- A **content variable** some `OnUpdate` filter reads was written since the
  last pulse. `NovaEventWorld::insert_variable` is the single write path
  (world.rs:647), so a dirty set is one line there.
- The **scenario clock crossed a threshold** some filter compares against.
  `GreaterThan(scenario_elapsed, 95.0)` is known at load, so it is scheduled
  rather than polled.

## No RON

Nothing is added to the authored vocabulary. The filters already declare
everything the engine needs, and the lint already walks it
(`collect_condition_vars` / `collect_condition_queries`, lint/scenario.rs:826).
An authored `wake:` list would be a SECOND source of truth that can disagree
with the filters - name two, read three, and the handler silently never fires
on the third. `/create/` does not grow and `content lint` stays whole-program.

## Fail safe, never silent

The profile's default is `EveryFrame`. A filterless `OnUpdate`, an
`Entity`/`Timer` filter on `OnUpdate`, an inline `Query(..)` in an expression,
or a watch that is not the clock all fall back to firing every frame. The
worst case of a missed case is today's behaviour, never a wrong result.

## Coarse on purpose

One decision per scenario, not per handler: the gate either queues the event
or does not. Per-handler gating would mean changing `nova_events`' dispatcher,
which this deliberately does not touch. `ledger_ch3` therefore gains nothing -
five of its handlers read `player_speed`, resampled every frame by definition,
and a speed ladder SHOULD poll.

## The evidence this is sized against

The 22 `OnUpdate` handlers left after stage 2, by what their filters read:

| class | handlers | outcome |
| --- | --- | --- |
| content variables only | 10 | dirty-set wake takes them off the frame |
| content variables + `scenario_elapsed > <literal>` | 7 | schedule covers the clock half |
| `player_speed`, a per-frame sample | 5 | correctly keep polling |

## Content fixes the analysis exposed

Both are `OnUpdate` handlers the stage-2 lint rule misses, because they read
variables as well as the clock:

- `ledger_ch3` handler 5 compares `scenario_elapsed > overspeed_deadline` - a
  keyed timer written by hand, and the last non-literal clock read in the
  mainline. Make it a `TimerStart` + `OnTimerEnd`.
- `lifeline` handlers 3 and 5 carry `GreaterThan(scenario_elapsed, 0.0)`, an
  always-true vestige. Delete it.

A narrow lint rule guards the first: an `OnUpdate` comparing the clock against
anything but a literal is a hand-rolled timer.

## Definition of done

- The pulse sleeps: a live scenario with no writes and no due threshold queues
  no `OnUpdate` event, proven by a test that counts fired events.
- Behaviour is unchanged: every existing scenario rig still passes, and a
  scenario the analyser cannot prove falls back to every frame.
- Both content fixes landed, with the lint rule that stops the first coming
  back.
- Measured before/after on the biggest scenario, so the claim is demonstrated
  rather than asserted.

## What landed

`crates/nova_scenario/src/loader/wake.rs` derives a `WakeProfile` at load, and
`configure_scenario_shape` is now the ONE place a `ScenarioConfig` becomes live
world shape - watches, the entity-query flag, and the wake profile together,
called by the loader and by `test_support` so the nine headless rigs cannot
drift from the game.

`fire_on_update` runs behind `.run_if(scenario_pulse_is_due)`. The read/write
split matters: Bevy run conditions must be read-only, so `is_wake_due()` reads
and `consume_wake()` mutates inside the fired system. A frame that does not
fire therefore leaves the wake reasons standing rather than swallowing them.

Three analyser rules are not obvious from the module:

- What an `OnUpdate` handler WRITES joins what it reads. Without it a counter
  the handler advances itself freezes the moment nothing else writes.
- A `Sequence` step's `until` gate is scanned with the authored handlers. It is
  a real spawned handler, and a gate waiting on the pulse that was not a reason
  to wake would stall its chain forever - `final_tally`'s cast-off is exactly
  that shape.
- A `Conditional` filter recurses. `Not` inverts the ANSWER, not the moments it
  can change, so a threshold under one is still a scheduled wake. Without this
  rule `lifeline` and both `Or`-gated branch milestones fell back to every
  frame.

## Measured

Headless runs, share of frames that queue the event:

| scenario | pulses / frames |
| --- | --- |
| `final_tally` | 308 / 18300 |
| `broadside` | 355 / 13500 |
| `broadside_gunship` | 319 / 12600 |
| `shakedown_run` | 425 / 16800 |
| `ledger_ch1` | 350 / 22500 |
| `ledger_ch3` | every frame |
| `lifeline` | every frame |

Two scenarios stay awake, and both are honest:

- `ledger_ch3` reads `player_speed` in five handlers. A speed ladder is a
  continuous question and SHOULD poll. This was predicted before the work.
- `lifeline` was NOT predicted. It recomputes `relief_remaining = 240 -
  scenario_elapsed` every frame to feed a `HudReadout`. The write joins the
  read set by the rule above, so the scenario wakes itself every frame. The
  value is display-only; binding a readout straight to a query would remove the
  handler, but that is a `HudReadout` change and out of this task's scope.

## Content

Rewritten against the last RELEASE (v0.11.0), all five Ledger chapters:
74 handlers -> 65, 51 latch variables -> 25. Chapter counts now, from the boot
log: ch1 18, ch2 10, ch2b 10, ch3 21, ch4 6.

`ledger_ch3`'s overspeed ladder also fixed a dead end that shipped: under the
stamped-deadline scheme a player easing into the 7..8 band at the deadline was
stuck in state 3 forever - never tripped, never re-armed, one frame of throttle
from death with no fresh countdown. The keyed timer pair resolves the window
one way or the other.

`lifeline`'s `paced_line` no longer emits `scenario_elapsed > 0.0` for the
opening line of an act.

The widened lint rule - an `OnUpdate` comparing the clock against anything but
a literal is a hand-rolled stopwatch - fired 8 times on shipped chapters that
stages 1-2 left alone (ch1 x5, ch2, ch2b, ch4). All 8 are fixed rather than
acked: a permanently-warning lint teaches readers to ignore lint.

## Proof

- `cargo test -p nova_scenario --lib` - 236 passed.
- `cargo test -p nova_authoring` - 77 + 2 + 14 + 1 + 3 + 2 + 2 passed.
- `nova_assets` rigs: `final_tally_claim` 7, `lifeline_convoy` 8,
  `neutralized_ships` 4, `scenario_act_machine` 7, `scenario_branch_choice` 9,
  `scenario_gate_course` 6, `scenario_provocation` 7 - all passed.
- `cargo check --workspace --all-targets` clean; `cargo fmt --all` applied.
- `content lint`: 0 errors, 0 warnings, 0 findings, 13 scenarios
  balance-audited, 1 acked.
- Mutation check: deleting `.run_if(scenario_pulse_is_due)` fails exactly
  `the_pulse_sleeps_until_a_variable_it_reads_is_written` and
  `a_scheduled_time_wakes_the_pulse_once_as_it_is_crossed`.
- All five chapters booted headless and loaded with the handler counts above.
- `gen-portal.py` republishes `the-ledger 1.26.0` (17 files) and
  `gauntlet 1.10.0`.

Not run: the full workspace test suite and Clippy, by standing instruction.
