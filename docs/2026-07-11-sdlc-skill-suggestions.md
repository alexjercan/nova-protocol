# SDLC skill suggestions from the twitching-family session (2026-07-11)

Source material: one long /flow session that ran nine full
plan-work-review-compound cycles (the five twitching-family tasks plus four
playtest follow-ups), two falsification-only cycles, one family spike, and
continuous mid-flow user feedback. Also a re-read of the retro corpus while
applying its lessons (104 retros, 18 spikes, 37 dated docs at time of
writing). Each suggestion says what evidence backs it.

## What the current setup demonstrably does well

Keep these; they are the load-bearing parts.

- **Retro lessons actually compound.** Four separate practices used today
  came straight from earlier retros and each paid off measurably:
  diagnostic-first tick traces (residual-roll retro) corrected wrong plan
  mechanisms in four cycles before code was written; record-the-exact-rig
  made two falsification closes reusable; sweep-then-delete (orbit-ring
  retro) kept the caption removal minimal; the landing-sequence rule
  caught a real from-inside-the-worktree merge mistake.
- **The family spike with a living "Fix record" section.** One mechanism
  doc (two-clocks) coordinated six tasks; each cycle appended its outcome
  to the fix record, so every later cycle started from the current family
  state instead of re-reading five TASK.md files. The doc was cited in
  code comments, commit messages and reviews.
- **One task, one worktree, one squash commit.** Nine sprout cycles, zero
  collisions, clean master history. sprout rm after squash worked every
  time.
- **Honest falsification closes.** Two "bug" tasks closed with evidence
  instead of code (hull provably steady; camera jump mechanically
  explained) and both redirected follow-up work to the right place. The
  review skill accepted these as legitimate outcomes, which kept the
  session from shipping fixes for nonexistent mechanisms.

## Suggestions per skill

### plan

1. **Verify-first phrasing for mechanism steps (promotion-ready).** Three
   plans in a row encoded a mechanism that reading one file would have
   falsified: the straight-line-burn regression (torque-blind by
   geometry), the bullet overshoot formula (wrong algebra), the
   balancer-chatter hypothesis (single-engine ship). Proposed guideline
   text for the skill's "Guidelines for Good Steps":
   "A step that encodes a physical mechanism, a formula, or a
   dependency's schedule/ordering behavior must either cite the file or
   derivation that verifies it, or be phrased as a verify-first question
   ('confirm X, then...'). Plans written from a model of the system
   instead of the system have been wrong repeatedly."
   The same applies to external-crate ordering assumptions: two plan
   steps this session assumed schedule orderings (bcs sync vs
   propagation, bevy_ui layout vs propagation) that the source
   contradicted.
2. Keep the existing "a plan's 'if feasible' must be answered
   explicitly" lesson - it held this session.

### work

3. **Codify fail-first for bug fixes.** Every fix cycle today proved its
   regression against the bug (A/B) before trusting it: 7.1 rad/s vs 0,
   4.26 vs 0, 54 px vs sub-pixel, scattered vs uniform stream. This is
   the single highest-value habit of the session and it lives nowhere in
   the skills. Suggested step 4 addition: "For a bug fix, demonstrate the
   new test failing against the pre-fix behavior (temporary revert or
   sabotage) before closing the task; record the failing numbers."
4. **A/B safety rule.** Corollary with a scar: commit the fix BEFORE
   applying any sabotage/revert for the A/B, so `git checkout <file>`
   restores the fix and not the branch base. A file-level checkout
   destroyed ~250 uncommitted lines this session (recovered from session
   context by luck). One sentence in the work skill prevents it.
5. **Production-faithful test rigs.** Two occurrences now: a rig without
   the production scheduling components (TransformInterpolation)
   understated a bug by an order of magnitude. Suggested note: "when a
   test rig models a scheduling/clock behavior, mirror the production
   entity's scheduling-relevant components; a clean trace on a
   non-faithful rig is not evidence."

### review

6. **Null-assertion checklist item.** "Nothing happens" assertions passed
   review only after gaining delivery guards proving the stimulus fired
   (three applications today after one MAJOR finding). Suggested
   guideline: "any 'X stays zero / nothing moves' assertion needs a
   paired delivery assertion proving the provoking stimulus actually
   acted."
7. **An honesty note on same-session review.** Six of nine rounds today
   were APPROVE with no findings. Some genuinely were clean one-format
   -string diffs, but implementer and reviewer sharing one context is a
   structural blind spot. Suggestion for the skill: for substantial
   branches, the reviewer should do at least one independent
   re-derivation or re-verification (this session: re-deriving the lerp
   steady state, re-checking spawn hierarchies) rather than reading the
   diff alone, and consider /code-review for an out-of-context pass on
   large changes.

### compound / the docs-taking method

