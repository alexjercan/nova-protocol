# Review 24: Web and script remainder

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor findings; review-20 findings confirmed

## Findings

### MINOR - `scripts/gen-scenario-thumbnails.py:7` and `gen-web-screenshots.py:55` reference deleted `gen-placeholder-sounds.py`

Both generator comments cite a sibling script that no longer exists. This is stale developer guidance only.

### MINOR - `web/tests/comic-renderer.test.ts:416-430` prints success before its final assertion block

The process still exits with failure if the last block fails, but a human can see "all assertions passed" before all assertions have run.

### MINOR - `web/src/docs.ts`, `news.ts`, and `index.ts` have no direct behavior tests

These shipped entry scripts own documentation search/navigation, news scroll state, and download-button wiring. The web test command does not execute their DOM behavior. This generalizes review 20's untested `downloads.ts` contract; `docs.ts` is the material gap because it owns substantial player-facing navigation logic.

### MINOR - `build/windows/installer/Package.wxs:6` retains a stale, disconnected installer version

The WiX package hardcodes `0.3.0` while the workspace is `0.14.0`. Release workflows permanently disable and never build the installer, shipping a zip instead. This is obsolete release tooling today, but enabling it would produce an incorrectly versioned MSI.

## Reverification

Full source review confirmed all review-20 findings: Pages toolchain divergence, hardcoded webpack development mode, missing lesson-media CI gate, orphaned WebGPU tests, untested download selector, and absent SFX check mode.

## Coverage

Fully read all 51 Python/shell files under `scripts/`, including the complete `nova_illustration` package and tests. Fully read all non-generated web TypeScript/JavaScript helpers and all web tests, comic episode code and art/test Python, remaining platform build text/code, and wasm lint configuration. JSON recipe data and binary art were excluded, but their parsers and schema validation were reviewed.

Combined with review 20, the previously named web/script/platform code gaps are closed.

Not checked: no generator, npm, webpack, TypeScript, workflow, installer, Cargo, game, GPU, workspace test, or Clippy run. Binary icons/media and prose-only content were not judged.
