# Transition pacing - design record

Task 20260717-163050, spike tasks/20260717-155740/SPIKE.md (option E),
carrying the USER DIRECTIVE: "add to the pacing by doing linger false in
some cases maybe with a time delay".

## What shipped

- NextScenarioActionConfig.delay: Option<f32>: the delayed non-lingering
  cut. The action arms NovaEventWorld.next_scenario_delay (a Timer,
  reset by clear()); state_to_world ticks it on the world's VIRTUAL
  clock (a player pausing holds the cut) and switches only at expiry.
  The tick deliberately does NOT early-return while waiting: the
  command-queue flush after the switch block must keep running through
  the delay window or every queued spawn/effect would starve - caught
  by reading the system's tail before committing, not by a test.
- OutcomeActionConfig.auto_advance_secs: Option<f64>: the timed banner.
  A nova_menu system ticks Time<Real> (the overlay pauses virtual time;
  the wall clock is the one still moving) while the outcome carries the
  field and a LINGERING chain waits, then calls release_lingering_next -
  the Continue button's exact path. Optional Time<Real> param: the
  headless menu rigs run without TimePlugin.
- content_lint WARN: Outcome + non-lingering NextScenario in one handler
  (undelayed = swallowed overlay, NovaEventWorld::clear's documented
  footgun; delayed = the overlay's pause freezes the cut clock). The
  lingering pair is the sanctioned shape.

## Clock choices (the load-bearing decisions)

- Delayed cut: VIRTUAL time - pausing must hold an un-overlaid cut (a
  scenario switching under the ESC menu would be hostile).
- Timed banner: REAL time - the overlay itself freezes virtual time, so
  only the wall clock can advance it.

## Verification

- a_delayed_cut_holds_then_switches (fail-first: today's instant cut
  fails the first assert), clear_drops_the_pending_delayed_cut,
  outcome_with_hard_switch_in_one_handler_warns (both trap shapes warn,
  the lingering pair clean), transition_pacing_ron_parses_and_defaults,
  auto_advance_releases_the_lingering_switch_after_real_seconds (+ the
  no-field-waits-forever delivery guard). nova_menu 62/62, nova_scenario
  --features serde green, constructor sweep across 7 crates/files
  (workspace --all-targets green), gen_content no-op (serde skip means
  the committed RON is unchanged), content_lint clean.

## Post-review addenda (Round 1)

- R1.1 (MAJOR): authored absurd durations (1e30 delay, 1e300
  auto_advance) would have PANICKED Timer::from_seconds at runtime -
  both construction sites now finite-check and cap (consts in actions.rs,
  runtime 300s), with panic-regression tests and range lint arms. The
  sibling dwell field had this treatment from day one; these two missed
  it because the clamp pattern lived in a different crate's file.
- R1.2: the no-early-return invariant is now a mutation-proven test.
- R1.3: release clears the pending delay - Enter skips the beat, and a
  cross-handler overlay's Continue can never be a dead button.
