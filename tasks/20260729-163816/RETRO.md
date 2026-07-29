# Retro: Objective read-notification stack

- TASK: 20260729-163816
- BRANCH: feat/objective-notification-stack
- REVIEW ROUNDS: 3 (R1: 2 MAJOR + 5 MINOR + 2 NIT; R2: 1 MAJOR + 2 MINOR + 1
  NIT, the MAJOR introduced by R1's own fix; R3: APPROVE with 3 NIT, all taken)

## What went well

- **Stopping to ask was the whole task.** "Objectives still look the same" was
  a look complaint, but the fix was a different ARTIFACT, and three of the
  plausible artifacts were mutually exclusive with constraints the owner could
  not see from a screenshot (a status-bar item cannot carry a bordered pill; the
  demo's chip has no count or TAB; top-centre is already occupied). Naming
  those constraints got a shape back in two exchanges - including one I had not
  offered, the read-notification model - instead of a fourth wrong build. The
  flow skill's "confirm the ARTIFACT, not the placement" rule earned its keep.
- **The out-of-context reviewer earned its keep three times over**, and not by
  reading the diff: it ran probes. R1.1 (my pop was dead) and R2.1 (my fix
  handed the wrong objective over) were both MEASURED with temporary
  integration probes driving the real card animation, and R2.2 was found by
  deleting a guard and observing that all ten tests still passed. Every finding
  I could have argued with came with a number.
- **The screenshots found what tests could not, again.** The tofu diamond and
  the 58 px collision with the run timer are invisible to every headless
  assertion and obvious in one frame. Both were fixed before review saw them.
- **Copying the in-repo callsite beat inventing one.** The diamond, the
  rebuild-per-frame + age-seeded emphasis, and the `SystemSet`-for-ordering all
  already existed in this module tree; each time I reached for the local
  precedent the result was smaller and the reviewer had nothing to say about it.

## What went wrong

- **I shipped a pop that never played, and asserted it in five places.** The
  chips are rebuilt every frame from `ObjectiveNotifications`, so writing
  `HudEmphasis::pop()` onto a chip ENTITY was overwritten before it could ease.
  Root cause: I designed the rebuild-per-frame model and its age-seeding
  discipline myself, wrote the lesson about it in the previous task's ledger
  entry, and then broke that exact discipline in the one system that reaches in
  from outside. Worse, the docstring, the TASK step, DECISION.md, the wiki and
  the CHANGELOG all described the behaviour as if it worked.
- **No test could have caught it, because my rig omitted the driver.** The test
  app never registered `drive_hud_emphasis`, so nothing in the module touched
  `UiTransform` at all. The two deleted hint tests
  (`the_hint_pops_when_the_reveal_tucks_in_and_settles_back`,
  `the_hint_breathes_only_while_objectives_are_outstanding`) pinned exactly this
  still-live behaviour, and I deleted them with `objective_hint.rs` without
  re-homing them - the `deleting-a-test-salvage-live-assertions` lesson, which
  is IN the ledger, on a module I deleted in this very task.
- **My fix for that introduced a worse bug.** Matching a landing card to "the
  oldest chip still waiting" was positional, and `ObjectiveRevealTucked` carried
  no identity, so a card that outlived its notification - or a fallback that
  beat its own card - handed over the NEXT objective, a second before its card
  landed. That is the duplication R1.9 asked me to remove, resurrected on a path
  a schedule tie-break decides. Root cause: I reached for "which one is
  waiting?" (positional, implicit) when the message could simply have SAID which
  objective it was.
- **My first regression test for that bug passed under the bug.** It tucked two
  cards in posting order, which positional matching also gets right. I only
  found out because I ran the mutation myself - had I trusted "it's green", I
  would have shipped a test that proved nothing about the thing it was named
  after.

## What to improve next time

- When a module owns a rebuilt-every-frame view, any system that reaches in
  from OUTSIDE must write the STATE, never the rebuilt entity. Write that rule
  at the top of the module the moment the rebuild model is chosen, not after a
  reviewer measures the consequence.
- Deleting a module means reading its tests one assertion at a time and asking
  which ones pin behaviour that still exists. Two of the five here did. The
  ledger lesson exists; what was missing was doing it at deletion time rather
  than trusting that "the module is gone, so are its tests".
- A test rig for a module that renders through a SHARED driver must register
  that driver, in the schedule production uses. If the rig cannot observe the
  rendered property (scale, alpha), the test is asserting intent, not behaviour.
- An event that triggers work on a specific subject should carry that subject's
  identity. "Find the one that must have meant" is a guess that survives exactly
  until two are in flight.
- Prove a regression test fails under the bug it names, by mutation, before
  believing it - especially when the test was written after the fix.

## Action items

- [x] Ledger: `rebuilt-view-writes-go-to-state-not-the-entity` (new).
- [x] Ledger: bumped `deleting-a-test-salvage-live-assertions` to x2.
- [x] Ledger: `identify-the-subject-in-the-event` (new).
- [x] Ledger: bumped `would-it-fail-without-it` with the passes-under-the-bug
      variant.
- [x] 20260729-182853 (backlog, filed): a screenshot beat that would show the
      card-to-chip handover - no harnessed walk can today, which is why the
      most visually load-bearing moment here is test-only.
- [ ] Open for the owner's playtest (DoD 5): the 12 s dwell, and whether "read"
      clearing everything with no standing cue feels right in the cockpit.
