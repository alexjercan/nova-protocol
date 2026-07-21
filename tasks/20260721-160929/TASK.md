# Base chain voice pass: StoryMessage comms, imperative objectives, hauler-survival flavor

- STATUS: OPEN
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

- [ ] Add shared speaker constants for the base cast in the nova_assets
      scenario builders (one module, single-point rename), with a comment
      that names await the owner nod.
- [ ] Broadside (crates/nova_assets/src/scenario/broadside.rs): move the
      story-bearing objective text (distress call, ambush spring, victory
      hook) into StoryMessage lines per the beat sheet (one line per beat,
      clock-gate consecutive lines); shrink the objectives to imperatives;
      add a first-corvette-down line.
- [ ] Broadside Gunship: same treatment (capital-burn warning line, victory
      closing); torpedo-screen instruction stays an objective (it is a goal,
      not voice).
- [ ] Conditional victory flavor: track hauler survival in a scenario-local
      variable in BOTH broadside parts; two act-gated victory handlers per
      scenario whose Outcome message varies (ledger lesson:
      gate-scenario-handlers-to-their-acts).
- [ ] Shakedown: the closing "distress call" hook becomes a comms line in
      the epilogue beat; tutorial objective text untouched.
- [ ] `content gen`; run `content lint` (beat-sheet arms must stay clean:
      no line beside an Outcome, dwell in range); fix parity tests.
- [ ] Docs sweep per keeping-docs-in-sync: CHANGELOG Unreleased entry; check
      whether any wiki page quotes the old objective text
      (cmd: grep the moved phrases repo-wide, excluding tasks/).

## Definition of Done

- The base chain speaks
  (cmd: `grep -l "speaker:" assets/base/scenarios/*.content.ron` lists
  shakedown_run, broadside, broadside_gunship).
- content lint green including beat-sheet arms
  (cmd: `cargo run -p nova_assets --bin content -- lint`).
- Victory text varies with hauler survival, proven by a rig or loader-level
  assertion (test: name recorded here when written).
- Parity tests green after regen (test: `content_ron_parity`).
- CHANGELOG names the voice pass (cmd: `grep -n "voice" CHANGELOG.md`).

## Notes

- Spike: tasks/20260721-155249/SPIKE.md (Polish pass). Umbrella: 20260721-160425.
- Edit the BUILDERS, never the generated RON (LESSONS:
  edit-the-builder-not-the-generated-ron).
- Depends on: nothing (independent of the rig task); lands before Lifeline
  so ch3 reuses the cast constants.
