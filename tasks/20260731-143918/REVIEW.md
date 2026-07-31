# Review: Port the NOVA OS phosphor theme to the web app

- TASK: 20260731-143918
- BRANCH: feat/web-nova-os-theme

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

No BLOCKER or MAJOR. The Story lands: the site reads as NOVA OS on all six page
kinds at both widths, the parity test is real (mutation-checked, see below), and
the close-out's claims reproduce.

- [x] R1.1 (MINOR) scripts/shoot-web-pages.sh:113 - `chromium --screenshot`
  exits 0 and writes a non-empty PNG for a failed load, so the only guard
  (`[[ -s "$file" ]]`) cannot tell a real page from a 404. Re-derived in-session:
  pointing the rig's own flags at `/news/9.9.9/` on the same server gave exit 0
  and a 17712-byte PNG. Combined with the readiness loop (lines ~95-105), which
  falls through silently after 50 tries as long as the server PROCESS is alive, a
  stale `PAGES` path or a not-yet-listening server makes the rig print
  "12 captures" over error pages - and DoD 4 is a `cmd:` proof that would pass
  falsely. Fix: fetch each URL's status with a `python3 -c 'urllib.request...'`
  one-liner before capturing and `exit 1` on non-200; make the readiness loop
  `exit 1` when it exhausts without connecting.
  - Response: Fixed. `assert_200()` fetches each URL before capturing and aborts on non-200; the readiness loop now tracks `listening` and exits 1 if it exhausts. Forced the failure: a PAGES entry pointed at `/news/9.9.9/` aborts with `returned 404` and exit 1 after 4 good captures.
- [x] R1.2 (MINOR) web/src/style.css:1149 - `.wiki-search::placeholder` is
  `var(--phosphor-muted)` (#0d6e35) on the `--screen` well (#001304) = ~3.0:1,
  below the 4.5:1 floor. It is readable text ("Search the wiki..."), not
  ornament, so it contradicts the rule this branch's own NOTES.md sets
  ("`--phosphor-muted` is kept for pure ORNAMENT ... every label that carries
  meaning is `--phosphor-dim`"). It is visibly the dimmest text on the wiki
  captures. Change to `color: var(--phosphor-dim);` (~6.0:1 on the same well).
  - Response: Fixed. `--phosphor-dim`, with the reason in a comment beside it. Confirmed on a fresh capture - the placeholder now reads clearly on the screen well.
- [x] R1.3 (MINOR) web/src/wiki/dev/keeping-docs-in-sync.md:60 - the branch
  creates a new hard coupling (the PoC `:root` now feeds BOTH
  `crates/nova_ui/src/theme.rs` and `web/src/style.css`), but the dependency
  map's `Menus, editor, UI (nova_menu, nova_editor, nova_ui)` row still lists
  only `hud.md`/`sections.md`/`guide-add-section.md`/tutorial/CHANGELOG.
  `development.md` documents the coupling, but THIS page is the map an agent
  consults. Add the web theme to that row's "Also" column, e.g. "web theme:
  `examples/ui/nova_ui_rework_poc.html` -> `web/src/style.css`".
  - Response: Fixed. The `nova_menu / nova_editor / nova_ui` row's Also column now names the PoC as the source for BOTH `theme.rs` and `web/src/style.css`.
- [x] R1.4 (NIT) web/src/style.css:234 - the bright-key gradient is a literal hex
  pair duplicated four times (234 and 243 for `.site-nav a.is-cta`; 462 and 469
  for `.btn--primary`), the only palette colours in the sheet outside `:root`, so
  a tweak to the key face is a four-site edit and is invisible to both the DoD-2
  grep and the parity test's dangling-`var()` check. Hoist to `--key-face` and
  `--key-face-hot` under the existing "site-only additions" comment and read them
  at all four sites.
  - Response: Fixed. Hoisted to `--key-face` / `--key-face-hot` under the site-only additions comment; all four call sites read them, and only the two `:root` definitions carry a literal hex. Confirmed the primary key still renders the bright gradient.

### Verified

- `cd web && npm run ci` green (format:check + lint + test + build) - run by the
  out-of-context reviewer and again in-session.
- `cd web && npm test` green (`site.test.ts` + `theme.test.ts`).
- DoD 2 grep (corrected form) exits 1, no hits; `glow-cyan` gone from both call
  sites.
- `scripts/shoot-web-pages.sh` -> 12/12 captures + manifest, exit 0.
- DoD 5 eyeball: all six page kinds read at 1440 and 390. Phosphor + case
  material on every one; no STYLED element kept a navy hue (only the captured
  game-UI images, which DECISION.md puts out of scope); mono prose legible at
  both widths; mermaid renders on-theme.
- The parity test is not coverage theatre. Mutations - reintroducing `--panel`,
  dropping `--case-2`, changing `--phosphor`'s value, adding a dangling
  `var(--nope)`, setting `--font-body: "Inter"`, and stripping the material from
  `.card` - each failed with the matching assertion. All reverted.
- The recorded first failure reproduces: running the test against
  `master:web/src/style.css` yields the same 22 missing tokens.
- Honesty: the "prettier was already red on master" claim reproduces; both
  "pre-existing, not mine" layout oddities (mobile SCROLL/LINUX overlap, amber +
  underlined wiki-index cards) confirmed present in a BEFORE capture taken from
  master by the reviewer; NOTES.md's contrast numbers recompute within rounding.
- Sampled rendered pixels: nav links and post-card excerpts land on the
  case-1/case-0 part of the `--face` gradient at ~4.9-5.7:1.

### Not verified

- `cargo check -p nova_ui` not re-run by the out-of-context reviewer (the
  `theme.rs` diff is comment-only); the implementer ran it green.

### Pending user checks

- DoD 6 (`manual:`) - the owner confirms the ported site against the game's look,
  and accepts the mono reading of a long wiki page. Correctly left PENDING; does
  not block APPROVE.

## Round 2

- REVIEWER: in-session (fix-confirmation round - all four Round-1 findings were
  MINOR/NIT and the round already carried an APPROVE verdict; a second
  out-of-context reader was not warranted for four contained fixes, each of which
  is confirmed by a check run below rather than by reading)
- VERDICT: APPROVE

All four Round-1 findings verified fixed and ticked above. Confirmations:

- R1.1 - forced the failure rather than trusting the guard. A copy of the rig
  with `news-post` pointed at `/news/9.9.9/` aborts after 4 good captures with
  `!! http://127.0.0.1:33099/news/9.9.9/ returned 404 - fix the PAGES entry or
  the build` and exit 1. The unmodified rig then re-ran to 12/12 captures, exit 0.
  (`degrade-paths-need-a-forced-failure`: the new guard is not a plan claim, it
  has been made to fire.)
- R1.2 - re-captured and cropped the wiki search field; the placeholder now reads
  clearly against the screen well.
- R1.3 - the map row now names the PoC coupling.
- R1.4 - `#7dffab` and `#9dffbf` each appear exactly once in `style.css`, both in
  `:root`; the primary key still renders the bright gradient in a fresh capture.
- `cd web && npm run ci` green after the fixes (format:check + lint + test +
  build); `bash -n scripts/shoot-web-pages.sh` clean.

No new findings. No regressions from the fixes.

### Pending user checks

- DoD 6 (`manual:`) - unchanged from Round 1: the owner confirms the ported site
  against the game's look, and accepts the mono reading of a long wiki page.
