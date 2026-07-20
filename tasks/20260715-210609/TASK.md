# Convert blog devlog HTML posts to markdown (blog-post shell)

- STATUS: CLOSED
- PRIORITY: 43
- TAGS: feature, docs, web

Part of the "unify the site on markdown" goal (with 20260715-205825). User wants
future devlogs authorable in markdown. Depends on 20260715-205825 landing first
(shared pipeline files markdown.js / webpack.config.js).

## Scope

The 5 devlog posts under `web/src/posts/devlog-*.html` (share the `post` chunk).
The blog INDEX `web/src/blog.html` stays HTML (hardcoded cards linking the same
post URLs - no URL change), so this task is posts only. A data-driven index is a
possible later follow-up, not here.

## What a blog post needs beyond the wiki docShell

Per-post content to preserve: the `prose__meta` line (`<- Devlog // DATE //
vX.Y.0`), the h1, h2/h3 sections, inline `<strong>/<em>/<code>/<kbd>`, lists,
`<figure class="figure">` placeholders (site.ts upgrades them), YouTube
`<div class="video-embed"><iframe ...>` embeds + `.video-embed__caption`, and the
`<footer class="post-footer">` (GitHub Discussions link + back-to-blog). Head:
`<title>` + `<meta name="description">`. No OpenGraph/JSON-LD today - preserve
current behavior (do not add or drop meta).

## Pipeline (web/markdown.js + webpack.config.js)

1. Add a `blogPostPage({ slug, mdPath, title, date, version, description })` +
   `blogPostShell(...)` alongside the wiki ones. The shell renders a STANDALONE
   `<article class="prose">` (no `.wiki` sidebar layout): the `prose__meta` line
   (back-to-devlog link + date + version, basePath inlined), `<h1>`, the
   `#doc-body` placeholder (rendered markdown), and the `post-footer` (the
   hardcoded Discussions URL + back-to-blog). Head gets the description meta.
   Reuse `renderMarkdownFile` (leading `# Title` stripped into the shell h1) and
   the existing `#doc-body` injection in webpack-partials.js.
2. Keep figures/kbd/video embeds as RAW HTML in the markdown (html: true passes
   them through) - the video embed block and figure placeholder divs paste in
   verbatim.
3. A `BLOG_POSTS` list in webpack.config.js (slug, md, title, date, version,
   description) mapped to `blogPostPage`, replacing the five `page("post", ...)`
   entries; keep the per-post `historyApiFallback` rewrites (or map them off
   BLOG_POSTS). `post.ts` stays the `post` chunk entry (initSite()).

## Steps

- [x] Add `blogPostPage` + `blogPostShell` to markdown.js; extend the config with
      `BLOG_POSTS`; wire the rewrites off it.
- [x] Convert the 5 devlog posts to `web/src/posts/devlog-*.md` (article content
      only; meta/h1/footer come from the shell; figures + video embeds as raw
      HTML; relative cross-links). Capture each post's date/version/description
      from its current `prose__meta` + `<meta>` into the BLOG_POSTS entry.
- [x] `git rm` the old post HTML; remove the dead `page("post", ...)` entries.
- [ ] Verify: `npm run ci` green; serve + headless-eyeball a converted post
      (meta line, figures upgrade, YouTube embed renders, footer + back-to-blog,
      code/kbd), the blog index still links correctly; check at the deploy
      subpath. Confirm the built post HTML matches the old one's structure
      (title, description meta, article classes).

## Notes

Pure consolidation, no visible change. The blog index stays HTML. Preserve the
exact `.prose`/`.figure`/`.video-embed`/`.post-footer` classes so shared CSS and
site.ts (figure upgrade) keep working. Do not invent OG/JSON-LD.
