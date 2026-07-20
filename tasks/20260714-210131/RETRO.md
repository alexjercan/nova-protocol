# Retro: Screenshot showcase pipeline (photo-mode actions + reel + web packaging)

- TASK: 20260714-210131
- BRANCH: task/screenshot-showcase-close (review-and-close pass; the feature
  itself landed earlier as commit 92aaf8da)
- REVIEW ROUNDS: 1 (APPROVE)

See TASK.md for what shipped and REVIEW.md for the findings. This retro is only
about how the review-and-close cycle went; the implementation cycle that
produced 92aaf8da predates this session.

## What went well

- Scoping the fork up front paid off. The task was IN_PROGRESS but its feature
  had already merged, with a stretch phase and a deferred-shots follow-up in
  play - three plausible meanings of "finish up". Asking once (review+close vs
  Phase 5 vs the 4 shots) before touching anything avoided building the wrong
  thing.
- Splitting the review by risk worked: read the engine diff (actions.rs,
  loader.rs, harness.rs) first-person because it is load-bearing, and fanned the
  lower-risk Python/TS/examples to a subagent. The subagent independently
  re-derived the PNG IHDR byte offsets instead of trusting the code comment,
  which is exactly the out-of-context skepticism a shared-session review needs.
- Fixing the cheap MINOR/NIT items on the open branch rather than filing them as
  a follow-up: three small hardening fixes landed with the review instead of
  rotting in a backlog.

## What went wrong

- R1.2 (wiki.ts icon flash): the first fix was a one-liner
  `img.onload = (): void => icon.appendChild(img)`. It failed `tsc` (TS2322:
  appendChild returns a node, not void, against the explicit `: void`
  annotation) and needed a block body. Root cause: assumed a two-line TS swap
  compiles by inspection. Only ran `npx tsc --noEmit` (after an `npm ci`)
  because the discipline said to, and it caught it.
- R1.1 (png truncation): the first version's comment asserted "callers collect
  ValueError into the failed list" - but `process_group` called `png_dimensions`
  with no try/except, so the new ValueError (and the pre-existing "not a PNG"
  one) would still crash the run. Root cause: wrote the fix's justifying comment
  from an assumed caller contract without reading the one call site. Grepping the
  callers turned a half-fix into a real one (add the try/except in
  process_group).

Both are the same shape: a review fix is a hypothesis, and both hypotheses
(this compiles / callers already handle this) were false until verified.

## What to improve next time

- Treat every review fix - even a "trivial" NIT or a comment - as unproven:
  compile/typecheck it, and when its rationale assumes caller/handler behavior,
  read that call site before writing the claim. This is the existing
  `verify-the-nit-compiles` lesson; this task is two more instances.

## Action items

- [x] bumped `verify-the-nit-compiles` in LESSONS.md (x1 -> x2), sharpened
  to cover the caller-contract variant.
- No follow-up code tasks: the 4 remaining shots (20260715-004216) and Phase 5
  attract mode were deliberately out of scope and remain tracked/optional.
</content>
