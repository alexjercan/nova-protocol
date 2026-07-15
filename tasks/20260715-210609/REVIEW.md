# Review: convert blog devlog posts to markdown

- TASK: 20260715-210609
- BRANCH: 20260715-210609-blog-md

## Round 1

- VERDICT: APPROVE

Format-only conversion of the 5 devlog posts to markdown, rendered through a new
`blogPostShell` (standalone `.prose` article, no wiki sidebar). Verification:

- All 5 posts emit under dist/blog/; `npm run ci` green; the old post `.html` are
  removed and the five per-post `page("post", ...)` entries + explicit rewrites
  collapse to a `BLOG_POSTS` map. The `post` chunk (post.ts) is unchanged.
- Headless render parity: devlog-2 shows the meta line (date // version), the
  YouTube `video-embed` iframe, and the article; devlog-5 shows both figure
  screenshots (upgraded), inline `<kbd>`, and the in-body v0.5.1 patch-note
  (kept as a raw `<p class="prose__meta">`). Head `<title>` is "<h1> - Nova
  Protocol" and the description meta is preserved.
- Content parity: article word counts match once the shell-provided scaffolding
  is accounted for - new md is lower by ~27-32 words on each post, exactly the
  dropped top meta line + post footer ("Got a reaction..." + "All devlogs"), which
  the shell now renders. No prose lost.
- Links: `/play/` links rewritten to `../../play/`; external YouTube srcs and the
  Discussions link preserved verbatim; no leftover `<%= basePath %>` tokens.

Scope note: the blog INDEX (blog.html) stays hand-authored HTML as planned - it
links the same post URLs, which did not change. A data-driven index from
BLOG_POSTS is a possible later follow-up, not this task.

No blocking or non-trivial findings.
