# Retro: devlog5-radar-stance-slots composite (stdlib PNG codec)

- TASK: 20260715-004216
- BRANCH: task/devlog5-stance-composite
- REVIEW ROUNDS: 1 (APPROVE, one NIT fixed in-round)

See TASK.md for what/why and REVIEW.md for the findings + the independent
verification. Process observations only below.

## What went well

- Picked the right approach off the menu. The task listed three options (guarded
  Pillow / hand-author / bespoke two-viewport capture); a fourth - a small stdlib
  PNG codec - was clearly better once weighed against the repo's stdlib-only
  convention and "correct over effort". No dependency, reproducible, and the
  determinism check (rebuild -> identical sha256) proved it commit-safe.
- The `--self-test` mode paid for itself: it caught nothing broken, but it makes
  the risky codec checkable in CI/isolation without a GPU capture, and writing it
  forced the decode/resize/compose contracts to be explicit.
- Eyeballing the first render changed the design. The literal spec ("scale each
  to half width") produced a 2:1 horizontal squish that looked bad; seeing it
  led to aspect-preserving contain-fit with black letterbox (invisible against
  space-black frames). Validating dimensions alone (1920x1080 RGBA) would have
  shipped the distorted version.

## What went wrong

- Nothing costly - a clean one-round cycle. The one NIT (R1.1: `decode_png`
  crashed on a corrupt PNG with `zlib.error` instead of the "report, don't crash"
  `ValueError` contract the earlier `png_dimensions` fix set) was a consistency
  miss: I hardened the encoder-adjacent `png_dimensions` last task but wrote the
  new `decode_png` without re-applying the same guard. Root cause: did not carry
  the just-established contract to the sibling function. Fixed in-round.

## What to improve next time

- When adding a sibling to a function whose error-handling contract was recently
  set, re-apply that contract deliberately (grep the last task's REVIEW for the
  established invariant).
- For any codec/serializer, verify the reverse against the spec independently -
  a round-trip test built on a self-authored forward pass proves symmetry, not
  correctness (a shared predictor bug cancels). Here the Paeth/Average reverse
  filters were re-derived against PNG spec 9.2 before trusting the round-trip.

## Action items

- [x] bumped `verify-the-nit-compiles` neighbour lesson set: added
  `render-output-eyeball` and `roundtrip-hides-shared-bug` to LESSONS.md.
- [ ] tatr 20260715-092658 (follow-up, OPEN): the 3 devlog thumbnails, deferred
  at the user's request (source choices not decided).
</content>
