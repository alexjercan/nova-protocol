# Review: remove the scatter_density graphics-preset lever

- TASK: 20260718-004834
- BRANCH: remove-scatter-density

## Round 1

- VERDICT: APPROVE

Diff reviewed against master (settings.rs, actions.rs, world.rs, two TASK.md,
NOTES.md). The change delivers the Goal: `scatter_density` and `scaled_count`
are gone, `for_quality` gates only `particles`, `ScatterObjects` spawns the
authored count on every tier, and the dead `graphics_budget` plumbing is removed
from `NovaEventWorld`.

Load-bearing claims independently re-verified (shared implementer/reviewer
session):

- **No stale API callers.** Swept the whole tree: the only surviving
  `scatter_density`/`scaled_count`/`graphics_budget()` references are in historical
  `tasks/` docs (the baseline report and task files), which are correct to leave.
- **No persistence break.** `GraphicsBudget` is not serde-persisted anywhere;
  `nova_menu` only persists the `GraphicsQuality` enum, so removing a budget field
  needs no save migration.
- **No unused-import/dead-code fallout.** `GraphicsBudget` is still read for
  `.particles` by the torpedo/turret render systems; `world.rs` referenced it only
  via a prelude glob, so no dangling `use`. `cargo check -p nova_gameplay
  -p nova_scenario` is clean with no warnings from our code.
- **Regression test has real guard value.** `scatter_action_ignores_graphics_budget`
  asserts the full authored count (20) spawns with a Low budget carried in;
  re-introducing thinning would make it spawn 10 and fail. Would-it-fail-without-the-
  fix: yes.
- **`world_to_state_system` correctly kept.** It is a required `EventWorld` trait
  method (no default in the upstream trait source), so the now-empty override is
  necessary, not dead code; the comment explains why.
- **Factual claim re-derived.** `scaled_count(20)@0.5 = 10`, `scaled_count(24)@0.5
  = 12`, and `asteroid_field`/`broadside` do carry `ScatterObjects((count: 20/24))`.
  The commit/NOTES claim that removal restores full counts on Low in those scenes
  is correct.

Tests run: `cargo test -p nova_gameplay --lib settings::` (5 passed);
`cargo test -p nova_scenario --lib scatter --features serde` (7 passed, incl. the
new regression). Full workspace suite left to CI (`cargo test --workspace
--features debug`) per the project's local-test policy - note that
`-p nova_scenario --lib` without `--features serde` fails to compile the
`loader.rs` round-trip tests; that is a pre-existing feature-flag artifact, not
introduced here.

- [x] R1.1 (NIT) crates/nova_gameplay/src/settings.rs:104 - the struct doc still
  says "nothing is silently thinned when the preset is absent"; with scatter
  thinning gone the word "thinned" is slightly vestigial (the only fallback now is
  particles-on). Reword to "renders everything" for precision. Non-blocking.
  - Response: Addressed - reworded to "particles render normally when the preset
    is absent".
- [x] R1.2 (NIT) crates/nova_scenario/src/actions.rs (scatter_action_ignores_graphics_budget)
  - the inserted `low_budget` no longer influences the outcome (the carry is a
  no-op), so the test would pass identically without it. Kept deliberately as
  intent documentation ("even the cheapest tier does not thin"); acceptable, but
  worth a one-line comment if it ever confuses. Non-blocking.
  - Response: Addressed - added a comment on the `world_to_state_system` call
    noting it is a no-op now, kept to prove a present Low budget does not thin.

Both NITs addressed in the same round; verdict stands at APPROVE.
