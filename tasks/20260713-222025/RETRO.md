# Retro: sharpen the web visual design (20260713-222025)

- DATE: 20260713-222025
- VERDICT: APPROVE (2 review rounds)

## What went well

- The spike paid off directly. Because tasks/20260713-221822/SPIKE.md had
  already enumerated the concrete "floaty" levers (radii, glow tokens,
  hover-float, backdrop-blur, gradient buttons, emoji cards), the work was a
  straight-line translation into a single cohesive `style.css` pass rather than
  a design debate mid-edit. Spike-before-flow is the right shape for a fuzzy
  visual ask.
- The spike's "are the fonts actually loaded?" open question surfaced a real
  latent bug: Rajdhani / Inter / JetBrains Mono were named in the font stack but
  never imported anywhere, so the site had silently rendered in system fonts the
  whole time. Fixing it is part of what makes the result "sharper".
- Review caught two genuine issues (the `[ // Systems online ]` double-decorated
  eyebrow, and missing keyboard focus states after glows/outlines were removed)
  and both were cheap to fix - the honest review round earned its keep.

## What went wrong / difficulties

- Shell cwd kept resetting to the worktree root between Bash calls, so `cd web
  && ...` chains silently ran from the wrong directory (one build "succeeded"
  against stale dist). Fix: use absolute paths / `git -C` and stop trusting a
  prior `cd` to persist.
- `web/node_modules` is gitignored and absent in a fresh worktree, so every
  `npm run ci` needed a `ln -sf` to the main checkout's modules first. Friction,
  not a blocker.
- The font fix introduced an external, render-blocking `@import` to Google
  Fonts. Accepted as a tradeoff (recorded in the CSS comment and REVIEW.md), but
  a self-hosted woff2 set under `assets/` would be strictly better and is worth
  a future task.

## Lessons

- `declared-but-not-loaded`: a resource named in config/markup (a font stack, an
  asset URL, a class hook) is not proof it is wired - grep for where it is
  actually imported/served before assuming it renders. Here a whole font stack
  had never been loaded.

## Follow-ups filed

- tatr 20260713-222824: fix the stale "angular aim-assist cone" targeting copy
  on the landing page (found while swapping the emoji cards; out of scope for a
  visual task).
- (Deferred, no task yet) self-host the webfonts instead of the Google Fonts
  `@import` to drop the external render-blocking request.
