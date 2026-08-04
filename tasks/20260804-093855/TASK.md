# Example categories: write the contract and resolve probe run policy from it

- PRIORITY: 85
- TAGS: v0.10.0, tooling, examples, testing
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244

## Story

Give every example category an explicit contract and teach `nova_probe` to
resolve its run policy from that contract, replacing the `perf`-vs-everything
split plus the hand-listed `fps_exempt`.

The roster spike (`20260804-003244`) settled five categories: `sections/`,
`systems/`, `stress/`, `ui/`, `screenshots/`. `gameplay/` and `perf/` are
retired as directory names - `gameplay/` was never a contract, and every
frame-time claim now lives in `stress/`.

This is code, not content. Every other task in the rebuild depends on it.

**Scope boundary (planning correction, 2026-08-04):** this task ships the
CONTRACT and the POLICY TABLE only. It moves no example and renames no
directory - `examples/gameplay/` and `examples/perf/` still exist on disk when
it lands, and `20260804-093910` / `093934` / `094006` move them, each atomic
with its own `tests/examples_smoke.rs` edit. So the policy table ships
transitional rows for `gameplay` and `perf`, and this task does not touch the
smoke-list consts.

## Steps

- [x] Add `CategoryPolicy` + `category_policy(category: &str)` to
      `crates/nova_probe/src/catalog.rs`, re-exported from `lib.rs:147`. Two
      fields, not three (see DECISION):
      `probed: bool` (probe runs it at all - gates both `--all` and bare
      category expansion) and `frame_time: bool` (carries the `--fps` pass).
      Rows: `sections`/`systems`/`ui` probed-no-fps, `stress` probed-with-fps,
      `screenshots` unprobed, plus TRANSITIONAL `gameplay` (probed-no-fps) and
      `perf` (probed-with-fps) carrying a `# remove with <task-id>` comment.
      Unknown category -> `probed: true, frame_time: false` (today's non-`perf`
      behavior), never silently reached (see the new smoke gate below).
- [x] Replace the two consumers of the old split:
      `resolve_fps_window` (`native/env.rs:64-65`, `if category == "perf"`)
      becomes `category_policy(category).frame_time`; `example_fps_policy`
      (`env.rs:33-51`) sources its reason string from the policy instead of
      `load_fps_exempt`, with the reason naming the category
      ("category `sections/` carries no frame-time pass").
- [x] Make `resolve_spec` (`native/spec.rs:33`) honor `probed`: the `--all`
      branch skips unprobed categories, recording each in `excluded` with its
      reason so the aggregate report shows the absence as a decision; a bare
      `probe run screenshots` fails with "category `screenshots/` is not a
      probe target" rather than expanding to a no-op. `NOT_PROBED` stays - it
      is per-EXAMPLE (`render_scale_shot`), an orthogonal axis.
- [x] Delete `parse_fps_exempt` / `load_fps_exempt` (`catalog.rs:113-176`),
      their four unit tests, and the `lib.rs:147` re-exports. Leave the
      `Cargo.toml` key alone - `20260804-093910` / `094006` own it.
- [x] Rename the report manifest's `fps_exempt: Option<String>` field to
      `fps_skipped`, keeping `Option<String>`; no schema shape change, no
      compatibility shim. Surfaces are wider than first scoped - all of
      `run_report/manifest.rs:31-34,73,125`, `run_report/html.rs:181-193`,
      `run_report/checks.rs`, `run_report/fixtures.rs`,
      `native/run.rs:64,106,120,180,228,269,316,327,341-342,374-378` and
      `native/report.rs`.
- [x] Write the contract down in its two homes: the `Cargo.toml` catalog
      section comments (per-block, so they stay true to what is on disk), and
      `web/src/wiki/dev/development.md` - the category list at :118-174 gains
      the five-row contract table (what each proves, what probe does with it,
      what disqualifies an example from it), and the `fps_exempt` prose at
      :477-489 is rewritten as category policy.
- [x] Add `every_category_has_a_probe_policy` to `tests/examples_smoke.rs`
      beside `catalog_matches_disk`: every category in the root catalog has an
      explicit policy row, so the unknown-category default can never quietly
      apply. It lives here because `CARGO_MANIFEST_DIR` is the repo root here.

## Definition of Done

- Probe resolves run policy per category: `stress` gets the full frame-time
  window, `sections`/`systems`/`ui` get correctness only, and `--all` skips
  `screenshots` with a recorded reason.
  (test: `category_run_policy_selects_passes_per_category`)
- No category on disk falls through to the unknown-category default.
  (test: `every_category_has_a_probe_policy`)
