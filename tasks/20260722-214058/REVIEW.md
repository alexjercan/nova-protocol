# Review - Ledger beat-sheet pacing pass: ch1/ch2/ch2b (20260722-214058)

Out-of-context adversarial review. Branch `content/ledger-pacing-ch1-ch2`
vs `master`. Verified every claim with my own reads/greps and by running the
two required checks.

## Round 1

### Verification summary (all green)

- `content lint --target the-ledger`: `0 error(s), 1 warning(s), 1 finding(s),
  5 scenario(s) balance-audited, 2 acked`. The single WARN cites
  `ledger_ch4.content.ron ledger_ch4_the_buyer` (the `auditor` multi-spawn),
  NOT ch1/ch2/ch2b, and both close-spawn findings are pre-existing ACKs from
  task 20260717-143806. Confirmed the warn is not in the touched files.
- `cargo test -p nova_assets --test ledger_ch2_encounter`: 12 passed, 0 failed.

### CONSTRAINT: no geometry change in ch2/ch2b - HOLDS

Grepped both diffs for every geometry-bearing token
(`position|rotation|engage_delay:|radius|health|SpawnScenarioObject|
ScatterObjects|count|seed|patrol|allegiance|controller`). Every match on an
added (`+`) line is a COMMENT mentioning `engage_delay` in prose. No
`position`/`rotation`/`engage_delay:` VALUE line, no spawn block, no count or
loadout changed in either file. The pacing layer is strictly additive (new
vars, new OnUpdate handlers, retasked holding objective, deferred Outcome).
The `engage_delay: 8.0` telegraphs are untouched. Constraint satisfied.

### Beat sheet compliance - HOLDS

- No objective posts at OnStart during a conversation: in all three files the
  OnStart objectives were removed and replaced by a single HOLDING line
  (`obj_ch*_recap` retasked to "stand by ..."). Real objectives lazy-post on
  the hand-off handler (ch1 `open_step==4 && quota_posted==0`; ch2/ch2b
  `open_step==2 && obj_posted==0`). Correct.
- One StoryMessage per handler: verified by reading every new handler. No
  handler carries two StoryMessages.
- No StoryMessage in the same handler as an Outcome: the ch2/ch2b win handler
  (`act==1 && kills>1`) fires the comms line + stamps `win_gate` only; the
  Outcome + NextScenario moved to a separate deferred handler
  (`act==2 && win_said==0 && elapsed>win_gate`). This is exactly the lint arm's
  requirement, and lint is clean.
- Opening cascade uses open_step + scenario_elapsed idiom: ch1 T=2/11/20/29,
  ch2/ch2b T=2/11. Breathers use gate-stamp + clock_past (`ack*_gate`,
  `ping_gate`, `beat_gate`, `win_gate`). Correct.
- dwell values: 7.0, 7.0, 8.0 (ch1), 7.0, 6.0 (ch2), 7.0, 6.0 (ch2b). All in
  [3,30]. Lint reports no out-of-range dwell.

### Reachability of clock gates - HOLDS (no soft-lock)

- ch1 opening cascade: open_step 0->1->2->3->4 on strictly ascending
  thresholds (2<11<20<29), each handler gated `open_step==N` and sets N+1, so
  exactly one fires per step. `scenario_elapsed` is monotonic and all gates are
  seeded 0 in OnStart, so no undefined read fails a beat closed. The hand-off
  (`open_step==4 && quota_posted==0`) is reachable once step 4 lands and
  one-shots via `quota_posted`. The first objective ALWAYS eventually posts; a
  deferred first objective cannot strand the player.
- ch1 pickup acks: each gated `ackN_gate>0` (stamped only by that crate's
  pickup) + `ackN_said==0` (one-shot) + `elapsed>ackN_gate`. Order-free and
  independent; an un-picked crate never speaks (gate stays 0). Correct.
- ch1 reveal chain: `crates>2 && act==1` (act flips to 2, one-shot) arms
  `ping_gate=elapsed+4`; announce fires once past `ping_gate` (`ping_said`
  one-shot) and re-stamps `beat_gate=elapsed+3.5`; `obj_blackbox` posts once
  past `beat_gate` (`setup_last<1` guard). Ordering across ack3 (elapsed+2.5)
  vs reveal (elapsed+4) is correct - the "manifest done" ack lands before the
  hook. No re-fire, no strand.
- ch2/ch2b: win-line handler self-disqualifies (sets `act=2`, filter was
  `act==1`); overlay one-shots via `win_said`; `win_gate` is stamped the same
  frame `act` flips, and `scenario_elapsed` advances, so the overlay always
  eventually fires. Covered green by the two pumped walk tests.

### Test change (`ledger_ch2_encounter.rs`) - DELIBERATE, not a masked regression

The two Victory walks now: seed `scenario_elapsed=30` before the killing blow,
assert the overlay is NOT up on the kill frame (proving the comms-line breather
exists), then `pump_clock(100)` and assert Victory + the correct NextScenario
(`ledger_ch2b_the_heavies` / `ledger_ch3_quiet_channel`) exactly as before.
`armed_app` additionally seeds `win_said=0`, mirroring OnStart. This is the
honest consequence of moving the StoryMessage off the Outcome frame (a lint
arm forces the split, which forces the Outcome to defer, which needs a clock
pump in a rig that sets no time). No geometry assertion was weakened - the
range/bearing/cover/mule-axis/rock-overlap pins and `on_start_seeds_the_act_
machine` are untouched and green.

### LOW - rig inertness of `deaths_after_the_win_declare_nothing` relies on a different mechanism than the live game

`deaths_after_the_win_declare_nothing` seeds `act=2, kills=2` but neither
`win_said` nor `scenario_elapsed`. It passes because the deferred overlay
handler's `win_said==0` filter reads an UNDEFINED variable and fails closed
(undefined-Name-fails-safe, per the wiki). In the real loader OnStart seeds
`win_said=0` and `win_gate=0`, so in-game that handler is held inert instead by
`win_said` flipping to 1 on the (legitimate) first fire. The test therefore
proves the post-win-death no-op via a different code path than production. It
is not a bug (the real win path is exercised by the two pumped walks, and a
post-win death genuinely cannot flip an earned Victory), but a reader could
mistake the inertness for act-gating. Suggested change (optional): seed
`win_said=1` in that test to model "the overlay already fired" and pin the
act-gating explicitly, or add a one-line comment noting the inertness here
comes from the undefined-var guard, not from `act`/`win_said` state. No action
required for approval.

## Verdict

APPROVE
