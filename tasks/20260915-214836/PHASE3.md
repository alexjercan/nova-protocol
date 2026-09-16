# Phase 3 - progress, completion, and the first-player route

Built in sprout `training-handbook`, on the Phase 2 content set. What a player
has read and been proven on is now a file of its own, and something in the
running game finally writes `Completed`.

## Two files, not one field

- `crates/nova_menu/src/training_store.rs` owns `PersistedTraining`: two
  explicit id lists, `viewed` and `completed`, and nothing else. No "first run"
  flag, no counts - a count derived from a list cannot disagree with the list.
- It is a SECOND file beside `settings.ron`, under the same root and the same
  `nova_assets::persist` codec. The settings file holds switches the player
  threw; what they learned is not a switch, and deleting `settings.ron` to fix
  an audio problem must not erase progress.
- The first-pilot dismissal stays in the settings, by the 2026-09-16 owner
  direction: `Not now` and `Start Basic Training` are answers to an offer, and
  Settings > Interface is where a player takes the answer back.
- Both fields carry a serde default, so a record written before a field existed
  still loads, and an id this build has no lesson for survives a save rather
  than being silently dropped.

## What may mark a lesson Completed

- A new lesson field, `proven_by`: the scenarios whose VICTORY proves this
  lesson. It is deliberately separate from `practice`, which is a BUTTON - one
  range you can fly, versus every scenario that counts as proof.
- `lint_lesson` checks it at lint and then at load: a duplicate name is an
  Error, a scenario no bundle provides is an Error, and a menu backdrop is an
  Error (nobody can win scenery). A `Chapter` may prove a lesson even though it
  may not be its practice button - which is exactly Basic Training's case.
- Authored across all 24 lessons in `base_content/lessons.rs`. Basic Training
  proves the eight lessons its objectives actually assert; `flight_momentum`
  and `combat_components` are deliberately NOT among them, because the tutorial
  never separately asserts either. Ten lessons are proven by a drill, fourteen
  by nothing yet.
- `complete_what_a_won_scenario_proves` is the only writer. It reads the live
  `CurrentOutcome` and `CurrentScenario`, ignores everything that is not a
  `Victory`, and marks every lesson whose `proven_by` names the won scenario.
  Opening a lesson, launching practice, and losing a range all prove nothing.
- It is registered OUTSIDE the store's access guard: completion is live state,
  and the access policy gates the FILE. A scripted run still draws the right
  rows; it just does not keep them.

## Isolation

- `TrainingStoreAccess` mirrors `SettingsStoreAccess`: `Inert`, `Read`,
  `ReadWrite`, with `from_env` going inert under `harness_env_active`. A
  scripted run cannot write lesson progress into the developer's profile, and
  a capture pass still READS, so a human's frames stay honest.
- `allow_training_saves` is the write grant, called by the one plugin that
  builds the handbook - the same split the settings store uses.
- The record coming into existence is not a change to keep: the save is skipped
  on the frame the resource is added and the startup load lands, so a boot in
  which the player learned nothing leaves no file behind.
- A missing or corrupt file reads as "nothing learned yet" rather than refusing
  to open the screen.

## The composed journey

`examples/systems/system_training_journey.rs` is the range the Verification
section asked for: three `App`s in one process on ONE temporary profile.

1. FRESH. An empty profile is offered Basic Training. `Open lessons` opens the
   handbook on the first lesson, a row click reads `combat_radar`, its Practice
   button launches `drill_gunnery` through the real hand-off, Target 1 dies
   through the production damage path, and the authored `OnDestroyed` handler
   wins the range. The four lessons that name `drill_gunnery` go to Completed,
   and the file is written.
2. RELAUNCH. A second app on the same profile comes up ON the saved record, and
   the list draws a badge for every proven lesson. The offer is still standing,
   because reading is not answering; `Not now` answers it, into the SETTINGS
   file, and the record beside it is untouched.