- The catalog, the on-disk layout and the smoke lists still agree - this task
  changes none of the three. (test: `catalog_matches_disk`)
- The exempt-list mechanism is gone from probe's code and from the dev wiki;
  only the orphaned `Cargo.toml` key remains, owned by a later task.
  (cmd: `! rg -n 'fps_exempt' crates/nova_probe web/src/wiki`)
- The contract names all five settled categories.
  (cmd: `rg -n 'stress/' Cargo.toml web/src/wiki/dev/development.md`)

All five verified RED on `master` at plan time.

## Notes

Owner principle 2026-08-04: **do not add tests for our tests. Examples should
NOT be tested - they ARE tests.** `catalog_examples_satisfy_their_category_contract`
is dropped: it would have inspected example SOURCE to judge whether a run
asserts enough, which is testing a test. The contract lives in the catalog
comments and the dev wiki page, and review enforces it.

The three tests this task touches all clear that bar - none judges an
example's content:

- `category_run_policy_selects_passes_per_category` (new) tests `nova_probe` -
  production code with real branching.
- `every_category_has_a_probe_policy` (new) pins catalog strings against the
  policy table. Same class as `catalog_matches_disk`: names, never behavior.
- `catalog_matches_disk` (existing, untouched) is a BUILD-INTEGRITY gate: with
  `autoexamples = false`, an example missing from the catalog does not compile
  at all.

### Planning corrections to the pre-plan draft

- **The smoke-list rename is NOT this task's.** The draft's step 4 renamed
  `GAMEPLAY` -> `SYSTEMS`/`STRESS` and added the matching
  `*_reach_playing_without_panic` tests. Those consts must move in the SAME
  commit as the directory they name (`catalog_matches_disk` goes red
  otherwise), and `093934` / `094006` own those directories. Doing it here
  would land a red tree. Step dropped.
- **The doc sweep is one page, not ten.** `rg 'gameplay/'` matches
  `crates/nova_gameplay/`, which inflated the count. Under a tight pattern the
  real hits are: `development.md` (this task's - it owns the category list and
  the `fps_exempt` prose); `architecture.md:117`, `scenario-system.md:31` and
  `guide-author-scenario.md:1072`, which all cite
  `examples/gameplay/scenario.rs` and belong to `093934` with the rename;
  `modding-ron.md:27` `broadside`, which is the SCENARIO id, not the example -
  a false positive, do not touch. Per `keeping-docs-in-sync.md` docs land with
  the change, so sweeping the sibling pages now would make them lie about code
  that still exists.
- **The DoD proof `! rg 'examples/perf|examples/gameplay|fps_exempt' Cargo.toml
  web/src/wiki` was unachievable here** - this task deletes neither directory,
  so `Cargo.toml` still carries `path = "examples/gameplay/scenario.rs"` after
  it. Narrowed to what this task can actually make green.
- **`fps_skipped` touches six files, not two.** The draft named `manifest.rs`
  and `html.rs`; `checks.rs`, `fixtures.rs`, `report.rs` and `run.rs` carry it
  too (26 hits total).

### Transitional behavior, in the open

Between this task and the sibling renames, `probe run gameplay --fps` stops
running frame-time passes for `scenario`, `playable` and `lifeline` (they get
the `gameplay` row, which is `frame_time: false`). `broadside` was already
exempt. This is a real, deliberate behavior change during transit, and its
end state is identical: all three are retired or move to `systems/`, which
carries no frame-time pass either. Flag if that is not acceptable and the
`gameplay` transitional row should keep fps instead.

### Ownership split for `fps_exempt`

- The `Cargo.toml` key `fps_exempt = ["broadside"]` (:33-35, with its comment
  block at :26-32) - deleted by whichever of `20260804-093910` /
  `20260804-094006` lands second.
- The parsers `parse_fps_exempt` / `load_fps_exempt` plus their unit tests and
  the `lib.rs:147` re-exports - deleted HERE.
- The checks.json field - RENAMED here to `fps_skipped`, kept as the reason
  string it already is.

Contract table and per-category probe behavior: see SPIKE.md of
`20260804-003244`, and the table in this task's `NOTES.md`.

## Close-out

**What / why.** Category run policy is now a two-boolean table in code
(`nova_probe::CATEGORY_POLICIES`) with a lookup (`category_policy`), and both
old mechanisms it replaces are gone: the `perf`-vs-everything window split and
the hand-listed `fps_exempt` parser. Three consumers read the table -
`fps_skip_reason` (does this run carry a frame-time pass, and if not, why),
`resolve_spec` (`--all` skips unprobed categories and records them; a bare
`probe run screenshots` errors), and the smoke gate
`every_category_has_a_probe_policy`. The contract's prose half landed in its
two homes: per-block comments in the root `Cargo.toml` and a five-row table
plus rewritten fps prose in `web/src/wiki/dev/development.md`.