8. **A lessons ledger, not just retro files.** With 104 retros, recurring
   -lesson mining ("is this the third occurrence?") depends on memory of
   which files to reopen. Today's promotion threshold was only detected
   because the same session wrote all three occurrences. Suggestion: a
   single `docs/retros/LESSONS.md` ledger with one line per lesson
   (slug, one sentence, occurrence count, retro links), appended by
   /compound. Retros keep the narrative; the ledger makes recurrence
   detection mechanical and gives /flow's read-recent-retros step a
   5-minute alternative to sampling.
9. **A pending-promotions section in that ledger.** The plan-skill
   guideline proposal from today sits inside one retro file where it will
   scroll away; promotions awaiting the user's decision (skills are
   user-global config) need one visible parking spot.
10. **Reduce triple-writing.** Several cycles today wrote overlapping
    prose into TASK.md Resolution, the spike fix record, and the retro.
    Sharper contract: TASK.md = what/why/evidence rig (complete);
    spike fix record = 3-5 line family status pointing at the task;
    retro = process observations ONLY, and a smooth cycle earns three
    lines. The compound skill says "do not duplicate TASK.md" - it needs
    the same warning about spike fix records.

### flow

11. **Promote the landing-sequence rule into the skill.** "Run the
    squash-merge from the main checkout" already appears in flow, but the
    failure keeps recurring through compound commands (three retro
    mentions plus one near-miss today). Sharpened wording worth adopting:
    "The landing command must not contain a `cd` at all; it starts with
    `pwd` in the main checkout as its own command."
12. **Mid-flow feedback protocol worked - write it down.** Today's
    improvised pattern was: queue each user observation as a task with
    priority, record interim playtest verdicts on the umbrella task, and
    never widen the current branch. It kept five interrupts orderly.
    Flow has one sentence about new work becoming tasks; an explicit
    "user feedback arriving mid-cycle" paragraph (verdicts -> umbrella
    notes, requests -> tasks, current branch untouched) would make it
    repeatable.
13. **Diagnosis-only cycles are legitimate flow outcomes.** Two of nine
    cycles closed with evidence and routing instead of code. Flow/work
    describe implementation cycles only; one sentence acknowledging the
    falsification close (with the same review/retro rigor) would stop
    future sessions from forcing a code change where none is warranted.

### tatr

14. **Same-second ID collision (real bug).** Two `tatr new` calls in one
    second returned the same ID and the second title silently overwrote
    the first (hit today; recovered manually, then worked around with
    `sleep 1`). tatr should bump the seconds or refuse. Until fixed, a
    note in the tatr skill: "creating several tasks in one command line
    needs a sleep between calls".
15. The `Depends on:` free-text convention plus priority ordering was
    sufficient across nine dependent tasks; no structured dependency
    field needed.

### spike

16. **Codify the living fix record.** For a spike that seeds multiple
    tasks, add to the spike skill: "give the doc a 'Fix record' section
    and have each implementing task append its outcome (a few lines plus
    a pointer to TASK.md); the spike doc is the family's single source of
    current state." This emerged ad hoc today and was the best
    cross-task continuity device of the session.
17. Spikes as evidence-holders for feel work: the queued feel spike
    (20260711-125227) starts from measured numbers (hitch transient
    magnitudes, lag constants) gathered by earlier diagnosis cycles.
    Diagnosis tasks should route their numbers into the spike's task file
    the way today's camera cycles did.

### sprout

18. No changes needed on this session's evidence. The only sprout-adjacent
    footgun (landing from inside a worktree) is flow's to fix (item 11).

## What was less helpful in the current corpus

- **Retro action-item checkboxes rarely get revisited.** Several older
  retros carry unchecked action items (e.g. "user re-runs the playtest
  checklist") whose state is unknowable without archaeology. The ledger
  (item 8) plus converting anything actionable into a tatr task at
  compound time (already the rule - enforce it) would let retro action
  items be trivially droppable.
- **The docs/ root is a flat pile of 37 dated files.** The dated-file
  convention itself is good (stable links from code comments), but
  discovery is now grep-only. A short `docs/README.md` index grouped by
  subsystem (flight, camera, HUD, turrets, process) would cost little and
  make the corpus navigable; it was NOT missed for writing (links were
  always known at write time) but was missed when hunting prior art.
- **Presence-style checks in older docs' verification sections.** The
  camera-twitch doc's own retro already flagged this; today's cycles
  consistently used behavioral bounds instead. Nothing to remove -
  just do not regress to component-exists assertions.

## Suggested next steps (user's call)

- Apply items 1, 3, 4, 6, 11 as edits to the plan/work/review/flow
  skills - each is a sentence or two, and each has 2-3 documented
  occurrences behind it.
- Create `docs/retros/LESSONS.md` seeded from this session's retros and
  teach /compound to append to it (items 8-9).
- Fix or document the tatr same-second collision (item 14).
