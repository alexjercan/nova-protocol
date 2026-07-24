# Review: Add per-OS release download buttons to the landing page hero

- VERDICT: APPROVE (round 1)

Task 20260724-074940. Reviewed `git diff master...HEAD`: `web/src/index.html`,
`web/src/style.css`, `web/src/downloads.ts`, `web/src/index.ts`.

## Summary

The change does exactly what the TASK, DECISION.md, and Outcome describe. All
five DoD checks pass, the build is green, the asset matcher is correct against
the real v0.8.0 release JSON, and every claimed behavior in the Outcome section
is backed by the diff. No blocking issues.

## Verification performed

- `grep -c 'data-os' web/src/index.html` -> 3 buttons (windows/macos/linux). PASS
- Every download button href defaults to
  `https://github.com/alexjercan/nova-protocol/releases/latest` (JS-off
  fallback). PASS. Built `dist/index.html` has exactly 3 `releases/latest`
  hrefs (the HTML comment is stripped by webpack minification).
- Enhancer wired: `grep -n downloads web/src/index.ts` -> import present, and
  `void enhanceDownloadButtons()` is called alongside `initSite()` /
  `warnIfNoWebGpu()`. PASS
- `cd web && npm run build` -> exit 0, `webpack ... compiled successfully`. PASS
- Real release assets (`gh release view --repo alexjercan/nova-protocol`):
  `nova-protocol_v0.8.0_windows.zip`, `_macOS.dmg`, `_linux.tar.gz`, `_web.zip`.
  Matcher suffix table matches all three OS assets and excludes `_web.zip`.
- Palette variables used in `.btn--download` (`--panel`, `--panel-2`, `--border`,
  `--border-bright`, `--text`, `--text-muted`) all resolve to real definitions
  in `:root` (style.css lines 22-37). PASS

## Correctness of asset matching (the risky core)

`pickDownloadUrls()` (downloads.ts:34-48) uses `name.endsWith(suffix)` with a
case-sensitive table:
- `_windows.zip` correctly does NOT match `_web.zip` (endsWith requires the full
  `_windows.zip` tail), so the web build is never mislabeled as a native
  download. This is the subtle case and it is handled correctly.
- `_macOS.dmg` preserves the mixed case of the real asset name; a lowercase
  `_macos.dmg` would silently miss and fall back. Correct as written.
- `_linux.tar.gz` matches the double extension. Correct.

## Fallback survives every failure path

`enhanceDownloadButtons()` (downloads.ts:52-84) leaves the static href untouched
on:
- no buttons found (early return),
- network throw (try/catch returns),
- `!res.ok` (rate limit, 404) -> return,
- `release.assets` not an array (unexpected JSON shape) -> return,
- a JSON parse throw inside `res.json()` -> caught,
- a specific OS asset missing -> `urls[os]` is `undefined`, the button is left
  as-is (only assigned when `url` is truthy).

The buttons are never left worse off than the static fallback. Matches the
defensive, idempotent style of `webgpu.ts`.

## Accessibility / semantics / CSS

- Anchors with text labels; `.hero__downloads-label` is a real `<span>` intro.
  No icon-only buttons, so no aria-label gap.
- `.hero__downloads` is a sibling of `.hero__cta`, in its own flex column; it
  does not touch the existing `.hero__cta` flex row, so the existing layout is
  unchanged. `flex-wrap: wrap` on the button row degrades on narrow viewports.
- No `target="_blank"`/`rel` on the download anchors - consistent with the
  existing internal Play/How-to-play CTAs (only the external Bevy link uses
  `rel="noopener"`). Same-tab navigation, so no reverse-tabnabbing risk.

## Security

The rewritten href comes from `browser_download_url` on the GitHub API for a
fixed, hardcoded repo. An attacker would need control of that repo's releases to
inject a URL, at which point the download itself is already compromised. No
open-redirect or attacker-controllable-URL concern introduced by this change.

## Findings

MAJOR: none.

MINOR: none.

NIT (optional, non-blocking):

- downloads.ts:75 - `button.dataset.os as Os | undefined` is cast without
  validating the value is a known key. In practice `urls[os]` is `undefined` for
  any unexpected `data-os`, so an unknown value just leaves the fallback (safe).
  No change needed; noted only for completeness.
- downloads.ts / DECISION.md correctly note the matcher is coupled to the
  `.github/workflows/release.yaml` filename convention. Consider a lightweight
  reminder in that workflow that renaming assets breaks the landing-page
  deep-links, but that is out of scope for this task.
