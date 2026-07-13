# Docs overhaul: concise AGENTS.md/CHANGELOG, restructured docs/

Start of the v0.5.1 cycle. Goal: make all documentation concise, simple, and
easy to read for both humans and LLMs.

## What changed

- `AGENTS.md` rewritten at ~60% of its size; stale facts fixed (missing
  `nova_menu` crate, `MainMenu` state, examples list, "currently 0.3.1").
  The two long lesson-bullets moved to the LESSONS.md ledger where they belong.
- `CHANGELOG.md`: one concise line per entry, playtest rounds merged,
  superseded-within-release items dropped, per-entry `@alexjercan` attribution
  removed (single-author noise); convention updated in development.md.
- `docs/` restructured: 36 dated records from the root moved into `retros/`
  (fix/feature records), `plans/` (v0.4.0 plan), `spikes/` (sdlc suggestions);
  all dates normalized to `YYYYMMDD-description.md`; 97 files of references
  updated by script. READMEs added: root index plus one per folder with a
  generated per-file index.
- Reference docs rewritten concise and fact-checked against the code:
  `architecture.md` (invented system-set list replaced with the real chain),
  `sections.md` (integrity core moved to bcs, typed-damage path), and
  `scenario-system.md` (8 events not 5, all 14 actions, config field drift).
- `LESSONS.md` condensed 577 -> ~200 lines: every slug and count kept, variants
  folded into the one sentence, at most a couple of retro ids each. Corrected
  `worktree-shares-main-target`, which recommended sharing `CARGO_TARGET_DIR`
  with the main checkout - the thing 20260709-131502 proved unsafe.

- Follow-up the same day: `docs/reviews/` removed; the two PR-level reviews
  were appended to their primary task's `tasks/<id>/REVIEW.md`, so ALL reviews
  now live next to their task.
- Second follow-up: retros and spikes moved the same way - `tasks/<id>/RETRO.md`
  and `tasks/<id>/SPIKE.md`, with date-only design records as
  `tasks/<id>/NOTES.md`. 48 pruned task folders were recreated as CLOSED
  archive-stub `TASK.md`s (a folder without `TASK.md` breaks `tatr ls`,
  verified empirically). Only `docs/plans/`, the LESSONS.md ledger, and a few
  task-less records stay under `docs/`.

## Difficulties

- The /compound skill hardcodes `docs/retros/LESSONS.md` and lives in a
  read-only nix-store symlink, so the ledger could not move to `docs/`.
- Old references used three shapes (`docs/<file>`, `docs/retros/<file>`, bare
  basename); the rewrite script had to handle prefixed forms before bare ones.

## Next time

- Keep ledger entries one or two lines at append time; the 577-line ledger grew
  one paragraph-variant at a time.
- Generated README indexes need a line added per new file; the folder READMEs
  say so.
