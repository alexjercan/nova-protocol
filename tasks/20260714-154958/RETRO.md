# Retro: Assets + licenses shipping hygiene (20260714-154958)

Closed 2026-07-15. Verdict APPROVE. Docs/file-move chore + a folded-in docs
refresh; no code, no tests.

## What shipped

- `assets/blender/*.blend` (2.7M) relocated to top-level `art/blender/` so
  neither shipped build carries it; `art/README.md` documents the split;
  `docs/architecture.md` updated.
- `credits/CREDITS.md` expanded into an explicit audit (all assets original bar
  the Bevy icon); dependency-license aggregation split into follow-up
  20260715-110417.
- `assets/sounds/README.md` required-files table refreshed from 5 to all 16
  cues (positional/non-positional split).

## What went well

- Reading `index.html` AND `release.yaml` up front revealed both builds bundle
  the whole `assets/` tree, so a single relocation fixed web + native with zero
  build-config change - much less fragile than the task's alternative
  (excluding blender in two separate build paths).
- Asking about asset provenance before writing credits avoided either inventing
  attribution or silently asserting "original" - the one genuinely
  owner-only, legally-meaningful decision in the task.
- The task's premise was partly stale ("three credits copies, top-level
  CREDITS.md"): investigation showed credits were already single-source and the
  top-level copy gone. Verifying the current state beat trusting the brief.

## Difficulties

- A sprout worktree is a fresh checkout, so the gitignored `.blend1` autosave
  backups only existed in the MAIN checkout, not the worktree. After the squash
  merge removed the tracked `.blend` from `assets/`, the main checkout still had
  `assets/blender/` on disk holding just those untracked backups (which a local
  Trunk build would still copy). Cleaned with `rm -rf assets/blender` in the
  main checkout post-landing.

## Lessons for next time

- **A relocation only moves TRACKED files; gitignored siblings stay behind in
  the main checkout.** When emptying a directory to stop it shipping, after
  landing also remove the directory's untracked/ignored leftovers from the main
  checkout (a copy-dir build ships on-disk files regardless of git status). CI
  checkouts are clean, but the local build and repo tidiness are not.
- **Verify a stale task brief against the live tree before planning.** Half of
  this task's "consolidate three credits copies" was already done; planning off
  the brief would have chased a non-problem.

## Left out / follow-ups

- 20260715-110417: aggregate Rust dependency licenses (Bevy/avian MIT + Apache)
  into shipped credits via cargo-about - a real binary-distribution obligation
  beyond the asset-only audit.
- A full `trunk build` was not run to eyeball `dist/` (cold wasm build is
  expensive); the fix is verified by construction (copy-dir input is
  `.blend`-free).
