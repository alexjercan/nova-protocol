# Retro: Thrust balancing via differential throttle

- TASK: 20260709-155920
- BRANCH: thrust-balancing (squash-merged as 8ab00e4)
- REVIEW ROUNDS: 1 (APPROVE, 2 NITs, both addressed)

What shipped is in the task's Resolution and
`tasks/20260709-155920/NOTES.md`. A smooth cycle; the design work was the
substance, the review was clean.

## What went well

- **The model call was settled before any code, with a computed argument, not a
  vibe.** Feed-forward was rejected because it cannot exceed the controller's
  `max_torque` - the very cap this task exists to stop fighting - so it removes
  lag, not the authority ceiling. That reasoning (from reading the retune doc
  and the R1.2 test first) made the AskUserQuestion a confirmation, not a
  fishing trip: one round, accepted, zero redirects. Ask-first on a feel/handling
  call continues to pay.
- **"Force hard, torque objective" was chosen by reasoning through the failure
  modes of the alternative.** "Maximize thrust s.t. zero torque" is the tempting
  clean formulation, but it ignores the autopilot's commanded burn magnitude
  (breaking arrival control) and strands a lone engine (torque=0 forces it off).
  Walking those two cases out on paper picked the formulation that degrades to
  the exact pre-balance behavior for one engine - so the R1.2 pin stayed valid
  untouched and there was no regression to argue about in review.
- **Arithmetic before assertions, again.** The QP optimum (uA=0.75, uB=0.25) and
  the twin-drive COM (x=0.75, arms 3.25/1.75) were hand-computed before the
  tests, and the physics test contrasts partial vs full burn on the *same*
  geometry so headroom is the only variable - non-vacuous by construction, not
  by tuning until green.
- **One test validated two things for free.** The balanced-holds + full-pulls
  pair also proved the world-COM lift wasn't double-counting avian's `Position`
  (a double-count would compute the allocation about the wrong center and drift).
  End-to-end physics was cheaper and more convincing than auditing avian's
  Position-vs-COM semantics from its source.

## What went wrong

- **Repeated the multi-filter `cargo test` mistake** (`cargo test a b c` errors
  after the slow compile). This is the exact lesson from the
  20260709-155922 disabled-controller retro - "one substring filter at a time" -
  and it still happened by reflex. Root cause: muscle memory reached for a list
  before recalling the rule. Second occurrence.
- **Wrote a redundant review NIT (R1.2).** As reviewer I flagged a missing
  "sections are direct children of the root" comment that I had *already* written
  into the firing-pass code as the implementer. Root cause: didn't re-read the
  target code region before writing a doc-gap finding.

## What to improve next time

- For a spread of related tests, filter by their common module prefix
  (`cargo test flight::`), never a space-separated list. Treat any urge to pass
  two names as the signal to use the prefix instead.
- Before writing a "missing comment/doc/name" review finding, re-read the exact
  lines - especially on a diff I authored, where I may have already addressed it.

## Action items

- [ ] tatr 20260709-224518 (created): recruit off-axis thrusters for pure
  counter-torque, so a *single* main drive can be balanced against a shifted
  COM. The firing-set-only scope this task shipped cannot fix the most common
  damage case (one centered main drive, a side section lost -> COM shifts ->
  the lone drive is now off-center with nothing in the firing set to trim
  against). Documented as a deliberate boundary in
  `tasks/20260709-155920/NOTES.md`.
- [ ] The "one cargo test filter" rule now has two occurrences across retros;
  if it recurs a third time, promote it from retro-lore to an AGENTS.md testing
  note rather than relying on memory.
</content>
