# Retro: Rust Tally mount rolls

- TASK: 20260717-151214
- BRANCH: fix/rust-tally-mounts (landed 1c116363)
- REVIEW ROUNDS: 1 (APPROVE; 3 MINOR + 1 NIT)

## What went well

- The prior cycle's GLB-derived mount-axis facts transferred directly:
  the rotation derivation was written once in the plan and survived the
  reviewer's independent re-derivation, including the highest-risk
  question (the turret aim chain is mount-local, so rolled mounts aim
  correctly) which was VERIFIED, not assumed.
- The id sweep found the port/starboard swap nobody reported - reading
  the thing you are editing pays.

## What went wrong

- R1.1: NOTES claimed the launch axis was local -Z when the bay kicks
  along +Y - the exact fact the PREVIOUS cycle's review had pinned, one
  task earlier. Root cause: I cited the prior analysis instead of
  re-reading it; a citation is not a re-check
  (recheck-referenced-task-freshness, claims edition).
- R1.3: a mechanical fix that changes BEHAVIOR (engagement arcs) was
  documented as pure geometry until review named it. When a spatial fix
  moves an actuator's frame, ask what the actuator now REACHES.

## What to improve next time

- When citing a sibling task's technical finding, re-open that REVIEW.md
  and quote it rather than paraphrasing from memory.

## Action items

- [x] Follow-up 20260717-162121 (mount-base adjacency lint) - the
  mechanical guard for this whole class, born on the branch.
- [x] docs/LESSONS.md: new lesson cited-finding-reread-not-recalled (x1).
