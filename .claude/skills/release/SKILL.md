---
name: release
description: Cut a nova-protocol release end to end - verify the sprint is clean and docs/web are current, bump the version, update CHANGELOG + the News post, commit and tag, then (on the owner's explicit go) push the tag to trigger the release-flow CI and dispatch the web deploy. Use when the user asks to "release vX.Y.Z", "cut a release", "tag and ship", or "do the release steps". Handles both feature releases (own News post) and patch releases (fold into the parent post's Point releases).
---

# Release - Cut and Ship a Version

Drive a nova-protocol release through its checklist: verify, bump, changelog +
news, commit, tag, and - only after an explicit owner go - push and deploy.
Pushing a `vX.Y.Z` tag builds and PUBLISHES public binaries; dispatching the
web deploy publishes the public site. Those two steps are outward-facing and
irreversible, so they are gated behind an explicit confirmation every time.

Canonical procedure: `web/src/wiki/dev/development.md` -> "Versioning and
release". This skill is that checklist plus the guardrails learned in practice.

## Facts about this repo's release machinery

- **Version** lives once, in `workspace.package.version` in root `Cargo.toml`;
  every crate inherits it. `nova_info::APP_VERSION` flows from it via `build.rs`.
- **`.github/workflows/release.yaml`** triggers on a pushed tag matching
  `v[0-9]+.[0-9]+.[0-9]+*` (also `workflow_dispatch` with a `version` input). It
  builds and uploads four assets to a GitHub release named after the tag: macOS
  `.dmg`, Linux `.tar.gz`, Windows `.zip`, wasm web zip.
- **`.github/workflows/deploy-page.yaml`** is **`workflow_dispatch` ONLY** - the
  public site (`/`, the game at `/play/`, the mod portal at `/mods/`) deploys
  only when you run it: `gh workflow run deploy-github-page`. It builds from
  `master` HEAD at dispatch time, NOT from the tag. This is the "web release
  page" / "deploy the web app" step.
- **News model:** one post per FEATURE release (`web/src/news/<minor>.md`).
  PATCH releases do NOT get their own post - they fold into the parent minor
  post's `## Point releases` section (v0.8.1 -> a `### v0.8.1` block in
  `web/src/news/0.8.0.md`).
