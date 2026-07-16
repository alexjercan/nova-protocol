# News posts: add a sticky section TOC sidebar and expand every post to exhaustive detail

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: web,docs,news

## Outcome

Both asks shipped. (1) A sticky section TOC sidebar on every news post: h2/h3
headings extracted in `renderMarkdownFile` (with the markdown-it-anchor slug
ids), rendered as a `.news__toc` aside in a two-column `.news` grid by
`newsPostShell` (fallback to plain `.prose` when a post has no sections), with
IntersectionObserver scroll-spy in `news.ts` highlighting the active section.
(2) All 6 posts expanded to exhaustive detail - every CHANGELOG bullet (feature
+ folded patches) covered with its specifics and many more h2/h3 subsections
(0.4.0 344 lines / 33 TOC entries, 0.5.0 305 / 29, 0.6.0 293 / 22, others
14-21), which also enriches the sidebar. Build green; TOC slugs match in-body
anchors; ASCII clean.

Independent review APPROVE (REVIEW.md): no fabrication (every specific traces to
CHANGELOG or the pre-merge devlogs), infra/scroll-spy/CSS all correct. One MINOR
invented-flavor fix applied (0.5.0 damage types).

Posts expanded by parallel subagents from CHANGELOG + pre-merge devlogs in git
history, then independently fact-checked.

## Goal

Two user asks (2026-07-16), following the /news/ merge (20260716-111557):

1. **Section TOC sidebar** on each news post, like Bevy news - a sticky list of
   the post's sections so a reader can jump/scroll fast, with the current
   section highlighted as you scroll.
2. **Exhaustive posts** - "even more verbose, cover all the nuts and crannies".
   Every CHANGELOG bullet for the release (and its folded patches) gets real
   prose with its specifics (numbers, file paths, rev hashes, mechanisms); more
   h2/h3 subsections (which also enrich the TOC); more figure placeholders.

The two reinforce each other: more sections => a richer sidebar.

## Steps

- [x] 1. Extract h2/h3 headings (with the markdown-it-anchor slug ids) in
      `renderMarkdownFile` (web/markdown.js) and return them alongside html/title.
- [x] 2. Render a sticky `.news__toc` sidebar in `newsPostShell` from those
      headings, in a two-column `.news` grid beside the `.prose` article
      (mirrors the wiki layout); fall back to plain `.prose` when a post has no
      sections. Thread `headings` through `newsPostPage`.
- [x] 3. CSS: `.news` grid + sticky `.news__toc` + `.is-active` highlight +
      responsive stack (web/src/style.css).
- [x] 4. Scroll-spy in `web/src/news.ts`: IntersectionObserver toggles
      `.is-active` on the TOC link for the section in view (no-ops on the index).
- [x] 5. Expand all 6 posts (web/src/news/0.1.0.md .. 0.6.0.md) to exhaustive
      detail: every CHANGELOG bullet covered, more subsections, more figures,
      keeping the video companion / breaking callout / Point releases structure.
      (Fanned out to per-post subagents from the CHANGELOG + pre-merge devlogs in
      git history.)
- [x] 6. `npm run build` green; TOC renders on every post with slugs matching the
      in-body anchors; scroll-spy works; ASCII clean; spot-check a couple posts.

## Notes

Follows up 20260716-111557 (the /news/ merge). TOC anchors reuse the existing
markdown-it-anchor ids (levels [2,3]); the TOC is built at build time (no-JS /
SEO friendly) with news.ts adding scroll-spy on top.
