# Review: sharpen the web visual design (20260713-222025)

- DATE: 20260713-222025
- VERDICT: APPROVE (round 2)

## Round 2

- Finding 1 fixed: `index.html` eyebrow content is now `Systems online`, so it
  renders `[ Systems online ]` (built dist confirms).
- Finding 2 fixed: `:focus-visible` cyan ring added for links, buttons, nav and
  card links (present in the bundle).
- Finding 3: accepted tradeoff, recorded in the CSS comment; no change.
- Finding 4: filed as task 20260713-222824 (stale aim-assist-cone copy).
- `npm run ci` green after the fixes.

APPROVE.

---

## Round 1

- DATE: 20260713-222025
- Round 1 VERDICT (superseded by the round 2 APPROVE above): REQUEST_CHANGES

Reviewed the `style.css` rewrite + `index.html` markup against the task Goal and
the spike's Option B. Palette and banner are untouched, the floaty levers are
gone (radii -> 2px, glow halos deleted, hover-float replaced by border/keycap
feedback, header de-glassed), fonts now actually load, and the emoji cards are
mono index numbers. Build is green (`npm run ci`). Two fixes before approve, one
tradeoff to record, one out-of-scope follow-up.

## Findings

### 1. [Minor] Eyebrow renders `[ // Systems online ]` - double-decorated
`.section__eyebrow::before/after` now wrap the label in `[ ]`, but the only
eyebrow in `index.html` still starts with a literal `// `. The two motifs stack
into `[ // Systems online ]`, which is busier than either alone and reads as a
mistake. Fix: drop the `// ` from the HTML content so the bracket motif (applied
consistently from CSS) stands on its own -> `[ Systems online ]`.

### 2. [Minor] No keyboard focus states
The redesign replaced glows/soft outlines with borders but adds no
`:focus-visible` styling, so keyboard focus falls back to the UA default (and on
some elements the removed transforms/backgrounds made it less visible). Add a
crisp, on-brand focus ring (cyan `outline` with an offset) for links, nav items,
and buttons so keyboard nav stays legible against the sharper surfaces.

### 3. [Tradeoff, accept] Google Fonts via CSS `@import`
Loading the now-actually-used fonts via a remote `@import` adds an external,
render-blocking request to `fonts.googleapis.com`. It is the correct fix for the
"fonts never loaded" bug and is fine for a GitHub Pages marketing site
(`display=swap` avoids FOIT), but `@import` is the slowest load path and there
is no shared `<head>` partial to host a `<link rel="preconnect">`. Accept for
now; a future optimization is self-hosting the woff2 files under `assets/` or
adding preconnect. Recorded in the CSS comment; no change required this task.

### 4. [Out of scope] Stale landing copy
The "Locks & turrets" feature card still describes the pre-v0.5.0 "angular
aim-assist cone" targeting, which deliberate radar locking replaced. That is a
copy-accuracy issue, not visual sharpening - file as its own task rather than
widen this one.

## Verdict

REQUEST_CHANGES: fix findings 1 and 2, then re-verify the build. 3 is an accepted
tradeoff; 4 becomes a follow-up task.
