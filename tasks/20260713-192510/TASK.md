# Release v0.5.2: bump version and roll the changelog

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.5.2,chore,release

## Goal

Close out the v0.5.2 release once every other v0.5.2 task is CLOSED, the
same way v0.5.1 shipped (commit ef1d7ad: Cargo.toml + Cargo.lock +
CHANGELOG.md in one `chore(release)` commit).

## Steps

- [x] Verify every other task tagged v0.5.2 is CLOSED
      (`grep -l "v0.5.2" tasks/*/TASK.md` + check STATUS).
- [x] Bump `[workspace.package] version` 0.5.1 -> 0.5.2 in Cargo.toml and
      refresh Cargo.lock (`cargo update --workspace`).
- [x] Roll CHANGELOG.md: move the Unreleased entries under a new
      `## [0.5.2] - <date>` heading, leave an empty Unreleased.
- [x] Full check suite green on the default branch (check --workspace
      --all-targets + fmt locally per standing instruction; the full test
      suite and the now-BLOCKING 12-example smoke gate run in CI on push).
- [x] Single `chore(release): v0.5.2` commit (exactly Cargo.toml,
      Cargo.lock, CHANGELOG.md, per docs/development.md). Tagging/pushing
      left to the user.

## Notes

- Small enough to do directly on master (v0.5.1 precedent), but keep the
  landing discipline: verify the current branch before committing.


## Record (2026-07-14)

Released at the user's cadence: the release was held (user request) until
the parallel wiki tasks (20260713-225338/225353) landed, then rolled fresh
so their work ships in 0.5.2. Two entries were added on behalf of the web
sessions (the wiki/devlogs/tutorial, and the gamepad binding changes) -
they had landed user-visible work without changelog lines. Version bump,
lock refresh, changelog roll with compare links; local verify per the
standing instruction, with the full suite + blocking smoke gate deferred
to CI on push. Tag + push are the user's.
