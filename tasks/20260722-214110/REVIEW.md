# Review - Ledger ch4 diverging endings (+ ending test rig)

Task 20260722-214110. Adversarial out-of-context review by a non-implementer.
Branch `content/ledger-ch4-endings` vs `master`.

## Round 1

Verified by reading the full diff, the shipped RON, `actions.rs`, and by
running lint + both test rigs under `nix develop`.

### Latch integrity (the central risk) - PASS

- **BURN path is race-free.** `burn_buoy` OnEnter (ledger_ch4.content.ron:1116)
  sets `choice=2` and `act=3` in the SAME actions block, synchronously, before
  any death window. The Defeat handler (ledger_ch4.content.ron:1229) gates on
  `act < 3`, so once the buoy latches act=3 it can never fire on the burn path.
  Confirmed no auditor spawn on this handler (the ~650-line `cargob` block was
  deleted). Pinned by `burn_path_has_no_death_window` and
  `burn_branch_never_spawns_the_auditor`.
- **BURN terminal is a deferred one-shot, StoryMessage-then-Outcome.** The buoy
  posts ONE Vesh StoryMessage (no Outcome beside it); the terminal Victory
  lands a beat later via the OnUpdate overlay (ledger_ch4.content.ron:1159)
  gated `choice==2 && burn_said==0 && burn_gate>0 && scenario_elapsed>burn_gate`,
  which sets `burn_said=1` and declares Outcome with NO StoryMessage alongside.
  Matches the ch2 deferred-Victory idiom. Pinned by `burn_branch_closes_without_a_fight`.
- **SELL path telegraphed.** `handoff_berth` OnEnter (ledger_ch4.content.ron:449)
  now carries `controller: AI((engage_delay: Some(8.0)))` and a strengthened
  warning comms line naming the incoming burn and a ~ten-second beat. The
  `OnDestroyed(auditor)` sell ending (ledger_ch4.content.ron:1196) fires on
  `act==2 && choice==1`, sets act=3, and is terminal. Pinned by
  `sell_branch_spawns_a_telegraphed_auditor` / `sell_branch_wins_by_breaking_the_auditor`.
- **Dead choice==2 auditor-death handler removed.** Confirmed via diff: the old
  `choice==2` OnDestroyed(auditor) handler is gone; exactly ONE auditor-death
  handler remains (choice==1). `grep 'id: "auditor"'` finds a single spawn site
  (line 446).
- **No overwritable settled outcome.** CurrentOutcome is genuinely
  last-write-wins (`actions.rs:540` does an unconditional `current.0 = Some(..)`),
  so integrity rests entirely on RON gating. All three Outcome-declaring
  handlers are mutually exclusive with a closed act: BURN overlay needs
  `choice==2 && burn_said==0`; SELL death needs `act==2 && choice==1`; Defeat
  needs `act<3`. None can re-fire over a settled ending. Pinned by
  `a_settled_outcome_is_not_overwritten`.
- **Distinct + terminal.** Both endings are Victory (the only win kind besides
  Defeat - confirmed `ScenarioOutcomeKind` at actions.rs:471 has exactly
  `Victory`/`Defeat`) but carry distinct messages ("SOLD..." vs "BURNED...") and
  the burn path is structurally fight-free. Only the Defeat retry queues a
  `NextScenario` (self-requeue); neither Victory chains one. Confirmed by
  greps: the sole `NextScenario` is at line 1245 inside the Defeat handler.

### Lint-ack prune - PASS (correct)

The two `close-spawn` acks collapsed to one. Verified the removed ack is
genuinely orphaned: only ONE `auditor` spawn remains (sell branch), and lint
reports the single acked finding as `OnEnter(handoff_berth)` - the surviving
sell-branch spawn, matching a real live finding. `grep -rn 'auditor'` across
`webmods/` finds references only in ledger_ch4; no other mod/scenario relied on
the pruned ack. Not a silenced-live-finding situation.

### Checks (run under nix develop) - all green

- `content lint --target the-ledger`: `0 error(s), 0 warning(s), 0 finding(s),
  5 scenario(s) balance-audited, 1 acked`. The single ACK is the intended
  sell-branch close-spawn (OnEnter(handoff_berth)).
- `cargo test -p nova_assets --test ledger_ch4_ending`: 10 passed.
- `cargo test -p nova_assets --test ledger_ch2_encounter`: 12 passed (no
  cross-break).

### Test quality (production-faithful) - PASS

The rig loads the REAL shipped `ledger_ch4.content.ron` via `include_str!`,
registers the real non-start handlers the way the loader pairs them, and drives
the real act machine with the same event infos the engine emits. It pumps the
clock (`pump_clock` seeding `scenario_elapsed`) for the deferred burn overlay.
Coverage against the requested contract:

- (a) HANDOFF spawns auditor with engage_delay >= 8.0 - reads the spawn config
  directly, not just a message. PASS.
- (b) BURN does NOT spawn auditor + advances act=3 + terminal Victory with no
  fight - asserts the STRUCTURAL no-spawn fact (`auditor_spawn_sites == 1` over
  the whole scenario), the synchronous act=3, and the deferred Victory. This is
  the key anti-false-green assertion and it is present. PASS.
- (c) two terminal messages DIFFER - `assert_ne!(sell_msg, burn_msg)` plus
  SOLD/BURNED content checks. PASS.
- (d) Defeat reachable only pre-auditor-death on sell path, inert at act=3 -
  `defeat_is_reachable_only_on_the_sell_path` + `burn_path_has_no_death_window`
  + `a_settled_outcome_is_not_overwritten`. PASS.
- (e) no NextScenario off either Victory - both terminal walks assert
  `queued_next == None`. PASS.

No assertion would pass if the divergence regressed: reintroducing a burn-branch
auditor spawn breaks `burn_branch_never_spawns_the_auditor`; collapsing the two
endings to shared text breaks `the_two_endings_are_distinct_terminal_outcomes`;
dropping the synchronous act=3 opens the death window that
`burn_path_has_no_death_window` guards.

### LOW - polish notes (non-blocking)

- LOW: ch4 StoryMessages omit `dwell` (rely on the default). This matches the
  pre-existing ch4 style and lint is clean (dwell only warns when set outside
  3-30), so it is not a beat-sheet violation - noted only for completeness.
- LOW (informational): the implementer batched an owner playtest question about
  burn-ending tone (bittersweet SAFE-BUT-BROKE vs clean escape). This is a
  message-only swap flagged for the Finish checkpoint, not a code issue.

## Verdict

- VERDICT: APPROVE