3. AGAIN. A third app: the corner is gone, the record is not, the menu card's
   `Lessons` row still opens the handbook, and the range the player was already
   proven on still launches from it.

Which lessons the range proves is read out of the live catalog, not listed in
the example: the claim is "exactly the lessons that name the range", and a
hand-copied list would stop being that the first time an author changes one.

Two doors it does NOT drive: `Start Basic Training` (the tutorial is a long
flight, and the button is already pinned by
`start_training_plays_the_declared_start_and_answers_the_offer`) and the
Interface row that brings the corner back
(`the_setting_is_what_decides_whether_the_corner_draws`). Both are live-tree
tests on the same widgets.

## Verified

- `cargo test -p nova_menu --lib training_store`: 16 passed. Four round-trip and
  record tests, two access-policy tests, five completion tests, and five
  file-level tests - restart survival, a reading store that writes nothing, an
  inert store that ignores a record already there, a corrupt record that reads
  as a fresh one, and a quiet boot that writes no file.
- `cargo test --lib`: `nova_training` 36, `nova_menu` 173, `nova_assets` 81,
  `nova_authoring` 106, all passed. The four new `validate.rs` tests cover
  `proven_by`'s duplicate, unknown-scenario, backdrop and chapter cases.
- `content gen` then `content lint`: 0 errors, 0 warnings, 0 findings, 13
  scenarios balance-audited. `content_ron_parity`, `lesson_media` and
  `lesson_wiki_links` pass on the regenerated bundle.
- `cargo test -p nova_probe_cli --test catalog_drift`: 2 passed, with the new
  range's 14 invariants on the systems roster.
- `NOVA_AUTOPILOT=1 system_training_journey`: exit 0, three clean cycles, every
  `PASS` line on the record.
- `probe run system_training_journey --correctness-only --norender`: OK.
  process_exit, run_completed, reached_playing (frame 82), invariants_held (0
  violations over 88 frames), log_clean and artifacts_loadable all PASS. All 14
  `outcome:` markers landed - seven on `timeline-fresh.jsonl`, four on
  `timeline-relaunch.jsonl`, three on the run's own `timeline.jsonl`.
- `cargo fmt --all --check` clean.

## One bug this phase caused and fixed

Adding the store to `NovaMenuPlugin` made the menu's own test fixture flaky, at
a rate of about three runs in eight: the store loads at startup and saves on
change, every menu test shares one temp root, and so a row click in one test
wrote a file the next run loaded over `dummy_progress`.
`the_progress_summary_counts_completed_and_opened` was the test that noticed,
because its counts are the only ones that read the record as a number.

The fixture now pins `TrainingProgressPlugin { access: Inert }` beside the
settings store it already pinned, and
`the_fixture_record_is_not_a_file_an_earlier_run_left` asserts both halves - the
access AND the drawn record - so a future edit that re-roots it fails here
instead of intermittently somewhere else.

## Known, deliberate, and left

- `prototype_progress` is GONE. The menu tests that needed a record with all
  three row states in it now build their own fixture beside `dummy_lessons`,
  where a fixture belongs.
- Fourteen lessons have an empty `proven_by` and can only ever be read. That is
  the honest state: nothing in the game asserts that a player has understood
  the camera, the mass budget or the mod browser, and inventing an assertion to
  fill the column would be exactly the filler the task forbids.
- The journey range is HEADLESS. Three apps live and die in one process and a
  winit event loop does not; the handbook's appearance is proven by the Phase 1
  and Phase 2 captures, not by this run.
- The range wins `drill_gunnery` rather than flying Basic Training end to end.
  Both are real scenario Victories through the same outcome machinery, and the
  drill's win condition is one authored `OnDestroyed` handler; what Basic
  Training proves is pinned separately by
  `basic_training_completes_every_lesson_its_flow_proves`.
- Phase 4 still owns the facts: non-repeating selection, and a home for the menu
  field note.
