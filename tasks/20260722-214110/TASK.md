# Ledger ch4 diverging endings: burn avoids the Auditor, distinct terminal outcomes (+ ending test rig)

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: v0.8.0, content, scenario

## Story

Make chapter four's sell-vs-burn choice DIVERGE. Today both beacon handlers
(handoff_berth, burn_buoy) spawn the same `auditor` gunship and set act=2, so
the Auditor fight is mandatory on both paths and the endings differ only in
flavor text. Owner decision (2026-07-22): one ending AVOIDS the Auditor fight
entirely (burning the box = nothing left to collect) and the two endings reach
DISTINCT terminal outcomes. Also fix the ch4 pacing: the Auditor spawns with no
engage_delay telegraph and the choice->fight transition is instant. RON-only,
shipped vocabulary. Co-delivers the pinning test rig (the test IS the contract).

Umbrella: 20260722-212808. File: `webmods/the-ledger/ledger_ch4.content.ron`;
new test `crates/nova_assets/tests/ledger_ch4_ending.rs`.

## Steps

- [x] BURN path (burn_buoy): remove the `auditor` SpawnScenarioObject; drive its
      own terminal beat instead (set act=3, land a distinct Outcome - a genuine
      "you slipped the belt" ending, optionally after a short beat_gate breather),
      no Auditor, no fight.
- [x] SELL path (handoff_berth): keep the Auditor fight, but add the missing
      engage_delay telegraph on the `auditor` controller and a warning comms
      line + choice->fight breather (announce -> arrive -> fight).
- [x] Make the two endings DISTINCT terminal outcomes (not just different
      message text on the same Victory): decide the burn ending's
      ScenarioOutcomeKind/framing vs the sell ending; owner playtest question if
      the tone split (bittersweet vs clean win) needs a call.
- [x] Remove/repurpose the now-dead choice==2 Auditor-death handler; ensure the
      burn path advances act to 3 before any death window so Defeat cannot
      spuriously trip (outcome-is-last-write-wins-close-the-act: every
      outcome-declaring handler sets its own terminal act).
- [x] Condition obj_auditor text/ObjectiveComplete per path (burn path never
      shows "break the Auditor").
- [x] New rig `ledger_ch4_ending.rs` (mirrors ledger_ch2_encounter style):
      loads the shipped ch4 RON; asserts HANDOFF sets choice=1/act=2 AND spawns
      auditor; BURN sets choice=2, does NOT spawn auditor, reaches a terminal
      Outcome without a fight; the two paths land distinct outcome kinds/messages;
      Defeat only reachable on the sell path (act<3); no NextScenario off the
      terminal chapter except retry.
- [x] `content lint --target the-ledger` clean; probe both branches against the
      real loader.

## Definition of Done

- The BURN ending reaches a terminal Outcome with NO Auditor spawn; the SELL
  ending keeps the (now telegraphed) Auditor fight; the two endings are distinct
  terminal outcomes. (test: `cargo test -p nova_assets --test ledger_ch4_ending`.)
- ch4 pacing fixed: Auditor telegraphs (engage_delay + warning line), choice->
  fight breather. (cmd: `content lint --target the-ledger` clean.)
- Owner replays both endings at Finish and confirms the divergence lands.
  (manual.)

## Notes

The ending rework and its rig are one architectural unit - ship them together.
Auditor is `cargob` gunship; controller currently `AI(())` with no engage_delay.
