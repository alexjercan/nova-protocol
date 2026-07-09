# Retro: Minimal faction/relation model (hostile/neutral/own)

- TASK: 20260708-203708
- BRANCH: feature/faction-relations (squash-merged as 05fa998)
- REVIEW ROUNDS: 1 (APPROVE, 1 MINOR + 1 NIT, both addressed)

What shipped is in the task's Resolution and
docs/2026-07-09-faction-relations.md. First task of the AI combat arc
(docs/spikes/20260709-225508-ai-combat-behaviors.md); a clean cycle.

## What went well

- **Component requires beat spawn wiring, discovered by reading first.** The
  plan originally said "insert Allegiance in the scenario spawner match".
  Reading the marker definitions before implementing surfaced
  `#[require(Allegiance = ...)]` instead: less code, and every existing test
  world that spawns a marker joined the relation model for free - which is
  exactly why the pre-existing targeting tests passed unchanged after the
  hostility swap. Updating the plan Step to reality (per the work skill)
  kept TASK.md honest.
- **A semantic edge case was decided on paper, not left to the match arm.**
  `Neutral vs Neutral = Neutral`, not `Own` - argued in the doc comment and
  pinned by the matrix test. Deciding it explicitly cost one sentence;
  inheriting it silently from a `a == b` pattern would have made two
  asteroids allies.
- **Plan-level scope discipline held.** AI target selection over relations
  was tempting to fold in (the query swap is right there) but stayed in
  20260709-225727 as planned; the diff stayed reviewable (+426) and the
  review took one round.

## What went wrong

- **The review pre-filled Response hashes before the fix commit existed.**
  REVIEW.md was written with `fixed in <hash>` placeholders, then the real
  commit forced a sed-and-recommit. Root cause: writing the review file and
  addressing the findings in one breath, out of order. Harmless here, but
  the file briefly claimed a fix in a nonexistent commit.
- **Task-ID collision in `tatr new` (spike phase, second occurrence of a
  tool-shape issue).** Nine creates in one shell invocation landed in the
  same second and clobbered into one task; recreated with `sleep 1`
  between. Root cause: tatr IDs have seconds granularity and `tatr new`
  does not detect the collision - it silently reuses the directory.

## What to improve next time

- Address review findings first, commit, then write their Response lines
  with the real hash - or write `Response: fixed (hash below)` and fill it
  in one pass after committing.
- When creating several tatr tasks in one go, always `sleep 1` between
  `tatr new` calls (or create them in separate invocations).

## Action items

- [ ] Upstream (tatr lives outside this repo, in the nix profile): `tatr
  new` should fail or auto-increment when the ID directory already exists
  instead of silently reusing it. Until then, the `sleep 1` rule above is
  the guard; if the collision bites a second time, propose the rule for
  AGENTS.md.
