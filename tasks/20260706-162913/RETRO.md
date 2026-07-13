# Retro: Torpedo module split + config-driven blast

- TASK: 20260706-162913
- BRANCH: refactor/torpedo-module
- PR: #34 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, one benign NIT)

See `tasks/20260706-162913/TASK.md`. A large mechanical refactor that went smoothly
because it was sequenced and executed to minimize risk.

## What went well

- Split the risky work into two isolated parts and verified between them: first the
  config-driven blast params (a real logic change - new component, new query), tested
  green; *then* the pure module move (no logic change). Keeping the behavior change
  and the code move in separate, separately-verified steps meant that when the split
  compiled, I already knew the blast change was sound - one variable at a time.
- Executed the move as a mechanical line-range extraction (`sed`) rather than
  retyping ~470 lines by hand. A verbatim move can't introduce logic bugs, and git
  recorded it as a clean rename (`torpedo_section.rs -> torpedo_section/mod.rs`),
  keeping the diff reviewable.
- Chose contiguous cut points. Reading the outline first showed the in-flight systems
  (497-741) and render systems (743-964) were each already contiguous, so the whole
  split was two range extractions, not a scatter-gather across the file. Verifying the
  exact boundary lines before cutting avoided off-by-one breakage.
- Kept the module name (`torpedo_section`) and moved the file to `mod.rs`, so the
  public path and `TorpedoSectionPlugin` did not change - zero churn for callers.
- `use super::*` in the submodules plus `pub(super)` on the moved systems (and
  `use projectile::*` in mod.rs) let the plugin and the still-in-`mod.rs` tests reach
  everything, including `mod.rs`'s private components - it compiled on the first try.

## What went wrong

- Nothing notable. The regression safety net (29 tests + the two example smoke runs)
  made "behavior preserved" a checkable claim rather than a hope, and it held.

## What to improve next time

- For any large file split, this sequence is worth repeating deliberately: (1) land
  any behavior change first and test it, (2) confirm contiguous cut boundaries by
  reading the exact lines, (3) move by line-range extraction, not retyping, (4) lean
  on `use super::*` + `pub(super)` to avoid a visibility rewrite, (5) prove
  behavior-preserved with the existing tests/smoke before and after.

## Action items

- [ ] NIT R1.1 left as-is: `projectile.rs` is cohesive at ~250 lines.
- [ ] `blast_radius` / `blast_damage` are now tunable per bay - the torpedo range
      (`06`) can vary them if a future task wants to tune blast feel.
