# Retro: creator docs journey (20260715-224030)

## What shipped

The creator journey is now complete and honest: guide-author-scenario.md section
6 ends with the whole objective loop as one runnable `*.content.ron` file; a new
guide-author-section.md documents the `Section` item and every `SectionKind`
(hull/thruster/controller/turret/torpedo) with field-level grounding; section 7
leads with the real no-Rust launch route; guide-make-a-mod.md cross-links both
authoring guides before the demo mod; and webmods/gauntlet/ got a copy-me README.

## Decisions

- Section reference as a NEW page (guide-author-section.md), not inlined into
  guide-make-a-mod. It documents five SectionKind variants with full field lists
  - guide-sized and parallel to the sibling creator guides - so inlining would
  bury a reference inside a task walkthrough. Registered in the "Scenarios &
  mods" band, mirroring the sibling dev-guide entry exactly (dev/ slug, no
  crumbParent, WIKI_PAGES + WIKI_DOC_PAGES).
- Ship-free starter scenario on purpose: every line is readable, and the page
  points at asteroid_field.content.ron as the clone-me full example whose
  `player_spaceship` block can be lifted verbatim. Trades self-containment for
  legibility, which is the right call for a teaching example.

## The premise that was stale (the important one)

The task's step 3 was written against "there is NO pure-RON way to boot your own
scenario; the picker is a tracked-OPEN future task." Between planning and
implementation the Scenarios picker (20260715-200828) SHIPPED and CLOSED. Writing
the doc to the task text verbatim would have shipped a wiki page that was wrong
on the day it landed. Catching it needed a status check of the referenced task
plus a read of the current menu code (`listed_scenarios` filters `GameScenarios`
by `!hidden`; `on_scenario_play` sets the New Game handoff), not just trust in
the task. Lesson: when a task references another task as "OPEN/tracked", re-check
that task's status before writing prose around it - plans go stale between
planning and doing, and docs that encode a stale plan are worse than no docs.

## What went well

- Field-name accuracy held up under review: every `*SectionConfig` field in the
  reference was cross-checked against the struct (including the two optional
  turret fields the example omits), and the assembled scenario's expression AST
  - the asymmetric `Add(Factor(...), Term(...))` that looks wrong - is verbatim
  the shipped grammar. Grounding every shape in a real file paid for itself.
- The out-of-band verification (diff the doc RON against the shipped
  `*.content.ron` line by line) is the right substitute when there is no cheap
  per-snippet RON validator; it caught nothing wrong, which is the point.

## What to do differently

- No cheap way to actually parse the doc's RON snippets exists, so correctness
  rests on line-by-line diffing against shipped files. A tiny dev tool that
  extracts fenced ```ron blocks from the wiki and runs them through the real
  `ron` + serde decode would turn "I diffed it carefully" into a test. Worth a
  future task if RON-in-docs keeps growing.
