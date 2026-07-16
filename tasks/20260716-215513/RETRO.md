# Retro: Consolidate demo + variety into ONE 'example' tutorial mod

- TASK: 20260716-215513
- BRANCH: example-mod
- REVIEW ROUNDS: 1 (APPROVE)

See TASK.md for what shipped and REVIEW.md for the findings; this is process only.

## What went well

- Front-loaded two parallel Explore agents (exact RON action/section shapes +
  a full untruncated demo/variety reference sweep) while reading the base
  scenario templates (`shakedown_run`, `menu_ambience`) directly. The content
  file was correct on the first lint once a stray dir was removed, and the
  sweep produced a working checklist rather than a guess.
- Distinguished REAL references to the removed mods (shipped catalog, count
  assertions, self:// file-existence tests) from generic test doubles with
  invented data (nova_menu "Alice", nova_mod_format's fabricated `reel` mod).
  Repointed the former, left the latter - avoided both misses and needless
  churn, and documented the judgment call in REVIEW.md R1.3.
- Independent out-of-context review agent gave genuine fresh eyes despite the
  implementer and reviewer sharing one session; it re-derived the load-bearing
  claims (RON shapes, id resolution, count math) instead of trusting the diff.

## What went wrong

- Renamed the menu-backdrop well id `example_menu_planetoid` -> `menu_planetoid`
  (so the menu's cinematic camera frames on it) but left the AI orbiter's
  `orbit: Some("example_menu_planetoid")` pointing at the old id. `content_lint`
  passed anyway - it validates spawn/prototype/filter refs but NOT AI
  orbit/patrol targets - so the break was silent. Caught only in a manual
  self-review pass. Root cause: renamed an id and trusted the linter to catch
  every dangling reference instead of grepping the file for the old id.
- `git mv`-ing the textures out of `assets/mods/variety/` then `git rm`-ing the
  folder left an empty `variety/textures/` directory on disk, which crashed
  `content_lint`'s repo bundle walk ("no *.bundle.ron at its root"). Root cause:
  git does not track empty dirs, so removing the files did not remove the
  emptied parent; the lint walks the filesystem, not the index.

## What to improve next time

- After renaming any entity/asset id, grep the whole content file for the OLD
  id before relying on the linter - linters cover some ref classes, not all.
- When relocating a mod/dir with `git mv` + `git rm`, explicitly `rm -rf` the
  old dir (or verify it is gone) - emptied parents linger and any
  filesystem-walking tool trips on them.

## Action items

- [x] Ledger: added `rename-id-sweep-in-file` and `git-mv-leaves-empty-parent`;
  bumped `keep-docs-in-sync-with-code` (x3, already enforced in AGENTS.md).
- No follow-up code tasks: the diff is complete and green.
