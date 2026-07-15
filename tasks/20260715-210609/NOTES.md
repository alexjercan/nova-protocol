# NOTES - blog devlog posts HTML -> markdown

## What changed and why

Extend the markdown consolidation to the blog: the 5 devlog posts are now
markdown under `web/src/posts/`, rendered by a new `blogPostShell`. The blog
INDEX (`blog.html`) stays hand-authored HTML (it links the same post URLs).

## Pipeline (web/markdown.js + webpack.config.js)

- `blogPostShell(title, basePath, { date, version, description })` + `blogPostPage`
  render a STANDALONE `<article class="prose">` (no `.wiki` sidebar): the
  `prose__meta` line (back-to-devlog link + date + version), the h1, the
  `#doc-body` placeholder, and the shared `post-footer` (GitHub Discussions
  prompt + back-to-blog). Head carries the description meta; `<title>` suffix is
  " - Nova Protocol" (posts) vs " - Nova Protocol Wiki" (wiki). Uses the `post`
  chunk; reuses `renderMarkdownFile` and the `#doc-body` injection.
- `BLOG_POSTS` in webpack.config.js (slug, title, date, version, description),
  newest first, replaces the five `page("post", ...)` entries and their explicit
  history rewrites. Metadata was lifted from each post's original `<title>`,
  `<meta name=description>`, and `prose__meta` line.

## Conversion approach

Each post: took only the `<article class="prose">` inner content; dropped the TOP
`prose__meta`, the `<h1>`, and the `<footer class="post-footer">` (the shell
re-provides them). Kept as RAW HTML: `<figure>` placeholders, `<div
class="video-embed">` YouTube iframes + captions, inline `<kbd>`, and any IN-BODY
`<p class="prose__meta">` patch-note (devlog-4/-5 have a "vX.Y.1 cleaned up..."
note). `/play/` links rewritten to `../../play/`; external links untouched.

## Verification

`npm run ci` green; all 5 posts emit; headless render confirms the video embed
(devlog-2), figures + patch-note (devlog-5), meta line, and footer. Content
word-count parity is exact once the shell scaffolding (top meta + footer, ~30
words) is subtracted.

## Follow-up filed (user request)

Mid-cycle the user asked to document the entity-filter `id` / `other_id` /
`type_name` / `other_type_name` semantics (why two of each; e.g. OnEnter `id` is
the area, `other_id` the entity that entered) in the scenario docs. Filed as a
separate task with a grounded per-event spec.
