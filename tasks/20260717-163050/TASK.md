# Outcome transition pacing: timed auto-advance behind the overlay + lint for the linger:false swallow trap

- STATUS: CLOSED
- PRIORITY: 37
- TAGS: spike,v0.7.0,scenario,menu,lint

Goal (USER DIRECTIVE 2026-07-17: "add to the pacing by doing linger
false in some cases maybe with a time delay"): a middle gear between the
hard cut and the modal overlay. An authorable delay on the non-lingering
NextScenario switch - queue the chain, let the world keep playing (or
show the outcome banner non-blocking), advance automatically after N
seconds; plus an optional timed auto-advance on the modal overlay; plus
a content_lint WARN for the Outcome + linger:false same-handler swallow
trap (NovaEventWorld::clear's documented footgun). Mind pause semantics
(delays tick on the scenario clock's gate). Spike:
tasks/20260717-155740/SPIKE.md.

Verified at plan time: the non-lingering switch executes in
NovaEventWorld::state_to_world_system (world.rs:119, consume-then-load);
the overlay's Continue rides release_lingering_next (world.rs:232, menu
lib.rs:774); OutcomeActionConfig { outcome, message } and
NextScenarioActionConfig { scenario_id, linger } both have Rust literal
constructors in the broadside builder + tests (sweep needed);
the outcome overlay pauses VIRTUAL time, so the overlay auto-advance
must tick Time<Real> while the delayed non-lingering switch ticks
virtual (a player pausing holds an un-overlaid cut - correct).

## Steps

- [x] Schema: NextScenarioActionConfig.delay: Option<f32> (strict RON
  `delay: Some(4.0)`; only meaningful with linger: false; non-positive =
  no delay) and OutcomeActionConfig.auto_advance_secs: Option<f64>
  (banner shows N real seconds, then advances the queued LINGERING chain
  as if Continue were pressed; absent = wait for the player). Serde
  defaults; constructor sweep (builder + tests + ::new).
- [x] Engine: NovaEventWorld.next_scenario_delay: Option<Timer> armed by
  the action apply (non-lingering + positive delay), reset by clear();
  state_to_world ticks it on the world's (virtual) clock and executes
  the switch only at expiry - the world keeps playing through the beat.
- [x] Menu: an auto-advance system ticking Time<Real> while the outcome
  overlay is up with auto_advance_secs and a lingering chain queued;
  at expiry it calls release_lingering_next - the same path as the
  button; Local timer resets when CurrentOutcome changes.
- [x] Lint: WARN when one handler fires Outcome AND a non-lingering
  NextScenario (delayed or not): undelayed swallows the overlay
  (NovaEventWorld::clear's documented footgun), delayed freezes the
  delay under the overlay's pause - both are authoring traps.
- [x] Tests: action arms the timer; state_to_world DEFERS the switch
  (manual clock: LoadScenario not fired before expiry, fired after -
  the fail-first vs today's instant cut); clear() drops the pending
  delay; menu auto-advance releases the linger after N real seconds
  (mirror the retry-button rig) and does nothing when absent; lint warn
  test; strict-RON parse tests for both fields.
- [x] Docs: scenario-system.md (NextScenario delay + Outcome
  auto-advance, the three transition gears - hard cut / delayed cut /
  modal hold, and when to use which); CHANGELOG (Scenarios &
  Objectives). NOTES.md.
- [x] Verify: cargo test -p nova_scenario --features serde; -p nova_menu;
  content_lint; workspace --all-targets; fmt last. Full suite on CI.

## Close-out record

All seven steps landed; the clock choices and the queue-starvation catch
are in NOTES.md. Verification: nova_scenario --features serde 110 green
(delayed-cut fail-first, clear, lint traps, RON parse), nova_menu 62/62
(timed banner + delivery guard), content_lint clean, gen_content no-op,
workspace --all-targets green, fmt last. Full suite on CI per standing
instruction.
