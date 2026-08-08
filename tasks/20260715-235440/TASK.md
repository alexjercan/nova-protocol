# Fix stale example count in development.md: 'ten of the twelve' -> 18 harnessed, enumerate 13-18

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: bug, docs, web

## The bug (verified)

`web/src/wiki/dev/development.md:98` says "Ten of the twelve carry
panic-on-failure assertions". `HARNESSED_EXAMPLES` in `tests/examples_smoke.rs`
lists 18 examples (01-18), not twelve. The Examples section on the same page also
enumerates only 01-12 and never mentions 13-18 (screenshot reel/ui/combat/
sections/juice/orbit).

## Fix

- Correct "the twelve" -> the real count (18), keeping the "ten of ..." exception
  wording accurate (recount which lack assertions, or reword).
- Extend the Examples enumeration to cover 13-18.
- Consider deriving the count from the list rather than typing it (this is the
  second freshness-drift bug of this kind; see spike 20260715-235232 theme 2).

Source: spike 20260715-235232 (developer persona review, verified).

## Done

development.md: rewrote the harness paragraph - "runs the full set ... Ten of
the twelve" -> "runs all eighteen ... The gameplay examples (01-12) additionally
carry ... except 06/09 ...; the screenshot examples (13-18) drive the shipped
scenes to capture frames." Also added a "Screenshots:" bullet enumerating 13-18
to the Examples list. Avoided a fragile "N of M" count; the 18 total is pinned to
HARNESSED_EXAMPLES in tests/examples_smoke.rs.
