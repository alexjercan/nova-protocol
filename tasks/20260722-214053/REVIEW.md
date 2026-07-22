# Review - Ledger campaign-wide pace-map + weak-spot brief (20260722-214053)

Out-of-context adversarial review of the diff on `docs/ledger-pacemap`. The
deliverable is `tasks/20260722-214053/NOTES.md` (a diagnostic brief); no code /
RON changed. I verified every load-bearing factual claim against the shipped RON
in `webmods/the-ledger/` with my own greps and reads. The brief is highly
accurate; the findings below are the only discrepancies I found, all in the
low/medium band, none of which would mislead a downstream authoring task.

## Round 1

### Finding 1 (medium) - ch4 handler-header miscount: "3 OnEnter" should be "2 OnEnter"
- NOTES.md:103 - "ch4 - The Buyer (1856 lines; OnStart + 3 OnEnter + 3 OnDestroyed)"
- Actual: `ledger_ch4.content.ron` has exactly **2** `OnEnter` handlers
  (line 392 `handoff_berth`, line 1082 `burn_buoy`) and 3 `OnDestroyed`
  (1774, 1803, 1833). Claimed 3 OnEnter, actual 2.
- Impact bounded: the body of the ch4 section (NOTES.md:108-116) correctly
  describes exactly the two beacon OnEnter handlers and the three OnDestroyed
  handlers with correct line numbers, so a downstream author reading the detail
  is not misled. Only the parenthetical structural header is wrong. Medium
  because it is a countable-primitive error, not cosmetic.

### Finding 2 (medium) - ch1 handler-header miscount: "5 OnEnter" should be "4 OnEnter"
- NOTES.md:39 - "ch1 - Dead Weight (1497 lines; OnStart + OnUpdate + 5 OnEnter + 1 OnDestroyed)"
- Actual: `ledger_ch1.content.ron` has **4** `OnEnter` handlers (1318, 1347,
  1376, 1439), one `OnUpdate` (1407), one `OnStart` (22), one `OnDestroyed`
  (1474). Claimed 5 OnEnter, actual 4.
- Impact bounded: the body (NOTES.md:47-49) references the three pickup OnEnter
  handlers (~1318/1347/1376) and the final Victory OnEnter (~1439) - that is 4,
  consistent with reality; only the header overcounts by one.

### Finding 3 (low) - ch1 OnStart objective count off by one (3 vs 4) and internal inconsistency
- NOTES.md:35 headline - "1 message + 3 objectives + 8 spawns at OnStart";
  NOTES.md:41 - "Objective=5(3 distinct + 2 updates)"; NOTES.md:47 - "THREE
  objectives back-to-back (`obj_ch1_recap`:38, `obj_crate_1`:443,
  `obj_crate_2`:451, `obj_crate_3`:459)".
- Actual: `OnStart` (lines 22-1317) posts **4** `Objective` calls: `obj_ch1_recap`
  (37/38), `obj_crate_1` (442/443), `obj_crate_2` (450/451), `obj_crate_3`
  (458/459). The prose says "THREE" but then enumerates four ids (recap + 3
  crates). 1 StoryMessage (33) and all 8 SpawnScenarioObject (43-468) in OnStart
  are correct.
- Low: the substantive claim (frame-0 objective+spawn dump) holds and is if
  anything understated; this is an internal "3 vs 4" wording slip, not a wrong
  structural conclusion.

