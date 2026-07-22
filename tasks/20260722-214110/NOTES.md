# Ch4 diverging endings - implementation notes

Task 20260722-214110. Data-only (RON) + a new test rig. No engine/Rust changes.

## What changed (from the final diff)

`webmods/the-ledger/ledger_ch4.content.ron` (net -611 lines, the bulk is the
deleted second Auditor ship block):

1. **OnStart** now also seeds two burn one-shots (`burn_gate = 0`,
   `burn_said = 0`) so the deferred burn overlay reads defined values (mirrors
   how ch2 seeds `win_gate`/`win_said`).
2. **SELL branch (handoff_berth OnEnter)**: the Auditor's controller gained the
   telegraph `engage_delay: Some(8.0)` (matching ch2/ch2b/ch3). The warning
   comms line was strengthened to name the incoming military burn and give a
   "~ten seconds" beat; the objective text now says "The sale brought the
   Auditor. Break it..." This path is unchanged structurally: choice=1, act=2,
   spawn the Auditor, fight, win on its death.
3. **BURN branch (burn_buoy OnEnter)**: the Auditor `SpawnScenarioObject` (the
   full ~650-line `cargob` ship block) was DELETED. The handler now: sets
   `choice=2`, latches `act=3` SYNCHRONOUSLY (terminal, no death window),
   stamps `burn_gate = scenario_elapsed + 3`, completes `obj_ch4_recap`, and
   posts ONE Vesh comms line. No objective pointing at an Auditor is ever added
   on this path (so it never shows "break the Auditor").
4. **New deferred BURN overlay** (`OnUpdate`, gated `choice==2 && burn_said==0
   && burn_gate>0 && scenario_elapsed>burn_gate`): sets `burn_said=1`, lands the
   terminal Victory with the distinct SAFE-BUT-BROKE message. No StoryMessage
   beside the Outcome; no NextScenario.
5. **Dead-wiring removed**: the old `choice==2` Auditor-death `OnDestroyed`
   handler is gone (it could never fire once the burn path stopped spawning the
   Auditor). Only ONE Auditor-death handler remains (`choice==1`, the sell
   ending), its SOLD message rewritten to "payday, and the price was a gunship".
6. **Header comment** rewritten to describe the divergence; the Defeat
   handler's comment now notes it is sell-only reachable.

`crates/nova_assets/balance_acks.ron`: the two `close-spawn` acks (one per
branch) collapsed to ONE. The burn-branch ack was stale (that spawn no longer
exists) and was pruned; the surviving sell-branch ack's reason was updated to
record the telegraph and the pruning (task 20260722-214110). Lint confirms 1
acked finding, matching the sole surviving spawn.

`crates/nova_assets/tests/ledger_ch4_ending.rs`: new rig (10 tests), mirrors the
`ledger_ch2_encounter.rs` style.

## Ending divergence design + the ScenarioOutcomeKind decision

`ScenarioOutcomeKind` (crates/nova_scenario/src/actions.rs:472) has exactly two
variants: `Victory` and `Defeat`. There is NO neutral/bittersweet/escape kind.
So per the task's fallback clause, both wins are `Victory` and the divergence
carries in (a) DISTINCT terminal messages and (b) the STRUCTURAL fact that the
burn path never fights:

- SELL = PAYDAY AT A PRICE: "SOLD. Paid in full... the Auditor is scrap...
  Payday, and the price was a gunship." Keeps the (now telegraphed) fight.
- BURN = SAFE BUT BROKE: "BURNED. The box is slag and nobody's left to chase
  you - no Auditor, no gunship, no fight. But there's no payday either... Clear,
  and broke." NO fight, no spawn.

The two paths are pinned distinct by `the_two_endings_are_distinct_terminal_outcomes`
(asserts messages differ) and `burn_branch_never_spawns_the_auditor` (asserts
exactly one Auditor spawn site in the whole scenario). Both are terminal
(`act=3`); neither chains a `NextScenario`.

## Latch / reachability trace

- **SELL**: enter berth -> choice=1, act=2, spawn Auditor (engage_delay 8s).
  Fight. Auditor dies -> act=3, ObjectiveComplete, Victory (SOLD). While act==2
  a player death hits the Defeat handler (act<3) -> Defeat + retry the finale.
- **BURN**: enter buoy -> choice=2, act=3 IMMEDIATELY (synchronous), burn_gate
  stamped, burn line posted. No spawn, no objective toward a fight. A beat
  later (scenario_elapsed > burn_gate) the deferred overlay fires ONCE
  (burn_said latch) -> Victory (BURNED). Because act is already 3 at the buoy,
  the Defeat handler's `act<3` gate can NEVER trip after a burn - there is no
  death window. Pinned by `burn_path_has_no_death_window`.
- **No overwrite of a settled outcome**: every outcome-declaring handler sets
  its own terminal `act=3`; the sole surviving auditor-death handler needs
  act==2 to fire, the burn overlay needs burn_said==0, and the Defeat handler
  needs act<3 - all mutually exclusive with a closed act. Pinned by
  `a_settled_outcome_is_not_overwritten`.

## Lint-ack update

Before: two `close-spawn` acks for `auditor` (handoff + burn branches). After:
removing the burn spawn made the second ack stale (an ack whose finding
disappears fails CI until pruned). Pruned it; folded its intent into the one
remaining ack and added the telegraph note. Final lint: `0 error(s),
0 warning(s), 0 finding(s), 5 scenario(s) balance-audited, 1 acked`.

## Verification

- `content lint --target the-ledger`: 0 errors, 0 warnings, 1 acked (the
  intended sell-branch close-spawn).
- `cargo test -p nova_assets --test ledger_ch4_ending`: 10 passed.
- `cargo test -p nova_assets --test ledger_ch2_encounter`: 12 passed (no
  cross-break).
- `cargo fmt -p nova_assets`: clean.

## Batched owner question (for the Finish checkpoint)

**Burn ending tone (owner playtest question 1)**: implemented the DEFAULT split
from the task - the burn ending is SAFE BUT BROKE (bittersweet: clear but no
payout, debt stands, Kestrel limps home empty). The alternative tone is a CLEAN
escape ("you slipped the belt, box gone, nobody left to collect" with no
downside beat). The current message leans bittersweet; if the owner wants the
clean-escape framing, it is a one-line message swap in the deferred burn
overlay (no structural change). Flagged for replay at Finish.
