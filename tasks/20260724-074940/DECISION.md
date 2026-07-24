# Decision: Deep-link download buttons via a runtime GitHub API fetch, progressively enhancing static release-page links

- DATE: 20260724-074940
- STATUS: ACCEPTED
- TASK: 20260724-074940
- TAGS: decision, web, releases

## Context

The landing page needs per-OS download buttons (Windows, macOS, Linux) that
always point at the newest release. Release assets are named with the version
baked in - `nova-protocol_v0.8.0_windows.zip`, `_macOS.dmg`, `_linux.tar.gz`
(verified via `gh release view --json assets`). The site is a static webpack
build with no server side, so it cannot know the current version at build time
without coupling the page build to the release version.

## Decision

Render three buttons whose static `href` is the GitHub
`releases/latest` page (a permanent URL that always resolves to the newest
release). A small dependency-free TS module (`web/src/downloads.ts`, imported by
`index.ts`) then progressively enhances them: it fetches
`api.github.com/repos/alexjercan/nova-protocol/releases/latest`, matches each
asset by filename suffix, and rewrites each button's `href` to the exact
`browser_download_url`. Any failure leaves the static release-page fallback
intact.

## Alternatives considered

- **Static `/latest/download/<asset-name>` links** - GitHub's redirect endpoint
  requires the exact asset filename, but our filenames embed the version, so a
  hardcoded name breaks the moment a new version ships. Rejected.
- **Change the release pipeline to also publish version-less asset names** -
  would let us use stable `/latest/download/nova-protocol_windows.zip` links with
  zero runtime JS, but it enlarges the release workflow and does not help the
  already-published v0.8.0 assets. More moving parts for a landing-page button.
  Rejected for now; could revisit if the API dependency proves flaky.
- **Buttons that just point at the release page (no deep link)** - simplest, no
  JS, but every button lands on the same page and the user still has to pick the
  right file. Loses the per-OS one-click intent. Kept only as the JS-off
  fallback, not the primary experience.

## Consequences

- One-click, correct-platform downloads that track latest with no per-release
  edits to the page.
- Works with JS disabled (degrades to the release page) and survives API
  failures/rate limits gracefully.
- Adds a runtime dependency on the unauthenticated GitHub API (60 req/hr/IP).
  Acceptable for a low-traffic landing page; the fallback covers the ceiling.
- Asset matching is coupled to the filename suffix convention in
  `.github/workflows/release.yaml`; if that naming changes, the matcher must
  change with it (noted in `downloads.ts`).