### Finding 4 (low) - headline #3 overreaches: ch3 spawns are also telegraphed
- NOTES.md:26-27 (headline #3) - "ch2/ch2b ... Their fights are the only
  well-telegraphed spawns in the mod (`engage_delay: 8.0`)."
- Actual: `ledger_ch3.content.ron` also telegraphs its ambush with
  `engage_delay: Some(8.0)` (lines 490, 824). So ch3 has well-telegraphed spawns
  too; "the only" in the headline is false.
- Low: the ch3 per-chapter section itself (NOTES.md:89-90) correctly states the
  Magpie ambush uses `engage_delay: 8.0`, so the detail contradicts the headline
  and a downstream author reading the ch3 block is correct. Recommend the
  headline be scoped to "the only clock-*paced* (scenario_elapsed) chapters" -
  which is the true distinction (ch2/ch2b have the only `scenario_elapsed` gates).

### Finding 5 (low) - ch4 `handoff_berth` mislabeled "asteroid trigger"; it is a Beacon
- NOTES.md:109 - "`handoff_berth` (asteroid trigger, line 359)".
- Actual: line 359 `id: "handoff_berth"` is `kind: Beacon(( label: "HANDOFF",
  radius: 3.0, area_radius: Some(25.0) ... ))`, not an asteroid. `burn_buoy`
  (line 374) is likewise a `Beacon`. The line number (359) is correct.
- Low: the OnEnter trigger mechanic and the branch wiring the brief describes are
  correct; only the object-kind noun is wrong ("asteroid" vs "beacon").

## Claims verified correct (spot-check log)

- `dwell` = 0 refs in all five files. TRUE (grep -c = 0 each).
- `scenario_elapsed`: ch1=0, ch2=1, ch2b=1, ch3=0, ch4=0. TRUE (exact).
- `engage_delay`: ch4=0. TRUE (grep returns NONE). ch2=2, ch2b=2, ch3=2, all
  `Some(8.0)`. TRUE.
- Per-chapter primitive counts (Objective / StoryMessage / SpawnScenarioObject /
  Outcome / NextScenario): all five chapters match NOTES exactly
  (ch1 5/5/8/2/3, ch2 3/4/12/3/3, ch2b 3/4/12/3/3, ch3 2/4/7/2/2, ch4 3/3/6/3/1).
- Line counts (1497 / 2155 / 2065 / 1230 / 1856). TRUE.
- ch4: both `auditor` spawns at lines 422 and 1112, both `controller: AI(())`,
  both `cargob`-family (`cargob_core_controller` prototype, name `IV 'Auditor'`),
  neither has `engage_delay` (none anywhere in ch4). TRUE.
- ch4: `handoff_berth` OnEnter (392, act==1) sets choice=1, act=2, posts
  `obj_auditor`, spawns auditor; `burn_buoy` OnEnter (1082, act==1) sets
  choice=2, act=2, spawns identical auditor. TRUE.
- ch4 endings: both `OnDestroyed(auditor)` (act==2), disjoint `choice` guard,
  Victory "SOLD" at Outcome line 1796, Victory "BURNED" at 1825; Defeat handler
  `OnDestroyed(player)` `act < 3` at 1833. TRUE (line numbers exact).
- ch3: `ScatterObjects count: 26` (line 404), decorative (no beat references the
  rocks). TRUE. Beacons nav_1/nav_2/nav_3/vesh_yard (NAV-1/2/3/YARD), strict
  `gate` 1..4 guards, single optional Magpie ambush sprung on NAV-2 OnEnter.
  TRUE. NextScenario -> `ledger_ch4_the_buyer`. TRUE.
- ch1 body line refs: Okono StoryMessage at 33/34, `obj_ch1_recap` at 37/38,
  `obj_crate_1/2/3` at 443/451/459, Victory -> `ledger_ch2_claim_jumpers`. TRUE.
- ch2 win: `kills > 1` OnUpdate -> Victory -> `ledger_ch2b_the_heavies`,
  `linger: true`; one `scenario_elapsed`-gated `teach_sent` one-shot. TRUE.
- Handoff chain ch2->ch2b->ch3->ch4 scenario_ids. TRUE.
- Bundle version 1.5.0 (`the-ledger.bundle.ron:21`). TRUE.
- `open_step` / `beat_gate` / `mark_clock` / `clock_past` in the cross-cutting
  section are correctly presented as beat-sheet conventions to APPLY, not
  claimed to already exist in the ledger (grep confirms none exist there). OK.

## Definition of Done

- Per-chapter pacing table + weak-spot list + target rhythm grounded in real
  handler/variable names: MET (five per-chapter blocks with real ids -
  `obj_auditor`, `handoff_berth`, `burn_buoy`, `gate`, `nav_*`, `kills`,
  `teach_sent`, `act`, `choice`, cited line numbers overwhelmingly correct).
- Owner playtest questions listed, not silently decided: MET (five questions,
  NOTES.md:149-167, each tied to a sibling task).

The brief is load-bearing for four downstream authoring tasks and its central
factual claims - the dwell/scenario_elapsed/engage_delay ledger, the ch4
converging-Victory-with-untelegraphed-Auditor structure, the ch3 linear-corridor
diagnosis, and every per-chapter primitive count - are all verified accurate.
The five discrepancies are header miscounts and noun/scoping slips whose
corrected detail already appears elsewhere in the same document, so none of them
would cause a sibling task to author against a wrong structure. I recommend the
author fix findings 1-5 (all one-line edits) as low-cost accuracy hygiene, but
they do not block: no finding rises to "wrong claim that would mislead a
downstream task."

## Verdict

- VERDICT: APPROVE