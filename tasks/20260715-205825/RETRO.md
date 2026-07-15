# Retro: player wiki HTML -> markdown

- TASK: 20260715-205825
- OUTCOME: shipped (landed 0aee0d90); review APPROVE.

## What went well

- The pipeline extension was tiny (an optional description + a two-level crumb),
  because the dev-page conversion already generalized most of it - reuse paid off.
- Raw-HTML passthrough (html: true) made the risky bits trivial: the control
  tables with PromptFont gamepad glyphs, `<kbd>`, and figure placeholders all
  carried over verbatim, so rendering is byte-faithful.
- Headless render parity was the deciding check (ci only proves it compiles):
  saw the keybinds tables, the sections child grid, and the child crumb actually
  render before landing.

## What went wrong

- One of four parallel conversion agents dropped the `../` prefix on cross-wiki
  links (`](flight-autopilot/)` instead of `](../flight-autopilot/)`), which
  would 404, plus one `../tutorial/` that needed `../../tutorial/`. Caught by a
  link sweep, not the agent's self-report. Relative-path rewriting is the part of
  an HTML->markdown conversion agents get wrong most.

## Lessons

- `verify-relative-link-depth-after-conversion` (x1): when agents convert pages
  to relative links, always sweep the output for links that are NOT `../`-prefixed
  or external, and count `../` depth against each page's directory - one agent
  silently emitted root-relative links that 404 under `/wiki/<slug>/`. Cheap grep,
  catches a whole class of dead links a build never flags. 20260715-205825.
- Reinforces `render-output-eyeball`: a green `npm run ci` says nothing about the
  client-rendered tables/grid/crumb - the headless screenshot is what confirmed
  parity.
