# Review: Generated placeholder thumbnails for the Scenarios picker

- TASK: 20260715-220011
- BRANCH: feat/scenario-thumbnails

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) tasks/20260715-220011/TASK.md:50 - Step 7 ("Inspect the
  picker at its shipped size and confirm EACH image is legible and distinct")
  is ticked, but the Evidence block (line 118) names one plate only - Broadside
  in `target/shots/news-090-scenario-campaigns.png`. The other 12 were rendered
  as files, never seen in the picker. Untick Step 7 and let the open `manual:`
  DoD item carry it, or rewrite the Evidence lines to state exactly which
  images were inspected and by what means.
  - Response: withdrawn. The owner inspected the rendered picker and accepted
    the plates; the reviewer had no way to see that. The tick stands.
- [x] R1.2 (MINOR) web/src/wiki/dev/guide-author-scenario.md:45 - the
  mod-author reference for the very field this task changed still teaches
  `thumbnail: Some("dep://base/banner.png")`, the shared-placeholder pattern
  `scenario_picker_thumbnails_are_distinct_and_not_shared_placeholders` now
  forbids; an author who follows it reproduces the Story's bug. Change the
  example to `thumbnail: Some("self://thumbnails/<id>.png")` and add one line
  pointing at `scripts/gen-scenario-thumbnails.py` and the mod's `resources`
  list.
  - Response: fixed. guide-author-scenario.md now shows
    `thumbnail: Some("self://thumbnails/my_scenario.png")`, says each scenario
    wants its own image and why, and points at the generator plus the bundle
    `resources` entry.
- [ ] R1.3 (MINOR) scripts/gen-web-screenshots.py:730 - `generated_placeholders`
  promises "the report never fails", but `except (OSError, ImportError)` guards
  only the module load; the comprehension below it calls
  `is_generated_placeholder` -> `glyph_mask`, which raises `ValueError` for a
  character missing from `FONT`
  (scripts/gen-scenario-thumbnails.py:247). A new scenario title with an
  unmapped glyph makes `--report` crash. Move the comprehension inside the
  `try` and return `[]` on `ValueError` too.
  - Response: declined. Reachable only by adding a scenario whose title uses a
    glyph `FONT` does not carry, which the generator itself refuses first with
    the same message. Not worth the churn.
- [x] R1.4 (MINOR) crates/nova_assets/tests/scenario_thumbnails.rs:120 - base
  entries are filtered by the picker rule at line 107
  (`!s.hidden || members.contains(&s.id)`), but the webmod loop applies no
  filter, so EVERY scenario in a webmod content file is required to own art.
  The test's own doc comment (line 12) says it mirrors `nova_menu`, which lists
  neither; a future hidden non-campaign webmod scenario - a menu backdrop, the
  shape base already has - would fail a test that claims to mirror the picker.
  Apply the same `!hidden || members.contains(&id)` filter to the webmod loop,
  using each mod's own campaign items.
  - Response: resolved by deleting the test. Owner call: the rig encodes a
    POLICY (no shared image, never `banner.png`) rather than a correctness
    property, and reusing `banner.png` as a thumbnail is a legitimate authoring
    choice it would have failed. `crates/nova_assets/tests/scenario_thumbnails.rs`
    is removed, with its Step and DoD line; the advisory coverage report is what
    lists a scenario with no art of its own.
- [ ] R1.5 (NIT) scripts/gen-scenario-thumbnails.py:226 - `draw_glyph`'s
  `amount=1.0` default is never used; its only caller, `draw_glyph_line`
  (line 355), always passes `amount`. `draw_glyph` and `glyph_mask` (line 240)
  also carry two copies of the same scale-expansion loop. Drop the default and
  build `draw_glyph_line` on `glyph_mask`.
  - Response: declined as a NIT, owner call.
- [ ] R1.6 (NIT) scripts/gen-web-screenshots.py:727 - `--report` loads
  `gen-scenario-thumbnails.py` by path, which loads a second full copy of
  `gen-web-screenshots.py` by path for `encode_png`. Harmless but doubles
  module execution; pass `encode_png` in, or hoist it into a shared helper.
  - Response: declined as a NIT, owner call. (The function is
    `scenario_thumbnail_rows`, misnamed above.)

Verified by the recording pass, independently of the round-1 reviewer:

- `python3 scripts/gen-scenario-thumbnails.py --check` - exit 0, "13 scenario
  thumbnail(s) match a fresh render (byte for byte)".
- `python3 scripts/gen-web-screenshots.py --report` - exit 0, 21 `manual` rows.
- `cargo test -p nova_assets --test scenario_thumbnails --test
  content_ron_parity` - 3 passed, 0 failed, including
  `base_bundle_ships_exactly_the_generated_files`.
- R1.1 re-derived from TASK.md directly: Step 7 ticked at line 50, Evidence at
  lines 118-119 names only the Broadside plate.
- R1.2 re-derived: guide-author-scenario.md:45 does carry the `dep://base/
  banner.png` example.
- R1.4 re-derived: line 107 filters base, the webmod loop from line 120 does
  not.

Not findings:

- The round-1 reviewer flagged the `zlib.compress(..., 9)` byte-for-byte
  dependency as undocumented. It is documented, at
  scripts/gen-web-screenshots.py:214-215. Dropped.
- Process signal: no CI target runs `--check`, so the committed PNGs can drift
  from the generator silently. That matches `gen-placeholder-sounds.py`'s
  existing contract, so the task did not introduce it - but the pair is now
  two generators with no drift guard.
- Out of scope: `assets/mods/example/example.content.ron` ships a non-hidden
  `example_arena` with no `thumbnail` at all. Picker-visible when the example
  mod is enabled, and outside both `SCENARIOS` and the new test.

Pending user checks:

- `manual:` inspect the Scenarios picker at its shipped layout size and accept
  the generated placeholders. Not resolvable by review; it does not block
  APPROVE, but R1.1 exists because Step 7 pre-empted it.

## Round 2

- REVIEWER: in-session (round 1's findings were resolved by owner decision -
  the manual inspection R1.1 asked for, and the call to delete the picker
  assertion R1.4 was about; neither is re-derivable by a fresh context)
- VERDICT: APPROVE

Round 1 closed:

- R1.1 withdrawn - the owner inspected the rendered picker and accepted the
  plates, which is the evidence the finding said was missing. Ticked.
- R1.2 fixed - `web/src/wiki/dev/guide-author-scenario.md` no longer teaches
  the shared-banner example. Ticked.
- R1.4 resolved by removing `crates/nova_assets/tests/scenario_thumbnails.rs`
  along with its Step and DoD line, plus the `development.md` sentence that
  promised the test. Owner call: the rig asserted a policy, not a property, and
  a scenario that deliberately reuses `banner.png` would have failed it.
  Ticked.
- R1.3, R1.5 and R1.6 declined as not worth the churn; MINOR and NIT, so they
  do not block.

Re-verified after the deletion:

- `cargo check -p nova_assets --all-targets` clean; `cargo fmt --check` clean.
- `cargo test -p nova_assets --test content_ron_parity` - 2 passed.
- `gen-scenario-thumbnails.py --check` exit 0, 13 byte-identical.
- `gen-web-screenshots.py --report` exit 0.
- No dangling reference to the deleted test remains outside `tasks/`
  (append-only history, exempt).

Pending user checks: none. The `manual:` DoD - the owner accepts the generated
placeholders in the rendered picker - was confirmed by the owner in round 1.
