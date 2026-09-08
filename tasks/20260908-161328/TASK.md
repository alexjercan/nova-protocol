# Develop the shared season-one story and comic visual drafts

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.13.0,story,comic

## Goal

Develop one coherent season-one story for Nova Protocol's mainline campaign and
comic. Use visual drafts to test the opening, then refine the shared events,
characters, dialogue, and presentation. Keep this request and its follow-up work
in one task.

The current release tag schedules this development work. It is not a promise to
ship the whole comic season or a playable campaign in that release.

## Maintained sources

- [Shared season-one production outline](../../web/src/lore/seasons/season-1.md).
- [Lore ownership and publication rules](../../web/src/lore/README.md).
- [Public encyclopedia source](../../web/src/lore/index.md).
- [Reusable illustration library](../../scripts/nova_illustration/README.md).

Keep approved world facts and shared continuity in those sources. Keep draft
artwork, page experiments, scripts, working dialogue, reviews, and proof here.
Do not create a second maintained story specification in the task.

## Current direction

- Comic and campaign share major events, decisions, and outcomes. The player
  operates the whole ship; comic interiors do not imply on-foot gameplay.
- The opening follows Kaveri's competent crew. Baikal's industrial line stops
  safely; Kaveri collects a replacement at Aquila and later diverts to rescue
  Gantry. No opening combat, pirate cutaway, or pursuit is added.
- The four-page PoC stops at departure from Baikal. It is not an episode
  boundary, released comic, or implementation of a campaign mission.
- The user likes the visual direction: broader color, green accents, warm
  interiors, and cool space, not an all-green filter. Individual designs,
  apparent ages, clothing, geometry, and small staging details remain concepts.
- Expand dialogue beyond the first sparse pass. Show professional judgment and
  working relationships without a lore lecture, a crew roll call, or tutorial
  instruction for an inexperienced captain.
- Add cinematic-style where/when/context cards at the first Baikal interior and
  the move aboard Kaveri. The game action has `location`, `date`, and `note`
  strings plus placement/duration. This is a visual analogy, not a new runtime
  contract or a copy of gameplay values.
- Keep the clean-line trial's zoom, frames, camera placement, backgrounds, necks,
  and body staging. The user rejected its angled faces; restore the original
  forward-facing portrait treatment with light shadows and linework.
- Keep the ship-sheet layout and green lore shader. Revise the hulls toward the
  game's faceted plating and industrial greebles, using local screenshots and
  recipes as references, not fictional dimensions or game-ship identities.
- Reuse one proposed design per ship across scenes and lore sheets. Keep
  portrait poses reusable, and all color constants in `colors.py`. Use full
  color for comic art and a color-only green treatment for lore. Keep incidental
  props local to a scene; only recurring props belong in a separate shared module.
- Baikal's precise orbit or moon association and the calendar date are not
  chosen. Do not turn the user's example wording into canon. Draft temporal
  labels must be visibly provisional until those decisions are made.

## Artifacts

- [Working opening PoC](comic-opening-poc/index.html).
- [PoC scope and regeneration](comic-opening-poc/README.md).
- [Clean-line style trial](clean-line-study/index.html), with shared-source ships
  and a Lore colors preview.
- [Style, library, palette, and reference review](STYLE-REVIEW.md).
- [Review and verification record](REVIEW.md).
- `proof/` holds inspected images and verification evidence, including the
  visual baseline before the dialogue/card revision.

The generator and its generated HTML/SVGs move together. Later artifacts for
this story work stay in this task. Do not move unrelated design studies:
`nova_ui_rework_poc.html`, `hud_rework_poc.html`, and
`nova_os_terminal_poc.html` are live website build inputs.

## Scope and constraints

1. Move the opening comic PoC out of `web/design/` without changing the public
   archive, website copy rules, or unrelated design files.
2. Expand the four-page dialogue and add readable location/time/context cards.
   Keep dialogue transcripts and source-generated art consistent.
3. Review color, composition, lettering, pacing, and character staging at useful
   reading sizes. Keep small-screen transcripts and full-size SVG inspection.
4. Resolve only story and visual choices that the next draft needs. Do not
   invent missing ship specifications, a calendar, an orbital map, or biographies
   for completeness.
5. Keep the production outline unpublished. Public subject histories follow
   released comic or campaign events, with spoiler scope and release links.
6. Do not edit wiki pages, gameplay code, unrelated media, or other tasks. No
   full workspace Rust checks, automatic publication, or commit is implied.
7. Durable documentation must not link to task artifacts. Use an active
   `TODO(<task-id>)` where a work pointer is useful, not a replacement task URL.

## Acceptance

- [x] Store the opening PoC source and generated artifacts under this task.
- [x] Preserve the user's approved visual baseline as task evidence.
- [x] Expand the opening dialogue and review its lettering and reading order.
- [x] Add Baikal and Kaveri cards without inventing a moon or calendar year.
- [x] Verify regeneration, local preview, source links, and publication isolation.
- [x] Obtain feedback on the revised dialogue and title-card treatment.
- [x] Create a three-shot clean-line trial without replacing the earlier pages.
- [x] Use shared ship/portrait sources, palette constants, and color-only schemes.
- [x] Add explicit Kaveri/Ebro design proposals and Elena's portrait to lore.
- [x] Obtain feedback on the clean-line trial and shared ship forms.
- [x] Review restored frontal faces and revised industrial plating in the same shots.
  The user accepted this pass as the working visual direction.
- [ ] Choose the calendar/date convention and any location precision needed by
  the opening before treating those labels as established story facts.
- [ ] Continue toward an approved opening script/storyboard and campaign brief
  without creating contradictory histories or a second novel.

## Intake

This task was requested after review of the four-page PoC. The visual work and
shared outline already existed; this task collects their continuing development
rather than claiming they were created during task intake. The initial PoC was
verified before intake, but each changed version needs its own checks.

Intake revision: `e3291f373a65a6da81786e749f8c56193956abfa`.
