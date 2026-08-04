# Retro: Move the design PoC HTML pages out of examples/ui into web/design

- TASK: 20260804-003301
- BRANCH: refactor/design-poc-to-web-design
- REVIEW ROUNDS: 1

## What went well

- The plan named its own risk ("worth confirming against the existing
  copy-plugin context") and the work resolved it the right way: read the config
  for a `context:` key, note the sibling `from: "src/assets"` is already
  cwd-relative, then prove it with a build that emits all three routes
  byte-identical to the sources. Reasoning from webpack defaults would have
  produced `../design/X.html` and a silently broken easter egg.
- The move was kept a pure rename. Putting `web/design/` outside `web/src/`
  means prettier, eslint and tsconfig globs never reach it, so
  `rename ... (100%)` x3 held without fighting a formatter.
- The Close-out volunteered its own known wart (the stale `(examples/ui/)`
  parenthetical inside `hud_rework_poc.html`) before review found it. The
  round-1 reviewer's only finding was that same item, at NIT.
- One review round, APPROVE, zero rework.

## What went wrong

- The plan encoded `POC_PATH = join(REPO, "web", "design", ...)`. That was a
  sound-looking edit at plan time - it is the literal old expression with the
  new segments - but it strands `REPO`, whose only reader was the old path.
  Following it verbatim would have left dead code plus an eslint unused-var
  warn and a pointless `web/../web` round trip. The worker caught it and
  deviated; the plan is the subject here, not the worker.
- NOTES.md's webpack sketch shows `from: "../design/X.html"`, the wrong value,
  while TASK.md Step 2 carries the correct `design/X.html`. Two records
  disagreeing on a load-bearing string is a trap for anyone who copies the
  sketch. NOTES.md is a scratchpad, so this did not block anything.
- The DoD prose says "no old path survives anywhere" while its own proof grep
  (`examples/ui/.*\.html`) cannot see the stale comment inside the moved HTML,
  and fixing that comment would break the 100%-rename proof. The two DoD
  criteria were written in mild conflict.

## What to improve next time

- Breadth: 17 files, but 14 are single-line path edits and 3 are pure renames.
  That is the inherent fan-out of relocating a file with many normative
  citations, not a missed split. No action.
- Churn: none to explain - the review approved on round 1. The one plan defect
  would have been caught by asking, at plan time, "which identifier builds this
  path, and does anything else read it?" A move plan enumerates strings; it
  should enumerate the variables that assemble them too.
- Context: this task crossed a context boundary mid-flow. The work phase
  committed in the sprout and advanced the record to REVIEWING there, while the
  main checkout still read WORKING; the resuming context advanced main's
  TASK.md before noticing, and had to revert it. When a sprout exists, resolve
  `<task-root>` to the worktree via `sprout ls` BEFORE calling `tatr flow`.
- Delegation: round 1 went to a fresh out-of-context reviewer, which
  independently re-derived the webpack-context claim. Worth repeating on any
  task whose correctness hinges on one path-resolution fact.

## Action items

- No follow-up task. The stale `(examples/ui/)` parenthetical in
  `web/design/hud_rework_poc.html:337` is recorded as R1.1 (NIT) and is
  deliberately deferred to whoever next edits that file's content, because
  touching it now costs the pure-rename proof.

## Landing message

```
refactor(web): move the design PoC HTML out of examples/ui into web/design

The three `*_poc.html` files were never examples - they are accepted design
sources with live consumers. `nova_ui_rework_poc.html`'s `:root` block is the
single origin for both `nova_ui/src/theme.rs` and `web/src/style.css`, guarded
by `web/tests/theme.test.ts`; webpack copies all three into the built site at
secret routes. They sit in a category about to get a "proves the live UI tree"
contract they can never satisfy.

Pure relocation: `git mv` x3 as 100% renames plus 14 reference updates across
the webpack copy entries, the theme drift test, seven rustdoc/script citations
and four docs. No HTML byte, token or style changed. `theme.test.ts` now
derives both sources from the `web/` cwd and drops the stranded `REPO`.
```
