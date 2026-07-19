# Retro: probe consolidation (one front door)

- TASK: 20260719-174603
- BRANCH: refactor/probe-consolidation (squash-landed as be603a3e)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs recorded)

## What went well

- Deletion-gated-on-validation as a structure: every script died only
  after its replacement passed a LIVE run, and the task text carried the
  fallback ("the script stays if not validated") so the risky web port
  could not silently ship unproven. The web capture validated at 29.4 ms
  against a 34-39 ms historical band - evidence, not hope.
- The first e2e round earned its cost three times over: console-line
  wrapping, sweep pass naming, and per-cell logs were all real
  composition gaps that unit tests could not reach, and each fix's pin
  uses the LIVE evidence (the actual chromium line is the parser
  fixture).
- The hardening task's foundations (manifest, fresh dirs,
  outcome-not-abort) slotted in exactly as designed - the report gate is
  four lines because probe-run.json already existed.

## What went wrong

- Three consecutive python-edit scripts died on exact-text anchors that
  had drifted (fmt reflow, mis-remembered context, an anchor INSIDE a
  splice range that also matched later). Each failure was safe
  (die-before-write) but cost a round trip; the splice that would have
  eaten the Run-timeline wiki section was only caught by checking section
  ORDER before re-running.
- The wiki re-read at the end still found two staleness spots (the lead
  paragraph presenting aliases as current; the skill's artifact list
  missing the manifest) AFTER the "reference sweep" step was ticked -
  grep finds names, not meaning; only a full re-read catches prose that
  is accurate word-by-word and wrong as a whole.

## What to improve next time

- For multi-edit doc surgery, verify each anchor with a probe pass
  (positions + uniqueness) BEFORE the mutating script - the two-step cost
  is lower than repeated die-before-write round trips.
- A reference sweep is grep PLUS a full re-read of each touched section;
  tick the docs step only after the re-read.

## Action items

- [x] Lesson added: doc-sweep-grep-plus-reread.
- [ ] NITs recorded for later: web adapter identity when chromium logs it
      outside the scrape window; positional overload on --platform web.
- [ ] Deprecated aliases removal is a one-line follow-up at the next
      release cut (noted in CHANGELOG).
