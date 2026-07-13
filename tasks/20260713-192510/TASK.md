# Release v0.5.2: bump version and roll the changelog

- STATUS: OPEN
- PRIORITY: 5
- TAGS: v0.5.2,chore,release

## Goal

Close out the v0.5.2 release once every other v0.5.2 task is CLOSED, the
same way v0.5.1 shipped (commit ef1d7ad: Cargo.toml + Cargo.lock +
CHANGELOG.md in one `chore(release)` commit).

## Steps

- [ ] Verify every other task tagged v0.5.2 is CLOSED
      (`grep -l "v0.5.2" tasks/*/TASK.md` + check STATUS).
- [ ] Bump `[workspace.package] version` 0.5.1 -> 0.5.2 in Cargo.toml and
      refresh Cargo.lock (`cargo update --workspace`).
- [ ] Roll CHANGELOG.md: move the Unreleased entries under a new
      `## [0.5.2] - <date>` heading, leave an empty Unreleased.
- [ ] Full check suite green on the default branch.
- [ ] Single `chore(release): v0.5.2` commit. Tagging/pushing is the
      user's call - report ready instead of pushing.

## Notes

- Small enough to do directly on master (v0.5.1 precedent), but keep the
  landing discipline: verify the current branch before committing.
