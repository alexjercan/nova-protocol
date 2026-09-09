# Independent comic publishing

## Delta

The user approved separate comic release tags and a comic-only deployment path.

- `build:story` emits only `web/dist-story/story/`: released pages, reader,
  selected images, favicon, and font assets. It does not load the docs/News
  builders. Normal serving stays unchanged; it includes labelled drafts.
- `build:site` skips comic generation and output. Normal `build` remains the
  complete released-only website check target.
- `comic-*` tag pushes start `deploy-comic`; manual runs accept an existing tag.
  A tag is a library snapshot, not a selector. Metadata still controls release,
  and incomplete releases fail. No game version bump is required.
- Build artifacts and their exact source commit pass to separate publish jobs.
  Both publishers share the same non-cancelling running-job lock. They read the
  latest `gh-pages` after acquiring it, use an isolated temporary index/tree,
  preserve the other owner's files, and never force-push.
- Story replaces only its own subtree. Site preserves that subtree and hosting
  configuration. Story's `release.json` records its tag and source revision.
- A legacy Story directory must first receive a comic bootstrap, so removing
  old root-level reader files cannot break the preserved library.
- Both jobs explicitly request and wait for a Pages build. Token-based pushes
  alone do not start one. Hosting remains the existing branch-root model, not
  a switch to Actions-artifact Pages hosting.

The REST build request supports GitHub App installation tokens with Pages write
permission. Reference checked during implementation:
<https://docs.github.com/en/rest/pages/pages#request-a-github-pages-build>.
This is source research, not evidence of a live deployment.

Docs describe immutable tags, bootstrap, retry/rollback, current-workflow use,
queue ordering limits, and the shared external Google Fonts dependency. No
whole-site offline guarantee is made.

## Verified

Evidence is under [proof/deployment/](proof/deployment/).

- Full website CI passes, including the new deployment tests. Publisher tests
  exercise local bare repositories only; Pages HTTP calls are mocked.
- Tests cover bootstrap, scoped replacement, stale cleanup, retained hosting
  files, repeat/no-op publication, exact source revision, malformed input,
  source-index preservation, legacy bootstrap refusal, and non-fast-forward
  rejection when an outside publisher wins a race.
- API tests cover expected hosting configuration, exact-commit build completion,
  a failed build, and timeout. Workflow tests check the common lock and separate
  build targets. `actionlint` passes for both workflows.
- Isolated root/prefix fixtures borrow the seven accepted illustrated pages as
  a complete test-only release. Throwing draft TS/Python and unrelated site
  modules do not execute or enter output. Site-only builds do not execute even
  an invalid comic entry. The real episode metadata is never changed.
- Browser checks load those fixtures at desktop and phone widths through CDP
  pipes and intercepted files. Every same-origin resource stays under `/story/`.
  All seven page exports remain pixel-identical at root and project prefix.
  Navigation, browser history, panel exports, modals, and safe SVG handling pass.
- Real comic-only builds remain empty released archives at root and prefix.
  Draft content is not published by an empty-library bootstrap.
- 923 retained source/art/proof files match their committed bytes. Earlier proof
  writers and snapshots remain untouched. mdBook passes with its existing
  Mermaid version warning.

The concurrent game release and task move to the v0.14.0 board were preserved.
This batch does not create a tag, commit, push, or live deployment. It starts no
server and runs no Rust checks. Inspection browsers were stopped by owned PID.

## Next

Review and commit the workflow. Publish `comic-reader-1` once from that new
commit before the first site-only deployment. Keep the episode draft until all
eighteen pages are finished and approved. Then mark it released and publish a
new comic tag; use a new tag for corrections rather than moving an old one.
