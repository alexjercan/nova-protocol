# Retro: News section TOC sidebar + exhaustive post expansion

- TASK: 20260716-114245
- BRANCH: changelog-revamp
- REVIEW ROUNDS: 1 (APPROVE, one MINOR fix applied)

## What went well

- Reusing the existing markdown-it-anchor ids made the TOC correct by
  construction: the sidebar link `#id` and the in-body heading id come from the
  same token attr, so they can never drift apart regardless of slugify details.
  Building the TOC at config time (in the shell) kept it no-JS/SEO friendly, with
  news.ts adding only the scroll-spy highlight on top.
- The two asks reinforced each other: expanding posts added h2/h3 subsections,
  which directly enriched the TOC sidebar (0.4.0 went to 33 entries). One task
  served both goals.
- Exemplar-consistent fan-out again produced six deep posts in parallel, and the
  out-of-context fact-check caught the one invented detail among thousands of
  words - exactly the failure mode verbose LLM prose is prone to.

## What went wrong

- The one review finding was a fabrication: the 0.5.0 expansion invented
  per-damage-type behaviors (AP vs plating, EMP vs systems, Explosive spread)
  that no source states - a plausible gloss of the type names. Root cause: an
  instruction to "be verbose / cover the nuts and crannies" pushes a drafter to
  fill gaps with plausible detail; the CHANGELOG named the four types but not
  their behavior, and the drafter invented the behavior to have more to say.

## What to improve next time

- When instructing a drafter to be MORE verbose, pair it with an explicit
  "verbosity comes from the sources, not from plausible invention - if a source
  names a thing without describing its behavior, name it without describing its
  behavior". The completeness push and the anti-fabrication guard must travel
  together, and the fact-check reviewer must see the exact sources.

## Action items

- [x] Recorded `verbosity-invites-fabrication` in docs/LESSONS.md.
- [x] Applied the review fix (0.5.0 damage-type flavor reduced to name-only).
- [ ] Screenshots for the figure__placeholder slots (now more of them across the
  6 expanded posts) still need capturing.
