# Release checklist

Release from `master`. Use semantic versions and tags in the form `vX.Y.Z`.

## Prepare

- [ ] Confirm the intended work is complete and `master` is green.
- [ ] Review everything since the previous tag. Ensure user-visible changes are in `CHANGELOG.md` and affected wiki pages are current.
- [ ] Confirm the developer book is current: `docs/` is the mdbook source, so pages the cycle's changes touched must already be updated (`nix develop --command mdbook build` is green).
- [ ] Check the game, web build, and release-critical player paths.

## Set the version

- [ ] Choose the next version from the scope of the changes.
- [ ] Update `workspace.package.version` in `Cargo.toml`.
- [ ] Refresh `Cargo.lock` and confirm the Nova packages use the new version.

## Finish the release notes

- [ ] Replace the current `Unreleased` changelog section with `[X.Y.Z] - YYYY-MM-DD`.
- [ ] Add a new, empty `Unreleased` section at the top.
- [ ] Merge duplicate subsystem headings and check every entry against the changes since the previous tag.
- [ ] Update the changelog comparison links: point `unreleased` at the new tag and add the new version range.
- [ ] Update News: add a post for a feature release, or add a point-release section to the parent feature post for a patch release.

## Publish

- [ ] Commit the version, changelog, News, and any final documentation updates.
- [ ] Check the final commit and create the `vX.Y.Z` tag on it.
- [ ] Push `master` and the tag. The tag starts the GitHub release workflow.
- [ ] Confirm the macOS, Linux, Windows, and web assets are attached to the GitHub release.
- [ ] Run the GitHub Pages deploy workflow, then verify the published site and download links.
