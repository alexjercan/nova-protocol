# Retro: UI-rework spike - HTML demos (menu widget language + contextual HUD)

- TASK: 20260728-175726
- BRANCH: spike/ui-rework-demos
- REVIEW ROUNDS: 1 (APPROVE, out-of-context)

Process notes only; the accepted directions and rationale live in SPIKE.md +
DECISION.md, the demos in `examples/ui/`.

## What went well

- HTML-PoC-first with LIVE owner review + chromium eyeball each pass kept the
  design forks cheap: 3 iterations on demo 1, 2 on demo 2, each a screenshot and
  a question, no engine code risked. The phosphor-vs-hardware and "widgets must
  be CLI-rendered" forks were settled in minutes, not a build.
- Reading the REAL surface via Explore agents before (re)mocking paid off twice:
  the menu map (corner panel over live backdrop, real button set, AUDIO/GRAPHICS/
  CONTROLS settings, two-pane mods/scenarios) and the HUD map (tiers, `~` levels,
  every element's position/visibility/units) let the demos MIRROR the game
  instead of inventing shapes.
- Mid-flow owner requests (web easter egg, key-glyph adoption) were captured as
  their own prioritized tatr tasks (185730, 214929) instead of widening the
  spike branch - and the load-bearing forks in them (deploy scope, egg chrome)
  went back to the owner rather than being guessed.

## What went wrong

- The combat reticle shipped mis-anchored in demo 2 v1 (brackets flew to the
  screen corners): `.reticle` lacked `position`, so its absolutely-positioned
  bracket children resolved their containing block against the viewport. My
  composed screenshot did NOT catch it because the sibling DST/CLS chip filled
  the spot where the reticle should be - the OWNER caught it. Root cause:
  eyeballed the whole scene, not the sub-element's positioning context. Lesson
  `absolute-child-needs-a-positioned-ancestor` (sharpens render-output-eyeball).
- DECISION.md first shipped a bare prose `STATUS: ACCEPTED` line and failed
  `tatr check` (`bad-decision-status`). Root cause: authored the STATUS line
  freehand instead of copying an existing DECISION.md's frontmatter. This is the
  SECOND occurrence (ledger `decision-status-enum` x2) - a template/seed is now
  warranted.
- Demo 1 v1 assumed screen shapes (tabbed settings, centered menu, invented mods
  content) before the menu code was read; it took a full rewrite once the owner
  asked to mimic the real UI. Cheap here because the PoC is throwaway and review
  was live, but reading first would have skipped a lap.

## What to improve next time

- For CSS/DOM mocks: any element using absolutely-positioned children gets an
  explicit `position` on the container, and the sub-element is eyeballed in
  place - not just the composed scene.
- Copy an existing sibling artifact's header (DECISION.md frontmatter) before
  authoring a new one, rather than writing the machine-parsed fields freehand.
- When the ask is "mimic the current UI", read the current UI code FIRST, before
  the first mock.

## Action items

- [x] Bumped `decision-status-enum` to x2 with a template-seed proposal in the
  ledger; the lessons skill (flow Finish) decides promotion.
- [x] Added `absolute-child-needs-a-positioned-ancestor` to the ledger.
- [ ] tatr 20260728-185730 (web easter egg) and 20260728-214929 (glyph adoption)
  already filed as follow-ups from this spike.
