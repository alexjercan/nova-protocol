# Review: Build systems/: code-built fixtures for scenario grammar, the player path, and outcomes

- TASK: 20260804-093934
- BRANCH: feat/systems-examples

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

The code is right and the run is real. Every DoD proof was reproduced green by
both the out-of-context reviewer and this pass: `probe run systems` OK on all
three (5/6 each, the sixth `fps_within_baseline` SKIPPED as `frame_time: false`
requires), the `outcomes` run log walks die -> Defeat overlay -> Retry -> clean
reload -> kill -> Victory + queued chain -> `outcome_probe_b` loaded with
`cycle complete, no panic`, `player_path` runs both rounds' full gesture chain,
`catalog_matches_disk` and `systems_reach_playing_without_panic` ok,
`cargo test -p nova_probe --lib invariants` 12 passed including the new
`a_gapless_reload_resets_monotonic_memory`, `cargo fmt --check` clean, both
shell proofs exit 0.

What blocks the round is the doc sweep. The rename deleted
`examples/gameplay/scenario.rs` and edited three doc surfaces PARTIALLY, so
those files now contradict themselves or point at a path that no longer exists.
A half-swept doc is worse than an untouched one, because the stale half reads
as current.

- [x] R1.1 (MAJOR) web/src/wiki/dev/scenario-system.md:31 - points at
  `examples/gameplay/scenario.rs`, which this branch deletes. The same dead
  path is at `web/src/wiki/dev/architecture.md:117` and
  `web/src/wiki/dev/guide-author-scenario.md:1072`. AGENTS.md's routing map
  (`web/src/wiki/dev/keeping-docs-in-sync.md:57`) names
  `dev/scenario-system.md` and `dev/guide-author-scenario.md` as the scenario
  engine's doc surface, so this sweep was owed in this task. Repoint all three
  at `examples/systems/scenario_grammar.rs`.
  - Response: fixed - `examples/gameplay/scenario.rs` repointed at `examples/systems/scenario_grammar.rs` in `dev/scenario-system.md:31`, `dev/architecture.md:117` and `dev/guide-author-scenario.md:1072`. The remaining hits (`crates/nova_probe/src/catalog.rs:255,274,298`) are synthetic manifest strings inside that module's own unit tests, not doc surfaces, and are left alone.
- [x] R1.2 (MAJOR) .claude/skills/probe/SKILL.md:142 - the diff updated three
  lines of this file and left it self-contradictory: line 154 now says
  `probe run scenario_grammar`, while the depth table above still lists
  `| gameplay/scenario | monotonics: beat, rocks_destroyed |` (142) and
  `| gameplay/playable | monotonics: target_down, leg + 7 beat markers |`
  (144), and the prose at 159 reads "Enrolled scenes (gameplay/scenario,
  playable - `loop_while_pending`)". Rename both rows to
  `systems/scenario_grammar` / `systems/player_path`, refresh the monotonic
  lists against `examples/systems/scenario_grammar.rs:189` (7 registered
  monotonics now, not 2), and drop `loop_while_pending` - it was removed by
  the predicate-driven autopilot rewrite and the enrolled scenes now use
  `loop_from`.
  - Response: fixed - the depth table rows are now `systems/scenario_grammar` (all seven monotonics listed) and `systems/player_path`, plus a new `systems/outcomes` row (monotonic `hostile_down` + its 4 beat markers, counted from the source). The enrolled-scenes prose now reads "the systems/ runs - `loop_from`"; `loop_while_pending` no longer exists outside the CHANGELOG entry that records its removal.
- [x] R1.3 (MAJOR) web/src/wiki/dev/development.md:648 - this diff rewrote
  large parts of this page but left the invariants paragraph reading
  "opt-in per example: playable registers `target_down`/`leg`, scenario
  `beat`/`rocks_destroyed`", naming two examples that no longer exist beside a
  category table that now describes `systems/`. It also says nothing about the
  behavior change this branch shipped in `nova_probe`: monotonic memory is now
  forgotten on `ScenarioLoaded` (`crates/nova_probe/src/invariants.rs:155`),
  not only when a key vanishes. Update the example names and add one clause
  for the reload reset.
  - Response: fixed - the invariants paragraph now names `player_path`, `scenario_grammar` and `outcomes`, and states the new rule: a monotonic is one-way within a SCENARIO LIFE, forgotten on `ScenarioLoaded`, so a replaying example re-seeds its latches without a false regression.
