# Review - scenario pacing (task 20260722-092421)

- VERDICT: APPROVE
- REVIEWER: out-of-context code reviewer
- DATE: 2026-07-22
- BRANCH: content/scenario-pacing

## Summary

The change promotes the shakedown clock-gate/breather idea into a shared
`scenario/pacing.rs` (`mark_clock` / `clock_past` / `gated_once`) and applies
objective-after-conversation sequencing plus objective-swap breathing room
across broadside, broadside_gunship, lifeline, and final_tally, and refactors
shakedown and final_tally onto the shared primitives. It removes shakedown's
"stand by" holding objective per the owner questionnaire.

The design is correct. I checked every deferred objective for soft-lock and
stale-post edge cases, verified the shakedown refactor is behaviour-preserving,
and empirically settled the one real risk (OnStart reading `scenario_elapsed`).
Tests pass (20/20 in `scenario::`), `content gen` produces no RON drift
(generated, not hand-edited), and `content lint` is clean (0 errors; the one
WARN/2 ACKs are pre-existing in the unrelated the-ledger mod). No findings rise
to MAJOR or MINOR. Two NITs below, neither blocking.

## Verification performed

1. **The OnStart-clock risk (the crux).** This branch introduces the FIRST
   OnStart reads of `scenario_elapsed` in the mainline (broadside, lifeline,
   final_tally, broadside_gunship OnStart all call
   `mark_clock(VAR_..._GATE, BEAT_GAP)`; on master `mark_clock`/`stamp_gate`
   was only ever called from OnUpdate/OnEnter/OnDestroyed/OnTravelLock). If
   `scenario_elapsed` were undefined when the OnStart action ran, the
   `VariableSet` would hit `Err(UndefinedVariable)`, log an `error!`, and NOT
   stamp the gate (crates/nova_scenario/src/actions.rs:438-452). OnStart fires
   exactly once, so the gate would stay 0, `gated_once`'s `gt_num(gate, 0)`
   guard would never pass, and the opening objective would NEVER post - a hard
   soft-lock (empty panel forever) across the whole story mainline. The Fix
   note asserts this is "purely a test artifact"; I verified it empirically by
   driving a scenario whose OnStart does `gate = scenario_elapsed + 4` through
   the real load path (`on_load_scenario` observer + `register_clock_and_pulse`)
   and asserting the gate is stamped positive. It is: the clock tick
   (loader.rs:376 `tick_scenario_clock`, chained before `fire_on_update`,
   gated live+unpaused) seeds `scenario_elapsed` before the fired OnStart event
   is drained. The claim holds; not a production risk. (Temp test removed.)

2. **`gated_once` frame-1 guard.** `gate > 0` (pacing.rs:80) plus
   `clock_past` correctly prevents a frame-0 fire: an unread var reads 0 (via
   `evaluate` returning the literal only for set vars; here the OnStart stamp
   makes it `elapsed + BEAT_GAP >= 4 > 0`). `done_flag` latches. Correct.

3. **Every deferred objective (7) checked for soft-lock / stale-post:**
   - broadside contact + defend: contact `gated_once` guarded `act==0`; if the
     player reaches the hauler inside the beat, act flips to 1, contact never
     posts (guard false), OnEnter's `complete(OBJ_CONTACT)` is a no-op, DEFEND
     posts off its own gate. No stale objective, no lock. Correct
     (broadside.rs:361-402).
   - broadside_gunship screen + break: guarded `act==1`; a gunship kill <4s
     after the taunt flips act to 2, the two objectives never post, the kill
     handler's `complete()` calls are no-ops, Victory fires. No stale objective
     under the overlay (broadside.rs:597-608, 616-641). Correct.
   - lifeline screen: guarded `act==1`; the defense is minutes long, no early
     terminal path inside the 4s. Correct (lifeline.rs:453-461).
   - final_tally survey: guarded `act==1`; no terminal path in the opening
     beat. Correct (final_tally.rs:393-401).
   - final_tally picket: guarded `Or(picket A alive, picket B alive)`. If BOTH
     pickets die inside the survey->picket 4s gap, the guard is false forever
     and the objective never posts - by design: the pickets-down beat
     (final_tally.rs:504-522) fires independently, `complete(OBJ_PICKET)` is a
     no-op, and it stamps the cast-off clock, so the scenario drives on. The
     gate is only stamped on the pickets-LIVE survey path (line 448); the
     already-drift survey variant (469-488) never stamps it, so the picket
     objective can never post pointing at dead ships. Correct.
   - final_tally break: guarded `act==1`; a flagship kill <4s after cast-off
     flips act to 4 (epilogue), the break objective never posts, the kill
     handler's `complete(OBJ_BREAK)` is a no-op. No stale objective under the
     epilogue/Victory. Correct (final_tally.rs:553-576).
   - shakedown scavenger (OBJ_B12): guarded `beat==12`; a fast kill sets
     beat 13, the objective never posts, its `complete` is a no-op under the
     Victory chain. Correct (shakedown.rs:945-950, 957-979).

