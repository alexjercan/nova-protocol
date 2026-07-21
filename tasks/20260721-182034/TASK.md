# Broadside: terminal act on player death (Lifeline review R1.1 class)

- STATUS: OPEN
- PRIORITY: 47
- TAGS: v0.8.0,content,scenario,bug

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

- [ ] In crates/nova_assets/src/scenario/broadside.rs, set a terminal act
      in both player-death Defeat handlers (mirror lifeline.rs review R1.1
      comment); `content gen`.
- [ ] Extend broadside_assault.rs: after a player death, the checkpoint /
      gunship kills must not overwrite the Defeat (mirror
      `player_death_retries_the_lane`'s trade pin).
- [ ] `content lint` + parity + suite green.

## Definition of Done

- Both broadside player-death handlers set a terminal act
  (cmd: `grep -n -A4 "destroyed(ID_PLAYER)" crates/nova_assets/src/scenario/broadside.rs`).
- The trade pin fails without the fix (test: name recorded when written;
  sabotage-prove per the A/B rule).
