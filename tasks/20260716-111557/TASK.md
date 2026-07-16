# Merge blog + changelog into a unified /news/ section (feature-release posts, patches folded, 0.6.0 expanded)

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: web,docs,news

## Outcome

Merged `/blog/` + `/changelog/` into one Bevy-news-style `/news/` section. Added
`newsPostShell`/`newsPostPage` (markdown.js, footer carries both the Discussions
prompt and the CHANGELOG.md pointer); 6 merged posts under `web/src/news/`
(0.1.0-0.6.0), one per FEATURE release, each combining the devlog narrative +
structured highlights, with the YouTube video an optional in-body companion on
0.1.0/0.2.0 and a `## Point releases` fold for that cycle's patches
(0.2.0<-0.2.1; 0.3.0<-0.3.1; 0.4.0<-0.4.1; 0.5.0<-0.5.1+0.5.2). 0.6.0 expanded
into a full feature-by-feature writeup of the mod-portal arc. Added a `/news/`
card index + `news.ts` entry; retired the old sources (`web/src/posts`,
`web/src/releases`, `blog.*`, `changelog.*`, `post.ts`); collapsed nav to a
single "News"; updated the landing Devlog card and the dev-guide authoring
section; and emitted meta-refresh redirect stubs for every old `/blog/` and
`/changelog/` URL (patches -> parent feature post). `npm run build` green;
`/news/` + 6 posts emit; all 18 old URLs redirect.

Independent review APPROVE (REVIEW.md), no BLOCKER/MAJOR/MINOR: no fabrication,
no content lost from the retired sources, redirects/wiring/nav correct. One NIT
copy fix applied.

Posts were drafted by parallel subagents merging each devlog + release page from
a hand-expanded 0.6.0 exemplar, then independently reviewed.

## Goal

Kill the duplication between the devlog (`/blog/`) and the release notes
(`/changelog/`) by merging both into a single Bevy-news-style `/news/` section.
Research: tasks/20260716-102940/SPIKE.md (Bevy news follow-up in this task's
Notes).

Design (decided with the user, 2026-07-16):

- **One post per FEATURE release** (0.1.0, 0.2.0, 0.3.0, 0.4.0, 0.5.0, 0.6.0).
  Patch releases get NO post of their own.
- Each post MERGES the devlog narrative + the structured "what's new"
  highlights + the breaking-changes callout into one page. The YouTube video
  (only 0.1.0 and 0.2.0 have one) becomes an optional "also on video" companion
  embedded in-body, NOT the lead.
- **Patches fold into their parent feature post** as a "Point releases"
  section: 0.2.0 <- 0.2.1; 0.3.0 <- 0.3.1; 0.4.0 <- 0.4.1; 0.5.0 <- 0.5.1 +
  0.5.2; 0.1.0 and 0.6.0 have none.
- **Nav: "News" only**, replacing both "Blog" and "Changelog". The terse
  `CHANGELOG.md` (which keeps EVERY version incl. patches) stays the exhaustive
  machine reference and is linked from each post's footer.
- **Expand 0.6.0** into a full feature-by-feature writeup (it has no devlog, so
  the News post is the definitive writeup; it currently undersells the
  mod-portal arc).
- Retire `/blog/` and `/changelog/`; add redirects from old URLs
  (`/blog/`, `/changelog/`, `/blog/<devlog>/`, `/changelog/<version>/`) to
  `/news/` or the matching post.

## Steps

- [x] 1. Add `newsPostShell` + `newsPostPage` to `web/markdown.js` (merge of the
      blog + changelog shells): News crumb, date/version meta, body, footer with
      the Discussions prompt AND the CHANGELOG.md pointer + back-to-News.
- [x] 2. Write `web/src/news/0.6.0.md` as the expanded exemplar (feature-by-
      feature, mod-portal arc in full, breaking callout, no video/patches).
- [x] 3. Write `web/src/news/{0.5.0,0.4.0,0.3.0,0.2.0,0.1.0}.md`, each merging
      devlog-N + changelog-<ver> + folded patches, with the video companion
      where one exists.
- [x] 4. Add `web/src/news.html` index (card stream, one card per feature
      release, newest-first; video thumbnail for 0.1.0/0.2.0 else placeholder)
      and `web/src/news.ts` entry.
- [x] 5. webpack: register `news` chunk + index + NEWS_POSTS map + rewrites;
      remove blog/post/changelog registrations; emit redirect stubs for old
      URLs.
- [x] 6. Nav: replace "Blog" and "Changelog" with "News" in `_header.html` and
      `_footer.html`.
- [x] 7. Delete the superseded sources (`web/src/posts/`, `web/src/releases/`,
      `blog.html`, `blog.ts`, `changelog.html`, `changelog.ts`, `post.ts`) once
      their content is merged into news.
- [x] 8. `npm run build` green; `/news/` + `/news/<version>/` emit; old URLs
      redirect; spot-check 0.6.0 and 0.5.0 (video + point-releases).

## Notes

Spike: tasks/20260716-102940/SPIKE.md (Bevy migration-guide / Wesnoth research).
Follows up 20260716-102954 (the /changelog/ section this merges away).
Bevy /news/ research (this task): single newest-first card feed, feature
releases only (no patch cards), written-first posts with inline media and a
separate exhaustive changelog, video optional. See conversation + SPIKE context.
