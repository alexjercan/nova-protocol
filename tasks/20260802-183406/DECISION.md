# Decision: the dead BCS names get one searchable home, and the absence proof only asserts BCS

- DATE: 20260803-141101
- STATUS: ACCEPTED
- TASK: 20260802-183406
- TAGS: decision, autopilot, tooling, docs

## Context

The planned absence proof was
`! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|debug::harness"`
excluding `tasks/**` and `web/src/news/**`. Two clauses could never go green,
for reasons that are correct behavior rather than leftover work.

`debug::harness` was written to catch `bevy_common_systems::debug::harness`,
but it is also a substring of Nova's own `nova_debug::harness` - 35+ hits
across `examples/`, `tests/` and `crates/nova_debug/`. That module is the
Nova-flavored adapter over `nova_autopilot` that `20260802-183403`
deliberately landed, and
`tests/examples_smoke.rs::examples_name_drivers_through_the_nova_harness`
*requires* every example to name it explicitly.

`CHANGELOG.md`'s "Examples are a testable curriculum" entry documents a
shipped release in which examples really did self-drive under `BCS_AUTOPILOT`.
The new `## [Unreleased]` **(breaking)** entry also has to spell the dead
names - that entry IS what someone with a broken run script greps for.
Separately,
`web/src/wiki/dev/automation-harness.md` carried its own rename note spelling
`BCS_HARNESS_DEADLINE`, so the dead names had two homes.

## Decision

Narrow the absence proof to the fully-qualified BCS paths
(`bevy_common_systems::debug::harness`, `bcs::debug::harness`) and add
`CHANGELOG.md` to the exclusion list alongside `tasks/**` and
`web/src/news/**`. Sweep with `--hidden --glob '!.git/**'` so dot-directory
surfaces (`.claude/`, `.github/`, `.gitignore`) are actually covered.

Give the old spellings exactly one home: the CHANGELOG's breaking entry. The
wiki page keeps the warning that an old script arms nothing and silently does
a plain play-through, but drops the dead spellings and points at the CHANGELOG
for them.

## Alternatives considered

- **Rename `nova_debug::harness`** so the original `debug::harness` clause goes
  green. Rejected: not this task, and it breaks the examples-smoke contract
  that `20260802-183403` just established.
- **Rewrite that historical CHANGELOG entry.** Rejected: it would
  falsify a shipped release, and the `## [Unreleased]` entry must spell the
  dead names regardless.
- **Keep the rename note in the wiki too.** Rejected: two searchable homes
  drift apart, and the wiki page then reads as a migration diary rather than a
  current contract.

## Consequences

- The absence proof no longer asserts "no `debug::harness` anywhere"; it
  asserts "no BCS harness surface". That is the claim the Story actually
  makes. Nova's own adapter is proved separately by the examples-smoke test.
- `CHANGELOG.md`'s content is still proved, by the separate
  `rg -n "nova_autopilot" CHANGELOG.md` criterion.
- Anyone grepping for `BCS_HARNESS_DEADLINE` after a broken run lands in the
  CHANGELOG, not the wiki. If the CHANGELOG entry is ever pruned during a
  release cut, that breadcrumb goes with it.
- The `--hidden` sweep is what caught the stale `.claude/skills/probe/SKILL.md`
  deadline prose in review round 1; a default `rg` had reported green.
