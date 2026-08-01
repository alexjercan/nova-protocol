# Release v0.9.1

- STATUS: IN_PROGRESS
- PRIORITY: 100
- TAGS: v0.9.1, release, meta
- KIND: TASK
- FLOW STEP: WORKING
- PLAN STATUS: APPROVED

## Story

Cut v0.9.1 from the web-release fix already landed on `master`, publish the
four platform artifacts through `release-flow`, and record the patch in the
v0.9.0 News post.

## Steps

- [ ] Confirm the `39b7bc5d` CI run succeeds, `master` remains clean and synced
  with `origin/master`, `docs/` contains only `README.md`, and `v0.9.1` does not
  exist locally or remotely.
- [x] Promote the one-entry `CHANGELOG.md` Unreleased section to
  `[0.9.1] - 2026-08-02`, add a fresh empty Unreleased section, and update its
  compare links.
- [x] Bump `workspace.package.version` to `0.9.1`; refresh `Cargo.lock` with
  Cargo metadata inside the Nix development shell.
- [x] Run release metadata and web checks. Re-read generated/edited output.
- [x] On `master`, commit exactly `Cargo.toml`, `Cargo.lock`, and
  `CHANGELOG.md` as `chore(release): v0.9.1`; create lightweight release tag
  `v0.9.1`, matching the existing `v0.9.0` tag style.
- [ ] Push `master`, then `v0.9.1`; watch `release-flow` to completion and
  verify the GitHub release has macOS DMG, Linux tarball, Windows zip, and web
  zip assets.
- [x] Replace the v0.9.0 News placeholder under `## Point releases` with a
  concise v0.9.1 note, run website CI, then land and push that follow-up commit.
- [ ] Record review, release evidence, and retrospective in this task's flow
  records.

## Definition of Done

- Release scratch is empty and version metadata is internally consistent.
  (cmd: scripts/check-docs-clean.sh && nix develop --command cargo metadata --format-version 1 --no-deps)
- Release metadata uses v0.9.1, date 2026-08-02, and correct compare links.
  (cmd: rg -n 'version = "0.9.1"|\[0.9.1\] - 2026-08-02|v0.9.1' Cargo.toml Cargo.lock CHANGELOG.md)
- The patch fix still compiles on its affected target and formatting passes.
  (cmd: nix develop --command cargo check -p nova_menu --target wasm32-unknown-unknown && nix develop --command cargo fmt --check)
- Website and point-release note pass repository checks.
  (cmd: cd web && npm run ci)
- `v0.9.1` resolves to the release commit on `origin/master` and the GitHub
  release workflow succeeds. (manual: inspect pushed refs, workflow conclusion, and four release assets)

## Notes

- Scope source: completed fix task `20260801-234352`; no other Unreleased
  entries exist.
- Release procedure: `web/src/wiki/dev/development.md`, "Cutting a release".
- Patch releases extend `web/src/news/0.9.0.md`; no standalone News post.
- Full workspace tests and clippy remain CI-owned per `AGENTS.md`.
- External effects after approval: two pushes and GitHub release publication.
- Release metadata date uses the current project-local date, 2026-08-02.
- Release commit: `16cda098`; lightweight `v0.9.1` resolves to that commit.
- News follow-up: `d2b798c6`; pushed and synced with `origin/master`.
- Release-flow run: `30718763513`; in progress after the tag push.
- The prerequisite CI run `30717933387` was cancelled when the release push
  superseded it, so the first Step remains unchecked despite its other clauses
  passing before publication.
- Verification: docs clean, metadata consistent, wasm `nova_menu` check and
  formatting passed, website CI passed, rendered v0.9.0 News HTML inspected.
