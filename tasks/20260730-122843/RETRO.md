# RETRO: the dock shows only the verbs you can use now

- TASK: 20260730-122843
- DATE: 2026-07-30
- ROUNDS: 1 review round (APPROVE with 1 MINOR + 4 NITs, all applied)

## What this was

A playtest reversal. Task 20260728-175742 shipped the icon dock with a
deliberate rule - "unlike the text cluster it replaced, a dim chip STAYS on
screen", on the argument that constant chip positions make the row glanceable -
and the owner played it and asked for the pre-dock rule back. The change itself
is small; the interesting part was that the reversal collided with an existing
feature.

## What went well

**The plan gate caught the real fork, and it was a genuine incompatibility.**
Scenarios spotlight a verb via `HintEmphasisSet`, and `pulse_emphasized_chips`
carries a dedicated `EMPHASIS_ALPHA_UNAVAILABLE` band precisely so a tutorial
can point at a verb BEFORE it becomes available. A strictly hidden chip cannot
pulse and a pulsing chip is not hidden: the two wants were mutually exclusive on
that one chip. That went to the owner as a DECISION.md with the constraint
named, not as a menu of options - which is exactly what the flow skill's
"confirm the ARTIFACT, name the constraint" guideline exists for. It cost one
question and bought a rule that did not have to be discovered mid-build. The
reviewer independently confirmed the exception is load-bearing on shipped
content: `shakedown_run.content.ron:1341` emphasizes GOTO while it is
unavailable.

**Fail-first, then A/B the subtle half.** Both rigs were written and watched
fail before the implementation. More useful: the change gate (adding
`HintEmphasis` to `update_dock`'s quiet-frame early-out) is the kind of clause
that looks decorative, so it got its own A/B - deleting just
`&& !emphasis.is_changed()` turns the emphasis rig red. Writing the rig so the
spotlight lands on a deliberately QUIET frame is what made that possible; a rig
that changed hints in the same frame would have passed either way and pinned
nothing.

**The rendered check found the evidence the reasoning could not.** Running
`screenshot_combat` under Xvfb and cropping the three shots at one zoom produced
an actual set change - five chips in two shots, four in the third, with the row
re-centring - rather than an argument that the row would re-centre. This is the
`eyeball-the-rendered-output` lesson paying off on a layout change.

## What went wrong

**I shipped an unpinned deviation and called it out as a deviation instead of
testing it.** The hidden-chip branch used to force `DockChipState::Dim`; I
changed it to write the chip's true state, reasoned that this was "more honest",
wrote that reasoning into TASK.md, and moved on. The reviewer mutated it back
and found all 14 tests still green - it was the one substantive part of the
change that survived deletion. Worse, when I looked at the case where the two
actually differ (keyless chip while `HudSituations` still reports a maneuver),
my version was the worse one: it leaves an off-screen chip marked `Hot`, which
`grow_hot_chips` holds grown so it pops back in mid-shrink. The honest-sounding
choice was wrong on the merits AND unpinned.

The tell was there and I walked past it: I wrote a sentence in TASK.md
explaining WHY the deviation was correct. A change that needs a paragraph of
justification and has no test is a change that has not been thought through -
the paragraph is doing the job the test should do.

**The DoD shipped a proof command that could not reach zero.** DoD 3's
`rg 'dim chip|greyed|all seven verbs' ... crates` hits nova_editor's unrelated
greyed coming-soon rows, and it hits the corrected CHANGELOG line's own "rather
than shown greyed out" - a correct sentence containing a banned word. I did spot
this at the plan gate and flagged it to the owner before building, which is the
right half; the wrong half is that a grep for the WORDS a stale claim is made of
rather than for the CLAIMS themselves is a shape I keep having to fix at verify
time rather than at plan time.

## Lessons

- `justify-a-deviation-with-a-test-not-a-paragraph` (NEW): when the
  implementation departs from the plan, the deviation needs its own failing-first
  pin, not a note in TASK.md explaining why it is right. Writing the
  justification IS the signal: here "the hidden branch should record the true
  state, not force Dim" survived full-suite mutation untouched, and in the single
  case where it mattered it was the worse choice (an off-screen chip left marked
  `Hot` for `grow_hot_chips` to hold grown). Kin of
  [[test-the-wiring-system-not-just-its-pure-helpers]] - same question ("would
  this pass if the change were reverted?"), asked of a design choice rather than
  of a system.
- `validate-proof-command-shape-at-plan-time` (recurrence, x4): the failure mode
  here is a new one for that slug - not wrong arity or a zero-match filter, but
  an ABSENCE grep whose terms are the WORDS of the stale claim rather than the
  claim. Such a grep can never reach zero: unrelated code legitimately uses the
  words, and the corrected prose usually names the thing it is correcting
  ("rather than shown greyed out"). Write absence proofs against the specific
  phrases that were really in the tree, and sanity-check at plan time that the
  command CAN return zero.
- `spotlight-beats-the-hide` (domain, x1): when a HUD surface gains a rule that
  removes elements, check what deliberately points AT elements before writing
  it. The dedicated `EMPHASIS_ALPHA_UNAVAILABLE` band was the evidence that
  "spotlight an unavailable verb" was a supported case and not an accident - a
  dead-looking constant is sometimes a feature's only remaining trace.

## For next time

Ask "would this test pass if I reverted this?" of every load-bearing DESIGN
choice in the diff, not just of the systems - the review found this by mutation
in one pass, so it is cheap and I should be running it myself before handing the
branch over. The two places I did run it (the change gate, and later the new
hot-chip pin) are the two places the branch is strongest.
