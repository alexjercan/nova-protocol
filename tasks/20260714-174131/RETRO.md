# Retro: Persist EnabledMods across restarts (native + wasm)

- TASK: 20260714-174131
- BRANCH: modding/mod-persist
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The persistence seam was already prepared by 174120 (`EnabledMods` + the
  `resource_changed` re-merge) and 174126 (the toggle), so this task was purely
  additive: two systems (load-at-Processing, save-on-change) + a cfg-split module.
- Factoring the native backend into pure `load_from(path)`/`save_to(path, ids)` made
  the file IO unit-testable (round-trip, missing, corrupt) without touching the real
  config dir - and the public `load/save` are thin wrappers over those.
- The in-game e2e was decisive without needing UI automation: a temp `XDG_CONFIG_HOME`,
  run 1 writes `[base]`, run 2 honors a pre-written `[base,demo]` and preserves demo. If
  load were broken, seed would reset to base-only and save would overwrite the file - it
  didn't, which proves BOTH load and save through the real systems.
- Changing `seed_enabled_mods` from "only if empty" to "union base ids" was a small
  robustness win: base (locked in the UI) stays on regardless of the restored set, and
  the seam is order-independent.
- I verified the uncompilable wasm backend against the actual web-sys 0.3 source myself
  before the review, so the one thing I couldn't run was still grounded.

## What went wrong

- I wrote "CI/Trunk builds it" for the wasm path in both the plan and a code comment
  without checking - the wasm build (`deploy-page.yaml`) is `workflow_dispatch` only, so
  automated PR/master CI never compiles it. The reviewer caught it. Root cause: assumed
  a wasm-capable project has wasm in its CI; it has a wasm DEPLOY, which is different.
  Fixed the comment to say static review is the real guard for that path.

## What to improve next time

- Before claiming a target/path is "covered by CI", read the workflow triggers. A
  `workflow_dispatch`/deploy job is not automated coverage. When a code path is only
  guarded by static review (an uncompiled cfg branch), say so in the comment so a future
  editor knows nothing will catch their mistake automatically.

## Action items

- [x] Native persistence proven in-game; wasm verified against web-sys source.
- [x] Comment corrected re: wasm CI coverage.
- [x] Lesson: `verify-ci-triggers-before-claiming-coverage`.
- [ ] (optional, out of scope) add the wasm target to automated CI so the wasm cfg
      branch is at least compiled on PRs - would be a small ci.yaml follow-up.
