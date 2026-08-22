# Release visuals: make v0.11.0 move across landing, news, and docs

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.11.0,docs,web,capture

Epic: `20260818-220812`.

Make the release visible before asking anyone to read its changelog. This task
owns the v0.11.0 presentation across the landing page, release post, wiki, and
creator documentation: prose, interactive explanations, capture placeholders,
deterministic capture examples, and the media that finally fills them.

## Owner direction - 2026-08-22

The release post is a release event, not an expanded changelog. It should feel
closer to a Factorio Friday Facts post: player-visible changes lead, mechanisms
and honest numbers explain them, and moving images prove them. The landing page
should also move rather than presenting six static feature cards.

Work text-first. Build the post with explicit media placeholders and working
widgets before creating new images or loops. That pass decides what each visual
must prove; capture starts only after the rendered story is coherent.

`tasks/20260818-220812/news-post-plan.md` is historical input, not the active
manifest. Its fixed list was stale before capture began. This task owns the
current story and derives the final capture manifest from the rendered post.

## Surfaces

### Release post

Rewrite `web/src/news/0.11.0.md` around a strong visual lead and a small number
of release stories, not the changelog's subsystem order. Each major story gets
one or more of:

- a WebM loop or image placeholder that states what the visual must prove;
- an interactive widget with useful static fallback prose;
- a sourced measurement or comparison when a number clarifies the change;
- a short remainder list for details that do not deserve a feature section.

Candidate stories, to confirm in the text-first pass:

- hull cracks deepening through eight damage levels, then sections severing;
- point-defence mounts dividing a salvo while Serpents weave through the fire;
- Kinetic punch versus Pierce rake;
- derived skins, four styles, and the greeble vocabulary;
- the editor's parts gallery, mating feedback, and live skin preview;
- close-range combat and the contextual HUD;
- structural performance wins, including rounds no longer being rigid bodies
  or colliders, plus the paired measurements that prove each claimed result.

Candidate interactives:

- an eight-level damage progression scrubber;
- a style and greeble explorer;
- Kinetic/Pierce travel and point-defence assignment comparisons;
- before/after performance counts and measured distributions;
- handling and frame-independent turret tracking graphs where they support the
  player-facing story.

Every `data-widget` block keeps static fallback prose. Every documented game
number comes from Rust and records its `file:line` in a nearby comment.

### Landing page

Replace selected static feature placeholders with short moving footage. The
editor, autopilot, locks and turrets, destruction, and contextual HUD are the
strongest candidates. Keep a still where motion adds no information.

Do not trade page performance for spectacle. One above-the-fold loop may load
eagerly; other loops load only near the viewport. Respect reduced motion and
retain an accessible still or first-frame experience.

### Wiki and creator documentation

Fill the already-authored capture slots, then sweep for prose that a visual
explains better. Reuse release media when it makes the same claim. Add a new
capture only when the page needs a distinct proof. Re-shoot captures invalidated
by the damage, carving, editor, or combat changes.

## Media contract

Use short WebM loops rather than literal GIF files: the existing pipeline is
smaller, sharper, and already supports autoplay and reduced motion. Loops are
muted, inline, deterministic, and at most 3 MB. Stills use the screenshot
pipeline. Every shipped asset has a committed capture example or a documented
reusable source so a later release can reproduce it.

Existing machinery:

- `figure__placeholder` names an asset and remains visible until it exists;
- `scripts/capture-web-media.sh` validates and packages WebM loops;
- `scripts/gen-web-screenshots.py` packages stills;
- `web/src/widgets.ts` hydrates sourced, progressively enhanced widgets.

Two loops already exist: `spine-cut.webm` and `torpedo-blast.webm`. Existing
screenshot examples should be reused before creating near-duplicates.

## Accepted news spine - 2026-08-22

The post follows the player-visible result, not subsystem ownership:

1. Ships come apart.
2. Every shot has a purpose.
3. Torpedoes attack; point defence answers.
4. Ships have a visual identity.
5. Build the ship you can see.
6. Flight and scenarios feel intentional.
7. A lighter battlefield.
8. More in v0.11.0, then the release call to action.

Only defects visible in v0.10.0 belong in the release story. A defect introduced
and fixed during this cycle never reached a player and is omitted. Instrument
failures and rejected optimisation premises remain in the epic record, not the
player post.

## Site-wide review direction - 2026-08-22

The visual pass now covers the landing page, current player wiki, creator docs,
and v0.11.0 news. Historical news is immutable. Use staged gameplay-authentic
media for landing/news, ordinary player actions for the wiki, and controlled
diagnostic scenes for creator contracts. Visual-first player prose may put its
mechanism behind an existing `details.explain` fold, but required actions stay
visible.

The full inventory, findings, capture families, and accepted implementation
decisions are in `visual-review.md`. The landing uses a close 1v1 WFC duel as
its hero and a 2v2 WFC fight in the combat row. Historical unresolved figure
slots are removed; missing archive-card thumbnails receive generated art.

## Sequence

1. Rewrite the news post with final text structure, media placeholders, and
   working widgets. Render and review it without making new media.
2. Derive the capture manifest from those placeholders and the landing/wiki
   slots that can reuse them.
3. Add or adapt deterministic screenshot examples and capture the assets.
4. Add moving media to the landing page with visibility-aware loading.
5. Fill or explicitly reject the remaining wiki/create slots.
6. Run the final release probe after capture examples stop changing.

## Phase 1 landed - 2026-08-22

The text-first release post is live at `/news/0.11.0/` and on the News index. It
follows the accepted eight-part spine and carries 12 figures: eight new loop
placeholders, three new still/comparison placeholders, and the existing
`spine-cut` loop. Eight interactive blocks support the story; four are new
(`damage-levels`, `point-defense`, `style-explorer`, and `battlefield-load`) and
four reuse sourced reference widgets.

No media was created. The placeholder descriptions now define the shots, so the
next step is to derive and review the capture manifest instead of inheriting the
superseded list.

## Done when

- The news post reads as a release feature, has a visual lead and Performance
  section, and explains major changes with placeholders, widgets, or media.
- The landing page contains reviewed moving footage without eagerly loading a
  page of autoplay videos.
- Authored wiki/create slots are filled or rejected with a reason.
- Every shipped visual has deterministic provenance and meets the media budget.
- Desktop, mobile, no-JS, and reduced-motion output has been opened and reviewed.
- `cd web && npm run ci` passes.
- The final release probe passes after the capture examples land.
