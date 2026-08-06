# Retro: Generated placeholder thumbnails for the Scenarios picker

- TASK: 20260715-220011
- BRANCH: feat/scenario-thumbnails
- REVIEW ROUNDS: 2

## What went well

- Reusing `gen-web-screenshots.py`'s PNG codec instead of writing a second one
  was cheap and paid twice: splitting `write_png` into `encode_png` + a file
  wrapper is what let `--check` compare in memory, which is also how the
  coverage report tells a placeholder from real art with no marker file.
- Byte-identity as the placeholder marker. No sidecar, no filename suffix, and
  real art dropped at the same path silently drops off the worklist - the
  "no code change" contract the Story asked for, with nothing to maintain.
- The round-1 out-of-context reviewer found the two things that mattered (the
  stale mod-author guide, and the over-tight test) without having seen the
  implementing session.

## What went wrong

- The plan asked for "a picker assertion that no listed scenario resolves to a
  shared placeholder path". It was built, it passed, and it was deleted in
  review. The rule it encoded - never `banner.png`, one file per scenario - is
  an authoring POLICY, not a correctness property: reusing `banner.png` as a
  deliberate thumbnail is legitimate, and the test would have failed it. The
  Step, its DoD line, and a `development.md` sentence all came out with it.
- It seemed sound at plan time because the Story's complaint WAS "every
  scenario shows the same image", so a test forbidding exactly that reads like
  pinning the fix. The gap: the Story described a bad state that existed once,
  not an invariant the project wants enforced forever.
- Review round 1 also raised a MAJOR on a self-ticked inspection Step. It was
  withdrawn - the owner had done the inspection - but the Evidence block named
  one plate out of thirteen, so the record could not show it.

## What to improve next time

- Before a Step says "add an assertion that X never happens", ask whether X is
  a defect or a choice. If a future author could reasonably want X, the guard
  belongs in an advisory report, not a failing test. Here the coverage report
  already did the job the test was added for.
- When a Step says "confirm EACH", the Evidence block owes a line per item or
  an explicit statement of what was sampled and why. One example plus a tick
  reads as a self-ticked `manual:` to any reviewer who was not there.

## Action items

- None open. The declined MINOR/NITs (R1.3, R1.5, R1.6) are recorded in
  REVIEW.md and seed no task.
- Carried in RETRO, not as a task: `SCENARIOS` in the generator is a second
  list of picker-visible scenarios beside the game's own, and nothing reconciles
  them. Fine at 13 scenarios and hand-authored webmods; revisit if it grows.

## Landing message

```
feat(scenario): a distinct placeholder thumbnail per picker scenario

Every picker-visible scenario showed the same image - the six base scenarios
pointed at `self://banner.png`, the seven portal-mod ones at
`dep://base/textures/asteroid.png` - so the details pane told the player
nothing about what they were about to fly.

Real per-scenario art is authored, not captured (owner call 2026-08-04), so
this ships good placeholders until it exists: `scripts/gen-scenario-thumbnails.py`
renders one deterministic 320x180 phosphor plate per scenario from a built-in
5x7 font. Each PNG lands in the OWNING mod's tree and is referenced
`self://thumbnails/<id>.png`; no scenario borrows another mod's art. Both
webmod bundles gained a `resources` list, a version bump and a CHANGELOG entry.

`gen-web-screenshots.py --report` closes with the scenarios still on generated
art, classed `manual` - it tells placeholder from real by re-rendering and
comparing bytes, so dropping real art at the same path needs no code change.
Its PNG encoder is now shared rather than duplicated.
```
