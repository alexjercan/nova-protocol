# Retro: Relocate input-prompt key glyphs to assets/ (Alt only) + credits

- TASK: 20260728-233707
- DATE: 2026-07-29
- OUTCOME: CLOSED, APPROVE (1 review round), landed as one squash commit
- PART OF: EPIC 20260728-175719 (UI rework)

## What went well

- The plan was concrete and PoC-grounded, so work was execution, not
  discovery: `git mv` the Alt tree, `git rm` Dark/White + NOTICE, absorb the
  provenance into credits/, repoint two consumers. No forks surfaced mid-build.
- The repo-wide sweep found exactly one live hit (`web/webpack.config.js:324`),
  confirming the [[sweep-content-repo-wide-not-just-assets]] discipline: the
  grep was run over the WHOLE repo (excluding only tasks/dated-history), not
  pre-narrowed to assets/ or web/.
- [[render-output-eyeball]] was honored properly: the file:// review copy was
  rendered with a headless-chromium screenshot and the crop confirmed real key
  glyphs (Z/G/O/X/R/Space/Tab) on the dock and NOVA OS button - not broken-image
  placeholders and not a green exit code standing in for a look.
- DoD 5 (web build) was NOT skipped despite npm being absent from the base
  shell: the flake ships `nodejs_22`, so `nix develop --command` ran the real
  `npm ci && npm run build` and the dist tree was verified (99 Alt, no
  Dark/White). Skipping would have been the easy, wrong call.

## What went wrong / friction

- npm/node are not on the default PATH in this environment; the first
  `npm run build` died with exit 127. Recovered by discovering the flake's
  `nodejs_22` devshell and running the build through `nix develop`. Cost one
  detour but no rework.
- The initial `tatr update --status IN_PROGRESS` appeared to no-op (STATUS
  stayed OPEN in TASK.md); set it via direct file edit instead. Minor, but a
  reminder to verify the artifact after a tatr mutation rather than trusting
  the command's stdout.

## Lessons for next time

- `web-build-runs-via-nix-develop`: when a web/DoD needs npm/node but the
  base shell lacks them, check `flake.nix` for `nodejs_*` and run the build
  through `nix develop --command bash -c '...'` before declaring the check
  un-runnable. The build IS runnable locally here.
- Coverage note for future glyph work: every glyph img in the HUD PoC carries
  `class="ki"` (dock chips AND the NOVA OS navbtn), so a single `img.ki`
  selector is the complete set - no separate `.navbtn img` pass needed.

## Follow-ups

- None. This unblocks the HUD icon dock (20260728-175742), the first real
  consumer, which owns the key-name -> filename mapping table. Backlog
  20260728-214929 keeps the remaining adoption surfaces (web key-UI, NOVA OS
  help lines, editor key chips, gamepad).
