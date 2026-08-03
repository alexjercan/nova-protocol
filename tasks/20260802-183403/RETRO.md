# Retro: Migrate nova_debug, nova_probe, and the example fleet onto nova_autopilot

- TASK: 20260802-183403
- BRANCH: refactor/autopilot-migration
- REVIEW ROUNDS: 1

## What went well

- The plan made `examples_smoke` a DoD proof, not an optional check. It is the
  only proof that RUNS the fleet, and it is the one that caught both real
  breaks. Keep pricing a slow run-the-code proof into rename plans.
- The name-resolution shadow became a permanent guard test
  (`examples_name_drivers_through_the_nova_harness`) instead of a one-off grep,
  and the guard was verified failing-for-the-right-reason first.
- The atomic rename with no compatibility alias held. A half-renamed tree that
  still boots is exactly what the shadow produced locally; an alias would have
  made that state permanent instead of loud.

## What went wrong

- The DoD's absence grep could never have gone green as planned: its bare
  `debug::harness` alternative also matched the `nova_debug::harness::` paths
  the same task's Notes required to SURVIVE. It looked sound at plan time
  because the shorter pattern reads as the more thorough one; the collision with
  the task's own retention rule is only visible once the proof is actually run.
  Corrected mid-work, then edited again at verification because the new guard
  test's own doc prose spelled `BCS_AUTOPILOT`.
- The plan modelled the migration as STRING substitution, so two breaks sat
  outside every `cmd:` proof: a `bevy_common_systems::completion` resource
  reach-in in four self-ending examples, and ten examples naming a bare
  `AutopilotPlugin` that resolved to the old glob prelude's inert twin. Both
  compile clean either way; nine of the ten would have booted silently
  autopilot-less.
- Recovering from the guard's fail-first check, a
  `git checkout examples/gameplay/playable.rs` reverted that file's real
  migration edits along with the temporary one. Caught immediately, reapplied.

## What to improve next time

- Breadth: the diff outgrew the plan by ~14 example call sites plus a guard
  test. That is scope FOUND LATE, not a missed split - the activation rename is
  inseparable, so none of the extra work was independently landable. The fix is
  earlier discovery, not a smaller task.
- Churn: review was a clean round-1 APPROVE, so there is no rework to
  attribute. The plan-time question that would have paid off anyway is `plan`'s
  red-on-base proof check: running the absence grep against the base branch
  would have exposed both that it matched surviving paths and that it could not
  see either break.
- Migrating off a glob-exporting prelude: enumerate the names the OLD prelude
  exports that the NEW one does not. Those sites are precisely where deleting
  the old wiring leaves compiling, silently-wrong code. A grep for the new name
  finds nothing there; only a grep for the ABSENCE of qualification does.
- Never `git checkout <file>` on a file that already carries uncommitted work.
  Revert the temporary edit in place, or stage before experimenting.
- Context: no context-pressure event was measured or recorded for this task -
  no compaction warning, no checkpoint handoff. Nothing to split or defer on
  that basis.

## Action items

- Six non-blocking review findings (R1.1-R1.6: four `broken_intra_doc_links`
  warnings, one `private_intra_doc_links` warning, an empty-reel body-freeze
  guard, a full-world-scan comment, a coincidental-constant assert, and a
  `DRIVERS` list comment) are seeded as `20260803-114158`.
- Stale `BCS_*` doc prose across `AGENTS.md`, `CHANGELOG.md`,
  `web/src/wiki/dev/*`, `.claude/skills/probe/SKILL.md`, CI and `.gitignore`
  stays with `20260802-183406`, whose repo-wide grep already covers it.
