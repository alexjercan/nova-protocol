# Retro: Editor skybox may miss its Cube view (FALSIFIED)

- TASK: 20260717-133332
- BRANCH: work/editor-skybox-cube-view
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Checked for a falsifier before writing any fix. The task's first step was
  "reproduce"; grepping the reformatted test comment surfaced
  `prepare_cubemap_view`, which turned out to already cover the editor. Ended the
  cycle as a falsification with a pin instead of shipping a needless code change
  (the `diagnostic-first` / verify-before-fixing discipline paying off).
- Left the cycle better than a plain "wontfix": a regression test pinning
  `prepare_cubemap_view`'s guarantee + a comment at the editor insert site, so
  the same false alarm cannot be re-filed and the coverage cannot silently
  regress.

## What went wrong

- The task existed only because review R1 of 20260717-111558 (same author) filed
  a suspected editor bug from a PARTIAL view of the system. It reasoned from the
  bcs observer + `apply_pending_skybox_swaps` and concluded the editor's direct
  insert "may miss its Cube view" - without grepping for other systems that touch
  the same `game_assets.cubemap` handle. `prepare_cubemap_view` (a startup system
  whose whole purpose is exactly this) was one grep away. Root cause: theorized a
  failure mode from the two mechanisms already in hand instead of asking "what
  else writes this handle before the consumer runs?"
- Net: a full plan/work/review/retro cycle spent to confirm a non-bug. Cheaper
  than shipping a wrong fix, but the filing itself was avoidable with one grep at
  review time.

## What to improve next time

- Before filing a "possible pre-existing bug" from a review, do the same
  existence check a real fix would need: grep for a COMPENSATING mechanism
  (another system/observer/startup step) that touches the same state before the
  suspect consumer. If found, there is no bug; if genuinely unsure, file it but
  say precisely what was NOT checked (this task did mark itself UNCONFIRMED,
  which is what kept the falsification honest).

## Action items

- [x] Bumped `verify-engine-guarantees-in-source` in the ledger with the
  "grep for a compensating system before theorizing a missing-write bug" variant.
- No follow-up code work: the editor is correct as-is.
