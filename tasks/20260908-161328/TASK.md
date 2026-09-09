# Write the shared season-one story

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.13.0, story, comic

## Goal

Write the complete first-season story for Nova Protocol's mainline campaign and
comic, from the approved opening through the season ending. Develop the causal
sequence, character decisions, scenes, and dialogue as one shared continuity,
not a separate novel. Keep this request and its follow-up work in this task.

The user accepted the four-page visual direction and Baikal card placement.
Story writing is now the main work; more illustration trials are not a
prerequisite. The season story itself is not yet complete or approved.

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

## Current batch: episode-one script and reader structure

The user approved organizing `/story/` as a series landing, season contents,
and episode reader. Comic publication is separate from campaign availability;
unreleased scripts stay private. A normal episode targets 12-16 landscape
pages, with room for a longer opening that completes the rescue and homecoming.

The user accepted the eighteen-page opening arc, with clarity and staging
revisions. The opening is in 2078. Page 2 introduces Samir checking residential
supplies while Leila examines the pump; the original dialogue and coffee/window
beats remain. Leila inspects the replacement in person at Aquila. The funding
refusal is simpler, and the docked evacuation is staged in freefall. Gantry's
before/after introduction and first departure remain; timing stays qualitative.
The user accepted the revised opening, Gantry's intact/stranded model, and
Nadia/Owen/Ivo as working likenesses. They then chose the pressurized,
non-rotating transfer hall for Aquila: handholds, restrained cargo, exterior
berths, and a freight lock feeding Kaveri's external cradle. Pages 5-7 now have
private artwork. The complete script review includes seven illustrated pages;
pages 8-18 remain script. The user accepted these pages and chose the real
website reader as the working preview, with a featured current season and
ordered episode lists. Maintained page/script sources now live under
`web/src/comics/season-1/episode-1/`. Normal local serving includes the draft;
deployable builds exclude it. The user requested this single source home and
removal of the demo. Task-local page boards and their `.txt` source
snapshots are frozen evidence, not parallel readers. The wider season and its
critical choices are incomplete.

The approved authoring refactor gives every page a TS script and leaves Python
with named panel artwork only. The shared engine owns composition, lettering,
transcripts, and exports. Panels and Transcript now use compact toolbar buttons
and modals; the user removed Layout checks UI. All seven page exports remain
pixel-identical to the accepted reader output. No server was started.

The user clarified the browsing hierarchy: Story lists seasons only, and each
season opens a separate episode list. There are no duplicate reading shortcuts
or repeated season covers. Story returns to the library; the reader returns to
its episode list through the season title. This replaces the grouped-library
iteration. [Season browser review](SEASON-BROWSER-REVIEW.md) records the checks.

- [Maintained episode-one script](../../web/src/comics/season-1/episode-1/episode.ts).
- [TS/panel refactor and modal review](DSL-REVIEW.md).
- [Real-reader authoring workflow](../../web/src/comics/README.md),
  [initial reader review](READER-REVIEW.md), and
  [source consolidation review](SOURCE-REVIEW.md).
- [Frozen complete script review](episode-1/index.html), from before the
  real-reader migration.
- [Episode and archive review](EPISODE-REVIEW.md).
- [Approved feedback and revised-opening review](EPISODE-FEEDBACK.md).
- [Gantry and crew design board](gantry-study/index.html) and
  [art review](GANTRY-REVIEW.md).
- [Aquila pages 5-7](aquila-pages/index.html) and [batch review](AQUILA-REVIEW.md).

## Writing plan

1. Develop the existing season progression into a complete causal outline.
   Connect its major turns through motivations, decisions, and consequences.
   Review unresolved choices instead of silently treating proposals as canon.
2. Break the whole season into scenes. Give each scene a viewpoint, active
   people and ships, purpose, decision or change, and consequence for the next
   scene. Propose episode and mission boundaries after the sequence works.
3. Draft scene action and dialogue for the comic. Show professional judgment,
   distinct voices, and relationships without lore lectures or crew roll calls.
   Keep scripts with the comic sources, proof with this task, and approved shared
   events in the production outline; do not maintain competing story accounts.
4. Develop the corresponding campaign story brief. Map the same events to
   whole-ship objectives and narrative cues. Identify interaction needs without
   claiming that unverified mechanics are implemented.
5. Review continuity, character arcs, setups and payoffs, ship whereabouts,
   travel and rescue logic, and the credibility of the ending. Resolve only the
   dates, locations, biographies, and technical details the story needs.

## Story constraints

- Comic and campaign share major events, decisions, and outcomes. The player
  operates the whole ship; comic interiors do not imply on-foot gameplay.