4. **Shakedown refactor behaviour-preservation.** Old: `beat_gate = elapsed`,
   breather gated `elapsed > beat_gate + delay`. New: `stamp_gate` =
   `mark_clock(VAR_GATE, BREATHER_DELAY)` = `beat_gate = elapsed + delay`,
   breather gated `clock_past(VAR_GATE)` = `elapsed > beat_gate`. Algebraically
   identical (`elapsed > elapsed_stamp + delay` both ways). Corroborated by the
   `content gen` no-diff and the passing walk tests.

5. **Invariant tests pin the owner's asks.**
   `no_mainline_handler_posts_an_objective_alongside_a_conversation` (no handler
   has both StoryMessage and Objective) and
   `no_mainline_scenario_posts_an_objective_at_onstart` (empty opening panel +
   a deferred OnUpdate objective exists) exhaustively cover ask (1) and the
   empty-panel decision. `opening_objectives_are_deferred_past_frame_one` pins
   ask (2)'s opening leg structurally (clock gate or open_step latch). The
   shakedown walk tests assert the scavenger objective is ABSENT right after
   the warning and posts only after the clock passes - end-to-end timing.
   The `set_clock(100.0)` jumps do not mask a bug: they follow an assertion
   that the objective is absent at the realistic pre-deadline clock value, then
   jump past the deadline and assert it posts. Sound.

6. **Docs and stale references.** The authoring guide now describes the real
   shared mechanism (`mark_clock`/`gated_once`/`clock_past`, the two
   invariants) and drops the "stand by" holding line. `grep` outside `tasks/`
   finds no stale holding-objective or old-gate-wording references. CHANGELOG
   entry present.

7. **Scope: shakedown beacon-to-beacon nav swaps left instant.** Defensible:
   those swaps complete + post the next objective with NO conversation, so the
   enforced invariant ("no objective shares a frame with a conversation") is
   satisfied; the between-beat breather comms still follows. Continuous
   waypoint flight would desync the target marker from the objective text if
   the objective were held back from the beacon spawn. Reasonable reading of
   the owner ask.

## Findings

### NIT-1: `mark(ID_HAULER, "CERES QUEEN")` deferred with OBJ_CONTACT

broadside.rs:366-368 - the hauler's gold marker is attached inside the contact
`gated_once` alongside the objective, so during the 4s opening beat the hauler
has no marker even though the distress line names it. This is consistent (the
marker rides the objective, and if the player springs the ambush inside the
beat the contact path is skipped entirely so the marker correctly never
appears), so it is intentional, not a bug. Noting only because a reader might
expect the named ship to be marked with its mention. No change required.

### NIT-2: `pacing.rs` doc references the pre-refactor helper names

pacing.rs:20 mentions "shakedown's `stamp_gate`/`past_gate`" as the historical
duplicates; `past_gate` no longer exists anywhere in the tree (it became
`clock_past`). The sentence is a history note, not a live reference, so it is
harmless, but "past_gate" could momentarily confuse a `grep`-ing reader. If
touched, reword to "shakedown's clock-gate/breather and final_tally's
`mark_clock`/`clock_past`". Optional.

## Checks run

- `cargo test -p nova_assets --lib scenario::` -> 20 passed, 0 failed.
- `cargo run -p nova_assets --bin content -- gen` -> no RON diff (generated).
- `cargo run -p nova_assets --bin content -- lint` -> 0 errors (pre-existing
  the-ledger WARN/ACKs only).
- Empirical loader test (temp, removed): OnStart CAN read `scenario_elapsed`
  through the real load path -> the OnStart soft-lock risk is not real.
