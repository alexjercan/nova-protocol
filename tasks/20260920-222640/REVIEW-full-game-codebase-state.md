# Full game codebase state review

- Task: `20260920-222640`
- Baseline: `b7a56f0586`
- Slug: `full-game-codebase-state`
- Verdict: ACTION REQUIRED
- Findings: 0 BLOCKER, 12 MAJOR, 42 MINOR

Six scouts mapped the repository. Twenty-five review rounds then ran in pairs, with at most two active subagents at any time. Seven ECS, content, and proof specialists rechecked claims that crossed ownership or trust boundaries. No source or content fix was made.

## Major findings

### 1. Headless architecture documentation contradicts app assembly

`docs/architecture.md:224` says HUD and NOVA OS UI are render-gated. `crates/nova_core/src/lib.rs:425-433` installs both unconditionally to preserve actions and OS behavior in headless runs. This gives proof authors the wrong CPU/system model. See `REVIEW-01-assembly-input-events.md`.

### 2. Gauntlet geometry is documented as proved, but no proof exists

`webmods/gauntlet/README.md:31-34`, `webmods/gauntlet/gauntlet.content.ron:15-25`, and `docs/scenario-system.md:670-673` claim gate non-overlap and racing-line clearance are checked. The named test was deleted, its synthetic replacement has no geometry, and content lint implements neither check. See `REVIEW-11-authoring-base-content.md`.

### 3. Objective-stack rebuilding can panic automated proof runs

`crates/nova_hud/src/objective_stack.rs:376-405,622-632` can queue `despawn_related` after a same-frame observer has despawned the stack. Automated examples install a panic command-error handler, so this teardown race can crash proof runs. Production normally warns instead. See `REVIEW-12-ui-foundation-hud.md`.

### 4. NOVA OS app launch can mutate the hidden prompt

`crates/nova_os_ui/src/terminal/input.rs:235-243` caches prompt-active state before draining a keyboard batch. Enter can launch an app, but later messages in the same batch still mutate or submit the hidden prompt. Autopilot can create this exact multi-event batch. See `REVIEW-14-os-console.md`.

### 5. A persistent cmd-agent connection can bypass the bench grace deadline

`crates/nova_bench/src/agent/socket.rs:47-96` checks `OVER_GRACE` only in the outer accept loop. The inner connection loop can run forever after the referee ends, preventing score/report writing and bounded child cleanup. See `REVIEW-18-bench-channel-debug.md`.

### 6. The bench targeting manual gives a stale dwell curve

`crates/nova_bench/src/pages/targeting.md:22-26` gives 0.645 s at 1 km and saturation at 20 km. Current targeting defaults produce 1.05 s at 1 km and saturate at 2 km. The recommended 54-tick hold releases before the real 63-tick requirement. See `REVIEW-18-bench-channel-debug.md`.

### 7. Bench shared-key validation ignores holds from earlier acts

`crates/nova_bench/src/gesture.rs:136-148` checks only the current act. A later tap of a shared-key counterpart can release the same physical key while the referee still records the first action as held. See `REVIEW-18-bench-channel-debug.md`.

### 8. Pages deployment repeats a previously fixed toolchain pattern

`.github/workflows/deploy-page.yaml:53-65` installs stable and adds wasm without resolving the pinned nightly. `release.yaml` documents that the same pattern registered targets against the wrong toolchain and was replaced elsewhere. See `REVIEW-20-platform-web-ci-scripts.md`.

### 9. Production web builds return development webpack mode

`web/webpack.config.js:293-469` receives `--mode production` but returns `mode: "development"`. Public site and comic bundles therefore use development defaults instead of production optimization/environment semantics. See `REVIEW-20-platform-web-ci-scripts.md`.

### 10. Lesson-media determinism is promised but not gated

`scripts/gen-lesson-media.py` implements `--check` and says stale media must fail CI. The generated-art workflow never runs it and does not install ffmpeg. See `REVIEW-20-platform-web-ci-scripts.md`.

### 11. The damage-level establishing shot races asynchronous capture

`examples/screenshots/screenshot_damage_levels.rs:453-457` requests a shot without waiting, then immediately reframes the camera. The image can use the next pose or be lost. The same file waits for all later shots and documents this race. See `REVIEW-23-screenshot-examples.md`.

### 12. The changelog header points to nonexistent policy

