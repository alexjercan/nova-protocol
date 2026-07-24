# Retro: The Ledger campaign - collapsible header + hidden-chapter replay

- TASK: 20260724-220842
- BRANCH: feature/ledger-campaign
- REVIEW ROUNDS: 1 (out-of-context APPROVE, one MINOR doc fix)

Process notes only; what/why/evidence is in TASK.md close-out.

## What went well

- The campaign machinery from umbrella 20260724-193016 made this a pure
  content-authoring task: a Campaign RON file + a bundle-manifest line, no engine
  code. The prior investment paid off exactly as intended - a second campaign was
  cheap to add.
- Referencing the chapters by their scenario IDs (not filenames) and pinning the
  membership+order+hidden-flags in a test that reads the committed files means a
  future chapter rename or reorder fails loudly.
- Re-hiding ch5 was invited by the content itself: its own comment said "RE-HIDE
  before release - reached only by winning the ch4 fight". The campaign header is
  the permanent mechanism the temporary-visible hack was standing in for, so the
  cleanup and the feature were the same change.

## What went wrong

- The doc-surface sweep missed the webmod's OWN README. I grepped `web/src` +
  `crates` for the stale chapter names and got 0 hits, and wrote "referenced
  nowhere outside the webmod" - but the mod ships its own `README.md` INSIDE
  `webmods/the-ledger/`, which still told players to start "The Ledger 1: Dead
  Weight" flat from the picker. Review caught it (MINOR). Root cause: the sweep's
  scope excluded the very directory whose content I was changing.

## What to improve next time

- When changing a mod's content (names, structure, visibility), sweep the mod's
  OWN directory too - its `README.md` and any per-mod docs are a first-class doc
  surface that ships to the player. Grep the changed directory itself, not only
  the central `web/`+`crates` doc tree.

## Action items

- [x] REVIEW.md R1.1 fixed: webmod README updated; whole `webmods/the-ledger/`
  re-swept clean.
- [x] ledger: added `doc-sweep-includes-the-changed-mods-own-readme`.
- No follow-up code tasks. Re-publishing The Ledger to the portal
  (`scripts/gen-portal.py`) is a release-time action, tracked by the version bump
  to 1.13.0.
