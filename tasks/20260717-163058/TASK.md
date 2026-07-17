# Beat-sheet pass: apply the storytelling rhythm across campaign and ledger; write the convention into the dev wiki

- STATUS: CLOSED
- PRIORITY: 36
- TAGS: spike,v0.7.0,scenario,content,docs

Goal: make the rhythm actual. Apply the storytelling convention across
shakedown_run, both broadside parts and the five ledger files using the
scenario clock + the three engine tasks' mechanics (comms queue, arrival
telegraphs, transition delays): announce -> breathe -> arrive -> fight ->
confirm -> breathe -> next; one story line per beat; every fight gets a
lead-in; checkpoint lines fire before their outcome beat or ride its
auto-advance. Write the beat-sheet convention into the dev wiki
(guide-author-scenario.md). Acceptance is checkable: no handler fires
more than one StoryMessage; every balance-audit spawn group trails a
warning beat. Depends on 163033/163042/163050 landing first. Spike:
tasks/20260717-155740/SPIKE.md.

Surveyed at plan time: TWO handlers fire double story lines (ch2a/ch2b
OnStart - my own rework's teach lines); SIX story lines fire in the same
handler as their Outcome (ch1, ch2a, ch2b, ch3, ch4 x2) and are DEAD
today - the overlay pauses the comms queue, then the chained teardown
drops them. All other shipped content is one-line-per-beat already.
Ch4's Auditor keeps its hot entrance (acked drama - NO grace there).

## Steps

- [x] Mechanize the convention in content_lint (warn-grade): (a) more
  than one StoryMessage in one handler ("space beats with the scenario
  clock; the queue is the safety net, not the style"); (b) StoryMessage
  + Outcome in one handler (the dead-line trap: frozen behind the
  overlay, dropped by the chained teardown - fold the line into the
  overlay message or move it earlier). Tests for both + the clean shapes.
- [x] Content compliance pass, ledger (bundle 1.3.0 -> 1.4.0): ch2a/ch2b
  clock-gate the second opening line (elapsed > 8, seeded one-shot
  flag); fold the six dead lines into their Outcome overlay messages
  (keep the writing, kill the dead delivery).
- [x] Arrival-grace pass: engage_delay Some(8.0) on ch2a magpies, ch2b
  heavies, ch3's nav-ambush magpies; Some(5.0) on broadside's corvettes
  and shakedown's scavenger (builders + gen_content + parity). The
  Auditor stays hot (acked).
- [x] The beat-sheet convention in guide-author-scenario.md: announce ->
  breathe -> arrive -> fight -> confirm -> breathe -> next; one line per
  beat; every fight gets a lead-in; outcome messages carry the closing
  line; the tools table (clock, dwell, grace, delayed cut, timed
  banner).
- [x] Verify: content_lint CLEAN over the whole tree (the new arms ARE
  the acceptance); ledger_ch2_encounter + broadside_assault +
  content_ron_parity green; gen_content stable; nova_scenario
  --features serde; workspace --all-targets; fmt last. CHANGELOG +
  NOTES.md.

## Close-out record

All five steps landed; the fold-by-fold record and the grace map are in
NOTES.md. The acceptance was mechanical: the two new lint arms found 9
violations in shipped content and the pass drove them to zero, with the
arms now guarding future content. Verification: content_lint clean,
parity 2/2 (gen_content stable twice), ledger_ch2_encounter 12/12,
broadside_assault 11/11, balance_audit 0/0/2 acked, nova_scenario
--features serde 115 green, workspace --all-targets green, fmt last.
Full suite on CI per standing instruction.
