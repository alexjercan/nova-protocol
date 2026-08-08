# Broadside: terminal act on player death (Lifeline review R1.1 class)

- STATUS: CLOSED
- PRIORITY: 47
- TAGS: v0.8.0, content, scenario, bug

## Goal

Lifeline's review R1.1 (tasks/20260721-160957/REVIEW.md) found the
player-death Defeat handlers leave the act variable live, so a
last-write-wins CurrentOutcome overwrite (mutual-destruction trade: the
player's blast kills the last objective target just after the player dies)
can replace the Defeat with a Victory over the queued retry. Lifeline fixed
its handler (terminal act 3 + a pinning test); broadside.rs's two
player-death handlers (both parts) share the shape - narrower window (their
victories gate on kill events, not an every-pulse clock), but the same
class. Apply the same terminal-act fix + harness pin to both broadside
parts.

## Steps

- [x] In crates/nova_assets/src/scenario/broadside.rs, set a terminal act
      in both player-death Defeat handlers (mirror lifeline.rs review R1.1
      comment); `content gen`.
- [x] Extend broadside_assault.rs: after a player death, the checkpoint /
      gunship kills must not overwrite the Defeat (mirror
      `player_death_retries_the_lane`'s trade pin).
- [x] `content lint` + parity + suite green.

## Definition of Done

- Both broadside player-death handlers set a terminal act
  (cmd: `grep -n -A4 "destroyed(ID_PLAYER)" crates/nova_assets/src/scenario/broadside.rs`).
- The trade pin fails without the fix (test: name recorded when written;
  sabotage-prove per the A/B rule).

## Fix (2026-07-22)

Both player-death handlers (broadside part one line ~526, gunship part two line
~685) fired `Outcome(Defeat)` + `NextScenario` but never advanced `VAR_ACT`,
while the Victory handlers set `act = 2`. Since `CurrentOutcome` is
last-write-wins, a mutual-destruction trade (the player's blast breaking the
last corvette / the gunship on the same beat the player dies) left the win gate
(`eq_num(act, 1)`) live, so the Victory overwrote the Defeat over the queued
retry - a win from a fight you died in.

Fix: `set(VAR_ACT, num(3.0))` FIRST in both death handlers (mirroring lifeline's
R1.1 terminal act 3), which closes every `act == 1` win gate. Chose 3 (not 2)
so it is distinct from the earned-Victory act, matching lifeline.

Reproduction (fail-first, A/B): new `broadside_assault.rs`
`a_trade_after_the_players_death_cannot_overwrite_the_defeat` - for BOTH parts,
seed the live act, kill the player (assert Defeat + terminal act 3), then kill
the win target (corvettes / gunship) and assert the Defeat HOLDS. Failed before
the fix (act stayed 1; the trade flipped Defeat -> Victory), passes after.
Existing `player_death_after_the_win_declares_nothing` (act 2 seed) still green:
the death handler's `act < 2` gate means the terminal set never fires under the
earned Victory. Content regenerated (only the two broadside RON files changed,
each gaining the `act = 3` set); lint clean; broadside_assault 15/15.