- [x] R1.4 (MINOR) examples/systems/player_path.rs:42 - the file was renamed
  but does not rename itself: `#[command(name = "playable")]`, against a
  convention every other example follows (`outcomes.rs:34`,
  `scenario_grammar.rs:38`, all of `sections/`). Every emitted line still
  carries the `playable:` prefix - confirmed in
  `probe-runs/1b9c696e/player_path/run.log:259` and `:466`, which read
  `player_path: playable: round 1 - ...`. The module doc at lines 32-33 pins
  those stale strings as the "look for" output. Change the clap name to
  `player_path`, drop or rename the log/assert prefixes, and update the doc
  block's grep strings to match.
  - Response: fixed - `#[command(name = "player_path")]`, and every `playable:` message prefix is now `player_path:`. The doc block's grep strings match: `probe-runs/abde17e3/player_path/run.log:258` and `:471` read `player_path: round 1/2 - ...`. `SCENARIO_ID`/`playable_run` are left as-is - they name the SCENARIO, not the example, and are not user-facing grep targets.
- [x] R1.5 (MINOR) examples/systems/outcomes.rs:22 - the module doc tells a
  reader to look for `outcomes: defeat overlay up, retrying`. No such line is
  emitted anywhere in the file, and it is absent from
  `probe-runs/1b9c696e/outcomes/run.log`. Either add an `info!` to a beat after
  `defeat_overlay_up()` holds, or delete line 22.
  - Response: fixed - rather than delete the doc line, added a `report the defeat overlay` beat that emits it, matching the file's existing `report the ...` pattern. Confirmed live at `probe-runs/abde17e3/outcomes/run.log:167`.
- [x] R1.6 (MINOR) CHANGELOG.md:15 - the Unreleased "Internals & Tooling" block
  documents exactly this class of change (harness renames, category contracts,
  `nova_debug::harness` predicates) and line 56 in it still names `playable` as
  a live example. Nothing was added for `scenario` ->
  `systems/scenario_grammar`, `playable` -> `systems/player_path`, the new
  `outcomes` example, or the monotonic reload reset. `cargo run --example
  playable` and `probe run playable` both break for anyone on the old names.
  Add one Internals & Tooling entry covering the renames plus the invariant
  fix, and correct line 56.
  - Response: fixed - added two Unreleased / Internals & Tooling entries: the `systems/` category with both renames and the new `outcomes` example, tagged **(breaking)** since the old `--example`/`probe run` names are gone; and the monotonic reload reset. Line 56's stale `playable` is now `player_path`.
- [x] R1.7 (NIT) examples/systems/outcomes.rs:100 - `defeat_overlay_up()` waits
  on the `Outcome Overlay` entity, but the next step's
  `activate_named("Outcome Primary Button")` panics if the BUTTON is missing.
  This is NOT the frame-lag race D6 describes - the overlay and the button are
  spawned in one `commands.spawn(...).with_children(...)` batch
  (`crates/nova_menu/src/outcome.rs:68-136`), so they land together. The real
  gap is conditional: the button only spawns when `primary.is_some()`
  (`outcome.rs:133`), which this run guarantees via the requeue. Still worth
  waiting on the entity the next beat actually presses, so a fixture that
  forgets the requeue stalls on a named step instead of panicking one beat
  later. Extend `outcome_overlay_up` to also require an entity named
  `Outcome Primary Button`.
  - Response: fixed - `outcome_overlay_up` now waits on `Outcome Primary Button` instead of `Outcome Overlay`, and its docstring carries the corrected mechanism (one command batch, so the gap is conditional existence, not frame lag).
- [x] R1.8 (NIT) README.md:81 - "The `examples/` tree is grouped by category
  (`gameplay`, `screenshots`, `perf`, ...)" leads with a TRANSITIONAL directory
  name and omits `systems`, which is now the tree's main category. Replace
  `gameplay` with `systems` in that list.
  - Response: fixed - the category list now reads `systems`, `sections`, `ui` rather than leading with the transitional `gameplay`.