`CHANGELOG.md` gained the matching `Internals & Tooling` lines - the doc
routing map makes CHANGELOG a required surface for any `nova_probe` change
(`keeping-docs-in-sync.md:61`), and the `checks.json` field rename
`fps_exempt -> fps_skipped` ships with no shim, so it is tagged
**(breaking)**. The other surface that line names, `dev/automation-harness.md`,
carries no category or `--fps` prose and needed no edit.

**Alternatives.** (1) Keeping `resolve_fps_window`'s per-category window as
Step 2 literally specified - rejected, the non-frame-time branch is
unreachable under the policy; see DECISION. (2) A third policy field carrying
the exclusion reason string - rejected, it reopens the two-vs-three-field
decision for text that is derivable from the category name; both reasons are
formatted at their consumer. (3) A `match` for the table - refined to a
`const` slice so the smoke gate can enumerate it; see DECISION.

**Difficulties / diagnosis.** The `excluded` list had one shape (per-EXAMPLE)
and now carries two axes. Recording an unprobed category once BY CATEGORY
rather than once per member keeps the aggregate report reading as the decision
that was actually made; `NOT_PROBED` stays for the orthogonal per-example axis
(`render_scale_shot`). The spec fixtures needed `stress`/`systems` members and
a NOT_PROBED example inside a PROBED category, since the old fixture's only
NOT_PROBED entry lived in `screenshots` - which is now unprobed wholesale, so
the two axes were no longer separable in the tests.

**Evidence.** `cargo test -p nova_probe`: 101 passed, 0 failed (69 lib + 28
bin + 3 integration + 1 doc). `cargo test --test examples_smoke
catalog_matches_disk` and `... every_category_has_a_probe_policy`: green,
display-free. `cargo check --workspace --all-targets`: clean (the four
`nova_gameplay` glob-ambiguity warnings are pre-existing on master).
`cargo fmt --all` applied. Both `cmd:` proofs verified in the worktree:
`rg fps_exempt crates/nova_probe web/src/wiki` exits 1, and `rg 'stress/'`
hits both `Cargo.toml` and `development.md`. Per repo policy the full suite is
CI's (`.github/workflows/ci.yaml`); no example was RUN, and none needed to be
- this task changes no example and no runtime game code.

**Reflection.** The scope boundary held exactly as planned: no example moved,
no directory was renamed, `catalog_matches_disk` never went red, and the smoke
consts are untouched for `093910`/`093934`/`094006`. The transitional rows
carry `# remove with <task-id>` so the follow-ups have a grep target. The
behavior change flagged in "Transitional behavior, in the open" landed as
described and widened slightly in the open: under the policy, `--fps` now
skips the frame-time pass for `sections/` and `ui/` too, not just
`gameplay/`. That is the point of the contract (only `stress/` claims frame
time), it is recorded in every report that asks for it, and its end state is
unchanged - but it is a live CLI behavior difference today, so it is called
out here rather than buried.

### Review round 1 (fixes)

Five findings taken, one recorded-not-changed. The MAJOR was a doc surface the
DoD proof could not see: `.claude/skills/probe/SKILL.md`, which root
`AGENTS.md:94` names as the probe manual, still documented the `fps_exempt`
list and the 60/240 non-`perf/` window this branch deletes. The proof scanned
`crates/nova_probe web/src/wiki` and ripgrep skips hidden directories, so
`.claude/` was invisible to it - the re-derivation now runs with `--hidden`.
The page is rewritten as category policy and its `--all` paragraph names both
exclusion axes.

The rest tightened what already shipped: `excluded` records a category as
`screenshots/` so the two axes are distinguishable in `index.json` without a
schema change (DECISION); `example_fps_skip_reason` goes back to fail-OPEN so
a catalog hiccup cannot silently suppress a capture; and
`resolve_all_and_explicit_excluded` gets its exact `assert_eq!` back - this
diff had weakened it to `.contains(...)` while introducing the dedupe branch
it was the only thing pinning. Deleting that guard now fails two tests
(verified RED by hand, then restored).

Re-verified whole: `cargo fmt --all --check` clean, `cargo check --workspace
--all-targets` clean, `cargo test -p nova_probe` 101 passed / 0 failed,
`cargo test --test examples_smoke` 7 passed / 0 failed in 111s, and both
`cmd:` proofs green (`rg fps_exempt crates/nova_probe web/src/wiki` exits 1;
`stress/` hits `Cargo.toml` twice and `development.md` four times).
