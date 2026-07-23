# Retro: ch5 gravity round 2 - thrusterless base clear of tiny wells

- TASK: 20260723-223954
- BRANCH: feature/ch5-gravity-r2
- REVIEW ROUNDS: 1 (APPROVE, out-of-context; one NIT, no change)

See TASK.md Outcome for what/why; this is process only.

## What went well

- The `bundle-version-string-pin-bites-on-bump` lesson from last cycle paid off
  immediately: I ran `grep -rn '"1.11.0"' crates/` BEFORE the 1.11->1.12 bump and
  fixed the rig pin in the same edit, so the version pin did not break the test
  run this time. A lesson turned into a habit that prevented its own recurrence.
- Computed the base/raider/well geometry (distances, SOI gaps) in the same Python
  pass that moved the objects, and the out-of-context reviewer independently
  reproduced every number - no "looks about right" placement.
- Recognised the real shape of the problem across three iterations: the AI cannot
  fly a gravity well, so content-side tricks (RCS, leash) were always fragile.
  Reverting to the simple robust hold (thrusterless base + keep everyone clear of
  gravity) and filing the actual fix as a backlog task is the honest resolution,
  not a fourth epicycle.

## What went wrong

- Nothing broke, but the review's NIT exposed an imprecise mental model I had
  carried since the first gravity round: I kept saying "only PILOTED ships feel
  gravity" as if that excluded AI. It does not - "piloted" means player OR AI, so
  AI ships DO feel gravity (`gravity.rs:226`). My fix was still correct (and the
  imprecision actually makes the fix MORE load-bearing), but I had the reason
  slightly wrong in the planning prose. Root cause: I read the gravity-affected
  insert for the player path and generalised the phrase without reading the AI
  insert a few lines down - the same "read the neighbouring branch" gap as last
  cycle's leash-override miss.

## What to improve next time

- When quoting an engine rule as a load-bearing rationale, read ALL the
  branches that establish it (player AND AI gravity opt-in, not just one), not
  the first one that confirms the phrasing. This is the second time this run a
  one-branch read produced a slightly-off claim (leash override last cycle,
  gravity opt-in this cycle) - it is the recurring edge.

## Action items

- [x] R2.1 accepted (no code change; shipped RON comment is accurate).
- [x] Lessons ledger: renamed/sharpened the override-branch lesson to
  `read-all-branches-of-a-load-bearing-engine-rule` (x2) - the leash override and
  the AI gravity opt-in both bit the same one-branch-read way.
- Backlog 20260723-224003 (gravity-aware AI) carries the real fix so the wells
  can grow back.
