# Retro: markdown-first developer wiki

- TASK: 20260715-195621
- OUTCOME: shipped (landed ea5edab4); review APPROVE round 2.

## What went well

- Vertical slice first: wired the markdown pipeline and proved it with a single
  build before doing the bulk content move, so the risky part (webpack/template
  wiring, basePath) was settled early and cheaply.
- Fanned the 6 doc conversions out to parallel subagents on distinct files with
  a shared rubric; no collisions, and each added faithful mermaid diagrams from
  its own doc's content.
- Actually ran the client render (headless chromium screenshots), per
  `ci-skips-client-render` / `render-output-eyeball`: confirmed all three diagram
  types render to SVG, code highlights, and the sidebar shows the new
  categories - none of which a green build proves.
- Verified base-path behavior at the real deploy subpath (`PUBLIC_PATH=
  /nova-protocol/`), per `verify-at-deploy-base-path`, catching nothing broken
  because basePath was inlined at config time - but the check is what makes that
  a fact rather than a hope.

## What went wrong

- The reference sweep used a `docs/`-PREFIXED grep, so it missed bare
  `[name.md](name.md)` intra-doc links (which 404 once served under
  `/wiki/dev/...`) and a `docs/`-referencing mod README that the prefix grep
  *did* match but got lost in the results. The out-of-context review caught both
  (two MAJORs). Lesson below.
- Two throwaway shell slips: an early fish `set VAR` didn't expand under the
  bash wrapper (used literal paths after), and a failed multi-line script still
  executed its trailing `rm`, deleting the main-checkout TASK.md copy (recovered
  from context). Prefer per-file paths and avoid trailing destructive lines in
  scripts that can partially fail.

## Lessons

- `sweep-symbol-not-path-prefix`: when a file MOVES, sweep the repo for the
  bare filename/stem and for MARKDOWN-link forms `[x](x.md)`, not just the old
  `dir/x.md` path - a path-prefixed grep misses relative links and renamed
  targets. (Sharpens `sweep-then-delete`: grep the symbol broadly, including the
  link syntaxes it appears in.) 20260715-195621.
