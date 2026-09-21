# Review 20: Platform, web, CI, and scripts

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: major and minor findings; residual coverage gaps

## Findings

### MAJOR - `.github/workflows/deploy-page.yaml:53-65` - Pages deploy bypasses the pinned Rust toolchain pattern

The workflow installs stable with `dtolnay/rust-toolchain` and then adds the wasm target without first resolving the repository's pinned nightly. `release.yaml` documents that this exact old pattern registered targets against the wrong toolchain and replaces it with `rustup show active-toolchain` before target installation.

The public `/play/` build therefore diverges from the toolchain contract and repeats a previously observed build-failure pattern. It was not reproduced in this review, so it is not BLOCKER.

### MAJOR - `web/webpack.config.js:293-469` - Production commands return a development-mode webpack configuration

The config receives `argv.mode` and forwards it to story build setup, but the returned webpack configuration hardcodes `mode: "development"`. `build:site` and `build:story` pass `--mode production`, yet shipped bundles use development defaults rather than production optimization and environment semantics.

The site still builds and functions, so this is MAJOR rather than BLOCKER.

### MAJOR - `scripts/gen-lesson-media.py` - Its deterministic check mode is not run in CI

The script implements `--check` and states that stale or nondeterministic committed lesson media must fail CI. The generated-art CI gate runs seven sibling generators but never this one. Its required `ffmpeg` dependency is also absent from that job.

This leaves a documented generated-media contract unenforced.

### MINOR - `build/web/webgpu-check.test.mjs` - WebGPU fallback tests are orphaned

The test covers the shipped pre-canvas WebGPU gate, but neither `npm run test`, `npm run ci`, nor a workflow invokes it. A regression in the black-canvas fallback can ship despite the existing test.

### MINOR - `web/src/downloads.ts:35-47` - The release asset selector has no test

`pickDownloadUrls` is explicitly exported as the DOM-free, runtime-testable core of download selection. No test imports it, even though it encodes the release workflow's platform filename contract.

### MINOR - `scripts/nova_sfx.py` and the three SFX generators lack a check mode

The NOVA OS, UI, and world SFX generators promise byte-stable reruns, but their shared renderer only overwrites committed WAVs. Unlike generated art, there is no non-writing comparison mode or CI gate. No current asset drift was established.

## Adjudication

Checked and cleared root feature gating, wasm exclusion rationale, release asset naming, toolchain pin agreement between Nix and rust-toolchain, probe shard validation, deployment path containment, and meta-sidecar generation.

## Coverage

Fully read root build/config files, all workflows, core deployment and shard scripts, webpack/package configuration, key web entry/download/WebGPU files, several web build modules, and code-linked environment/documentation contracts. One reviewer also read all comic source modules and selected web tests.

Residual gaps remain: many generator/capture/illustration scripts, several web tests and large widget bodies, build platform files, and some web helper modules were sampled or only enumerated. They require an additional paired coverage batch.

Not checked: no Cargo, npm, webpack, Trunk, workflow, game, probe, wasm, workspace test, or Clippy run. Build/deploy findings are static and were not reproduced in CI.