- [x] R1.9 (NIT) examples/systems/scenario_grammar.rs:63 - `AREA_ROCK`'s
  comment justifies itself by "the rounds ... work the ring down in index
  order", but `scoped_entities` sorts ids as STRINGS
  (`scenario_grammar.rs:553`), so the order stops being numeric at
  `ASTEROID_COUNT >= 10`. It is 6 today, so nothing is broken, and the
  justification is unneeded anyway because the area beats run before any round.
  Trim the comment to that fact, or sort numerically.
  - Response: fixed - the index-order justification is gone; the comment now rests on the fact that carries it, that the area beats run before any round.

Process signal: D5 and D6 record two deviations honestly rather than silently
ticking the Step - the rounds stop at GOTO-closing instead of `arrive`, and a
production `nova_probe` fix was pulled in. The `arrive` clause of Step 3 is
literally undelivered, but the deviation is argued and the coverage is
relocated. Recording it beat hiding it.

Process signal: the Step that demanded RUNNING under Xvfb rather than
`cargo check` is vindicated three times over in Close-out. None of those three
bugs was reachable by a check, and two would have shipped an example that
proves nothing.

Process signal: the doc sweep was scoped by a Step that named only
`Cargo.toml`'s comments and `dev/development.md`'s category table. Everything
in R1.1-R1.3 and R1.6 sits outside that list. A rename Step should own the
grep, not an enumerated file list.

Pending user checks: none. The DoD carries no `manual:` proofs.

Inspection commands:

```sh
cd "$(sprout show feat/systems-examples)"
rg -n 'examples/gameplay/(scenario|playable)' web/ .claude/ README.md
rg -n 'playable|scenario_grammar' .claude/skills/probe/SKILL.md CHANGELOG.md
DISPLAY=:99 nix develop --command cargo run -p nova_probe -- run systems
nix develop --command cargo test --test examples_smoke
nix develop --command cargo test -p nova_probe --lib invariants
```

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

All nine round-1 findings verified fixed on disk by a fresh out-of-context
reviewer, which also confirmed the two responses that pushed back on a
finding's stated mechanism (R1.7's frame-lag reading, R1.4's `SCENARIO_ID`).
Their boxes are ticked above on that confirmation. No functional regressions in
the fix commit. Three cosmetic findings remain, none blocking.

Re-verified independently at `c74962b8`: `probe run systems` OK 3/3 (5/6 each,
the sixth `fps_within_baseline` SKIPPED by category contract),
`probe run outcomes` standalone OK with all four documented `outcomes:`
landmarks emitted in order, `cargo test --test examples_smoke` 8/8 under
`DISPLAY=:99`, `cargo test -p nova_probe --lib invariants` 12 passed,
`cargo fmt --check` clean, both DoD shell proofs exit 0.

- [ ] R2.1 (NIT) .claude/skills/probe/SKILL.md:145 - the new `systems/outcomes`
  row's marker parenthetical is wrong. Reported as five markers per cycle; the
  timeline shows SIX (`probe-runs/abde17e3/outcomes/timeline.jsonl`: kill,
  defeat overlay, activate, kill, activate, done) - `beat: kill` repeats as
  well as `beat: activate`. State the distinct count and the real order.
  - Response: fixed - now reads "4 distinct beat markers, 6 per cycle (kill ->
    defeat overlay -> activate -> kill -> activate -> done)", counted off the
    timeline rather than off the source order.
- [ ] R2.2 (NIT) examples/systems/outcomes.rs:577 - `report_defeat_overlay`'s
  docstring says "the step's predicate held both the overlay and the button",
  but R1.7's fix made the predicate check the button only. Reword.
  - Response: fixed - now "held the Defeat outcome and the button".
- [ ] R2.3 (NIT) .claude/skills/probe/SKILL.md:160 and
  web/src/wiki/dev/development.md:652 - both round-1 edits left a short
  unwrapped line mid-paragraph. Reflow to the file's wrap width.
  - Response: fixed - both paragraphs reflowed.

Process signal: `cargo test --test examples_smoke` inherits an ambient
`DISPLAY` and fails `systems_`/`ui_reach_playing_without_panic` on `:0`; it is
green 8/8 under `:99`. The DoD's `test:` proof is `catalog_matches_disk`, which
needs no display, so no proof line is wrong - but the smoke tests' display
dependency is undocumented and the next example task will trip on it.

Pending user checks: none. The DoD carries no `manual:` proofs.
