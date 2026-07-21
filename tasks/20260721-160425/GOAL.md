# Goal: base campaign extension - Chapter 3 + voice pass

- DATE: 20260721
- UMBRELLA TASK: 20260721-160425
- LANDING SCOPE: squash-merge each task to master via sprout land; NO push
  (pushing is the user's call). Spike branch already landed (0fe918ee).

## Goal

Deliver task 20260718-152313 as directed by spike tasks/20260721-155249/SPIKE.md:
extend the base campaign with Chapter 3 - convoy-defense "Lifeline" plus
gravity-well finale "Final Tally", chained after broadside_gunship - and give
the existing chain its first StoryMessage voice/polish pass. Data/scenario
work only (v0.8.0 no-new-features); names from the spike are working
placeholders confirmed or renamed at Finish.

## Done means

1. New Game chain runs shakedown_run -> broadside -> broadside_gunship ->
   lifeline -> final_tally; the gunship victory chains onward instead of
   dead-ending. (test: harness asserts the NextScenario wiring)
2. The ally question is settled by evidence: a harness rig proves an
   `allegiance: Some(Player)` AI ship is acquired as a target by enemy AI
   (Lifeline primary), or the rig fails and the documented salvage-under-fire
   fallback is applied. (test: named rig test, red-or-green recorded)
3. Every scenario in the chain keeps win+lose Outcome paths and a checkpoint
   that never replays more than one fight. (test: harness + content lint)
4. The base chain speaks: StoryMessage comms per the beat-sheet convention.
   (cmd: `grep -l 'speaker:' assets/base/scenarios/*.content.ron` non-empty;
   lint clean)
5. `content lint` (refs + balance) passes; any balance WARN acks carry
   reasons. (cmd: `cargo run -p nova_assets --bin content -- lint`)
6. New scenarios follow the picker precedent: chapter head visible with
   thumbnail, continuation hidden. (cmd: grep hidden/thumbnail in generated RON)
7. Docs synced in-task: player wiki scenarios.md, CHANGELOG; playtest
   questions listed in the task. (manual: owner reads at Finish)
8. Each new fight is winnable AND losable - first tuning pass + probe
   evidence. (manual: owner playtest at Finish)

Overall: cargo check + fmt green, newly written tests green, content lint
green. Full cargo test/clippy stay in CI per repo policy (AGENTS.md).

## Tasks

- [x] 20260721-160842 (p56, nova-protocol) resolve asteroid_field hidden-vs-wiki contradiction
      landed 8c7be318; 2 review rounds (r1 caught a false history narrative +
      missing visibility pin); verdict: the hide premise was NEVER true -
      asteroid_field was the original New Game scenario; unhidden + pinned.
      Follow-up filed outside this goal: 20260721-163942 (CI fmt gate).
- [x] 20260721-160906 (p55, nova-protocol) harness-prove ally allegiance + orbit-directive guards (ch3 mechanisms)
      landed 16509993; 1 review round (APPROVE, no findings); verdict PRIMARY:
      ally convoy + orbit picket both proven, no fallback needed - Lifeline
      and Final Tally build the spike's primary variants.
- [x] 20260721-160929 (p54, nova-protocol) base chain voice pass (StoryMessage comms, hauler-survival flavor)
      landed 770bde4f; 1 review round (APPROVE, one MINOR pin added in-round);
      first voiced cast (Halloran/Rust Tally/Belt Relay), banners now track
      the Ceres Queen's fate; shakedown epilogue amendment: banner keeps the
      hook per the no-line-beside-Outcome lint.
- [x] 20260721-160957 (p53, nova-protocol) Lifeline (ch3a): convoy defense + gunship chain hook + picker wiring
      landed 4a1c0274; 1 review round (APPROVE, 2 MINORs + NIT fixed in-round);
      first shipped ally content (stalled controller-None + Player-allegiance
      convoy); probe verdict OK on the full 14-stage walk; balance audit clean,
      zero acks; broadside same-class act fix filed as 20260721-182034 (p47,
      outside this goal).
- [x] 20260721-161020 (p52, nova-protocol) Final Tally (ch3b): gravity-well anchorage finale + campaign ending
      landed 09463091; 1 review round (APPROVE, MINOR + NIT fixed in-round);
      the campaign completes properly (paced epilogue, nothing queued by
      design); balance clean with zero acks (berth moved out of its envelope
      instead); the lifeline example walks both ch3 parts, probe OK.
- [x] 20260718-152313 (p49, nova-protocol) campaign close-out: end-to-end verification, variety matrix, playtest questions, news note
      landed 831f35d2; 1 review round (APPROVE, 2 NITs no-action); chain
      verified end to end from shipped artifacts; variety matrix meets the
      DoD; 7 playtest questions batched below; news note drafted; CHANGELOG
      reordered into campaign order. Nothing else needed fixing.

## Done-definition verification (Finish, 2026-07-21)

1. Chain wiring: VERIFIED - grep of the shipped RON (shakedown->broadside,
   broadside->gunship x2, gunship->lifeline x2, lifeline->final_tally x4,
   final_tally retry-only) + the chain harness tests + both probe walks.
2. Ally mechanism: VERIFIED by rig (PRIMARY; 4 green rigs incl. the
   marker-less convoy shape) and shipped in Lifeline.
3. Win+lose + one-fight checkpoints: VERIFIED (31 harness tests + lint).
4. The chain speaks: VERIFIED (speaker grep: broadside, gunship, lifeline,
   final_tally; shakedown's banner amendment recorded with its lint
   rationale in 20260721-160929).
5. content lint: VERIFIED - 0 errors, 13 scenarios, base campaign zero
   acks.
6. Picker precedent: VERIFIED (heads visible + thumbnails, continuations
   hidden).
7. Docs synced + playtest questions listed: VERIFIED (scenarios.md,
   CHANGELOG, 7 questions in tasks/20260718-152313; owner reads at
   acceptance).
8. Winnable AND losable feel: PENDING the owner playtest (batched below).

Overall: cargo check + fmt green, 49 campaign-related tests green, lint
green on master (ed2ba5e6); full cargo test/clippy run in CI per repo
policy.

## Manual acceptance (batched for the user at Finish)

- (pending) spike names/tone: gang "the Tally", flagship "Final Tally",
  Captain Halloran, the Tallyman, Belt Relay, chapter "Lifeline" - confirm or
  rename (text-only edits).
- (pending) picker policy: chapter heads visible (Broadside precedent) kept;
  asteroid_field hidden-vs-wiki contradiction resolution.
- (pending) playtest: difficulty/pacing of lifeline + final_tally feel fair;
  relief timer length, wave sizes, escort count.
- (pending) 20260721-160929: comms pacing feel in play (7 new lines across
  Broadside) + the cast names nod (also listed under names/tone above).
