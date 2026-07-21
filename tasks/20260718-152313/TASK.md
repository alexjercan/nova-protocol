# Base campaign polish + extension: make Shakedown to Broadside longer and more interesting (more beats/acts, pacing, encounters)

- STATUS: CLOSED
- PRIORITY: 49
- TAGS: v0.8.0,content,scenario,playtest

## Story

As a player starting New Game, I want the base campaign to carry me through a
fuller arc - more encounters, clearer stakes, story between the fights - so
that finishing it feels like completing a short campaign rather than sampling
two scenarios.

Direction settled by spike tasks/20260721-155249/SPIKE.md (2026-07-21):
append Chapter 3 (convoy-defense "Lifeline" + gravity-well finale "Final
Tally") and give the existing chain its first StoryMessage voice pass. The
build was split at plan time (flow umbrella 20260721-160425) into:

- 20260721-160842 (p56) asteroid_field hidden-vs-wiki resolution
- 20260721-160906 (p55) ch3 mechanisms rig (ally allegiance, orbit picket)
- 20260721-160929 (p54) base chain voice pass
- 20260721-160957 (p53) Lifeline (ch3a) + gunship chain hook
- 20260721-161020 (p52) Final Tally (ch3b) + campaign ending

THIS task is the campaign-level close-out: the end-to-end verification that
the sum delivers the original DoD, the playtest question list for the owner,
and the release-post note. It lands LAST.

## Steps

- [x] Verify the extended chain end to end from the evidence the sub-tasks
      left (harness chain-wiring tests + probe reports): New Game runs
      shakedown_run -> broadside -> broadside_gunship -> lifeline ->
      final_tally with lingering checkpoints at each seam.
- [x] Record the encounter variety matrix in this file (per-fight comp +
      shape) and check the original DoD line "no two consecutive encounters
      share composition and shape" against it.
- [x] Run the full `content lint` on the final tree; confirm every balance
      ack carries a reason.
- [x] List the playtest questions for the owner in this file (difficulty
      per fight, relief timer, names/tone nod, picker policy) - decided by
      the owner, not silently.
- [x] Write the v0.8.0 news-post note line (what the release post should
      say about the campaign) into this file; confirm CHANGELOG coherence
      across the landed sub-tasks (one voice, no duplicate entries).

## Definition of Done

- The base chain is at least one full scenario/act longer than v0.7.0's
  (it grows by two), with no two consecutive encounters sharing the same
  composition and shape (matrix recorded here; cmd:
  `grep -n "Variety matrix" tasks/20260718-152313/TASK.md`).
- Every scenario in the chain has an Outcome path for both win and lose, a
  checkpoint structure that never replays more than one fight on death, and
  comms beats following the beat-sheet convention
  (cmd: `cargo run -p nova_assets --bin content -- lint`).
- scenarios.md and CHANGELOG reflect the new chain
  (cmd: `grep -n "Lifeline\|Final Tally" web/src/wiki/scenarios.md CHANGELOG.md`).
- Playtest questions for the owner are listed in this task, not silently
  decided (cmd: `grep -n "Playtest questions" tasks/20260718-152313/TASK.md`).

## Notes

- T1 verdict (20260721-160906, 2026-07-21): PRIMARY variant confirmed.
  `allegiance: Some(Player)` on an AI ship survives the spawn path
  (nova_scenario `authored_allegiance_overrides_the_controller_default`,
  Player case) and the relation model treats the ally as a first-class
  combatant both ways, with nearest-hostile fire draw
  (nova_gameplay `ally_relation_tests`: 3 rigs). Orbit-directive picket:
  already pinned by shipped tests (`combat_interrupts_the_orbit_and_calm_
  resumes_it`, `an_orbit_directive_wins_the_passive_fallback` in ai.rs) -
  use the orbit directive for Final Tally's picket, no fallback needed.
  OnDestroyed emission is allegiance-agnostic at the source
  (integrity/explode.rs `on_destroyed_entity` fires for any id-carrying
  entity), so ally-death Defeat wiring is authorable; pin it in the
  Lifeline harness test.
- Spike: tasks/20260721-155249/SPIKE.md (2026-07-21, RECOMMENDED) - the
  extended-arc beat sheet this task's original step 2 asked a nod on. Key
  finding: AI targeting is relation-driven (ai.rs update_ai_target) and
  `allegiance: Some(Player)` is authorable; unshipped, so the rig task
  decides Lifeline's primary vs fallback variant.
- Depends on: 20260721-160842, 20260721-160906, 20260721-160929,
  20260721-160957, 20260721-161020 (all must be CLOSED).
- Base scenarios: `assets/base/scenarios/*.content.ron` are GENERATED - edit
  the `nova_assets` builders and run `content gen`, never the .ron directly
  (LESSONS.md: edit-the-builder-not-the-generated-ron).
- Feel/balance is ultimately the user's call; deliver the content + a first
  tuning pass, flag playtest questions.
- Menu ambience scenes are separate and out of scope here.

## Close-out record (2026-07-21)

### Chain verification (end to end, from the shipped artifacts)

NextScenario targets grepped from the generated RON: shakedown_run ->
broadside (win) / self (retry); broadside -> broadside_gunship (2 fate
variants) / self; broadside_gunship -> lifeline (2 variants) / self;
lifeline -> final_tally (4 win variants) / self (2 defeat paths);
final_tally -> self (retry only) - the victory queues NOTHING by design,
stated in the banner. Checkpoints never replay more than one fight.

Harness evidence re-run on this branch: broadside_assault 14 green,
lifeline_convoy 8 green, final_tally_claim 7 green, content_ron_parity 2
green. Probe fleet `run broadside,lifeline`: aggregate OK (broadside 343s
real-time walk of ch2 both parts + the chain assert into lifeline;
lifeline 15s clock-compressed walk of ch3 both parts to the
campaign-complete banner). Together the five-scenario chain is played
end to end by scripted walks of the real app.

### Variety matrix (per-fight composition + shape)

1. Shakedown (final beat): 1 light scavenger; open debris field; duel-exam.
2. Broadside p1: 2 light corvettes, simultaneous; chaff + boulder bowl;
   break-the-ambush.
3. Broadside p2: 1 full-grade capital (torpedoes); same bowl; PDC-screen +
   section kill.
4. Lifeline: 3 staged waves of lights (2 / 3 / 1 full-grade + 1 light);
   open lane + stalled ally convoy; PROTECT under a relief countdown.
5. Final Tally: 2 light orbital pickets, then capital + light escort;
   gravity well + ring belt + wreck cover; survey -> picket -> capital
   assault.

No two consecutive fights share composition AND shape; every scenario has
win + lose Outcome paths and its own one-fight retry. DoD line met.

### content lint

0 errors over 13 balance-audited scenarios. The base campaign carries ZERO
balance acks (the one WARN + 2 acks are the Ledger mod's pre-existing
Auditor entries, reasons recorded in balance_acks.ron).

### Playtest questions (owner decides; nothing silently decided)

1. Names/tone nod (spike placeholders, single-constant renames in
   cast.rs): the gang "the Tally", boss "The Tallyman", flagship "Final
   Tally", Capt. Halloran, Belt Relay; chapter titles "Lifeline" /
   "Final Tally".
2. Lifeline difficulty: relief 240s; waves 2 / 3 / 1 full-grade corvette +
   1 light; raider engage graces 8s; leash 520. Winnable AND losable in
   YOUR hands?
3. Lifeline convoy image: the haulers are STALLED (drives cold) rather
   than the spike's crawling convoy - does the stalled read work, or is a
   crawling convoy (AI patrol + leash gymnastics) worth a follow-up?
4. Final Tally peak: full-grade flagship + 1 light escort after a 2-picket
   fight - is the escort grade right, or should the finale bite harder
   (full-grade escort / second escort)?
5. Epilogue pacing: 4s to the close line, 9s to the banner - does the
   beat land, or drag?
6. Broadside voice pass: 7 comms lines across ch2 - right density?
7. Picker policy confirmation: chapter heads visible (shakedown_run,
   broadside, lifeline + the restored asteroid_field), continuations
   hidden (gunship, final_tally, asteroid_next).

### v0.8.0 news-post note

Suggested line for the release post: "The base campaign more than doubles:
after the Rust Tally falls, the gang hits back at the belt's supply convoy
(Lifeline - hold the lane under a live relief countdown while the haulers
genuinely draw fire), and the trail ends at their claim (Final Tally - a
gravity-well anchorage assault with a real ending). The whole chain now
speaks over the comms panel, and the Asteroid Field sandbox is back in the
picker."

### CHANGELOG coherence

All five landed entries read in one voice under Unreleased > Scenarios &
Objectives; reordered Lifeline before Final Tally so the section reads in
campaign order (the two entries cross-reference).

Reflection: the close-out found nothing to fix beyond the CHANGELOG swap -
the per-cycle discipline (docs-in-task, lint-per-land, probe-per-scenario)
left no debt for the sweep, which is the point of paying it per cycle.
