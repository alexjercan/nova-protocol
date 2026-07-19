# Retro: harness completion protocol (S1, bcs v0.19.3)

- TASK: 20260720-000609
- BRANCH: feature/harness-completion-adoption (landed 59bd8419); bcs
  feature/harness-completion (shipped v0.19.3, master 3f6f7c8)
- REVIEW ROUNDS: 1 (APPROVE; 3 NITs/records)

## What went well

- The spike's architecture survived contact intact: register/done/
  deadline landed exactly as designed, and the headline e2e was the
  FIELD case itself (scenario + default windows -> full 900-frame
  capture) rather than a synthetic stand-in - the strongest possible
  close of the loop from user bug report to protocol fix.
- Ungating the completion module at the bcs crate root was caught at
  DESIGN time by asking "who compiles this featureless?" (wasm perf_web)
  - a one-line placement decision that would have been a confusing
  build break two hours later.
- The upstream rhythm held for the second release in two days: branch,
  convert, both-config tests, version+CHANGELOG, user-authorized
  push+tag, pin bump, retest against the PUBLISHED tag (the lock's
  source line, not faith).

## What went wrong

- The dev-loop cost a full hour of build cycles to discover what the
  ledger now records: [patch] rejects a version-bumped patch of a
  git-tag dep, the unpatched pin must stay resolvable, and a missed
  manifest (nova has FIVE bcs dependents, one with a features clause)
  splits the graph into two crate instances with non-matching traits.
  Each failure surfaced serially through cold rebuilds.
- Two small self-inflicted cuts: `sprout new` ran from the bcs worktree
  and created the adoption branch in the WRONG repo (sprout derives the
  project from cwd); and the [patch]-section removal left a trailing
  blank line that rode into the squash.

## What to improve next time

- Before the first upstream dev-loop build: enumerate ALL dependent
  manifests (grep the workspace for the git URL) and re-point them in
  ONE edit - the serial-discovery tax was the avoidable part.
- `sprout new` is cwd-sensitive: run it from the target repo's main
  checkout, always.

## Action items

- [x] Ledger: upstream-dev-via-patch-not-premature-push filed and
      sharpened in-cycle.
- [ ] S2 (20260720-000616) next: always-split fps pass + scene looping +
      reload lines - turns e2e B's idle tail into honest activity.