- The opening follows Kaveri's competent civilian crew through useful work and
  a costly noncombat rescue. Keep the approved sequence in the production
  outline. No opening combat, pirate cutaway, or pursuit is added.
- Establish recurring people, ships, and communities through work and
  dependencies before loss. Name deliberately authored ships. Hull loss and
  crew survival are separate decisions.
- Preserve the distinction between approved direction and open choices in the
  production outline. Do not invent completeness or technical specifications.
- The four-page draft stops at departure from Baikal. It is not an episode
  boundary, released comic, or implementation of a campaign mission.

## Visual baseline and later work

- Keep the accepted frontal heads, close framing, body staging, light shadows,
  fine linework, and game-inspired industrial plating. The rejected angled
  faces are not the default for later drafts.
- Comic art uses broader color with green accents. Lore uses the color-only
  green treatment. Shared sources own recurring characters, ships, locations,
  and the palette; incidental props stay local.
- Baikal's orientation card belongs on its first exterior picture. Kaveri's
  stays at the move aboard. The approved year is 2078 on Earth's calendar;
  day, month, and precise orbital relationships remain open.
- The user accepted the page-1 expression trial and requested its commit.
  Rina uses an amused expression and Jonah a wry half-smile for the coffee
  exchange. Original defaults and other appearances are unchanged.
  Further expression work is not a prerequisite for writing the season.
- Visual acceptance does not establish dimensions, capacity, physical livery,
  ages, biographies, or final character likenesses.

## Artifacts

- [Working opening PoC](comic-opening-poc/index.html).
- [PoC scope and regeneration](comic-opening-poc/README.md).
- [Clean-line style trial](clean-line-study/index.html), with shared-source ships
  and a Lore colors preview.
- [Style, library, palette, and reference review](STYLE-REVIEW.md).
- [Expression comparisons](expression-study/README.md) and
  [expression trial review](EXPRESSION-REVIEW.md).
- [Review and verification record](REVIEW.md).
- [Requested commits](COMMIT-REVIEW.md) and [four-page restyle review](RESTYLE-REVIEW.md).
- [Frozen previous four-page version](proof/before-clean-line-opening/index.html).
- `proof/` holds inspected images and verification evidence, including the
  visual baseline before the dialogue/card revision.

The generator and its generated HTML/SVGs move together. Later artifacts for
this story work stay in this task. Do not move unrelated design studies:
`nova_ui_rework_poc.html`, `hud_rework_poc.html`, and
`nova_os_terminal_poc.html` are live website build inputs.

## Scope and constraints

1. The opening artwork and reusable-expression work are committed. The current
   request authorizes the season/episode website structure and first-episode
   script. It does not authorize publishing the draft or implementing missions.
2. Preserve the accepted studies, comparison copies, and evidence. New scripts
   and reviews stay with this task. Keep generated art and its sources together.
3. Keep the production outline unpublished. Public subject histories follow
   released comic or campaign events, with spoiler scope and release links.
4. Do not edit wiki pages, gameplay code, unrelated media, or other tasks.
   Writing a campaign brief does not authorize implementing its mechanics.
   No automatic publication or release is implied.
5. Durable documentation must not link to task artifacts. Use an active
   `TODO(<task-id>)` where a work pointer is useful, not a replacement task URL.

## Writing acceptance

- [ ] Complete and review the whole-season causal outline in the maintained source.
- [ ] Draft the season's scene sequence and propose episode/mission boundaries.
- [ ] Draft and review scene action and dialogue with distinct character voices.
- [ ] Prepare a campaign story brief that preserves the same major events and outcomes.
- [ ] Check character arcs, setups/payoffs, logistics, ship continuity, and the ending.
- [ ] Resolve story-critical open choices; keep unnecessary detail explicitly open.
- [ ] Obtain approval of the complete season story and record decisions in the
  maintained production outline without creating a separate novel.

## Completed visual groundwork

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
- [x] Commit the reviewed library, lore, shared story, drafts, and evidence.
- [x] Apply the accepted style to the four pages; preserve the prior version.
- [x] Check unchanged dialogue/cards, regeneration, rendered pages, and isolation.
- [x] Obtain feedback on the restyled four-page staging.
- [x] Put Baikal's unchanged card on the first exterior picture and review it.
- [x] Add reusable Rina/Jonah expressions and obtain approval of the page-1 trial.

## Intake

This task was requested after review of the four-page PoC. The visual work and
shared outline already existed; this task collects their continuing development
rather than claiming they were created during task intake. The initial PoC was
verified before intake, but each changed version needs its own checks.

Intake revision: `e3291f373a65a6da81786e749f8c56193956abfa`.
