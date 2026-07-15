# Retro: blog devlog posts HTML -> markdown

- TASK: 20260715-210609
- OUTCOME: shipped (landed d224cca9); review APPROVE.

## What went well

- The blog shell was a small addition on the pipeline the wiki work already
  built: a second shell function + a BLOG_POSTS config, no new machinery. The
  `#doc-body` injection and `renderMarkdownFile` were reused as-is.
- Raw-HTML passthrough again did the heavy lifting: YouTube `video-embed`
  iframes, figure placeholders, and the in-body `prose__meta` patch-notes all
  survived verbatim.
- Word-count parity was a clean check: new md was lower by exactly the
  shell-provided scaffolding (top meta line + footer, ~30 words), which
  positively confirms nothing else was lost rather than just "close enough".

## What went wrong

- Nothing. The two conversion agents handled the video embeds and patch-note
  prose__meta correctly on the first pass (the tuned prompt calling out "keep
  in-body prose__meta, drop only the top one" paid off).

## Lessons

- `parity-delta-should-be-explainable` (positive, x1): when a mechanical
  conversion changes a word/line count, the delta should equal a NAMED cause
  (here: the shell now owns the meta line + footer). An unexplained delta is a
  dropped-content signal; an exactly-accounted delta is proof of fidelity.
  20260715-210609.
- Reinforces the markdown-consolidation pattern: one pipeline (markdown-it +
  raw-HTML passthrough + a per-surface shell) now renders the wiki dev pages,
  the wiki player pages, and the blog - three shells over one renderer.
