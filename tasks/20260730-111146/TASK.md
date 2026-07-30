# Tutorial page still names the retired objective panel

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,docs,web


## Story

`web/src/tutorial.html:85` still says "Her lines carry the story; the objective
panel just states the goal." The always-on compact objective PANEL was removed
from flight in task 20260724-134312; a posted objective is now a chip in the
top-centre objective stack (and, since 20260729-211200, the chip is the whole
posting). A reader following the Shakedown Run walkthrough is told to look at a
widget that no longer exists.

Pre-existing drift, spotted during 20260729-211200's doc sweep by the
out-of-context reviewer's wider grep. Filed rather than folded into that branch
because the sentence was already stale before that change touched anything
(review skill: pre-existing problems become new tasks, not scope on the branch).

## Steps

- [ ] Rewrite the sentence to name the objective chip, e.g. "Her lines carry the
      story; the objective chip just states the goal."
- [ ] Re-read the whole tutorial page for other flight-HUD references that the
      contextual-HUD rework (20260724-134312 onward) invalidated - the page has
      not been swept since.

## Definition of Done

1. cmd: `grep -rn "objective panel" web/src/` returns nothing.
2. manual: the whole tutorial page has been re-read against the current flight
   HUD, and every widget it names is one that still exists - list in this task
   what was checked and what (if anything) else was rewritten, so the sweep is
   proven rather than implied by DoD 1's single-hit grep (raised by the
   20260729-211200 round-2 reviewer).
3. cmd: `cd web && npm run ci` passes.
