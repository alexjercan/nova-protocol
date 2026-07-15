# Review: dev wiki IA refactor + Extend-the-game guides

- TASK: 20260715-204358
- BRANCH: 20260715-204358-wiki-guides

## Round 1

- VERDICT: APPROVE

Out-of-context fact-check of the five new guides against the code (the main risk
with generated how-to content is fabricated file:line / RON). Result: unusually
accurate - all cited paths, enum variant sets, RON shapes, the nova_portal_gen
CLI, the validation gates, and the merge/overlay semantics check out; the two
RON snippets the author flagged as "constructed" (the `Conditional` filter and
`SetControllerVerb`) both serialize correctly. Slug parity verified: every
`](../slug/)` cross-link resolves to a real WIKI_DOC_PAGES + manifest entry, and
all 11 dev pages are assigned to one of the three new categories. Build + headless
render confirmed (sidebar groups, guide page with highlighted code + mermaid).
Two NITs, both fixed on the branch:

- [x] R1.1 (NIT) guide-author-scenario.md - `SetControllerVerb` described the
  verb set as `Goto / Lock / Orbit`, omitting the real fourth `FlightVerb`
  variant `Stop`. Fixed: note `Stop` exists but is never withheld.
  - Response: added the parenthetical.
- [x] R1.2 (NIT) guide-add-section.md - step 3 said the new damage class must be
  added to "the two class arrays in this file's tests", but only one test array
  iterates `SectionDamageClass` (the other iterates `DamageType`). Fixed to
  reference the single `SectionDamageClass` array.
  - Response: corrected to one array.
