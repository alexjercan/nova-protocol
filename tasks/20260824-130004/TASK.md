# Publish the v0.14.0 demo on itch.io

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.14.0,release,distribution,itchio

## Goal

After the v0.14.0 GitHub release is complete, publish its Windows, macOS,
Linux, and web artifacts as Nova Protocol's free initial demo on itch.io. Steam
receives only the separate Coming Soon page (`20260909-212935`) in this cycle.
No Steam build, depot, or demo ships.

Owner decision, 2026-09-18: v0.14.0 is a quick initial release. The itch.io
demo is free. Use itch.io `No payments`, not a minimum price or suggested
donation. The repository remains public on GitHub. Future Steam release scope
and pricing remain open.

Owner decision, 2026-09-19: deploy the tagged GitHub artifacts through a
separate, manually dispatched `.github/workflows/deploy-itch.yaml`. Prepare the
four Butler payloads with
`scripts/prepare-itch-upload.sh <tag> <download-directory> <output-directory>`.
The protected `itch-production` GitHub Environment owns approval and the API
key. Publication remains manual after clean-platform checks.

Owner decision, 2026-09-19: the macOS bundle executable is named
`nova-protocol`. Release CI writes both bundle version fields from the tag and
must fail before creating the DMG when `Info.plist` does not name the copied
executable.

## Page identity

- Owner account: `alexjercan`
- Project slug: `nova-protocol`
- Butler target: `alexjercan/nova-protocol`
- Intended public URL: `https://alexjercan.itch.io/nova-protocol`
- Edit URL: pending capture from the live dashboard

## Inputs

- The final `v0.14.0` GitHub release and its exact Windows ZIP, macOS DMG,
  Linux tarball, and web ZIP. Do not rebuild store binaries from another commit.
- The approved pitch, cover, screenshots, trailer link, copy, requirements, and
  metadata from `20260909-212954`.
- The public Steam URL from `20260909-212935` when available. The itch.io page
  can be prepared without it, but the two public pages must link to each other.

## Create the itch.io page

1. Create the project in Draft visibility. Record the owner account, project
   slug, edit URL, and eventual public URL on this task.
2. Set the project classification and kind for a game with a browser build and
   native downloads. Set its release status to the honest in-development/demo
   state offered by the live form.
3. Set pricing to `No payments`. Label the release as a free initial demo; do
   not imply that the later Steam product will be free or paid.
4. Populate:
   - title and approved short description;
   - the 630x500 cover using itch.io's 315:250 aspect ratio;
   - approved long description and feature list;
   - eight to ten gameplay screenshots;
   - supported platforms, controls, languages, genre, tags, accessibility, and
     current development status;
   - links to the website, source repository, issue tracker, and Steam Coming
     Soon page;
   - install notes and Windows/macOS/Linux requirements derived from the tagged
     release.
5. Preview the page at desktop and narrow widths. Check image crops, contrast,
   download labels, external links, and that no copy promises open world,
   stations, expanded ships, mobile controls, or full gamepad support.

## Upload the tagged artifacts

Use the manually approved `deploy-itch` workflow so later versions update
stable Butler channels from the exact GitHub release artifacts. The
`itch-production` Environment supplies `BUTLER_API_KEY`, `ITCH_TARGET`, the
pinned Butler version, and the official archive checksums without committing
credentials. `scripts/prepare-itch-upload.sh` verifies and unpacks the release
artifacts before any channel push. Production uses the workflow's native macOS
DMG tools. Linux local runs use 7-Zip from the Nix shell and restore the app
executable's mode before producing the same channel layout.

- `butler push <windows-dir> <user>/<project>:windows --userversion 0.14.0`
- `butler push <macos-dir> <user>/<project>:osx --userversion 0.14.0`
- `butler push <linux-dir> <user>/<project>:linux --userversion 0.14.0`
- `butler push <web-dir> <user>/<project>:html5 --userversion 0.14.0`

Channel names must stay stable. Confirm platform detection after each first
push. Mark the web channel `HTML5 / Playable in browser` in itch.io if Butler
does not do it. The web upload must contain `index.html` at its root, use exact
case for referenced files, and make only HTTPS external requests.

Before making the page public, compare every uploaded channel with the matching
GitHub artifact by file inventory and recorded checksum. Do not upload a local
working-tree build.

## Publish and verify

1. Keep the page Draft until the GitHub release and website are live and all
   uploads pass checks.
2. Set visibility to Public. Butler pushes become live as soon as processing
   finishes, so do not push replacement content casually after publication.
3. In a clean profile or machine:
   - download, install, and launch Windows;
   - download, mount/install, and launch macOS;
   - download/extract and launch Linux;
   - launch the browser build and reach playable flight.
4. Judge actual title-screen-to-play behavior, assets, logs, and rendered web
   output. An installer exit code or page load alone is not proof.
5. Add the itch.io badge beside the website's GitHub download actions. Keep the
   existing GitHub release downloads as direct alternatives.
6. Verify the itch.io page links to Steam and the Steam page links to itch.io as
   the current place to play. Record URLs, upload versions, checksums, machines,
   operating-system versions, and observed results on this task.

## Agent findings

The deployment preflight exposes an existing macOS release-package defect that
must be fixed before the first itch.io run:

- `build/macos/src/Game.app/Contents/Info.plist` declares
  `CFBundleExecutable=bevy_protocol`, while the release workflow copies the
  binary as `Contents/MacOS/nova-protocol`.

The real `v0.13.2` Windows ZIP and macOS DMG were inspected on 2026-09-19; both
have the intended `assets/base/...` and `credits/...` layout, so the source
copy commands do not justify an archive-layout change. The macOS executable
mismatch is present in the real DMG. `scripts/prepare-itch-upload.sh` rejects it
rather than uploading an app that Finder cannot launch. The source bundle now
declares `nova-protocol`, and release CI verifies that contract before creating
the next DMG. A tagged macOS build is still required to prove the repair.

## Failure policy

Do not publish a missing, mismatched, or untested platform channel. Keep the
page Draft, or return it to restricted visibility, if the project metadata,
pricing, artifacts, or launch path is wrong. A failed platform is a release
blocker unless the owner explicitly removes that platform from the page and
records the cut here.

## Done when

The public itch.io page presents the approved game accurately, accepts no
payments, offers the exact tagged v0.14.0 demo through stable Windows, macOS,
Linux, and HTML5 channels, and each channel reaches playable flight in its
clean-platform check. The website, itch.io page, GitHub release, and Steam
Coming Soon page link to the intended destinations.

Requirements checked 2026-09-18 against itch.io `Your first itch.io page`,
`Pricing`, Butler `Pushing builds`, and `Uploading HTML5 games`. Recheck the live
project form before publication because it is authoritative.