`CHANGELOG.md:12-14` directs contributors to a nonexistent AGENTS.md Changelog section and claims it owns rules that actually live in `nova-docs/SKILL.md`. A contributor following the required pointer misses the character cap and intra-cycle omission rule. See `REVIEW-25-final-code-edges.md`.

## Minor finding index

The batch reports contain full evidence, blast radius, adjudication, and coverage. Minor findings fall into these groups:

- Core and shared infrastructure: stale loading-note refresh, prelude bypasses, lint allowance, stale rustdoc links, and a stale neutralize owner reference (`REVIEW-01` to `REVIEW-03`).
- Gameplay: unused orbit plugins and obsolete vertex-mesh paths with false caller docs (`REVIEW-04`).
- Ship: vacuous cross-schedule point-defense ordering, deleted `WithheldVerbs` comments, and a false-positive unauthored-audio fixture (`REVIEW-05`, `REVIEW-07`).
- Scenario/content: stale asteroid `material` docs and dead `PlanetHeight` public surface (`REVIEW-09`).
- UI/menu: unconditional text-field repaint, unproved media load/rejection transitions, and duplicate menu test helpers (`REVIEW-12`, `REVIEW-13`).
- Editor: duplicate ordinal parsing, incomplete module map, visible tooltip whitespace, and duplicate colour-picker docs (`REVIEW-15`, `REVIEW-16`).
- Proof infrastructure: stale systems-example policy/docs, silent malformed movie-rail lines, and nine incompletely pinned probe environment names (`REVIEW-17`, `REVIEW-18`, `REVIEW-25`).
- Playable examples: duplicated skin-bench and idle-orbit implementations, stale WFC paths, and hardcoded exported IDs (`REVIEW-19`).
- Web/scripts/build: orphaned WebGPU tests, untested download/docs/news entries, missing SFX check mode, deleted script references, premature test success log, stale inert WiX version, and PI relay outside TypeScript CI (`REVIEW-20`, `REVIEW-24`, `REVIEW-25`).
- System proofs: stale catalog comment, lint-policy exception, and damaged assertion diagnostics (`REVIEW-21`, `REVIEW-22`).
- Screenshot/lesson proofs: missing contact-selection and standoff assertions plus hardcoded exported IDs (`REVIEW-23`).

## Clean reviewed slices

No finding survived in:

- ship weapon section fire/ammo/reload/despawn behavior (`REVIEW-06`);
- scenario lifecycle, event world, filters, trackers, preload, clock, and gate (`REVIEW-08`);
- asset merge, mod formats, portal install, cache, refs, safe mode, and native/wasm persistence source (`REVIEW-10`).

Other reports contain findings but also document substantial cleared behavior.

## Coverage audit

Full static reads now cover:

- every Rust source file in all workspace crates;
- root source, build script, root tests, PI subagent config, and tool sources;
- every playable, system, lesson, loop, and screenshot example, including nested/shared modules;
- every non-generated web TypeScript/JavaScript source and web test;
- every Python/shell script and the illustration package/tests;
- workflows, Cargo/Nix/toolchain/Trunk configuration, hooks, wasm lint config, and platform build text;
- Rust base-content builders and authoring tests;
- representative real base/mod/webmod RON and every runtime-ID class needed to trace code contracts.

Named exclusions and limits:

- Binary meshes, textures, audio, fonts, screenshots, videos, and icons were not decoded or visually judged.
- JSON art recipes were not read value-by-value; all parsers and schema/refusal paths were reviewed.
- Generated base RON was not manually diffed file-by-file against builders, and the parity test did not complete in this session.
- Narrative, marketing, changelog-body, and wiki prose was reviewed only where it states a code or authoring contract.
- Dependency vulnerability/licensing audits and full Cargo.lock version analysis were excluded.
- Browser-only IndexedDB behavior and rendered visual claims received static review, not runtime observation.

## Verification

Executed focused checks reported by reviewers:

- `cargo test -p nova_gameplay --lib`: 343 passed, 2 ignored.
- `cargo test -p nova_wfc --lib`: 17 passed.
- `cargo test --example wfc_arena --features debug`: 15 passed.
- `cargo test -p nova_probe_cli --test catalog_drift`: 2 passed.
- `cargo test -p nova_authoring --test content_ron_parity`: timed out after two minutes on a cold build; not a pass.

No workspace test, Clippy, content generation, full content lint, game run, browser run, probe run, bench play, or GPU measurement was performed. Static findings that need a focused reproduction are labeled in their batch reports.