- The landing download buttons deep-link release assets by filename suffix
  (`web/src/downloads.ts`, `OS_ASSET_SUFFIX`); if release asset names ever
  change, that file must change with them (noted in release.yaml's env comment).

## Steps

### 1. Pre-flight checks (nothing is committed yet)

- **Sprint clean:** `tatr ls -f "(:tags contains vX.Y.Z) and (not :status eq
  CLOSED)"` returns nothing, or the user explicitly ships with tasks open. For a
  patch release off a prior minor, check that minor's tag instead.
- **docs/ clean:** `ls docs/` shows only `README.md`. If scratch remains, distil
  it first (reusable lessons -> `knowledge add` in the central repo, reference
  -> the wiki) then clear everything under `docs/` except its `README.md`. This
  is a convention, not a gate: nothing in CI checks it any more, so it only
  happens if you do it.
- **What is shipping:** `git log --oneline <lastTag>..HEAD` and
  `git diff --stat <lastTag>..HEAD`. Every user-facing change in that range must
  be reflected in CHANGELOG + news; anything missing gets written now. A change
  that landed without a changelog line is the common gap.
- **master is green:** `.github/workflows/ci.yaml` owns the test suite (fmt,
  clippy, `cargo test --workspace --features debug`, the windowed
  `examples_smoke` run under Xvfb, and the license gate). Confirm the last
  `master` run passed - `gh run list --workflow=ci --branch master -L 3`. Do not
  run the full suite locally to substitute for it. If a job is red, cite the job
  LOG's result line, not the run conclusion.
- **Fleet probe sweep:** `cargo run -p nova_probe -- run --all` (25-40 min) is
  the pre-release run-harness check. Read `index.json`, not just the exit code:
  the aggregate verdict is the WORST row, and a SKIPPED check means UNMEASURED.
  See the `probe` skill. Skip it only for a docs-only patch release, and say so.
- **Docs/web current:** sweep the doc surfaces the shipped changes invalidate
  (`web/src/wiki/dev/keeping-docs-in-sync.md` is the map). Then validate the
  site actually builds: `cd web && npm run ci` (format:check + lint + test + build) -
  this also renders the News markdown, so it catches a malformed post. Note
  `format:check` covers ts/html/js only, NOT markdown - do not run prettier over
  wiki or news `.md` files, it reformats the whole file as unrelated churn.

### 2. Version bump + lock

- Edit `workspace.package.version` in root `Cargo.toml` to the new version.
- Refresh the lock: `nix develop --command cargo metadata --format-version 1
  >/dev/null`, then confirm `grep -A1 'name = "nova-protocol"' Cargo.lock` shows
  the new version. (No cargo on PATH without the devshell - every cargo command
  in this repo goes through `nix develop --command`.)

### 3. CHANGELOG

Promote `## [Unreleased]` to `## [<version>] - <YYYY-MM-DD>` (today's date), add
the shipped entries under the subsystem headings (one terse commit-title line
each - no paragraphs; that is what News is for), leave a fresh empty
`## [Unreleased]` on top, and fix the compare-links footer: repoint
`[unreleased]` to `<version>...HEAD` and add `[<version>]:
.../compare/<lastTag>...<version>`. Write entries FROM THE DIFF, not from memory
(central lessons `docs/document-observed-behavior`,
`docs/update-restatements-with-the-source`).

### 4. News

- **Feature release** (`vX.Y.0`): write `web/src/news/<X.Y>.md` (body only - the
  shell adds H1/meta/footer), register it in `web/webpack.config.js` `NEWS_POSTS`
  (newest-first) and add a card in `web/src/news.html`. Mirror an existing post.
- **Patch release** (`vX.Y.Z`, Z>0): add a `### vX.Y.Z` block under a
  `## Point releases` section at the END of `web/src/news/<X.Y>.md` (create the
  section if absent). No new post, no webpack change.
- Re-run `cd web && npm run ci` after editing news.

### 5. Commit + tag

- Verify branch: `git branch --show-current` = `master` (shared-checkout
  discipline; never `git add -A` here - stage explicit paths, and NEVER stage an
  unrelated dirty file like a local `Trunk.toml` dev tweak).
- Release commit is EXACTLY the three version files:
  `git add Cargo.toml Cargo.lock CHANGELOG.md && git commit -m "chore(release):
  vX.Y.Z"`.
- News (and any wiki sync) as a follow-up commit:
  `git add web/src/news/... && git commit -m "docs(news): ..."`.
- Tag: `git tag vX.Y.Z` (on HEAD, after both commits).

### 6. GATE: confirm before push + deploy

STOP and present exactly what is about to become public:
- the two commits + the tag,
- that `git push origin master && git push origin vX.Y.Z` triggers release-flow
  (public binaries on a GitHub release),
- that `gh workflow run deploy-github-page` publishes the site.

Get an explicit "go". This mirrors the project's convention that the owner
authorizes the actual publish (v0.8.0 was tagged locally and left for the owner
to push). Do not push on your own initiative.

### 7. Push, watch, deploy (on go)

- `git push origin master && git push origin vX.Y.Z`.
- Watch the release build: `gh run watch` (or `gh run list --workflow=release-flow`).
  Confirm the four assets uploaded: `gh release view vX.Y.Z`.
- Deploy the site: `gh workflow run deploy-github-page`, then `gh run watch`.
- Optionally add summarized release notes:
  `gh release edit vX.Y.Z --notes-file <file>`.

### 8. Report

Summarize: the version, what shipped (changelog delta), the release URL, the
deployed site URL, and anything left for the owner (e.g. Discussions post). If a
CI job went red, cite the job LOG's result line, not the run conclusion
(central lesson `verification/trust-the-authority-not-the-report`).

## Guardrails

- The push and the deploy are the only irreversible, outward-facing steps - they
  are always gated behind an explicit go (step 6). Everything before is local and
  safe to prepare without asking.
- Changelog/news prose comes from the DIFF, not intent; re-read each entry
  against `git diff <lastTag>..HEAD` before committing.
- Keep the release commit to exactly the three version files; news and wiki are
  separate commits.
- Stage explicit paths on master; never sweep a dirty working tree into the
  release commit.
- A patch release folds into the parent minor's News post; only a feature
  release gets its own post + webpack registration.
