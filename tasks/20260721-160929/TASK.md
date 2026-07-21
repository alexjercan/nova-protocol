# Base chain voice pass: StoryMessage comms, imperative objectives, hauler-survival flavor

- STATUS: CLOSED
- PRIORITY: 54
- TAGS: v0.8.0,content,scenario

## Story

The base campaign has ZERO StoryMessage comms - all narrative rides
objective text and outcome banners; the only voiced speakers in shipped
content live in the Ledger mod (spike tasks/20260721-155249/SPIKE.md,
Context). Give the base chain its voice: a small recurring cast, comms
lines per the beat-sheet convention, imperative-short objectives, and
victory text that finally acknowledges whether the Ceres Queen survived.
Names are working placeholders (confirmed/renamed at flow Finish): Captain
Halloran (Ceres Queen), the Tallyman (gang boss), Belt Relay (dispatch).

## Steps

- [x] Add shared speaker constants for the base cast in the nova_assets
      scenario builders (one module, single-point rename), with a comment
      that names await the owner nod. (cast.rs: Capt. Halloran, Rust Tally,
      Belt Relay; the Tallyman waits for ch3 so no dead consts ship.)
- [x] Broadside (crates/nova_assets/src/scenario/broadside.rs): move the
      story-bearing objective text (distress call, ambush spring) into
      StoryMessage lines per the beat sheet (one line per beat; act-gated,
      no clock gates needed - every line rides its own event); shrink the
      objectives to imperatives; add a first-corvette-down line (two
      mutually-exclusive handlers gated on the other kill flag). Victory
      hooks stay in the banners per the convention.
- [x] Broadside Gunship: same treatment - the Rust Tally taunts on
      arrival (the capital-burn warning already lives in part one's
      checkpoint banner); torpedo-screen instruction stays an objective,
      shrunk to the imperative.
- [x] Conditional victory flavor: track hauler survival in a scenario-local
      variable in BOTH broadside parts; two act-gated victory handlers per
      scenario whose Outcome message varies (ledger lesson:
      gate-scenario-handlers-to-their-acts). The soft-fail beat now raises
      the flag + speaks via Belt Relay instead of pushing a HUD objective.
- [x] Shakedown: AMENDED - the hook STAYS in the Victory banner: the
      epilogue beat fires its Outcome in the same handler, and the lint
      forbids a StoryMessage beside an Outcome (the banner carries the
      closing line by convention). The voice pickup is Broadside's opening
      line instead: Halloran's spoken distress IS the promised call.
      Tutorial text untouched; shakedown.rs only gains the shared story()
      helper.
- [x] `content gen`; `content lint` clean (0 errors; pre-existing Ledger
      WARN + 2 acks only); parity tests green.
- [x] Docs sweep per keeping-docs-in-sync: CHANGELOG Unreleased entry;
      repo-wide grep of the moved phrases (excluding tasks/) found no
      other surface; scenario descriptions and wiki text remain true.

## Definition of Done

- The base chain speaks
  (cmd: `grep -l "speaker:" assets/base/scenarios/*.content.ron` lists
  broadside, broadside_gunship; shakedown AMENDED out - its hook must stay
  in the banner because the epilogue handler carries the Outcome and the
  lint forbids a line beside it; see the step note and NOTES.md).
- content lint green including beat-sheet arms
  (cmd: `cargo run -p nova_assets --bin content -- lint`).
- Victory text varies with hauler survival, proven by a rig
  (test: `victory_banner_reflects_the_haulers_fate` - both parts, both
  branches, driven through the act machine; plus the reshaped
  `hauler_death_on_a_live_act_pushes_the_soft_fail_beat` pinning
  flag + Belt Relay line + objective absence).
- Parity tests green after regen (test: `content_ron_parity`).
- CHANGELOG names the voice pass (cmd: `grep -n "voice" CHANGELOG.md`).

## Notes

- Spike: tasks/20260721-155249/SPIKE.md (Polish pass). Umbrella: 20260721-160425.
- Edit the BUILDERS, never the generated RON (LESSONS:
  edit-the-builder-not-the-generated-ron).
- Depends on: nothing (independent of the rig task); lands before Lifeline
  so ch3 reuses the cast constants.

## Record (2026-07-21)

What changed: cast.rs (3 speaker consts), story() helper in shakedown.rs,
broadside.rs voice pass (7 comms lines across both parts, imperative
objectives, hauler_lost flag + gated Victory variants, soft-fail beat
reshaped from objective to comms), regenerated RON, CHANGELOG entry, and
the broadside_assault.rs updates (2 tests reshaped, 1 new variant test, 2
tests gained the hauler_lost seed). Design record: NOTES.md.

Difficulties: none structural. The one real design collision - the planned
shakedown epilogue comms line vs the no-line-beside-Outcome lint - resolved
in the content's favor (banner keeps the hook; Broadside's opening speaks
it), recorded in the amended step and NOTES.md.

Verification: broadside_assault 13 green (incl. the new variant test),
content_ron_parity 2 green, content lint 0 errors, cargo check green, fmt
clean. The broadside autopilot example stages on the act machine which is
structurally unchanged (both victory variants set act=2), so it is
unaffected; no probe run this task (text/flag changes only - the ch3 tasks
carry the probe steps). Full clippy/test suite left to CI per repo policy.

Reflection: writing the variant test BEFORE converting part two would have
caught the missing hauler_lost seed in two older tests one compile earlier;
otherwise the beat-sheet conventions made the pass mechanical - the
authoring stack from v0.7.0 held up with zero engine changes.
