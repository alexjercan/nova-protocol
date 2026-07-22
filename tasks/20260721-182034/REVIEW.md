# Review - Broadside terminal act on player death (task 20260721-182034)

- VERDICT: APPROVE

Out-of-context review of `fix/broadside-terminal-act` (`git diff master...HEAD`).
Small, well-scoped fix mirroring lifeline's review R1.1 terminal-act fix into both
broadside parts. Correctness verified by enumeration, regression guards hold, and
the new test is sabotage-proven to fail-first on the exact assertion the author
claimed. No blocking issues.

## Summary of findings

No blocking (R) issues. Everything below is confirmation of the fix's correctness
plus minor notes.

## Correctness (focus 1) - PASS

The fix inserts `set(VAR_ACT, num(3.0))` as the FIRST action in both player-death
`OnDestroyed` handlers:
- part one: crates/nova_assets/src/scenario/broadside.rs:536
- part two: crates/nova_assets/src/scenario/broadside.rs:701

Enumerated every act-reading win/outcome gate and confirmed act 3 closes each:

Part one (broadside):
- Victory (hauler intact) broadside.rs:457-481 - gate `eq_num(act, 1.0)`; at 3 -> false. CLOSED.
- Victory (hauler lost) broadside.rs:482-506 - gate `eq_num(act, 1.0)`; at 3 -> false. CLOSED.
- Hauler soft-fail broadside.rs:510-521 - gate `lt_num(act, 2.0)`; at 3 -> false. CLOSED.

Part two (broadside_gunship):
- Victory (hauler intact) broadside.rs:626-651 - gate `eq_num(act, 1.0)`; at 3 -> false. CLOSED.
- Victory (hauler lost) broadside.rs:652-677 - gate `eq_num(act, 1.0)`; at 3 -> false. CLOSED.
- Hauler soft-fail broadside.rs:679-690 - gate `lt_num(act, 2.0)`; at 3 -> false. CLOSED.

No win/outcome gate uses `>= 1`, `> 0`, or an open upper bound that act 3 could
satisfy, so 3 is fully absorbing. Terminal value 3 is distinct from the earned
Victory's act 2 (matching lifeline), so a Defeat state is unambiguous from a
Victory state - correct choice over reusing 2.

## Regression (focus 2) - PASS

- The terminal set only fires on a LIVE-act death: the death handler itself gates
  `lt_num(act, 2.0)` (broadside.rs:528, :694). A post-win death (act 2) never
  enters the handler, so the terminal set never runs and the earned Victory is
  untouched. `player_death_after_the_win_declares_nothing` (seeds act 2) still
  green - verified in the 15/15 run.
- No other act-reader can misbehave at act 3. Beyond the win/soft-fail gates above,
  the remaining act reads are act-0/act-1 progression gates (objectives, ambush,
  first-kill lines, gated_once posts) - all `eq_num(act, 0.0)` / `eq_num(act, 1.0)`,
  false at 3. Act 3 is a dead-end state, which is exactly what a terminal Defeat
  wants: nothing else fires after the retry is queued.

## Test quality (focus 3) - PASS

New test `a_trade_after_the_players_death_cannot_overwrite_the_defeat`
(crates/nova_assets/tests/broadside_assault.rs:275-315):
- Covers BOTH parts via the `[(&str, fn)]` table (part one kills corvette_a +
  corvette_b to open the OnUpdate win; part two kills the gunship).
- Reproduces the trade faithfully: seeds the live act (1), kills the player first
  (asserts Defeat + act==3), THEN kills the win target. In part one the corvette
  flag handlers plus the pumped `app.update()` would drive the OnUpdate win on a
  live act; in part two the gunship OnDestroyed win. The post-fix act 3 gate keeps
  the Defeat from flipping.
- Assertion is the right pin: Defeat holds AND act == 3 (the intermediate check).
- SABOTAGE-PROVEN (A/B): removed BOTH terminal sets, regenerated RON, ran the test
  in isolation - it FAILED at broadside_assault.rs:299, the `act == 3` assertion,
  exactly as the author claimed ("failed on the act==3 assertion pre-fix"). The
  Defeat-holds assertion would also fail downstream once the win re-fires; the
  act==3 pin trips first, which is the tighter cause-level assertion. Restored the
  fix and re-verified 15/15 green + clean git status.

## Scope + RON provenance (focus 4) - PASS

- `git diff master...HEAD` touches exactly: broadside.rs, broadside_assault.rs, the
  two broadside RON files, and TASK.md. No stray files.
- RON is generated, not hand-edited: ran `content gen` on the current tree and git
  status stayed CLEAN - the committed RON matches the generator byte-for-byte. Each
  RON gained only the `VariableSet(act = 3.0)` action ahead of the Defeat outcome.

## Verification run

- `cargo test -p nova_assets --test broadside_assault`: 15 passed, 0 failed.
- `content lint`: 0 errors, 1 warning + 1 finding (both pre-existing, acked, in the
  unrelated `the-ledger` scenario - not broadside).
